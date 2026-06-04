#![cfg(feature = "candle-runtime-engine")]

//! Real teacher-forcing scoring and embedding for the Candle backend.
//!
//! `score` forwards a token sequence through the loaded model with the
//! per-position teacher-forcing seam ([`TransformerModel::forward_full_logits`])
//! and computes the genuine next-token log-probability at each position via a
//! numerically-stable log-softmax (log-sum-exp). `embed` tokenizes the text,
//! forwards it through [`TransformerModel::forward_hidden_states`], and
//! mean-pools the post-final-norm hidden states into a single real embedding
//! vector. No placeholders: every absent/degenerate case is a typed error.

use std::sync::{Arc, Mutex};

use candle_core::{IndexOp, Tensor};

use super::{hooks::CandleSteeringHooks, transformer::TransformerModel};
use crate::model_runtime::{Embedding, ModelRuntimeError, Score};

/// Compute REAL per-token log-probabilities for `sequence` under the model via
/// teacher forcing.
///
/// For a sequence `t_0, t_1, ... t_{n-1}` we forward all `n` tokens and read the
/// logits at position `i` (the model's prediction for the token that follows
/// `t_i`). The log-probability of the ACTUAL next token `t_{i+1}` under that
/// distribution is `log_softmax(logits_i)[t_{i+1}]`. We collect one logprob per
/// adjacent pair, so `token_logprobs.len() == sequence.len() - 1`, matching the
/// teacher-forcing convention (the first token has no preceding context to be
/// scored against).
pub fn candle_score_sequence(
    model: &Arc<Mutex<Box<dyn TransformerModel>>>,
    hooks: &CandleSteeringHooks,
    sequence: Vec<u32>,
) -> Result<Score, ModelRuntimeError> {
    // A sequence of <2 tokens has no scorable adjacent pair. Return an empty,
    // honest score (mean 0.0 over zero tokens) rather than erroring — mirrors
    // the llama.cpp backend's contract.
    if sequence.len() < 2 {
        return Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        });
    }

    let mut locked = model.lock().map_err(|_| {
        ModelRuntimeError::ScoreError("Candle transformer model lock is poisoned".to_string())
    })?;

    let vocab_size = locked.vocab_size() as usize;
    if vocab_size == 0 {
        return Err(ModelRuntimeError::ScoreError(
            "Candle model reports zero vocab size".to_string(),
        ));
    }
    for &token in &sequence {
        if token as usize >= vocab_size {
            return Err(ModelRuntimeError::ScoreError(format!(
                "Candle score token id {token} is outside vocab size {vocab_size}"
            )));
        }
    }

    let device = locked.device();
    let input = Tensor::new(sequence.as_slice(), &device)
        .and_then(|tensor| tensor.reshape((1, sequence.len())))
        .map_err(|error| {
            ModelRuntimeError::ScoreError(format!("Candle score input tensor failed: {error}"))
        })?;

    // [1, seq, vocab] F32 — logits at every position.
    let logits = locked.forward_full_logits(&input, hooks, &[], &[])?;
    drop(locked);

    let dims = logits.dims().to_vec();
    let (seq_len, logits_vocab) = match dims.as_slice() {
        [1, seq, vocab] => (*seq, *vocab),
        [seq, vocab] => (*seq, *vocab),
        other => {
            return Err(ModelRuntimeError::ScoreError(format!(
                "Candle score expected logits shape [1, seq, vocab] or [seq, vocab], got {other:?}"
            )));
        }
    };
    if seq_len != sequence.len() {
        return Err(ModelRuntimeError::ScoreError(format!(
            "Candle score logits seq dim {seq_len} does not match input length {}",
            sequence.len()
        )));
    }
    if logits_vocab != vocab_size {
        return Err(ModelRuntimeError::ScoreError(format!(
            "Candle score logits vocab dim {logits_vocab} does not match model vocab {vocab_size}"
        )));
    }

    // Flatten to [seq, vocab] so we can index rows uniformly.
    let logits_2d = logits
        .reshape((seq_len, logits_vocab))
        .and_then(|tensor| tensor.to_dtype(candle_core::DType::F32))
        .map_err(|error| {
            ModelRuntimeError::ScoreError(format!("Candle score logits reshape failed: {error}"))
        })?;

    let source_count = sequence.len() - 1;
    let mut token_logprobs = Vec::with_capacity(source_count);
    for position in 0..source_count {
        let row = logits_2d.i(position).map_err(|error| {
            ModelRuntimeError::ScoreError(format!(
                "Candle score failed to read logits row {position}: {error}"
            ))
        })?;
        let row: Vec<f32> = row.to_vec1().map_err(|error| {
            ModelRuntimeError::ScoreError(format!(
                "Candle score failed to materialize logits row {position}: {error}"
            ))
        })?;
        let target = sequence[position + 1] as usize;
        token_logprobs.push(logprob_for_target(&row, target)?);
    }

    let mean_logprob = if token_logprobs.is_empty() {
        0.0
    } else {
        token_logprobs.iter().sum::<f32>() / token_logprobs.len() as f32
    };
    Ok(Score {
        token_logprobs,
        mean_logprob,
    })
}

/// Numerically-stable `log_softmax(logits)[target]` with full finiteness
/// guards, matching the llama.cpp backend's contract (returns a non-positive,
/// finite logprob clamped at 0.0).
fn logprob_for_target(logits: &[f32], target_token: usize) -> Result<f32, ModelRuntimeError> {
    if target_token >= logits.len() {
        return Err(ModelRuntimeError::ScoreError(format!(
            "Candle score target token {target_token} is outside logits width {}",
            logits.len()
        )));
    }
    for (index, logit) in logits.iter().enumerate() {
        if !logit.is_finite() {
            return Err(ModelRuntimeError::ScoreError(format!(
                "Candle score logit at index {index} is not finite"
            )));
        }
    }

    let max_logit = logits
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, |left, right| left.max(right));
    if !max_logit.is_finite() {
        return Err(ModelRuntimeError::ScoreError(
            "Candle score logits did not contain a finite maximum".to_string(),
        ));
    }

    let exp_sum = logits
        .iter()
        .map(|logit| f64::from(*logit - max_logit).exp())
        .sum::<f64>();
    if !exp_sum.is_finite() || exp_sum <= 0.0 {
        return Err(ModelRuntimeError::ScoreError(
            "Candle score logits produced an invalid logsumexp denominator".to_string(),
        ));
    }

    let logsumexp = f64::from(max_logit) + exp_sum.ln();
    let logprob = f64::from(logits[target_token]) - logsumexp;
    if !logprob.is_finite() {
        return Err(ModelRuntimeError::ScoreError(
            "Candle score produced a non-finite log probability".to_string(),
        ));
    }
    // log-prob is mathematically <= 0; allow a tiny positive epsilon for fp
    // rounding, then clamp.
    if logprob > 1.0e-4 {
        return Err(ModelRuntimeError::ScoreError(format!(
            "Candle score produced positive log probability {logprob}"
        )));
    }
    Ok((logprob as f32).min(0.0))
}

/// Compute a REAL embedding for `token_ids` by forwarding through the model and
/// mean-pooling the post-final-norm hidden states across the sequence. Returns
/// a single `[hidden]` vector. Caller tokenizes the text and supplies the ids.
pub fn candle_embed_tokens(
    model: &Arc<Mutex<Box<dyn TransformerModel>>>,
    hooks: &CandleSteeringHooks,
    token_ids: Vec<u32>,
) -> Result<Embedding, ModelRuntimeError> {
    if token_ids.is_empty() {
        return Err(ModelRuntimeError::EmbedError(
            "Candle embed produced no tokens for the input text".to_string(),
        ));
    }

    let mut locked = model.lock().map_err(|_| {
        ModelRuntimeError::EmbedError("Candle transformer model lock is poisoned".to_string())
    })?;

    let vocab_size = locked.vocab_size() as usize;
    for &token in &token_ids {
        if token as usize >= vocab_size {
            return Err(ModelRuntimeError::EmbedError(format!(
                "Candle embed token id {token} is outside vocab size {vocab_size}"
            )));
        }
    }
    let expected_dim = locked.hidden_dim() as usize;

    let device = locked.device();
    let seq_len = token_ids.len();
    let input = Tensor::new(token_ids.as_slice(), &device)
        .and_then(|tensor| tensor.reshape((1, seq_len)))
        .map_err(|error| {
            ModelRuntimeError::EmbedError(format!("Candle embed input tensor failed: {error}"))
        })?;

    // [1, seq, hidden] F32.
    let hidden = locked.forward_hidden_states(&input, hooks, &[], &[])?;
    drop(locked);

    let dims = hidden.dims().to_vec();
    let hidden_2d = match dims.as_slice() {
        [1, seq, h] => hidden.reshape((*seq, *h)),
        [seq, h] => hidden.reshape((*seq, *h)),
        other => {
            return Err(ModelRuntimeError::EmbedError(format!(
                "Candle embed expected hidden shape [1, seq, hidden] or [seq, hidden], got {other:?}"
            )));
        }
    }
    .map_err(|error| {
        ModelRuntimeError::EmbedError(format!("Candle embed hidden reshape failed: {error}"))
    })?;

    // Mean-pool across the sequence (dim 0) -> [hidden].
    let pooled = hidden_2d
        .mean(0)
        .and_then(|tensor| tensor.to_dtype(candle_core::DType::F32))
        .map_err(|error| {
            ModelRuntimeError::EmbedError(format!("Candle embed mean-pool failed: {error}"))
        })?;
    let vector: Vec<f32> = pooled.to_vec1().map_err(|error| {
        ModelRuntimeError::EmbedError(format!(
            "Candle embed failed to materialize vector: {error}"
        ))
    })?;

    validate_embedding_vector(vector, expected_dim)
}

fn validate_embedding_vector(
    vector: Vec<f32>,
    expected_dim: usize,
) -> Result<Embedding, ModelRuntimeError> {
    if vector.is_empty() {
        return Err(ModelRuntimeError::EmbedError(
            "Candle embedding vector is empty".to_string(),
        ));
    }
    if vector.len() != expected_dim {
        return Err(ModelRuntimeError::EmbedError(format!(
            "Candle embedding dimension mismatch: got {}, expected {expected_dim}",
            vector.len()
        )));
    }
    for (index, value) in vector.iter().enumerate() {
        if !value.is_finite() {
            return Err(ModelRuntimeError::EmbedError(format!(
                "Candle embedding value at index {index} is not finite"
            )));
        }
    }
    let norm_sq = vector.iter().map(|value| value * value).sum::<f32>();
    if !norm_sq.is_finite() || norm_sq <= 0.0 {
        return Err(ModelRuntimeError::EmbedError(
            "Candle embedding vector has invalid norm".to_string(),
        ));
    }
    Ok(Embedding { vector })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logprob_for_target_matches_manual_log_softmax() {
        // logits [1, 2, 3]; log_softmax of index 2 = 3 - log(e^1+e^2+e^3).
        let logits = [1.0_f32, 2.0, 3.0];
        let lp = logprob_for_target(&logits, 2).expect("finite logprob");
        let denom = (1.0_f64.exp() + 2.0_f64.exp() + 3.0_f64.exp()).ln();
        let expected = (3.0_f64 - denom) as f32;
        assert!((lp - expected).abs() < 1e-5, "lp={lp} expected={expected}");
        assert!(
            lp < 0.0,
            "log-prob of a non-dominant token must be negative"
        );
    }

    #[test]
    fn logprob_uniform_logits_is_negative_log_vocab() {
        // Uniform logits => each prob = 1/N => logprob = -ln(N).
        let logits = [0.0_f32; 8];
        let lp = logprob_for_target(&logits, 3).expect("finite logprob");
        let expected = -(8.0_f64.ln()) as f32;
        assert!((lp - expected).abs() < 1e-5, "lp={lp} expected={expected}");
    }

    #[test]
    fn logprob_rejects_out_of_range_target() {
        let logits = [0.0_f32, 1.0];
        assert!(logprob_for_target(&logits, 5).is_err());
    }

    #[test]
    fn logprob_rejects_non_finite_logits() {
        let logits = [0.0_f32, f32::NAN, 1.0];
        assert!(logprob_for_target(&logits, 0).is_err());
    }

    #[test]
    fn validate_embedding_rejects_dim_mismatch() {
        assert!(validate_embedding_vector(vec![1.0, 2.0], 3).is_err());
    }

    #[test]
    fn validate_embedding_rejects_zero_norm() {
        assert!(validate_embedding_vector(vec![0.0, 0.0, 0.0], 3).is_err());
    }

    #[test]
    fn validate_embedding_accepts_finite_nonzero() {
        let emb = validate_embedding_vector(vec![0.1, -0.2, 0.3], 3).expect("valid embedding");
        assert_eq!(emb.vector.len(), 3);
    }

    // ENV-GATED REAL-MODEL PROOF: load TinyLlama through the real candle load
    // path, then run REAL teacher-forcing score() and embed() against it. No
    // mocks — a genuine forward through the loaded safetensors.
    //
    // Run with:
    //   HANDSHAKE_TEST_CANDLE_LLAMA_MODEL=D:/Local Models/Maykeye_TinyLLama-v0/model.safetensors
    //   HANDSHAKE_TEST_CANDLE_LLAMA_SHA256=c302246d91e2854ff350e52a79938a818c203186c2f0671213ce372a0289cd91
    //   cargo test -p handshake_core --features candle-runtime-engine \
    //     real_tinyllama_score_embed -- --nocapture
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn real_tinyllama_score_embed() {
        use crate::model_runtime::candle::adapter::load_local_candle_model;
        use crate::model_runtime::ModelRuntime;

        let Some(artifact) = std::env::var_os("HANDSHAKE_TEST_CANDLE_LLAMA_MODEL") else {
            eprintln!("SKIP real_tinyllama_score_embed: HANDSHAKE_TEST_CANDLE_LLAMA_MODEL not set");
            return;
        };
        let artifact = std::path::PathBuf::from(artifact);
        let sha256 = std::env::var("HANDSHAKE_TEST_CANDLE_LLAMA_SHA256")
            .expect("HANDSHAKE_TEST_CANDLE_LLAMA_SHA256 required when the model path is set");

        let loaded = load_local_candle_model(artifact, sha256)
            .await
            .expect("real TinyLlama load");
        let runtime = loaded.runtime;
        let model_id = loaded.model_id;

        // --- REAL score(): teacher-force a short token sequence. ---
        // Token ids well within TinyLlama's vocab; a real forward produces a
        // genuine next-token distribution at each position.
        let sequence: Vec<u32> = vec![1, 450, 7483, 310, 3444, 338];
        let score = runtime
            .score(model_id, sequence.clone())
            .await
            .expect("real candle score");
        assert_eq!(
            score.token_logprobs.len(),
            sequence.len() - 1,
            "one logprob per scored next-token (teacher-forcing convention)"
        );
        for (i, lp) in score.token_logprobs.iter().enumerate() {
            assert!(lp.is_finite(), "token logprob {i} must be finite: {lp}");
            assert!(*lp <= 0.0, "token logprob {i} must be <= 0: {lp}");
        }
        assert!(
            score.mean_logprob.is_finite() && score.mean_logprob < 0.0,
            "mean_logprob must be a sane negative number, got {}",
            score.mean_logprob
        );
        eprintln!(
            "REAL score: mean_logprob={:.4} token_logprobs={:?}",
            score.mean_logprob, score.token_logprobs
        );

        // --- REAL embed(): mean-pooled hidden state for a string. ---
        let embedding = runtime
            .embed(model_id, "The capital of France is")
            .await
            .expect("real candle embed");
        assert!(!embedding.vector.is_empty(), "embedding must be non-empty");
        for (i, v) in embedding.vector.iter().enumerate() {
            assert!(v.is_finite(), "embedding value {i} must be finite: {v}");
        }
        let norm = embedding.vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!(
            norm.is_finite() && norm > 0.0,
            "embedding must have positive norm"
        );
        eprintln!(
            "REAL embed: dim={} norm={:.4} head={:?}",
            embedding.vector.len(),
            norm,
            &embedding.vector[..embedding.vector.len().min(6)]
        );
    }
}
