use crate::model_runtime::{Embedding, KvQuantSupport, ModelRuntimeError, Score};

use super::context::LlamaCppContext;

#[cfg(not(feature = "llama-cpp-runtime-engine"))]
use super::context::LLAMA_CPP_NATIVE_FEATURE_DISABLED;

const ADAPTER: &str = "llama_cpp";
const EMBEDDING_CAPABILITY: &str = "llama_cpp_embedding";

pub(super) async fn score(
    context: &LlamaCppContext,
    quantization: KvQuantSupport,
    sequence: Vec<u32>,
) -> Result<Score, ModelRuntimeError> {
    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let native = context.native_backend();
        return score_native_blocking(native, quantization, sequence).await;
    }

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let _ = (context, quantization, sequence);
        Err(ModelRuntimeError::ScoreError(
            LLAMA_CPP_NATIVE_FEATURE_DISABLED.to_string(),
        ))
    }
}

pub(super) async fn embed(
    context: &LlamaCppContext,
    quantization: KvQuantSupport,
    text: &str,
) -> Result<Embedding, ModelRuntimeError> {
    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let native = context.native_backend();
        let text = text.to_string();
        return embed_native_blocking(native, quantization, text).await;
    }

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let _ = (context, quantization, text);
        Err(ModelRuntimeError::EmbedError(
            LLAMA_CPP_NATIVE_FEATURE_DISABLED.to_string(),
        ))
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
async fn score_native_blocking(
    native: std::sync::Arc<super::context::NativeLlamaCppBackend>,
    quantization: KvQuantSupport,
    sequence: Vec<u32>,
) -> Result<Score, ModelRuntimeError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::spawn_blocking(move || score_native(native, quantization, sequence))
            .await
            .map_err(|error| {
                ModelRuntimeError::ScoreError(format!(
                    "llama.cpp score worker failed to join: {error}"
                ))
            })?
    } else {
        score_native(native, quantization, sequence)
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
async fn embed_native_blocking(
    native: std::sync::Arc<super::context::NativeLlamaCppBackend>,
    quantization: KvQuantSupport,
    text: String,
) -> Result<Embedding, ModelRuntimeError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::spawn_blocking(move || embed_native(native, quantization, &text))
            .await
            .map_err(|error| {
                ModelRuntimeError::EmbedError(format!(
                    "llama.cpp embedding worker failed to join: {error}"
                ))
            })?
    } else {
        embed_native(native, quantization, &text)
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn score_native(
    native: std::sync::Arc<super::context::NativeLlamaCppBackend>,
    quantization: KvQuantSupport,
    sequence: Vec<u32>,
) -> Result<Score, ModelRuntimeError> {
    use llama_cpp_2::llama_batch::LlamaBatch;

    if sequence.len() < 2 {
        return Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        });
    }

    let tokens = score_tokens(&native, &sequence)?;
    let source_count = sequence.len().saturating_sub(1);
    let mut context = native.new_context(quantization)?;
    if source_count > context.n_ctx() as usize {
        return Err(ModelRuntimeError::ScoreError(format!(
            "llama.cpp score sequence too long: {source_count} source tokens exceed n_ctx {}",
            context.n_ctx()
        )));
    }

    let batch_capacity = usize::try_from(context.n_batch()).unwrap_or(1).max(1);
    let mut token_logprobs = Vec::with_capacity(source_count);
    let mut source_start = 0_usize;

    while source_start < source_count {
        let chunk_len = batch_capacity.min(source_count - source_start);
        let mut batch = LlamaBatch::new(chunk_len, 1);
        for offset in 0..chunk_len {
            let absolute_position = source_start + offset;
            let position = i32::try_from(absolute_position).map_err(|error| {
                ModelRuntimeError::ScoreError(format!(
                    "llama.cpp score token position does not fit i32: {error}"
                ))
            })?;
            batch
                .add(tokens[absolute_position], position, &[0], true)
                .map_err(|error| {
                    ModelRuntimeError::ScoreError(format!(
                        "failed to add score token to llama.cpp batch: {error}"
                    ))
                })?;
        }

        context.decode(&mut batch).map_err(|error| {
            ModelRuntimeError::ScoreError(format!("llama.cpp score decode failed: {error}"))
        })?;

        for offset in 0..chunk_len {
            let logits_index = i32::try_from(offset).map_err(|error| {
                ModelRuntimeError::ScoreError(format!(
                    "llama.cpp score logits row does not fit i32: {error}"
                ))
            })?;
            let target_token =
                usize::try_from(sequence[source_start + offset + 1]).map_err(|error| {
                    ModelRuntimeError::ScoreError(format!(
                        "llama.cpp score target token does not fit usize: {error}"
                    ))
                })?;
            let logits = context.get_logits_ith(logits_index);
            token_logprobs.push(logprob_for_target(logits, target_token)?);
        }

        source_start += chunk_len;
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

#[cfg(feature = "llama-cpp-runtime-engine")]
fn score_tokens(
    native: &super::context::NativeLlamaCppBackend,
    sequence: &[u32],
) -> Result<Vec<llama_cpp_2::token::LlamaToken>, ModelRuntimeError> {
    let n_vocab = native.model.n_vocab();
    if n_vocab <= 0 {
        return Err(ModelRuntimeError::ScoreError(format!(
            "llama.cpp model reports invalid n_vocab {n_vocab}"
        )));
    }
    let n_vocab = u32::try_from(n_vocab).map_err(|error| {
        ModelRuntimeError::ScoreError(format!("llama.cpp n_vocab does not fit u32: {error}"))
    })?;

    sequence
        .iter()
        .copied()
        .map(|token_id| {
            if token_id >= n_vocab {
                return Err(ModelRuntimeError::ScoreError(format!(
                    "llama.cpp score token id {token_id} is outside vocab size {n_vocab}"
                )));
            }
            let token_id = i32::try_from(token_id).map_err(|error| {
                ModelRuntimeError::ScoreError(format!(
                    "llama.cpp score token id does not fit i32: {error}"
                ))
            })?;
            Ok(llama_cpp_2::token::LlamaToken::new(token_id))
        })
        .collect()
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn logprob_for_target(logits: &[f32], target_token: usize) -> Result<f32, ModelRuntimeError> {
    if target_token >= logits.len() {
        return Err(ModelRuntimeError::ScoreError(format!(
            "llama.cpp score target token {target_token} is outside logits width {}",
            logits.len()
        )));
    }
    for (index, logit) in logits.iter().enumerate() {
        if !logit.is_finite() {
            return Err(ModelRuntimeError::ScoreError(format!(
                "llama.cpp score logit at index {index} is not finite"
            )));
        }
    }

    let max_logit = logits
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, |left, right| left.max(right));
    if !max_logit.is_finite() {
        return Err(ModelRuntimeError::ScoreError(
            "llama.cpp score logits did not contain a finite maximum".to_string(),
        ));
    }

    let exp_sum = logits
        .iter()
        .map(|logit| f64::from(*logit - max_logit).exp())
        .sum::<f64>();
    if !exp_sum.is_finite() || exp_sum <= 0.0 {
        return Err(ModelRuntimeError::ScoreError(
            "llama.cpp score logits produced an invalid logsumexp denominator".to_string(),
        ));
    }

    let logsumexp = f64::from(max_logit) + exp_sum.ln();
    let logprob = f64::from(logits[target_token]) - logsumexp;
    if !logprob.is_finite() {
        return Err(ModelRuntimeError::ScoreError(
            "llama.cpp score produced a non-finite log probability".to_string(),
        ));
    }
    if logprob > 1.0e-4 {
        return Err(ModelRuntimeError::ScoreError(format!(
            "llama.cpp score produced positive log probability {logprob}"
        )));
    }
    Ok((logprob as f32).min(0.0))
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn embed_native(
    native: std::sync::Arc<super::context::NativeLlamaCppBackend>,
    quantization: KvQuantSupport,
    text: &str,
) -> Result<Embedding, ModelRuntimeError> {
    use llama_cpp_2::model::AddBos;

    let tokens = native
        .model
        .str_to_token(text, AddBos::Always)
        .map_err(|error| {
            ModelRuntimeError::EmbedError(format!(
                "llama.cpp embedding tokenization failed: {error}"
            ))
        })?;
    if tokens.is_empty() {
        return Err(ModelRuntimeError::EmbedError(
            "llama.cpp embedding tokenization produced no tokens".to_string(),
        ));
    }

    let mut context = native.new_context(quantization)?;
    if tokens.len() > context.n_ctx() as usize {
        return Err(ModelRuntimeError::EmbedError(format!(
            "llama.cpp embedding text too long: {} tokens exceed n_ctx {}",
            tokens.len(),
            context.n_ctx()
        )));
    }

    let mut batch = embedding_batch(&tokens)?;
    match context.decode(&mut batch) {
        Ok(()) => embedding_from_context(&context),
        Err(decode_error) => {
            let mut context = native.new_context(quantization)?;
            let mut batch = embedding_batch(&tokens)?;
            context.encode(&mut batch).map_err(|encode_error| {
                ModelRuntimeError::EmbedError(format!(
                    "llama.cpp embedding decode failed ({decode_error}); encode fallback also failed: {encode_error}"
                ))
            })?;
            embedding_from_context(&context)
        }
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn embedding_batch<'a>(
    tokens: &'a [llama_cpp_2::token::LlamaToken],
) -> Result<llama_cpp_2::llama_batch::LlamaBatch<'a>, ModelRuntimeError> {
    let mut batch = llama_cpp_2::llama_batch::LlamaBatch::new(tokens.len(), 1);
    batch.add_sequence(tokens, 0, true).map_err(|error| {
        ModelRuntimeError::EmbedError(format!(
            "failed to add embedding tokens to llama.cpp batch: {error}"
        ))
    })?;
    Ok(batch)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn embedding_from_context(
    context: &llama_cpp_2::context::LlamaContext<'_>,
) -> Result<Embedding, ModelRuntimeError> {
    let vector = match context.embeddings_seq_ith(0) {
        Ok(embedding) => embedding.to_vec(),
        Err(llama_cpp_2::EmbeddingsError::NotEnabled) => {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: EMBEDDING_CAPABILITY.to_string(),
                adapter: ADAPTER.to_string(),
            });
        }
        Err(llama_cpp_2::EmbeddingsError::NonePoolType) => context
            .embeddings_ith(-1)
            .map_err(|error| {
                ModelRuntimeError::EmbedError(format!(
                    "llama.cpp token embedding extraction failed: {error}"
                ))
            })?
            .to_vec(),
        Err(error) => {
            return Err(ModelRuntimeError::EmbedError(format!(
                "llama.cpp pooled embedding extraction failed: {error}"
            )));
        }
    };
    validate_embedding_vector(vector, context.model.n_embd())
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn validate_embedding_vector(
    vector: Vec<f32>,
    expected_dim: i32,
) -> Result<Embedding, ModelRuntimeError> {
    let expected_dim = usize::try_from(expected_dim).map_err(|error| {
        ModelRuntimeError::EmbedError(format!("llama.cpp n_embd does not fit usize: {error}"))
    })?;
    if vector.is_empty() {
        return Err(ModelRuntimeError::EmbedError(
            "llama.cpp embedding vector is empty".to_string(),
        ));
    }
    if vector.len() != expected_dim {
        return Err(ModelRuntimeError::EmbedError(format!(
            "llama.cpp embedding dimension mismatch: got {}, expected {expected_dim}",
            vector.len()
        )));
    }
    for (index, value) in vector.iter().enumerate() {
        if !value.is_finite() {
            return Err(ModelRuntimeError::EmbedError(format!(
                "llama.cpp embedding value at index {index} is not finite"
            )));
        }
    }
    let norm_sq = vector.iter().map(|value| value * value).sum::<f32>();
    if !norm_sq.is_finite() || norm_sq <= 0.0 {
        return Err(ModelRuntimeError::EmbedError(
            "llama.cpp embedding vector has invalid norm".to_string(),
        ));
    }
    Ok(Embedding { vector })
}
