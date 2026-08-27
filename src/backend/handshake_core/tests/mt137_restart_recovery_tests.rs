use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use handshake_core::{
    flight_recorder::{
        fr_emitter::{FrRecorder, SurrealFrRecorder, SurrealFrRecorderConfig},
        fr_event_registry::FrEventId,
        spans::{Limit, SessionAggregateQueries},
    },
    model_manual::model_manual,
    mt_executor::cancellation::ReclaimRecord,
    process_ledger::{
        restart_resume::{
            RegistryStartupProcessCleanup, RestartOrphanReclaimer, RestartReclaimStore,
            StartupProcessCleanup, SurrealRestartOrphanReclaimer,
        },
        KillError, KillOutcome, LedgerEvent, LedgerEventKind, ProcessEngineKind,
        ProcessLedgerError, ProcessLedgerStore, ProcessStart, ProcessStop, Reclaim,
        ReclaimProcessStore, ReclaimReport, ReclaimStopWriter, ReclaimTrigger, ReclaimableProcess,
        ReclaimedProcess, SandboxKill, SurrealProcessLedgerStore,
    },
    sandbox::{
        AdapterId, ProcessHandle, SandboxAdapter, SandboxAdapterRegistry, Signal,
        WindowsNativeJailAdapter, WINDOWS_NATIVE_JAIL_ADAPTER_ID,
    },
    session_checkpoint::{
        CheckpointSink, CheckpointStateKind, CheckpointWriter, CheckpointWriterConfig,
        SessionCheckpoint, SurrealCheckpointSink,
    },
    storage::surreal::{
        bootstrap_mt137_flight_recorder_test_schema, bootstrap_mt137_process_ledger_test_schema,
        SurrealStorage, SurrealStorageConfig,
    },
};
use surrealdb::types::SurrealValue;
use uuid::Uuid;

#[test]
fn forced_cancel_projection_rejects_a_report_with_a_surviving_owned_process() {
    let process_uuid = Uuid::now_v7();
    let report = ReclaimReport {
        session_id: "sess-surviving-process".to_string(),
        trigger: ReclaimTrigger::OperatorCancel,
        processes_reclaimed: vec![ReclaimedProcess {
            process_uuid,
            engine_kind: ProcessEngineKind::HelperSubprocess,
            sandbox_adapter_id: Some("test-adapter".to_string()),
            kill_result: KillOutcome::Failed {
                error: "simulated process survived".to_string(),
            },
            stop_event_kind: LedgerEventKind::Stop,
        }],
        total_duration_ms: 1,
    };

    let error = ReclaimRecord::try_from(report)
        .expect_err("forced cancellation must never count a failed kill as reclaimed");
    let message = error.to_string();
    assert!(message.contains(&process_uuid.to_string()));
    assert!(message.contains("did not terminate every owned process"));
}

#[test]
fn process_reclaim_model_manual_matches_fail_loud_durable_stop_semantics() {
    let command = model_manual()
        .command_reference
        .iter()
        .find(|command| command.id == "process_ledger_reclaim")
        .expect("process reclaim manual command must remain registered");

    assert!(command
        .expected_output
        .contains("only after every owned process cleanup succeeds"));
    assert!(command.expected_output.contains("durably persisted"));
    assert!(command
        .expected_output
        .contains("surviving process non-terminal"));
    let recovery = command.recovery_steps.join(" ");
    assert!(recovery.contains("kill error as failed reclaim"));
    assert!(recovery.contains("direct SurrealDB STOP write"));
    assert!(!recovery.contains("FOR UPDATE"));
    assert!(!recovery.contains("writer accepted"));
}

#[derive(Default)]
struct RecordingStartupCleanup {
    cleaned: Mutex<Vec<Uuid>>,
}

impl RecordingStartupCleanup {
    fn cleaned(&self) -> Vec<Uuid> {
        self.cleaned.lock().expect("cleanup spy lock").clone()
    }
}

#[async_trait]
impl StartupProcessCleanup for RecordingStartupCleanup {
    async fn cleanup(&self, process: &ReclaimableProcess) -> Result<(), String> {
        self.cleaned
            .lock()
            .expect("cleanup spy lock")
            .push(process.process_uuid);
        Ok(())
    }
}

struct FailStopOnceStore {
    inner: Arc<SurrealProcessLedgerStore>,
    fail_next_stop: AtomicBool,
}

impl FailStopOnceStore {
    fn new(inner: Arc<SurrealProcessLedgerStore>) -> Self {
        Self {
            inner,
            fail_next_stop: AtomicBool::new(true),
        }
    }
}

#[async_trait]
impl RestartReclaimStore for FailStopOnceStore {
    async fn claim_active(
        &self,
        session_run_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        RestartReclaimStore::claim_active(self.inner.as_ref(), session_run_id).await
    }

    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError> {
        RestartReclaimStore::mark_cleanup_completed(self.inner.as_ref(), process).await
    }

    async fn write_reclaim_stop(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError> {
        if self.fail_next_stop.swap(false, Ordering::SeqCst) {
            return Err(ProcessLedgerError::Store(
                "injected STOP persistence failure after cleanup marker".to_owned(),
            ));
        }
        RestartReclaimStore::write_reclaim_stop(self.inner.as_ref(), process).await
    }

    async fn abandon(&self, processes: &[ReclaimableProcess]) -> Result<(), ProcessLedgerError> {
        RestartReclaimStore::abandon(self.inner.as_ref(), processes).await
    }
}

#[async_trait]
impl ReclaimStopWriter for FailStopOnceStore {
    async fn append_reclaim_stop(&self, stop: ProcessStop) -> Result<(), ProcessLedgerError> {
        if self.fail_next_stop.swap(false, Ordering::SeqCst) {
            return Err(ProcessLedgerError::Store(
                "injected generic STOP persistence failure after cleanup marker".to_owned(),
            ));
        }
        self.inner.write_batch(vec![LedgerEvent::Stop(stop)]).await
    }
}

#[derive(Default)]
struct RecordingSandboxKill {
    killed: Mutex<Vec<Uuid>>,
}

impl RecordingSandboxKill {
    fn killed(&self) -> Vec<Uuid> {
        self.killed.lock().expect("generic kill spy lock").clone()
    }
}

impl SandboxKill for RecordingSandboxKill {
    fn kill(&self, process_uuid: Uuid) -> Result<(), KillError> {
        self.killed
            .lock()
            .expect("generic kill spy lock")
            .push(process_uuid);
        Ok(())
    }
}

struct FailMarkerOnceStore {
    inner: Arc<SurrealProcessLedgerStore>,
    fail_next_marker: AtomicBool,
}

impl FailMarkerOnceStore {
    fn new(inner: Arc<SurrealProcessLedgerStore>) -> Self {
        Self {
            inner,
            fail_next_marker: AtomicBool::new(true),
        }
    }
}

#[async_trait]
impl ReclaimProcessStore for FailMarkerOnceStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        ReclaimProcessStore::active_processes_for_session(self.inner.as_ref(), session_id).await
    }

    async fn mark_cleanup_completed(
        &self,
        process: &ReclaimableProcess,
    ) -> Result<(), ProcessLedgerError> {
        if self.fail_next_marker.swap(false, Ordering::SeqCst) {
            return Err(ProcessLedgerError::Store(
                "injected cleanup-marker persistence failure".to_owned(),
            ));
        }
        ReclaimProcessStore::mark_cleanup_completed(self.inner.as_ref(), process).await
    }

    async fn abandon(&self, processes: &[ReclaimableProcess]) -> Result<(), ProcessLedgerError> {
        ReclaimProcessStore::abandon(self.inner.as_ref(), processes).await
    }
}

fn cleanup_process(
    engine_kind: ProcessEngineKind,
    sandbox_adapter_id: Option<&str>,
    sandbox_internal_id: Option<&str>,
) -> ReclaimableProcess {
    ReclaimableProcess {
        process_uuid: Uuid::now_v7(),
        os_pid: Some(77_777),
        parent_session_id: "SR-mt137-cleanup-contract".to_owned(),
        parent_process_id: None,
        sandbox_adapter_id: sandbox_adapter_id.map(str::to_owned),
        sandbox_internal_id: sandbox_internal_id.map(str::to_owned),
        engine_kind,
        started_at: Utc::now(),
        model_artifact_sha256: None,
        work_profile_id: None,
        owner_role: "mt137-restart-proof".to_owned(),
        owner_wp: Some("WP-KERNEL-012".to_owned()),
        role_id: None,
        wp_id: Some("WP-KERNEL-012".to_owned()),
        mt_id: Some("MT-137".to_owned()),
        sandbox_capabilities_snapshot: serde_json::json!({}),
        metadata_jsonb: serde_json::json!({}),
        reclaim_claimed_at: Utc::now(),
        reclaim_expected_reason: "reclaim_claimed:mt137-fixture".to_owned(),
        reclaim_expected_killed_reason: "reclaim_killed:mt137-fixture".to_owned(),
        reclaim_cleanup_completed: false,
    }
}

async fn open(path: &std::path::Path) -> SurrealStorage {
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(path).expect("valid MT-137 restart test path"),
    )
    .await
    .expect("open embedded MT-137 restart store");
    bootstrap_mt137_process_ledger_test_schema(&storage)
        .await
        .expect("bootstrap focused MT-137 process-ledger schema");
    storage
}

async fn open_flight_recorder(path: &std::path::Path) -> SurrealStorage {
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(path).expect("valid MT-137 Flight Recorder test path"),
    )
    .await
    .expect("open embedded MT-137 Flight Recorder store");
    bootstrap_mt137_flight_recorder_test_schema(&storage)
        .await
        .expect("bootstrap focused MT-137 Flight Recorder schema");
    storage
}

async fn seed_active_process(storage: &SurrealStorage, process_uuid: Uuid, session_run_id: &str) {
    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "mt137-restart-proof",
        Some("WP-KERNEL-012".to_owned()),
    )
    .with_process_uuid(process_uuid)
    .with_os_pid(13_704)
    .with_parent_session_id(session_run_id)
    .with_mt_id("MT-137");
    SurrealProcessLedgerStore::new(storage.clone())
        .write_batch(vec![LedgerEvent::Start(start)])
        .await
        .expect("seed active MT-137 process");
}

#[derive(Debug, SurrealValue)]
struct ProcessEffect {
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    parent_process_id: Option<Uuid>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    started_at: chrono::DateTime<Utc>,
    stopped_at: Option<chrono::DateTime<Utc>>,
    exit_code: Option<i64>,
    stop_reason: Option<String>,
    owner_role: String,
    metadata: serde_json::Value,
}

#[derive(Debug, SurrealValue)]
struct CheckpointEffect {
    checkpoint_id: Uuid,
    session_id: Uuid,
    model_session_id: Uuid,
    last_event_ledger_seq: i64,
    compact_state: serde_json::Value,
    state_kind: String,
}

async fn read_process_effect(storage: &SurrealStorage, process_uuid: Uuid) -> ProcessEffect {
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one("kernel_process_lifecycle", &process_uuid.to_string())
                    .await
            })
        })
        .await
        .expect("read MT-137 process under a storage lifecycle lease")
        .expect("exact MT-137 process must exist")
}

#[tokio::test]
async fn fresh_adapter_uses_restart_contract_not_live_handle_registry() {
    let adapter_id = AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID);
    let adapter = Arc::new(WindowsNativeJailAdapter::unavailable(
        "fresh restart-proof adapter",
    ));
    let handle = ProcessHandle {
        id: Uuid::now_v7(),
        adapter_id: adapter_id.clone(),
        pid: Some(44_444),
        sandbox_internal_id: "handshake.mt046.restartproof".to_owned(),
        spawned_at_utc: Utc::now(),
    };
    assert!(adapter.kill(&handle, Signal::Kill).await.is_err());

    let mut registry = SandboxAdapterRegistry::new(adapter_id);
    registry.register(adapter as Arc<dyn SandboxAdapter>);
    let cleanup = RegistryStartupProcessCleanup::new(Arc::new(registry));
    let mut persisted = cleanup_process(
        ProcessEngineKind::SandboxContainer,
        Some(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        Some("handshake.mt046.restartproof"),
    );
    persisted.process_uuid = handle.id;
    cleanup
        .cleanup(&persisted)
        .await
        .expect("restart cleanup must not consult an empty live-handle map");

    cleanup
        .cleanup(&cleanup_process(ProcessEngineKind::Candle, None, None))
        .await
        .expect("in-process model state ended with the prior backend boot");
    assert!(
        cleanup
            .cleanup(&cleanup_process(
                ProcessEngineKind::HelperSubprocess,
                Some(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
                None,
            ))
            .await
            .is_err(),
        "partial durable identity must fail closed"
    );
}

#[tokio::test]
async fn cleanup_marker_survives_stop_failure_and_prevents_second_cleanup_after_reopen() {
    let directory = tempfile::tempdir().expect("temporary MT-137 restart root");
    let path = directory.path().join("store");
    let session_run_id = "SR-mt137-post-cleanup-stop-failure";
    let process_uuid = Uuid::now_v7();

    let storage = open(&path).await;
    seed_active_process(&storage, process_uuid, session_run_id).await;

    let merge_store = SurrealProcessLedgerStore::new(storage.clone());
    let original = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "mt137-original-owner",
        Some("WP-KERNEL-012".to_owned()),
    )
    .with_parent_session_id("SR-mt137-merge-contract")
    .with_os_pid(13_701)
    .with_sandbox_adapter_id(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    .with_metadata_jsonb(serde_json::json!({"phase":"original"}));
    let merge_process_uuid = original.process_uuid;
    merge_store
        .write_batch(vec![LedgerEvent::Start(original.clone())])
        .await
        .expect("persist initial MT-137 merge-contract START");

    let mut enriched = original.clone();
    enriched.os_pid = None;
    enriched.parent_session_id = None;
    enriched.sandbox_adapter_id = None;
    enriched.parent_process_id = Some(Uuid::now_v7());
    enriched.sandbox_internal_id = Some("handshake.mt137.enriched".to_owned());
    enriched.started_at = original.started_at - chrono::Duration::seconds(5);
    enriched.owner_role = "mt137-enriched-owner".to_owned();
    enriched.metadata_jsonb = serde_json::json!({"phase":"enriched"});
    merge_store
        .write_batch(vec![LedgerEvent::Start(enriched.clone())])
        .await
        .expect("merge enriched MT-137 START");
    let after_start = read_process_effect(&storage, merge_process_uuid).await;
    assert_eq!(after_start.os_pid, Some(13_701));
    assert_eq!(
        after_start.parent_session_id.as_deref(),
        Some("SR-mt137-merge-contract")
    );
    assert_eq!(
        after_start.sandbox_adapter_id.as_deref(),
        Some(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    );
    assert_eq!(after_start.parent_process_id, enriched.parent_process_id);
    assert_eq!(
        after_start.sandbox_internal_id.as_deref(),
        Some("handshake.mt137.enriched")
    );
    assert_eq!(after_start.started_at, enriched.started_at);
    assert_eq!(after_start.owner_role, "mt137-enriched-owner");
    assert_eq!(
        after_start.metadata,
        serde_json::json!({"phase":"enriched"})
    );
    assert!(after_start.stopped_at.is_none());

    let mut stop = ProcessStop::from_start(&enriched, Some(37)).with_stop_reason("merged-stop");
    stop.owner_role = "mt137-stop-owner".to_owned();
    stop.metadata_jsonb = serde_json::json!({"phase":"stop"});
    merge_store
        .write_batch(vec![LedgerEvent::Stop(stop)])
        .await
        .expect("apply MT-137 STOP conflict merge");
    let after_stop = read_process_effect(&storage, merge_process_uuid).await;
    assert_eq!(after_stop.os_pid, Some(13_701));
    assert_eq!(
        after_stop.parent_session_id.as_deref(),
        Some("SR-mt137-merge-contract")
    );
    assert_eq!(
        after_stop.sandbox_adapter_id.as_deref(),
        Some(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    );
    assert_eq!(after_stop.started_at, enriched.started_at);
    assert_eq!(after_stop.exit_code, Some(37));
    assert_eq!(after_stop.stop_reason.as_deref(), Some("merged-stop"));
    assert_eq!(after_stop.owner_role, "mt137-stop-owner");
    assert_eq!(after_stop.metadata, serde_json::json!({"phase":"stop"}));
    drop(merge_store);

    let cleanup = Arc::new(RecordingStartupCleanup::default());
    let real_store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
    let failing_store = Arc::new(FailStopOnceStore::new(Arc::clone(&real_store)));
    let first_reclaimer = SurrealRestartOrphanReclaimer::new(
        Arc::clone(&failing_store),
        Arc::clone(&cleanup) as Arc<dyn StartupProcessCleanup>,
    );
    let error = first_reclaimer
        .reclaim_session(session_run_id)
        .await
        .expect_err("injected STOP failure must fail the first recovery pass");
    assert!(error
        .to_string()
        .contains("injected STOP persistence failure after cleanup marker"));
    assert_eq!(cleanup.cleaned(), vec![process_uuid]);

    let marker = read_process_effect(&storage, process_uuid).await;
    assert!(marker.stopped_at.is_some());
    assert_eq!(marker.exit_code, None);
    assert!(marker
        .stop_reason
        .as_deref()
        .is_some_and(|reason| reason.starts_with("reclaim_killed:")));

    drop(first_reclaimer);
    drop(failing_store);
    drop(real_store);
    storage.shutdown().await.expect("close failed STOP store");
    drop(storage);

    let reopened = open(&path).await;
    let second_reclaimer = SurrealRestartOrphanReclaimer::new(
        Arc::new(SurrealProcessLedgerStore::new(reopened.clone())),
        Arc::clone(&cleanup) as Arc<dyn StartupProcessCleanup>,
    );
    assert_eq!(
        second_reclaimer
            .reclaim_session(session_run_id)
            .await
            .expect("reopened recovery finalizes the cleanup marker"),
        1
    );
    assert_eq!(cleanup.cleaned(), vec![process_uuid]);

    let terminal = read_process_effect(&reopened, process_uuid).await;
    assert_eq!(terminal.exit_code, Some(-1));
    assert_eq!(terminal.stop_reason.as_deref(), Some("reclaim"));
    assert!(terminal.stopped_at.is_some());
    reopened.shutdown().await.expect("close reopened store");
}

#[tokio::test]
async fn generic_reclaim_uses_durable_marker_and_never_treats_stop_failure_as_success() {
    let directory = tempfile::tempdir().expect("temporary MT-137 generic reclaim root");
    let path = directory.path().join("store");
    let session_run_id = "SR-mt137-generic-stop-failure";
    let process_uuid = Uuid::now_v7();

    let storage = open(&path).await;
    seed_active_process(&storage, process_uuid, session_run_id).await;
    let real_store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
    let failing_writer = Arc::new(FailStopOnceStore::new(Arc::clone(&real_store)));
    let first_kill = Arc::new(RecordingSandboxKill::default());
    let first = Reclaim::new(
        Arc::clone(&real_store),
        Arc::clone(&first_kill),
        Arc::clone(&failing_writer),
    );

    let error = first
        .run(session_run_id, ReclaimTrigger::Failure)
        .await
        .expect_err("durable STOP failure must not produce a successful reclaim report");
    assert!(error
        .to_string()
        .contains("injected generic STOP persistence failure after cleanup marker"));
    assert_eq!(first_kill.killed(), vec![process_uuid]);

    let marker = read_process_effect(&storage, process_uuid).await;
    assert_eq!(marker.exit_code, None);
    assert!(marker
        .stop_reason
        .as_deref()
        .is_some_and(|reason| reason.starts_with("reclaim_killed:")));

    drop(first);
    drop(failing_writer);
    drop(real_store);
    storage
        .shutdown()
        .await
        .expect("close generic failed STOP store");
    drop(storage);

    let reopened = open(&path).await;
    let reopened_store = Arc::new(SurrealProcessLedgerStore::new(reopened.clone()));
    let second_kill = Arc::new(RecordingSandboxKill::default());
    let second = Reclaim::new(
        Arc::clone(&reopened_store),
        Arc::clone(&second_kill),
        Arc::clone(&reopened_store),
    );
    let report = second
        .run(session_run_id, ReclaimTrigger::Failure)
        .await
        .expect("reopened generic reclaim finalizes the durable cleanup marker");
    assert_eq!(report.processes_reclaimed.len(), 1);
    assert!(second_kill.killed().is_empty());

    let terminal = read_process_effect(&reopened, process_uuid).await;
    assert_eq!(terminal.exit_code, Some(-1));
    assert_eq!(terminal.stop_reason.as_deref(), Some("reclaim"));
    reopened
        .shutdown()
        .await
        .expect("close generic reopened store");
}

#[tokio::test]
async fn generic_reclaim_releases_claim_when_cleanup_marker_persistence_fails() {
    let directory = tempfile::tempdir().expect("temporary MT-137 marker failure root");
    let path = directory.path().join("store");
    let session_run_id = "SR-mt137-generic-marker-failure";
    let process_uuid = Uuid::now_v7();

    let storage = open(&path).await;
    seed_active_process(&storage, process_uuid, session_run_id).await;
    let real_store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
    let failing_store = Arc::new(FailMarkerOnceStore::new(Arc::clone(&real_store)));
    let first_kill = Arc::new(RecordingSandboxKill::default());
    let first = Reclaim::new(
        Arc::clone(&failing_store),
        Arc::clone(&first_kill),
        Arc::clone(&real_store),
    );
    let error = first
        .run(session_run_id, ReclaimTrigger::Failure)
        .await
        .expect_err("marker failure must fail and compensate the generic reclaim");
    assert!(error
        .to_string()
        .contains("injected cleanup-marker persistence failure"));
    assert_eq!(first_kill.killed(), vec![process_uuid]);

    let released = read_process_effect(&storage, process_uuid).await;
    assert_eq!(released.exit_code, None);
    assert_eq!(released.stopped_at, None);
    assert_eq!(released.stop_reason, None);

    let second_kill = Arc::new(RecordingSandboxKill::default());
    let second = Reclaim::new(
        Arc::clone(&real_store),
        Arc::clone(&second_kill),
        Arc::clone(&real_store),
    );
    let report = second
        .run(session_run_id, ReclaimTrigger::Failure)
        .await
        .expect("same-boot retry must reclaim after exact claim release");
    assert_eq!(report.processes_reclaimed.len(), 1);
    assert_eq!(second_kill.killed(), vec![process_uuid]);

    let terminal = read_process_effect(&storage, process_uuid).await;
    assert_eq!(terminal.exit_code, Some(-1));
    assert_eq!(terminal.stop_reason.as_deref(), Some("reclaim"));
    storage
        .shutdown()
        .await
        .expect("close marker failure store");
}

#[tokio::test]
async fn flight_recorder_append_is_consumer_visible_after_reopen() {
    let directory = tempfile::tempdir().expect("temporary MT-137 Flight Recorder root");
    let path = directory.path().join("store");
    let session_id = Uuid::now_v7();
    let from_utc = Utc::now() - chrono::Duration::seconds(1);
    let storage = open_flight_recorder(&path).await;
    let mut recorder = SurrealFrRecorder::spawn(
        storage.clone(),
        SurrealFrRecorderConfig {
            channel_capacity: 4,
            batch_size: 1,
            flush_interval: Duration::from_secs(5),
            kernel_task_run_id: "mt137-flight-recorder-proof".to_owned(),
            session_run_id: session_id.to_string(),
        },
    );
    recorder
        .record(
            FrEventId::SpanStarted,
            serde_json::json!({"proof":"mt137-start"}),
            None,
        )
        .await
        .expect("queue Flight Recorder start event");
    tokio::time::sleep(Duration::from_millis(2)).await;
    recorder
        .record(
            FrEventId::SpanEnded,
            serde_json::json!({"proof":"mt137-end"}),
            None,
        )
        .await
        .expect("queue Flight Recorder end event");
    recorder
        .shutdown()
        .await
        .expect("flush Flight Recorder events");
    storage
        .shutdown()
        .await
        .expect("close Flight Recorder store");
    drop(recorder);
    drop(storage);

    let reopened = open_flight_recorder(&path).await;
    let timeline = SessionAggregateQueries::new(reopened.clone())
        .session_timeline(
            session_id,
            from_utc,
            Utc::now() + chrono::Duration::seconds(1),
            Limit::new(10),
        )
        .await
        .expect("query Flight Recorder through the reopened consumer surface");
    let summaries = timeline
        .entries
        .iter()
        .filter(|entry| entry.kind == "event")
        .map(|entry| entry.summary.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        summaries,
        vec![
            FrEventId::SpanStarted.as_str(),
            FrEventId::SpanEnded.as_str()
        ]
    );
    reopened
        .shutdown()
        .await
        .expect("close reopened Flight Recorder store");
}

#[tokio::test]
async fn session_checkpoint_writer_state_survives_store_reopen() {
    let directory = tempfile::tempdir().expect("temporary MT-137 checkpoint root");
    let path = directory.path().join("store");
    let storage = open_flight_recorder(&path).await;
    let sink = Arc::new(SurrealCheckpointSink::new(storage.clone()));
    let writer = CheckpointWriter::new(
        CheckpointWriterConfig {
            period: Duration::from_secs(60),
            channel_capacity: 4,
            batch_size: 4,
            shutdown_grace: Duration::from_secs(5),
        },
        sink as Arc<dyn CheckpointSink>,
    );
    let handle = writer.start();
    let session_id = Uuid::now_v7();
    let model_session_id = Uuid::now_v7();
    let checkpoint = SessionCheckpoint::new(
        session_id,
        model_session_id,
        137,
        serde_json::json!({"phase":"restart-proof"}),
        CheckpointStateKind::PreShutdown,
    )
    .expect("build MT-137 checkpoint");
    let checkpoint_id = checkpoint.checkpoint_id.as_uuid();
    handle.submit(checkpoint).expect("queue MT-137 checkpoint");
    handle
        .shutdown()
        .await
        .expect("flush MT-137 checkpoint writer");
    storage
        .shutdown()
        .await
        .expect("close MT-137 checkpoint store");
    drop(storage);

    let reopened = open_flight_recorder(&path).await;
    let record_id = checkpoint_id.to_string();
    let row: CheckpointEffect = reopened
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .select_one("kernel_session_checkpoint", &record_id)
                    .await
            })
        })
        .await
        .expect("query reopened MT-137 checkpoint")
        .expect("read reopened MT-137 checkpoint");
    assert_eq!(row.checkpoint_id, checkpoint_id);
    assert_eq!(row.session_id, session_id);
    assert_eq!(row.model_session_id, model_session_id);
    assert_eq!(row.last_event_ledger_seq, 137);
    assert_eq!(
        row.compact_state,
        serde_json::json!({"phase":"restart-proof"})
    );
    assert_eq!(row.state_kind, "pre_shutdown");
    reopened
        .shutdown()
        .await
        .expect("close reopened MT-137 checkpoint store");
}
