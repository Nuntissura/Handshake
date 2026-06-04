use std::{fs, path::Path};

use futures::StreamExt;
#[cfg(not(feature = "llama-cpp-runtime-engine"))]
use handshake_core::model_runtime::llama_cpp::LLAMA_CPP_NATIVE_FEATURE_DISABLED;
use handshake_core::model_runtime::{
    llama_cpp::LlamaCppRuntime, CancellationToken, GenPrompt, GenerateRequest, KvCacheOps,
    KvCachePolicy, KvPrefixHandle, KvQuantSupport, LoadSpec, ModelCapabilities, ModelRuntime,
    ProviderKind, RuntimeKind, SamplingParams,
};
use sha2::{Digest, Sha256};

#[cfg(feature = "llama-cpp-runtime-engine")]
static LLAMA_CPP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test]
async fn env_gated_llama_cpp_kv_cache_quantization_prefix_restore_and_evict() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };
    let mut runtime = LlamaCppRuntime::default();
    let sha256 = sha256_file(&path);

    #[cfg(not(feature = "llama-cpp-runtime-engine"))]
    {
        let err = runtime
            .load(load_spec(&path, sha256))
            .await
            .expect_err("native-disabled builds validate then reject the real fixture");

        assert!(err.to_string().contains(LLAMA_CPP_NATIVE_FEATURE_DISABLED));
    }

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let _guard = llama_cpp_test_guard();
        let model_id = runtime
            .load(load_spec(&path, sha256))
            .await
            .expect("native-enabled build loads representative GGUF");

        let public_handle = runtime
            .kv_cache(model_id)
            .expect("loaded llama.cpp model exposes a public KV cache handle");
        assert!(
            public_handle.as_str().starts_with("llama_cpp:"),
            "public handle should be scoped to the llama.cpp adapter"
        );

        let cache = runtime
            .llama_cpp_kv_cache(model_id)
            .expect("loaded llama.cpp model exposes native KV cache ops");
        assert_eq!(cache.quantization(), KvQuantSupport::Q4);

        cache
            .set_quantization(KvQuantSupport::Q8)
            .expect("q8 quantization is supported");
        assert_eq!(cache.quantization(), KvQuantSupport::Q8);

        cache
            .set_quantization(KvQuantSupport::Q4Q8Mix)
            .expect("mixed q4/q8 quantization is supported");
        assert_eq!(cache.quantization(), KvQuantSupport::Q4Q8Mix);

        let prefix = cache
            .prefix_commit(&[1, 2, 3, 4])
            .expect("prefix commit captures native KV state");
        let committed_stats = cache.occupancy();
        assert_eq!(committed_stats.prefix_cache_entries, 1);
        assert!(committed_stats.bytes_used > 0);
        assert!(committed_stats.bytes_capacity >= committed_stats.bytes_used);
        assert_eq!(committed_stats.quant_level_current, KvQuantSupport::Q4Q8Mix);

        cache
            .prefix_restore(&prefix)
            .expect("known prefix restores cleanly");
        assert_eq!(cache.occupancy().prefix_cache_hit_count, 1);

        let mut tampered_hash = *prefix.content_hash();
        tampered_hash[0] ^= 0xff;
        let tampered =
            KvPrefixHandle::from_parts(prefix.prefix_id(), tampered_hash, prefix.token_count())
                .expect("tampered v7 handle shape is valid");
        let error = cache
            .prefix_restore(&tampered)
            .expect_err("tampered prefix content hash is rejected");
        assert!(
            error.to_string().contains("content_hash"),
            "tamper error should name content_hash: {error}"
        );
        assert_eq!(cache.occupancy().prefix_cache_miss_count, 1);

        cache
            .prefix_evict(prefix.clone())
            .expect("prefix eviction succeeds");
        assert_eq!(cache.occupancy().prefix_cache_entries, 0);

        let _second = cache
            .prefix_commit(&[5, 6])
            .expect("second prefix commit succeeds");
        cache.evict_all().expect("evict_all succeeds");
        assert_eq!(cache.occupancy().prefix_cache_entries, 0);

        runtime.unload(model_id).await.expect("unload model");
    }
}

#[tokio::test]
async fn env_gated_llama_cpp_kv_cache_rejects_unsupported_quantization() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let _guard = llama_cpp_test_guard();
        let mut runtime = LlamaCppRuntime::default();
        let model_id = runtime
            .load(load_spec_with_kv_policy(
                &path,
                sha256_file(&path),
                KvQuantSupport::None,
                KvQuantSupport::None,
                300,
                None,
            ))
            .await
            .expect("native-enabled build loads representative GGUF without KV quant support");
        let cache = runtime
            .llama_cpp_kv_cache(model_id)
            .expect("loaded llama.cpp model exposes native KV cache ops");

        assert_eq!(cache.quantization(), KvQuantSupport::None);
        let err = cache
            .set_quantization(KvQuantSupport::Q4)
            .expect_err("declared no-quant model rejects q4 KV quantization");
        assert!(
            err.to_string().contains("Capability not supported")
                || err.to_string().contains("capability"),
            "unsupported quantization should be a capability error: {err}"
        );
        assert_eq!(cache.quantization(), KvQuantSupport::None);

        cache
            .set_quantization(KvQuantSupport::None)
            .expect("no-quant mode remains supported");
        runtime.unload(model_id).await.expect("unload model");
    }
}

#[tokio::test]
async fn env_gated_llama_cpp_kv_cache_quant_switch_flushes_prefixes() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let _guard = llama_cpp_test_guard();
        let mut runtime = LlamaCppRuntime::default();
        let model_id = runtime
            .load(load_spec(&path, sha256_file(&path)))
            .await
            .expect("native-enabled build loads representative GGUF");
        let cache = runtime
            .llama_cpp_kv_cache(model_id)
            .expect("loaded llama.cpp model exposes native KV cache ops");

        let prefix = cache
            .prefix_commit(&[1, 2, 3])
            .expect("prefix commit captures native KV state");
        assert_eq!(cache.occupancy().prefix_cache_entries, 1);

        cache
            .set_quantization(KvQuantSupport::Q8)
            .expect("q8 quantization is supported");
        assert_eq!(cache.occupancy().prefix_cache_entries, 0);
        let err = cache
            .prefix_restore(&prefix)
            .expect_err("quantization switch invalidates old prefix snapshots");
        assert!(
            err.to_string().contains("unknown prefix"),
            "old prefix should be gone after quantization switch: {err}"
        );

        runtime.unload(model_id).await.expect("unload model");
    }
}

#[tokio::test]
async fn env_gated_llama_cpp_kv_cache_max_bytes_keeps_prefix_store_bounded() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let _guard = llama_cpp_test_guard();
        let mut runtime = LlamaCppRuntime::default();
        let model_id = runtime
            .load(load_spec_with_kv_policy(
                &path,
                sha256_file(&path),
                KvQuantSupport::Q4,
                KvQuantSupport::Q4Q8Mix,
                300,
                Some(1),
            ))
            .await
            .expect("native-enabled build loads representative GGUF");
        let cache = runtime
            .llama_cpp_kv_cache(model_id)
            .expect("loaded llama.cpp model exposes native KV cache ops");

        let first = cache
            .prefix_commit(&[1, 2])
            .expect("first prefix commit captures native KV state");
        let second = cache
            .prefix_commit(&[3, 4])
            .expect("second prefix commit captures native KV state");

        let stats = cache.occupancy();
        assert!(
            stats.prefix_cache_entries <= 1,
            "max_bytes should bound retained prefix entries even when one snapshot exceeds the budget: {stats:?}"
        );
        cache
            .prefix_restore(&second)
            .expect("newest prefix remains restorable");
        let err = cache
            .prefix_restore(&first)
            .expect_err("older prefix is evicted by bounded LRU policy");
        assert!(
            err.to_string().contains("unknown prefix"),
            "oldest prefix should be evicted under max_bytes pressure: {err}"
        );

        runtime.unload(model_id).await.expect("unload model");
    }
}

#[tokio::test]
async fn env_gated_llama_cpp_generation_rejects_mismatched_prefix_handle() {
    let Some(path) = std::env::var_os("HANDSHAKE_TEST_GGUF_PATH").map(std::path::PathBuf::from)
    else {
        return;
    };

    #[cfg(feature = "llama-cpp-runtime-engine")]
    {
        let _guard = llama_cpp_test_guard();
        let mut runtime = LlamaCppRuntime::default();
        let model_id = runtime
            .load(load_spec(&path, sha256_file(&path)))
            .await
            .expect("native-enabled build loads representative GGUF");
        let cache = runtime
            .llama_cpp_kv_cache(model_id)
            .expect("loaded llama.cpp model exposes native KV cache ops");
        let prefix = cache
            .prefix_commit(&[1, 2])
            .expect("prefix commit captures native KV state");

        let mut stream = runtime.generate(GenerateRequest {
            id: model_id,
            prompt: GenPrompt::from("Hello"),
            sampling: SamplingParams::default(),
            lora_overrides: Vec::new(),
            steering_overrides: Vec::new(),
            kv_prefix_handle: Some(prefix),
            cancel: CancellationToken::new(),
            max_tokens: 2,
            stop_sequences: Vec::new(),
            speculative_mode: None,
            structured_decoding: None,
        });

        let err = stream
            .next()
            .await
            .expect("mismatched prefix emits a typed error")
            .expect_err("mismatched prefix should not generate");
        assert!(
            err.to_string().contains("content_hash"),
            "mismatched prompt prefix should name content_hash: {err}"
        );

        runtime.unload(model_id).await.expect("unload model");
    }
}

#[cfg(feature = "llama-cpp-runtime-engine")]
fn llama_cpp_test_guard() -> std::sync::MutexGuard<'static, ()> {
    LLAMA_CPP_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn load_spec(artifact_path: &Path, sha256_expected: String) -> LoadSpec {
    load_spec_with_kv_policy(
        artifact_path,
        sha256_expected,
        KvQuantSupport::Q4,
        KvQuantSupport::Q4Q8Mix,
        300,
        None,
    )
}

fn load_spec_with_kv_policy(
    artifact_path: &Path,
    sha256_expected: String,
    quant: KvQuantSupport,
    supports_kv_quantization: KvQuantSupport,
    prefix_cache_ttl_seconds: u64,
    max_bytes: Option<u64>,
) -> LoadSpec {
    LoadSpec {
        artifact_path: artifact_path.to_path_buf(),
        sha256_expected,
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::Default {
            quant,
            prefix_cache_ttl_seconds,
            max_bytes,
        },
        declared_capabilities: ModelCapabilities {
            supports_lora: true,
            supports_kv_prefix_cache: true,
            supports_kv_quantization,
            supports_activation_steering: false,
            supports_subquadratic: false,
            supports_speculative_draft: true,
            supports_eagle3: false,
        },
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    }
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read fixture");
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
