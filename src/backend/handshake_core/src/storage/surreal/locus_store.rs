use std::collections::{HashMap, HashSet, VecDeque};

use chrono::Utc;
use serde_json::{json, Value as JsonValue};
use sha2::{Digest, Sha256};
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;

use super::SurrealStorage;
use crate::storage::{StorageError, StorageResult};
use crate::workflows::locus::types::{
    executor_eligibility_policy_ids_for_family, governed_action_ids_for_family,
    queue_automation_rule_ids_for_reason, resolve_queue_reason_with_mailbox_context,
    transition_rule_ids_for_family, DependencyType, LocusAddDependencyParams,
    LocusBindSessionParams, LocusCloseWpParams, LocusCompleteMtParams, LocusCreateWpParams,
    LocusDeleteWpParams, LocusGateKind, LocusGateWpParams, LocusGetMtProgressParams,
    LocusGetWpStatusParams, LocusOperation, LocusQueryReadyParams, LocusRecordIterationParams,
    LocusRegisterMtsParams, LocusRemoveDependencyParams, LocusStartMtParams,
    LocusSyncTaskBoardParams, LocusUnbindSessionParams, LocusUpdateWpParams,
    MicroTaskIterationOutcome, MicroTaskStatus, RoutingPolicy, TaskBoardStatus, TrackedMicroTask,
    TrackedMicroTaskArtifactV1, WorkPacketPhase, WorkPacketStatus, WorkflowQueueReasonCode,
    WorkflowStateFamily,
};

const WORK_PACKETS: &str = "work_packets";
const MICRO_TASKS: &str = "micro_tasks";
const MT_ITERATIONS: &str = "mt_iterations";
const DEPENDENCIES: &str = "dependencies";

static DEPENDENCY_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Clone, SurrealValue)]
struct WorkPacketRow {
    id: RecordId,
    wp_id: String,
    version: i64,
    title: String,
    description: Option<String>,
    status: String,
    priority: i64,
    phase: Option<String>,
    routing: Option<String>,
    task_packet_path: Option<String>,
    task_board_status: String,
    assignee: Option<String>,
    reporter: String,
    created_at: String,
    updated_at: String,
    vector_clock: String,
    metadata: String,
}

#[derive(SurrealValue)]
struct CreateWorkPacketBindings {
    record: RecordId,
    wp_id: String,
    version: i64,
    title: String,
    description: Option<String>,
    status: String,
    priority: i64,
    phase: String,
    routing: String,
    task_packet_path: Option<String>,
    task_board_status: String,
    assignee: Option<String>,
    reporter: String,
    created_at: String,
    updated_at: String,
    vector_clock: String,
    metadata: String,
}

#[derive(SurrealValue)]
struct ReplaceWorkPacketBindings {
    record: RecordId,
    expected_version: i64,
    title: String,
    description: Option<String>,
    status: String,
    priority: i64,
    phase: Option<String>,
    routing: Option<String>,
    task_packet_path: Option<String>,
    task_board_status: String,
    assignee: Option<String>,
    updated_at: String,
    metadata: String,
}

#[derive(SurrealValue)]
struct TaskBoardUpdateBindings {
    record: RecordId,
    status: String,
    task_board_status: String,
    updated_at: String,
    metadata: String,
}

#[derive(SurrealValue)]
struct RegisterMtWrite {
    record: RecordId,
    mt_id: String,
    name: String,
    status: String,
    current_iteration: i64,
    escalation_level: i64,
    metadata: String,
}

#[derive(SurrealValue)]
struct RegisterMtsBindings {
    wp: RecordId,
    rows: Vec<RegisterMtWrite>,
}

#[derive(SurrealValue)]
struct MicroTaskRow {
    id: RecordId,
    mt_id: String,
    wp_id: RecordId,
    name: String,
    status: String,
    current_iteration: Option<i64>,
    escalation_level: Option<i64>,
    metadata: String,
}

#[derive(SurrealValue)]
struct MicroTaskBinding {
    record: RecordId,
    wp: RecordId,
}

#[derive(SurrealValue)]
struct PersistMicroTaskBindings {
    record: RecordId,
    wp: RecordId,
    expected_metadata: String,
    name: String,
    status: String,
    current_iteration: i64,
    escalation_level: i64,
    metadata: String,
}

#[derive(SurrealValue)]
struct IterationWrite {
    record: RecordId,
    iteration_id: String,
    mt_id: RecordId,
    iteration: i64,
    escalation_level: i64,
    model_id: String,
    lora_id: Option<String>,
    outcome: String,
    validation_passed: Option<bool>,
    duration_ms: i64,
}

#[derive(SurrealValue)]
struct RecordIterationBindings {
    task: PersistMicroTaskBindings,
    iteration: IterationWrite,
}

#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

#[derive(Clone, SurrealValue)]
struct DependencyRow {
    id: RecordId,
    dependency_id: String,
    from_wp_id: RecordId,
    to_wp_id: RecordId,
    dependency_type: String,
    created_at: String,
    vector_clock: String,
}

#[derive(SurrealValue)]
struct DependencyWrite {
    record: RecordId,
    dependency_id: String,
    from_wp_id: RecordId,
    to_wp_id: RecordId,
    dependency_type: String,
    created_at: String,
    vector_clock: String,
}

#[derive(SurrealValue)]
struct EmptyBindings {}

#[derive(SurrealValue)]
struct ReadyBindings {
    limit: i64,
}

pub(crate) async fn execute_locus_operation(
    storage: &SurrealStorage,
    op: LocusOperation,
) -> StorageResult<JsonValue> {
    match op {
        LocusOperation::CreateWp(params) => create_wp(storage, params).await,
        LocusOperation::UpdateWp(params) => update_wp(storage, params).await,
        LocusOperation::GateWp(params) => gate_wp(storage, params).await,
        LocusOperation::CloseWp(params) => close_wp(storage, params).await,
        LocusOperation::DeleteWp(params) => delete_wp(storage, params).await,
        LocusOperation::RegisterMts(params) => register_mts(storage, params).await,
        LocusOperation::StartMt(params) => start_mt(storage, params).await,
        LocusOperation::BindSession(params) => bind_session(storage, params).await,
        LocusOperation::UnbindSession(params) => unbind_session(storage, params).await,
        LocusOperation::RecordIteration(params) => record_iteration(storage, params).await,
        LocusOperation::CompleteMt(params) => complete_mt(storage, params).await,
        LocusOperation::GetMtProgress(params) => get_mt_progress(storage, params).await,
        LocusOperation::AddDependency(params) => add_dependency(storage, params).await,
        LocusOperation::RemoveDependency(params) => remove_dependency(storage, params).await,
        LocusOperation::QueryReady(params) => query_ready(storage, params).await,
        LocusOperation::GetWpStatus(params) => get_wp_status(storage, params).await,
        LocusOperation::SyncTaskBoard(params) => sync_task_board_snapshot(storage, params).await,
    }
}

pub(crate) async fn locus_task_board_update_work_packet(
    storage: &SurrealStorage,
    status: &str,
    task_board_status: &str,
    updated_at: &str,
    metadata: &str,
    wp_id: &str,
) -> StorageResult<()> {
    serde_json::from_str::<JsonValue>(metadata)?;
    let status = canonical_work_packet_status_for_storage(status)?;
    let task_board_status = canonical_task_board_status_for_storage(task_board_status)?;
    let bindings = TaskBoardUpdateBindings {
        record: RecordId::new(WORK_PACKETS, wp_id.to_owned()),
        status: status.to_owned(),
        task_board_status: task_board_status.to_owned(),
        updated_at: updated_at.to_owned(),
        metadata: metadata.to_owned(),
    };
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         LET $current = SELECT * FROM ONLY $record; \
                         IF $current = NONE { THROW 'HSK-LOCUS-WP-MISSING'; }; \
                         UPDATE $record SET version += 1, status = $status, \
                           task_board_status = $task_board_status, updated_at = $updated_at, \
                           metadata = $metadata RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await;
    match result {
        Ok(rows) if rows.len() == 1 => Ok(()),
        Ok(_) => Err(StorageError::Database(
            "Locus task-board update returned an unexpected row count".to_owned(),
        )),
        Err(error) => Err(map_locus_error(error)),
    }
}

async fn create_wp(
    storage: &SurrealStorage,
    params: LocusCreateWpParams,
) -> StorageResult<JsonValue> {
    if params.priority > 4 {
        return Err(StorageError::Validation("priority must be between 0 and 4"));
    }
    let now = now_rfc3339();
    let metadata = json!({
        "labels": params.labels.unwrap_or_default(),
        "spec_session_id": params.spec_session_id,
        "notes": [],
        "gates": {
            "pre_work": { "status": "pending" },
            "post_work": { "status": "pending" }
        },
        "started_at": null,
        "completed_at": null,
        "due_at": null,
        "tombstone": null,
        "work_packet_type": serde_json::to_value(params.kind)?,
    });
    let bindings = CreateWorkPacketBindings {
        record: RecordId::new(WORK_PACKETS, params.wp_id.clone()),
        wp_id: params.wp_id.clone(),
        version: 1,
        title: params.title,
        description: Some(params.description),
        status: work_packet_status_str(WorkPacketStatus::Unknown).to_owned(),
        priority: i64::from(params.priority),
        phase: phase_str(params.phase).to_owned(),
        routing: routing_str(params.routing).to_owned(),
        task_packet_path: params.task_packet_path,
        task_board_status: task_board_status_str(TaskBoardStatus::Unknown).to_owned(),
        assignee: params.assignee,
        reporter: params.reporter,
        created_at: now.clone(),
        updated_at: now.clone(),
        vector_clock: serde_json::to_string(&json!({"local": 1}))?,
        metadata: serde_json::to_string(&metadata)?,
    };
    run_create_wp(storage, bindings).await?;
    Ok(json!({
        "wp_id": params.wp_id,
        "version": 1,
        "status": "stub",
        "task_board_status": "STUB",
        "created_at": now,
        "updated_at": now,
    }))
}

async fn run_create_wp(
    storage: &SurrealStorage,
    bindings: CreateWorkPacketBindings,
) -> StorageResult<()> {
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $record)[0] != NONE { THROW 'HSK-LOCUS-WP-DUPLICATE'; }; \
                         CREATE $record CONTENT { wp_id: $wp_id, version: $version, title: $title, \
                           description: $description, status: $status, priority: $priority, phase: $phase, \
                           routing: $routing, task_packet_path: $task_packet_path, \
                           task_board_status: $task_board_status, assignee: $assignee, reporter: $reporter, \
                           created_at: $created_at, updated_at: $updated_at, vector_clock: $vector_clock, \
                           metadata: $metadata } RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        2,
                    )
                    .await
            })
        })
        .await;
    match result {
        Ok(rows) if rows.len() == 1 => Ok(()),
        Ok(_) => Err(StorageError::Database(
            "Locus work-packet create returned an unexpected row count".to_owned(),
        )),
        Err(error) => Err(map_locus_error(error)),
    }
}

async fn update_wp(
    storage: &SurrealStorage,
    params: LocusUpdateWpParams,
) -> StorageResult<JsonValue> {
    let now = now_rfc3339();
    if params.updates.is_empty() {
        ensure_wp_exists(storage, &params.wp_id).await?;
        return Ok(json!({ "wp_id": params.wp_id, "updated_at": now, "no_op": true }));
    }
    let mut row = load_wp(storage, &params.wp_id).await?;
    for (key, value) in params.updates {
        match key.as_str() {
            "title" => row.title = required_string(value)?,
            "description" => row.description = optional_string(value)?,
            "priority" => {
                let priority = value
                    .as_i64()
                    .ok_or(StorageError::Validation("priority must be an integer"))?;
                if !(0..=4).contains(&priority) {
                    return Err(StorageError::Validation("priority must be between 0 and 4"));
                }
                row.priority = priority;
            }
            "status" => {
                let value = required_string(value)?;
                row.status = canonical_work_packet_status_for_storage(&value)?.to_owned();
            }
            "assignee" => row.assignee = optional_string(value)?,
            "governance.phase" | "phase" => row.phase = optional_string(value)?,
            "governance.routing" | "routing" => row.routing = optional_string(value)?,
            "governance.task_packet_path" | "task_packet_path" => {
                row.task_packet_path = optional_string(value)?
            }
            "governance.task_board_status" | "task_board_status" => {
                let value = required_string(value)?;
                row.task_board_status = canonical_task_board_status_for_storage(&value)?.to_owned();
            }
            "" => return Err(StorageError::Validation("empty update key")),
            _ => return Err(StorageError::Validation("unsupported update key")),
        }
    }
    row.updated_at = now.clone();
    replace_wp(storage, row).await?;
    Ok(json!({ "wp_id": params.wp_id, "updated_at": now }))
}

async fn gate_wp(storage: &SurrealStorage, params: LocusGateWpParams) -> StorageResult<JsonValue> {
    let mut row = load_wp(storage, &params.wp_id).await?;
    let mut metadata: JsonValue = serde_json::from_str(&row.metadata)?;
    let gate_key = match params.gate {
        LocusGateKind::PreWork => "pre_work",
        LocusGateKind::PostWork => "post_work",
    };
    metadata
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata must be an object"))?
        .entry("gates".to_owned())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata.gates must be an object"))?
        .insert(gate_key.to_owned(), serde_json::to_value(params.result)?);
    let now = now_rfc3339();
    row.updated_at = now.clone();
    row.metadata = serde_json::to_string(&metadata)?;
    replace_wp(storage, row).await?;
    Ok(json!({ "wp_id": params.wp_id, "gate": gate_key, "updated_at": now }))
}

async fn close_wp(
    storage: &SurrealStorage,
    params: LocusCloseWpParams,
) -> StorageResult<JsonValue> {
    let mut row = load_wp(storage, &params.wp_id).await?;
    let now = now_rfc3339();
    row.status = work_packet_status_str(WorkPacketStatus::Done).to_owned();
    row.task_board_status = task_board_status_str(TaskBoardStatus::Done).to_owned();
    row.updated_at = now.clone();
    replace_wp(storage, row).await?;
    Ok(json!({ "wp_id": params.wp_id, "status": "done", "updated_at": now }))
}

async fn delete_wp(
    storage: &SurrealStorage,
    params: LocusDeleteWpParams,
) -> StorageResult<JsonValue> {
    let mut row = load_wp(storage, &params.wp_id).await?;
    let now = now_rfc3339();
    let mut metadata: JsonValue = serde_json::from_str(&row.metadata)?;
    metadata
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata must be an object"))?
        .insert("tombstone".to_owned(), json!({ "deleted_at": now }));
    row.status = work_packet_status_str(WorkPacketStatus::Cancelled).to_owned();
    row.task_board_status = task_board_status_str(TaskBoardStatus::Cancelled).to_owned();
    row.updated_at = now.clone();
    row.metadata = serde_json::to_string(&metadata)?;
    replace_wp(storage, row).await?;
    Ok(json!({ "wp_id": params.wp_id, "status": "cancelled", "updated_at": now }))
}

async fn load_wp(storage: &SurrealStorage, wp_id: &str) -> StorageResult<WorkPacketRow> {
    let record = RecordId::new(WORK_PACKETS, wp_id.to_owned());
    let row: Option<WorkPacketRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first("SELECT * FROM $record;", RecordBinding { record })
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let row = row.ok_or(StorageError::NotFound("work_packet"))?;
    ensure_record_key(&row.id, WORK_PACKETS, &row.wp_id)?;
    Ok(row)
}

async fn ensure_wp_exists(storage: &SurrealStorage, wp_id: &str) -> StorageResult<()> {
    load_wp(storage, wp_id).await.map(|_| ())
}

async fn replace_wp(storage: &SurrealStorage, row: WorkPacketRow) -> StorageResult<()> {
    let bindings = ReplaceWorkPacketBindings {
        record: row.id,
        expected_version: row.version,
        title: row.title,
        description: row.description,
        status: row.status,
        priority: row.priority,
        phase: row.phase,
        routing: row.routing,
        task_packet_path: row.task_packet_path,
        task_board_status: row.task_board_status,
        assignee: row.assignee,
        updated_at: row.updated_at,
        metadata: row.metadata,
    };
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         LET $current = SELECT * FROM ONLY $record; \
                         IF $current = NONE { THROW 'HSK-LOCUS-WP-MISSING'; }; \
                         IF $current.version != $expected_version { THROW 'HSK-LOCUS-WP-STALE'; }; \
                         UPDATE $record SET version += 1, title = $title, description = $description, \
                           status = $status, priority = $priority, phase = $phase, routing = $routing, \
                           task_packet_path = $task_packet_path, task_board_status = $task_board_status, \
                           assignee = $assignee, updated_at = $updated_at, metadata = $metadata RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await;
    match result {
        Ok(rows) if rows.len() == 1 => Ok(()),
        Ok(_) => Err(StorageError::Database(
            "Locus work-packet update returned an unexpected row count".to_owned(),
        )),
        Err(error) => Err(map_locus_error(error)),
    }
}

async fn register_mts(
    storage: &SurrealStorage,
    params: LocusRegisterMtsParams,
) -> StorageResult<JsonValue> {
    let mut rows = Vec::with_capacity(params.micro_tasks.len());
    for mut task in params.micro_tasks {
        if task.wp_id != params.wp_id {
            return Err(StorageError::Validation("micro task wp_id mismatch"));
        }
        dedupe_session_ids(&mut task.active_session_ids);
        rows.push(RegisterMtWrite {
            record: RecordId::new(MICRO_TASKS, task.mt_id.clone()),
            mt_id: task.mt_id.clone(),
            name: task.name.clone(),
            status: micro_task_status_str(task.status).to_owned(),
            current_iteration: i64::from(task.current_iteration),
            escalation_level: i64::from(task.escalation.current_level),
            metadata: serde_json::to_string(&task)?,
        });
    }
    let bindings = RegisterMtsBindings {
        wp: RecordId::new(WORK_PACKETS, params.wp_id.clone()),
        rows,
    };
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         IF record::exists($wp) = false { THROW 'HSK-LOCUS-WP-MISSING'; }; \
                         FOR $row IN $rows { \
                           LET $existing_wp = (SELECT VALUE wp_id FROM $row.record)[0]; \
                           LET $existing_metadata = (SELECT VALUE metadata FROM $row.record)[0]; \
                           IF $existing_wp != NONE AND $existing_wp != $wp { THROW 'HSK-LOCUS-MT-OTHER-WP'; }; \
                           IF $existing_wp != NONE AND $existing_metadata != $row.metadata \
                             { THROW 'HSK-LOCUS-MT-DIVERGENT'; }; \
                           IF $existing_wp = NONE { \
                             CREATE $row.record CONTENT { mt_id: $row.mt_id, wp_id: $wp, name: $row.name, \
                               status: $row.status, current_iteration: $row.current_iteration, \
                               escalation_level: $row.escalation_level, metadata: $row.metadata }; \
                           }; \
                         }; \
                         COMMIT TRANSACTION;",
                        bindings,
                    )
                    .await
            })
        })
        .await;
    if let Err(error) = result {
        return Err(map_locus_error(error));
    }
    Ok(json!({ "wp_id": params.wp_id, "registered": true }))
}

async fn start_mt(
    storage: &SurrealStorage,
    params: LocusStartMtParams,
) -> StorageResult<JsonValue> {
    let (mut task, expected) = load_tracked_mt(storage, &params.wp_id, &params.mt_id).await?;
    task.status = MicroTaskStatus::InProgress;
    task.escalation.current_level = params.escalation_level;
    if task.started_at.is_none() {
        task.started_at = Some(Utc::now());
    }
    persist_tracked_mt(storage, &task, expected).await?;
    Ok(json!({
        "wp_id": params.wp_id, "mt_id": params.mt_id, "status": "in_progress",
        "model_id": params.model_id, "lora_id": params.lora_id,
        "escalation_level": params.escalation_level, "updated_at": now_rfc3339(),
    }))
}

async fn bind_session(
    storage: &SurrealStorage,
    params: LocusBindSessionParams,
) -> StorageResult<JsonValue> {
    let session_id = params.session_id.trim().to_owned();
    if session_id.is_empty() {
        return Err(StorageError::Validation("session_id"));
    }
    let (mut task, expected) = load_tracked_mt(storage, &params.wp_id, &params.mt_id).await?;
    task.status = MicroTaskStatus::InProgress;
    task.escalation.current_level = params.escalation_level;
    task.active_session_ids.push(session_id.clone());
    dedupe_session_ids(&mut task.active_session_ids);
    let active_session_ids = task.active_session_ids.clone();
    persist_tracked_mt(storage, &task, expected).await?;
    Ok(json!({
        "wp_id": params.wp_id, "mt_id": params.mt_id, "session_id": session_id,
        "active_session_ids": active_session_ids,
    }))
}

async fn unbind_session(
    storage: &SurrealStorage,
    params: LocusUnbindSessionParams,
) -> StorageResult<JsonValue> {
    let session_id = params.session_id.trim().to_owned();
    if session_id.is_empty() {
        return Err(StorageError::Validation("session_id"));
    }
    let (mut task, expected) = load_tracked_mt(storage, &params.wp_id, &params.mt_id).await?;
    task.active_session_ids
        .retain(|existing| existing != &session_id);
    let active_session_ids = task.active_session_ids.clone();
    persist_tracked_mt(storage, &task, expected).await?;
    Ok(json!({
        "wp_id": params.wp_id, "mt_id": params.mt_id, "session_id": session_id,
        "active_session_ids": active_session_ids, "reason": params.reason,
    }))
}

async fn complete_mt(
    storage: &SurrealStorage,
    params: LocusCompleteMtParams,
) -> StorageResult<JsonValue> {
    let (mut task, expected) = load_tracked_mt(storage, &params.wp_id, &params.mt_id).await?;
    task.status = MicroTaskStatus::Completed;
    task.current_iteration = task
        .current_iteration
        .max(task.iterations.len() as u32)
        .max(params.final_iteration);
    task.active_session_ids.clear();
    if task.completed_at.is_none() {
        task.completed_at = Some(Utc::now());
    }
    persist_tracked_mt(storage, &task, expected).await?;
    Ok(json!({ "wp_id": params.wp_id, "mt_id": params.mt_id, "status": "completed" }))
}

async fn record_iteration(
    storage: &SurrealStorage,
    params: LocusRecordIterationParams,
) -> StorageResult<JsonValue> {
    let (mut task, expected) = load_tracked_mt(storage, &params.wp_id, &params.mt_id).await?;
    let recorded_iteration = params.iteration.iteration;
    task.status = MicroTaskStatus::InProgress;
    task.current_iteration = task.current_iteration.max(recorded_iteration);
    task.escalation.current_level = params.iteration.escalation_level;
    upsert_tracked_mt_iteration(&mut task, params.iteration.clone());
    let iteration_identity = format!(
        "{}\0{}\0{}",
        params.mt_id, params.iteration.iteration, params.iteration.escalation_level
    );
    let iteration_id = format!("{:x}", Sha256::digest(iteration_identity.as_bytes()));
    let bindings = RecordIterationBindings {
        task: persist_bindings(&task, expected)?,
        iteration: IterationWrite {
            record: RecordId::new(MT_ITERATIONS, iteration_id.clone()),
            iteration_id,
            mt_id: RecordId::new(MICRO_TASKS, params.mt_id.clone()),
            iteration: i64::from(params.iteration.iteration),
            escalation_level: i64::from(params.iteration.escalation_level),
            model_id: params.iteration.model_id.clone(),
            lora_id: params.iteration.lora_id.clone(),
            outcome: iteration_outcome_str(params.iteration.outcome).to_owned(),
            validation_passed: params.iteration.validation_passed,
            duration_ms: i64::try_from(params.iteration.duration_ms)
                .map_err(|_| StorageError::Validation("iteration duration exceeds i64"))?,
        },
    };
    run_record_iteration(storage, bindings).await?;
    Ok(json!({
        "wp_id": params.wp_id, "mt_id": params.mt_id,
        "iteration": params.iteration.iteration, "recorded_iteration": recorded_iteration,
    }))
}

async fn run_record_iteration(
    storage: &SurrealStorage,
    bindings: RecordIterationBindings,
) -> StorageResult<()> {
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         LET $current = SELECT * FROM ONLY $task.record; \
                         IF $current = NONE OR $current.wp_id != $task.wp { THROW 'HSK-LOCUS-MT-MISSING'; }; \
                         IF $current.metadata != $task.expected_metadata { THROW 'HSK-LOCUS-MT-STALE'; }; \
                         UPSERT $iteration.record SET iteration_id = $iteration.iteration_id, \
                           mt_id = $iteration.mt_id, iteration = $iteration.iteration, \
                           escalation_level = $iteration.escalation_level, \
                           model_id = $iteration.model_id, lora_id = $iteration.lora_id, \
                           outcome = $iteration.outcome, validation_passed = $iteration.validation_passed, \
                           duration_ms = $iteration.duration_ms RETURN AFTER; \
                         UPDATE $task.record SET name = $task.name, status = $task.status, \
                           current_iteration = $task.current_iteration, escalation_level = $task.escalation_level, \
                           metadata = $task.metadata RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        5,
                    )
                    .await
            })
        })
        .await;
    match result {
        Ok(rows) if rows.len() == 1 => Ok(()),
        Ok(_) => Err(StorageError::Database(
            "Locus iteration update returned an unexpected row count".to_owned(),
        )),
        Err(error) => Err(map_locus_error(error)),
    }
}

async fn add_dependency(
    storage: &SurrealStorage,
    params: LocusAddDependencyParams,
) -> StorageResult<JsonValue> {
    if params.from_wp_id == params.to_wp_id {
        return Err(StorageError::Validation("dependency would create a cycle"));
    }
    let _guard = DEPENDENCY_MUTATION_LOCK.lock().await;
    ensure_wp_exists(storage, &params.from_wp_id).await?;
    ensure_wp_exists(storage, &params.to_wp_id).await?;
    let dependencies = load_dependencies(storage).await?;
    if dependencies
        .iter()
        .any(|row| row.dependency_id == params.dependency_id)
    {
        return Err(StorageError::Conflict("dependency already exists"));
    }
    if dependency_would_create_cycle(&dependencies, &params.from_wp_id, &params.to_wp_id)? {
        return Err(StorageError::Validation("dependency would create a cycle"));
    }
    let now = now_rfc3339();
    let bindings = DependencyWrite {
        record: RecordId::new(DEPENDENCIES, params.dependency_id.clone()),
        dependency_id: params.dependency_id.clone(),
        from_wp_id: RecordId::new(WORK_PACKETS, params.from_wp_id.clone()),
        to_wp_id: RecordId::new(WORK_PACKETS, params.to_wp_id.clone()),
        dependency_type: dependency_type_str(params.kind).to_owned(),
        created_at: now.clone(),
        vector_clock: serde_json::to_string(&json!({"local": 1}))?,
    };
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         IF record::exists($from_wp_id) = false OR record::exists($to_wp_id) = false \
                           { THROW 'HSK-LOCUS-WP-MISSING'; }; \
                         IF record::exists($record) { THROW 'HSK-LOCUS-DEPENDENCY-DUPLICATE'; }; \
                         CREATE $record CONTENT { dependency_id: $dependency_id, \
                           from_wp_id: $from_wp_id, to_wp_id: $to_wp_id, \
                           dependency_type: $dependency_type, created_at: $created_at, \
                           vector_clock: $vector_clock } RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        3,
                    )
                    .await
            })
        })
        .await;
    match result {
        Ok(rows) if rows.len() == 1 => {}
        Ok(_) => {
            return Err(StorageError::Database(
                "Locus dependency create returned an unexpected row count".to_owned(),
            ));
        }
        Err(error) => return Err(map_locus_error(error)),
    }
    Ok(json!({
        "dependency_id": params.dependency_id,
        "from_wp_id": params.from_wp_id,
        "to_wp_id": params.to_wp_id,
        "type": dependency_type_str(params.kind),
        "created_at": now,
    }))
}

async fn remove_dependency(
    storage: &SurrealStorage,
    params: LocusRemoveDependencyParams,
) -> StorageResult<JsonValue> {
    let _guard = DEPENDENCY_MUTATION_LOCK.lock().await;
    let bindings = RecordBinding {
        record: RecordId::new(DEPENDENCIES, params.dependency_id.clone()),
    };
    let rows: Vec<DependencyRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; DELETE $record RETURN BEFORE; COMMIT TRANSACTION;",
                        bindings,
                        1,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    if rows.is_empty() {
        return Err(StorageError::NotFound("dependency"));
    }
    Ok(json!({ "dependency_id": params.dependency_id, "deleted": true }))
}

async fn query_ready(
    storage: &SurrealStorage,
    params: LocusQueryReadyParams,
) -> StorageResult<JsonValue> {
    let limit = params.limit.unwrap_or(100);
    if limit == 0 {
        return Ok(json!({ "wp_ids": [] }));
    }
    let candidates: Vec<WorkPacketRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         LET $blocked = array::union( \
                           array::union( \
                             (SELECT VALUE to_wp_id FROM dependencies \
                              WHERE dependency_type = 'blocks' \
                                AND from_wp_id.status NOT IN ['done', 'cancelled']), \
                             (SELECT VALUE from_wp_id FROM dependencies \
                              WHERE dependency_type = 'blocked_by' \
                                AND to_wp_id.status NOT IN ['done', 'cancelled']) \
                           ), \
                           (SELECT VALUE from_wp_id FROM dependencies \
                            WHERE dependency_type = 'depends-on' \
                              AND to_wp_id.status NOT IN ['done', 'cancelled']) \
                         ); \
                         SELECT * FROM work_packets \
                           WHERE status = 'ready' AND id NOT IN $blocked \
                           ORDER BY priority ASC, created_at ASC LIMIT $limit; \
                         COMMIT TRANSACTION;",
                        ReadyBindings {
                            limit: i64::from(limit),
                        },
                        2,
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let wp_ids = candidates
        .into_iter()
        .map(|candidate| candidate.wp_id)
        .collect::<Vec<_>>();
    Ok(json!({ "wp_ids": wp_ids }))
}

async fn get_wp_status(
    storage: &SurrealStorage,
    params: LocusGetWpStatusParams,
) -> StorageResult<JsonValue> {
    let row = load_wp(storage, &params.wp_id).await?;
    Ok(json!({
        "wp_id": params.wp_id,
        "version": row.version,
        "status": row.status,
        "task_board_status": row.task_board_status,
        "updated_at": row.updated_at,
    }))
}

async fn sync_task_board_snapshot(
    storage: &SurrealStorage,
    params: LocusSyncTaskBoardParams,
) -> StorageResult<JsonValue> {
    #[derive(SurrealValue)]
    struct ProjectionRow {
        wp_id: String,
        status: String,
        task_board_status: String,
        updated_at: String,
        metadata: String,
    }
    let rows: Vec<ProjectionRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT wp_id, status, task_board_status, updated_at, metadata \
                         FROM work_packets ORDER BY updated_at ASC, wp_id ASC;",
                        EmptyBindings {},
                    )
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let authority_rows = rows
        .into_iter()
        .map(|row| {
            let metadata: JsonValue = serde_json::from_str(&row.metadata)?;
            Ok(json!({
                "wp_id": row.wp_id,
                "status": row.status,
                "task_board_status": row.task_board_status,
                "updated_at": row.updated_at,
                "metadata": metadata,
            }))
        })
        .collect::<StorageResult<Vec<_>>>()?;
    Ok(json!({
        "dry_run": params.dry_run.unwrap_or(false),
        "authority_rows": authority_rows,
    }))
}

async fn load_dependencies(storage: &SurrealStorage) -> StorageResult<Vec<DependencyRow>> {
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values("SELECT * FROM dependencies;", EmptyBindings {})
                    .await
            })
        })
        .await
        .map_err(StorageError::from)
}

fn dependency_would_create_cycle(
    rows: &[DependencyRow],
    from_wp_id: &str,
    to_wp_id: &str,
) -> StorageResult<bool> {
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        outgoing
            .entry(record_string_key(&row.from_wp_id, WORK_PACKETS)?)
            .or_default()
            .push(record_string_key(&row.to_wp_id, WORK_PACKETS)?);
    }
    let mut queue = VecDeque::from([to_wp_id.to_owned()]);
    let mut visited = HashSet::new();
    while let Some(current) = queue.pop_front() {
        if current == from_wp_id {
            return Ok(true);
        }
        if visited.insert(current.clone()) {
            queue.extend(outgoing.get(&current).into_iter().flatten().cloned());
        }
    }
    Ok(false)
}

async fn get_mt_progress(
    storage: &SurrealStorage,
    params: LocusGetMtProgressParams,
) -> StorageResult<JsonValue> {
    let record = RecordId::new(MICRO_TASKS, params.mt_id);
    let row: Option<MicroTaskRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first("SELECT * FROM $record;", RecordBinding { record })
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let row = row.ok_or(StorageError::NotFound("micro_task"))?;
    let metadata = match serde_json::from_str::<TrackedMicroTask>(&row.metadata) {
        Ok(task) => tracked_mt_progress_metadata(&task),
        Err(_) => serde_json::from_str(&row.metadata).unwrap_or_else(|_| json!({})),
    };
    Ok(json!({
        "mt_id": row.mt_id,
        "wp_id": record_string_key(&row.wp_id, WORK_PACKETS)?,
        "name": row.name,
        "status": row.status,
        "current_iteration": row.current_iteration,
        "escalation_level": row.escalation_level,
        "metadata": metadata,
    }))
}

async fn load_tracked_mt(
    storage: &SurrealStorage,
    wp_id: &str,
    mt_id: &str,
) -> StorageResult<(TrackedMicroTask, String)> {
    let bindings = MicroTaskBinding {
        record: RecordId::new(MICRO_TASKS, mt_id.to_owned()),
        wp: RecordId::new(WORK_PACKETS, wp_id.to_owned()),
    };
    let row: Option<MicroTaskRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first("SELECT * FROM $record WHERE wp_id = $wp;", bindings)
                    .await
            })
        })
        .await
        .map_err(StorageError::from)?;
    let row = row.ok_or(StorageError::NotFound("micro_task"))?;
    ensure_record_key(&row.id, MICRO_TASKS, &row.mt_id)?;
    let expected = row.metadata;
    let mut task: TrackedMicroTask = serde_json::from_str(&expected)?;
    dedupe_session_ids(&mut task.active_session_ids);
    Ok((task, expected))
}

async fn persist_tracked_mt(
    storage: &SurrealStorage,
    task: &TrackedMicroTask,
    expected_metadata: String,
) -> StorageResult<()> {
    let bindings = persist_bindings(task, expected_metadata)?;
    let result = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         LET $current = SELECT * FROM ONLY $record; \
                         IF $current = NONE OR $current.wp_id != $wp { THROW 'HSK-LOCUS-MT-MISSING'; }; \
                         IF $current.metadata != $expected_metadata { THROW 'HSK-LOCUS-MT-STALE'; }; \
                         UPDATE $record SET name = $name, status = $status, \
                           current_iteration = $current_iteration, escalation_level = $escalation_level, \
                           metadata = $metadata RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await;
    match result {
        Ok(rows) if rows.len() == 1 => Ok(()),
        Ok(_) => Err(StorageError::Database(
            "Locus micro-task update returned an unexpected row count".to_owned(),
        )),
        Err(error) => Err(map_locus_error(error)),
    }
}

fn persist_bindings(
    task: &TrackedMicroTask,
    expected_metadata: String,
) -> StorageResult<PersistMicroTaskBindings> {
    Ok(PersistMicroTaskBindings {
        record: RecordId::new(MICRO_TASKS, task.mt_id.clone()),
        wp: RecordId::new(WORK_PACKETS, task.wp_id.clone()),
        expected_metadata,
        name: task.name.clone(),
        status: micro_task_status_str(task.status).to_owned(),
        current_iteration: i64::from(task.current_iteration),
        escalation_level: i64::from(task.escalation.current_level),
        metadata: serde_json::to_string(task)?,
    })
}

fn required_string(value: JsonValue) -> StorageResult<String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or(StorageError::Validation("unsupported update value type"))
}

fn optional_string(value: JsonValue) -> StorageResult<Option<String>> {
    match value {
        JsonValue::String(value) => Ok(Some(value)),
        JsonValue::Null => Ok(None),
        _ => Err(StorageError::Validation("unsupported update value type")),
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn work_packet_status_str(status: WorkPacketStatus) -> &'static str {
    match status {
        WorkPacketStatus::Unknown => "stub",
        WorkPacketStatus::Ready => "ready",
        WorkPacketStatus::InProgress => "in_progress",
        WorkPacketStatus::Blocked => "blocked",
        WorkPacketStatus::Gated => "gated",
        WorkPacketStatus::Done => "done",
        WorkPacketStatus::Cancelled => "cancelled",
    }
}

fn canonical_work_packet_status_for_storage(value: &str) -> StorageResult<&'static str> {
    match value.trim() {
        "STUB" | "UNKNOWN" | "stub" | "unknown" => Ok("stub"),
        "READY" | "READY_FOR_DEV" | "ready" => Ok("ready"),
        "IN_PROGRESS" | "in_progress" => Ok("in_progress"),
        "BLOCKED" | "blocked" => Ok("blocked"),
        "GATED" | "gated" => Ok("gated"),
        "DONE" | "done" => Ok("done"),
        "CANCELLED" | "cancelled" => Ok("cancelled"),
        _ => Err(StorageError::Validation("unsupported work-packet status")),
    }
}

fn canonical_task_board_status_for_storage(value: &str) -> StorageResult<&'static str> {
    match value.trim() {
        "STUB" | "UNKNOWN" | "stub" | "unknown" => Ok("STUB"),
        "READY" | "READY_FOR_DEV" | "ready" => Ok("READY"),
        "IN_PROGRESS" | "in_progress" => Ok("IN_PROGRESS"),
        "BLOCKED" | "blocked" => Ok("BLOCKED"),
        "GATED" | "gated" => Ok("GATED"),
        "DONE" | "done" => Ok("DONE"),
        "CANCELLED" | "cancelled" => Ok("CANCELLED"),
        _ => Err(StorageError::Validation("unsupported task-board status")),
    }
}

fn task_board_status_str(status: TaskBoardStatus) -> &'static str {
    match status {
        TaskBoardStatus::Unknown => "STUB",
        TaskBoardStatus::Ready => "READY",
        TaskBoardStatus::InProgress => "IN_PROGRESS",
        TaskBoardStatus::Blocked => "BLOCKED",
        TaskBoardStatus::Gated => "GATED",
        TaskBoardStatus::Done => "DONE",
        TaskBoardStatus::Cancelled => "CANCELLED",
    }
}

fn micro_task_status_str(status: MicroTaskStatus) -> &'static str {
    match status {
        MicroTaskStatus::Pending => "pending",
        MicroTaskStatus::InProgress => "in_progress",
        MicroTaskStatus::Completed => "completed",
        MicroTaskStatus::Failed => "failed",
        MicroTaskStatus::Blocked => "blocked",
        MicroTaskStatus::Skipped => "skipped",
    }
}

fn phase_str(phase: WorkPacketPhase) -> &'static str {
    match phase {
        WorkPacketPhase::Phase0 => "0",
        WorkPacketPhase::Phase0_5 => "0.5",
        WorkPacketPhase::Phase1 => "1",
        WorkPacketPhase::Phase2 => "2",
        WorkPacketPhase::Phase3 => "3",
        WorkPacketPhase::Phase4 => "4",
    }
}

fn routing_str(routing: RoutingPolicy) -> &'static str {
    match routing {
        RoutingPolicy::GovStrict => "GOV_STRICT",
        RoutingPolicy::GovStandard => "GOV_STANDARD",
        RoutingPolicy::GovLight => "GOV_LIGHT",
        RoutingPolicy::GovNone => "GOV_NONE",
    }
}

fn dependency_type_str(kind: DependencyType) -> &'static str {
    match kind {
        DependencyType::Blocks => "blocks",
        DependencyType::BlockedBy => "blocked_by",
        DependencyType::Related => "related",
        DependencyType::ParentChild => "parent-child",
        DependencyType::DiscoveredFrom => "discovered-from",
        DependencyType::DuplicateOf => "duplicate-of",
        DependencyType::DependsOn => "depends-on",
        DependencyType::Implements => "implements",
        DependencyType::Tests => "tests",
        DependencyType::Documents => "documents",
    }
}

fn iteration_outcome_str(outcome: MicroTaskIterationOutcome) -> &'static str {
    match outcome {
        MicroTaskIterationOutcome::Success => "SUCCESS",
        MicroTaskIterationOutcome::Retry => "RETRY",
        MicroTaskIterationOutcome::Escalate => "ESCALATE",
        MicroTaskIterationOutcome::Blocked => "BLOCKED",
        MicroTaskIterationOutcome::Skipped => "SKIPPED",
    }
}

fn dedupe_session_ids(session_ids: &mut Vec<String>) {
    let mut seen = HashSet::new();
    *session_ids = session_ids
        .iter()
        .filter_map(|session_id| {
            let normalized = session_id.trim();
            if normalized.is_empty() || !seen.insert(normalized.to_owned()) {
                None
            } else {
                Some(normalized.to_owned())
            }
        })
        .collect();
}

fn tracked_mt_iteration_index(
    task: &TrackedMicroTask,
    iteration: &crate::workflows::locus::types::MicroTaskIterationRecord,
) -> Option<usize> {
    task.iterations.iter().position(|existing| {
        existing.iteration == iteration.iteration
            && existing.escalation_level == iteration.escalation_level
    })
}

fn upsert_tracked_mt_iteration(
    task: &mut TrackedMicroTask,
    iteration: crate::workflows::locus::types::MicroTaskIterationRecord,
) {
    if let Some(index) = tracked_mt_iteration_index(task, &iteration) {
        task.iterations[index] = iteration;
    } else {
        task.iterations.push(iteration);
    }
}

fn tracked_mt_progress_metadata(task: &TrackedMicroTask) -> JsonValue {
    let has_mailbox_wait = task
        .metadata
        .get("has_pending_mailbox_wait")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let (family, base_reason) = match task.status {
        MicroTaskStatus::Pending => (
            WorkflowStateFamily::Ready,
            WorkflowQueueReasonCode::ReadyForLocalSmallModel,
        ),
        MicroTaskStatus::InProgress => (
            WorkflowStateFamily::Active,
            WorkflowQueueReasonCode::ReadyForLocalSmallModel,
        ),
        MicroTaskStatus::Completed => (
            WorkflowStateFamily::Done,
            WorkflowQueueReasonCode::ValidationWait,
        ),
        MicroTaskStatus::Failed => (
            WorkflowStateFamily::Blocked,
            WorkflowQueueReasonCode::BlockedError,
        ),
        MicroTaskStatus::Blocked => (
            WorkflowStateFamily::Blocked,
            WorkflowQueueReasonCode::BlockedMissingContext,
        ),
        MicroTaskStatus::Skipped => (
            WorkflowStateFamily::Canceled,
            WorkflowQueueReasonCode::BlockedPolicy,
        ),
    };
    let reason = resolve_queue_reason_with_mailbox_context(base_reason, has_mailbox_wait);
    let summary_ref = task
        .summary_record_path
        .clone()
        .or_else(|| {
            task.metadata
                .get("structured_collaboration_summary_path")
                .and_then(JsonValue::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    serde_json::to_value(TrackedMicroTaskArtifactV1 {
        schema_id: task.schema_id.clone(),
        schema_version: task.schema_version.clone(),
        record_id: task.record_id.clone(),
        record_kind: task.record_kind.clone(),
        project_profile_kind: task.project_profile_kind,
        profile_extension: task.profile_extension.clone(),
        updated_at: task.updated_at.to_rfc3339(),
        mirror_state: task.mirror_state,
        authority_refs: task.authority_refs.clone(),
        evidence_refs: task.evidence_refs.clone(),
        mirror_contract: None,
        workflow_state_family: family,
        queue_reason_code: reason,
        allowed_action_ids: governed_action_ids_for_family(family),
        transition_rule_ids: transition_rule_ids_for_family(family),
        queue_automation_rule_ids: queue_automation_rule_ids_for_reason(reason),
        executor_eligibility_policy_ids: executor_eligibility_policy_ids_for_family(family),
        summary_ref,
        mt_id: task.mt_id.clone(),
        wp_id: task.wp_id.clone(),
        name: task.name.clone(),
        scope: task.scope.clone(),
        files: task.files.clone(),
        done_criteria: task.done_criteria.clone(),
        status: task.status,
        active_session_ids: task.active_session_ids.clone(),
        iterations: task.iterations.clone(),
        current_iteration: task.current_iteration,
        max_iterations: task.max_iterations,
        validation_result: task.validation_result.clone(),
        escalation: task.escalation.clone(),
        started_at: task.started_at,
        completed_at: task.completed_at,
        duration_ms: task.duration_ms,
        depends_on: task.depends_on.clone(),
        metadata: task.metadata.clone(),
    })
    .unwrap_or_else(|_| task.metadata.clone())
}

fn ensure_record_key(record: &RecordId, table: &'static str, expected: &str) -> StorageResult<()> {
    let actual = record_string_key(record, table)?;
    if actual == expected {
        Ok(())
    } else {
        Err(StorageError::Database(format!(
            "{table} record key `{actual}` does not match alias `{expected}`"
        )))
    }
}

fn record_string_key(record: &RecordId, table: &'static str) -> StorageResult<String> {
    if record.table.as_str() != table {
        return Err(StorageError::Database(format!(
            "expected {table} record, got {}",
            record.table.as_str()
        )));
    }
    match &record.key {
        RecordIdKey::String(value) => Ok(value.clone()),
        _ => Err(StorageError::Database(format!(
            "{table} record does not have a string key"
        ))),
    }
}

fn map_locus_error(error: super::SurrealStorageError) -> StorageError {
    let message = error.to_string();
    if message.contains("HSK-LOCUS-WP-MISSING") {
        StorageError::NotFound("work_packet")
    } else if message.contains("HSK-LOCUS-MT-MISSING") {
        StorageError::NotFound("micro_task")
    } else if message.contains("HSK-LOCUS-WP-DUPLICATE") {
        StorageError::Conflict("work_packet already exists")
    } else if message.contains("HSK-LOCUS-MT-OTHER-WP") {
        StorageError::Conflict("micro_task already registered to a different work_packet")
    } else if message.contains("HSK-LOCUS-MT-DIVERGENT") {
        StorageError::Conflict("micro_task retry payload diverged from stored state")
    } else if message.contains("HSK-LOCUS-WP-STALE") {
        StorageError::Conflict("work_packet changed concurrently")
    } else if message.contains("HSK-LOCUS-MT-STALE") {
        StorageError::Conflict("micro_task changed concurrently")
    } else {
        StorageError::from(error)
    }
}
