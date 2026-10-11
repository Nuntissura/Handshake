use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use async_trait::async_trait;
use handshake_core::{
    flight_recorder::fr_event_registry::FrEventId,
    kernel::{KernelActor, KernelEventType, NewKernelEvent},
    process_ledger::restart_resume::{
        RestartOrphanReclaimer, RestartReclaimStore, RestartResumeDbBackoffPolicy,
        RestartResumeRuntimeError, StartupProcessCleanup, SurrealRestartResumeRunner,
    },
    session_checkpoint::ResumeReport,
    storage::surreal::{
        bootstrap_schema, RowFilter, SurrealStorage, SurrealStorageConfig, SurrealStorageError,
    },
    storage::Database,
};
use surrealdb::types::SurrealValue;

const MT141_RUNTIME_CHAOS_EXECUTABLE_MAPPINGS: &[(&str, &str, &str)] = &[
    (
        "mt195_runtime_hard_kill_child_leaves_checkpoint_process_evidence_and_persists_report",
        "real child hard-kill leaves one checkpoint and one unterminated process row, then embedded startup recovery persists one successful report and post-failure checkpoint",
        "mt195_runtime_hard_kill_child_leaves_reopened_checkpoint_and_active_process_evidence proves hard-kill, exact checkpoint/process readback, successful replay, durable report, retry queue, and the generated post-failure checkpoint through the read-only inspector",
    ),
    (
        "mt195_runtime_real_handshake_core_binary_runs_startup_recovery_and_persists_report",
        "real product binary startup-only recovery persists report, retry queue row, post-failure checkpoint, and final counter",
        "mt195_runtime_real_handshake_core_binary_runs_startup_recovery_and_persists_report",
    ),
    (
        "mt195_runtime_event_gap_persists_failed_report_and_decision_evidence",
        "event gap persists failed report, operator-decision row, failed checkpoint, and recovery-failed event",
        "mt195_runtime_event_gap_persists_failed_report_and_decision_evidence",
    ),
    (
        "mt195_runtime_operator_cancel_persists_failed_report_and_decision_evidence",
        "operator cancel persists failed report, decision row, failed checkpoint, and recovery-failed event",
        "mt195_runtime_operator_cancel_persists_failed_report_and_decision_evidence",
    ),
    (
        "mt195_runtime_transient_db_unavailable_backs_off_and_resumes_after_db_return_without_shared_db_damage",
        "transient owner-local store unavailability performs bounded backoff, resumes after return, and persists the recovery report",
        "mt195_runtime_transient_embedded_unavailable_backs_off_and_resumes_after_return",
    ),
];

const MT141_RUNTIME_CHAOS_NON_TEST_DISPOSITIONS: &[(&str, &str, &str)] = &[
    (
        "external_shared_backend_termination",
        "retired mt195_runtime_backend_termination_records_db_loss_without_shared_db_damage",
        "NO_EMBEDDED_EQUIVALENT: Handshake now owns an in-process SurrealDB engine with no external database service or foreign backend process to terminate; shutting down that owner-local handle is covered as transient local unavailability and cannot represent external-service termination",
    ),
    (
        "external_shared_backend_non_damage",
        "retired shared-backend non-damage assertions from both backend-termination runtime tests",
        "NO_EMBEDDED_EQUIVALENT: each embedded runtime proof owns one isolated data directory and the engine lock excludes a second writer, so there is no shared multi-tenant backend state whose non-damage can be observed; close-reopen durability and per-directory isolation are the applicable embedded proofs",
    ),
];

// This fixture adapter observes only the child this test killed and awaited. It never kills a PID.
struct OwnedReapedChildCleanup {
    storage: SurrealStorage,
    original_scope: handshake_core::process_ledger::ReclaimResourceScope,
    ready: Option<crate::runtime_child::ChildReadyEvidence>,
    reaped_failed: bool,
}

impl OwnedReapedChildCleanup {
    fn from_killed_child(
        storage: SurrealStorage,
        original_scope: handshake_core::process_ledger::ReclaimResourceScope,
        killed: &crate::runtime_child::KilledChildEvidence,
    ) -> Self {
        assert_eq!(
            original_scope,
            handshake_core::process_ledger::ReclaimResourceScope::from_exact(
                &killed.ready.original_scope
            )
            .expect("killed child attribution is an exact cleanup scope")
        );
        Self {
            storage,
            original_scope,
            ready: Some(killed.ready.clone()),
            reaped_failed: !killed.exit_status.success(),
        }
    }

    fn no_owned_child(
        storage: SurrealStorage,
        original_scope: handshake_core::process_ledger::ReclaimResourceScope,
    ) -> Self {
        Self {
            storage,
            original_scope,
            ready: None,
            reaped_failed: false,
        }
    }

    fn matches_reaped_identity(
        &self,
        process: &handshake_core::process_ledger::ReclaimableProcess,
    ) -> bool {
        self.reaped_failed
            && self.ready.as_ref().is_some_and(|ready| {
                process.process_uuid == ready.process_uuid
                    && process.os_pid == Some(ready.os_pid)
                    && ready.os_pid != 0
                    && process.parent_session_id == format!("SR-{}", ready.session_id)
                    && process.engine_kind
                        == handshake_core::process_ledger::ProcessEngineKind::HelperSubprocess
            })
    }
}

#[derive(SurrealValue)]
struct OwnedChildOriginRow {
    process_uuid: uuid::Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    owner_account_id: Option<String>,
    actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>,
    access_space_id: Option<String>,
    workspace_id: Option<String>,
}

#[async_trait]
impl handshake_core::process_ledger::restart_resume::StartupProcessCleanup
    for OwnedReapedChildCleanup
{
    async fn cleanup(
        &self,
        process: &handshake_core::process_ledger::ReclaimableProcess,
    ) -> Result<(), String> {
        if !self.matches_reaped_identity(process) {
            return Err("child cleanup lacks exact owned kill-and-wait evidence".into());
        }
        let record_id = process.process_uuid.to_string();
        let row = self
            .storage
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .select_one::<OwnedChildOriginRow>("kernel_process_lifecycle", &record_id)
                        .await
                })
            })
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "owned child canonical process row missing".to_owned())?;
        let scope = &self.original_scope;
        if row.process_uuid != process.process_uuid
            || row.os_pid != process.os_pid.map(i64::from)
            || row.parent_session_id.as_deref() != Some(process.parent_session_id.as_str())
            || row.owner_account_id != Some(scope.account_uuid.to_string())
            || row.actor_principal_id != Some(scope.actor_uuid.to_string())
            || row.authenticated_session_id != Some(scope.session_uuid.to_string())
            || row.access_space_id != Some(scope.access_space_uuid.to_string())
            || row.workspace_id.as_deref() != Some(scope.workspace_id.as_str())
        {
            return Err("owned child canonical identity or original scope mismatch".into());
        }
        Ok(())
    }
}

fn owned_reclaimer(
    storage: SurrealStorage,
    killed: &crate::runtime_child::KilledChildEvidence,
) -> Arc<dyn RestartOrphanReclaimer> {
    let scope = handshake_core::process_ledger::ReclaimResourceScope::from_exact(
        &killed.ready.original_scope,
    )
    .expect("killed child attribution is an exact reclaimer scope");
    Arc::new(
        handshake_core::process_ledger::restart_resume::SurrealRestartOrphanReclaimer::new(
            Arc::new(ObservedReclaimStore {
                inner: handshake_core::process_ledger::SurrealProcessLedgerStore::new(
                    storage.clone(),
                ),
                storage: storage.clone(),
            }),
            Arc::new(OwnedReapedChildCleanup::from_killed_child(
                storage, scope, killed,
            )),
        ),
    )
}

struct ObservedReclaimStore {
    inner: handshake_core::process_ledger::SurrealProcessLedgerStore,
    storage: SurrealStorage,
}

#[async_trait]
impl RestartReclaimStore for ObservedReclaimStore {
    async fn claim_active(
        &self,
        session: &str,
    ) -> Result<
        Vec<handshake_core::process_ledger::ReclaimableProcess>,
        handshake_core::process_ledger::ProcessLedgerError,
    > {
        self.inner.claim_active(session).await
    }
    async fn mark_cleanup_completed(
        &self,
        process: &handshake_core::process_ledger::ReclaimableProcess,
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        self.inner.mark_cleanup_completed(process).await?;
        let row: ProcessProofRow = select_one(
            &self.storage,
            "kernel_process_lifecycle",
            process.process_uuid.to_string(),
        )
        .await
        .expect("canonical cleanup marker row");
        assert_eq!(row.stopped_at, Some(process.reclaim_claimed_at));
        assert_eq!(
            row.stop_reason.as_deref(),
            Some(process.reclaim_expected_killed_reason.as_str())
        );
        Ok(())
    }
    async fn write_reclaim_stop(
        &self,
        process: &handshake_core::process_ledger::ReclaimableProcess,
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        self.inner.write_reclaim_stop(process).await?;
        let row: ProcessProofRow = select_one(
            &self.storage,
            "kernel_process_lifecycle",
            process.process_uuid.to_string(),
        )
        .await
        .expect("canonical terminal STOP row");
        assert!(row.stopped_at.is_some());
        assert_eq!(row.stop_reason.as_deref(), Some("reclaim"));
        assert_eq!(row.exit_code, Some(-1));
        Ok(())
    }
    async fn abandon(
        &self,
        processes: &[handshake_core::process_ledger::ReclaimableProcess],
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        self.inner.abandon(processes).await
    }
}

struct DeferredOwnedReclaimer {
    retained: Arc<Mutex<Option<SurrealStorage>>>,
    killed: crate::runtime_child::KilledChildEvidence,
}

#[derive(SurrealValue)]
struct CheckpointProofRow {
    checkpoint_id: uuid::Uuid,
    session_id: uuid::Uuid,
    state_kind: String,
}

#[derive(SurrealValue)]
struct ProcessProofRow {
    process_uuid: uuid::Uuid,
    stopped_at: Option<chrono::DateTime<chrono::Utc>>,
    stop_reason: Option<String>,
    exit_code: Option<i64>,
}

#[derive(SurrealValue)]
struct RestartReportProofRow {
    report_id: uuid::Uuid,
    sessions_resumed: serde_json::Value,
    sessions_recovery_failed: serde_json::Value,
    fr_events_emitted: Vec<String>,
}

#[derive(SurrealValue)]
struct QueueProofRow {
    session_run_id: String,
    state: String,
}

async fn select_one<R: SurrealValue + Send + 'static>(
    storage: &SurrealStorage,
    table: &'static str,
    record_id: String,
) -> Option<R> {
    storage
        .with_data_operation(move |database| {
            Box::pin(async move { database.select_one::<R>(table, &record_id).await })
        })
        .await
        .expect("read exact reopened runtime row")
}

#[async_trait]
impl RestartOrphanReclaimer for DeferredOwnedReclaimer {
    async fn reclaim_session(
        &self,
        session_run_id: &str,
    ) -> Result<u32, RestartResumeRuntimeError> {
        let storage = self
            .retained
            .lock()
            .map_err(|_| {
                RestartResumeRuntimeError::Store("reopened fixture handle poisoned".into())
            })?
            .as_ref()
            .cloned()
            .ok_or_else(|| {
                RestartResumeRuntimeError::Store("successful original-store reopen absent".into())
            })?;
        owned_reclaimer(storage, &self.killed)
            .reclaim_session(session_run_id)
            .await
    }
}

#[test]
fn mt141_runtime_chaos_retirement_mappings_are_explicit() {
    assert_eq!(MT141_RUNTIME_CHAOS_EXECUTABLE_MAPPINGS.len(), 5);
    for (retired, behavior, successor) in MT141_RUNTIME_CHAOS_EXECUTABLE_MAPPINGS {
        assert!(retired.starts_with("mt195_runtime_"));
        assert!(!behavior.is_empty());
        assert!(!successor.is_empty());
        assert!(!successor.contains("UNPROVEN"));
        assert!(!successor.contains("PARTIAL"));
    }
    assert_eq!(MT141_RUNTIME_CHAOS_NON_TEST_DISPOSITIONS.len(), 2);
    for (id, retired_behavior, rationale) in MT141_RUNTIME_CHAOS_NON_TEST_DISPOSITIONS {
        assert!(id.starts_with("external_shared_backend_"));
        assert!(retired_behavior.starts_with("retired "));
        assert!(rationale.starts_with("NO_EMBEDDED_EQUIVALENT:"));
    }
}

async fn assert_failed_recovery_evidence(
    storage: &SurrealStorage,
    report: &ResumeReport,
    session_id: uuid::Uuid,
) {
    assert_eq!(report.sessions_examined, 1);
    assert!(report.sessions_resumed.is_empty());
    assert_eq!(report.sessions_recovery_failed.len(), 1);
    assert_eq!(report.operator_decision_requests.len(), 1);
    assert!(report
        .fr_events_emitted
        .iter()
        .any(|event| event == FrEventId::RestartResumeSessionRecoveryFailed.as_str()));

    let persisted: RestartReportProofRow = select_one(
        storage,
        "kernel_restart_resume_report",
        report.report_id.to_string(),
    )
    .await
    .expect("failed restart report persists");
    assert_eq!(persisted.report_id, report.report_id);
    assert!(persisted
        .sessions_resumed
        .as_array()
        .expect("persisted resumed sessions array")
        .is_empty());
    assert_eq!(
        persisted
            .sessions_recovery_failed
            .as_array()
            .expect("persisted failed sessions array")
            .len(),
        1
    );
    assert!(persisted
        .fr_events_emitted
        .iter()
        .any(|event| event == FrEventId::RestartResumeSessionRecoveryFailed.as_str()));

    let queue: QueueProofRow =
        select_one(storage, "kernel_session_queue", format!("SR-{session_id}"))
            .await
            .expect("failed recovery queue row persists");
    assert_eq!(queue.state, "FAILED");

    let inspector = storage.test_inspector();
    let checkpoints = inspector
        .table_selector("kernel_session_checkpoint")
        .await
        .expect("select failed checkpoints");
    let checkpoint_session = checkpoints
        .field("session_id")
        .expect("select failed checkpoint session");
    let state_kind = checkpoints
        .field("state_kind")
        .expect("select failed checkpoint state kind");
    let checkpoint_rows = inspector
        .project(
            &checkpoints,
            &[checkpoint_session, state_kind],
            RowFilter::All,
        )
        .await
        .expect("project failed checkpoints");
    let expected_session_id = session_id.to_string();
    assert_eq!(
        checkpoint_rows
            .iter()
            .filter(|row| {
                row.values["session_id"].as_str() == Some(expected_session_id.as_str())
                    && row.values["state_kind"].as_str() == Some("recovery_failed")
            })
            .count(),
        1
    );

    let messages = inspector
        .table_selector("role_mailbox_message")
        .await
        .expect("select operator decision messages");
    let message_type = messages
        .field("message_type")
        .expect("select operator decision message type");
    let message_rows = inspector
        .project(&messages, &[message_type], RowFilter::All)
        .await
        .expect("project operator decision messages");
    assert_eq!(
        message_rows
            .iter()
            .filter(|row| row.values["message_type"].as_str() == Some("decision_request"))
            .count(),
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt195_runtime_hard_kill_child_leaves_reopened_checkpoint_and_active_process_evidence() {
    let root = tempfile::tempdir().expect("create MT-195 runtime root");
    let killed = crate::runtime_child::spawn_and_hard_kill_child(root.path());
    assert!(
        !killed.exit_status.success(),
        "the child must cross a real hard-kill boundary"
    );

    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&killed.data_dir)
            .expect("configure killed-child store reopen")
            .with_test_datastore_sync_durable(),
    )
    .await
    .expect("reopen killed-child embedded store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap killed-child reopened schema");
    crate::runtime_child::verify_original_seed_rows(
        &storage,
        &killed.ready.original_scope,
        killed.ready.session_id,
        killed.ready.checkpoint_id,
        Some(killed.ready.process_uuid),
    )
    .await;
    let checkpoint: CheckpointProofRow = select_one(
        &storage,
        "kernel_session_checkpoint",
        killed.ready.checkpoint_id.to_string(),
    )
    .await
    .expect("hard-killed child checkpoint survives reopen");
    assert_eq!(checkpoint.checkpoint_id, killed.ready.checkpoint_id);
    assert_eq!(checkpoint.session_id, killed.ready.session_id);
    assert_eq!(checkpoint.state_kind, "periodic");

    let process: ProcessProofRow = select_one(
        &storage,
        "kernel_process_lifecycle",
        killed.ready.process_uuid.to_string(),
    )
    .await
    .expect("hard-killed child process row survives reopen");
    assert_eq!(process.process_uuid, killed.ready.process_uuid);
    assert!(process.stopped_at.is_none());

    let process_store =
        handshake_core::process_ledger::SurrealProcessLedgerStore::new(storage.clone());
    let claimed = process_store
        .claim_active(&format!("SR-{}", killed.ready.session_id))
        .await
        .expect("claim original owned process for evidence rejection");
    assert_eq!(claimed.len(), 1);
    let scope = handshake_core::process_ledger::ReclaimResourceScope::from_exact(
        &killed.ready.original_scope,
    )
    .expect("hard-killed child attribution is an exact evidence scope");
    let cleanup =
        OwnedReapedChildCleanup::from_killed_child(storage.clone(), scope.clone(), &killed);
    let mut mismatch = claimed[0].clone();
    mismatch.process_uuid = uuid::Uuid::now_v7();
    assert!(
        cleanup.cleanup(&mismatch).await.is_err(),
        "unmatched process must be denied without killing a PID"
    );
    mismatch = claimed[0].clone();
    mismatch.os_pid = Some(killed.ready.os_pid.saturating_add(1));
    assert!(
        cleanup.cleanup(&mismatch).await.is_err(),
        "unmatched PID must be denied"
    );
    let unowned = OwnedReapedChildCleanup::no_owned_child(storage.clone(), scope);
    assert!(
        unowned.cleanup(&claimed[0]).await.is_err(),
        "no owned kill-and-wait receipt must be denied"
    );
    process_store
        .abandon(&claimed)
        .await
        .expect("restore exact claim after negative evidence assertions");
    drop(process_store);
    drop(cleanup);
    drop(unowned);

    let report = SurrealRestartResumeRunner::new(
        storage.clone(),
        handshake_core::process_ledger::ReclaimResourceScope::from_exact(
            &killed.ready.original_scope,
        )
        .expect("hard-killed child attribution is an exact recovery scope"),
        owned_reclaimer(storage.clone(), &killed),
    )
    .run()
    .await
    .expect("run recovery against killed-child store");
    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed.len(), 1);
    assert!(report.sessions_recovery_failed.is_empty());
    assert_eq!(report.total_replay_events, 1);

    let persisted_report: RestartReportProofRow = select_one(
        &storage,
        "kernel_restart_resume_report",
        report.report_id.to_string(),
    )
    .await
    .expect("restart report survives its durable write");
    assert_eq!(persisted_report.report_id, report.report_id);
    assert_eq!(
        persisted_report
            .sessions_resumed
            .as_array()
            .expect("persisted resumed sessions array")
            .len(),
        1
    );
    assert!(persisted_report
        .sessions_recovery_failed
        .as_array()
        .expect("persisted failed sessions array")
        .is_empty());

    let session_run_id = format!("SR-{}", killed.ready.session_id);
    let queue: QueueProofRow = select_one(&storage, "kernel_session_queue", session_run_id.clone())
        .await
        .expect("recovered queue row persists");
    assert_eq!(queue.session_run_id, session_run_id);
    assert_eq!(queue.state, "RETRY_SCHEDULED");

    let inspector = storage.test_inspector();
    let checkpoints = inspector
        .table_selector("kernel_session_checkpoint")
        .await
        .expect("select checkpoint table");
    let state_kind = checkpoints
        .field("state_kind")
        .expect("select checkpoint state kind");
    let session_id = checkpoints
        .field("session_id")
        .expect("select checkpoint session id");
    let compact_state = checkpoints
        .field("compact_state")
        .expect("select checkpoint compact state");
    let rows = inspector
        .project(
            &checkpoints,
            &[state_kind, session_id, compact_state],
            RowFilter::All,
        )
        .await
        .expect("project recovered checkpoints");
    let post_failure = rows
        .iter()
        .filter(|row| {
            row.values["state_kind"] == "post_failure"
                && row.values["session_id"] == killed.ready.session_id.to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(post_failure.len(), 1);
    assert_eq!(post_failure[0].values["compact_state"]["counter"], 3);

    storage
        .shutdown()
        .await
        .expect("close recovered killed-child store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt195_runtime_real_handshake_core_binary_runs_startup_recovery_and_persists_report() {
    let root = tempfile::tempdir().expect("create real-binary recovery root");
    let data_dir = root.path().join("real-binary-store");
    let seeded = crate::runtime_child::seed_closed_recovery_store(&data_dir).await;
    let process = crate::runtime_child::run_real_handshake_core_startup_recovery(
        root.path(),
        &data_dir,
        &seeded,
    );
    assert!(
        process.status.success(),
        "real handshake_core startup recovery"
    );
    let report_file: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&process.report_file).expect("read product startup recovery report"),
    )
    .expect("decode product startup recovery report");
    assert_eq!(report_file["sessions_examined"], 1);
    assert_eq!(report_file["sessions_resumed"], 1);
    assert_eq!(report_file["sessions_recovery_failed"], 0);

    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&data_dir)
            .expect("configure product-recovered store reopen")
            .with_test_datastore_sync_durable(),
    )
    .await
    .expect("reopen product-recovered store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap product-recovered store");
    crate::runtime_child::verify_original_seed_rows(
        &storage,
        &seeded.original_scope,
        seeded.session_id,
        seeded.checkpoint_id,
        None,
    )
    .await;
    let report_id = report_file["report_id"]
        .as_str()
        .expect("product report id")
        .to_owned();
    let persisted: RestartReportProofRow =
        select_one(&storage, "kernel_restart_resume_report", report_id)
            .await
            .expect("product startup report persists");
    assert_eq!(
        persisted
            .sessions_resumed
            .as_array()
            .expect("persisted product resumed sessions")
            .len(),
        1
    );
    assert!(persisted
        .sessions_recovery_failed
        .as_array()
        .expect("persisted product failed sessions")
        .is_empty());
    let queue: QueueProofRow = select_one(
        &storage,
        "kernel_session_queue",
        format!("SR-{}", seeded.session_id),
    )
    .await
    .expect("product-recovered queue row persists");
    assert_eq!(queue.state, "RETRY_SCHEDULED");
    let inspector = storage.test_inspector();
    let checkpoints = inspector
        .table_selector("kernel_session_checkpoint")
        .await
        .expect("select product-recovered checkpoints");
    let checkpoint_id = checkpoints
        .field("checkpoint_id")
        .expect("select product checkpoint id");
    let session_id = checkpoints
        .field("session_id")
        .expect("select product checkpoint session");
    let state_kind = checkpoints
        .field("state_kind")
        .expect("select product checkpoint state kind");
    let compact_state = checkpoints
        .field("compact_state")
        .expect("select product checkpoint state");
    let rows = inspector
        .project(
            &checkpoints,
            &[checkpoint_id, session_id, state_kind, compact_state],
            RowFilter::All,
        )
        .await
        .expect("project product-recovered checkpoints");
    assert!(rows.iter().any(|row| {
        row.values["checkpoint_id"] == seeded.checkpoint_id.to_string()
            && row.values["state_kind"] == "periodic"
    }));
    let post_failure = rows
        .iter()
        .filter(|row| {
            row.values["session_id"] == seeded.session_id.to_string()
                && row.values["state_kind"] == "post_failure"
        })
        .collect::<Vec<_>>();
    assert_eq!(post_failure.len(), 1);
    assert_eq!(post_failure[0].values["compact_state"]["counter"], 3);
    storage
        .shutdown()
        .await
        .expect("close product-recovered store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt195_runtime_event_gap_persists_failed_report_and_decision_evidence() {
    let root = tempfile::tempdir().expect("create event-gap recovery root");
    let killed = crate::runtime_child::spawn_and_hard_kill_child(root.path());
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&killed.data_dir)
            .expect("configure event-gap store reopen")
            .with_test_datastore_sync_durable(),
    )
    .await
    .expect("reopen event-gap store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap event-gap store");
    crate::runtime_child::verify_original_seed_rows(
        &storage,
        &killed.ready.original_scope,
        killed.ready.session_id,
        killed.ready.checkpoint_id,
        Some(killed.ready.process_uuid),
    )
    .await;
    let runner = SurrealRestartResumeRunner::new(
        storage.clone(),
        handshake_core::process_ledger::ReclaimResourceScope::from_exact(
            &killed.ready.original_scope,
        )
        .expect("event-gap child attribution is an exact recovery scope"),
        owned_reclaimer(storage.clone(), &killed),
    );
    assert_eq!(runner.consume_event_sequence_for_test().await.unwrap(), 2);
    let session_run_id = format!("SR-{}", killed.ready.session_id);
    let gap_tail = NewKernelEvent::builder(
        "KTR-MT195-HARD-KILL",
        &session_run_id,
        KernelEventType::ModelResponseRecorded,
        KernelActor::System("mt195-event-gap".to_owned()),
    )
    .aggregate("session_run", &session_run_id)
    .idempotency_key(format!("mt195-gap-tail-{}", killed.ready.session_id))
    .source_component("mt195-event-gap")
    .payload(serde_json::json!({"by": 4}))
    .build()
    .expect("build event-gap tail event");
    let tail_sequence = crate::runtime_child::seed_original_replay_event(
        &storage,
        &killed.ready.original_scope,
        gap_tail,
    )
    .await;
    assert_eq!(tail_sequence, 3);
    let report = runner.run().await.expect("run event-gap recovery");
    assert_failed_recovery_evidence(&storage, &report, killed.ready.session_id).await;
    storage.shutdown().await.expect("close event-gap store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt195_runtime_operator_cancel_persists_failed_report_and_decision_evidence() {
    let root = tempfile::tempdir().expect("create operator-cancel recovery root");
    let killed = crate::runtime_child::spawn_and_hard_kill_child(root.path());
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&killed.data_dir)
            .expect("configure operator-cancel store reopen")
            .with_test_datastore_sync_durable(),
    )
    .await
    .expect("reopen operator-cancel store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap operator-cancel store");
    crate::runtime_child::verify_original_seed_rows(
        &storage,
        &killed.ready.original_scope,
        killed.ready.session_id,
        killed.ready.checkpoint_id,
        Some(killed.ready.process_uuid),
    )
    .await;
    let runner = SurrealRestartResumeRunner::new(
        storage.clone(),
        handshake_core::process_ledger::ReclaimResourceScope::from_exact(
            &killed.ready.original_scope,
        )
        .expect("operator-cancel child attribution is an exact recovery scope"),
        owned_reclaimer(storage.clone(), &killed),
    );
    runner.arm_operator_cancel_before_resume();
    let report = runner.run().await.expect("run operator-cancel recovery");
    assert_failed_recovery_evidence(&storage, &report, killed.ready.session_id).await;
    assert!(matches!(
        &report.sessions_recovery_failed[0].1,
        handshake_core::session_checkpoint::ResumeError::SessionApplyError { reason }
            if reason == "operator_cancel_during_recovery"
    ));
    storage
        .shutdown()
        .await
        .expect("close operator-cancel store");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt195_runtime_transient_embedded_unavailable_backs_off_and_resumes_after_return() {
    let root = tempfile::tempdir().expect("create transient recovery root");
    let killed = crate::runtime_child::spawn_and_hard_kill_child(root.path());
    let attempts = Arc::new(AtomicUsize::new(0));
    let retained = Arc::new(Mutex::new(None::<SurrealStorage>));
    let data_dir = killed.data_dir.clone();
    let evidence = SurrealRestartResumeRunner::run_with_db_backoff(
        {
            let attempts = Arc::clone(&attempts);
            let retained = Arc::clone(&retained);
            let original = killed.ready.clone();
            move || {
                let origin = original.clone();
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                let data_dir = data_dir.clone();
                let retained = Arc::clone(&retained);
                async move {
                    if attempt == 0 {
                        return Err(RestartResumeRuntimeError::Storage(
                            SurrealStorageError::Closed,
                        ));
                    }
                    let storage = SurrealStorage::open(
                        SurrealStorageConfig::for_data_dir(&data_dir)
                            .expect("configure transient-return store")
                            .with_test_datastore_sync_durable(),
                    )
                    .await?;
                    bootstrap_schema(&storage).await?;
                    crate::runtime_child::verify_original_seed_rows(
                        &storage,
                        &origin.original_scope,
                        origin.session_id,
                        origin.checkpoint_id,
                        Some(origin.process_uuid),
                    )
                    .await;
                    *retained.lock().expect("retain reopened storage") = Some(storage.clone());
                    Ok(storage)
                }
            }
        },
        RestartResumeDbBackoffPolicy::new(2, Duration::from_millis(1)),
        handshake_core::process_ledger::ReclaimResourceScope::from_exact(
            &killed.ready.original_scope,
        )
        .expect("transient-return child attribution is an exact recovery scope"),
        Arc::new(DeferredOwnedReclaimer {
            retained: Arc::clone(&retained),
            killed: killed.clone(),
        }),
    )
    .await
    .expect("resume after transient embedded unavailability");
    assert_eq!(evidence.db_unavailable_attempts, 1);
    assert!(evidence.backoff_observed);
    assert_eq!(evidence.backoff_delay_ms, vec![1]);
    assert_eq!(evidence.report.sessions_resumed.len(), 1);
    assert!(evidence.report.sessions_recovery_failed.is_empty());
    assert!(evidence
        .report
        .fr_events_emitted
        .iter()
        .any(|event| event == FrEventId::RestartResumeDbUnavailable.as_str()));

    let storage = retained
        .lock()
        .expect("take reopened storage")
        .take()
        .expect("storage factory retained successful store");
    let inspector = storage.test_inspector();
    let reports = inspector
        .table_selector("kernel_restart_resume_report")
        .await
        .expect("select restart reports");
    assert_eq!(
        inspector
            .row_count(&reports, RowFilter::All)
            .await
            .expect("count durable restart reports"),
        1
    );
    storage
        .shutdown()
        .await
        .expect("close transient-return store");
}
