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
SELECT compact_state, last_event_ledger_seq, state_kind
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
            Box::pin(async move { database.execute_returning(statement, bindings).await })
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
            Box::pin(async move { database.query_first::<T, _>(statement, bindings).await })
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
                database
                    .query_first::<CheckpointProbe, _>(READ_LATEST_CHECKPOINT, bindings)
                    .await
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
    let session_id = Uuid::now_v7();
    let queue_record = seed_queue(&storage, &scope, session_id, "RUNNING").await;
    seed_checkpoint(&storage, &scope, session_id, 0, json!({ "counter": 10 })).await;
    seed_event(&storage, &scope, session_id, 1, json!({ "by": 5 })).await;
    seed_event(&storage, &scope, session_id, 2, json!({ "by": 7 })).await;

    let runner = SurrealRestartResumeRunner::new(storage.clone(), scope.clone());
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

    let report = SurrealRestartResumeRunner::new(storage.clone(), scope.clone())
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

    let report = SurrealRestartResumeRunner::new(storage.clone(), scope.clone())
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
