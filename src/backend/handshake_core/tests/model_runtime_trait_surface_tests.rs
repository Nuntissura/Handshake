use std::{
    collections::HashSet,
    error::Error,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use handshake_core::{
    model_runtime::{
        process_ledger_integration::{
            ModelProcessLedgerRegistrar, ModelProcessRollback, ModelProcessSpawnContext,
        },
        BaseModelTag, CaptureResult, CaptureSpec, Embedding, GenerateRequest, KvCacheHandle,
        KvCacheOps, KvCachePolicy, KvCacheStats, KvPrefixHandle, KvQuantSupport, LayerIndex,
        LoadSpec, LoraDescriptor, LoraId, LoraStackEntry, LoraStackHandle, LoraStackOps,
        LoraStackSnapshot, LoraStrength, ModelCapabilities, ModelId, ModelRegistration,
        ModelRegistry, ModelRuntime, ModelRuntimeError, OperatorId, ProviderKind, RuntimeBinding,
        RuntimeKind, SamplingParams, Score, SteeringHookHandle, SteeringHookOps,
        SteeringProvenance, SteeringVector, SteeringVectorId, SteeringVectorMeta,
        SteeringVectorValues, TokenStream,
    },
    process_ledger::{
        LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, ProcessEngineKind,
        ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore,
    },
};
use serde::Serialize;

const CAPABILITIES_SAMPLE: &str = include_str!("fixtures/model_runtime/capabilities_sample.json");
const LORA_DESCRIPTOR_SAMPLE: &str =
    include_str!("fixtures/model_runtime/lora_descriptor_sample.json");

#[test]
fn object_safety_compile_test_accepts_model_runtime_and_subtrait_objects() {
    fn takes_model_runtime(_: &dyn ModelRuntime) {}
    fn takes_kv_cache_ops(_: Box<dyn KvCacheOps>) {}
    fn takes_lora_stack_ops(_: Box<dyn LoraStackOps>) {}
    fn takes_steering_hook_ops(_: Box<dyn SteeringHookOps>) {}

    takes_model_runtime(&SurfaceRuntime::default());
    takes_kv_cache_ops(Box::new(SurfaceKvCacheOps::default()));
    takes_lora_stack_ops(Box::new(SurfaceLoraStackOps));
    takes_steering_hook_ops(Box::new(SurfaceSteeringHookOps));
}

#[test]
fn trait_is_send_sync_for_runtime_and_subtrait_objects() {
    fn assert_send_sync<T: Send + Sync + ?Sized>() {}

    assert_send_sync::<dyn ModelRuntime>();
    assert_send_sync::<dyn KvCacheOps>();
    assert_send_sync::<dyn LoraStackOps>();
    assert_send_sync::<dyn SteeringHookOps>();
    assert_send_sync::<Arc<dyn ModelRuntime>>();
    assert_send_sync::<Box<dyn KvCacheOps>>();
}

#[test]
fn model_id_v7_monotonic_mints_1000_non_decreasing_timestamp_prefixes() {
    let mut previous = 0_u64;
    for _ in 0..1000 {
        let id = ModelId::new_v7();
        assert_eq!(id.as_uuid().get_version_num(), 7);
        let current = uuid_v7_unix_millis(id);
        assert!(
            current >= previous,
            "ModelId v7 timestamp prefix moved backwards: previous={previous}, current={current}"
        );
        previous = current;
    }
}

#[test]
fn capabilities_serde_round_trip_matches_fixture_bytes() {
    let fixture = CAPABILITIES_SAMPLE.trim();
    let capabilities: ModelCapabilities =
        serde_json::from_str(fixture).expect("capabilities fixture deserializes");

    let serialized = serde_json::to_string(&capabilities).expect("capabilities serialize");

    assert_eq!(serialized.as_bytes(), fixture.as_bytes());
}

#[test]
fn capabilities_core_snake_case_and_ipc_projection_camelcase_are_both_stable() {
    let capabilities: ModelCapabilities =
        serde_json::from_str(CAPABILITIES_SAMPLE).expect("capabilities fixture deserializes");

    let core = serde_json::to_value(&capabilities).expect("core capabilities serialize");
    assert!(core.get("supports_lora").is_some());
    assert!(core.get("supportsLora").is_none());

    let ipc = serde_json::to_value(ModelCapabilitiesIpc::from(&capabilities))
        .expect("ipc capabilities serialize");
    assert_eq!(ipc["supportsLora"], true);
    assert_eq!(ipc["supportsKvPrefixCache"], true);
    assert_eq!(ipc["supportsKvQuantization"], "q4_q8_mix");
    assert!(ipc.get("supports_lora").is_none());
}

#[test]
fn error_variant_completeness_has_display_text_and_unique_discriminants() {
    fn assert_error_traits<T: Error + Send + Sync + 'static>() {}
    assert_error_traits::<ModelRuntimeError>();

    let variants = vec![
        ModelRuntimeError::LoadError("load".to_string()),
        ModelRuntimeError::UnloadError("unload".to_string()),
        ModelRuntimeError::GenerateError("generate".to_string()),
        ModelRuntimeError::ScoreError("score".to_string()),
        ModelRuntimeError::EmbedError("embed".to_string()),
        ModelRuntimeError::CapabilityNotSupported {
            capability: "kv_cache".to_string(),
            adapter: "surface-test".to_string(),
        },
        ModelRuntimeError::KvCacheError("kv".to_string()),
        ModelRuntimeError::LoraStackError("lora".to_string()),
        ModelRuntimeError::SteeringHookError("steering".to_string()),
        ModelRuntimeError::Cancelled,
        ModelRuntimeError::AdapterMismatch {
            expected: "llama_cpp".to_string(),
            got: "candle".to_string(),
        },
    ];
    let mut discriminants = HashSet::new();

    assert_eq!(variants.len(), 11);
    for variant in &variants {
        assert!(
            !variant.to_string().trim().is_empty(),
            "ModelRuntimeError variant has empty Display text: {variant:?}"
        );
        assert!(
            discriminants.insert(std::mem::discriminant(variant)),
            "duplicate ModelRuntimeError discriminant for {variant:?}"
        );
    }
}

#[test]
fn lora_descriptor_serde_round_trip_matches_fixture_bytes() {
    let fixture = LORA_DESCRIPTOR_SAMPLE.trim();
    let descriptor: LoraDescriptor =
        serde_json::from_str(fixture).expect("lora descriptor fixture deserializes");

    assert_eq!(descriptor.id.as_uuid().get_version_num(), 7);
    assert_eq!(descriptor.base_model_compat.as_str(), "local-routing-base");
    assert_eq!(descriptor.license_tag.as_str(), "operator-local");

    let serialized = serde_json::to_string(&descriptor).expect("lora descriptor serializes");
    assert_eq!(serialized.as_bytes(), fixture.as_bytes());
}

#[test]
fn lora_strength_clamp_rejects_out_of_range_values() {
    assert!(LoraStrength::try_new(3.0).is_err());
    assert!(LoraStrength::try_new(-0.5).is_err());
    assert_eq!(LoraStrength::try_new(0.5).unwrap().value(), 0.5);
}

#[test]
fn kv_prefix_handle_tamper_changes_equality_even_when_prefix_id_matches() {
    let original = KvPrefixHandle::from_tokens(&[10, 20, 30]).expect("prefix handle");
    let mut tampered_hash = *original.content_hash();
    tampered_hash[0] ^= 0xFF;

    let tampered =
        KvPrefixHandle::from_parts(original.prefix_id(), tampered_hash, original.token_count())
            .expect("tampered handle with same id and different hash");

    assert_ne!(original, tampered);
    assert_eq!(original.prefix_id(), tampered.prefix_id());
    assert_ne!(original.content_hash(), tampered.content_hash());
}

#[test]
fn steering_vector_provenance_required_rejects_empty_manual_author() {
    let err = SteeringVector::try_new(
        None,
        "manual-vector",
        LayerIndex::new(12),
        handshake_core::model_runtime::HookPoint::ResidStream,
        SteeringVectorValues::try_new(vec![0.1, 0.2], 1.0).unwrap(),
        "manual provenance must name its author",
        Some(SteeringProvenance::Manual {
            author: " ".to_string(),
            notes: String::new(),
        }),
    )
    .expect_err("empty manual provenance author must be rejected");

    assert!(err.to_string().contains("author"), "{err}");
}

#[test]
fn capability_consistency_rejects_llamacpp_activation_steering_registration() {
    let mut registry = ModelRegistry::default();
    let err = registry
        .register(model_registration(
            ModelId::new_v7(),
            RuntimeBinding::LlamaCpp,
            ModelCapabilities {
                supports_activation_steering: true,
                ..Default::default()
            },
        ))
        .expect_err("llama.cpp cannot claim activation steering");

    assert!(err.to_string().contains("activation_steering"), "{err}");
}

#[tokio::test]
async fn external_compat_short_circuit_registers_no_process_ledger_row() {
    let store = InMemoryProcessLedgerStore::default();
    let (batcher, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 8,
            batch_size: 8,
            flush_interval: Duration::from_millis(100),
        },
        Arc::new(InMemoryOverflowSink::default()),
    )
    .expect("ledger batcher");
    let registrar = ModelProcessLedgerRegistrar::new(batcher);
    let rollback = NoopRollback;

    let outcome = registrar
        .register_model_process(
            &load_spec(ProviderKind::ExternalCompat, RuntimeKind::LlamaCpp),
            9090,
            ProcessEngineKind::ExternalCompat,
            ModelProcessSpawnContext::new(
                ModelId::new_v7(),
                RuntimeBinding::LlamaCpp,
                "SR-MT071",
                "external-compat",
            ),
            &rollback,
        )
        .expect("external compat short-circuits");

    assert!(outcome.is_none());
    drain
        .drain_available_to(Arc::new(store.clone()))
        .await
        .expect("drain ledger");
    assert!(store.events().is_empty());
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelCapabilitiesIpc {
    supports_lora: bool,
    supports_kv_prefix_cache: bool,
    supports_kv_quantization: KvQuantSupport,
    supports_activation_steering: bool,
    supports_subquadratic: bool,
    supports_speculative_draft: bool,
    supports_eagle3: bool,
}

impl From<&ModelCapabilities> for ModelCapabilitiesIpc {
    fn from(value: &ModelCapabilities) -> Self {
        Self {
            supports_lora: value.supports_lora,
            supports_kv_prefix_cache: value.supports_kv_prefix_cache,
            supports_kv_quantization: value.supports_kv_quantization,
            supports_activation_steering: value.supports_activation_steering,
            supports_subquadratic: value.supports_subquadratic,
            supports_speculative_draft: value.supports_speculative_draft,
            supports_eagle3: value.supports_eagle3,
        }
    }
}

#[derive(Default)]
struct SurfaceRuntime {
    capabilities: ModelCapabilities,
}

#[async_trait]
impl ModelRuntime for SurfaceRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _req: GenerateRequest) -> TokenStream {
        Box::pin(futures::stream::empty())
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

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(KvCacheHandle::new("surface-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("surface-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("surface-steering"))
    }

    fn cancel(&self, token: handshake_core::model_runtime::CancellationToken) {
        token.cancel();
    }
}

#[derive(Default)]
struct SurfaceKvCacheOps {
    quant: Mutex<KvQuantSupport>,
}

impl KvCacheOps for SurfaceKvCacheOps {
    fn quantization(&self) -> KvQuantSupport {
        *self.quant.lock().unwrap()
    }

    fn set_quantization(&self, level: KvQuantSupport) -> Result<(), ModelRuntimeError> {
        *self.quant.lock().unwrap() = level;
        Ok(())
    }

    fn occupancy(&self) -> KvCacheStats {
        KvCacheStats {
            bytes_used: 0,
            bytes_capacity: 0,
            prefix_cache_entries: 0,
            prefix_cache_hit_count: 0,
            prefix_cache_miss_count: 0,
            quant_level_current: self.quantization(),
        }
    }

    fn prefix_commit(&self, prefix_tokens: &[u32]) -> Result<KvPrefixHandle, ModelRuntimeError> {
        KvPrefixHandle::from_tokens(prefix_tokens)
    }

    fn prefix_restore(&self, _handle: &KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn prefix_evict(&self, _handle: KvPrefixHandle) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn evict_all(&self) -> Result<(), ModelRuntimeError> {
        Ok(())
    }
}

struct SurfaceLoraStackOps;

#[async_trait]
impl LoraStackOps for SurfaceLoraStackOps {
    async fn mount(
        &self,
        _desc: LoraDescriptor,
        _strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    async fn unmount(&self, _id: LoraId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn list_active(&self) -> Vec<LoraStackEntry> {
        Vec::new()
    }

    async fn set_strength(
        &self,
        _id: LoraId,
        _strength: LoraStrength,
    ) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    async fn swap(
        &self,
        _new_stack: Vec<(LoraDescriptor, LoraStrength)>,
    ) -> Result<LoraStackSnapshot, ModelRuntimeError> {
        Ok(LoraStackSnapshot::default())
    }
}

struct SurfaceSteeringHookOps;

#[async_trait]
impl SteeringHookOps for SurfaceSteeringHookOps {
    async fn capture(&self, _spec: CaptureSpec) -> Result<CaptureResult, ModelRuntimeError> {
        Ok(CaptureResult {
            activations: Default::default(),
            tokens_seen: 0,
        })
    }

    async fn register_vector(
        &self,
        vector: SteeringVector,
    ) -> Result<SteeringVectorId, ModelRuntimeError> {
        Ok(vector.id)
    }

    fn list_vectors(&self) -> Vec<SteeringVectorMeta> {
        Vec::new()
    }

    async fn set_active(&self, _ids: Vec<SteeringVectorId>) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    async fn unregister(&self, _id: SteeringVectorId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
struct InMemoryProcessLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl InMemoryProcessLedgerStore {
    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().extend(events);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct InMemoryOverflowSink;

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
}

struct NoopRollback;

impl ModelProcessRollback for NoopRollback {
    fn kill_spawned_process(&self, _pid: u32) -> Result<(), ModelRuntimeError> {
        Ok(())
    }
}

fn model_registration(
    model_id: ModelId,
    runtime_binding: RuntimeBinding,
    declared_capabilities: ModelCapabilities,
) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: PathBuf::from("fixtures/models/surface.gguf"),
        sha256: [3; 32],
        runtime_binding,
        declared_capabilities,
        base_model_tag: BaseModelTag::new("surface-base"),
        registered_at_utc: chrono::Utc::now(),
        registered_by: OperatorId::new("operator-ilja"),
        provider: ProviderKind::Local,
    }
}

fn load_spec(provider: ProviderKind, runtime_kind: RuntimeKind) -> LoadSpec {
    LoadSpec {
        artifact_path: "fixtures/models/surface.gguf".into(),
        sha256_expected: "0303030303030303030303030303030303030303030303030303030303030303"
            .to_string(),
        runtime_kind,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider,
        engine_origin: None,
        external_engine_import: None,
    }
}

fn uuid_v7_unix_millis(id: ModelId) -> u64 {
    let bytes = *id.as_uuid().as_bytes();
    ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | bytes[5] as u64
}
