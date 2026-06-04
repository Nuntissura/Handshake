use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde_json::json;

use handshake_core::{
    model_runtime::{
        process_ledger_integration::{
            ModelProcessLedgerRegistrar, ModelProcessRollback, ModelProcessSpawnContext,
            MODEL_PROCESS_METADATA_CAP_BYTES,
        },
        KvCachePolicy, KvQuantSupport, LoadSpec, ModelCapabilities, ModelId, ModelRuntimeError,
        ProviderKind, RuntimeBinding, RuntimeKind, SamplingParams,
    },
    process_ledger::{
        LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, ProcessEngineKind,
        ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessStart,
    },
};

#[tokio::test]
async fn model_runtime_process_ledger_tests_register_local_spawn_returns_v7_record_id() {
    let fixture = LedgerFixture::new(8, OverflowMode::Record).expect("ledger fixture");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();
    let model_id = ModelId::new_v7();

    let record_id = registrar
        .register_model_process(
            &load_spec(ProviderKind::Local, RuntimeKind::LlamaCpp),
            4242,
            ProcessEngineKind::LlamaCpp,
            spawn_context(model_id, RuntimeBinding::LlamaCpp),
            &rollback,
        )
        .expect("local spawn registers")
        .expect("local provider produces ledger record");

    assert_eq!(record_id.as_uuid().get_version_num(), 7);
    assert!(rollback.killed_pids().is_empty());

    fixture.drain().await.expect("drain ledger");
    let events = fixture.store.events();
    assert_eq!(events.len(), 1);
    let LedgerEvent::Start(start) = &events[0] else {
        panic!("expected start event");
    };
    assert_eq!(start.process_uuid, record_id.as_uuid());
    assert_eq!(start.os_pid, Some(4242));
    assert_eq!(start.engine_kind, ProcessEngineKind::LlamaCpp);
    assert_eq!(start.parent_session_id.as_deref(), Some("SR-MODEL-LEDGER"));
    assert_eq!(start.sandbox_adapter_id.as_deref(), Some("wsl2-podman"));
    assert_eq!(
        start.model_artifact_sha256.as_deref(),
        Some("0707070707070707070707070707070707070707070707070707070707070707")
    );
    assert_eq!(start.metadata_jsonb["model_id"], model_id.to_string());
    assert_eq!(start.metadata_jsonb["runtime_binding"], "llama_cpp");
}

#[test]
fn model_runtime_process_ledger_tests_duplicate_pid_rolls_back_spawn() {
    let fixture = LedgerFixture::new(8, OverflowMode::Record).expect("ledger fixture");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();
    let spec = load_spec(ProviderKind::Local, RuntimeKind::Candle);

    registrar
        .register_model_process(
            &spec,
            5555,
            ProcessEngineKind::Candle,
            spawn_context(ModelId::new_v7(), RuntimeBinding::Candle),
            &rollback,
        )
        .expect("first pid registers");
    let err = registrar
        .register_model_process(
            &spec,
            5555,
            ProcessEngineKind::Candle,
            spawn_context(ModelId::new_v7(), RuntimeBinding::Candle),
            &rollback,
        )
        .expect_err("duplicate pid must fail");

    assert!(err.to_string().contains("already registered"), "{err}");
    assert_eq!(rollback.killed_pids(), vec![5555]);
}

#[tokio::test]
async fn model_runtime_process_ledger_tests_external_compat_short_circuits_without_owned_row() {
    let fixture = LedgerFixture::new(8, OverflowMode::Record).expect("ledger fixture");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();

    let outcome = registrar
        .register_model_process(
            &load_spec(ProviderKind::ExternalCompat, RuntimeKind::LlamaCpp),
            7777,
            ProcessEngineKind::ExternalCompat,
            spawn_context(ModelId::new_v7(), RuntimeBinding::LlamaCpp),
            &rollback,
        )
        .expect("external compat short-circuits");

    assert!(outcome.is_none());
    assert!(rollback.killed_pids().is_empty());
    fixture.drain().await.expect("drain ledger");
    assert!(fixture.store.events().is_empty());
}

#[tokio::test]
async fn model_runtime_process_ledger_tests_rejects_abliteration_tool_as_model_runtime_engine() {
    let fixture = LedgerFixture::new(8, OverflowMode::Record).expect("ledger fixture");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();

    let err = registrar
        .register_model_process(
            &load_spec(ProviderKind::Local, RuntimeKind::LlamaCpp),
            7801,
            ProcessEngineKind::AbliterationTool,
            spawn_context(ModelId::new_v7(), RuntimeBinding::LlamaCpp),
            &rollback,
        )
        .expect_err("offline-only AbliterationTool must not be a ModelRuntime engine");

    assert!(err.to_string().contains("AbliterationTool"), "{err}");
    assert!(err.to_string().contains("offline-only"), "{err}");
    assert_eq!(rollback.killed_pids(), vec![7801]);
    fixture.drain().await.expect("drain ledger");
    assert!(fixture.store.events().is_empty());
}

#[tokio::test]
async fn model_runtime_process_ledger_tests_rejects_runtime_binding_engine_kind_mismatch() {
    let fixture = LedgerFixture::new(8, OverflowMode::Record).expect("ledger fixture");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();

    let err = registrar
        .register_model_process(
            &load_spec(ProviderKind::Local, RuntimeKind::LlamaCpp),
            7802,
            ProcessEngineKind::Candle,
            spawn_context(ModelId::new_v7(), RuntimeBinding::LlamaCpp),
            &rollback,
        )
        .expect_err("model runtime process engine kind must match RuntimeBinding");

    assert!(err.to_string().contains("runtime_binding"), "{err}");
    assert!(err.to_string().contains("llama_cpp"), "{err}");
    assert!(err.to_string().contains("candle"), "{err}");
    assert_eq!(rollback.killed_pids(), vec![7802]);
    fixture.drain().await.expect("drain ledger");
    assert!(fixture.store.events().is_empty());
}

#[tokio::test]
async fn model_runtime_process_ledger_tests_oversized_metadata_is_capped_before_enqueue() {
    let fixture = LedgerFixture::new(8, OverflowMode::Record).expect("ledger fixture");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();
    let large_note = "x".repeat(MODEL_PROCESS_METADATA_CAP_BYTES + 512);
    let context = spawn_context(ModelId::new_v7(), RuntimeBinding::Candle)
        .with_metadata_blob(json!({ "large_note": large_note }));

    let record_id = registrar
        .register_model_process(
            &load_spec(ProviderKind::Local, RuntimeKind::Candle),
            8888,
            ProcessEngineKind::Candle,
            context,
            &rollback,
        )
        .expect("register with capped metadata")
        .expect("local provider produces ledger record");

    fixture.drain().await.expect("drain ledger");
    let events = fixture.store.events();
    let LedgerEvent::Start(start) = &events[0] else {
        panic!("expected start event");
    };
    assert_eq!(start.process_uuid, record_id.as_uuid());
    assert_eq!(start.metadata_jsonb["capped"], true);
    assert!(start.metadata_jsonb["original_bytes"].as_u64().unwrap() > 4096);
    assert_eq!(
        start.metadata_jsonb["warning_event"],
        "FR-EVT-MODEL-PROCESS-METADATA-CAPPED"
    );
}

#[test]
fn model_runtime_process_ledger_tests_backpressure_returns_without_blocking_and_emits_overflow() {
    let fixture = LedgerFixture::new(1, OverflowMode::Record).expect("ledger fixture");
    fixture
        .batcher
        .record_start(ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            "KERNEL_BUILDER",
            Some("WP-KERNEL-004".to_string()),
        ))
        .expect("fill one-slot channel");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();

    let started = Instant::now();
    let record_id = registrar
        .register_model_process(
            &load_spec(ProviderKind::Local, RuntimeKind::LlamaCpp),
            9999,
            ProcessEngineKind::LlamaCpp,
            spawn_context(ModelId::new_v7(), RuntimeBinding::LlamaCpp),
            &rollback,
        )
        .expect("overflow is degraded but accepted")
        .expect("local provider produces ledger record");

    assert_eq!(record_id.as_uuid().get_version_num(), 7);
    assert!(
        started.elapsed() < Duration::from_millis(20),
        "registration should not block on full ledger channel"
    );
    assert!(rollback.killed_pids().is_empty());
    assert_eq!(fixture.overflow.events().len(), 1);
}

#[test]
fn model_runtime_process_ledger_tests_ledger_enqueue_failure_rolls_back_spawn() {
    let fixture = LedgerFixture::new(1, OverflowMode::Fail).expect("ledger fixture");
    fixture
        .batcher
        .record_start(ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            "KERNEL_BUILDER",
            Some("WP-KERNEL-004".to_string()),
        ))
        .expect("fill one-slot channel");
    let registrar = ModelProcessLedgerRegistrar::new(fixture.batcher.clone());
    let rollback = RecordingRollback::default();

    let err = registrar
        .register_model_process(
            &load_spec(ProviderKind::Local, RuntimeKind::LlamaCpp),
            10_001,
            ProcessEngineKind::LlamaCpp,
            spawn_context(ModelId::new_v7(), RuntimeBinding::LlamaCpp),
            &rollback,
        )
        .expect_err("overflow sink failure rejects registration");

    assert!(err.to_string().contains("process ledger"), "{err}");
    assert_eq!(rollback.killed_pids(), vec![10_001]);
}

fn load_spec(provider: ProviderKind, runtime_kind: RuntimeKind) -> LoadSpec {
    LoadSpec {
        artifact_path: "fixtures/models/local-test.gguf".into(),
        sha256_expected: "0707070707070707070707070707070707070707070707070707070707070707"
            .to_string(),
        runtime_kind,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities {
            supports_lora: true,
            supports_kv_prefix_cache: true,
            supports_kv_quantization: KvQuantSupport::Q4,
            supports_activation_steering: runtime_kind == RuntimeKind::Candle,
            supports_subquadratic: false,
            supports_speculative_draft: false,
            supports_eagle3: false,
        },
        provider,
        engine_origin: None,
        external_engine_import: None,
    }
}

fn spawn_context(model_id: ModelId, runtime_binding: RuntimeBinding) -> ModelProcessSpawnContext {
    ModelProcessSpawnContext::new(model_id, runtime_binding, "SR-MODEL-LEDGER", "wsl2-podman")
        .with_owner("KERNEL_BUILDER", Some("WP-KERNEL-004"))
        .with_role_id("KERNEL_BUILDER")
        .with_mt_id("MT-069")
        .with_work_profile_id("local-model-lab")
        .with_metadata_blob(json!({ "purpose": "model-runtime-process-ledger-test" }))
}

struct LedgerFixture {
    batcher: LedgerBatcher,
    drain: handshake_core::process_ledger::ProcessLedgerDrain,
    store: InMemoryProcessLedgerStore,
    overflow: InMemoryOverflowSink,
}

impl LedgerFixture {
    fn new(capacity: usize, overflow_mode: OverflowMode) -> Result<Self, ProcessLedgerError> {
        let overflow = InMemoryOverflowSink::new(overflow_mode);
        let (batcher, drain) = LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig {
                capacity,
                batch_size: 8,
                flush_interval: Duration::from_millis(100),
            },
            Arc::new(overflow.clone()),
        )?;
        Ok(Self {
            batcher,
            drain,
            store: InMemoryProcessLedgerStore::default(),
            overflow,
        })
    }

    async fn drain(&self) -> Result<(), ProcessLedgerError> {
        self.drain
            .drain_available_to(Arc::new(self.store.clone()))
            .await
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

#[derive(Clone, Copy)]
enum OverflowMode {
    Record,
    Fail,
}

#[derive(Clone)]
struct InMemoryOverflowSink {
    mode: OverflowMode,
    events: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl InMemoryOverflowSink {
    fn new(mode: OverflowMode) -> Self {
        Self {
            mode,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn events(&self) -> Vec<LedgerOverflowEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().push(event);
        match self.mode {
            OverflowMode::Record => Ok(()),
            OverflowMode::Fail => Err(ProcessLedgerError::OverflowEmit(
                "overflow sink unavailable".to_string(),
            )),
        }
    }
}

#[derive(Default)]
struct RecordingRollback {
    killed_pids: Mutex<Vec<u32>>,
}

impl RecordingRollback {
    fn killed_pids(&self) -> Vec<u32> {
        self.killed_pids.lock().unwrap().clone()
    }
}

impl ModelProcessRollback for RecordingRollback {
    fn kill_spawned_process(&self, pid: u32) -> Result<(), ModelRuntimeError> {
        self.killed_pids.lock().unwrap().push(pid);
        Ok(())
    }
}
