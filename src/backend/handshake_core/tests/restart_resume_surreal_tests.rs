mod process_ledger_surreal_support;

use chrono::{DateTime, Utc};
use handshake_core::{
    process_ledger::{restart_resume::SurrealRestartResumeRunner, ReclaimResourceScope},
    storage::surreal::SurrealStorage,
};
use process_ledger_surreal_support::ProcessLedgerSurrealHarness;
use serde_json::{json, Value};
use surrealdb::types::{RecordId, SurrealValue};
use uuid::Uuid;

#[derive(Debug, SurrealValue)]
struct SeedQueueBindings {
    record: RecordId,
    session_run_id: String,
    kernel_task_run_id: String,
    state: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct SeedCheckpointBindings {
    record: RecordId,
    checkpoint_id: Uuid,
    session_id: Uuid,
    model_session_id: Uuid,
    last_event_ledger_seq: i64,
    compact_state: Value,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct SeedEventBindings {
    record: RecordId,
    event_id: String,
    event_sequence: i64,
    session_run_id: String,
    kernel_task_run_id: String,
    payload: Value,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ExactRecordBindings {
    record: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ExactSessionBindings {
    session_id: Uuid,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct QueueProbe {
    state: String,
    claimed_by: Option<String>,
    lease_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, SurrealValue)]
struct CheckpointProbe {
    compact_state: Value,
    last_event_ledger_seq: i64,
    state_kind: String,
    /// Projected only because SurrealDB requires every `ORDER BY` idiom to be
    /// present in the statement's selection; the probe reads newest-first.
    #[allow(dead_code)]
    created_at_utc: DateTime<Utc>,
}

#[derive(Debug, SurrealValue)]
struct ReportProbe {
    sessions_examined: i64,
    sessions_resumed: Vec<Value>,
    sessions_recovery_failed: Vec<Value>,
    operator_decision_requests: Vec<Value>,
    fr_events_emitted: Vec<String>,
    schema_version: i64,
}

fn scope_values(scope: &ReclaimResourceScope) -> (String, String, String, String, String) {
    (
        scope.account_uuid.to_string(),
        scope.actor_uuid.to_string(),
        scope.session_uuid.to_string(),
        scope.access_space_uuid.to_string(),
        scope.workspace_id.clone(),
    )
}

fn exact_record(scope: &ReclaimResourceScope, record: RecordId) -> ExactRecordBindings {
    let (owner, actor, session, access_space, workspace) = scope_values(scope);
    ExactRecordBindings {
        record,
        owner_account_id: owner,
        actor_principal_id: actor,
        authenticated_session_id: session,
        access_space_id: access_space,
        workspace_id: workspace,
    }
}

const SEED_QUEUE: &str = r#"
CREATE $record CONTENT {
    session_run_id: $session_run_id,
    kernel_task_run_id: $kernel_task_run_id,
    adapter_id: 'restart-resume-surreal-test', state: $state,
    claimed_by: 'previous-worker', lease_expires_at: time::now() + 30m,
    attempt_count: 1, available_at: time::now(), created_at: time::now(),
    updated_at: time::now(), owner_account_id: $owner_account_id,
    actor_principal_id: $actor_principal_id,
    authenticated_session_id: $authenticated_session_id,
    access_space_id: $access_space_id, workspace_id: $workspace_id
};
"#;

const SEED_CHECKPOINT: &str = r#"
CREATE $record CONTENT {
    checkpoint_id: $checkpoint_id, session_id: $session_id,
    model_session_id: $model_session_id,
    last_event_ledger_seq: $last_event_ledger_seq,
    compact_state: $compact_state, state_kind: 'periodic', pending_artifacts: [],
    created_at_utc: time::now(), created_by_process: 1234, schema_version: 1,
    owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
    authenticated_session_id: $authenticated_session_id,
    access_space_id: $access_space_id, workspace_id: $workspace_id
};
"#;

const SEED_EVENT: &str = r#"
CREATE $record CONTENT {
    event_id: $event_id, event_sequence: $event_sequence,
    event_version: 'kernel_event_v1', kernel_task_run_id: $kernel_task_run_id,
    session_run_id: $session_run_id, aggregate_type: 'session_run',
    aggregate_id: $session_run_id, idempotency_key: $event_id,
    event_type: 'MODEL_RESPONSE_RECORDED', actor_kind: 'session_broker',
    actor_id: 'restart-resume-surreal-test',
    payload_hash: '0000000000000000000000000000000000000000000000000000000000000000',
    source_component: 'restart_resume_surreal_tests', payload: $payload,
    owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
    authenticated_session_id: $authenticated_session_id,
    access_space_id: $access_space_id, workspace_id: $workspace_id,
    created_at: time::now()
};
"#;

const READ_QUEUE: &str = r#"
SELECT state, claimed_by, lease_expires_at FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

const READ_LATEST_CHECKPOINT: &str = r#"
SELECT compact_state, last_event_ledger_seq, state_kind, created_at_utc
FROM kernel_session_checkpoint
WHERE session_id = $session_id
    AND owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id
ORDER BY created_at_utc DESC LIMIT 1;
"#;

const READ_REPORT: &str = r#"
SELECT sessions_examined, sessions_resumed, sessions_recovery_failed,
    operator_decision_requests, fr_events_emitted, schema_version
FROM ONLY $record
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

async fn execute<B: SurrealValue + Send + 'static>(
    storage: &SurrealStorage,
    statement: &'static str,
    bindings: B,
) {
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<surrealdb::types::Value, _>(statement, bindings)
                    .await
                    .map(|rows| rows.len())
            })
        })
        .await
        .expect("execute static exact-scope restart-resume fixture statement");
}

async fn seed_queue(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    session_id: Uuid,
    state: &str,
) -> RecordId {
    let record = RecordId::new("kernel_session_queue", session_id.to_string());
    let (owner, actor, session, access_space, workspace) = scope_values(scope);
    execute(
        storage,
        SEED_QUEUE,
        SeedQueueBindings {
            record: record.clone(),
            session_run_id: session_id.to_string(),
            kernel_task_run_id: format!("KTR-{session_id}"),
            state: state.to_string(),
            owner_account_id: owner,
            actor_principal_id: actor,
            authenticated_session_id: session,
            access_space_id: access_space,
            workspace_id: workspace,
        },
    )
    .await;
    record
}

async fn seed_checkpoint(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    session_id: Uuid,
    last_sequence: i64,
    compact_state: Value,
) {
    let checkpoint_id = Uuid::now_v7();
    let (owner, actor, session, access_space, workspace) = scope_values(scope);
    execute(
        storage,
        SEED_CHECKPOINT,
        SeedCheckpointBindings {
            record: RecordId::new("kernel_session_checkpoint", checkpoint_id.to_string()),
            checkpoint_id,
            session_id,
            model_session_id: Uuid::now_v7(),
            last_event_ledger_seq: last_sequence,
            compact_state,
            owner_account_id: owner,
            actor_principal_id: actor,
            authenticated_session_id: session,
            access_space_id: access_space,
            workspace_id: workspace,
        },
    )
    .await;
}

async fn seed_event(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    session_id: Uuid,
    event_sequence: i64,
    payload: Value,
) {
    let event_id = format!("KE-{}", Uuid::now_v7());
    let (owner, actor, session, access_space, workspace) = scope_values(scope);
    execute(
        storage,
        SEED_EVENT,
        SeedEventBindings {
            record: RecordId::new("kernel_event_ledger", event_id.clone()),
            event_id,
            event_sequence,
            session_run_id: session_id.to_string(),
            kernel_task_run_id: format!("KTR-{session_id}"),
            payload,
            owner_account_id: owner,
            actor_principal_id: actor,
            authenticated_session_id: session,
            access_space_id: access_space,
            workspace_id: workspace,
        },
    )
    .await;
}

async fn query_first<T: SurrealValue + Send + 'static>(
    storage: &SurrealStorage,
    statement: &'static str,
    bindings: ExactRecordBindings,
) -> Option<T> {
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                let values = database
                    .query_values::<surrealdb::types::Value, _>(statement, bindings)
                    .await?;
                if matches!(values.as_slice(), [surrealdb::types::Value::None]) {
                    return Ok(None);
                }
                let rows = values
                    .into_iter()
                    .map(|value| {
                        T::from_value(value)
                            .map_err(|error| surrealdb::Error::internal(error.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows.into_iter().next())
            })
        })
        .await
        .expect("query static exact-scope restart-resume fixture statement")
}

async fn latest_checkpoint(
    storage: &SurrealStorage,
    scope: &ReclaimResourceScope,
    session_id: Uuid,
) -> Option<CheckpointProbe> {
    let (owner, actor, session, access_space, workspace) = scope_values(scope);
    let bindings = ExactSessionBindings {
        session_id,
        owner_account_id: owner,
        actor_principal_id: actor,
        authenticated_session_id: session,
        access_space_id: access_space,
        workspace_id: workspace,
    };
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                let values = database
                    .query_values::<surrealdb::types::Value, _>(READ_LATEST_CHECKPOINT, bindings)
                    .await?;
                if matches!(values.as_slice(), [surrealdb::types::Value::None]) {
                    return Ok(None);
                }
                let rows = values
                    .into_iter()
                    .map(|value| {
                        CheckpointProbe::from_value(value)
                            .map_err(|error| surrealdb::Error::internal(error.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows.into_iter().next())
            })
        })
        .await
        .expect("read latest exact-scope restart-resume checkpoint")
}

#[tokio::test]
async fn startup_runner_resumes_persists_report_and_is_idempotent() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let storage = harness.storage();
    let scope = harness.resource_scope().clone();
    assert_eq!(
        serde_json::from_value::<ReclaimResourceScope>(
            serde_json::to_value(&scope).expect("serialize five-field scope")
        )
        .expect("five-field roundtrip"),
        scope
    );
    let exact = handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution::try_from_resource_scope(&harness.model_resource_scope()).expect("exact fixture origin");
    assert_eq!(
        ReclaimResourceScope::from_exact(&exact).expect("exact scope bridge"),
        scope
    );
    let dimensions = [
        scope.account_uuid.to_string(),
        scope.actor_uuid.to_string(),
        scope.session_uuid.to_string(),
        scope.access_space_uuid.to_string(),
    ];
    for position in 0..4 {
        let mut invalid = dimensions.clone();
        invalid[position] = "invalid-or-missing-uuid".to_owned();
        assert!(ReclaimResourceScope::try_from_stored(
            &invalid[0],
            &invalid[1],
            &invalid[2],
            &scope.workspace_id,
            &invalid[3]
        )
        .is_err());
    }
    assert!(ReclaimResourceScope::try_from_stored(
        &dimensions[0],
        &dimensions[1],
        &dimensions[2],
        "   ",
        &dimensions[3]
    )
    .is_err());
    let session_id = Uuid::now_v7();
    let queue_record = seed_queue(&storage, &scope, session_id, "RUNNING").await;
    seed_checkpoint(&storage, &scope, session_id, 0, json!({ "counter": 10 })).await;
    seed_event(&storage, &scope, session_id, 1, json!({ "by": 5 })).await;
    seed_event(&storage, &scope, session_id, 2, json!({ "by": 7 })).await;

    let runner = harness.restart_runner().await;
    let report = runner.run().await.expect("restart-resume run");
    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed.len(), 1);
    assert!(report.sessions_recovery_failed.is_empty());
    assert_eq!(report.sessions_resumed[0].session_id, session_id);
    assert_eq!(report.sessions_resumed[0].events_applied, 2);
    assert_eq!(report.sessions_resumed[0].final_seq, 2);
    assert!(report.orphan_reclaims.is_empty());
    assert_eq!(
        report.fr_events_emitted,
        [
            "FR-EVT-RESTART-RESUME-STARTED",
            "FR-EVT-RESTART-RESUME-SESSION-RESUMED",
            "FR-EVT-RESTART-RESUME-COMPLETED",
        ]
    );

    let persisted: ReportProbe = query_first(
        &storage,
        READ_REPORT,
        exact_record(
            &scope,
            RecordId::new("kernel_restart_resume_report", report.report_id.to_string()),
        ),
    )
    .await
    .expect("exact-scope restart-resume report");
    assert_eq!(persisted.sessions_examined, 1);
    assert_eq!(persisted.schema_version, 2);
    assert_eq!(persisted.sessions_resumed.len(), 1);
    assert!(persisted.sessions_recovery_failed.is_empty());
    assert!(persisted.operator_decision_requests.is_empty());
    assert_eq!(persisted.fr_events_emitted.len(), 3);

    let queue: QueueProbe = query_first(&storage, READ_QUEUE, exact_record(&scope, queue_record))
        .await
        .expect("exact-scope queue row");
    assert_eq!(queue.state, "RETRY_SCHEDULED");
    assert!(queue.claimed_by.is_none());
    assert!(queue.lease_expires_at.is_none());

    let checkpoint = latest_checkpoint(&storage, &scope, session_id)
        .await
        .expect("post-failure checkpoint");
    assert_eq!(checkpoint.compact_state["counter"], 22);
    assert_eq!(checkpoint.last_event_ledger_seq, 2);
    assert_eq!(checkpoint.state_kind, "post_failure");

    let second = runner
        .run()
        .await
        .expect("idempotent second restart-resume run");
    assert_eq!(second.sessions_examined, 0);
    assert!(second.sessions_resumed.is_empty());
    harness.close().await;
}

#[tokio::test]
async fn startup_runner_replays_only_the_exact_scoped_session_when_sequences_interleave() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let storage = harness.storage();
    let scope = harness.resource_scope().clone();
    let session_id = Uuid::now_v7();
    seed_queue(&storage, &scope, session_id, "RUNNING").await;
    seed_checkpoint(&storage, &scope, session_id, 0, json!({ "counter": 10 })).await;
    seed_event(&storage, &scope, session_id, 1, json!({ "by": 5 })).await;

    let unrelated_session_id = Uuid::now_v7();
    seed_event(
        &storage,
        &scope,
        unrelated_session_id,
        2,
        json!({ "by": 999, "unrelated": true }),
    )
    .await;
    seed_event(&storage, &scope, session_id, 3, json!({ "by": 7 })).await;

    let report = harness
        .restart_runner()
        .await
        .run()
        .await
        .expect("restart-resume run");
    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed[0].events_applied, 2);
    assert_eq!(report.sessions_resumed[0].final_seq, 3);
    let checkpoint = latest_checkpoint(&storage, &scope, session_id)
        .await
        .expect("interleaved replay checkpoint");
    assert_eq!(checkpoint.compact_state["counter"], 22);
    assert_eq!(checkpoint.last_event_ledger_seq, 3);
    harness.close().await;
}

#[tokio::test]
async fn startup_runner_fails_closed_without_checkpoint_and_ignores_one_field_mismatch() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let storage = harness.storage();
    let scope = harness.resource_scope().clone();
    let missing_checkpoint_session = Uuid::now_v7();
    let queue_record = seed_queue(&storage, &scope, missing_checkpoint_session, "RUNNING").await;

    let mut foreign_scope = scope.clone();
    foreign_scope.access_space_uuid = Uuid::now_v7();
    let foreign_session = Uuid::now_v7();
    seed_queue(&storage, &foreign_scope, foreign_session, "RUNNING").await;
    seed_checkpoint(
        &storage,
        &foreign_scope,
        foreign_session,
        0,
        json!({ "counter": 99 }),
    )
    .await;

    let report = harness
        .restart_runner()
        .await
        .run()
        .await
        .expect("restart-resume fail-closed run");
    assert_eq!(report.sessions_examined, 1);
    assert!(report.sessions_resumed.is_empty());
    assert_eq!(report.sessions_recovery_failed.len(), 1);
    assert_eq!(report.operator_decision_requests.len(), 1);

    let queue: QueueProbe = query_first(&storage, READ_QUEUE, exact_record(&scope, queue_record))
        .await
        .expect("failed exact-scope queue row");
    assert_eq!(queue.state, "FAILED");
    let foreign_queue: QueueProbe = query_first(
        &storage,
        READ_QUEUE,
        exact_record(
            &foreign_scope,
            RecordId::new("kernel_session_queue", foreign_session.to_string()),
        ),
    )
    .await
    .expect("foreign queue row remains visible only under its exact scope");
    assert_eq!(foreign_queue.state, "RUNNING");
    harness.close().await;
}

/// Operator-requested repeat-safety proof for decision
/// WP012-STARTUP-UNCONFIRMED-COMPLETION-20261126: a pass whose COMPLETED write
/// fails stops startup with the store error after its other effects committed,
/// and the next pass over the same store repeats none of them. The orphan is a
/// real child process, so the external kill runs through the production path.
#[cfg(windows)]
mod repeat_safety {
    use std::{
        os::windows::process::CommandExt,
        path::PathBuf,
        process::{Child, Command, Stdio},
        time::{Duration, Instant},
    };

    use handshake_core::{
        process_ledger::{
            acquire_embedded_runtime_instance_lease,
            restart_resume::{
                AbortReportFault, BoundedRestartResumeOutcome, RestartResumeRuntimeError,
            },
            LedgerEvent, ProcessEngineKind, ProcessStart,
        },
        sandbox::{process_creation_time_100ns, HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID},
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};

    use super::*;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    struct ChildGuard(Option<Child>);

    impl ChildGuard {
        fn pid(&self) -> u32 {
            self.0.as_ref().expect("child present").id()
        }

        fn is_running(&mut self) -> bool {
            self.0
                .as_mut()
                .expect("child present")
                .try_wait()
                .expect("query child state")
                .is_none()
        }

        fn wait_exited(&mut self, timeout: Duration) -> bool {
            let deadline = Instant::now() + timeout;
            while Instant::now() < deadline {
                if !self.is_running() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            false
        }
    }

    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn powershell_path() -> PathBuf {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:/Windows".to_owned());
        let path =
            PathBuf::from(system_root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        assert!(path.is_file(), "required real-process executable is absent");
        path
    }

    #[derive(SurrealValue)]
    struct ScopeBindings {
        owner_account_id: String,
        actor_principal_id: String,
        authenticated_session_id: String,
        access_space_id: String,
        workspace_id: String,
    }

    fn scope_bindings(scope: &ReclaimResourceScope) -> ScopeBindings {
        let (owner, actor, session, access_space, workspace) = scope_values(scope);
        ScopeBindings {
            owner_account_id: owner,
            actor_principal_id: actor,
            authenticated_session_id: session,
            access_space_id: access_space,
            workspace_id: workspace,
        }
    }

    #[derive(SurrealValue)]
    struct NoBindings {}

    const COUNT_EXACT_CHECKPOINTS: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_session_checkpoint
    WHERE owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id);
"#;

    const COUNT_EXACT_REPORTS: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_restart_resume_report
    WHERE owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id);
"#;

    const COUNT_EXACT_COMPLETED_REPORTS: &str = r#"
RETURN array::len(SELECT VALUE id FROM kernel_restart_resume_report
    WHERE owner_account_id = $owner_account_id
        AND actor_principal_id = $actor_principal_id
        AND authenticated_session_id = $authenticated_session_id
        AND access_space_id = $access_space_id
        AND workspace_id = $workspace_id
        AND fr_events_emitted CONTAINS 'FR-EVT-RESTART-RESUME-COMPLETED');
"#;

    const COUNT_MAILBOX_THREADS: &str = "RETURN array::len(SELECT VALUE id FROM role_mailbox_thread);";
    const COUNT_MAILBOX_MESSAGES: &str =
        "RETURN array::len(SELECT VALUE id FROM role_mailbox_message);";
    const COUNT_IDEMPOTENCY_CLAIMS: &str =
        "RETURN array::len(SELECT VALUE id FROM kernel_idempotency_ledger);";

    async fn count<B: SurrealValue + Send + 'static>(
        storage: &SurrealStorage,
        statement: &'static str,
        bindings: B,
    ) -> i64 {
        let counts = storage
            .with_data_operation(|database| {
                Box::pin(async move { database.query_values::<i64, _>(statement, bindings).await })
            })
            .await
            .expect("count restart-resume repeat-safety rows");
        match counts.as_slice() {
            [count] => *count,
            other => panic!("count query returned {other:?}"),
        }
    }

    /// Every durable effect a recovery pass can repeat, read from the store.
    #[derive(Debug, PartialEq)]
    struct DurableEffects {
        queues: Vec<(String, Option<String>, Option<DateTime<Utc>>)>,
        checkpoints: i64,
        mailbox_threads: i64,
        mailbox_messages: i64,
        idempotency_claims: i64,
        lifecycles: i64,
        open_lifecycles: i64,
        process_events: i64,
        orphan: (Option<DateTime<Utc>>, Option<String>, Option<i64>, Value),
        kill_calls: usize,
    }

    async fn durable_effects(
        harness: &ProcessLedgerSurrealHarness,
        queues: &[RecordId],
        orphan_uuid: Uuid,
    ) -> DurableEffects {
        let storage = harness.storage();
        let scope = harness.resource_scope().clone();
        let mut queue_rows = Vec::with_capacity(queues.len());
        for queue in queues {
            let row: QueueProbe = query_first(&storage, READ_QUEUE, exact_record(&scope, queue.clone()))
                .await
                .expect("exact-scope queue row");
            queue_rows.push((row.state, row.claimed_by, row.lease_expires_at));
        }
        let orphan = harness.lifecycle(orphan_uuid).await.expect("orphan lifecycle row");
        DurableEffects {
            queues: queue_rows,
            checkpoints: count(&storage, COUNT_EXACT_CHECKPOINTS, scope_bindings(&scope)).await,
            mailbox_threads: count(&storage, COUNT_MAILBOX_THREADS, NoBindings {}).await,
            mailbox_messages: count(&storage, COUNT_MAILBOX_MESSAGES, NoBindings {}).await,
            idempotency_claims: count(&storage, COUNT_IDEMPOTENCY_CLAIMS, NoBindings {}).await,
            lifecycles: harness.lifecycle_count().await,
            open_lifecycles: harness.open_lifecycle_count().await,
            process_events: harness.process_event_count().await,
            orphan: (orphan.stopped_at, orphan.stop_reason, orphan.exit_code, orphan.metadata),
            kill_calls: harness.restart_kill_calls(),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn startup_pass_with_failed_completed_write_is_repeat_safe_on_the_next_pass() {
        let harness = ProcessLedgerSurrealHarness::open().await;
        let storage = harness.storage();
        let scope = harness.resource_scope().clone();
        let bound = Duration::from_secs(60);

        // Recovery fails for a checkpointed session through the one-shot operator
        // cancel seam: decision message, recovery_failed checkpoint, FAILED queue.
        // Seeded first, so it is the first candidate examined and consumes the cancel.
        let cancelled = Uuid::now_v7();
        let cancelled_queue = seed_queue(&storage, &scope, cancelled, "RUNNING").await;
        seed_checkpoint(&storage, &scope, cancelled, 0, json!({ "counter": 1 })).await;
        // Recovery fails for a session with no checkpoint: decision message, FAILED queue.
        let missing = Uuid::now_v7();
        let missing_queue = seed_queue(&storage, &scope, missing, "RUNNING").await;
        // A resumable session whose prior boot left a real orphan child process.
        let resumed = Uuid::now_v7();
        let resumed_queue = seed_queue(&storage, &scope, resumed, "RUNNING").await;
        seed_checkpoint(&storage, &scope, resumed, 0, json!({ "counter": 10 })).await;
        seed_event(&storage, &scope, resumed, 1, json!({ "by": 5 })).await;
        seed_event(&storage, &scope, resumed, 2, json!({ "by": 7 })).await;

        let executable = powershell_path();
        let child = Command::new(&executable)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 600",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn restart orphan child");
        let mut child = ChildGuard(Some(child));
        let process_uuid = Uuid::now_v7();
        let mut metadata = harness.metadata();
        metadata["effective_executable_sha256"] = Value::String(hex::encode(Sha256::digest(
            std::fs::read(&executable).expect("read real executable"),
        )));
        metadata["os_creation_time_100ns"] = Value::from(
            process_creation_time_100ns(child.pid()).expect("capture child generation identity"),
        );
        metadata["sandbox_handle_id"] = Value::String(process_uuid.to_string());
        let old_lease = acquire_embedded_runtime_instance_lease(
            Uuid::now_v7(),
            harness.restart_host_scope(),
        )
        .expect("acquire prior runtime lease before the orphan START");
        let start = ProcessStart::new(
            ProcessEngineKind::OfficialCliBridge,
            "restart-repeat-safety-orphan",
            None,
        )
        .with_process_uuid(process_uuid)
        .with_os_pid(child.pid())
        .with_parent_session_id(resumed.to_string())
        .with_sandbox_adapter_id(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
        .with_sandbox_internal_id(process_uuid.to_string())
        .with_runtime_owner(old_lease.descriptor().process_runtime_owner())
        .with_metadata_jsonb(metadata);
        harness
            .write_batch(vec![LedgerEvent::Start(start)])
            .await
            .expect("seed orphan START");
        drop(old_lease);
        assert!(child.is_running());

        // First pass: every effect commits, then the COMPLETED write fails and
        // startup stops with the store error.
        let first = harness.restart_runner().await;
        first.arm_operator_cancel_before_resume();
        first.arm_completed_report_fault_for_test(AbortReportFault::PersistenceFailure);
        match first.run_with_bound(bound).await {
            Err(RestartResumeRuntimeError::Store(_)) => {}
            other => panic!("a failed COMPLETED write must stop startup with the store error: {other:?}"),
        }
        drop(first);
        assert!(
            child.wait_exited(Duration::from_secs(10)),
            "the first pass killed the real orphan child"
        );
        let queues = [cancelled_queue, missing_queue, resumed_queue];
        let first_effects = durable_effects(&harness, &queues, process_uuid).await;
        assert_eq!(first_effects.queues[0].0, "FAILED");
        assert_eq!(first_effects.queues[1].0, "FAILED");
        assert_eq!(first_effects.queues[2].0, "RETRY_SCHEDULED");
        assert_eq!(first_effects.kill_calls, 1, "the first pass killed the orphan once");
        assert!(first_effects.orphan.0.is_some(), "the orphan has a durable STOP");
        assert_eq!(first_effects.orphan.1.as_deref(), Some("reclaim"));
        assert_eq!(first_effects.open_lifecycles, 0);
        assert_eq!(first_effects.mailbox_threads, 2, "one decision thread per failed session");
        assert_eq!(first_effects.mailbox_messages, 2, "one decision message per failed session");
        assert_eq!(
            first_effects.checkpoints, 4,
            "two seeded checkpoints, one post_failure and one recovery_failed"
        );
        let resumed_checkpoint = latest_checkpoint(&storage, &scope, resumed)
            .await
            .expect("resumed checkpoint");
        assert_eq!(resumed_checkpoint.state_kind, "post_failure");
        assert_eq!(resumed_checkpoint.compact_state["counter"], 22);
        assert_eq!(resumed_checkpoint.last_event_ledger_seq, 2);
        let cancelled_checkpoint = latest_checkpoint(&storage, &scope, cancelled)
            .await
            .expect("cancelled checkpoint");
        assert_eq!(cancelled_checkpoint.state_kind, "recovery_failed");
        assert_eq!(count(&storage, COUNT_EXACT_REPORTS, scope_bindings(&scope)).await, 0);

        // Second pass over the same store: it completes normally and repeats nothing.
        let second = harness
            .restart_runner()
            .await
            .run_with_bound(bound)
            .await
            .expect("the repeated recovery pass completes normally");
        let report = match second {
            BoundedRestartResumeOutcome::Completed(report) => report,
            other => panic!("the repeated recovery pass must complete: {other:?}"),
        };
        assert_eq!(report.sessions_examined, 0);
        assert!(report.sessions_resumed.is_empty());
        assert!(report.sessions_recovery_failed.is_empty());
        assert!(report.orphan_reclaims.is_empty());
        assert!(report.operator_decision_requests.is_empty());
        assert_eq!(
            report.fr_events_emitted,
            [
                "FR-EVT-RESTART-RESUME-STARTED",
                "FR-EVT-RESTART-RESUME-COMPLETED",
            ]
        );
        assert_eq!(
            durable_effects(&harness, &queues, process_uuid).await,
            first_effects,
            "the repeated pass adds no resume, kill, STOP, mailbox message or checkpoint"
        );
        assert_eq!(
            count(&storage, COUNT_EXACT_COMPLETED_REPORTS, scope_bindings(&scope)).await,
            1,
            "only the repeated pass left a COMPLETED report"
        );
        drop(storage);
        harness.close().await;
    }
}
