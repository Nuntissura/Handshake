//! MT-112 — INF-9 Subquadratic technique surface tests.
//!
//! Exercises `subquadratic::{state_commit, state_restore, state_list,
//! evict_all, persist, rehydrate}` against a fake adapter that impls
//! `KvCacheOps` via `KvCacheHandle::with_ops`. Mirrors the structural
//! pattern of `kv_cache_technique_tests` so the recording fixtures
//! stay consistent across technique surfaces.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use futures_util::stream;
use handshake_core::model_runtime::{
    techniques::subquadratic::{
        self, FR_EVT_LLM_INFER_SUBQUAD_PERSIST, FR_EVT_LLM_INFER_SUBQUAD_STATE_COMMIT,
        FR_EVT_LLM_INFER_SUBQUAD_STATE_RESTORE,
    },
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, KvCacheOps, KvCacheStats,
    KvPrefixHandle, KvQuantSupport, LoadSpec, LoraStackHandle, ModelCapabilities, ModelId,
    ModelRuntime, ModelRuntimeError, Score, SteeringHookHandle, TokenStream,
};
use uuid::Uuid;

#[test]
fn subquadratic_state_commit_dispatches_and_returns_canonical_event_type() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "candle_subquad_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: true,
            ..Default::default()
        },
        stack.clone(),
    );

    let prefix_tokens = vec![10u32, 20, 30, 40];
    let receipt = subquadratic::state_commit(&runtime, model_id, &prefix_tokens)
        .expect("state_commit must dispatch on a capable adapter");
    assert_eq!(receipt.event_type, FR_EVT_LLM_INFER_SUBQUAD_STATE_COMMIT);
    assert_eq!(receipt.model_id, model_id);
    assert_eq!(receipt.prefix_handle.token_count(), 4);
    assert_eq!(receipt.occupancy.prefix_cache_entries, 1);
    assert_eq!(stack.commit_calls(), 1);
}

#[test]
fn subquadratic_state_commit_fails_closed_when_capability_unsupported() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "transformer_only_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: false,
            supports_kv_prefix_cache: true,
            ..Default::default()
        },
        stack.clone(),
    );

    let err = subquadratic::state_commit(&runtime, model_id, &[1u32, 2, 3])
        .expect_err("supports_subquadratic=false must reject the technique surface");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
            if capability == "subquadratic"
        ),
        "expected CapabilityNotSupported{{subquadratic}}, got {err:?}"
    );
    assert_eq!(
        stack.commit_calls(),
        0,
        "capability gate must precede mutation"
    );
}

#[test]
fn subquadratic_state_commit_and_restore_round_trip() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "candle_subquad_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: true,
            ..Default::default()
        },
        stack.clone(),
    );

    let prefix_tokens = vec![1u32, 2, 3, 4];
    let commit = subquadratic::state_commit(&runtime, model_id, &prefix_tokens)
        .expect("state_commit must dispatch on a capable adapter");

    let restore = subquadratic::state_restore(&runtime, model_id, &commit.prefix_handle)
        .expect("state_restore must dispatch on the committed handle");
    assert_eq!(restore.event_type, FR_EVT_LLM_INFER_SUBQUAD_STATE_RESTORE);
    assert_eq!(restore.prefix_handle, commit.prefix_handle);
    assert!(restore.hit);
    assert_eq!(restore.occupancy.prefix_cache_hit_count, 1);

    let list = subquadratic::state_list(&runtime, model_id)
        .expect("state_list must dispatch on a capable adapter");
    assert_eq!(list.occupancy.prefix_cache_entries, 1);
    assert_eq!(list.occupancy.prefix_cache_hit_count, 1);
}

#[test]
fn subquadratic_state_restore_rejects_tampered_handle() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "candle_subquad_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: true,
            ..Default::default()
        },
        stack.clone(),
    );

    // Synthesize a v7 handle that was never committed — adapter rejects.
    let bogus = KvPrefixHandle::from_parts(Uuid::now_v7(), [0u8; 32], 5).expect("v7 UUID is valid");
    let err = subquadratic::state_restore(&runtime, model_id, &bogus)
        .expect_err("uncommitted/tampered handle must be rejected");
    assert!(
        matches!(err, ModelRuntimeError::KvCacheError(_)),
        "expected KvCacheError, got {err:?}"
    );
}

#[test]
fn subquadratic_evict_all_returns_before_after_occupancy() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "candle_subquad_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: true,
            ..Default::default()
        },
        stack.clone(),
    );

    subquadratic::state_commit(&runtime, model_id, &[10u32, 20])
        .expect("state_commit must succeed before evict_all test");
    subquadratic::state_commit(&runtime, model_id, &[30u32, 40, 50])
        .expect("second state_commit must succeed");

    let receipt = subquadratic::evict_all(&runtime, model_id)
        .expect("evict_all must dispatch on a capable adapter");
    assert_eq!(receipt.previous_occupancy.prefix_cache_entries, 2);
    assert_eq!(receipt.current_occupancy.prefix_cache_entries, 0);
    assert_eq!(stack.evict_all_calls(), 1);
}

#[test]
fn subquadratic_persist_returns_deferred_mt117_marker_after_capability_gate_passes() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "candle_subquad_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: true,
            ..Default::default()
        },
        stack.clone(),
    );

    let commit = subquadratic::state_commit(&runtime, model_id, &[7u32, 8])
        .expect("state_commit must succeed before persist deferral test");

    let err = subquadratic::persist(&runtime, model_id, &commit.prefix_handle)
        .expect_err("persist must defer to MT-117 even on a capable adapter");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
            if capability == "subquadratic_persist_disk_deferred_mt117"
        ),
        "expected CapabilityNotSupported{{subquadratic_persist_disk_deferred_mt117}}, got {err:?}"
    );

    // Sanity: the canonical FR event id for persist is still exposed so
    // the IPC layer can name it deterministically when the MT-117
    // wiring lands.
    assert_eq!(
        FR_EVT_LLM_INFER_SUBQUAD_PERSIST,
        "FR-EVT-LLM-INFER-SUBQUAD-PERSIST"
    );
}

#[test]
fn subquadratic_persist_fails_closed_when_capability_unsupported() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "transformer_only_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: false,
            ..Default::default()
        },
        stack.clone(),
    );

    let bogus = KvPrefixHandle::from_parts(Uuid::now_v7(), [0u8; 32], 1).expect("v7 UUID is valid");
    let err = subquadratic::persist(&runtime, model_id, &bogus)
        .expect_err("persist must fail closed on a non-subquadratic adapter");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
            if capability == "subquadratic"
        ),
        "capability gate must precede the MT-117 deferral; got {err:?}"
    );
}

#[test]
fn subquadratic_rehydrate_returns_deferred_mt117_marker_after_capability_gate_passes() {
    let model_id = ModelId::new_v7();
    let stack = Arc::new(RecordingStateVectorOps::new());
    let runtime = RecordingRuntime::new(
        "candle_subquad_adapter",
        model_id,
        ModelCapabilities {
            supports_subquadratic: true,
            ..Default::default()
        },
        stack.clone(),
    );

    let err = subquadratic::rehydrate(&runtime, model_id)
        .expect_err("rehydrate must defer to MT-117 even on a capable adapter");
    assert!(
        matches!(
            err,
            ModelRuntimeError::CapabilityNotSupported { ref capability, .. }
            if capability == "subquadratic_rehydrate_disk_deferred_mt117"
        ),
        "expected CapabilityNotSupported{{subquadratic_rehydrate_disk_deferred_mt117}}, got {err:?}"
    );
}

// ----------------------------------------------------------------------------
// In-process fixtures.
// ----------------------------------------------------------------------------

struct RecordingRuntime {
    adapter_name: &'static str,
    model_capabilities: HashMap<ModelId, ModelCapabilities>,
    stack: Arc<RecordingStateVectorOps>,
}

impl RecordingRuntime {
    fn new(
        adapter_name: &'static str,
        model_id: ModelId,
        capabilities: ModelCapabilities,
        stack: Arc<RecordingStateVectorOps>,
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
        Ok(KvCacheHandle::with_ops(
            "recording-subquadratic-kv",
            self.stack.clone(),
        ))
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

struct RecordingStateVectorOps {
    state: Mutex<RecordingState>,
}

struct RecordingState {
    prefixes: HashMap<Uuid, ([u8; 32], u32)>,
    hits: u64,
    misses: u64,
    commit_calls: u64,
    evict_all_calls: u64,
}

impl RecordingStateVectorOps {
    fn new() -> Self {
        Self {
            state: Mutex::new(RecordingState {
                prefixes: HashMap::new(),
                hits: 0,
                misses: 0,
                commit_calls: 0,
                evict_all_calls: 0,
            }),
        }
    }

    fn commit_calls(&self) -> u64 {
        self.state.lock().unwrap().commit_calls
    }

    fn evict_all_calls(&self) -> u64 {
        self.state.lock().unwrap().evict_all_calls
    }
}

impl KvCacheOps for RecordingStateVectorOps {
    fn quantization(&self) -> KvQuantSupport {
        KvQuantSupport::None
    }

    fn set_quantization(&self, _level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn occupancy(&self) -> KvCacheStats {
        let state = self.state.lock().unwrap();
        KvCacheStats {
            bytes_used: state.prefixes.len() as u64 * 4096,
            bytes_capacity: 1024 * 1024,
            prefix_cache_entries: state.prefixes.len() as u32,
            prefix_cache_hit_count: state.hits,
            prefix_cache_miss_count: state.misses,
            quant_level_current: KvQuantSupport::None,
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
                    "state-vector handle content_hash mismatch (tampered)".to_string(),
                ))
            }
            None => {
                state.misses += 1;
                Err(ModelRuntimeError::KvCacheError(format!(
                    "state-vector handle {} not found in cache",
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
