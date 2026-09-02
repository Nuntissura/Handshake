use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
use crate::storage::{
    JobState, NewNodeExecution, StorageError, StorageResult, WorkflowNodeExecution, WorkflowRun,
};

const WORKFLOW_RUNS: &str = "workflow_runs";
const NODE_EXECUTIONS: &str = "workflow_node_executions";

#[derive(SurrealValue)]
struct WorkflowRow {
    id: RecordId,
    job_id: RecordId,
    status: String,
    last_heartbeat: Datetime,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct WorkflowCreateBindings {
    record: RecordId,
    job: RecordId,
    status: String,
    heartbeat: Datetime,
    now: Datetime,
}

#[derive(SurrealValue)]
struct WorkflowUpdateBindings {
    record: RecordId,
    status: String,
    error_message: Option<String>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct WorkflowHeartbeatBindings {
    record: RecordId,
    at: Datetime,
}

#[derive(SurrealValue)]
struct CutoffBinding {
    cutoff: Datetime,
}

#[derive(SurrealValue)]
struct NodeRow {
    id: RecordId,
    workflow_run_id: RecordId,
    node_id: String,
    node_type: String,
    status: String,
    sequence: i64,
    input_payload: Option<serde_json::Value>,
    output_payload: Option<serde_json::Value>,
    error_message: Option<String>,
    started_at: Datetime,
    finished_at: Option<Datetime>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct NodeCreateBindings {
    record: RecordId,
    run: RecordId,
    node_id: String,
    node_type: String,
    status: String,
    sequence: i64,
    input_payload: Option<serde_json::Value>,
    started_at: Datetime,
    now: Datetime,
}

#[derive(SurrealValue)]
struct NodeUpdateBindings {
    record: RecordId,
    status: String,
    output: Option<serde_json::Value>,
    error_message: Option<String>,
    terminal: bool,
    now: Datetime,
}

#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

pub(crate) async fn create_workflow_run(
    storage: &SurrealStorage,
    job_id: uuid::Uuid,
    status: JobState,
    last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
) -> StorageResult<WorkflowRun> {
    let now = chrono::Utc::now();
    let bindings = WorkflowCreateBindings {
        record: RecordId::new(WORKFLOW_RUNS, uuid::Uuid::now_v7().to_string()),
        job: RecordId::new("ai_jobs", job_id.to_string()),
        status: status.as_str().to_owned(),
        heartbeat: Datetime::from(last_heartbeat.unwrap_or(now)),
        now: Datetime::from(now),
    };
    let row: Option<WorkflowRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "CREATE $record SET job_id = $job, status = $status, \
                         last_heartbeat = $heartbeat, created_at = $now, updated_at = $now RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_workflow_error)?;
    row.map(map_workflow)
        .transpose()?
        .ok_or_else(|| StorageError::Database("workflow run create returned no row".to_owned()))
}

pub(crate) async fn update_workflow_run_status(
    storage: &SurrealStorage,
    run_id: uuid::Uuid,
    status: JobState,
    error_message: Option<String>,
) -> StorageResult<WorkflowRun> {
    let bindings = WorkflowUpdateBindings {
        record: RecordId::new(WORKFLOW_RUNS, run_id.to_string()),
        status: status.as_str().to_owned(),
        error_message,
        now: Datetime::from(chrono::Utc::now()),
    };
    let rows: Vec<WorkflowRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         LET $updated = UPDATE $record SET status = $status, updated_at = $now RETURN AFTER; \
                         IF array::len($updated) = 0 { THROW 'HSK-WORKFLOW-RUN-MISSING'; }; \
                         IF $error_message != NONE { \
                            UPDATE ai_jobs SET error_message = $error_message, updated_at = $now \
                              WHERE id = $updated[0].job_id; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM $record;",
                        bindings,
                        5,
                    )
                    .await
            })
        })
        .await
        .map_err(map_workflow_error)?;
    rows.into_iter()
        .next()
        .map(map_workflow)
        .transpose()?
        .ok_or(StorageError::NotFound("workflow_run"))
}

pub(crate) async fn heartbeat_workflow(
    storage: &SurrealStorage,
    run_id: uuid::Uuid,
    at: chrono::DateTime<chrono::Utc>,
) -> StorageResult<()> {
    let count = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        "UPDATE $record SET last_heartbeat = $at, updated_at = $at RETURN AFTER;",
                        WorkflowHeartbeatBindings {
                            record: RecordId::new(WORKFLOW_RUNS, run_id.to_string()),
                            at: Datetime::from(at),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    if count == 0 {
        Err(StorageError::NotFound("workflow_run"))
    } else {
        Ok(())
    }
}

pub(crate) async fn create_node_execution(
    storage: &SurrealStorage,
    exec: NewNodeExecution,
) -> StorageResult<WorkflowNodeExecution> {
    let now = chrono::Utc::now();
    let bindings = NodeCreateBindings {
        record: RecordId::new(NODE_EXECUTIONS, uuid::Uuid::now_v7().to_string()),
        run: RecordId::new(WORKFLOW_RUNS, exec.workflow_run_id.to_string()),
        node_id: exec.node_id,
        node_type: exec.node_type,
        status: exec.status.as_str().to_owned(),
        sequence: exec.sequence,
        input_payload: exec.input_payload,
        started_at: Datetime::from(exec.started_at),
        now: Datetime::from(now),
    };
    let row: Option<NodeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "CREATE $record SET workflow_run_id = $run, node_id = $node_id, node_type = $node_type, \
                         status = $status, sequence = $sequence, input_payload = $input_payload, \
                         started_at = $started_at, created_at = $now, updated_at = $now RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_workflow_error)?;
    row.map(map_node)
        .transpose()?
        .ok_or_else(|| StorageError::Database("workflow node create returned no row".to_owned()))
}

pub(crate) async fn update_node_execution_status(
    storage: &SurrealStorage,
    exec_id: uuid::Uuid,
    status: JobState,
    output: Option<serde_json::Value>,
    error_message: Option<String>,
) -> StorageResult<WorkflowNodeExecution> {
    let terminal = matches!(
        status,
        JobState::Completed
            | JobState::CompletedWithIssues
            | JobState::Failed
            | JobState::Cancelled
            | JobState::Stalled
            | JobState::Poisoned
    );
    let bindings = NodeUpdateBindings {
        record: RecordId::new(NODE_EXECUTIONS, exec_id.to_string()),
        status: status.as_str().to_owned(),
        output,
        error_message,
        terminal,
        now: Datetime::from(chrono::Utc::now()),
    };
    let row: Option<NodeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE $record SET status = $status, output_payload = $output ?? output_payload, \
                         error_message = $error_message ?? error_message, \
                         finished_at = IF $terminal { $now } ELSE { finished_at }, \
                         updated_at = $now RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_node)
        .transpose()?
        .ok_or(StorageError::NotFound("workflow_node_execution"))
}

pub(crate) async fn list_node_executions(
    storage: &SurrealStorage,
    run_id: uuid::Uuid,
) -> StorageResult<Vec<WorkflowNodeExecution>> {
    let rows: Vec<NodeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM workflow_node_executions WHERE workflow_run_id = $record \
                         ORDER BY sequence ASC, id ASC;",
                        RecordBinding {
                            record: RecordId::new(WORKFLOW_RUNS, run_id.to_string()),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_node).collect()
}

pub(crate) async fn find_stalled_workflows(
    storage: &SurrealStorage,
    threshold_secs: u64,
) -> StorageResult<Vec<WorkflowRun>> {
    let threshold = i64::try_from(threshold_secs)
        .map_err(|_| StorageError::Validation("workflow threshold is too large"))?;
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(threshold);
    let rows: Vec<WorkflowRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM workflow_runs WHERE status = 'running' \
                         AND last_heartbeat < $cutoff ORDER BY last_heartbeat ASC, id ASC;",
                        CutoffBinding {
                            cutoff: Datetime::from(cutoff),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_workflow).collect()
}

fn key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "workflow record has a non-string id".to_owned(),
        )),
    }
}

fn uuid_key(record: RecordId, label: &'static str) -> StorageResult<uuid::Uuid> {
    uuid::Uuid::parse_str(&key(record)?).map_err(|_| StorageError::Validation(label))
}

fn map_workflow(row: WorkflowRow) -> StorageResult<WorkflowRun> {
    Ok(WorkflowRun {
        id: uuid_key(row.id, "invalid workflow run id")?,
        job_id: uuid_key(row.job_id, "invalid workflow job id")?,
        status: JobState::try_from(row.status.as_str())?,
        last_heartbeat: row.last_heartbeat.into_inner(),
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn map_node(row: NodeRow) -> StorageResult<WorkflowNodeExecution> {
    Ok(WorkflowNodeExecution {
        id: uuid_key(row.id, "invalid workflow node id")?,
        workflow_run_id: uuid_key(row.workflow_run_id, "invalid workflow run id")?,
        node_id: row.node_id,
        node_type: row.node_type,
        status: JobState::try_from(row.status.as_str())?,
        sequence: row.sequence,
        input_payload: row.input_payload,
        output_payload: row.output_payload,
        error_message: row.error_message,
        started_at: row.started_at.into_inner(),
        finished_at: row.finished_at.map(|value| value.into_inner()),
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn map_workflow_error(error: super::SurrealStorageError) -> StorageError {
    let message = error.to_string();
    if message.contains("HSK-WORKFLOW-RUN-MISSING") {
        StorageError::NotFound("workflow_run")
    } else if message.contains("record::exists") {
        StorageError::NotFound("workflow_parent")
    } else {
        StorageError::from(error)
    }
}

