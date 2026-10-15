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

#[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
mod full_main_shutdown_acceptance {
    use crate::runtime_child::FullMainChild;
    use handshake_core::{
        api::account_scope::ProductLocalResourceScope,
        flight_recorder::shutdown_proof::{ProofAction, ProofFailureClass, ProofObservation},
        swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    };
    use serde::Deserialize;
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::{
        collections::BTreeMap,
        io::{self, IsTerminal, Read},
        path::{Path, PathBuf},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    const MAX_BYTES: usize = 65_536;
    const APPROVED_CONFIG_SHA256: &str = "71700f65e37ccc0db1b6465035a033d3228ea441b295aada1b38b274f070ac17";

    fn incomplete(reason: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, format!("MT188_INCOMPLETE: {reason}"))
    }

    #[derive(Clone, Copy, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum Method { Get, Post }

    #[derive(Clone, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GenuineRequest {
        method: Method,
        path: String,
        headers: BTreeMap<String, String>,
        json: Option<Value>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ScopeProjection {
        owner_account_id: String,
        actor_principal_id: String,
        authenticated_session_id: String,
        access_space_id: String,
        workspace_id: String,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GenuineCase {
        output_directory: PathBuf,
        data_directory: PathBuf,
        approval_configuration: PathBuf,
        executable_sha256: String,
        listen_addr: String,
        deadline_unix_ms: u64,
        bootstrap_requests: Vec<GenuineRequest>,
        context_read: GenuineRequest,
        context_projection: ScopeProjection,
        original_scope: ExactResourceScopeAttribution,
        launch: GenuineRequest,
        launch_instance_pointer: String,
        launch_run_pointer: String,
        canonical_flight_recorder_read: GenuineRequest,
        canonical_events_pointer: String,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct GenuineInput {
        version: u8,
        artifact_root: PathBuf,
        positive: GenuineCase,
        negative: GenuineCase,
    }

    fn read_genuine_input() -> io::Result<GenuineInput> {
        if io::stdin().is_terminal() {
            return Err(incomplete("supply one bounded version1 genuine positive/negative configuration frame on the test process stdin; no fixture or skip is available"));
        }
        let until = Instant::now() + Duration::from_secs(5);
        let mut input = owned_stdin()?;
        let mut length = [0u8; 4];
        read_input_exact(&mut input, &mut length, until)?;
        let size = u32::from_be_bytes(length) as usize;
        if size == 0 || size > MAX_BYTES { return Err(incomplete("genuine input frame exceeds 65536 bytes")); }
        let mut bytes = vec![0; size];
        read_input_exact(&mut input, &mut bytes, until)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| incomplete("genuine input is not UTF8"))?;
        serde_json::from_str(text).map_err(|_| incomplete("genuine input does not match the version1 typed case contract"))
    }

    #[cfg(windows)]
    fn owned_stdin() -> io::Result<std::fs::File> {
        use std::os::windows::io::AsHandle;
        Ok(std::fs::File::from(io::stdin().as_handle().try_clone_to_owned()?))
    }

    #[cfg(unix)]
    fn owned_stdin() -> io::Result<std::fs::File> {
        use std::os::fd::AsFd;
        Ok(std::fs::File::from(io::stdin().as_fd().try_clone_to_owned()?))
    }

    fn read_input_exact(input: &mut std::fs::File, mut bytes: &mut [u8], until: Instant) -> io::Result<()> {
        while !bytes.is_empty() {
            let available = input_read_capacity(input, bytes.len(), until)?;
            match input.read(&mut bytes[..available]) {
                Ok(0) => return Err(incomplete("genuine execution configuration is absent or truncated on parent stdin")),
                Ok(count) => bytes = &mut bytes[count..],
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {},
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn input_read_capacity(input: &std::fs::File, remaining: usize, until: Instant) -> io::Result<usize> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{
            Storage::FileSystem::{GetFileType, FILE_TYPE_DISK, FILE_TYPE_PIPE},
            System::Pipes::PeekNamedPipe,
        };
        let handle = input.as_raw_handle();
        // This is an owned duplicate, not a mode change or close of process stdin.
        let kind = unsafe { GetFileType(handle) };
        loop {
            if Instant::now() >= until { return Err(incomplete("no complete genuine input frame within five seconds")); }
            if kind == FILE_TYPE_DISK { return Ok(remaining); }
            if kind != FILE_TYPE_PIPE { return Err(incomplete("genuine stdin must be a pipe or regular input file")); }
            let mut available = 0u32;
            if unsafe { PeekNamedPipe(handle, std::ptr::null_mut(), 0,
                std::ptr::null_mut(), &mut available, std::ptr::null_mut()) } == 0 {
                return Err(incomplete("genuine input pipe is closed or unavailable"));
            }
            if available != 0 { return Ok(remaining.min(available as usize)); }
            std::thread::sleep(Duration::from_millis(5).min(until.saturating_duration_since(Instant::now())));
        }
    }

    #[cfg(unix)]
    fn input_read_capacity(input: &std::fs::File, remaining: usize, until: Instant) -> io::Result<usize> {
        use std::os::fd::AsRawFd;
        loop {
            let wait = until.checked_duration_since(Instant::now())
                .ok_or_else(|| incomplete("no complete genuine input frame within five seconds"))?;
            let mut descriptor = libc::pollfd { fd: input.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            let timeout = wait.as_millis().clamp(1, i32::MAX as u128) as i32;
            let result = unsafe { libc::poll(&mut descriptor, 1, timeout) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted { continue; }
                return Err(error);
            }
            if result == 0 { return Err(incomplete("no complete genuine input frame within five seconds")); }
            if descriptor.revents & libc::POLLNVAL != 0 { return Err(incomplete("genuine input descriptor is invalid")); }
            return Ok(remaining);
        }
    }

    fn deadline(case: &GenuineCase) -> io::Result<Instant> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|_| incomplete("host clock precedes UNIX epoch"))?;
        let target = Duration::from_millis(case.deadline_unix_ms);
        let remaining = target.checked_sub(now).ok_or_else(|| incomplete("the granted absolute proof deadline already elapsed"))?;
        Instant::now().checked_add(remaining).ok_or_else(|| incomplete("proof deadline is out of range"))
    }

    fn validate_case(root: &Path, case: &GenuineCase) -> io::Result<()> {
        for path in [&case.output_directory, &case.data_directory, &case.approval_configuration] {
            if !path.is_absolute() || !path.canonicalize()?.starts_with(root) {
                return Err(incomplete("approved output, actual data and existing config must resolve within the supplied owned artifact root"));
            }
        }
        if !case.output_directory.is_dir() || !case.data_directory.is_dir() {
            return Err(incomplete("approved existing output and genuine data directories are required; this case does not seed or provision a store"));
        }
        for filename in ["mt188-main-stderr.log", "mt188-main-listener.json"] {
            if case.output_directory.join(filename).exists() { return Err(incomplete("case outputs already exist; preserve them and supply an unused owned case directory")); }
        }
        let hash = format!("{:x}", Sha256::digest(std::fs::read(&case.approval_configuration)?));
        if hash != APPROVED_CONFIG_SHA256 { return Err(incomplete("existing approved registry configuration hash differs from its original binding")); }
        ProductLocalResourceScope::from_exact(case.original_scope.clone())
            .map_err(|_| incomplete("supplied scope is not a complete genuine account/Principal/session/Space/workspace tuple"))?;
        if !matches!(case.context_read.method, Method::Get)
            || !matches!(case.canonical_flight_recorder_read.method, Method::Get)
            || !matches!(case.launch.method, Method::Post) {
            return Err(incomplete("context/Flight Recorder reads must be GET and genuine launch must be POST"));
        }
        let path = &case.canonical_flight_recorder_read.path;
        if !path.starts_with("/flight_recorder?") && !path.starts_with("/api/flight_recorder?") {
            return Err(incomplete("canonical read must use the actual scoped Flight Recorder route"));
        }
        Ok(())
    }

    async fn send(
        client: &reqwest::Client,
        base: &str,
        request: GenuineRequest,
        until: Instant,
    ) -> io::Result<Value> {
        if !request.path.starts_with('/') || request.path.starts_with("//") || request.path.contains('#') {
            return Err(incomplete("HTTP input must name an existing relative product route on the owned child"));
        }
        let remaining = until.checked_duration_since(Instant::now()).ok_or_else(|| incomplete("HTTP proof deadline elapsed"))?;
        let mut builder = match request.method {
            Method::Get => client.get(format!("{base}{}", request.path)),
            Method::Post => client.post(format!("{base}{}", request.path)),
        }.timeout(remaining);
        for (name, value) in request.headers {
            let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| incomplete("invalid genuine HTTP header name"))?;
            let value = reqwest::header::HeaderValue::from_str(&value)
                .map_err(|_| incomplete("invalid genuine HTTP credential/header value"))?;
            builder = builder.header(name, value);
        }
        if let Some(json) = request.json { builder = builder.json(&json); }
        let mut response = builder.send().await.map_err(|_| incomplete("genuine product request failed; inspect the owned child diagnostic log"))?;
        if !response.status().is_success() { return Err(incomplete("product rejected genuine setup/context/launch/read input; inspect the actual HTTP admission")); }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| incomplete("product response read failed"))? {
            if bytes.len().saturating_add(chunk.len()) > MAX_BYTES { return Err(incomplete("scope the genuine product response below 65536 bytes")); }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| incomplete("product did not return a JSON receipt"))
    }

    fn validate_context(response: &Value, case: &GenuineCase) -> io::Result<()> {
        let expected = serde_json::to_value(&case.original_scope).map_err(|_| incomplete("scope serialization"))?;
        for (field, pointer) in [
            ("owner_account_id", &case.context_projection.owner_account_id),
            ("actor_principal_id", &case.context_projection.actor_principal_id),
            ("authenticated_session_id", &case.context_projection.authenticated_session_id),
            ("access_space_id", &case.context_projection.access_space_id),
            ("workspace_id", &case.context_projection.workspace_id),
        ] {
            let value = expected.get(field).ok_or_else(|| incomplete("canonical scope field is missing"))?;
            if pointer.is_empty() || response.pointer(pointer) != Some(value) {
                return Err(incomplete("actual authenticated product context does not match the immutable supplied original scope"));
            }
        }
        Ok(())
    }

    struct OwnedLaunch(Option<tokio::task::JoinHandle<io::Result<Value>>>);

    impl OwnedLaunch {
        async fn result(&mut self, until: Instant, settlement: &mut Option<String>) -> io::Result<Value> {
            let task = self.0.as_mut().ok_or_else(|| incomplete("launch task already consumed"))?;
            let result = tokio::time::timeout_at(tokio::time::Instant::from_std(until), task).await
                .map_err(|_| incomplete("genuine launch did not finish before the original deadline"))?;
            *settlement = Some(launch_outcome(&result));
            self.0.take();
            result.map_err(|_| incomplete("genuine launch task failed"))?
        }

        async fn observe_failure_settlement(&mut self) -> Option<String> {
            if !self.0.as_ref().is_some_and(tokio::task::JoinHandle::is_finished) {
                return None;
            }
            let task = self.0.take()?;
            Some(launch_outcome(&task.await))
        }

    }

    impl Drop for OwnedLaunch {
        fn drop(&mut self) {
            if let Some(task) = self.0.take() { task.abort(); }
        }
    }

    fn launch_outcome(result: &Result<io::Result<Value>, tokio::task::JoinError>) -> String {
        match result {
            Ok(Ok(_)) => "success".to_owned(),
            Ok(Err(error)) => format!("io_error:{}", error_cause(error)),
            Err(error) if error.is_cancelled() => format!("join_cancelled;{}", message_fingerprint(&error.to_string())),
            Err(error) if error.is_panic() => format!("join_panicked;{}", message_fingerprint(&error.to_string())),
            Err(error) => format!("join_error_unclassified;{}", message_fingerprint(&error.to_string())),
        }
    }

    fn message_fingerprint(message: &str) -> String {
        format!("message_utf8_len={};message_sha256={:x}", message.len(), Sha256::digest(message.as_bytes()))
    }

    fn error_cause(error: &io::Error) -> String {
        format!("io_kind={:?};raw_os_error={:?};{}", error.kind(), error.raw_os_error(),
            message_fingerprint(&error.to_string()))
    }

    fn panic_cause(payload: &(dyn std::any::Any + Send)) -> String {
        if let Some(message) = payload.downcast_ref::<String>() {
            format!("panic_payload_type=String;{}", message_fingerprint(message))
        } else if let Some(message) = payload.downcast_ref::<&str>() {
            format!("panic_payload_type=str;{}", message_fingerprint(message))
        } else {
            "panic_payload_type=opaque;message_fingerprint=unavailable".to_owned()
        }
    }

    struct CaseOwners<'a> {
        case: &'a GenuineCase,
        child: Option<FullMainChild>,
        client: Option<reqwest::Client>,
        launch: OwnedLaunch,
        launch_settlement: Option<String>,
    }

    impl CaseOwners<'_> {
        async fn retain_failed_case(&mut self, primary: &str) {
            loop {
                if self.launch_settlement.is_none() {
                    self.launch_settlement = self.launch.observe_failure_settlement().await;
                }
                let launch_settled = self.launch.0.is_none();
                let launch_outcome = self.launch_settlement.as_deref().unwrap_or(if launch_settled {
                    "no_outstanding_task"
                } else { "pending" });
                let pending = format!(
                    "{primary}; runner_pid={}; runner_status=failed_incomplete; case_owners_pending=true; launch_task_pending={}; launch_outcome={launch_outcome}",
                    std::process::id(), !launch_settled,
                );
                let child_settled = match self.child.as_mut() {
                    Some(child) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        child.observe_failure_settlement(&pending)
                    })).unwrap_or(false),
                    None => true,
                };
                if launch_settled && child_settled {
                    let completed = format!(
                        "{primary}; runner_pid={}; runner_status=failed_incomplete; case_owners_pending=false; launch_task_pending=false; launch_outcome={launch_outcome}",
                        std::process::id(),
                    );
                    let recorded = match self.child.as_mut() {
                        Some(child) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            child.observe_failure_settlement(&completed)
                        })).unwrap_or(false),
                        None => true,
                    };
                    if recorded { return; }
                }
                // Failure-only custody stays in this original task, without a new deadline.
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }

    async fn run_case(case: &GenuineCase, inject_insert_fault: bool) -> io::Result<()> {
        use futures::FutureExt;
        let mut owners = CaseOwners { case, child: None, client: None, launch: OwnedLaunch(None), launch_settlement: None };
        let outcome = std::panic::AssertUnwindSafe(run_case_body(&mut owners, inject_insert_fault))
            .catch_unwind().await;
        match outcome {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                owners.retain_failed_case(&format!("MT188_CASE_RETURNED_ERROR;{}", error_cause(&error))).await;
                Err(error)
            }
            Err(panic) => {
                owners.retain_failed_case(&format!("MT188_CASE_PANICKED;{}", panic_cause(panic.as_ref()))).await;
                std::panic::resume_unwind(panic)
            }
        }
    }

    async fn run_case_body(owners: &mut CaseOwners<'_>, inject_insert_fault: bool) -> io::Result<()> {
        let CaseOwners { case, child, client, launch, launch_settlement } = owners;
        let case = *case;
        let until = deadline(case)?;
        *child = Some(FullMainChild::spawn(&case.output_directory, &case.data_directory,
            &case.listen_addr, &case.approval_configuration, until)?);
        let child = child.as_mut().ok_or_else(|| incomplete("owned main was not retained"))?;
        let identity = serde_json::to_value(child.identity()?).map_err(|_| incomplete("actual child identity serialization"))?;
        if case.executable_sha256.len() != 64 || !case.executable_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || identity["executable_sha256"].as_str() != Some(case.executable_sha256.as_str()) {
            return Err(incomplete("actual owned process image hash differs from the supplied frozen candidate executable"));
        }
        eprintln!("MT188_OWNED_MAIN_IDENTITY {identity}");
        let base = child.await_listener(&case.output_directory)?;
        *client = Some(reqwest::Client::builder().redirect(reqwest::redirect::Policy::none())
            .build().map_err(|_| incomplete("HTTP client initialization"))?);
        let client = client.as_ref().ok_or_else(|| incomplete("HTTP client was not retained"))?;
        for request in &case.bootstrap_requests {
            send(&client, &base, request.clone(), until).await?;
        }
        let context = send(&client, &base, case.context_read.clone(), until).await?;
        validate_context(&context, case)?;
        child.request(ProofAction::ObserveAllocation {
            scope: case.original_scope.clone(), deadline_unix_ms: case.deadline_unix_ms,
        })?;
        let launch_client = (*client).clone();
        let launch_base = base.clone();
        let launch_request = case.launch.clone();
        launch.0 = Some(tokio::spawn(async move {
            send(&launch_client, &launch_base, launch_request, until).await
        }));
        let allocation = match child.request(ProofAction::AwaitAllocation)?.receipt {
            ProofObservation::AllocationPending { allocation } => allocation,
            _ => return Err(incomplete("actual producer allocation was not observed")),
        };
        if serde_json::to_value(&allocation.scope).ok() != serde_json::to_value(&case.original_scope).ok()
            || allocation.instance_id.is_empty() || allocation.parent_session_id.is_empty()
            || allocation.run_id.is_empty() || allocation.lane_id.is_empty() || allocation.trace_id.is_empty() {
            return Err(incomplete("actual allocation lacks the original scope or genuine producer identities"));
        }
        child.request(ProofAction::SelectAllocation { pending_id: allocation.pending_id, allocation: allocation.clone() })?;
        let (target_event, target_process) = match child.request(ProofAction::AwaitEvent)?.receipt {
            ProofObservation::PendingEvent { event_id, process_uuid } => (event_id, process_uuid),
            _ => return Err(incomplete("genuine SessionSpawned did not reach the selected pre-SQL boundary")),
        };
        if target_event.is_nil() || target_process.is_nil() { return Err(incomplete("actual event/process identity is nil")); }
        if inject_insert_fault { child.request(ProofAction::Arm { event_id: target_event })?; }
        child.request(ProofAction::ReleaseEvent { event_id: target_event })?;
        if inject_insert_fault {
            match child.request(ProofAction::AwaitFailure)?.receipt {
                ProofObservation::StickyFailure { event_id, failure_class: ProofFailureClass::OwnedTemporarySequenceExhausted,
                    error, restore_error: None } if event_id == target_event && !error.is_empty() => {},
                _ => return Err(incomplete("actual target INSERT error, sticky failure and local physical restoration were not all observed")),
            }
            child.request(ProofAction::ConfirmRestored)?;
        }
        let launched = launch.result(until, launch_settlement).await?;
        if launched.pointer(&case.launch_instance_pointer).and_then(Value::as_str) != Some(allocation.instance_id.as_str())
            || launched.pointer(&case.launch_run_pointer).and_then(Value::as_str) != Some(allocation.run_id.as_str()) {
            return Err(incomplete("actual launch response does not bind the selected producer instance and run"));
        }
        let healthy_event = match child.request(ProofAction::AwaitHealthy)?.receipt {
            ProofObservation::HealthyEvent { event_id } if !event_id.is_nil() => event_id,
            _ => return Err(incomplete("no subsequent genuine healthy recorder INSERT was observed")),
        };
        if inject_insert_fault && healthy_event == target_event { return Err(incomplete("failed target cannot substitute for a subsequent healthy event")); }
        let mut read = case.canonical_flight_recorder_read.clone();
        read.path = read.path.replace("{event_id}", &healthy_event.to_string())
            .replace("{trace_id}", &allocation.trace_id);
        let canonical = send(&client, &base, read, until).await?;
        let events = canonical.pointer(&case.canonical_events_pointer).and_then(Value::as_array)
            .ok_or_else(|| incomplete("canonical Flight Recorder response has no actual event list"))?;
        let expected_id = healthy_event.to_string();
        let event = events.iter().find(|event| event["event_id"].as_str() == Some(expected_id.as_str()))
            .ok_or_else(|| incomplete("actual healthy event is absent from canonical Flight Recorder read"))?;
        if event["trace_id"].as_str() != Some(allocation.trace_id.as_str()) {
            return Err(incomplete("canonical healthy event trace differs from the selected real invocation"));
        }
        child.request(ProofAction::Shutdown)?;
        let status = child.wait_for_exit()?;
        let expected_exit = if inject_insert_fault { 1 } else { 0 };
        if status.code() != Some(expected_exit) { return Err(incomplete("actual full main numeric exit did not match the positive/negative boundary")); }
        let diagnostic = std::fs::read_to_string(case.output_directory.join("mt188-main-stderr.log"))?;
        let ansi = regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]")
            .map_err(|_| incomplete("diagnostic ANSI normalization"))?;
        let diagnostic = ansi.replace_all(&diagnostic, "");
        let failure = "shutdown durability was not fully proven";
        if inject_insert_fault {
            let line = diagnostic.lines().find(|line| line.contains(failure))
                .ok_or_else(|| incomplete("negative main omitted its actual shutdown durability diagnostic"))?;
            for field in ["connection_drain_timed_out=false", "coordinator_quiesced=true",
                "coordinator_error=None", "lease_released=true", "lease_retained_reason=None",
                "process_reclaim_quiesced=true", "swarm_events_flushed=false", "operator_chat_ledger_flushed=Some(true)"] {
                if !line.contains(field) { return Err(incomplete("negative case did not prove otherwise-successful actual shutdown drains")); }
            }
        } else if diagnostic.contains(failure) {
            return Err(incomplete("positive case emitted a failed shutdown durability diagnostic"));
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn mt188_real_full_main_positive_and_recorder_only_negative_shutdown() -> io::Result<()> {
        // Parent stdin is consumed once. It is separate from each owned child's private control pipe.
        let input = read_genuine_input()?;
        if input.version != 1 || !input.artifact_root.is_absolute() {
            return Err(incomplete("version1 approved absolute artifact root required"));
        }
        let root = input.artifact_root.canonicalize()?;
        validate_case(&root, &input.positive)?;
        validate_case(&root, &input.negative)?;
        if input.positive.output_directory.canonicalize()? == input.negative.output_directory.canonicalize()?
            || input.positive.data_directory.canonicalize()? == input.negative.data_directory.canonicalize()? {
            return Err(incomplete("serial positive/negative cases require separate approved owned outputs and genuine data roots"));
        }
        run_case(&input.positive, false).await?;
        run_case(&input.negative, true).await
    }
}
