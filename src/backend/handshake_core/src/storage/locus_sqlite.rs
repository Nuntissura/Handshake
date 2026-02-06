use super::{sqlite::SqliteDatabase, Database, StorageError, StorageResult};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::workflows::locus::types::{
    DependencyType, LocusAddDependencyParams, LocusCloseWpParams, LocusCompleteMtParams,
    LocusCreateWpParams, LocusDeleteWpParams, LocusGateKind, LocusGateWpParams,
    LocusGetMtProgressParams, LocusGetWpStatusParams, LocusOperation, LocusQueryReadyParams,
    LocusRecordIterationParams, LocusRegisterMtsParams, LocusRemoveDependencyParams,
    LocusStartMtParams, LocusUpdateWpParams, MicroTaskIterationOutcome, MicroTaskStatus,
    RoutingPolicy, TaskBoardStatus, WorkPacketPhase, WorkPacketStatus,
};

fn sqlite_db(db: &dyn Database) -> StorageResult<&SqliteDatabase> {
    db.as_any()
        .downcast_ref::<SqliteDatabase>()
        .ok_or(StorageError::NotImplemented("locus sqlite"))
}

pub(crate) fn ensure_locus_sqlite(db: &dyn Database) -> StorageResult<()> {
    sqlite_db(db).map(|_| ())
}

pub(crate) async fn execute_locus_operation(
    db: &dyn Database,
    op: LocusOperation,
) -> StorageResult<Value> {
    let sqlite = sqlite_db(db)?;
    execute_sqlite_locus_operation(sqlite, op).await
}

pub(crate) async fn locus_work_packet_exists(
    db: &dyn Database,
    wp_id: &str,
) -> StorageResult<bool> {
    let sqlite = sqlite_db(db)?;
    let exists =
        sqlx::query_scalar::<_, i64>("SELECT 1 FROM work_packets WHERE wp_id = $1 LIMIT 1")
            .bind(wp_id)
            .fetch_optional(sqlite.pool())
            .await?
            .is_some();
    Ok(exists)
}

pub(crate) async fn locus_task_board_get_status_and_metadata(
    db: &dyn Database,
    wp_id: &str,
) -> StorageResult<Option<(String, String)>> {
    let sqlite = sqlite_db(db)?;
    sqlx::query_as::<_, (String, String)>(
        "SELECT task_board_status, metadata FROM work_packets WHERE wp_id = $1",
    )
    .bind(wp_id)
    .fetch_optional(sqlite.pool())
    .await
    .map_err(StorageError::from)
}

pub(crate) async fn locus_task_board_update_work_packet(
    db: &dyn Database,
    status: &str,
    task_board_status: &str,
    updated_at: &str,
    metadata: &str,
    wp_id: &str,
) -> StorageResult<()> {
    let sqlite = sqlite_db(db)?;
    sqlx::query(
        r#"
                        UPDATE work_packets
                        SET
                            version = version + 1,
                            status = $1,
                            task_board_status = $2,
                            updated_at = $3,
                            metadata = $4
                        WHERE wp_id = $5
                        "#,
    )
    .bind(status)
    .bind(task_board_status)
    .bind(updated_at)
    .bind(metadata)
    .bind(wp_id)
    .execute(sqlite.pool())
    .await?;
    Ok(())
}

pub(crate) async fn locus_task_board_list_rows(
    db: &dyn Database,
) -> StorageResult<Vec<(String, String, String)>> {
    let sqlite = sqlite_db(db)?;
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT wp_id, task_board_status, metadata FROM work_packets",
    )
    .fetch_all(sqlite.pool())
    .await?;
    Ok(rows)
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

fn mt_iteration_outcome_str(outcome: MicroTaskIterationOutcome) -> &'static str {
    match outcome {
        MicroTaskIterationOutcome::Success => "SUCCESS",
        MicroTaskIterationOutcome::Retry => "RETRY",
        MicroTaskIterationOutcome::Escalate => "ESCALATE",
        MicroTaskIterationOutcome::Blocked => "BLOCKED",
        MicroTaskIterationOutcome::Skipped => "SKIPPED",
    }
}

async fn ensure_wp_exists(pool: &SqlitePool, wp_id: &str) -> StorageResult<()> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM work_packets WHERE wp_id = $1")
        .bind(wp_id)
        .fetch_optional(pool)
        .await?
        .is_some();

    if !exists {
        return Err(StorageError::NotFound("work_packet"));
    }

    Ok(())
}

async fn ensure_mt_exists_for_wp(pool: &SqlitePool, wp_id: &str, mt_id: &str) -> StorageResult<()> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM micro_tasks WHERE mt_id = $1 AND wp_id = $2 LIMIT 1",
    )
    .bind(mt_id)
    .bind(wp_id)
    .fetch_optional(pool)
    .await?
    .is_some();

    if !exists {
        return Err(StorageError::NotFound("micro_task"));
    }

    Ok(())
}

async fn dependency_would_create_cycle(
    pool: &SqlitePool,
    from_wp_id: &str,
    to_wp_id: &str,
) -> StorageResult<bool> {
    if from_wp_id == to_wp_id {
        return Ok(true);
    }

    // Adding edge from -> to creates a cycle if there is already a path to -> ... -> from.
    let sql = r#"
        WITH RECURSIVE reach(wp_id) AS (
            SELECT to_wp_id FROM dependencies WHERE from_wp_id = $1
            UNION
            SELECT d.to_wp_id FROM dependencies d
            INNER JOIN reach r ON d.from_wp_id = r.wp_id
        )
        SELECT 1 FROM reach WHERE wp_id = $2 LIMIT 1;
    "#;
    let found = sqlx::query_scalar::<_, i64>(sql)
        .bind(to_wp_id)
        .bind(from_wp_id)
        .fetch_optional(pool)
        .await?
        .is_some();

    Ok(found)
}

async fn create_wp(pool: &SqlitePool, params: LocusCreateWpParams) -> StorageResult<Value> {
    if params.priority > 4 {
        return Err(StorageError::Validation("priority must be between 0 and 4"));
    }

    let existing = sqlx::query_scalar::<_, i64>("SELECT 1 FROM work_packets WHERE wp_id = $1")
        .bind(&params.wp_id)
        .fetch_optional(pool)
        .await?;
    if existing.is_some() {
        return Err(StorageError::Conflict("work_packet already exists"));
    }

    let now = now_rfc3339();
    let status = WorkPacketStatus::Unknown;
    let task_board_status = TaskBoardStatus::Unknown;
    let vector_clock = json!({"local": 1});
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
    });

    sqlx::query(
        r#"
        INSERT INTO work_packets (
            wp_id, version, title, description, status, priority, phase, routing, task_packet_path,
            task_board_status, assignee, reporter, created_at, updated_at, vector_clock, metadata
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9,
            $10, $11, $12, $13, $14, $15, $16
        )
        "#,
    )
    .bind(&params.wp_id)
    .bind(1i64)
    .bind(&params.title)
    .bind(&params.description)
    .bind(work_packet_status_str(status))
    .bind(params.priority as i64)
    .bind(phase_str(params.phase))
    .bind(routing_str(params.routing))
    .bind(params.task_packet_path.as_deref())
    .bind(task_board_status_str(task_board_status))
    .bind(params.assignee.as_deref())
    .bind(&params.reporter)
    .bind(&now)
    .bind(&now)
    .bind(serde_json::to_string(&vector_clock)?)
    .bind(serde_json::to_string(&metadata)?)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "version": 1,
        "status": work_packet_status_str(status),
        "task_board_status": task_board_status_str(task_board_status),
        "created_at": now,
        "updated_at": now,
    }))
}

async fn update_wp(pool: &SqlitePool, params: LocusUpdateWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;

    let now = now_rfc3339();

    let mut cols: Vec<(&'static str, Value)> = Vec::new();
    for (key, value) in params.updates {
        let col = match key.as_str() {
            "title" => "title",
            "description" => "description",
            "priority" => "priority",
            "status" => "status",
            "assignee" => "assignee",
            "governance.phase" | "phase" => "phase",
            "governance.routing" | "routing" => "routing",
            "governance.task_packet_path" | "task_packet_path" => "task_packet_path",
            "governance.task_board_status" | "task_board_status" => "task_board_status",
            other => {
                return Err(StorageError::Validation(match other {
                    "" => "empty update key",
                    _ => "unsupported update key",
                }));
            }
        };
        cols.push((col, value));
    }

    if cols.is_empty() {
        return Ok(json!({
            "wp_id": params.wp_id,
            "updated_at": now,
            "no_op": true,
        }));
    }

    let mut sql = String::from("UPDATE work_packets SET version = version + 1, updated_at = $1");
    for (idx, (col, _)) in cols.iter().enumerate() {
        sql.push_str(", ");
        sql.push_str(col);
        sql.push_str(" = $");
        sql.push_str(&(idx + 2).to_string());
    }
    sql.push_str(" WHERE wp_id = $");
    sql.push_str(&(cols.len() + 2).to_string());

    let mut query = sqlx::query(&sql).bind(&now);
    for (col, value) in cols {
        match col {
            "priority" => match value {
                Value::Number(n) => {
                    let prio = n
                        .as_i64()
                        .ok_or(StorageError::Validation("priority must be an integer"))?;
                    if !(0..=4).contains(&prio) {
                        return Err(StorageError::Validation("priority must be between 0 and 4"));
                    }
                    query = query.bind(prio);
                }
                _ => return Err(StorageError::Validation("priority must be an integer")),
            },
            _ => match value {
                Value::String(s) => query = query.bind(s),
                Value::Null => query = query.bind(Option::<String>::None),
                _ => return Err(StorageError::Validation("unsupported update value type")),
            },
        }
    }
    query = query.bind(&params.wp_id);

    let result = query.execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound("work_packet"));
    }

    Ok(json!({
        "wp_id": params.wp_id,
        "updated_at": now,
    }))
}

async fn gate_wp(pool: &SqlitePool, params: LocusGateWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;

    // Persist gate status into metadata JSON (append-only gate state is out-of-scope for Phase 1).
    let row = sqlx::query_scalar::<_, String>("SELECT metadata FROM work_packets WHERE wp_id = $1")
        .bind(&params.wp_id)
        .fetch_one(pool)
        .await?;
    let mut metadata: Value = serde_json::from_str(&row)?;

    let gate_key = match params.gate {
        LocusGateKind::PreWork => "pre_work",
        LocusGateKind::PostWork => "post_work",
    };

    let gate_value = serde_json::to_value(&params.result)?;
    metadata
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata must be an object"))?
        .entry("gates".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata.gates must be an object"))?
        .insert(gate_key.to_string(), gate_value);

    let now = now_rfc3339();
    sqlx::query(
        "UPDATE work_packets SET version = version + 1, updated_at = $1, metadata = $2 WHERE wp_id = $3",
    )
    .bind(&now)
    .bind(serde_json::to_string(&metadata)?)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "gate": gate_key,
        "updated_at": now,
    }))
}

async fn close_wp(pool: &SqlitePool, params: LocusCloseWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    let now = now_rfc3339();
    sqlx::query(
        r#"
        UPDATE work_packets
        SET
            version = version + 1,
            status = $1,
            task_board_status = $2,
            updated_at = $3
        WHERE wp_id = $4
        "#,
    )
    .bind(work_packet_status_str(WorkPacketStatus::Done))
    .bind(task_board_status_str(TaskBoardStatus::Done))
    .bind(&now)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "status": "done",
        "updated_at": now,
    }))
}

async fn delete_wp(pool: &SqlitePool, params: LocusDeleteWpParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    let now = now_rfc3339();

    let row = sqlx::query_scalar::<_, String>("SELECT metadata FROM work_packets WHERE wp_id = $1")
        .bind(&params.wp_id)
        .fetch_one(pool)
        .await?;
    let mut metadata: Value = serde_json::from_str(&row)?;
    metadata
        .as_object_mut()
        .ok_or(StorageError::Validation("metadata must be an object"))?
        .insert("tombstone".to_string(), json!({ "deleted_at": now }));

    sqlx::query(
        r#"
        UPDATE work_packets
        SET
            version = version + 1,
            status = $1,
            task_board_status = $2,
            updated_at = $3,
            metadata = $4
        WHERE wp_id = $5
        "#,
    )
    .bind(work_packet_status_str(WorkPacketStatus::Cancelled))
    .bind(task_board_status_str(TaskBoardStatus::Cancelled))
    .bind(&now)
    .bind(serde_json::to_string(&metadata)?)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "status": "cancelled",
        "updated_at": now,
    }))
}

async fn register_mts(pool: &SqlitePool, params: LocusRegisterMtsParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;

    let mut tx = pool.begin().await?;
    for mt in params.micro_tasks {
        if mt.wp_id != params.wp_id {
            return Err(StorageError::Validation("micro task wp_id mismatch"));
        }

        let metadata = serde_json::to_string(&mt)?;
        sqlx::query(
            r#"
            INSERT INTO micro_tasks (
                mt_id, wp_id, name, status, current_iteration, escalation_level, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(&mt.mt_id)
        .bind(&mt.wp_id)
        .bind(&mt.name)
        .bind(micro_task_status_str(mt.status))
        .bind(mt.current_iteration as i64)
        .bind(mt.escalation.current_level as i64)
        .bind(metadata)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(json!({
        "wp_id": params.wp_id,
        "registered": true,
    }))
}

async fn start_mt(pool: &SqlitePool, params: LocusStartMtParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;
    let now = now_rfc3339();

    let result = sqlx::query(
        r#"
        UPDATE micro_tasks
        SET status = $1
        WHERE mt_id = $2 AND wp_id = $3
        "#,
    )
    .bind(micro_task_status_str(MicroTaskStatus::InProgress))
    .bind(&params.mt_id)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound("micro_task"));
    }

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "status": "in_progress",
        "updated_at": now,
    }))
}

async fn record_iteration(
    pool: &SqlitePool,
    params: LocusRecordIterationParams,
) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO mt_iterations (
            mt_id, iteration, model_id, lora_id, outcome, validation_passed, duration_ms
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&params.mt_id)
    .bind(params.iteration.iteration as i64)
    .bind(&params.iteration.model_id)
    .bind(params.iteration.lora_id.as_deref())
    .bind(mt_iteration_outcome_str(params.iteration.outcome))
    .bind(
        params
            .iteration
            .validation_passed
            .map(|v| if v { 1i64 } else { 0i64 }),
    )
    .bind(params.iteration.duration_ms as i64)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE micro_tasks
        SET status = $1, current_iteration = $2, escalation_level = $3
        WHERE mt_id = $4 AND wp_id = $5
        "#,
    )
    .bind(micro_task_status_str(MicroTaskStatus::InProgress))
    .bind(params.iteration.iteration as i64)
    .bind(params.iteration.escalation_level as i64)
    .bind(&params.mt_id)
    .bind(&params.wp_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "iteration": params.iteration.iteration,
    }))
}

async fn complete_mt(pool: &SqlitePool, params: LocusCompleteMtParams) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.wp_id).await?;
    ensure_mt_exists_for_wp(pool, &params.wp_id, &params.mt_id).await?;

    let result = sqlx::query(
        r#"
        UPDATE micro_tasks
        SET status = $1, current_iteration = $2
        WHERE mt_id = $3 AND wp_id = $4
        "#,
    )
    .bind(micro_task_status_str(MicroTaskStatus::Completed))
    .bind(params.final_iteration as i64)
    .bind(&params.mt_id)
    .bind(&params.wp_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound("micro_task"));
    }

    Ok(json!({
        "wp_id": params.wp_id,
        "mt_id": params.mt_id,
        "status": "completed",
    }))
}

async fn add_dependency(
    pool: &SqlitePool,
    params: LocusAddDependencyParams,
) -> StorageResult<Value> {
    ensure_wp_exists(pool, &params.from_wp_id).await?;
    ensure_wp_exists(pool, &params.to_wp_id).await?;

    if dependency_would_create_cycle(pool, &params.from_wp_id, &params.to_wp_id).await? {
        return Err(StorageError::Validation("dependency would create a cycle"));
    }

    let existing =
        sqlx::query_scalar::<_, i64>("SELECT 1 FROM dependencies WHERE dependency_id = $1 LIMIT 1")
            .bind(&params.dependency_id)
            .fetch_optional(pool)
            .await?;
    if existing.is_some() {
        return Err(StorageError::Conflict("dependency already exists"));
    }

    let now = now_rfc3339();
    let vector_clock = json!({"local": 1});
    sqlx::query(
        r#"
        INSERT INTO dependencies (
            dependency_id, from_wp_id, to_wp_id, type, created_at, vector_clock
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&params.dependency_id)
    .bind(&params.from_wp_id)
    .bind(&params.to_wp_id)
    .bind(dependency_type_str(params.kind))
    .bind(&now)
    .bind(serde_json::to_string(&vector_clock)?)
    .execute(pool)
    .await?;

    Ok(json!({
        "dependency_id": params.dependency_id,
        "from_wp_id": params.from_wp_id,
        "to_wp_id": params.to_wp_id,
        "type": dependency_type_str(params.kind),
        "created_at": now,
    }))
}

async fn remove_dependency(
    pool: &SqlitePool,
    params: LocusRemoveDependencyParams,
) -> StorageResult<Value> {
    let result = sqlx::query("DELETE FROM dependencies WHERE dependency_id = $1")
        .bind(&params.dependency_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(StorageError::NotFound("dependency"));
    }
    Ok(json!({ "dependency_id": params.dependency_id, "deleted": true }))
}

async fn query_ready(pool: &SqlitePool, params: LocusQueryReadyParams) -> StorageResult<Value> {
    let limit = params.limit.unwrap_or(100) as i64;

    // Ready work = status=ready and no open blocking dependencies exist (type=blocks/blocked_by).
    let rows = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT wp.wp_id
        FROM work_packets wp
        WHERE wp.status = 'ready'
          AND NOT EXISTS (
            SELECT 1
            FROM dependencies d
            JOIN work_packets blocker ON blocker.wp_id = d.from_wp_id
            WHERE d.type = 'blocks'
              AND d.to_wp_id = wp.wp_id
              AND blocker.status NOT IN ('done', 'cancelled')
          )
          AND NOT EXISTS (
            SELECT 1
            FROM dependencies d
            JOIN work_packets blocker ON blocker.wp_id = d.to_wp_id
            WHERE d.type = 'blocked_by'
              AND d.from_wp_id = wp.wp_id
              AND blocker.status NOT IN ('done', 'cancelled')
          )
        ORDER BY wp.priority ASC, wp.created_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let wp_ids: Vec<String> = rows.into_iter().map(|row| row.0).collect();
    Ok(json!({ "wp_ids": wp_ids }))
}

async fn get_wp_status(pool: &SqlitePool, params: LocusGetWpStatusParams) -> StorageResult<Value> {
    let row = sqlx::query_as::<_, (i64, String, String, String)>(
        r#"
        SELECT version, status, task_board_status, updated_at
        FROM work_packets
        WHERE wp_id = $1
        "#,
    )
    .bind(&params.wp_id)
    .fetch_optional(pool)
    .await?;

    let Some((version, status, task_board_status, updated_at)) = row else {
        return Err(StorageError::NotFound("work_packet"));
    };

    Ok(json!({
        "wp_id": params.wp_id,
        "version": version,
        "status": status,
        "task_board_status": task_board_status,
        "updated_at": updated_at,
    }))
}

async fn get_mt_progress(
    pool: &SqlitePool,
    params: LocusGetMtProgressParams,
) -> StorageResult<Value> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            String,
            String,
            Option<i64>,
            Option<i64>,
            String,
        ),
    >(
        r#"
        SELECT mt_id, wp_id, name, status, current_iteration, escalation_level, metadata
        FROM micro_tasks
        WHERE mt_id = $1
        "#,
    )
    .bind(&params.mt_id)
    .fetch_optional(pool)
    .await?;

    let Some((mt_id, wp_id, name, status, current_iteration, escalation_level, metadata)) = row
    else {
        return Err(StorageError::NotFound("micro_task"));
    };

    let metadata_json: Value = serde_json::from_str(&metadata).unwrap_or_else(|_| json!({}));

    Ok(json!({
        "mt_id": mt_id,
        "wp_id": wp_id,
        "name": name,
        "status": status,
        "current_iteration": current_iteration,
        "escalation_level": escalation_level,
        "metadata": metadata_json,
    }))
}

async fn execute_sqlite_locus_operation(
    sqlite: &SqliteDatabase,
    op: LocusOperation,
) -> StorageResult<Value> {
    let pool = sqlite.pool();
    match op {
        LocusOperation::CreateWp(params) => create_wp(pool, params).await,
        LocusOperation::UpdateWp(params) => update_wp(pool, params).await,
        LocusOperation::GateWp(params) => gate_wp(pool, params).await,
        LocusOperation::CloseWp(params) => close_wp(pool, params).await,
        LocusOperation::DeleteWp(params) => delete_wp(pool, params).await,
        LocusOperation::RegisterMts(params) => register_mts(pool, params).await,
        LocusOperation::StartMt(params) => start_mt(pool, params).await,
        LocusOperation::RecordIteration(params) => record_iteration(pool, params).await,
        LocusOperation::CompleteMt(params) => complete_mt(pool, params).await,
        LocusOperation::AddDependency(params) => add_dependency(pool, params).await,
        LocusOperation::RemoveDependency(params) => remove_dependency(pool, params).await,
        LocusOperation::QueryReady(params) => query_ready(pool, params).await,
        LocusOperation::GetWpStatus(params) => get_wp_status(pool, params).await,
        LocusOperation::GetMtProgress(params) => get_mt_progress(pool, params).await,
        LocusOperation::SyncTaskBoard(_params) => Err(StorageError::NotImplemented(
            "locus_sync_task_board not implemented yet",
        )),
    }
}
