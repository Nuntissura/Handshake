//! WP-1 MT-013: ProcessOwnershipLedger START/STOP proof for the default
//! embedded-model load path.
//!
//! NON-SKIPPABLE BY DESIGN: this test has no `#[ignore]` and no
//! skip-if-no-weights guard, so it FAILS (never silently passes as "skipped")
//! if the load+shutdown seam does not emit the START/STOP rows. It drives the
//! explicit `test-utils` assembly seams with a minimal in-process runtime, so it
//! exercises lifecycle compatibility and the `LlmClient::shutdown_gracefully`
//! quiescence+STOP seam without pretending to be the production boot path. The
//! separate real-Candle proof is the production load/READY realism gate.
//!
//! The ONLY part not covered here is the concrete `CandleRuntime::load()`
//! minting the model_id from real weights — that requires an operator live-run
//! (`cargo test --features "test-utils,candle-runtime-engine"
//! candle_e2e_smoke`) with a captured ledger dump. Everything about the ledger
//! obligation itself (START on load with pid-less `os_pid=None` keyed on the
//! minted UUIDv7, worker quiescence + unique-owner runtime unload before STOP,
//! and no STOP on an unproven shutdown) is proven deterministically here.

use std::{
    collections::BTreeSet,
    net::{SocketAddr, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::{DateTime, Timelike, Utc};
use handshake_core::{
    flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError},
    llm::{
        boot::{assemble_local_runtime_client, assemble_local_runtime_client_with_registrations},
        embedded_ledger::EmbeddedModelProcess,
        DisabledLlmClient, LlmClient,
    },
    model_runtime::{
        BaseModelTag, CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoadSpec,
        LoraStackHandle, ModelCapabilities, ModelId, ModelRegistration, ModelRuntime,
        ModelRuntimeError, OperatorId, ProviderKind, RuntimeActivityKind, RuntimeActivityTracker,
        RuntimeBinding, RuntimeQuiesceError, Score, SteeringHookHandle, TokenStream,
    },
    process_ledger::{
        acquire_embedded_runtime_instance_lease, drain_and_join_ledger_writer,
        reclaim_pidless_embedded_orphans, resolve_embedded_runtime_host_scope_with_managed_local,
        resolve_embedded_runtime_host_scope_with_override, EmbeddedRuntimeInstanceDescriptor,
        LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome, LedgerEvent,
        LedgerOverflowEvent, LegacyHostScopeOpenRowProbe, NoopOverflowSink,
        PidlessEmbeddedReclaimReport, PostgresProcessLedgerStore, ProcessEngineKind,
        ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessStart,
        ProcessStop, StopRecordOutcome, EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX,
        PIDLESS_RECLAIM_INSTANCE_CAP,
    },
};
use sqlx::{postgres::PgPoolOptions, Connection};
use tokio::sync::{Notify, OnceCell};

mod knowledge_pg_support;

fn postgres_timestamp_precision(value: DateTime<Utc>) -> DateTime<Utc> {
    let nanos = value.timestamp_subsec_nanos();
    value
        .with_nanosecond(nanos - (nanos % 1_000))
        .unwrap_or(value)
}

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

struct FailingOverflowSink;

impl ProcessLedgerOverflowSink for FailingOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        Err(ProcessLedgerError::OverflowEmit(
            "forced overflow sink failure".to_string(),
        ))
    }
}

/// A `ModelRuntime` that does nothing. `assemble_local_runtime_client` only
/// stores the runtimes in the router — it never loads/generates/embeds through
/// them — so a no-op runtime is sufficient to exercise the ledger seam.
struct NoopRuntime {
    capabilities: ModelCapabilities,
    quiescence: TestQuiescence,
    unload: TestUnload,
}

enum TestQuiescence {
    Immediate,
    Tracked(RuntimeActivityTracker),
    Reject,
}

enum TestUnload {
    Immediate,
    Controlled(Arc<UnloadProbe>),
    Fail(String),
}

#[derive(Default)]
struct UnloadProbe {
    entered: AtomicBool,
    completed: AtomicBool,
    release: Notify,
}

impl NoopRuntime {
    fn new() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            quiescence: TestQuiescence::Immediate,
            unload: TestUnload::Immediate,
        }
    }

    fn tracked(tracker: RuntimeActivityTracker) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            quiescence: TestQuiescence::Tracked(tracker),
            unload: TestUnload::Immediate,
        }
    }

    fn rejecting() -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            quiescence: TestQuiescence::Reject,
            unload: TestUnload::Immediate,
        }
    }

    fn controlled_unload(probe: Arc<UnloadProbe>) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            quiescence: TestQuiescence::Immediate,
            unload: TestUnload::Controlled(probe),
        }
    }

    fn failing_unload(reason: &str) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            quiescence: TestQuiescence::Immediate,
            unload: TestUnload::Fail(reason.to_owned()),
        }
    }
}

#[async_trait]
impl ModelRuntime for NoopRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        match &self.unload {
            TestUnload::Immediate => Ok(()),
            TestUnload::Controlled(probe) => {
                probe.entered.store(true, Ordering::Release);
                probe.release.notified().await;
                probe.completed.store(true, Ordering::Release);
                Ok(())
            }
            TestUnload::Fail(reason) => Err(ModelRuntimeError::UnloadError(reason.clone())),
        }
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

    async fn quiesce(&self, timeout: Duration) -> Result<(), RuntimeQuiesceError> {
        match &self.quiescence {
            TestQuiescence::Immediate => Ok(()),
            TestQuiescence::Tracked(tracker) => tracker.quiesce(timeout).await,
            TestQuiescence::Reject => Err(RuntimeQuiesceError::Unsupported {
                adapter: "rejecting-test-runtime".to_string(),
            }),
        }
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
        self.events
            .lock()
            .expect("ledger store lock")
            .extend(events);
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

fn embedded_embedding_registration(model_id: ModelId) -> ModelRegistration {
    let mut registration = embedded_registration(model_id);
    registration.artifact_path =
        std::path::PathBuf::from("fixtures/models/embedded-embedding.safetensors");
    registration.sha256 = [8; 32];
    registration.declared_capabilities.supports_embedding = true;
    registration.declared_capabilities.embedding_dimension = Some(3);
    registration.base_model_tag = BaseModelTag::new("test-embedded-embedding-model");
    registration.registered_by = OperatorId::new("handshake-embedded-embedding");
    registration
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

    // --- unload completes, then the shutdown seam emits STOP ---
    client
        .shutdown_gracefully()
        .await
        .expect("quiesce and unload the runtime before STOP");
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
async fn embedded_stop_waits_for_exact_model_unload_completion() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());
    let model_id = ModelId::new_v7();
    let probe = Arc::new(UnloadProbe::default());
    let candle: Arc<dyn ModelRuntime> =
        Arc::new(NoopRuntime::controlled_unload(Arc::clone(&probe)));
    let client = Arc::new(
        assemble_local_runtime_client(
            embedded_registration(model_id),
            Arc::new(NoopRuntime::new()),
            candle,
            Arc::new(DisabledLlmClient::new(
                "embedded-fallback".to_string(),
                "no external fallback".to_string(),
            )),
            Arc::new(NoopRecorder),
            8192,
            Some(ledger),
        )
        .expect("assemble controlled-unload embedded runtime"),
    );
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain START before controlled unload");

    let shutdown = tokio::spawn({
        let client = Arc::clone(&client);
        async move { client.shutdown_gracefully().await }
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while !probe.entered.load(Ordering::Acquire) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("runtime unload begins");

    drain
        .drain_available_to(store.clone())
        .await
        .expect("inspect ledger while unload is blocked");
    assert!(
        store.stops().is_empty(),
        "STOP must not be emitted while ModelRuntime::unload is still pending"
    );
    assert!(
        !shutdown.is_finished(),
        "shutdown must await unload completion"
    );

    probe.release.notify_one();
    shutdown
        .await
        .expect("controlled-unload shutdown task joins")
        .expect("controlled unload and graceful STOP succeed");
    assert!(probe.completed.load(Ordering::Acquire));
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain STOP after unload completion");
    assert_eq!(store.stops().len(), 1);
    assert_eq!(store.stops()[0].process_uuid, model_id.as_uuid());
}

#[tokio::test]
async fn embedded_unload_failure_leaves_start_open_without_stop() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());
    let model_id = ModelId::new_v7();
    let client = assemble_local_runtime_client(
        embedded_registration(model_id),
        Arc::new(NoopRuntime::new()),
        Arc::new(NoopRuntime::failing_unload("injected unload failure")),
        Arc::new(DisabledLlmClient::new(
            "embedded-fallback".to_string(),
            "no external fallback".to_string(),
        )),
        Arc::new(NoopRecorder),
        8192,
        Some(ledger),
    )
    .expect("assemble failing-unload embedded runtime");

    let error = client
        .shutdown_gracefully()
        .await
        .expect_err("runtime unload failure must fail graceful shutdown closed");
    assert!(
        error.to_string().contains("injected unload failure"),
        "typed shutdown error must preserve the unload failure: {error}"
    );
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain lifecycle after failed unload");
    assert_eq!(store.starts().len(), 1);
    assert!(
        store.stops().is_empty(),
        "failed unload must leave START open for reconciliation"
    );
}

#[tokio::test]
async fn embedded_extra_runtime_arc_owner_blocks_unload_and_stop() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());
    let model_id = ModelId::new_v7();
    let candle: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let retained_owner = Arc::clone(&candle);
    let client = assemble_local_runtime_client(
        embedded_registration(model_id),
        Arc::new(NoopRuntime::new()),
        candle,
        Arc::new(DisabledLlmClient::new(
            "embedded-fallback".to_string(),
            "no external fallback".to_string(),
        )),
        Arc::new(NoopRecorder),
        8192,
        Some(ledger),
    )
    .expect("assemble shared-owner embedded runtime");

    let error = client
        .shutdown_gracefully()
        .await
        .expect_err("an extra runtime Arc owner must block unload proof");
    assert!(
        error.to_string().contains("cannot prove final ownership"),
        "unexpected shared-owner shutdown error: {error}"
    );
    drop(retained_owner);
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain lifecycle after shared-owner rejection");
    assert_eq!(store.starts().len(), 1);
    assert!(
        store.stops().is_empty(),
        "unproven final ownership must leave START open for reconciliation"
    );
}

#[tokio::test]
async fn optional_embedding_model_load_emits_matching_ledger_start_stop() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());

    let chat_model_id = ModelId::new_v7();
    let embedding_model_id = ModelId::new_v7();
    let llama: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let candle: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let fallback: Arc<dyn LlmClient> = Arc::new(DisabledLlmClient::new(
        "embedded-fallback".to_string(),
        "no external fallback".to_string(),
    ));
    let recorder: Arc<dyn FlightRecorder> = Arc::new(NoopRecorder);

    let client = assemble_local_runtime_client_with_registrations(
        embedded_registration(chat_model_id),
        vec![embedded_embedding_registration(embedding_model_id)],
        llama,
        candle,
        fallback,
        recorder,
        8192,
        Some(ledger),
    )
    .expect("assemble embedded local runtime client with chat + embedding ledger handles");

    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain START rows");
    let start_ids = store
        .starts()
        .into_iter()
        .map(|start| {
            assert_eq!(start.os_pid, None);
            assert_eq!(start.engine_kind, ProcessEngineKind::Candle);
            start.process_uuid
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        start_ids,
        BTreeSet::from([chat_model_id.as_uuid(), embedding_model_id.as_uuid()]),
        "both default chat and optional embedding model loads must emit START rows"
    );

    client
        .shutdown_gracefully()
        .await
        .expect("quiesce runtimes before STOP");
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain STOP rows");
    let stop_ids = store
        .stops()
        .into_iter()
        .map(|stop| {
            assert_eq!(stop.os_pid, None);
            assert_eq!(stop.stop_reason.as_deref(), Some("llm-client-shutdown"));
            stop.process_uuid
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        stop_ids, start_ids,
        "every embedded START row must have a matching STOP row on shutdown"
    );
}

#[tokio::test]
async fn retired_source_lifecycle_does_not_block_replacement_graceful_stop() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());
    let source_model_id = ModelId::new_v7();
    let replacement_model_id = ModelId::new_v7();
    let client = assemble_local_runtime_client_with_registrations(
        embedded_registration(source_model_id),
        vec![embedded_embedding_registration(replacement_model_id)],
        Arc::new(NoopRuntime::new()),
        Arc::new(NoopRuntime::new()),
        Arc::new(DisabledLlmClient::new(
            "embedded-fallback".to_string(),
            "no external fallback".to_string(),
        )),
        Arc::new(NoopRecorder),
        8192,
        Some(ledger),
    )
    .expect("assemble source plus replacement lifecycle set");

    assert!(
        client.retire_embedded_process_for_tests(source_model_id.as_uuid()),
        "runtime-control retirement must remove the exact unloaded source lifecycle"
    );
    client
        .shutdown_gracefully()
        .await
        .expect("stale source must not block replacement graceful shutdown");
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain replacement STOP");

    let stops = store.stops();
    assert_eq!(
        stops.len(),
        1,
        "retired source remains open while the still-loaded replacement stops exactly once"
    );
    assert_eq!(stops[0].process_uuid, replacement_model_id.as_uuid());
    assert_eq!(stops[0].stop_reason.as_deref(), Some("llm-client-shutdown"));

    let source = include_str!("../src/llm/local_router.rs");
    assert!(source.contains("self.remove_embedded_process(model_id.as_uuid());"));
    assert!(
        source
            .matches("self.remove_embedded_process(source_process.process_uuid());")
            .count()
            >= 2,
        "adapter-swap success and durable-rebind failure must both retire the unloaded source"
    );
}

#[tokio::test]
async fn supplied_ledger_start_failure_fails_local_client_assembly_closed() {
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 1,
            batch_size: 1,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(FailingOverflowSink),
    )
    .expect("manual ledger batcher");
    ledger
        .record_start(ProcessStart::new(
            ProcessEngineKind::Candle,
            "pre-filled-ledger",
            None,
        ))
        .expect("fill single-slot ledger channel");

    let model_id = ModelId::new_v7();
    let llama: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let candle: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let fallback: Arc<dyn LlmClient> = Arc::new(DisabledLlmClient::new(
        "embedded-fallback".to_string(),
        "no external fallback".to_string(),
    ));
    let recorder: Arc<dyn FlightRecorder> = Arc::new(NoopRecorder);

    let err = match assemble_local_runtime_client(
        embedded_registration(model_id),
        llama,
        candle,
        fallback,
        recorder,
        8192,
        Some(ledger),
    ) {
        Ok(_) => panic!("supplied ledger START failure must fail local client assembly closed"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("complete embedded ProcessOwnershipLedger lifecycle reservation failed"),
        "{err}"
    );
}

#[tokio::test]
async fn supplied_ledger_start_overflow_fails_local_client_assembly_closed() {
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 1,
            batch_size: 1,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual ledger batcher");
    ledger
        .record_start(ProcessStart::new(
            ProcessEngineKind::Candle,
            "pre-filled-ledger",
            None,
        ))
        .expect("fill single-slot ledger channel");

    let model_id = ModelId::new_v7();
    let llama: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let candle: Arc<dyn ModelRuntime> = Arc::new(NoopRuntime::new());
    let fallback: Arc<dyn LlmClient> = Arc::new(DisabledLlmClient::new(
        "embedded-fallback".to_string(),
        "no external fallback".to_string(),
    ));
    let recorder: Arc<dyn FlightRecorder> = Arc::new(NoopRecorder);

    let err = match assemble_local_runtime_client(
        embedded_registration(model_id),
        llama,
        candle,
        fallback,
        recorder,
        8192,
        Some(ledger),
    ) {
        Ok(_) => panic!("supplied ledger START overflow must fail local client assembly closed"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("complete embedded ProcessOwnershipLedger lifecycle reservation failed"),
        "{err}"
    );
}

#[tokio::test]
async fn embedded_model_reserved_stop_survives_unrelated_queue_saturation() {
    let (ledger, drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 3,
            batch_size: 3,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual ledger batcher");

    let model_id = ModelId::new_v7();
    let embedded_process = EmbeddedModelProcess::record_load(
        ledger.clone(),
        RuntimeBinding::Candle,
        model_id,
        "overflow-stop-model",
        Some("sha256-overflow-stop".to_string()),
    )
    .expect("initial embedded START row fits");
    ledger
        .record_start(ProcessStart::new(
            ProcessEngineKind::Candle,
            "pre-filled-ledger",
            None,
        ))
        .expect("fill second ledger slot");

    embedded_process
        .shutdown("overflow-stop-test")
        .expect("reserved STOP does not compete with unrelated queue occupancy");

    let store = Arc::new(InMemoryLedgerStore::default());
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain saturated queue with reserved STOP");
    assert_eq!(store.starts().len(), 2);
    let stops = store.stops();
    assert_eq!(stops.len(), 1);
    assert_eq!(stops[0].process_uuid, model_id.as_uuid());
}

#[tokio::test]
async fn embedded_model_shutdown_bounded_waits_for_authoritative_stop_durability() {
    let store = Arc::new(InMemoryLedgerStore::default());
    let (ledger, writer_join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 2,
            batch_size: 2,
            ..LedgerBatcherConfig::default()
        },
    );
    let model_id = ModelId::new_v7();
    let embedded_process = EmbeddedModelProcess::record_load(
        ledger.clone(),
        RuntimeBinding::Candle,
        model_id,
        "bounded-stop-model",
        Some("sha256-bounded-stop".to_string()),
    )
    .expect("START and future STOP capacity are both preaccepted");

    embedded_process
        .shutdown_bounded("reserved-stop-durable", Duration::from_secs(2))
        .await
        .expect("reserved STOP is durably acknowledged before shutdown returns");

    let drain_outcome =
        drain_and_join_ledger_writer(&ledger, writer_join, Duration::from_secs(2)).await;
    assert!(matches!(drain_outcome, LedgerDrainJoinOutcome::Flushed));
    assert_eq!(store.starts().len(), 1);
    let stops = store.stops();
    assert_eq!(stops.len(), 1, "recovered capacity must retain one STOP");
    assert_eq!(stops[0].process_uuid, model_id.as_uuid());
    assert_eq!(
        stops[0].stop_reason.as_deref(),
        Some("reserved-stop-durable")
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

    // Call the proven graceful seam twice. Both calls must collapse to one STOP.
    client
        .shutdown_gracefully()
        .await
        .expect("first graceful shutdown");
    client
        .shutdown_gracefully()
        .await
        .expect("second graceful shutdown is idempotent");
    drop(client);

    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain rows");

    assert_eq!(
        store.stops().len(),
        1,
        "repeated proven graceful shutdown must emit exactly one STOP row (idempotent)"
    );
}

#[tokio::test]
async fn dropping_unquiesced_client_leaves_start_open_for_reconciliation() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());
    let model_id = ModelId::new_v7();

    let client = assemble_local_runtime_client(
        embedded_registration(model_id),
        Arc::new(NoopRuntime::new()),
        Arc::new(NoopRuntime::new()),
        Arc::new(DisabledLlmClient::new(
            "embedded-fallback".to_string(),
            "no external fallback".to_string(),
        )),
        Arc::new(NoopRecorder),
        8192,
        Some(ledger),
    )
    .expect("assemble embedded local runtime client with ledger");

    drop(client);
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain open lifecycle");

    assert_eq!(store.starts().len(), 1);
    assert!(
        store.stops().is_empty(),
        "Drop cannot prove detached runtime work stopped and must not forge STOP"
    );
}

#[tokio::test]
async fn graceful_stop_waits_for_the_exact_loaded_runtime_worker_guard() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());
    let model_id = ModelId::new_v7();
    let tracker = RuntimeActivityTracker::new();
    let worker_guard = tracker
        .try_register(model_id, RuntimeActivityKind::Embed, None)
        .expect("admit controlled embedded worker");

    let client = Arc::new(
        assemble_local_runtime_client(
            embedded_registration(model_id),
            // This unused binding rejects quiescence. Success therefore proves
            // shutdown resolves the exact process UUID and does not call every
            // router handle indiscriminately.
            Arc::new(NoopRuntime::rejecting()),
            Arc::new(NoopRuntime::tracked(tracker.clone())),
            Arc::new(DisabledLlmClient::new(
                "embedded-fallback".to_string(),
                "no external fallback".to_string(),
            )),
            Arc::new(NoopRecorder),
            8192,
            Some(ledger),
        )
        .expect("assemble tracked embedded runtime client"),
    );

    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain START before shutdown");
    let shutdown = tokio::spawn({
        let client = Arc::clone(&client);
        async move { client.shutdown_gracefully().await }
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while tracker.is_accepting() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("loaded runtime quiescence begins");

    drain
        .drain_available_to(store.clone())
        .await
        .expect("inspect ledger while worker remains active");
    assert!(
        store.stops().is_empty(),
        "STOP must not precede the actual loaded runtime worker guard"
    );
    assert!(!shutdown.is_finished());

    drop(worker_guard);
    shutdown
        .await
        .expect("shutdown task joins")
        .expect("shutdown succeeds after the exact worker exits");
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain proven STOP");
    assert_eq!(store.stops().len(), 1);
    assert_eq!(store.stops()[0].process_uuid, model_id.as_uuid());
}

#[tokio::test]
async fn failed_runtime_quiescence_abandons_stop_and_drop_cannot_reclassify_success() {
    let (ledger, drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    let store = Arc::new(InMemoryLedgerStore::default());
    let model_id = ModelId::new_v7();
    let client = assemble_local_runtime_client(
        embedded_registration(model_id),
        Arc::new(NoopRuntime::new()),
        Arc::new(NoopRuntime::rejecting()),
        Arc::new(DisabledLlmClient::new(
            "embedded-fallback".to_string(),
            "no external fallback".to_string(),
        )),
        Arc::new(NoopRecorder),
        8192,
        Some(ledger),
    )
    .expect("assemble rejecting embedded runtime client");

    let error = client
        .shutdown_gracefully()
        .await
        .expect_err("unsupported loaded runtime quiescence must fail closed");
    assert!(error.to_string().contains("rejecting-test-runtime"));
    drop(client);
    drain
        .drain_available_to(store.clone())
        .await
        .expect("drain open lifecycle after failed quiescence");

    assert_eq!(store.starts().len(), 1);
    assert!(
        store.stops().is_empty(),
        "failed quiescence and later Drop must leave START open"
    );
}

/// WP-1 MT-013 (F1a): the graceful-shutdown SEQUENCE — shutdown seam emits the
/// STOP, then a bounded close + drain-and-join over the REAL spawned background
/// writer (the production main.rs path, NOT the manual drain) — flushes the STOP
/// row to the store. This proves the STOP genuinely reaches durable storage at
/// shutdown, not just that the seam enqueues it.
#[tokio::test]
async fn graceful_shutdown_sequence_flushes_stop_through_background_writer() {
    let store = Arc::new(InMemoryLedgerStore::default());
    let (ledger, writer_join) = LedgerBatcher::spawn(
        store.clone(),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    // Clone kept for the shutdown close signal; the original is moved into the
    // client (which owns the embedded-model STOP seam), mirroring main.rs.
    let ledger_close = ledger.clone();

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

    // Graceful-shutdown SEQUENCE step 1: the shutdown seam enqueues the STOP row.
    client
        .shutdown_gracefully()
        .await
        .expect("bounded graceful STOP enqueue");

    // Step 2: bounded drain-and-join closes the writer channel and awaits the
    // background writer, flushing START + STOP to the store before teardown.
    let outcome =
        drain_and_join_ledger_writer(&ledger_close, writer_join, Duration::from_secs(5)).await;
    assert!(
        matches!(outcome, LedgerDrainJoinOutcome::Flushed),
        "graceful-shutdown drain-and-join must flush cleanly, got {outcome:?}"
    );

    let starts = store.starts();
    assert_eq!(
        starts.len(),
        1,
        "the embedded START row must be flushed by the background writer at shutdown"
    );
    let stops = store.stops();
    assert_eq!(
        stops.len(),
        1,
        "the shutdown seam's STOP row must be flushed by the background writer's drain-and-join"
    );
    assert_eq!(
        stops[0].process_uuid,
        model_id.as_uuid(),
        "flushed STOP row must correlate to the embedded model's START via process_uuid"
    );
    assert_eq!(
        stops[0].os_pid, None,
        "flushed STOP row must remain pid-less"
    );

    // `client` still holds a LedgerBatcher clone; dropping it after the writer
    // already terminated must be a no-op (begin_close is idempotent and the
    // explicit proven STOP already consumed its reserved permit).
    drop(client);
}

// ===========================================================================
// WP-1 MT-013 V2: hard-crash orphan reconciliation. These tests use real
// PostgreSQL plus real loopback UDP sockets. Database-session loss is
// intentionally independent of process liveness.
// ===========================================================================

static ORPHAN_PG_SERIAL: OnceCell<Arc<tokio::sync::Mutex<()>>> = OnceCell::const_new();

struct OrphanTestDb {
    base_url: String,
    schema: String,
    schema_url: String,
    pool: sqlx::PgPool,
    _serial_guard: tokio::sync::OwnedMutexGuard<()>,
}

#[derive(Clone)]
struct ConflictObservingPgStore {
    inner: Arc<PostgresProcessLedgerStore>,
    first_batch: Arc<Mutex<Option<Vec<LedgerEvent>>>>,
    conflict_seen: Arc<AtomicBool>,
    conflict_notify: Arc<Notify>,
}

impl ConflictObservingPgStore {
    fn new(inner: Arc<PostgresProcessLedgerStore>) -> Self {
        Self {
            inner,
            first_batch: Arc::new(Mutex::new(None)),
            conflict_seen: Arc::new(AtomicBool::new(false)),
            conflict_notify: Arc::new(Notify::new()),
        }
    }

    async fn wait_for_conflict(&self) {
        if self.conflict_seen.load(Ordering::SeqCst) {
            return;
        }
        let notified = self.conflict_notify.notified();
        if self.conflict_seen.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::timeout(Duration::from_secs(2), notified)
            .await
            .expect("real PostgreSQL co-batch collision was not observed");
    }

    fn first_batch(&self) -> Vec<LedgerEvent> {
        self.first_batch
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .expect("writer submitted a first PostgreSQL batch")
    }
}

#[async_trait]
impl ProcessLedgerStore for ConflictObservingPgStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        {
            let mut first_batch = self
                .first_batch
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if first_batch.is_none() {
                *first_batch = Some(events.clone());
            }
        }
        let result = self.inner.write_batch(events).await;
        if matches!(
            result,
            Err(ProcessLedgerError::StartIdentityConflict { .. })
        ) {
            self.conflict_seen.store(true, Ordering::SeqCst);
            self.conflict_notify.notify_waiters();
        }
        result
    }
}

async fn managed_pg_handle() -> &'static handshake_core::managed_postgres::ManagedPostgres {
    knowledge_pg_support::task_owned_managed_postgres().await
}

async fn managed_pg_base_url() -> String {
    managed_pg_handle().await.database_url()
}

fn url_with_search_path(base_url: &str, encoded_search_path: &str) -> String {
    let separator = if base_url.contains('?') { "&" } else { "?" };
    format!("{base_url}{separator}options=-csearch_path%3D{encoded_search_path}")
}

async fn orphan_reclaim_schema_pool() -> OrphanTestDb {
    // These tests deliberately install hostile locks, RLS policies, triggers,
    // and very small PostgreSQL deadlines. Isolated schemas prevent data
    // collisions, but parallel catalog/lock pressure can still consume those
    // deadlines and turn unrelated tests into false failures. Hold one permit
    // for the complete schema-pool lifetime so the default test runner remains
    // deterministic without requiring a special --test-threads flag.
    let serial_guard = ORPHAN_PG_SERIAL
        .get_or_init(|| async { Arc::new(tokio::sync::Mutex::new(())) })
        .await
        .clone()
        .lock_owned()
        .await;
    let base_url = managed_pg_base_url().await;
    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect Handshake-managed postgres");
    let schema = format!("mt013_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut conn)
        .await
        .expect("create isolated schema");
    drop(conn);
    let schema_url = url_with_search_path(&base_url, &schema);
    let pool = sqlx::PgPool::connect(&schema_url)
        .await
        .expect("connect isolated schema");
    sqlx::raw_sql(include_str!(
        "../migrations/0021_kernel_process_lifecycle.sql"
    ))
    .execute(&pool)
    .await
    .expect("apply kernel_process_lifecycle migration");
    sqlx::raw_sql(
        r#"
        CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_pidless_embedded_instance_open
            ON kernel_process_lifecycle (
                (metadata_jsonb->>'runtime_instance_id'),
                process_uuid,
                started_at
            )
            WHERE parent_session_id IS NULL
              AND os_pid IS NULL
              AND stopped_at IS NULL
              AND exit_code IS NULL
              AND stop_reason IS NULL
              AND engine_kind IN ('llamacpp', 'candle');
        CREATE TABLE IF NOT EXISTS kernel_pidless_embedded_reclaim_cursor (
            host_scope_id TEXT PRIMARY KEY,
            last_instance_id TEXT,
            updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.clock_timestamp()
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("apply bounded pid-less embedded reclaim index from migration 0348");
    OrphanTestDb {
        base_url,
        schema,
        schema_url,
        pool,
        _serial_guard: serial_guard,
    }
}

#[tokio::test]
async fn postgres_writer_pins_logged_authority_and_synchronous_commit() {
    let db = orphan_reclaim_schema_pool().await;
    let shadow = format!("mt013_writer_shadow_{}", uuid::Uuid::now_v7().simple());
    let mut admin = sqlx::PgConnection::connect(&db.base_url)
        .await
        .expect("connect for process-ledger writer shadow setup");
    sqlx::query(&format!(r#"CREATE SCHEMA "{shadow}""#))
        .execute(&mut admin)
        .await
        .expect("create persistent writer shadow schema");
    sqlx::query(&format!(
        r#"CREATE TABLE "{shadow}".kernel_process_lifecycle (junk TEXT)"#
    ))
    .execute(&mut admin)
    .await
    .expect("create incomplete persistent writer shadow");
    drop(admin);

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db.schema_url)
        .await
        .expect("connect one-session writer authority pool");
    {
        let mut connection = pool
            .acquire()
            .await
            .expect("acquire writer authority session for poisoning");
        sqlx::query(&format!(
            r#"CREATE TEMP TABLE kernel_process_lifecycle
               (LIKE "{}".kernel_process_lifecycle INCLUDING ALL)"#,
            db.schema
        ))
        .execute(&mut *connection)
        .await
        .expect("create complete temporary lifecycle shadow");
        let poisoned_path: String =
            sqlx::query_scalar("SELECT pg_catalog.set_config('search_path', $1, false)")
                .bind(format!(
                    r#""{shadow}", "{}", pg_catalog, pg_temp"#,
                    db.schema
                ))
                .fetch_one(&mut *connection)
                .await
                .expect("poison writer session search path");
        assert!(poisoned_path.contains(&shadow));
        let session_sync: String =
            sqlx::query_scalar("SELECT pg_catalog.set_config('synchronous_commit', 'off', false)")
                .fetch_one(&mut *connection)
                .await
                .expect("poison writer session synchronous_commit");
        assert_eq!(session_sync, "off");
        let (fsync, full_page_writes): (String, String) = sqlx::query_as(
            "SELECT pg_catalog.current_setting('fsync'), pg_catalog.current_setting('full_page_writes')",
        )
        .fetch_one(&mut *connection)
        .await
        .expect("read crash-durability settings for ledger proof");
        assert_eq!(fsync, "on", "ledger durability proof requires fsync=on");
        assert_eq!(
            full_page_writes, "on",
            "ledger durability proof requires full_page_writes=on"
        );
    }

    let store = PostgresProcessLedgerStore::new(pool.clone());
    let first = ProcessStart::new(ProcessEngineKind::Candle, "writer-authority-proof", None);
    store
        .write_batch(vec![LedgerEvent::Start(first.clone())])
        .await
        .expect("canonical logged authority accepts synchronously committed START");

    let canonical_rows: i64 = sqlx::query_scalar(&format!(
        r#"SELECT pg_catalog.count(*) FROM "{}".kernel_process_lifecycle"#,
        db.schema
    ))
    .fetch_one(&pool)
    .await
    .expect("count canonical writer rows");
    let persistent_shadow_rows: i64 = sqlx::query_scalar(&format!(
        r#"SELECT pg_catalog.count(*) FROM "{shadow}".kernel_process_lifecycle"#
    ))
    .fetch_one(&pool)
    .await
    .expect("count persistent writer shadow rows");
    let temporary_shadow_rows: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM pg_temp.kernel_process_lifecycle")
            .fetch_one(&pool)
            .await
            .expect("count temporary writer shadow rows");
    assert_eq!(canonical_rows, 1);
    assert_eq!(persistent_shadow_rows, 0);
    assert_eq!(temporary_shadow_rows, 0);
    let session_sync_after_commit: String =
        sqlx::query_scalar("SELECT pg_catalog.current_setting('synchronous_commit')")
            .fetch_one(&pool)
            .await
            .expect("read writer session default after commit");
    assert_eq!(session_sync_after_commit, "off");

    sqlx::query(&format!(
        r#"ALTER TABLE "{}".kernel_process_lifecycle SET UNLOGGED"#,
        db.schema
    ))
    .execute(&db.pool)
    .await
    .expect("convert cached canonical relation to UNLOGGED");
    let second = ProcessStart::new(ProcessEngineKind::Candle, "writer-drift-proof", None);
    let drift_error = store
        .write_batch(vec![LedgerEvent::Start(second.clone())])
        .await
        .expect_err("cached writer authority must reject persistence drift");
    assert!(
        drift_error.to_string().contains("crash-durability class"),
        "writer drift error must identify lost crash durability: {drift_error}"
    );
    let fresh_error = PostgresProcessLedgerStore::new(pool)
        .write_batch(vec![LedgerEvent::Start(second)])
        .await
        .expect_err("fresh writer authority resolution must reject UNLOGGED relation");
    assert!(
        fresh_error.to_string().contains("no migration-0021-shaped"),
        "fresh writer must find no logged authority: {fresh_error}"
    );
}

#[tokio::test]
async fn durable_start_ack_rejects_uuid_reuse_without_emitting_stop() {
    let db = orphan_reclaim_schema_pool().await;
    let store = Arc::new(PostgresProcessLedgerStore::new(db.pool.clone()));
    let open_start = ProcessStart::new(ProcessEngineKind::Candle, "original-open", None)
        .with_metadata_jsonb(serde_json::json!({"identity": "original-open"}));
    store
        .write_batch(vec![LedgerEvent::Start(open_start.clone())])
        .await
        .expect("seed original open lifecycle");

    let stopped_start = ProcessStart::new(ProcessEngineKind::Candle, "original-stopped", None)
        .with_metadata_jsonb(serde_json::json!({"identity": "original-stopped"}));
    let stopped = ProcessStop::from_start(&stopped_start, Some(0))
        .with_stop_reason("original-terminal-outcome");
    let original_stopped_at = stopped.stopped_at;
    store
        .write_batch(vec![
            LedgerEvent::Start(stopped_start.clone()),
            LedgerEvent::Stop(stopped),
        ])
        .await
        .expect("seed original terminal lifecycle");

    let (batcher, join) = LedgerBatcher::spawn(
        store,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 6,
            batch_size: 1,
            flush_interval: Duration::from_secs(60),
        },
    );

    let exact_reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve exact retry lifecycle")
        .pop()
        .expect("one exact retry reservation");
    let (exact_active, exact_ack) = exact_reservation
        .begin_with_durable_ack_for_test(open_start.clone())
        .expect("begin exact open retry");
    exact_ack
        .wait(Duration::from_secs(2))
        .await
        .expect("exact open START retry is idempotently acknowledged");
    assert!(exact_active.leave_open_for_reconciliation());
    drop(exact_active);

    let mut conflicting_start = open_start.clone();
    conflicting_start.owner_role = "conflicting-owner".to_string();
    let conflict_reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve conflicting retry lifecycle")
        .pop()
        .expect("one conflicting retry reservation");
    let (conflict_active, conflict_ack) = conflict_reservation
        .begin_with_durable_ack_for_test(conflicting_start)
        .expect("enqueue conflicting open retry");
    let conflict_error = conflict_ack
        .wait(Duration::from_secs(2))
        .await
        .expect_err("different lifecycle identity must be rejected");
    assert!(matches!(
        conflict_error,
        ProcessLedgerError::DurabilityRejected { .. }
    ));
    assert_eq!(
        conflict_active
            .stop(Some(-1), "must-not-overwrite-original-open")
            .expect("suppressed rejected STOP is observable"),
        StopRecordOutcome::LeftOpenForReconciliation
    );
    drop(conflict_active);

    let mut legacy_conflicting_start = open_start.clone();
    legacy_conflicting_start.owner_role = "legacy-conflicting-owner".to_string();
    let legacy_conflict_reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve legacy conflicting lifecycle")
        .pop()
        .expect("one legacy conflicting reservation");
    let legacy_conflict_active = legacy_conflict_reservation
        .begin(legacy_conflicting_start)
        .expect("enqueue legacy conflicting START");
    drop(legacy_conflict_active);

    let stopped_reservation = batcher
        .try_reserve_lifecycles(1)
        .expect("reserve stopped-row retry lifecycle")
        .pop()
        .expect("one stopped-row retry reservation");
    let (stopped_active, stopped_ack) = stopped_reservation
        .begin_with_durable_ack_for_test(stopped_start.clone())
        .expect("enqueue exact stopped-row START retry");
    let stopped_error = stopped_ack
        .wait(Duration::from_secs(2))
        .await
        .expect_err("terminal lifecycle must never be acknowledged as a fresh START");
    assert!(matches!(
        stopped_error,
        ProcessLedgerError::DurabilityRejected { .. }
    ));
    assert_eq!(
        stopped_active
            .stop(Some(-1), "must-not-overwrite-terminal-row")
            .expect("suppressed terminal-row STOP is observable"),
        StopRecordOutcome::LeftOpenForReconciliation
    );
    drop(stopped_active);

    batcher.begin_close();
    tokio::time::timeout(Duration::from_secs(2), join)
        .await
        .expect("writer closes after identity probes")
        .expect("writer task joins")
        .expect("writer drains after identity probes");

    let open_row: (String, Option<chrono::DateTime<Utc>>, Option<i32>, Option<String>) =
        sqlx::query_as(
            "SELECT owner_role, stopped_at, exit_code, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(open_start.process_uuid)
        .fetch_one(&db.pool)
        .await
        .expect("read original open lifecycle after rejected collision");
    assert_eq!(open_row.0, "original-open");
    assert_eq!(open_row.1, None);
    assert_eq!(open_row.2, None);
    assert_eq!(open_row.3, None);

    let stopped_row: (chrono::DateTime<Utc>, Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT stopped_at, exit_code, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(stopped_start.process_uuid)
    .fetch_one(&db.pool)
    .await
    .expect("read original terminal lifecycle after rejected replay");
    assert_eq!(
        stopped_row.0,
        postgres_timestamp_precision(original_stopped_at)
    );
    assert_eq!(stopped_row.1, Some(0));
    assert_eq!(stopped_row.2.as_deref(), Some("original-terminal-outcome"));
}

#[tokio::test]
async fn postgres_writer_rejects_conflicting_stop_without_mutating_original_start() {
    let db = orphan_reclaim_schema_pool().await;
    let store = PostgresProcessLedgerStore::new(db.pool.clone());
    let start = ProcessStart::new(ProcessEngineKind::Candle, "original-stop-identity", None)
        .with_role_id("original-stop-identity")
        .with_wp_id("wp-stop-identity")
        .with_mt_id("mt-stop-identity")
        .with_metadata_jsonb(serde_json::json!({"identity": "original"}));
    store
        .write_batch(vec![LedgerEvent::Start(start.clone())])
        .await
        .expect("seed original open START");
    let original_row: String = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.to_jsonb(lifecycle)::pg_catalog.text
        FROM ONLY kernel_process_lifecycle AS lifecycle
        WHERE process_uuid = $1
        "#,
    )
    .bind(start.process_uuid)
    .fetch_one(&db.pool)
    .await
    .expect("capture original lifecycle row");

    let mut conflicting_stop = ProcessStop::from_start(&start, Some(-1));
    conflicting_stop.owner_role = "forged-conflicting-stop-owner".to_string();
    let error = store
        .write_batch(vec![LedgerEvent::Stop(conflicting_stop)])
        .await
        .expect_err("conflicting STOP identity must fail closed");
    assert!(
        matches!(&error, ProcessLedgerError::StopIdentityConflict { .. }),
        "STOP conflict must surface as a typed identity conflict: {error}"
    );
    assert!(error
        .to_string()
        .contains("PROCESS_LEDGER_STOP_IDENTITY_CONFLICT"));

    let after_row: String = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.to_jsonb(lifecycle)::pg_catalog.text
        FROM ONLY kernel_process_lifecycle AS lifecycle
        WHERE process_uuid = $1
        "#,
    )
    .bind(start.process_uuid)
    .fetch_one(&db.pool)
    .await
    .expect("capture lifecycle row after rejected STOP");
    assert_eq!(
        after_row, original_row,
        "rejected conflicting STOP must leave the original row byte-equivalent"
    );
    assert!(
        stopped_at(&db.pool, start.process_uuid).await.is_none(),
        "rejected conflicting STOP must leave the original START open"
    );
}

#[tokio::test]
async fn postgres_writer_allows_exact_stop_replay_but_rejects_terminal_outcome_rewrite() {
    let db = orphan_reclaim_schema_pool().await;
    let store = PostgresProcessLedgerStore::new(db.pool.clone());
    let start = ProcessStart::new(ProcessEngineKind::Candle, "terminal-history-owner", None)
        .with_metadata_jsonb(serde_json::json!({"identity": "terminal-history"}));
    store
        .write_batch(vec![LedgerEvent::Start(start.clone())])
        .await
        .expect("seed terminal-history START");
    let original_stop =
        ProcessStop::from_start(&start, Some(0)).with_stop_reason("original-terminal-outcome");
    store
        .write_batch(vec![LedgerEvent::Stop(original_stop.clone())])
        .await
        .expect("write original STOP");
    store
        .write_batch(vec![LedgerEvent::Stop(original_stop.clone())])
        .await
        .expect("exact STOP replay must remain idempotent");
    let original_terminal_row: String = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.to_jsonb(lifecycle)::pg_catalog.text
        FROM ONLY kernel_process_lifecycle AS lifecycle
        WHERE process_uuid = $1
        "#,
    )
    .bind(start.process_uuid)
    .fetch_one(&db.pool)
    .await
    .expect("capture original terminal lifecycle row");

    let mut rewrite = original_stop;
    rewrite.stopped_at += chrono::Duration::seconds(1);
    rewrite.exit_code = Some(-9);
    rewrite.stop_reason = Some("forged-terminal-rewrite".to_string());
    let error = store
        .write_batch(vec![LedgerEvent::Stop(rewrite)])
        .await
        .expect_err("different terminal outcome must not rewrite history");
    assert!(matches!(
        &error,
        ProcessLedgerError::StopIdentityConflict { .. }
    ));
    assert!(error
        .to_string()
        .contains("PROCESS_LEDGER_STOP_IDENTITY_CONFLICT"));

    let after_terminal_row: String = sqlx::query_scalar(
        r#"
        SELECT pg_catalog.to_jsonb(lifecycle)::pg_catalog.text
        FROM ONLY kernel_process_lifecycle AS lifecycle
        WHERE process_uuid = $1
        "#,
    )
    .bind(start.process_uuid)
    .fetch_one(&db.pool)
    .await
    .expect("capture terminal row after rejected rewrite");
    assert_eq!(
        after_terminal_row, original_terminal_row,
        "rejected terminal rewrite must leave historical STOP byte-equivalent"
    );
}

async fn run_exact_and_conflicting_start_cobatch_case(db: &OrphanTestDb, conflict_first: bool) {
    let pg_store = Arc::new(PostgresProcessLedgerStore::new(db.pool.clone()));
    let ordering = if conflict_first {
        "conflict-first"
    } else {
        "exact-first"
    };
    let exact = ProcessStart::new(
        ProcessEngineKind::Candle,
        format!("original-{ordering}"),
        None,
    )
    .with_metadata_jsonb(serde_json::json!({"ordering": ordering}));
    pg_store
        .write_batch(vec![LedgerEvent::Start(exact.clone())])
        .await
        .expect("seed original open lifecycle for co-batch collision");
    let mut conflicting = exact.clone();
    conflicting.owner_role = format!("conflicting-{ordering}");

    let observed_store = ConflictObservingPgStore::new(pg_store);
    let (batcher, join) = LedgerBatcher::spawn(
        Arc::new(observed_store.clone()),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 4,
            batch_size: 2,
            flush_interval: Duration::from_secs(60),
        },
    );
    let mut reservations = batcher
        .try_reserve_lifecycles(2)
        .expect("reserve both co-batch lifecycle pairs")
        .into_iter();
    let first = reservations.next().expect("first lifecycle reservation");
    let second = reservations.next().expect("second lifecycle reservation");
    let first_active = first
        .begin(if conflict_first {
            conflicting.clone()
        } else {
            exact.clone()
        })
        .expect("enqueue first co-batch START");
    let second_active = second
        .begin(if conflict_first {
            exact.clone()
        } else {
            conflicting.clone()
        })
        .expect("enqueue second co-batch START");
    let (conflicting_active, exact_active) = if conflict_first {
        (first_active, second_active)
    } else {
        (second_active, first_active)
    };

    observed_store.wait_for_conflict().await;
    let first_batch = observed_store.first_batch();
    assert_eq!(first_batch.len(), 2, "proof requires one real co-batch");
    let submitted_roles = first_batch
        .iter()
        .map(|event| match event {
            LedgerEvent::Start(start) => start.owner_role.as_str(),
            LedgerEvent::Stop(_) => panic!("first collision batch must contain only START rows"),
        })
        .collect::<Vec<_>>();
    let expected_roles = if conflict_first {
        vec![conflicting.owner_role.as_str(), exact.owner_role.as_str()]
    } else {
        vec![exact.owner_role.as_str(), conflicting.owner_role.as_str()]
    };
    assert_eq!(submitted_roles, expected_roles);

    assert_eq!(
        conflicting_active
            .stop(Some(-1), "rejected-conflict-must-not-stop")
            .expect("conflicting STOP suppression is observable"),
        StopRecordOutcome::LeftOpenForReconciliation
    );
    let accepted_reason = format!("accepted-exact-{ordering}");
    assert_eq!(
        exact_active
            .stop(Some(0), &accepted_reason)
            .expect("exact retry retains STOP authority"),
        StopRecordOutcome::Recorded
    );
    batcher.begin_close();
    tokio::time::timeout(Duration::from_secs(2), join)
        .await
        .expect("co-batch writer closes")
        .expect("co-batch writer task joins")
        .expect("co-batch writer drains retained exact lifecycle");

    let row: (String, Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT owner_role, exit_code, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(exact.process_uuid)
    .fetch_one(&db.pool)
    .await
    .expect("read co-batch lifecycle result");
    assert_eq!(row.0, exact.owner_role);
    assert_eq!(row.1, Some(0));
    assert_eq!(row.2.as_deref(), Some(accepted_reason.as_str()));
}

#[tokio::test]
async fn exact_and_conflicting_same_uuid_cobatch_rejects_only_conflict_in_both_orderings() {
    let db = orphan_reclaim_schema_pool().await;
    run_exact_and_conflicting_start_cobatch_case(&db, false).await;
    run_exact_and_conflicting_start_cobatch_case(&db, true).await;
}

#[tokio::test]
async fn cached_writer_rejects_same_schema_lifecycle_relation_replacement() {
    let db = orphan_reclaim_schema_pool().await;
    let store = PostgresProcessLedgerStore::new(db.pool.clone());
    let original = ProcessStart::new(ProcessEngineKind::Candle, "oid-original", None);
    store
        .write_batch(vec![LedgerEvent::Start(original.clone())])
        .await
        .expect("cache lifecycle authority OID with one durable write");

    let archive_schema = format!("mt013_archive_{}", uuid::Uuid::now_v7().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{archive_schema}""#))
        .execute(&db.pool)
        .await
        .expect("create archive schema for preserved original relation");
    sqlx::query(&format!(
        r#"ALTER TABLE "{}".kernel_process_lifecycle SET SCHEMA "{archive_schema}""#,
        db.schema
    ))
    .execute(&db.pool)
    .await
    .expect("move original lifecycle relation without changing its OID");
    sqlx::raw_sql(include_str!(
        "../migrations/0021_kernel_process_lifecycle.sql"
    ))
    .execute(&db.pool)
    .await
    .expect("create an exact-shaped replacement in the canonical schema");

    let attempted = ProcessStart::new(ProcessEngineKind::Candle, "oid-replacement", None);
    let error = store
        .write_batch(vec![LedgerEvent::Start(attempted)])
        .await
        .expect_err("cached writer must reject a same-schema relation replacement");
    assert!(
        error.to_string().contains("changed identity"),
        "replacement rejection must identify cached authority drift: {error}"
    );
    let archived_rows: i64 = sqlx::query_scalar(&format!(
        r#"SELECT pg_catalog.count(*) FROM ONLY "{archive_schema}".kernel_process_lifecycle"#
    ))
    .fetch_one(&db.pool)
    .await
    .expect("count preserved rows in original relation");
    let replacement_rows: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY kernel_process_lifecycle")
            .fetch_one(&db.pool)
            .await
            .expect("count rows in rejected replacement relation");
    assert_eq!(archived_rows, 1);
    assert_eq!(replacement_rows, 0);
}

#[tokio::test]
async fn writer_rejects_wrong_generated_expression_with_exact_types_and_primary_key() {
    let db = orphan_reclaim_schema_pool().await;
    sqlx::raw_sql("DROP TABLE kernel_process_lifecycle CASCADE")
        .execute(&db.pool)
        .await
        .expect("remove migrated lifecycle authority before expression-drift proof");
    let wrong_expression_migration =
        include_str!("../migrations/0021_kernel_process_lifecycle.sql").replacen(
            "process_id UUID GENERATED ALWAYS AS (process_uuid) STORED",
            "process_id UUID GENERATED ALWAYS AS (parent_process_id) STORED",
            1,
        );
    assert!(wrong_expression_migration
        .contains("process_id UUID GENERATED ALWAYS AS (parent_process_id) STORED"));
    sqlx::raw_sql(&wrong_expression_migration)
        .execute(&db.pool)
        .await
        .expect("create exact-column lifecycle table with one wrong generated expression");

    let attempted = ProcessStart::new(ProcessEngineKind::Candle, "wrong-expression", None);
    let error = PostgresProcessLedgerStore::new(db.pool.clone())
        .write_batch(vec![LedgerEvent::Start(attempted)])
        .await
        .expect_err("writer must reject behaviorally wrong generated-column authority");
    assert!(
        error.to_string().contains("no migration-0021-shaped"),
        "generated-expression rejection must identify authority-shape drift: {error}"
    );
    let rows: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY kernel_process_lifecycle")
            .fetch_one(&db.pool)
            .await
            .expect("count rows after generated-expression rejection");
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn pidless_reclaimer_forces_sync_commit_for_cursor_and_terminal_update() {
    let db = orphan_reclaim_schema_pool().await;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&db.schema_url)
        .await
        .expect("connect single-session reclaimer durability proof pool");
    let host_scope = "mt013-sync-commit-host";
    let process_uuid = uuid::Uuid::now_v7();
    let descriptor = released_descriptor(uuid::Uuid::now_v7(), host_scope);
    insert_pidless_embedded_row(
        &pool,
        process_uuid,
        "candle",
        Utc::now() - chrono::Duration::minutes(5),
        descriptor.metadata_fields(),
    )
    .await;
    let session_sync: String =
        sqlx::query_scalar("SELECT pg_catalog.set_config('synchronous_commit', 'off', false)")
            .fetch_one(&pool)
            .await
            .expect("poison pooled reclaimer session synchronous_commit");
    assert_eq!(session_sync, "off");
    let (fsync, full_page_writes): (String, String) = sqlx::query_as(
        "SELECT pg_catalog.current_setting('fsync'), pg_catalog.current_setting('full_page_writes')",
    )
    .fetch_one(&pool)
    .await
    .expect("read PostgreSQL crash-durability settings");
    assert_eq!(fsync, "on", "durability proof requires fsync=on");
    assert_eq!(
        full_page_writes, "on",
        "durability proof requires full_page_writes=on"
    );

    let report = reclaim_pidless_embedded_orphans(
        &pool,
        Utc::now() - chrono::Duration::minutes(1),
        host_scope,
    )
    .await
    .expect("transaction-local synchronous commit protects both durable reclaim mutations");
    assert_eq!(report.closed_rows, 1);
    let session_sync_after: String =
        sqlx::query_scalar("SELECT pg_catalog.current_setting('synchronous_commit')")
            .fetch_one(&pool)
            .await
            .expect("read pooled session default after reclaimer commits");
    assert_eq!(session_sync_after, "off");
}

async fn assert_authority_rejection_preserves_reclaim_state(
    db: &OrphanTestDb,
    process_uuid: uuid::Uuid,
    host_scope: &str,
) {
    let error = reclaim_pidless_embedded_orphans(
        &db.pool,
        Utc::now() - chrono::Duration::minutes(1),
        host_scope,
    )
    .await
    .expect_err("unsafe PostgreSQL authority must fail closed");
    assert!(
        error.to_string().contains("hook/RLS/rule-free"),
        "authority rejection must identify the unsafe behavior surface: {error}"
    );
    assert!(
        stopped_at(&db.pool, process_uuid).await.is_none(),
        "rejected authority must leave the lifecycle row open"
    );
    let cursor_rows: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_pidless_embedded_reclaim_cursor",
    )
    .fetch_one(&db.pool)
    .await
    .expect("count cursor rows after authority rejection");
    assert_eq!(cursor_rows, 0, "rejected authority must not advance cursor");
}

async fn seed_authority_rejection_row(db: &OrphanTestDb, host_scope: &str) -> uuid::Uuid {
    let process_uuid = uuid::Uuid::now_v7();
    let descriptor = released_descriptor(uuid::Uuid::now_v7(), host_scope);
    insert_pidless_embedded_row(
        &db.pool,
        process_uuid,
        "candle",
        Utc::now() - chrono::Duration::minutes(5),
        descriptor.metadata_fields(),
    )
    .await;
    process_uuid
}

#[tokio::test]
async fn pidless_reclaimer_rejects_lifecycle_user_trigger_without_mutation() {
    let db = orphan_reclaim_schema_pool().await;
    let host_scope = "mt013-lifecycle-trigger-host";
    let process_uuid = seed_authority_rejection_row(&db, host_scope).await;
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION mt013_lifecycle_trigger_fn()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            NEW.stop_reason := 'trigger-rewrite';
            RETURN NEW;
        END
        $$;
        CREATE TRIGGER mt013_lifecycle_trigger
        BEFORE UPDATE ON kernel_process_lifecycle
        FOR EACH ROW EXECUTE FUNCTION mt013_lifecycle_trigger_fn();
        "#,
    )
    .execute(&db.pool)
    .await
    .expect("install hostile lifecycle trigger");

    let writer_error = PostgresProcessLedgerStore::new(db.pool.clone())
        .write_batch(vec![LedgerEvent::Start(ProcessStart::new(
            ProcessEngineKind::Candle,
            "trigger-rejected-writer",
            None,
        ))])
        .await
        .expect_err("writer must reject lifecycle user triggers");
    assert!(writer_error.to_string().contains("hook/RLS/rule-free"));
    assert_authority_rejection_preserves_reclaim_state(&db, process_uuid, host_scope).await;
    let lifecycle_rows: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY kernel_process_lifecycle")
            .fetch_one(&db.pool)
            .await
            .expect("count lifecycle rows after trigger rejection");
    assert_eq!(lifecycle_rows, 1, "writer rejection must insert no row");
}

#[tokio::test]
async fn pidless_reclaimer_rejects_cursor_user_trigger_without_mutation() {
    let db = orphan_reclaim_schema_pool().await;
    let host_scope = "mt013-cursor-trigger-host";
    let process_uuid = seed_authority_rejection_row(&db, host_scope).await;
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION mt013_cursor_trigger_fn()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            NEW.last_instance_id := 'trigger-rewrite';
            RETURN NEW;
        END
        $$;
        CREATE TRIGGER mt013_cursor_trigger
        BEFORE INSERT OR UPDATE ON kernel_pidless_embedded_reclaim_cursor
        FOR EACH ROW EXECUTE FUNCTION mt013_cursor_trigger_fn();
        "#,
    )
    .execute(&db.pool)
    .await
    .expect("install hostile cursor trigger");

    assert_authority_rejection_preserves_reclaim_state(&db, process_uuid, host_scope).await;
}

#[tokio::test]
async fn pidless_reclaimer_rejects_lifecycle_rls_policy_without_mutation() {
    let db = orphan_reclaim_schema_pool().await;
    let host_scope = "mt013-lifecycle-rls-host";
    let process_uuid = seed_authority_rejection_row(&db, host_scope).await;
    sqlx::raw_sql(
        r#"
        ALTER TABLE kernel_process_lifecycle ENABLE ROW LEVEL SECURITY;
        CREATE POLICY mt013_lifecycle_policy ON kernel_process_lifecycle
        FOR ALL USING (true) WITH CHECK (true);
        "#,
    )
    .execute(&db.pool)
    .await
    .expect("install hostile lifecycle RLS policy");

    let writer_error = PostgresProcessLedgerStore::new(db.pool.clone())
        .write_batch(vec![LedgerEvent::Start(ProcessStart::new(
            ProcessEngineKind::Candle,
            "rls-rejected-writer",
            None,
        ))])
        .await
        .expect_err("writer must reject lifecycle RLS/policies");
    assert!(writer_error.to_string().contains("hook/RLS/rule-free"));
    assert_authority_rejection_preserves_reclaim_state(&db, process_uuid, host_scope).await;
    let lifecycle_rows: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM ONLY kernel_process_lifecycle")
            .fetch_one(&db.pool)
            .await
            .expect("count lifecycle rows after RLS rejection");
    assert_eq!(lifecycle_rows, 1, "writer rejection must insert no row");
}

#[tokio::test]
async fn pidless_reclaimer_rejects_cursor_rls_policy_without_mutation() {
    let db = orphan_reclaim_schema_pool().await;
    let host_scope = "mt013-cursor-rls-host";
    let process_uuid = seed_authority_rejection_row(&db, host_scope).await;
    sqlx::raw_sql(
        r#"
        ALTER TABLE kernel_pidless_embedded_reclaim_cursor ENABLE ROW LEVEL SECURITY;
        CREATE POLICY mt013_cursor_policy ON kernel_pidless_embedded_reclaim_cursor
        FOR ALL USING (true) WITH CHECK (true);
        "#,
    )
    .execute(&db.pool)
    .await
    .expect("install hostile cursor RLS policy");

    assert_authority_rejection_preserves_reclaim_state(&db, process_uuid, host_scope).await;
}

#[tokio::test]
async fn pidless_reclaimer_reports_authority_lock_timeout_without_advancing_cursor() {
    let db = orphan_reclaim_schema_pool().await;
    let host_scope = "mt013-authority-lock-host";
    let process_uuid = uuid::Uuid::now_v7();
    let descriptor = released_descriptor(uuid::Uuid::now_v7(), host_scope);
    insert_pidless_embedded_row(
        &db.pool,
        process_uuid,
        "candle",
        Utc::now() - chrono::Duration::minutes(5),
        descriptor.metadata_fields(),
    )
    .await;
    let mut blocker = db
        .pool
        .begin()
        .await
        .expect("begin DDL blocker transaction");
    sqlx::query("LOCK TABLE ONLY kernel_process_lifecycle IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .expect("hold lifecycle authority DDL lock");

    let blocked_report = tokio::time::timeout(
        Duration::from_secs(20),
        reclaim_pidless_embedded_orphans(
            &db.pool,
            Utc::now() - chrono::Duration::minutes(1),
            host_scope,
        ),
    )
    .await
    .expect("reclaimer lock wait remains bounded")
    .expect("bounded authority contention returns a typed report");
    assert!(blocked_report.candidate_scan_timed_out);
    assert!(!blocked_report.is_complete());
    blocker
        .rollback()
        .await
        .expect("release lifecycle authority DDL lock");

    let open: bool = sqlx::query_scalar(
        "SELECT stopped_at IS NULL FROM ONLY kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(&db.pool)
    .await
    .expect("contended lifecycle remains open");
    assert!(open);
    let cursor_rows: i64 = sqlx::query_scalar(
        "SELECT pg_catalog.count(*) FROM ONLY kernel_pidless_embedded_reclaim_cursor",
    )
    .fetch_one(&db.pool)
    .await
    .expect("contended candidate transaction did not advance cursor");
    assert_eq!(cursor_rows, 0);

    let mut retry_reports = Vec::new();
    for _ in 0..3 {
        let retry = reclaim_pidless_embedded_orphans(
            &db.pool,
            Utc::now() - chrono::Duration::minutes(1),
            host_scope,
        )
        .await
        .expect("a later bounded sweep remains recoverable after authority lock release");
        retry_reports.push(retry);
        if retry.closed_rows == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(
        retry_reports
            .iter()
            .map(|report| report.closed_rows)
            .sum::<u64>(),
        1,
        "a later bounded sweep must close exactly once: {retry_reports:?}"
    );
}

#[tokio::test]
async fn pidless_reclaimer_rejects_unlogged_cursor_and_inherited_lifecycle_rows() {
    let unlogged_db = orphan_reclaim_schema_pool().await;
    sqlx::query("ALTER TABLE kernel_pidless_embedded_reclaim_cursor SET UNLOGGED")
        .execute(&unlogged_db.pool)
        .await
        .expect("convert cursor projection to crash-truncated UNLOGGED storage");
    let unlogged_error = reclaim_pidless_embedded_orphans(
        &unlogged_db.pool,
        Utc::now() - chrono::Duration::minutes(1),
        "mt013-unlogged-cursor-host",
    )
    .await
    .expect_err("UNLOGGED cursor cannot satisfy durable reclaim authority");
    assert!(
        unlogged_error.to_string().contains("permanent logged"),
        "cursor drift error must identify lost durability: {unlogged_error}"
    );
    drop(unlogged_db);

    let inherited_db = orphan_reclaim_schema_pool().await;
    sqlx::query(
        "CREATE TABLE kernel_process_lifecycle_inherited_child () INHERITS (kernel_process_lifecycle)",
    )
    .execute(&inherited_db.pool)
    .await
    .expect("attach inherited lifecycle child");
    let inherited_uuid = uuid::Uuid::now_v7();
    let host_scope = "mt013-inherited-host";
    let descriptor = released_descriptor(uuid::Uuid::now_v7(), host_scope);
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle_inherited_child
            (process_uuid, os_pid, parent_session_id, engine_kind, started_at, owner_role, metadata_jsonb)
        VALUES ($1, NULL, NULL, 'candle', $2, 'inherited-nonauthority', $3)
        "#,
    )
    .bind(inherited_uuid)
    .bind(Utc::now() - chrono::Duration::minutes(5))
    .bind(descriptor.metadata_fields())
    .execute(&inherited_db.pool)
    .await
    .expect("insert realistic stale row into inherited non-authority");
    let inheritance_error = reclaim_pidless_embedded_orphans(
        &inherited_db.pool,
        Utc::now() - chrono::Duration::minutes(1),
        host_scope,
    )
    .await
    .expect_err("lifecycle authority participating in inheritance must fail closed");
    assert!(inheritance_error
        .to_string()
        .contains("no migration-0021-shaped"));
    let inherited_stopped_at: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT stopped_at FROM ONLY kernel_process_lifecycle_inherited_child WHERE process_uuid = $1",
    )
    .bind(inherited_uuid)
    .fetch_one(&inherited_db.pool)
    .await
    .expect("read inherited non-authority row after rejection");
    assert_eq!(inherited_stopped_at, None);
}

fn released_descriptor(
    instance_id: uuid::Uuid,
    host_scope_id: &str,
) -> EmbeddedRuntimeInstanceDescriptor {
    let lease = acquire_embedded_runtime_instance_lease(instance_id, host_scope_id)
        .expect("acquire temporary runtime lease descriptor");
    let descriptor = lease.descriptor().clone();
    drop(lease);
    descriptor
}

async fn insert_pidless_embedded_row(
    pool: &sqlx::PgPool,
    process_uuid: uuid::Uuid,
    engine_kind: &str,
    started_at: chrono::DateTime<Utc>,
    metadata: serde_json::Value,
) {
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle
            (process_uuid, os_pid, parent_session_id, engine_kind, started_at, owner_role, metadata_jsonb)
        VALUES ($1, NULL, NULL, $2, $3, 'handshake-embedded-default', $4)
        "#,
    )
    .bind(process_uuid)
    .bind(engine_kind)
    .bind(started_at)
    .bind(metadata)
    .execute(pool)
    .await
    .expect("insert pid-less embedded lifecycle row");
}

async fn insert_reclaim_control_row(
    pool: &sqlx::PgPool,
    process_uuid: uuid::Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<&str>,
    engine_kind: &str,
    started_at: chrono::DateTime<Utc>,
    stopped_at: Option<chrono::DateTime<Utc>>,
    metadata: serde_json::Value,
) {
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle
            (process_uuid, os_pid, parent_session_id, engine_kind, started_at, stopped_at, owner_role, metadata_jsonb)
        VALUES ($1, $2, $3, $4, $5, $6, 'handshake-embedded-default', $7)
        "#,
    )
    .bind(process_uuid)
    .bind(os_pid)
    .bind(parent_session_id)
    .bind(engine_kind)
    .bind(started_at)
    .bind(stopped_at)
    .bind(metadata)
    .execute(pool)
    .await
    .expect("insert embedded reclaim control row");
}

async fn stopped_at(
    pool: &sqlx::PgPool,
    process_uuid: uuid::Uuid,
) -> Option<chrono::DateTime<Utc>> {
    sqlx::query_scalar("SELECT stopped_at FROM kernel_process_lifecycle WHERE process_uuid = $1")
        .bind(process_uuid)
        .fetch_one(pool)
        .await
        .expect("read lifecycle stopped_at")
}

fn assert_complete_reclaim(report: PidlessEmbeddedReclaimReport, expected_closed_rows: u64) {
    assert!(
        report.is_complete(),
        "ordinary reclaim must not defer work: {report:?}"
    );
    assert_eq!(report.closed_rows, expected_closed_rows);
}

#[tokio::test]
async fn loopback_database_host_scope_is_stable_and_credential_independent() {
    let managed = managed_pg_handle().await;
    let proof = managed
        .proven_local_endpoint()
        .expect("real managed PostgreSQL must issue local-endpoint proof");
    let base_url = managed.database_url();
    let first_url = base_url.replacen("postgres://postgres@", "postgresql://first:secret@", 1);
    let second_url = base_url
        .replacen("postgres://postgres@", "postgresql://other:changed@", 1)
        .replace("127.0.0.1", "localhost");
    let first =
        resolve_embedded_runtime_host_scope_with_managed_local(&first_url, None, Some(proof))
            .expect("derive first proven-managed loopback host scope");
    let second =
        resolve_embedded_runtime_host_scope_with_managed_local(&second_url, None, Some(proof))
            .expect("derive alias-equivalent proven-managed loopback host scope");
    assert_eq!(first, second);
    assert!(first.starts_with(EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX));
    assert!(!first.contains("secret"));
    assert!(!first.contains("changed"));
}

#[test]
fn explicit_host_scope_overrides_identical_loopback_database_endpoint() {
    let database_url = "postgresql://user:secret@127.0.0.1:55432/handshake";
    let first = resolve_embedded_runtime_host_scope_with_override(database_url, Some("host-a"))
        .expect("resolve first explicit loopback host scope");
    let second = resolve_embedded_runtime_host_scope_with_override(database_url, Some("host-b"))
        .expect("resolve second explicit loopback host scope");

    assert_eq!(first, "host-a");
    assert_eq!(second, "host-b");
    assert_ne!(first, second);
}

#[tokio::test]
async fn unproven_mismatched_or_external_database_endpoints_fail_closed() {
    let managed = managed_pg_handle().await;
    let proof = managed
        .proven_local_endpoint()
        .expect("real managed PostgreSQL must issue local-endpoint proof");
    let database_url = managed.database_url().replace("127.0.0.1", "localhost");
    let unproven =
        resolve_embedded_runtime_host_scope_with_managed_local(&database_url, None, None)
            .expect_err("loopback scope without managed provenance must fail closed");
    assert!(unproven
        .to_string()
        .contains("unproven loopback PostgreSQL"));

    let mismatched_database_url = format!("{database_url}_different");
    let mismatched = resolve_embedded_runtime_host_scope_with_managed_local(
        &mismatched_database_url,
        None,
        Some(proof),
    )
    .expect_err("mismatched managed endpoint must fail closed");
    assert!(mismatched
        .to_string()
        .contains("does not match the proven local managed PostgreSQL endpoint"));

    let external_url = database_url.replace("localhost", "postgres.example.invalid");
    let external =
        resolve_embedded_runtime_host_scope_with_managed_local(&external_url, None, Some(proof))
            .expect_err("an external endpoint cannot borrow local managed provenance");
    assert!(external
        .to_string()
        .contains("non-loopback PostgreSQL host"));
}

#[tokio::test]
async fn pidless_reclaim_preserves_two_live_instances_and_closes_one_stale_instance() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let past = cutoff - chrono::Duration::hours(1);
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let live_a = acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), &host_scope)
        .expect("hold first live OS lease");
    let live_b = acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), &host_scope)
        .expect("hold second live OS lease");
    let stale = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    let row_a = uuid::Uuid::now_v7();
    let row_b = uuid::Uuid::now_v7();
    let stale_row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row_a,
        "candle",
        past,
        live_a.descriptor().metadata_fields(),
    )
    .await;
    insert_pidless_embedded_row(
        &db.pool,
        row_b,
        "llamacpp",
        past,
        live_b.descriptor().metadata_fields(),
    )
    .await;
    insert_pidless_embedded_row(&db.pool, stale_row, "candle", past, stale.metadata_fields()).await;

    // Preserve the original precision controls while attacking multi-instance
    // liveness: pid-ful, session-scoped, non-model, already-stopped, and
    // post-cutoff rows must never be swept by this pid-less boot reconciler.
    let pidful = uuid::Uuid::now_v7();
    let session_scoped = uuid::Uuid::now_v7();
    let non_model = uuid::Uuid::now_v7();
    let already_stopped = uuid::Uuid::now_v7();
    let this_boot = uuid::Uuid::now_v7();
    insert_reclaim_control_row(
        &db.pool,
        pidful,
        Some(4321),
        None,
        "candle",
        past,
        None,
        stale.metadata_fields(),
    )
    .await;
    insert_reclaim_control_row(
        &db.pool,
        session_scoped,
        None,
        Some("SR-live"),
        "candle",
        past,
        None,
        stale.metadata_fields(),
    )
    .await;
    insert_reclaim_control_row(
        &db.pool,
        non_model,
        None,
        None,
        "mechanical_job",
        past,
        None,
        stale.metadata_fields(),
    )
    .await;
    let prior_stop = postgres_timestamp_precision(cutoff - chrono::Duration::minutes(30));
    insert_reclaim_control_row(
        &db.pool,
        already_stopped,
        None,
        None,
        "candle",
        past,
        Some(prior_stop),
        stale.metadata_fields(),
    )
    .await;
    insert_reclaim_control_row(
        &db.pool,
        this_boot,
        None,
        None,
        "candle",
        cutoff + chrono::Duration::hours(1),
        None,
        stale.metadata_fields(),
    )
    .await;

    let report = reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope)
        .await
        .expect("instance-aware OS lease reclaim");
    assert_complete_reclaim(report, 1);
    assert!(stopped_at(&db.pool, row_a).await.is_none());
    assert!(stopped_at(&db.pool, row_b).await.is_none());
    assert!(stopped_at(&db.pool, stale_row).await.is_some());
    for row in [pidful, session_scoped, non_model, this_boot] {
        assert!(
            stopped_at(&db.pool, row).await.is_none(),
            "precision control row {row} must remain open"
        );
    }
    assert_eq!(
        stopped_at(&db.pool, already_stopped).await,
        Some(prior_stop)
    );
    let (reason, exit_code): (Option<String>, Option<i32>) = sqlx::query_as(
        "SELECT stop_reason, exit_code FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(stale_row)
    .fetch_one(&db.pool)
    .await
    .expect("read stale-row reclaim evidence");
    assert_eq!(
        reason.as_deref(),
        Some("orphan_reclaim_pidless_embedded_boot")
    );
    assert_eq!(exit_code, Some(-1));
}

#[tokio::test]
async fn database_session_loss_does_not_reclaim_live_os_leased_instance() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let past = cutoff - chrono::Duration::hours(1);
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let live = acquire_embedded_runtime_instance_lease(uuid::Uuid::now_v7(), &host_scope)
        .expect("hold live OS lease");
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        past,
        live.descriptor().metadata_fields(),
    )
    .await;

    // Drop every client session from this pool and reconnect while the owning
    // process socket remains alive. Database-session continuity is not liveness.
    db.pool.close().await;
    let reconnected = sqlx::PgPool::connect(&db.schema_url)
        .await
        .expect("reconnect after complete pool/session loss");
    let live_report = reclaim_pidless_embedded_orphans(&reconnected, cutoff, &host_scope)
        .await
        .expect("reclaim after database session loss");
    assert_complete_reclaim(live_report, 0);
    assert!(stopped_at(&reconnected, row).await.is_none());

    drop(live);
    let stale_report = reclaim_pidless_embedded_orphans(&reconnected, Utc::now(), &host_scope)
        .await
        .expect("reclaim after OS lease owner exits");
    assert_complete_reclaim(stale_report, 1);
    assert!(stopped_at(&reconnected, row).await.is_some());
}

#[tokio::test]
async fn concurrent_reconcilers_close_stale_instance_exactly_once() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let stale = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        stale.metadata_fields(),
    )
    .await;

    let pool_a = db.pool.clone();
    let pool_b = db.pool.clone();
    let host_a = host_scope.clone();
    let host_b = host_scope.clone();
    let (first, second) = tokio::join!(
        reclaim_pidless_embedded_orphans(&pool_a, cutoff, &host_a),
        reclaim_pidless_embedded_orphans(&pool_b, cutoff, &host_b),
    );
    let first = first.expect("first concurrent reclaimer");
    let second = second.expect("second concurrent reclaimer");
    assert!(first.is_complete(), "first reclaimer deferred: {first:?}");
    assert!(
        second.is_complete(),
        "second reclaimer deferred: {second:?}"
    );
    assert_eq!(first.closed_rows + second.closed_rows, 1);
    assert!(stopped_at(&db.pool, row).await.is_some());
}

#[tokio::test]
async fn pidless_reclaim_row_lock_timeout_is_bounded_reported_and_recoverable() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let stale = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        stale.metadata_fields(),
    )
    .await;

    let mut blocker = sqlx::PgConnection::connect(&db.schema_url)
        .await
        .expect("connect independent row-lock holder");
    let mut blocker_tx = blocker.begin().await.expect("begin row-lock transaction");
    sqlx::query(
        "SELECT process_uuid FROM kernel_process_lifecycle WHERE process_uuid = $1 FOR UPDATE",
    )
    .bind(row)
    .fetch_one(&mut *blocker_tx)
    .await
    .expect("hold hostile lifecycle row lock");

    let started = Instant::now();
    let report = tokio::time::timeout(
        Duration::from_secs(25),
        reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope),
    )
    .await
    .expect("reclaim must return within its bounded PostgreSQL deadlines")
    .expect("row-lock contention is a reported deferral, not a boot error");
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(1_500),
        "proof must traverse the configured 1.5-second row-lock timeout: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(25),
        "row-lock timeout exceeded the boot bound: {elapsed:?}"
    );
    assert_eq!(
        report,
        PidlessEmbeddedReclaimReport {
            closed_rows: 0,
            deferred_instances: 1,
            candidate_scan_timed_out: false,
            candidate_instance_limit_reached: false,
            legacy_host_scope_open_rows: LegacyHostScopeOpenRowProbe::NoneDetected,
        }
    );
    let terminal: (
        Option<chrono::DateTime<Utc>>,
        Option<i32>,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT stopped_at, exit_code, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(row)
    .fetch_one(&db.pool)
    .await
    .expect("read row after bounded lock deferral");
    assert_eq!(terminal, (None, None, None));

    blocker_tx
        .rollback()
        .await
        .expect("release hostile lifecycle row lock");
    let retry = reclaim_pidless_embedded_orphans(&db.pool, Utc::now(), &host_scope)
        .await
        .expect("retry deferred reclaim after lock release");
    assert_complete_reclaim(retry, 1);
    assert!(stopped_at(&db.pool, row).await.is_some());
}

#[tokio::test]
async fn pidless_reclaim_caps_each_boot_batch_and_reports_deferred_candidates() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let total = PIDLESS_RECLAIM_INSTANCE_CAP + 2;
    for _ in 0..total {
        let descriptor = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
        insert_pidless_embedded_row(
            &db.pool,
            uuid::Uuid::now_v7(),
            "candle",
            cutoff - chrono::Duration::hours(1),
            descriptor.metadata_fields(),
        )
        .await;
    }

    let first = tokio::time::timeout(
        Duration::from_secs(10),
        reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope),
    )
    .await
    .expect("bounded instance batch must return within the boot proof deadline")
    .expect("bounded instance batch reclaim");
    assert_eq!(first.closed_rows, PIDLESS_RECLAIM_INSTANCE_CAP as u64);
    assert_eq!(first.deferred_instances, 0);
    assert!(!first.candidate_scan_timed_out);
    assert!(first.candidate_instance_limit_reached);
    assert!(!first.is_complete());

    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM kernel_process_lifecycle WHERE stopped_at IS NULL",
    )
    .fetch_one(&db.pool)
    .await
    .expect("count deferred capped reclaim rows");
    assert_eq!(remaining, 2);

    let second = reclaim_pidless_embedded_orphans(&db.pool, Utc::now(), &host_scope)
        .await
        .expect("next boot sweep closes the deferred bounded batch");
    assert_complete_reclaim(second, 2);
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM kernel_process_lifecycle WHERE stopped_at IS NULL",
    )
    .fetch_one(&db.pool)
    .await
    .expect("count rows after second bounded sweep");
    assert_eq!(remaining, 0);
}

#[tokio::test]
async fn cyclic_reclaim_cursor_advances_past_full_live_prefix_to_later_stale_instance() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let mut instance_ids: Vec<uuid::Uuid> = (0..=PIDLESS_RECLAIM_INSTANCE_CAP)
        .map(|_| uuid::Uuid::now_v7())
        .collect();
    instance_ids.sort();

    let mut live_leases = Vec::with_capacity(PIDLESS_RECLAIM_INSTANCE_CAP);
    for instance_id in instance_ids
        .iter()
        .copied()
        .take(PIDLESS_RECLAIM_INSTANCE_CAP)
    {
        let lease = acquire_embedded_runtime_instance_lease(instance_id, &host_scope)
            .expect("acquire protected-prefix runtime lease");
        insert_pidless_embedded_row(
            &db.pool,
            uuid::Uuid::now_v7(),
            "candle",
            cutoff - chrono::Duration::hours(1),
            lease.descriptor().metadata_fields(),
        )
        .await;
        live_leases.push(lease);
    }
    let stale_descriptor = released_descriptor(
        *instance_ids.last().expect("later stale instance id"),
        &host_scope,
    );
    let stale_row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        stale_row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        stale_descriptor.metadata_fields(),
    )
    .await;

    let first = reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope)
        .await
        .expect("first bounded live-prefix sweep");
    assert_eq!(first.closed_rows, 0);
    assert!(first.candidate_instance_limit_reached);
    assert!(stopped_at(&db.pool, stale_row).await.is_none());

    let second = reclaim_pidless_embedded_orphans(&db.pool, Utc::now(), &host_scope)
        .await
        .expect("cyclic cursor advances beyond protected prefix");
    assert_eq!(second.closed_rows, 1, "later stale row must not starve");
    assert!(stopped_at(&db.pool, stale_row).await.is_some());
    assert_eq!(live_leases.len(), PIDLESS_RECLAIM_INSTANCE_CAP);
}

#[tokio::test]
async fn foreign_host_scope_is_never_reclaimed_through_local_loopback() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let foreign = released_descriptor(uuid::Uuid::now_v7(), "foreign-host-scope");
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        foreign.metadata_fields(),
    )
    .await;
    let report = reclaim_pidless_embedded_orphans(&db.pool, cutoff, "local-host-scope")
        .await
        .expect("foreign-host rows fail safe");
    assert_complete_reclaim(report, 0);
    assert!(stopped_at(&db.pool, row).await.is_none());
}

#[tokio::test]
async fn legacy_endpoint_only_host_scope_is_not_adopted_by_v2_reclaimer() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let legacy_scope = format!("local-pg-sha256:{}", "0".repeat(64));
    let current_scope = format!(
        "{EMBEDDED_RUNTIME_MANAGED_LOCAL_HOST_SCOPE_V2_PREFIX}{}",
        "1".repeat(64)
    );
    let legacy = released_descriptor(uuid::Uuid::now_v7(), &legacy_scope);
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        legacy.metadata_fields(),
    )
    .await;

    let report = reclaim_pidless_embedded_orphans(&db.pool, cutoff, &current_scope)
        .await
        .expect("v2 reclaimer must leave ambiguous legacy-scope rows untouched");
    assert_eq!(report.closed_rows, 0);
    assert_eq!(report.deferred_instances, 0);
    assert_eq!(
        report.legacy_host_scope_open_rows,
        LegacyHostScopeOpenRowProbe::Detected,
        "ambiguous legacy rows must be explicitly surfaced for operator inspection"
    );
    assert!(
        !report.is_complete(),
        "legacy ambiguity must prevent a silently complete reclaim report"
    );
    assert!(stopped_at(&db.pool, row).await.is_none());
}

#[tokio::test]
async fn reused_udp_port_fails_safe_until_exclusive_claim_is_available() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let stale = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    let occupied = UdpSocket::bind(SocketAddr::new(stale.loopback_address, stale.loopback_port))
        .expect("reuse released descriptor port with unrelated socket");
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        stale.metadata_fields(),
    )
    .await;
    let occupied_report = reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope)
        .await
        .expect("occupied port is protected");
    assert_complete_reclaim(occupied_report, 0);
    assert!(stopped_at(&db.pool, row).await.is_none());
    drop(occupied);
    let released_report = reclaim_pidless_embedded_orphans(&db.pool, Utc::now(), &host_scope)
        .await
        .expect("released port becomes positively stale");
    assert_complete_reclaim(released_report, 1);
}

#[tokio::test]
async fn corrupt_and_conflicting_metadata_does_not_abort_valid_reclaim() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let past = cutoff - chrono::Duration::hours(1);
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let valid = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    let valid_row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(&db.pool, valid_row, "candle", past, valid.metadata_fields()).await;

    let missing_row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(&db.pool, missing_row, "candle", past, serde_json::json!({})).await;
    let null_row = uuid::Uuid::now_v7();
    let mut null_metadata =
        released_descriptor(uuid::Uuid::now_v7(), &host_scope).metadata_fields();
    null_metadata["runtime_instance_id"] = serde_json::Value::Null;
    insert_pidless_embedded_row(&db.pool, null_row, "candle", past, null_metadata).await;
    let malformed_row = uuid::Uuid::now_v7();
    let mut malformed_metadata =
        released_descriptor(uuid::Uuid::now_v7(), &host_scope).metadata_fields();
    malformed_metadata["runtime_instance_id"] = serde_json::json!("not-a-uuid");
    insert_pidless_embedded_row(&db.pool, malformed_row, "candle", past, malformed_metadata).await;

    let terminal_metadata_row = uuid::Uuid::now_v7();
    let terminal_descriptor = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    insert_pidless_embedded_row(
        &db.pool,
        terminal_metadata_row,
        "candle",
        past,
        terminal_descriptor.metadata_fields(),
    )
    .await;
    sqlx::query(
        "UPDATE kernel_process_lifecycle SET exit_code = 77, stop_reason = 'preexisting-terminal-metadata' WHERE process_uuid = $1",
    )
    .bind(terminal_metadata_row)
    .execute(&db.pool)
    .await
    .expect("inject inconsistent terminal metadata on an open row");

    let conflict_id = uuid::Uuid::now_v7();
    let conflict_a_lease = acquire_embedded_runtime_instance_lease(conflict_id, &host_scope)
        .expect("allocate first conflicting descriptor port");
    let conflict_b_lease = acquire_embedded_runtime_instance_lease(conflict_id, &host_scope)
        .expect("allocate second conflicting descriptor port");
    let conflict_a = conflict_a_lease.descriptor().clone();
    let conflict_b = conflict_b_lease.descriptor().clone();
    drop(conflict_a_lease);
    drop(conflict_b_lease);
    let conflict_row_a = uuid::Uuid::now_v7();
    let conflict_row_b = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        conflict_row_a,
        "candle",
        past,
        conflict_a.metadata_fields(),
    )
    .await;
    insert_pidless_embedded_row(
        &db.pool,
        conflict_row_b,
        "candle",
        past,
        conflict_b.metadata_fields(),
    )
    .await;

    let report = reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope)
        .await
        .expect("corrupt rows do not abort valid reconciliation");
    assert_eq!(report.closed_rows, 1);
    assert!(
        report.deferred_instances >= 1,
        "unsafe descriptor groups must be reported as deferred: {report:?}"
    );
    assert!(!report.is_complete());
    assert!(stopped_at(&db.pool, valid_row).await.is_some());
    for row in [
        missing_row,
        null_row,
        malformed_row,
        terminal_metadata_row,
        conflict_row_a,
        conflict_row_b,
    ] {
        assert!(
            stopped_at(&db.pool, row).await.is_none(),
            "unsafe row {row} must stay open"
        );
    }
    let terminal_metadata: (Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT exit_code, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(terminal_metadata_row)
    .fetch_one(&db.pool)
    .await
    .expect("read preserved inconsistent terminal metadata");
    assert_eq!(terminal_metadata.0, Some(77));
    assert_eq!(
        terminal_metadata.1.as_deref(),
        Some("preexisting-terminal-metadata")
    );
}

#[tokio::test]
async fn terminal_metadata_only_row_makes_reclaim_report_incomplete() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let descriptor = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        descriptor.metadata_fields(),
    )
    .await;
    sqlx::query(
        "UPDATE kernel_process_lifecycle SET exit_code = 77, stop_reason = 'inconsistent-terminal-metadata' WHERE process_uuid = $1",
    )
    .bind(row)
    .execute(&db.pool)
    .await
    .expect("inject terminal metadata without stopped_at");

    let report = reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope)
        .await
        .expect("terminal metadata is reported rather than silently omitted");
    assert_eq!(report.closed_rows, 0);
    assert!(report.deferred_instances >= 1, "{report:?}");
    assert!(!report.is_complete());
    assert!(stopped_at(&db.pool, row).await.is_none());
}

#[tokio::test]
async fn missing_descriptor_only_row_makes_reclaim_report_incomplete() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        serde_json::json!({}),
    )
    .await;

    let report = reclaim_pidless_embedded_orphans(&db.pool, cutoff, &host_scope)
        .await
        .expect("missing descriptor is reported rather than silently omitted");
    assert_eq!(report.closed_rows, 0);
    assert!(report.deferred_instances >= 1, "{report:?}");
    assert!(!report.is_complete());
    assert!(stopped_at(&db.pool, row).await.is_none());
}

#[tokio::test]
async fn pg_temp_search_path_and_function_shadows_cannot_redirect_reclaim() {
    let db = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let host_scope = format!("test-host-{}", uuid::Uuid::now_v7());
    let stale = released_descriptor(uuid::Uuid::now_v7(), &host_scope);
    let authority_row = uuid::Uuid::now_v7();
    insert_pidless_embedded_row(
        &db.pool,
        authority_row,
        "candle",
        cutoff - chrono::Duration::hours(1),
        stale.metadata_fields(),
    )
    .await;

    let shadow = format!("mt013_shadow_{}", uuid::Uuid::now_v7().simple());
    let mut admin = sqlx::PgConnection::connect(&db.base_url)
        .await
        .expect("connect for shadow setup");
    sqlx::query(&format!(r#"CREATE SCHEMA "{shadow}""#))
        .execute(&mut admin)
        .await
        .expect("create shadow schema");
    sqlx::query(&format!(
        r#"CREATE TABLE "{shadow}".kernel_process_lifecycle (junk TEXT)"#
    ))
    .execute(&mut admin)
    .await
    .expect("create incomplete persistent relation shadow");
    sqlx::query(&format!(
        r#"CREATE FUNCTION "{shadow}".pg_try_advisory_xact_lock(bigint)
           RETURNS boolean LANGUAGE SQL IMMUTABLE AS 'SELECT false'"#
    ))
    .execute(&mut admin)
    .await
    .expect("create function shadow");
    drop(admin);

    let encoded_path = format!("{shadow}%2C{}%2Cpg_catalog", db.schema);
    let shadow_url = url_with_search_path(&db.base_url, &encoded_path);
    let shadow_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&shadow_url)
        .await
        .expect("connect shadow-first single-connection pool");
    {
        let mut connection = shadow_pool
            .acquire()
            .await
            .expect("acquire shadow connection");
        sqlx::query("CREATE TEMP TABLE kernel_process_lifecycle (junk TEXT)")
            .execute(&mut *connection)
            .await
            .expect("create pg_temp relation shadow");
    }

    let report = reclaim_pidless_embedded_orphans(&shadow_pool, cutoff, &host_scope)
        .await
        .expect("qualified authority reclaim under hostile search_path");
    assert_complete_reclaim(report, 1);
    assert!(stopped_at(&db.pool, authority_row).await.is_some());
    let persistent_shadow_rows: i64 = sqlx::query_scalar(&format!(
        r#"SELECT pg_catalog.count(*) FROM "{shadow}".kernel_process_lifecycle"#
    ))
    .fetch_one(&shadow_pool)
    .await
    .expect("persistent shadow remains untouched");
    let temporary_shadow_rows: i64 =
        sqlx::query_scalar("SELECT pg_catalog.count(*) FROM pg_temp.kernel_process_lifecycle")
            .fetch_one(&shadow_pool)
            .await
            .expect("temporary shadow remains untouched");
    assert_eq!(persistent_shadow_rows, 0);
    assert_eq!(temporary_shadow_rows, 0);
}
