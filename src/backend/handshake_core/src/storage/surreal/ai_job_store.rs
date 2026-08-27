use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};

use super::SurrealStorage;
use crate::storage::{
    validate_job_contract, AccessMode, AiJob, AiJobListFilter, JobKind, JobMetrics, JobState,
    NewAiJob, PruneReport, SafetyMode, StorageError, StorageResult,
};

const AI_JOBS: &str = "ai_jobs";

#[derive(SurrealValue)]
struct AiJobRow {
    id: RecordId,
    trace_id: String,
    workflow_run_id: Option<String>,
    job_kind: String,
    status: String,
    status_reason: String,
    error_message: Option<String>,
    protocol_id: String,
    profile_id: String,
    capability_profile_id: String,
    access_mode: String,
    safety_mode: String,
    entity_refs: serde_json::Value,
    planned_operations: serde_json::Value,
    metrics: serde_json::Value,
    job_inputs: Option<serde_json::Value>,
    job_outputs: Option<serde_json::Value>,
    is_pinned: bool,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CreateBindings {
    record: RecordId,
    trace_id: String,
    workflow_run_id: Option<String>,
    workflow_record: Option<RecordId>,
    job_kind: String,
    status: String,
    status_reason: String,
    protocol_id: String,
    profile_id: String,
    capability_profile_id: String,
    access_mode: String,
    safety_mode: String,
    entity_refs: serde_json::Value,
    planned_operations: serde_json::Value,
    metrics: serde_json::Value,
    job_inputs: Option<serde_json::Value>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct IdBinding {
    record: RecordId,
}

#[derive(SurrealValue)]
struct ListBindings {
    status: Option<String>,
    job_kind: Option<String>,
    wsid: Option<String>,
    from: Option<Datetime>,
    to: Option<Datetime>,
}

#[derive(SurrealValue)]
struct UpdateBindings {
    record: RecordId,
    state: String,
    status_reason: String,
    metrics: Option<serde_json::Value>,
    workflow_run_id: Option<String>,
    trace_id: Option<String>,
    error_message: Option<String>,
    job_outputs: Option<serde_json::Value>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct OutputBindings {
    record: RecordId,
    outputs: Option<serde_json::Value>,
    now: Datetime,
}

#[derive(SurrealValue)]
struct PruneBindings {
    cutoff: Datetime,
    limit: i64,
}

#[derive(SurrealValue)]
struct EligibleRow {
    is_pinned: bool,
}

#[derive(SurrealValue)]
struct CountRow {
    count: i64,
}

pub(crate) async fn get(storage: &SurrealStorage, job_id: &str) -> StorageResult<AiJob> {
    let row: Option<AiJobRow> = storage
        .with_data_operation({
            let job_id = job_id.to_owned();
            move |database| Box::pin(async move { database.select_one(AI_JOBS, &job_id).await })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_job)
        .transpose()?
        .ok_or(StorageError::NotFound("ai_job"))
}

pub(crate) async fn list(
    storage: &SurrealStorage,
    filter: AiJobListFilter,
) -> StorageResult<Vec<AiJob>> {
    let bindings = ListBindings {
        status: filter.status.map(|value| value.as_str().to_owned()),
        job_kind: filter.job_kind.map(|value| value.as_str().to_owned()),
        wsid: filter.wsid,
        from: filter.from.map(Datetime::from),
        to: filter.to.map(Datetime::from),
    };
    let rows: Vec<AiJobRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT * FROM ai_jobs WHERE ($status = NONE OR status = $status) \
                         AND ($job_kind = NONE OR job_kind = $job_kind) \
                         AND ($from = NONE OR created_at >= $from) AND ($to = NONE OR created_at <= $to) \
                         AND ($wsid = NONE OR array::len(entity_refs[WHERE entity_kind = 'workspace' \
                              AND entity_id = $wsid]) > 0) \
                         ORDER BY created_at DESC, id DESC LIMIT 200;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter().map(map_job).collect()
}

pub(crate) async fn create(storage: &SurrealStorage, job: NewAiJob) -> StorageResult<AiJob> {
    validate_job_contract(&job.job_kind, &job.profile_id, &job.protocol_id)?;
    let job_id = uuid::Uuid::now_v7();
    let workflow_run_id = matches!(job.job_kind, JobKind::ModelRun).then(uuid::Uuid::now_v7);
    let now = chrono::Utc::now();
    let bindings = CreateBindings {
        record: RecordId::new(AI_JOBS, job_id.to_string()),
        trace_id: job.trace_id.to_string(),
        workflow_run_id: workflow_run_id.map(|id| id.to_string()),
        workflow_record: workflow_run_id.map(|id| RecordId::new("workflow_runs", id.to_string())),
        job_kind: job.job_kind.as_str().to_owned(),
        status: JobState::Queued.as_str().to_owned(),
        status_reason: job.status_reason,
        protocol_id: job.protocol_id,
        profile_id: job.profile_id,
        capability_profile_id: job.capability_profile_id,
        access_mode: job.access_mode.as_str().to_owned(),
        safety_mode: job.safety_mode.as_str().to_owned(),
        entity_refs: serde_json::to_value(job.entity_refs)?,
        planned_operations: serde_json::to_value(job.planned_operations)?,
        metrics: serde_json::to_value(job.metrics)?,
        job_inputs: job.job_inputs,
        now: Datetime::from(now),
    };
    let rows: Vec<AiJobRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         CREATE $record SET trace_id = $trace_id, workflow_run_id = $workflow_run_id, \
                           job_kind = $job_kind, status = $status, status_reason = $status_reason, \
                           protocol_id = $protocol_id, profile_id = $profile_id, \
                           capability_profile_id = $capability_profile_id, access_mode = $access_mode, \
                           safety_mode = $safety_mode, entity_refs = $entity_refs, \
                           planned_operations = $planned_operations, metrics = $metrics, \
                           job_inputs = $job_inputs, created_at = $now, updated_at = $now; \
                         IF $workflow_record != NONE { \
                            CREATE $workflow_record SET job_id = $record, status = $status, \
                              last_heartbeat = $now, created_at = $now, updated_at = $now; \
                         }; \
                         COMMIT TRANSACTION; \
                         SELECT * FROM $record;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    rows.into_iter()
        .next()
        .map(map_job)
        .transpose()?
        .ok_or_else(|| StorageError::Database("AI job create returned no row".to_owned()))
}

pub(crate) async fn update_status(
    storage: &SurrealStorage,
    update: crate::storage::JobStatusUpdate,
) -> StorageResult<AiJob> {
    let bindings = UpdateBindings {
        record: RecordId::new(AI_JOBS, update.job_id.to_string()),
        state: update.state.as_str().to_owned(),
        status_reason: update.status_reason,
        metrics: update.metrics.map(serde_json::to_value).transpose()?,
        workflow_run_id: update.workflow_run_id.map(|id| id.to_string()),
        trace_id: update.trace_id.map(|id| id.to_string()),
        error_message: update.error_message,
        job_outputs: update.job_outputs,
        now: Datetime::from(chrono::Utc::now()),
    };
    let row: Option<AiJobRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "UPDATE $record SET status = $state, status_reason = $status_reason, \
                         metrics = $metrics ?? metrics, workflow_run_id = $workflow_run_id ?? workflow_run_id, \
                         trace_id = $trace_id ?? trace_id, error_message = $error_message ?? error_message, \
                         job_outputs = $job_outputs ?? job_outputs, updated_at = $now RETURN AFTER;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    row.map(map_job)
        .transpose()?
        .ok_or(StorageError::NotFound("ai_job"))
}

pub(crate) async fn set_outputs(
    storage: &SurrealStorage,
    job_id: &str,
    outputs: Option<serde_json::Value>,
) -> StorageResult<()> {
    let count = storage
        .with_data_operation({
            let job_id = job_id.to_owned();
            move |database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "UPDATE $record SET job_outputs = $outputs, updated_at = $now RETURN AFTER;",
                            OutputBindings {
                                record: RecordId::new(AI_JOBS, job_id),
                                outputs,
                                now: Datetime::from(chrono::Utc::now()),
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    if count == 0 {
        Err(StorageError::NotFound("ai_job"))
    } else {
        Ok(())
    }
}

pub(crate) async fn prune(
    storage: &SurrealStorage,
    cutoff: chrono::DateTime<chrono::Utc>,
    min_versions: u32,
    dry_run: bool,
) -> StorageResult<PruneReport> {
    let cutoff_value = Datetime::from(cutoff);
    let eligible: Vec<EligibleRow> = storage
        .with_data_operation({
            let cutoff = cutoff_value.clone();
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT is_pinned FROM ai_jobs WHERE status IN ['completed', 'failed'] \
                             AND created_at < $cutoff;",
                            PruneBindings { cutoff, limit: 0 },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;
    let pinned = eligible.iter().filter(|row| row.is_pinned).count() as u32;
    let deletable = (eligible.len() as u32).saturating_sub(pinned);
    let total_non_pinned = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first::<CountRow, _>(
                        "SELECT count() AS count FROM ai_jobs WHERE is_pinned = false \
                         AND status IN ['completed', 'failed'] GROUP ALL;",
                        IdBinding {
                            record: RecordId::new(AI_JOBS, "unused"),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?
        .map(|row| row.count.max(0) as u32)
        .unwrap_or(0);
    let actual = deletable.min(total_non_pinned.saturating_sub(min_versions));
    let mut report = PruneReport::new();
    report.items_scanned = eligible.len() as u32;
    report.items_spared_pinned = pinned;
    report.items_pruned = actual;
    report.items_spared_window = deletable.saturating_sub(actual);
    if dry_run || actual == 0 {
        return Ok(report);
    }
    let deleted: Vec<AiJobRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "DELETE ai_jobs WHERE id IN (SELECT VALUE id FROM ai_jobs \
                           WHERE status IN ['completed', 'failed'] AND created_at < $cutoff \
                           AND is_pinned = false ORDER BY created_at ASC, id ASC LIMIT $limit) \
                         RETURN BEFORE;",
                        PruneBindings {
                            cutoff: cutoff_value,
                            limit: i64::from(actual),
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    report.items_pruned = deleted.len() as u32;
    report.items_spared_window = deletable.saturating_sub(report.items_pruned);
    Ok(report)
}

fn key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Database(
            "AI job record has a non-string id".to_owned(),
        )),
    }
}

fn map_job(row: AiJobRow) -> StorageResult<AiJob> {
    Ok(AiJob {
        job_id: uuid::Uuid::parse_str(&key(row.id)?)
            .map_err(|_| StorageError::Validation("invalid ai job id"))?,
        trace_id: uuid::Uuid::parse_str(&row.trace_id)
            .map_err(|_| StorageError::Validation("invalid ai job trace id"))?,
        workflow_run_id: row
            .workflow_run_id
            .map(|id| uuid::Uuid::parse_str(&id))
            .transpose()
            .map_err(|_| StorageError::Validation("invalid AI job workflow run id"))?,
        job_kind: row.job_kind.parse::<JobKind>()?,
        state: JobState::try_from(row.status.as_str())?,
        error_message: row.error_message,
        protocol_id: row.protocol_id,
        profile_id: row.profile_id,
        capability_profile_id: row.capability_profile_id,
        access_mode: AccessMode::try_from(row.access_mode.as_str())?,
        safety_mode: SafetyMode::try_from(row.safety_mode.as_str())?,
        entity_refs: serde_json::from_value(row.entity_refs)?,
        planned_operations: serde_json::from_value(row.planned_operations)?,
        metrics: serde_json::from_value::<JobMetrics>(row.metrics)?,
        status_reason: row.status_reason,
        job_inputs: row.job_inputs,
        job_outputs: row.job_outputs,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}
