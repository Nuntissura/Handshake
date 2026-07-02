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
        drain_and_join_ledger_writer, reclaim_pidless_embedded_orphans, LedgerBatcher,
        LedgerBatcherConfig, LedgerDrainJoinOutcome, LedgerEvent, NoopOverflowSink,
        ProcessEngineKind, ProcessLedgerError, ProcessLedgerStore, ProcessStart, ProcessStop,
    },
};
use sqlx::Connection;
use std::time::Duration;
use tokio::sync::OnceCell;

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
    client.shutdown();

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
    // already terminated must be a no-op (begin_close is idempotent; the Drop
    // STOP safety net is a no-op after the explicit shutdown).
    drop(client);
}

// ===========================================================================
// WP-1 MT-013 (F1b): hard-crash orphan reconcile. A kill -9 / power-loss leaves
// the pid-less, session-less embedded START row open forever; both session-
// scoped reclaim paths filter on parent_session_id, so they can NEVER match it.
// The boot-time sweep closes exactly those orphans. This proof uses REAL
// PostgreSQL (Handshake-managed cluster) and FAILS (never skips) if it cannot
// connect.
// ===========================================================================

static MANAGED_PG: OnceCell<
    handshake_core::managed_postgres::ManagedPostgres,
> = OnceCell::const_new();

async fn managed_pg_base_url() -> String {
    let managed = MANAGED_PG
        .get_or_init(|| async {
            handshake_core::managed_postgres::ManagedPostgres::ensure_running(
                handshake_core::managed_postgres::ManagedPostgresConfig::from_env(),
            )
            .await
            .expect(
                "Handshake-managed PostgreSQL must start for the embedded orphan-reconcile proof",
            )
        })
        .await;
    managed.database_url()
}

async fn orphan_reclaim_schema_pool() -> sqlx::PgPool {
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
    let sep = if base_url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{base_url}{sep}options=-csearch_path%3D{schema}");
    let pool = sqlx::PgPool::connect(&schema_url)
        .await
        .expect("connect isolated schema");
    sqlx::raw_sql(include_str!(
        "../migrations/0021_kernel_process_lifecycle.sql"
    ))
    .execute(&pool)
    .await
    .expect("apply kernel_process_lifecycle migration");
    pool
}

async fn insert_lifecycle_row(
    pool: &sqlx::PgPool,
    process_uuid: uuid::Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<&str>,
    engine_kind: &str,
    started_at: chrono::DateTime<Utc>,
    stopped_at: Option<chrono::DateTime<Utc>>,
) {
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle
            (process_uuid, os_pid, parent_session_id, engine_kind, started_at, stopped_at, owner_role)
        VALUES ($1, $2, $3, $4, $5, $6, 'handshake-embedded-default')
        "#,
    )
    .bind(process_uuid)
    .bind(os_pid)
    .bind(parent_session_id)
    .bind(engine_kind)
    .bind(started_at)
    .bind(stopped_at)
    .execute(pool)
    .await
    .expect("insert kernel_process_lifecycle row");
}

#[tokio::test]
async fn hard_crash_pidless_embedded_orphan_is_reconciled_on_boot() {
    let pool = orphan_reclaim_schema_pool().await;
    let cutoff = Utc::now();
    let past = cutoff - chrono::Duration::hours(1);

    // The kill -9 orphan: session-less, pid-less, embedded (candle), open,
    // started before this boot. This is the ONLY row the sweep must close.
    let orphan = uuid::Uuid::now_v7();
    insert_lifecycle_row(&pool, orphan, None, None, "candle", past, None).await;

    // Controls that MUST survive the sweep:
    // (a) pid-ful — has an OS-kill target; not our pid-less lane.
    let pidful = uuid::Uuid::now_v7();
    insert_lifecycle_row(&pool, pidful, Some(4321), None, "candle", past, None).await;
    // (b) session-scoped — reclaimable by the existing session sweep.
    let session_scoped = uuid::Uuid::now_v7();
    insert_lifecycle_row(&pool, session_scoped, None, Some("SR-abc"), "candle", past, None).await;
    // (c) non-embedded engine — not a regular-model runtime engine.
    let non_embedded = uuid::Uuid::now_v7();
    insert_lifecycle_row(&pool, non_embedded, None, None, "mechanical_job", past, None).await;
    // (d) already stopped — excluded by `stopped_at IS NULL`.
    let already_stopped = uuid::Uuid::now_v7();
    insert_lifecycle_row(
        &pool,
        already_stopped,
        None,
        None,
        "candle",
        past,
        Some(cutoff - chrono::Duration::minutes(30)),
    )
    .await;
    // (e) started AFTER the boot cutoff — this boot's own live row.
    let this_boot = uuid::Uuid::now_v7();
    insert_lifecycle_row(
        &pool,
        this_boot,
        None,
        None,
        "candle",
        cutoff + chrono::Duration::hours(1),
        None,
    )
    .await;

    let closed = reclaim_pidless_embedded_orphans(&pool, cutoff)
        .await
        .expect("pid-less embedded orphan reclaim sweep");
    assert_eq!(
        closed, 1,
        "exactly the one session-less/pid-less/embedded/open/pre-cutoff orphan must be closed"
    );

    // The orphan is now closed with the boot-reclaim stop_reason + sentinel exit.
    let (orphan_stopped, orphan_reason, orphan_exit): (
        Option<chrono::DateTime<Utc>>,
        Option<String>,
        Option<i32>,
    ) = sqlx::query_as(
        "SELECT stopped_at, stop_reason, exit_code FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(orphan)
    .fetch_one(&pool)
    .await
    .expect("read reconciled orphan row");
    assert!(
        orphan_stopped.is_some(),
        "the orphan row must be closed (stopped_at set) by the boot reclaim sweep"
    );
    assert_eq!(
        orphan_reason.as_deref(),
        Some("orphan_reclaim_pidless_embedded_boot"),
        "the orphan row must carry the boot-reclaim stop_reason"
    );
    assert_eq!(
        orphan_exit,
        Some(-1),
        "the orphan row must carry the sentinel reclaim exit_code"
    );

    // Every control row remains open — the sweep is precise.
    for (label, id) in [
        ("pidful", pidful),
        ("session_scoped", session_scoped),
        ("non_embedded", non_embedded),
        ("this_boot", this_boot),
    ] {
        let stopped: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
            "SELECT stopped_at FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("read control row");
        assert!(
            stopped.is_none(),
            "control row {label} must NOT be closed by the pid-less embedded orphan reclaim sweep"
        );
    }
}
