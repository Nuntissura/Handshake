use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use handshake_core::model_runtime::{
    KvCacheOps, KvCachePolicy, KvCacheStats, KvPrefixHandle, KvQuantSupport, LoadSpec,
    ModelCapabilities, ModelRuntimeError, ProviderKind, RuntimeKind, SamplingParams,
};

#[derive(Default)]
struct ValidatingKvCache {
    quantization: Mutex<KvQuantSupport>,
    prefixes: Mutex<HashMap<uuid::Uuid, [u8; 32]>>,
}

impl KvCacheOps for ValidatingKvCache {
    fn quantization(&self) -> KvQuantSupport {
        *self.quantization.lock().unwrap()
    }

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        if level == KvQuantSupport::Q4Q8Mix {
            return Err(ModelRuntimeError::CapabilityNotSupported {
                capability: "kv_cache.q4_q8_mix".to_string(),
                adapter: "validating-test-cache".to_string(),
            });
        }
        *self.quantization.lock().unwrap() = level;
        Ok(())
    }

    fn occupancy(&self) -> KvCacheStats {
        let prefixes = self.prefixes.lock().unwrap();
        KvCacheStats {
            bytes_used: (prefixes.len() as u64) * 128,
            bytes_capacity: 1024,
            prefix_cache_entries: prefixes.len() as u32,
            prefix_cache_hit_count: 7,
            prefix_cache_miss_count: 2,
            quant_level_current: self.quantization(),
        }
    }

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        let handle = KvPrefixHandle::from_tokens(prefix_tokens)?;
        self.prefixes
            .lock()
            .unwrap()
            .insert(handle.prefix_id(), *handle.content_hash());
        Ok(handle)
    }

    fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        let prefixes = self.prefixes.lock().unwrap();
        match prefixes.get(&handle.prefix_id()) {
            Some(stored_hash) if stored_hash == handle.content_hash() => Ok(()),
            Some(_) => Err(ModelRuntimeError::KvCacheError(
                "prefix handle content_hash mismatch".to_string(),
            )),
            None => Err(ModelRuntimeError::KvCacheError(
                "unknown prefix handle".to_string(),
            )),
        }
    }

    fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        self.prefixes.lock().unwrap().remove(&handle.prefix_id());
        Ok(())
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        self.prefixes.lock().unwrap().clear();
        Ok(())
    }
}

#[test]
fn model_runtime_kv_cache_tests_ops_are_object_safe_and_policy_variants_compile() {
    fn assert_object_safe(_: Box<dyn KvCacheOps>) {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Box<dyn KvCacheOps>>();
    assert_object_safe(Box::new(ValidatingKvCache::default()));

    let default_policy = KvCachePolicy::Default {
        quant: KvQuantSupport::Q4,
        prefix_cache_ttl_seconds: 60,
        max_bytes: Some(4096),
    };
    let disabled_policy = KvCachePolicy::Disabled;
    let custom_policy = KvCachePolicy::Custom(Box::new(ValidatingKvCache::default()));

    assert!(matches!(
        default_policy,
        KvCachePolicy::Default {
            quant: KvQuantSupport::Q4,
            prefix_cache_ttl_seconds: 60,
            max_bytes: Some(4096)
        }
    ));
    assert!(matches!(disabled_policy, KvCachePolicy::Disabled));
    assert!(matches!(custom_policy, KvCachePolicy::Custom(_)));

    let _load_spec = LoadSpec {
        artifact_path: PathBuf::from("models/tiny.gguf"),
        sha256_expected: "abc123".to_string(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: default_policy,
        declared_capabilities: ModelCapabilities::default(),
        provider: ProviderKind::Local,
        engine_origin: None,
        external_engine_import: None,
    };
}

#[test]
fn model_runtime_kv_cache_tests_prefix_handles_are_v7_and_reject_tampered_hashes() {
    let cache = ValidatingKvCache::default();
    let handle = cache.prefix_commit(&[10, 20, 30]).unwrap();

    assert_eq!(handle.prefix_id().get_version_num(), 7);
    assert_eq!(handle.token_count(), 3);
    assert_eq!(
        *handle.content_hash(),
        KvPrefixHandle::content_hash_for_tokens(&[10, 20, 30])
    );
    cache.prefix_restore(&handle).unwrap();

    let mut tampered_hash = *handle.content_hash();
    tampered_hash[0] ^= 0xff;
    let tampered =
        KvPrefixHandle::from_parts(handle.prefix_id(), tampered_hash, handle.token_count())
            .unwrap();

    let error = cache.prefix_restore(&tampered).unwrap_err();
    assert!(
        matches!(error, ModelRuntimeError::KvCacheError(_)),
        "tampered prefix restore must fail with a typed KV-cache error"
    );
}

#[test]
fn model_runtime_kv_cache_tests_prefix_hashes_are_scope_order_and_length_sensitive() {
    let unscoped = KvPrefixHandle::content_hash_for_tokens(&[1, 2, 3]);
    let scoped_a =
        KvPrefixHandle::content_hash_for_scope_and_tokens(b"model-a/cache-1", &[1, 2, 3]);
    let scoped_b =
        KvPrefixHandle::content_hash_for_scope_and_tokens(b"model-b/cache-1", &[1, 2, 3]);
    let different_order =
        KvPrefixHandle::content_hash_for_scope_and_tokens(b"model-a/cache-1", &[1, 3, 2]);
    let different_length =
        KvPrefixHandle::content_hash_for_scope_and_tokens(b"model-a/cache-1", &[1, 2, 3, 0]);

    assert_ne!(unscoped, scoped_a);
    assert_ne!(scoped_a, scoped_b);
    assert_ne!(scoped_a, different_order);
    assert_ne!(scoped_a, different_length);

    let scoped_handle = KvPrefixHandle::from_scoped_tokens(b"model-a/cache-1", &[1, 2, 3])
        .expect("scoped prefix handle");
    assert_eq!(
        *scoped_handle.content_hash(),
        KvPrefixHandle::content_hash_for_scope_and_tokens(b"model-a/cache-1", &[1, 2, 3])
    );

    assert!(KvPrefixHandle::from_parts(uuid::Uuid::nil(), scoped_a, 3).is_err());
    let v4_shaped_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000")
        .expect("parse v4-shaped uuid");
    assert!(KvPrefixHandle::from_parts(v4_shaped_id, scoped_a, 3).is_err());
}

#[test]
fn model_runtime_kv_cache_tests_quantization_stats_and_eviction_are_engine_agnostic() {
    let cache = Arc::new(ValidatingKvCache::default());

    assert_eq!(cache.quantization(), KvQuantSupport::None);
    cache.set_quantization(KvQuantSupport::Q4).unwrap();
    assert_eq!(cache.quantization(), KvQuantSupport::Q4);
    assert!(cache.set_quantization(KvQuantSupport::Q4Q8Mix).is_err());

    let first = cache.prefix_commit(&[1, 2]).unwrap();
    let second = cache.prefix_commit(&[3, 4, 5]).unwrap();
    let stats = cache.occupancy();
    assert_eq!(stats.prefix_cache_entries, 2);
    assert_eq!(stats.quant_level_current, KvQuantSupport::Q4);
    assert!(stats.bytes_used <= stats.bytes_capacity);

    cache.prefix_evict(first).unwrap();
    assert_eq!(cache.occupancy().prefix_cache_entries, 1);
    cache.prefix_restore(&second).unwrap();
    cache.evict_all().unwrap();
    assert_eq!(cache.occupancy().prefix_cache_entries, 0);
}

#[test]
fn model_runtime_kv_cache_tests_public_surface_is_engine_agnostic() {
    let source = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/model_runtime/kv_cache.rs"),
    )
    .expect("read kv_cache.rs");
    let normalized = source.to_ascii_lowercase();

    for banned in ["llama_cpp_2::", "candle_core::", "candle_transformers::"] {
        assert!(
            !normalized.contains(banned),
            "kv_cache surface must not leak engine-specific type `{banned}`"
        );
    }
}
