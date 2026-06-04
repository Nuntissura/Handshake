use std::{fs, path::Path};

use handshake_core::model_runtime::{
    llama_cpp::{
        gguf_loader::{LlamaCppContextLoadConfig, LlamaCppLoadConfig},
        LlamaCppRuntime,
    },
    KvCachePolicy, KvQuantSupport, LoadSpec, ModelCapabilities, ModelId, ModelRuntime,
    ModelRuntimeError, ProviderKind, RuntimeKind, SamplingParams,
};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn score_and_embed_unknown_model_report_not_loaded_not_placeholder() {
    let runtime = LlamaCppRuntime::default();
    let model_id = ModelId::new_v7();

    let score_err = runtime
        .score(model_id, vec![1, 2])
        .await
        .expect_err("missing model score must fail");
    assert_not_placeholder(&score_err);
    assert!(
        score_err.to_string().contains("not loaded"),
        "missing score model should report not loaded, got {score_err}"
    );

    let embed_err = runtime
        .embed(model_id, "hello")
        .await
        .expect_err("missing model embed must fail");
    assert_not_placeholder(&embed_err);
    assert!(
        embed_err.to_string().contains("not loaded"),
        "missing embed model should report not loaded, got {embed_err}"
    );
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_score_returns_finite_negative_mean_when_runtime_available() {
    let Some(path) = test_gguf_path() else {
        return;
    };
    let mut runtime = LlamaCppRuntime::default();
    let model_id = runtime
        .load(load_spec(&path, sha256_file(&path)))
        .await
        .expect("native-enabled build loads representative GGUF");

    let score = runtime
        .score(model_id, vec![1, 2, 3, 4])
        .await
        .expect("score computes token log probabilities");

    assert_eq!(score.token_logprobs.len(), 3);
    assert!(
        score.token_logprobs.iter().all(|value| value.is_finite()),
        "token logprobs must be finite: {:?}",
        score.token_logprobs
    );
    assert!(
        score.token_logprobs.iter().all(|value| *value <= 0.0),
        "log probabilities must be non-positive: {:?}",
        score.token_logprobs
    );
    let expected_mean =
        score.token_logprobs.iter().sum::<f32>() / score.token_logprobs.len() as f32;
    assert_close(score.mean_logprob, expected_mean, 1.0e-5);

    runtime.unload(model_id).await.expect("unload model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_score_is_deterministic_and_handles_small_batch_chunks() {
    let Some(path) = test_gguf_path() else {
        return;
    };
    let load_config = LlamaCppLoadConfig {
        context: LlamaCppContextLoadConfig {
            n_ctx: 64,
            n_batch: 2,
            ..LlamaCppContextLoadConfig::default()
        },
        ..LlamaCppLoadConfig::default()
    };
    let mut runtime = LlamaCppRuntime::with_load_config(KvCachePolicy::default(), load_config);
    let model_id = runtime
        .load(load_spec(&path, sha256_file(&path)))
        .await
        .expect("native-enabled build loads representative GGUF");
    let sequence = vec![1, 2, 3, 4, 5, 6];

    let first = runtime
        .score(model_id, sequence.clone())
        .await
        .expect("first chunked score succeeds");
    let second = runtime
        .score(model_id, sequence)
        .await
        .expect("second chunked score succeeds");

    assert_eq!(first.token_logprobs.len(), 5);
    assert_eq!(second.token_logprobs.len(), 5);
    assert_vec_close(&first.token_logprobs, &second.token_logprobs, 1.0e-5);
    assert_close(first.mean_logprob, second.mean_logprob, 1.0e-5);

    runtime.unload(model_id).await.expect("unload model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_score_rejects_invalid_token_id_before_decode() {
    let Some(path) = test_gguf_path() else {
        return;
    };
    let mut runtime = LlamaCppRuntime::default();
    let model_id = runtime
        .load(load_spec(&path, sha256_file(&path)))
        .await
        .expect("native-enabled build loads representative GGUF");

    let err = runtime
        .score(model_id, vec![1, u32::MAX])
        .await
        .expect_err("invalid token id must fail before llama.cpp decode");

    assert_not_placeholder(&err);
    assert!(
        err.to_string().contains("token"),
        "invalid score token should mention token id, got {err}"
    );

    runtime.unload(model_id).await.expect("unload model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_embed_returns_finite_deterministic_vector_when_runtime_available() {
    let Some(path) = test_gguf_path() else {
        return;
    };
    let mut runtime = LlamaCppRuntime::default();
    let model_id = runtime
        .load(load_spec(&path, sha256_file(&path)))
        .await
        .expect("native-enabled build loads representative GGUF");

    let first = runtime
        .embed(model_id, "local model runtime embedding smoke test")
        .await
        .expect("first embedding succeeds");
    let second = runtime
        .embed(model_id, "local model runtime embedding smoke test")
        .await
        .expect("second embedding succeeds");

    assert!(
        !first.vector.is_empty(),
        "embedding vector must not be empty"
    );
    assert!(
        first.vector.iter().all(|value| value.is_finite()),
        "embedding vector must be finite"
    );
    assert_eq!(
        first.vector.len(),
        second.vector.len(),
        "embedding dimension must be stable"
    );
    assert_vec_close(&first.vector, &second.vector, 1.0e-5);
    assert!(
        cosine_similarity(&first.vector, &second.vector) > 0.999,
        "same prompt embeddings should have near-identical cosine similarity"
    );

    runtime.unload(model_id).await.expect("unload model");
}

#[cfg(feature = "llama-cpp-runtime-engine")]
#[tokio::test]
async fn env_gated_embed_without_embedding_context_returns_capability_not_supported() {
    let Some(path) = test_gguf_path() else {
        return;
    };
    let load_config = LlamaCppLoadConfig {
        context: LlamaCppContextLoadConfig {
            embeddings: false,
            ..LlamaCppContextLoadConfig::default()
        },
        ..LlamaCppLoadConfig::default()
    };
    let mut runtime = LlamaCppRuntime::with_load_config(KvCachePolicy::default(), load_config);
    let model_id = runtime
        .load(load_spec(&path, sha256_file(&path)))
        .await
        .expect("native-enabled build loads representative GGUF");

    let err = runtime
        .embed(model_id, "embedding disabled")
        .await
        .expect_err("embedding-disabled context must fail closed");

    match err {
        ModelRuntimeError::CapabilityNotSupported {
            capability,
            adapter,
        } => {
            assert!(
                capability.contains("embedding"),
                "capability should name embedding support, got {capability}"
            );
            assert_eq!(adapter, "llama_cpp");
        }
        other => panic!("expected embedding capability error, got {other:?}"),
    }

    runtime.unload(model_id).await.expect("unload model");
}

fn assert_not_placeholder(error: &ModelRuntimeError) {
    let message = error.to_string();
    assert!(
        !message.contains("not implemented")
            && !message.contains("mt072_scaffold")
            && !message.contains("placeholder"),
        "score/embed must not return placeholder errors: {error}"
    );
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn test_gguf_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn load_spec(artifact_path: &Path, sha256_expected: String) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected,
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant: KvQuantSupport::Q4,
            prefix_cache_ttl_seconds: 0,
            max_bytes: None,
        },
        declared_capabilities: capabilities(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn capabilities() -> ModelCapabilities {
    ModelCapabilities {
        supports_lora: true,
        supports_kv_prefix_cache: true,
        supports_kv_quantization: KvQuantSupport::Q4,
        supports_activation_steering: false,
        supports_subquadratic: false,
        supports_speculative_draft: true,
        supports_eagle3: false,
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read GGUF fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn assert_close(actual: f32, expected: f32, epsilon: f32) {
    assert!(
        (actual - expected).abs() <= epsilon,
        "expected {actual} to be within {epsilon} of {expected}"
    );
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn assert_vec_close(actual: &[f32], expected: &[f32], epsilon: f32) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (*actual - *expected).abs() <= epsilon,
            "vector mismatch at {index}: {actual} vs {expected}"
        );
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    assert_eq!(left.len(), right.len());
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    dot / (left_norm * right_norm)
}
