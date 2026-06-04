//! MT-093 — INF-2 KV cache technique surface tests.
//!
//! Exercises `kv_cache_technique::{set_quantization, prefix_commit,
//! prefix_restore, evict_all, occupancy}` against a fake adapter that
//! impls `KvCacheOps` via the new `KvCacheHandle::with_ops` wiring.
//! Mirrors the structural pattern of `lora_hotswap_techniques_tests`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures_util::stream;
use handshake_core::{
    flight_recorder::fr_event_registry::FrEventId,
    model_runtime::{
        techniques::kv_cache_technique::{
            self, FR_EVT_LLM_INFER_KV_EVICT, FR_EVT_LLM_INFER_KV_PREFIX_COMMIT,
            FR_EVT_LLM_INFER_KV_PREFIX_RESTORE, FR_EVT_LLM_INFER_KV_SET_QUANTIZATION,
        },
        CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCacheOps, KvCacheStats,
        KvPrefixHandle, KvQuantSupport, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
        ModelRuntime, ModelRuntimeError, Score, SteeringHookHandle, TokenStream,
    },
};
use uuid::Uuid;

#[test]
fn kv_cache_set_quantization_dispatches_and_returns_canonical_event_type() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::Q4));
    let runtime = RecordingRuntime::new(
        "test_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4Q8Mix,
            ..Default::default()
        },
        stack.clone(),
    );

    let receipt = kv_cache_technique::set_quantization(&runtime, model_id, KvQuantSupport::Q8)
        .expect("set_quantization must dispatch on a capable adapter");
    assert_eq!(receipt.event_type, FR_EVT_LLM_INFER_KV_SET_QUANTIZATION);
    assert_eq!(receipt.previous_quantization, KvQuantSupport::Q4);
    assert_eq!(receipt.current_quantization, KvQuantSupport::Q8);
    assert_eq!(stack.set_calls(), 1);
}

#[test]
fn kv_cache_set_quantization_fails_closed_when_quantization_unsupported() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::None));
    let runtime = RecordingRuntime::new(
        "test_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::None,
            ..Default::default()
        },
        stack.clone(),
    );

    let err = kv_cache_technique::set_quantization(&runtime, model_id, KvQuantSupport::Q8)
        .expect_err("supports_kv_quantization=None must reject the technique surface");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
            if capability == "kv_cache_quantization"
        ),
        "expected CapabilityNotSupported{{kv_cache_quantization}}, got {err:?}"
    );
    assert_eq!(
        stack.set_calls(),
        0,
        "capability gate must precede mutation"
    );
}

#[test]
fn kv_cache_prefix_commit_and_restore_round_trip() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::Q4));
    let runtime = RecordingRuntime::new(
        "test_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4,
            ..Default::default()
        },
        stack.clone(),
    );

    let prefix_tokens = vec![1u32, 2, 3, 4];
    let commit = kv_cache_technique::prefix_commit(&runtime, model_id, &prefix_tokens)
        .expect("prefix_commit must dispatch on a capable adapter");
    assert_eq!(commit.event_type, FR_EVT_LLM_INFER_KV_PREFIX_COMMIT);
    assert_eq!(commit.prefix_handle.token_count(), 4);

    let restore = kv_cache_technique::prefix_restore(&runtime, model_id, &commit.prefix_handle)
        .expect("prefix_restore must dispatch on the committed handle");
    assert_eq!(restore.event_type, FR_EVT_LLM_INFER_KV_PREFIX_RESTORE);
    assert_eq!(restore.prefix_handle, commit.prefix_handle);

    let occupancy = kv_cache_technique::occupancy(&runtime, model_id)
        .expect("occupancy must dispatch on a capable adapter");
    assert_eq!(occupancy.occupancy.prefix_cache_entries, 1);
    assert_eq!(occupancy.occupancy.prefix_cache_hit_count, 1);
}

#[test]
fn kv_cache_prefix_commit_fails_closed_when_prefix_cache_unsupported() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::None));
    let runtime = RecordingRuntime::new(
        "test_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: false,
            supports_kv_quantization: KvQuantSupport::Q4,
            ..Default::default()
        },
        stack.clone(),
    );

    let err = kv_cache_technique::prefix_commit(&runtime, model_id, &[1u32, 2, 3])
        .expect_err("supports_kv_prefix_cache=false must reject commit");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
            if capability == "kv_cache_prefix"
        ),
        "expected CapabilityNotSupported{{kv_cache_prefix}}, got {err:?}"
    );
    assert_eq!(stack.commit_calls(), 0);
}

#[test]
fn kv_cache_evict_all_returns_canonical_event_type_and_clears_state() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::Q4));
    let runtime = RecordingRuntime::new(
        "test_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4,
            ..Default::default()
        },
        stack.clone(),
    );

    kv_cache_technique::prefix_commit(&runtime, model_id, &[10u32, 20])
        .expect("commit must succeed before evict_all test");

    let receipt = kv_cache_technique::evict_all(&runtime, model_id)
        .expect("evict_all must dispatch on a capable adapter");
    assert_eq!(receipt.event_type, FR_EVT_LLM_INFER_KV_EVICT);
    assert_eq!(receipt.previous_occupancy.prefix_cache_entries, 1);
    assert_eq!(receipt.current_occupancy.prefix_cache_entries, 0);
    assert_eq!(stack.evict_all_calls(), 1);
}

#[test]
fn kv_cache_occupancy_fails_closed_when_neither_capability_is_declared() {
    // Genuinely KV-less adapter (e.g., a remote BYOK endpoint).
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::None));
    let runtime = RecordingRuntime::new(
        "kvless_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: false,
            supports_kv_quantization: KvQuantSupport::None,
            ..Default::default()
        },
        stack.clone(),
    );

    let err = kv_cache_technique::occupancy(&runtime, model_id)
        .expect_err("kv-less adapter must reject occupancy too");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
            if capability == "kv_cache"
        ),
        "expected CapabilityNotSupported{{kv_cache}}, got {err:?}"
    );
}

#[test]
fn kv_cache_occupancy_allowed_when_only_quantization_is_declared() {
    // Quantization-only adapter (no prefix cache). Occupancy should
    // still return — telemetry is read-only.
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::Q4));
    let runtime = RecordingRuntime::new(
        "quant_only_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: false,
            supports_kv_quantization: KvQuantSupport::Q4,
            ..Default::default()
        },
        stack.clone(),
    );

    let receipt = kv_cache_technique::occupancy(&runtime, model_id)
        .expect("occupancy must succeed when quantization-only adapter declares the capability");
    assert_eq!(receipt.occupancy.quant_level_current, KvQuantSupport::Q4);
}

#[test]
fn kv_cache_prefix_restore_rejects_tampered_handle() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::Q4));
    let runtime = RecordingRuntime::new(
        "test_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4,
            ..Default::default()
        },
        stack.clone(),
    );

    // Synthesize a v7 handle that was never committed — adapter must
    // reject with KvCacheError (tampered / unknown handle).
    let bogus = KvPrefixHandle::from_parts(Uuid::now_v7(), [0u8; 32], 5).expect("v7 UUID is valid");
    let err = kv_cache_technique::prefix_restore(&runtime, model_id, &bogus)
        .expect_err("uncommitted/tampered handle must be rejected");
    assert!(
        matches!(err, ModelRuntimeError::KvCacheError(_)),
        "expected KvCacheError, got {err:?}"
    );
}

#[test]
fn kv_cache_prefix_commit_binds_replay_resistance_and_restore_enforces_it() {
    // MT-095: the public technique surface must bind a per-process
    // replay-resistance derivation at commit and verify it at restore — even
    // though the adapter produces an unbound handle (from_tokens). This is the
    // end-to-end enforcement the Phase-1 verdict found missing.
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingKvCacheOps::new(KvQuantSupport::Q4));
    let runtime = RecordingRuntime::new(
        "test_adapter",
        model_id,
        ModelCapabilities {
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4,
            ..Default::default()
        },
        stack.clone(),
    );

    let commit = kv_cache_technique::prefix_commit(&runtime, model_id, &[1u32, 2, 3, 4])
        .expect("commit dispatches");
    // The handle the operator receives carries the MT-095 binding even though
    // the adapter (RecordingKvCacheOps) minted it via from_tokens (unbound).
    assert!(
        commit.prefix_handle.derived_id().is_some(),
        "commit must bind a replay-resistance derived_id"
    );

    // A handle the adapter would otherwise accept (same prefix_id +
    // content_hash) but carrying NO binding must be refused at the surface —
    // proving the gate is enforced, not bypassable by re-presenting an
    // unbound handle.
    let unbound_replay = KvPrefixHandle::from_parts(
        commit.prefix_handle.prefix_id(),
        *commit.prefix_handle.content_hash(),
        commit.prefix_handle.token_count(),
    )
    .expect("v7 prefix_id");
    assert!(unbound_replay.derived_id().is_none());
    let err = kv_cache_technique::prefix_restore(&runtime, model_id, &unbound_replay)
        .expect_err("unbound handle must be refused by the MT-095 restore gate");
    assert!(
        matches!(err, ModelRuntimeError::KvCacheError(_)),
        "expected KvCacheError, got {err:?}"
    );

    // The properly bound handle from commit restores cleanly.
    kv_cache_technique::prefix_restore(&runtime, model_id, &commit.prefix_handle)
        .expect("the bound handle restores");
}

#[test]
fn kv_cache_technique_event_types_resolve_through_fr_registry() {
    // Independent sanity that the new FR registry variants round-trip
    // the canonical strings advertised by the technique surface.
    for event_id in [
        FR_EVT_LLM_INFER_KV_SET_QUANTIZATION,
        FR_EVT_LLM_INFER_KV_PREFIX_COMMIT,
        FR_EVT_LLM_INFER_KV_PREFIX_RESTORE,
        FR_EVT_LLM_INFER_KV_EVICT,
    ] {
        let parsed = FrEventId::from_str_id(event_id)
            .expect("FR registry must know the canonical KV event id");
        assert_eq!(parsed.as_str(), event_id);
    }
}

// ----------------------------------------------------------------------------
// In-process fixtures.
// ----------------------------------------------------------------------------

struct RecordingRuntime {
    adapter_name: &'static str,
    model_capabilities: HashMap<ModelId, ModelCapabilities>,
    stack: Arc<RecordingKvCacheOps>,
}

impl RecordingRuntime {
    fn new(
        adapter_name: &'static str,
        model_id: ModelId,
        capabilities: ModelCapabilities,
        stack: Arc<RecordingKvCacheOps>,
    ) -> Self {
        Self {
            adapter_name,
            model_capabilities: HashMap::from([(model_id, capabilities)]),
            stack,
        }
    }
}

#[async_trait]
impl ModelRuntime for RecordingRuntime {
    fn adapter_name(&self) -> &'static str {
        self.adapter_name
    }

    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Err(ModelRuntimeError::LoadError(
            "recording runtime does not load models".to_string(),
        ))
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(stream::empty())
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        self.model_capabilities
            .get(&id)
            .ok_or_else(|| ModelRuntimeError::LoadError(format!("unknown model {id}")))
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::with_ops("recording-kv", self.stack.clone()))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("recording-lora-stack"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("recording-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

struct RecordingKvCacheOps {
    state: Mutex<RecordingKvState>,
}

struct RecordingKvState {
    quantization: KvQuantSupport,
    prefixes: HashMap<Uuid, ([u8; 32], u32)>,
    hits: u64,
    misses: u64,
    set_calls: u64,
    commit_calls: u64,
    evict_all_calls: u64,
}

impl RecordingKvCacheOps {
    fn new(initial: KvQuantSupport) -> Self {
        Self {
            state: Mutex::new(RecordingKvState {
                quantization: initial,
                prefixes: HashMap::new(),
                hits: 0,
                misses: 0,
                set_calls: 0,
                commit_calls: 0,
                evict_all_calls: 0,
            }),
        }
    }

    fn set_calls(&self) -> u64 {
        self.state.lock().unwrap().set_calls
    }
    fn commit_calls(&self) -> u64 {
        self.state.lock().unwrap().commit_calls
    }
    fn evict_all_calls(&self) -> u64 {
        self.state.lock().unwrap().evict_all_calls
    }
}

impl KvCacheOps for RecordingKvCacheOps {
    fn quantization(&self) -> KvQuantSupport {
        self.state.lock().unwrap().quantization
    }

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        let mut state = self.state.lock().unwrap();
        state.set_calls += 1;
        state.quantization = level;
        Ok(())
    }

    fn occupancy(&self) -> KvCacheStats {
        let state = self.state.lock().unwrap();
        KvCacheStats {
            bytes_used: state.prefixes.len() as u64 * 1024,
            bytes_capacity: 1024 * 1024,
            prefix_cache_entries: state.prefixes.len() as u32,
            prefix_cache_hit_count: state.hits,
            prefix_cache_miss_count: state.misses,
            quant_level_current: state.quantization,
        }
    }

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        let mut state = self.state.lock().unwrap();
        state.commit_calls += 1;
        let handle = KvPrefixHandle::from_tokens(prefix_tokens)?;
        state.prefixes.insert(
            handle.prefix_id(),
            (*handle.content_hash(), handle.token_count()),
        );
        Ok(handle)
    }

    fn prefix_restore(&self, handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        let mut state = self.state.lock().unwrap();
        match state.prefixes.get(&handle.prefix_id()) {
            Some((expected_hash, _)) if expected_hash == handle.content_hash() => {
                state.hits += 1;
                Ok(())
            }
            Some(_) => {
                state.misses += 1;
                Err(ModelRuntimeError::KvCacheError(
                    "prefix handle content_hash mismatch (tampered)".to_string(),
                ))
            }
            None => {
                state.misses += 1;
                Err(ModelRuntimeError::KvCacheError(format!(
                    "prefix handle {} not found in cache",
                    handle.prefix_id()
                )))
            }
        }
    }

    fn prefix_evict(&self, handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        let mut state = self.state.lock().unwrap();
        state.prefixes.remove(&handle.prefix_id());
        Ok(())
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        let mut state = self.state.lock().unwrap();
        state.evict_all_calls += 1;
        state.prefixes.clear();
        Ok(())
    }
}
