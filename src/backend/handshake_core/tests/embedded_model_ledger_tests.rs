//! WP-1 MT-013: ProcessOwnershipLedger START/STOP proof for the default
//! embedded-model load path.
//!
//! NON-SKIPPABLE BY DESIGN: this test has no `#[ignore]` and no
//! skip-if-no-weights guard, so it FAILS (never silently passes as "skipped")
//! if the load+shutdown seam does not emit the START/STOP rows. It drives the
//! REAL production assembly seam (`llm::boot::assemble_local_runtime_client`,
//! the function `build_default_local_client` calls after a real model load) with
//! a minimal in-process runtime, so it exercises `record_start` on load and the
//! `LlmClient::shutdown` STOP seam WITHOUT needing real Candle weights.
//!
//! The ONLY part not covered here is the concrete `CandleRuntime::load()`
//! minting the model_id from real weights — that requires an operator live-run
//! (`cargo test --features "test-utils,candle-runtime-engine"
//! candle_e2e_smoke`) with a captured ledger dump. Everything about the ledger
//! obligation itself (START on load with pid-less `os_pid=None` keyed on the
//! minted UUIDv7, STOP via the shutdown seam) is proven deterministically here.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use handshake_core::{
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    llm::{boot::assemble_local_runtime_client, DisabledLlmClient, LlmClient},
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoadSpec,
        LoraStackHandle, ModelCapabilities, ModelId, ModelRegistration, ModelRuntime,
        ModelRuntimeError, OperatorId, ProviderKind, RuntimeBinding, Score, SteeringHookHandle,
        TokenStream,
    },
    process_ledger::{
        LedgerBatcher, LedgerBatcherConfig, LedgerEvent, NoopOverflowSink, ProcessEngineKind,
        ProcessLedgerError, ProcessLedgerStore, ProcessStart, ProcessStop,
    },
};

/// A Flight Recorder sink that discards events (the ledger seam under test does
/// not emit Flight Recorder events).
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

/// A `ModelRuntime` that does nothing. `assemble_local_runtime_client` only
/// stores the runtimes in the router — it never loads/generates/embeds through
/// them — so a no-op runtime is sufficient to exercise the ledger seam.
struct NoopRuntime {
    capabilities: ModelCapabilities,
}

impl NoopRuntime {
    fn new() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
        }
    }
}

#[async_trait]
impl ModelRuntime for NoopRuntime {
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
        Ok(KvCacheHandle::new("noop-kv"))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(LoraStackHandle::new("noop-lora"))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(SteeringHookHandle::new("noop-steering"))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

/// Captures drained ledger rows so the test can assert on START/STOP shape.
#[derive(Default)]
struct InMemoryLedgerStore {
    events: Mutex<Vec<LedgerEvent>>,
}

impl InMemoryLedgerStore {
    fn snapshot(&self) -> Vec<LedgerEvent> {
        self.events.lock().expect("ledger store lock").clone()
    }

    fn starts(&self) -> Vec<ProcessStart> {
        self.snapshot()
            .into_iter()
            .filter_map(|event| match event {
                LedgerEvent::Start(start) => Some(start),
                LedgerEvent::Stop(_) => None,
            })
            .collect()
    }

    fn stops(&self) -> Vec<ProcessStop> {
        self.snapshot()
            .into_iter()
            .filter_map(|event| match event {
                LedgerEvent::Stop(stop) => Some(stop),
                LedgerEvent::Start(_) => None,
            })
            .collect()
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().expect("ledger store lock").extend(events);
        Ok(())
    }
}

fn embedded_registration(model_id: ModelId) -> ModelRegistration {
    ModelRegistration {
        model_id,
        artifact_path: std::path::PathBuf::from("fixtures/models/embedded-default.gguf"),
        sha256: [7; 32],
        runtime_binding: RuntimeBinding::Candle,
        declared_capabilities: ModelCapabilities::default(),
        base_model_tag: BaseModelTag::new("test-embedded-model"),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("handshake-embedded-default"),
        provider: ProviderKind::Local,
    }
}

#[tokio::test]
async fn embedded_model_load_emits_ledger_start_and_shutdown_seam_emits_stop() {
    // Manual (drain-based) ledger — no PostgreSQL, no background writer.
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());

    let model_id = ModelId::new_v7();
    let llama: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let candle: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let fallback: Arc<dyn LlmClient> = Arc::new(DisabledLlmClient::new(
        "embedded-fallback".to_string(),
        "no external fallback".to_string(),
    ));
    let recorder: Arc<dyn FlightRecorder> = Arc::new(NoopRecorder);

    // Drive the REAL production assembly seam WITH a ledger handle: this is the
    // exact function `build_default_local_client` calls after a real load, so the
    // START row is emitted by production code, not a test helper.
    let client = assemble_local_runtime_client(
        embedded_registration(model_id),
        llama,
        candle,
        fallback,
        recorder,
        8192,
        Some(ledger),
    )
    .expect("assemble embedded local runtime client with ledger");

    // --- START row (emitted on load, before any shutdown) ---
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain START row");

    let starts = store.starts();
    assert_eq!(
        starts.len(),
        1,
        "exactly one ProcessOwnershipLedger START row must be emitted on embedded load"
    );
    let start = &starts[0];
    assert_eq!(
        start.process_uuid,
        model_id.as_uuid(),
        "START row must be keyed on the minted model UUIDv7"
    );
    assert_eq!(
        start.os_pid, None,
        "in-process embedded load is pid-less: os_pid MUST be None (no synthetic pid)"
    );
    assert_eq!(
        start.engine_kind,
        ProcessEngineKind::Candle,
        "engine_kind must reflect the Candle runtime binding"
    );
    assert_eq!(
        start.metadata_jsonb["model_id"].as_str(),
        Some(model_id.to_string().as_str()),
        "START metadata must carry the minted model_id"
    );
    assert_eq!(
        start.metadata_jsonb["display_name"].as_str(),
        Some("test-embedded-model"),
        "START metadata must carry the display_name for MT-008 labeling"
    );
    assert!(
        store.stops().is_empty(),
        "no STOP row may exist before the shutdown seam is exercised"
    );

    // --- STOP row (emitted via the shutdown seam, NOT via unload()) ---
    client.shutdown();
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain STOP row");

    let stops = store.stops();
    assert_eq!(
        stops.len(),
        1,
        "the shutdown seam must emit exactly one matching STOP row"
    );
    let stop = &stops[0];
    assert_eq!(
        stop.process_uuid,
        model_id.as_uuid(),
        "STOP row must correlate to the START row via process_uuid"
    );
    assert_eq!(
        stop.os_pid, None,
        "STOP row must remain pid-less, matching the START row"
    );
    assert!(
        stop.stop_reason.is_some(),
        "STOP row must carry a stop_reason from the shutdown seam"
    );
}

#[tokio::test]
async fn shutdown_seam_is_idempotent_single_stop_row() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());

    let model_id = ModelId::new_v7();
    let llama: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let candle: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let fallback: Arc<dyn LlmClient> = Arc::new(DisabledLlmClient::new(
        "embedded-fallback".to_string(),
        "no external fallback".to_string(),
    ));
    let recorder: Arc<dyn FlightRecorder> = Arc::new(NoopRecorder);

    let client = assemble_local_runtime_client(
        embedded_registration(model_id),
        llama,
        candle,
        fallback,
        recorder,
        8192,
        Some(ledger),
    )
    .expect("assemble embedded local runtime client with ledger");

    // Call the shutdown seam twice; the Drop safety net will also fire when
    // `client` is dropped at end of scope. All must collapse to ONE STOP row.
    client.shutdown();
    client.shutdown();
    drop(client);

    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain rows");

    assert_eq!(
        store.stops().len(),
        1,
        "repeated shutdown + Drop safety-net must emit exactly one STOP row (idempotent)"
    );
}
