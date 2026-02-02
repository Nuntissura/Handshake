use super::{
    validate_job_contract, AccessMode, AiJob, AiJobListFilter, Block, BlockUpdate, BronzeRecord,
    Canvas, CanvasEdge, CanvasGraph, CanvasNode, DefaultStorageGuard, Document,
    EmbeddingModelRecord, EmbeddingRegistry, EntityRef, JobKind, JobMetrics, JobState,
    JobStatusUpdate, MutationMetadata, NewAiJob, NewBlock, NewBronzeRecord, NewCanvas,
    NewCanvasEdge, NewCanvasNode, NewDocument, NewNodeExecution, NewSilverRecord, NewWorkspace,
    PlannedOperation, SafetyMode, SilverRecord, StorageError, StorageGuard, StorageResult,
    WorkflowNodeExecution, WorkflowRun, Workspace, WriteContext,
};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use serde_json::Value;
use sqlx::{
    postgres::{PgPool, PgPoolOptions, PgRow},
    Row,
};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

pub struct PostgresDatabase {
    pool: PgPool,
    guard: Arc<dyn StorageGuard>,
}

#[cfg(test)]
impl PostgresDatabase {
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl PostgresDatabase {
    pub async fn connect(db_url: &str, max_connections: u32) -> StorageResult<Self> {
        let guard: Arc<dyn StorageGuard> = Arc::new(DefaultStorageGuard);
        Self::connect_with_guard(db_url, max_connections, guard).await
    }

    pub async fn connect_with_guard(
        db_url: &str,
        max_connections: u32,
        guard: Arc<dyn StorageGuard>,
    ) -> StorageResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(db_url)
            .await?;
        Ok(Self { pool, guard })
    }

    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            guard: Arc::new(DefaultStorageGuard),
        }
    }

    pub fn into_arc(self) -> Arc<dyn super::Database> {
        Arc::new(self)
    }
}

fn map_workspace(row: PgRow) -> Workspace {
    Workspace {
        id: row.get("id"),
        name: row.get("name"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_document(row: PgRow) -> Document {
    Document {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_canvas(row: PgRow) -> Canvas {
    Canvas {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_canvas_edge(row: PgRow) -> CanvasEdge {
    CanvasEdge {
        id: row.get("id"),
        canvas_id: row.get("canvas_id"),
        from_node_id: row.get("from_node_id"),
        to_node_id: row.get("to_node_id"),
        kind: row.get("kind"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    }
}

fn map_block(row: PgRow) -> StorageResult<Block> {
    let derived_raw: String = row.get("derived_content");
    let derived = serde_json::from_str(&derived_raw)?;
    let exportable_int: Option<i32> = row.get("exportable");
    Ok(Block {
        id: row.get("id"),
        document_id: row.get("document_id"),
        kind: row.get("kind"),
        sequence: map_i64_from_i32(&row, "sequence"),
        raw_content: row.get("raw_content"),
        display_content: row.get("display_content"),
        derived_content: derived,
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
        sensitivity: row.get("sensitivity"),
        exportable: exportable_int.map(|v| v != 0),
    })
}

fn map_canvas_node(row: PgRow) -> StorageResult<CanvasNode> {
    let data_raw: String = row.get("data");
    let data = serde_json::from_str(&data_raw)?;
    Ok(CanvasNode {
        id: row.get("id"),
        canvas_id: row.get("canvas_id"),
        kind: row.get("kind"),
        position_x: map_f64_from_f32(&row, "position_x"),
        position_y: map_f64_from_f32(&row, "position_y"),
        data,
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_ai_job(row: PgRow) -> StorageResult<AiJob> {
    let job_inputs = row
        .get::<Option<String>, _>("job_inputs")
        .map(|val| serde_json::from_str::<Value>(&val))
        .transpose()?;
    let job_outputs = row
        .get::<Option<String>, _>("job_outputs")
        .map(|val| serde_json::from_str::<Value>(&val))
        .transpose()?;

    Ok(AiJob {
        job_id: Uuid::parse_str(row.get::<String, _>("id").as_str())
            .map_err(|_| StorageError::Validation("invalid job_id uuid"))?,
        trace_id: Uuid::parse_str(row.get::<String, _>("trace_id").as_str())
            .map_err(|_| StorageError::Validation("invalid trace_id uuid"))?,
        workflow_run_id: row
            .get::<Option<String>, _>("workflow_run_id")
            .map(|id| Uuid::parse_str(&id))
            .transpose()
            .map_err(|_| StorageError::Validation("invalid workflow_run_id uuid"))?,
        job_kind: JobKind::from_str(row.get::<String, _>("job_kind").as_str())?,
        state: JobState::try_from(row.get::<String, _>("status").as_str())?,
        error_message: row.get("error_message"),
        protocol_id: row.get("protocol_id"),
        profile_id: row.get("profile_id"),
        capability_profile_id: row.get("capability_profile_id"),
        access_mode: AccessMode::try_from(row.get::<String, _>("access_mode").as_str())?,
        safety_mode: SafetyMode::try_from(row.get::<String, _>("safety_mode").as_str())?,
        entity_refs: serde_json::from_str::<Vec<EntityRef>>(&row.get::<String, _>("entity_refs"))?,
        planned_operations: serde_json::from_str::<Vec<PlannedOperation>>(
            &row.get::<String, _>("planned_operations"),
        )?,
        metrics: serde_json::from_str::<JobMetrics>(&row.get::<String, _>("metrics"))?,
        status_reason: row.get("status_reason"),
        job_inputs,
        job_outputs,
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_workflow_run(row: PgRow) -> StorageResult<WorkflowRun> {
    Ok(WorkflowRun {
        id: Uuid::parse_str(row.get::<String, _>("id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run id"))?,
        job_id: Uuid::parse_str(row.get::<String, _>("job_id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run job_id"))?,
        status: JobState::try_from(row.get::<String, _>("status").as_str())?,
        last_heartbeat: map_timestamp(&row, "last_heartbeat"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_workflow_node_execution(row: PgRow) -> StorageResult<WorkflowNodeExecution> {
    let input_payload = row
        .get::<Option<String>, _>("input_payload")
        .map(|val| serde_json::from_str(&val))
        .transpose()?;
    let output_payload = row
        .get::<Option<String>, _>("output_payload")
        .map(|val| serde_json::from_str(&val))
        .transpose()?;

    Ok(WorkflowNodeExecution {
        id: Uuid::parse_str(row.get::<String, _>("id").as_str())
            .map_err(|_| StorageError::Validation("invalid node execution id"))?,
        workflow_run_id: Uuid::parse_str(row.get::<String, _>("workflow_run_id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run_id"))?,
        node_id: row.get("node_id"),
        node_type: row.get("node_type"),
        status: JobState::try_from(row.get::<String, _>("status").as_str())?,
        sequence: map_i64_from_i32(&row, "sequence"),
        input_payload,
        output_payload,
        error_message: row.get("error_message"),
        started_at: map_timestamp(&row, "started_at"),
        finished_at: map_optional_timestamp(&row, "finished_at"),
        created_at: map_timestamp(&row, "created_at"),
        updated_at: map_timestamp(&row, "updated_at"),
    })
}

fn map_timestamp(row: &PgRow, column: &str) -> chrono::DateTime<Utc> {
    let value: NaiveDateTime = row.get(column);
    value.and_utc()
}

fn map_optional_timestamp(row: &PgRow, column: &str) -> Option<chrono::DateTime<Utc>> {
    row.get::<Option<NaiveDateTime>, _>(column)
        .map(|value| value.and_utc())
}

fn map_i64_from_i32(row: &PgRow, column: &str) -> i64 {
    let value: i32 = row.get(column);
    value as i64
}

fn map_f64_from_f32(row: &PgRow, column: &str) -> f64 {
    let value: f32 = row.get(column);
    value as f64
}

#[async_trait]
impl super::Database for PostgresDatabase {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn run_migrations(&self) -> StorageResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    async fn migration_version(&self) -> StorageResult<i64> {
        let version = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = TRUE",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(version)
    }

    async fn ping(&self) -> StorageResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>> {
        let rows = sqlx::query(
            r#"SELECT id, name, created_at, updated_at FROM workspaces ORDER BY created_at ASC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_workspace).collect())
    }

    async fn create_workspace(
        &self,
        ctx: &WriteContext,
        workspace: NewWorkspace,
    ) -> StorageResult<Workspace> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO workspaces (
                id,
                name,
                created_at,
                updated_at,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, name, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&workspace.name)
        .bind(now)
        .bind(now)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_workspace(row))
    }

    async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, id).await?;
        let res = sqlx::query(r#"DELETE FROM workspaces WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("workspace"));
        }
        Ok(())
    }

    async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>> {
        let row =
            sqlx::query(r#"SELECT id, name, created_at, updated_at FROM workspaces WHERE id = $1"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(map_workspace))
    }

    async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM documents
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_document).collect())
    }

    async fn get_document(&self, doc_id: &str) -> StorageResult<Document> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM documents
            WHERE id = $1
            "#,
        )
        .bind(doc_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(map_document(row)),
            None => Err(StorageError::NotFound("document")),
        }
    }

    async fn create_document(
        &self,
        ctx: &WriteContext,
        doc: NewDocument,
    ) -> StorageResult<Document> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO documents (
                id,
                workspace_id,
                title,
                created_at,
                updated_at,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, workspace_id, title, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&doc.workspace_id)
        .bind(&doc.title)
        .bind(now)
        .bind(now)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_document(row))
    }

    async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, doc_id).await?;
        let res = sqlx::query(r#"DELETE FROM documents WHERE id = $1"#)
            .bind(doc_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("document"));
        }
        Ok(())
    }

    async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT id, document_id, kind, sequence, raw_content, display_content, derived_content,
                   created_at, updated_at, sensitivity, exportable
            FROM blocks
            WHERE document_id = $1
            ORDER BY sequence ASC
            "#,
        )
        .bind(doc_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(map_block)
            .collect::<StorageResult<Vec<_>>>()
    }

    async fn get_block(&self, block_id: &str) -> StorageResult<Block> {
        let row = sqlx::query(
            r#"
            SELECT id, document_id, kind, sequence, raw_content, display_content, derived_content,
                   created_at, updated_at, sensitivity, exportable
            FROM blocks
            WHERE id = $1
            "#,
        )
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_block(row),
            None => Err(StorageError::NotFound("block")),
        }
    }

    async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block> {
        let now = Utc::now();
        let id = block.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let display_content = block
            .display_content
            .unwrap_or_else(|| block.raw_content.clone());
        let derived_content = block
            .derived_content
            .unwrap_or_else(|| Value::Object(Default::default()))
            .to_string();
        let exportable_int: Option<i32> = block.exportable.map(|v| if v { 1 } else { 0 });

        let row = sqlx::query(
            r#"
            INSERT INTO blocks (
                id,
                document_id,
                kind,
                sequence,
                raw_content,
                display_content,
                derived_content,
                created_at,
                updated_at,
                sensitivity,
                exportable,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING id, document_id, kind, sequence, raw_content, display_content, derived_content,
                      created_at, updated_at, sensitivity, exportable
            "#,
        )
        .bind(&id)
        .bind(&block.document_id)
        .bind(&block.kind)
        .bind(block.sequence)
        .bind(&block.raw_content)
        .bind(&display_content)
        .bind(&derived_content)
        .bind(now)
        .bind(now)
        .bind(&block.sensitivity)
        .bind(exportable_int)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        map_block(row)
    }

    async fn update_block(
        &self,
        ctx: &WriteContext,
        block_id: &str,
        data: BlockUpdate,
    ) -> StorageResult<()> {
        if data.kind.is_none()
            && data.sequence.is_none()
            && data.raw_content.is_none()
            && data.display_content.is_none()
            && data.derived_content.is_none()
        {
            return Err(StorageError::Validation("no block fields provided"));
        }

        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let mut block = self.get_block(block_id).await?;

        if let Some(kind) = data.kind {
            block.kind = kind;
        }
        if let Some(seq) = data.sequence {
            block.sequence = seq;
        }
        if let Some(raw) = data.raw_content {
            block.raw_content = raw.clone();
            if block.display_content.is_empty() {
                block.display_content = raw;
            }
        }
        if let Some(display) = data.display_content {
            block.display_content = display;
        }
        if let Some(derived) = data.derived_content {
            block.derived_content = derived;
        }

        let derived_content = block.derived_content.to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE blocks
            SET kind = $1,
                sequence = $2,
                raw_content = $3,
                display_content = $4,
                derived_content = $5,
                last_actor_kind = $6,
                last_actor_id = $7,
                last_job_id = $8,
                last_workflow_id = $9,
                edit_event_id = $10,
                updated_at = $11
            WHERE id = $12
            "#,
        )
        .bind(&block.kind)
        .bind(block.sequence)
        .bind(&block.raw_content)
        .bind(&block.display_content)
        .bind(&derived_content)
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(&block.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, block_id).await?;
        let res = sqlx::query(r#"DELETE FROM blocks WHERE id = $1"#)
            .bind(block_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("block"));
        }
        Ok(())
    }

    async fn replace_blocks(
        &self,
        ctx: &WriteContext,
        document_id: &str,
        blocks: Vec<NewBlock>,
    ) -> StorageResult<Vec<Block>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(r#"DELETE FROM blocks WHERE document_id = $1"#)
            .bind(document_id)
            .execute(&mut *tx)
            .await?;

        let mut inserted = Vec::with_capacity(blocks.len());
        for block in blocks {
            let now = Utc::now();
            let id = block.id.unwrap_or_else(|| Uuid::new_v4().to_string());
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let job_id = metadata.job_id.map(|v| v.to_string());
            let workflow_id = metadata.workflow_id.map(|v| v.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();
            let display_content = block
                .display_content
                .unwrap_or_else(|| block.raw_content.clone());
            let derived_content = block
                .derived_content
                .unwrap_or_else(|| Value::Object(Default::default()))
                .to_string();
            let exportable_int: Option<i32> = block.exportable.map(|v| if v { 1 } else { 0 });

            let row = sqlx::query(
                r#"
                INSERT INTO blocks (
                    id,
                    document_id,
                    kind,
                    sequence,
                    raw_content,
                    display_content,
                    derived_content,
                    created_at,
                    updated_at,
                    sensitivity,
                    exportable,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                RETURNING id, document_id, kind, sequence, raw_content, display_content, derived_content,
                          created_at, updated_at, sensitivity, exportable
                "#,
            )
            .bind(&id)
            .bind(document_id)
            .bind(&block.kind)
            .bind(block.sequence)
            .bind(&block.raw_content)
            .bind(&display_content)
            .bind(&derived_content)
            .bind(now)
            .bind(now)
            .bind(&block.sensitivity)
            .bind(exportable_int)
            .bind(actor_kind)
            .bind(&actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .fetch_one(&mut *tx)
            .await?;

            inserted.push(map_block(row)?);
        }

        let doc_metadata = self.guard.validate_write(ctx, document_id).await?;
        let doc_actor_kind = doc_metadata.actor_kind.as_str();
        let doc_actor_id = doc_metadata.actor_id.clone();
        let doc_job_id = doc_metadata.job_id.map(|v| v.to_string());
        let doc_workflow_id = doc_metadata.workflow_id.map(|v| v.to_string());
        let doc_edit_event_id = doc_metadata.edit_event_id.to_string();
        let doc_updated_at = doc_metadata.timestamp;

        let updated = sqlx::query(
            r#"
            UPDATE documents
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5,
                updated_at = $6
            WHERE id = $7
            "#,
        )
        .bind(doc_actor_kind)
        .bind(doc_actor_id)
        .bind(doc_job_id)
        .bind(doc_workflow_id)
        .bind(doc_edit_event_id)
        .bind(doc_updated_at)
        .bind(document_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(StorageError::NotFound("document"));
        }

        tx.commit().await?;
        Ok(inserted)
    }

    async fn create_canvas(&self, ctx: &WriteContext, canvas: NewCanvas) -> StorageResult<Canvas> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let job_id = metadata.job_id.map(|v| v.to_string());
        let workflow_id = metadata.workflow_id.map(|v| v.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let row = sqlx::query(
            r#"
            INSERT INTO canvases (
                id,
                workspace_id,
                title,
                created_at,
                updated_at,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, workspace_id, title, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&canvas.workspace_id)
        .bind(&canvas.title)
        .bind(now)
        .bind(now)
        .bind(actor_kind)
        .bind(&actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(map_canvas(row))
    }

    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM canvases
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_canvas).collect())
    }

    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph> {
        let canvas_row = sqlx::query(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at
            FROM canvases
            WHERE id = $1
            "#,
        )
        .bind(canvas_id)
        .fetch_optional(&self.pool)
        .await?;

        let canvas_row = match canvas_row {
            Some(row) => row,
            None => return Err(StorageError::NotFound("canvas")),
        };

        let nodes = sqlx::query(
            r#"
            SELECT id, canvas_id, kind, position_x, position_y, data, created_at, updated_at
            FROM canvas_nodes
            WHERE canvas_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(canvas_id)
        .fetch_all(&self.pool)
        .await?;

        let edges = sqlx::query(
            r#"
            SELECT id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at
            FROM canvas_edges
            WHERE canvas_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(canvas_id)
        .fetch_all(&self.pool)
        .await?;

        let parsed_nodes = nodes
            .into_iter()
            .map(map_canvas_node)
            .collect::<StorageResult<Vec<_>>>()?;
        let parsed_edges = edges.into_iter().map(map_canvas_edge).collect::<Vec<_>>();

        Ok(CanvasGraph {
            canvas: map_canvas(canvas_row),
            nodes: parsed_nodes,
            edges: parsed_edges,
        })
    }

    async fn update_canvas_graph(
        &self,
        ctx: &WriteContext,
        canvas_id: &str,
        nodes: Vec<NewCanvasNode>,
        edges: Vec<NewCanvasEdge>,
    ) -> StorageResult<CanvasGraph> {
        let canvas_metadata = self.guard.validate_write(ctx, canvas_id).await?;
        let mut tx = self.pool.begin().await?;

        let canvas_row = sqlx::query(
            r#"SELECT id, workspace_id, title, created_at, updated_at FROM canvases WHERE id = $1"#,
        )
        .bind(canvas_id)
        .fetch_optional(&mut *tx)
        .await?;

        let canvas_row = match canvas_row {
            Some(row) => row,
            None => return Err(StorageError::NotFound("canvas")),
        };

        sqlx::query(r#"DELETE FROM canvas_edges WHERE canvas_id = $1"#)
            .bind(canvas_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"DELETE FROM canvas_nodes WHERE canvas_id = $1"#)
            .bind(canvas_id)
            .execute(&mut *tx)
            .await?;

        let now = Utc::now();
        let mut inserted_nodes = Vec::with_capacity(nodes.len());
        for node in nodes {
            let id = node.id.unwrap_or_else(|| Uuid::new_v4().to_string());
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let job_id = metadata.job_id.map(|v| v.to_string());
            let workflow_id = metadata.workflow_id.map(|v| v.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();
            let data = node
                .data
                .unwrap_or_else(|| Value::Object(Default::default()))
                .to_string();

            let row = sqlx::query(
                r#"
                INSERT INTO canvas_nodes (
                    id,
                    canvas_id,
                    kind,
                    position_x,
                    position_y,
                    data,
                    created_at,
                    updated_at,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                RETURNING id, canvas_id, kind, position_x, position_y, data, created_at, updated_at
                "#,
            )
            .bind(&id)
            .bind(canvas_id)
            .bind(&node.kind)
            .bind(node.position_x)
            .bind(node.position_y)
            .bind(&data)
            .bind(now)
            .bind(now)
            .bind(actor_kind)
            .bind(&actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .fetch_one(&mut *tx)
            .await?;

            inserted_nodes.push(map_canvas_node(row)?);
        }

        let mut inserted_edges = Vec::with_capacity(edges.len());
        for edge in edges {
            let id = edge.id.unwrap_or_else(|| Uuid::new_v4().to_string());
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let job_id = metadata.job_id.map(|v| v.to_string());
            let workflow_id = metadata.workflow_id.map(|v| v.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();

            let row = sqlx::query(
                r#"
                INSERT INTO canvas_edges (
                    id,
                    canvas_id,
                    from_node_id,
                    to_node_id,
                    kind,
                    created_at,
                    updated_at,
                    last_actor_kind,
                    last_actor_id,
                    last_job_id,
                    last_workflow_id,
                    edit_event_id
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                RETURNING id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at
                "#,
            )
            .bind(&id)
            .bind(canvas_id)
            .bind(&edge.from_node_id)
            .bind(&edge.to_node_id)
            .bind(&edge.kind)
            .bind(now)
            .bind(now)
            .bind(actor_kind)
            .bind(&actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .fetch_one(&mut *tx)
            .await?;

            inserted_edges.push(map_canvas_edge(row));
        }

        let canvas_actor_kind = canvas_metadata.actor_kind.as_str();
        let canvas_actor_id = canvas_metadata.actor_id.clone();
        let canvas_job_id = canvas_metadata.job_id.map(|v| v.to_string());
        let canvas_workflow_id = canvas_metadata.workflow_id.map(|v| v.to_string());
        let canvas_edit_event_id = canvas_metadata.edit_event_id.to_string();
        let canvas_updated_at = canvas_metadata.timestamp;

        let updated = sqlx::query(
            r#"
            UPDATE canvases
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5,
                updated_at = $6
            WHERE id = $7
            "#,
        )
        .bind(canvas_actor_kind)
        .bind(canvas_actor_id)
        .bind(canvas_job_id)
        .bind(canvas_workflow_id)
        .bind(canvas_edit_event_id)
        .bind(canvas_updated_at)
        .bind(canvas_id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(StorageError::NotFound("canvas"));
        }

        tx.commit().await?;

        let mut canvas = map_canvas(canvas_row);
        canvas.updated_at = canvas_updated_at;

        Ok(CanvasGraph {
            canvas,
            nodes: inserted_nodes,
            edges: inserted_edges,
        })
    }

    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, canvas_id).await?;
        let res = sqlx::query(r#"DELETE FROM canvases WHERE id = $1"#)
            .bind(canvas_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("canvas"));
        }
        Ok(())
    }

    async fn create_ai_bronze_record(
        &self,
        ctx: &WriteContext,
        record: NewBronzeRecord,
    ) -> StorageResult<BronzeRecord> {
        let now = Utc::now();
        self.guard.validate_write(ctx, &record.bronze_id).await?;

        let row = sqlx::query(
            r#"
            INSERT INTO ai_bronze_records (
                bronze_id, workspace_id, content_hash, content_type, content_encoding, size_bytes,
                original_filename, artifact_path, ingested_at, ingestion_source_type, ingestion_source_id,
                ingestion_method, external_source_json, is_deleted, deleted_at, retention_policy
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,0,NULL,$14)
            RETURNING
                bronze_id,
                workspace_id,
                content_hash,
                content_type,
                content_encoding,
                size_bytes,
                original_filename,
                artifact_path,
                ingested_at,
                ingestion_source_type,
                ingestion_source_id,
                ingestion_method,
                external_source_json,
                is_deleted,
                deleted_at,
                retention_policy
            "#,
        )
        .bind(record.bronze_id)
        .bind(record.workspace_id)
        .bind(record.content_hash)
        .bind(record.content_type)
        .bind(record.content_encoding)
        .bind(record.size_bytes as i64)
        .bind(record.original_filename)
        .bind(record.artifact_path)
        .bind(now.naive_utc())
        .bind(record.ingestion_source_type.as_str())
        .bind(record.ingestion_source_id)
        .bind(record.ingestion_method)
        .bind(record.external_source_json)
        .bind(record.retention_policy)
        .fetch_one(&self.pool)
        .await?;

        Ok(BronzeRecord {
            bronze_id: row.get("bronze_id"),
            workspace_id: row.get("workspace_id"),
            content_hash: row.get("content_hash"),
            content_type: row.get("content_type"),
            content_encoding: row.get("content_encoding"),
            size_bytes: row.get::<i64, _>("size_bytes") as u64,
            original_filename: row.get("original_filename"),
            artifact_path: row.get("artifact_path"),
            ingested_at: map_timestamp(&row, "ingested_at"),
            ingestion_source_type: crate::ai_ready_data::records::IngestionSourceType::from_str(
                row.get::<String, _>("ingestion_source_type").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
            ingestion_source_id: row.get("ingestion_source_id"),
            ingestion_method: row.get("ingestion_method"),
            external_source_json: row.get("external_source_json"),
            is_deleted: map_i64_from_i32(&row, "is_deleted") != 0,
            deleted_at: map_optional_timestamp(&row, "deleted_at"),
            retention_policy: row.get("retention_policy"),
        })
    }

    async fn get_ai_bronze_record(&self, bronze_id: &str) -> StorageResult<Option<BronzeRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                bronze_id,
                workspace_id,
                content_hash,
                content_type,
                content_encoding,
                size_bytes,
                original_filename,
                artifact_path,
                ingested_at,
                ingestion_source_type,
                ingestion_source_id,
                ingestion_method,
                external_source_json,
                is_deleted,
                deleted_at,
                retention_policy
            FROM ai_bronze_records
            WHERE bronze_id = $1
            "#,
        )
        .bind(bronze_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(BronzeRecord {
            bronze_id: row.get("bronze_id"),
            workspace_id: row.get("workspace_id"),
            content_hash: row.get("content_hash"),
            content_type: row.get("content_type"),
            content_encoding: row.get("content_encoding"),
            size_bytes: row.get::<i64, _>("size_bytes") as u64,
            original_filename: row.get("original_filename"),
            artifact_path: row.get("artifact_path"),
            ingested_at: map_timestamp(&row, "ingested_at"),
            ingestion_source_type: crate::ai_ready_data::records::IngestionSourceType::from_str(
                row.get::<String, _>("ingestion_source_type").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
            ingestion_source_id: row.get("ingestion_source_id"),
            ingestion_method: row.get("ingestion_method"),
            external_source_json: row.get("external_source_json"),
            is_deleted: map_i64_from_i32(&row, "is_deleted") != 0,
            deleted_at: map_optional_timestamp(&row, "deleted_at"),
            retention_policy: row.get("retention_policy"),
        }))
    }

    async fn list_ai_bronze_records(&self, workspace_id: &str) -> StorageResult<Vec<BronzeRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                bronze_id,
                workspace_id,
                content_hash,
                content_type,
                content_encoding,
                size_bytes,
                original_filename,
                artifact_path,
                ingested_at,
                ingestion_source_type,
                ingestion_source_id,
                ingestion_method,
                external_source_json,
                is_deleted,
                deleted_at,
                retention_policy
            FROM ai_bronze_records
            WHERE workspace_id = $1
            ORDER BY ingested_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(BronzeRecord {
                bronze_id: row.get("bronze_id"),
                workspace_id: row.get("workspace_id"),
                content_hash: row.get("content_hash"),
                content_type: row.get("content_type"),
                content_encoding: row.get("content_encoding"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                original_filename: row.get("original_filename"),
                artifact_path: row.get("artifact_path"),
                ingested_at: map_timestamp(&row, "ingested_at"),
                ingestion_source_type:
                    crate::ai_ready_data::records::IngestionSourceType::from_str(
                        row.get::<String, _>("ingestion_source_type").as_str(),
                    )
                    .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
                ingestion_source_id: row.get("ingestion_source_id"),
                ingestion_method: row.get("ingestion_method"),
                external_source_json: row.get("external_source_json"),
                is_deleted: map_i64_from_i32(&row, "is_deleted") != 0,
                deleted_at: map_optional_timestamp(&row, "deleted_at"),
                retention_policy: row.get("retention_policy"),
            });
        }

        Ok(out)
    }

    async fn mark_ai_bronze_deleted(
        &self,
        ctx: &WriteContext,
        bronze_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, bronze_id).await?;
        let now = Utc::now();
        let res = sqlx::query(
            r#"
            UPDATE ai_bronze_records
            SET is_deleted = 1, deleted_at = $2
            WHERE bronze_id = $1
            "#,
        )
        .bind(bronze_id)
        .bind(now.naive_utc())
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("ai_bronze_record"));
        }
        Ok(())
    }

    async fn create_ai_silver_record(
        &self,
        ctx: &WriteContext,
        record: NewSilverRecord,
    ) -> StorageResult<SilverRecord> {
        self.guard.validate_write(ctx, &record.silver_id).await?;
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO ai_silver_records (
                silver_id, workspace_id, bronze_ref, chunk_index, total_chunks, token_count,
                content_hash, byte_start, byte_end, line_start, line_end,
                chunk_artifact_path, embedding_artifact_path, embedding_model_id, embedding_model_version,
                embedding_dimensions, embedding_compute_latency_ms,
                chunking_strategy, chunking_version, processing_pipeline_version,
                processed_at, processing_duration_ms, metadata_json,
                validation_status, validation_failed_checks_json, validated_at, validator_version,
                is_current, superseded_by, created_at
            )
            VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,
                $12,$13,$14,$15,$16,$17,
                $18,$19,$20,
                $21,$22,$23,
                $24,$25,$26,$27,
                1,NULL,$28
            )
            RETURNING
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            "#,
        )
        .bind(record.silver_id)
        .bind(record.workspace_id)
        .bind(record.bronze_ref)
        .bind(record.chunk_index as i32)
        .bind(record.total_chunks as i32)
        .bind(record.token_count as i32)
        .bind(record.content_hash)
        .bind(record.byte_start as i64)
        .bind(record.byte_end as i64)
        .bind(record.line_start as i32)
        .bind(record.line_end as i32)
        .bind(record.chunk_artifact_path)
        .bind(record.embedding_artifact_path)
        .bind(record.embedding_model_id)
        .bind(record.embedding_model_version)
        .bind(record.embedding_dimensions as i32)
        .bind(record.embedding_compute_latency_ms as i64)
        .bind(record.chunking_strategy)
        .bind(record.chunking_version)
        .bind(record.processing_pipeline_version)
        .bind(now.naive_utc())
        .bind(record.processing_duration_ms as i64)
        .bind(record.metadata_json)
        .bind(record.validation_status.as_str())
        .bind(record.validation_failed_checks_json)
        .bind(now.naive_utc())
        .bind(record.validator_version)
        .bind(now.naive_utc())
        .fetch_one(&self.pool)
        .await?;

        Ok(SilverRecord {
            silver_id: row.get("silver_id"),
            workspace_id: row.get("workspace_id"),
            bronze_ref: row.get("bronze_ref"),
            chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
            total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
            token_count: map_i64_from_i32(&row, "token_count") as u32,
            content_hash: row.get("content_hash"),
            byte_start: row.get::<i64, _>("byte_start") as u64,
            byte_end: row.get::<i64, _>("byte_end") as u64,
            line_start: map_i64_from_i32(&row, "line_start") as u32,
            line_end: map_i64_from_i32(&row, "line_end") as u32,
            chunk_artifact_path: row.get("chunk_artifact_path"),
            embedding_artifact_path: row.get("embedding_artifact_path"),
            embedding_model_id: row.get("embedding_model_id"),
            embedding_model_version: row.get("embedding_model_version"),
            embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
            embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms") as u64,
            chunking_strategy: row.get("chunking_strategy"),
            chunking_version: row.get("chunking_version"),
            processing_pipeline_version: row.get("processing_pipeline_version"),
            processed_at: map_timestamp(&row, "processed_at"),
            processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
            metadata_json: row.get("metadata_json"),
            validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                row.get::<String, _>("validation_status").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid validation_status"))?,
            validation_failed_checks_json: row.get("validation_failed_checks_json"),
            validated_at: map_timestamp(&row, "validated_at"),
            validator_version: row.get("validator_version"),
            is_current: map_i64_from_i32(&row, "is_current") != 0,
            superseded_by: row.get("superseded_by"),
            created_at: map_timestamp(&row, "created_at"),
        })
    }

    async fn get_ai_silver_record(&self, silver_id: &str) -> StorageResult<Option<SilverRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            FROM ai_silver_records
            WHERE silver_id = $1
            "#,
        )
        .bind(silver_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(SilverRecord {
            silver_id: row.get("silver_id"),
            workspace_id: row.get("workspace_id"),
            bronze_ref: row.get("bronze_ref"),
            chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
            total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
            token_count: map_i64_from_i32(&row, "token_count") as u32,
            content_hash: row.get("content_hash"),
            byte_start: row.get::<i64, _>("byte_start") as u64,
            byte_end: row.get::<i64, _>("byte_end") as u64,
            line_start: map_i64_from_i32(&row, "line_start") as u32,
            line_end: map_i64_from_i32(&row, "line_end") as u32,
            chunk_artifact_path: row.get("chunk_artifact_path"),
            embedding_artifact_path: row.get("embedding_artifact_path"),
            embedding_model_id: row.get("embedding_model_id"),
            embedding_model_version: row.get("embedding_model_version"),
            embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
            embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms") as u64,
            chunking_strategy: row.get("chunking_strategy"),
            chunking_version: row.get("chunking_version"),
            processing_pipeline_version: row.get("processing_pipeline_version"),
            processed_at: map_timestamp(&row, "processed_at"),
            processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
            metadata_json: row.get("metadata_json"),
            validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                row.get::<String, _>("validation_status").as_str(),
            )
            .map_err(|_| StorageError::Validation("invalid validation_status"))?,
            validation_failed_checks_json: row.get("validation_failed_checks_json"),
            validated_at: map_timestamp(&row, "validated_at"),
            validator_version: row.get("validator_version"),
            is_current: map_i64_from_i32(&row, "is_current") != 0,
            superseded_by: row.get("superseded_by"),
            created_at: map_timestamp(&row, "created_at"),
        }))
    }

    async fn list_ai_silver_records_by_bronze(
        &self,
        bronze_id: &str,
    ) -> StorageResult<Vec<SilverRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            FROM ai_silver_records
            WHERE bronze_ref = $1
            ORDER BY chunk_index ASC
            "#,
        )
        .bind(bronze_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(SilverRecord {
                silver_id: row.get("silver_id"),
                workspace_id: row.get("workspace_id"),
                bronze_ref: row.get("bronze_ref"),
                chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
                total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
                token_count: map_i64_from_i32(&row, "token_count") as u32,
                content_hash: row.get("content_hash"),
                byte_start: row.get::<i64, _>("byte_start") as u64,
                byte_end: row.get::<i64, _>("byte_end") as u64,
                line_start: map_i64_from_i32(&row, "line_start") as u32,
                line_end: map_i64_from_i32(&row, "line_end") as u32,
                chunk_artifact_path: row.get("chunk_artifact_path"),
                embedding_artifact_path: row.get("embedding_artifact_path"),
                embedding_model_id: row.get("embedding_model_id"),
                embedding_model_version: row.get("embedding_model_version"),
                embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
                embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms")
                    as u64,
                chunking_strategy: row.get("chunking_strategy"),
                chunking_version: row.get("chunking_version"),
                processing_pipeline_version: row.get("processing_pipeline_version"),
                processed_at: map_timestamp(&row, "processed_at"),
                processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
                metadata_json: row.get("metadata_json"),
                validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                    row.get::<String, _>("validation_status").as_str(),
                )
                .map_err(|_| StorageError::Validation("invalid validation_status"))?,
                validation_failed_checks_json: row.get("validation_failed_checks_json"),
                validated_at: map_timestamp(&row, "validated_at"),
                validator_version: row.get("validator_version"),
                is_current: map_i64_from_i32(&row, "is_current") != 0,
                superseded_by: row.get("superseded_by"),
                created_at: map_timestamp(&row, "created_at"),
            });
        }

        Ok(out)
    }

    async fn list_ai_silver_records(&self, workspace_id: &str) -> StorageResult<Vec<SilverRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                silver_id,
                workspace_id,
                bronze_ref,
                chunk_index,
                total_chunks,
                token_count,
                content_hash,
                byte_start,
                byte_end,
                line_start,
                line_end,
                chunk_artifact_path,
                embedding_artifact_path,
                embedding_model_id,
                embedding_model_version,
                embedding_dimensions,
                embedding_compute_latency_ms,
                chunking_strategy,
                chunking_version,
                processing_pipeline_version,
                processed_at,
                processing_duration_ms,
                metadata_json,
                validation_status,
                validation_failed_checks_json,
                validated_at,
                validator_version,
                is_current,
                superseded_by,
                created_at
            FROM ai_silver_records
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(SilverRecord {
                silver_id: row.get("silver_id"),
                workspace_id: row.get("workspace_id"),
                bronze_ref: row.get("bronze_ref"),
                chunk_index: map_i64_from_i32(&row, "chunk_index") as u32,
                total_chunks: map_i64_from_i32(&row, "total_chunks") as u32,
                token_count: map_i64_from_i32(&row, "token_count") as u32,
                content_hash: row.get("content_hash"),
                byte_start: row.get::<i64, _>("byte_start") as u64,
                byte_end: row.get::<i64, _>("byte_end") as u64,
                line_start: map_i64_from_i32(&row, "line_start") as u32,
                line_end: map_i64_from_i32(&row, "line_end") as u32,
                chunk_artifact_path: row.get("chunk_artifact_path"),
                embedding_artifact_path: row.get("embedding_artifact_path"),
                embedding_model_id: row.get("embedding_model_id"),
                embedding_model_version: row.get("embedding_model_version"),
                embedding_dimensions: map_i64_from_i32(&row, "embedding_dimensions") as u32,
                embedding_compute_latency_ms: row.get::<i64, _>("embedding_compute_latency_ms")
                    as u64,
                chunking_strategy: row.get("chunking_strategy"),
                chunking_version: row.get("chunking_version"),
                processing_pipeline_version: row.get("processing_pipeline_version"),
                processed_at: map_timestamp(&row, "processed_at"),
                processing_duration_ms: row.get::<i64, _>("processing_duration_ms") as u64,
                metadata_json: row.get("metadata_json"),
                validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                    row.get::<String, _>("validation_status").as_str(),
                )
                .map_err(|_| StorageError::Validation("invalid validation_status"))?,
                validation_failed_checks_json: row.get("validation_failed_checks_json"),
                validated_at: map_timestamp(&row, "validated_at"),
                validator_version: row.get("validator_version"),
                is_current: map_i64_from_i32(&row, "is_current") != 0,
                superseded_by: row.get("superseded_by"),
                created_at: map_timestamp(&row, "created_at"),
            });
        }

        Ok(out)
    }

    async fn supersede_ai_silver_record(
        &self,
        ctx: &WriteContext,
        superseded_silver_id: &str,
        new_silver_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, superseded_silver_id).await?;
        self.guard.validate_write(ctx, new_silver_id).await?;

        let res = sqlx::query(
            r#"
            UPDATE ai_silver_records
            SET is_current = 0, superseded_by = $2
            WHERE silver_id = $1
            "#,
        )
        .bind(superseded_silver_id)
        .bind(new_silver_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("ai_silver_record"));
        }

        Ok(())
    }

    async fn upsert_ai_embedding_model(
        &self,
        ctx: &WriteContext,
        model: EmbeddingModelRecord,
    ) -> StorageResult<()> {
        let key = format!("embedding_model:{}@{}", model.model_id, model.model_version);
        self.guard.validate_write(ctx, &key).await?;

        let content_types_json = serde_json::to_string(&model.content_types)?;
        let compatible_with_json = serde_json::to_string(&model.compatible_with)?;

        sqlx::query(
            r#"
            INSERT INTO ai_embedding_models (
                model_id, model_version, dimensions, max_input_tokens, content_types_json, status, introduced_at, compatible_with_json
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            ON CONFLICT (model_id, model_version) DO UPDATE SET
                dimensions = excluded.dimensions,
                max_input_tokens = excluded.max_input_tokens,
                content_types_json = excluded.content_types_json,
                status = excluded.status,
                compatible_with_json = excluded.compatible_with_json
            "#,
        )
        .bind(model.model_id)
        .bind(model.model_version)
        .bind(model.dimensions as i32)
        .bind(model.max_input_tokens as i32)
        .bind(content_types_json)
        .bind(model.status.as_str())
        .bind(model.introduced_at.naive_utc())
        .bind(compatible_with_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_ai_embedding_models(&self) -> StorageResult<Vec<EmbeddingModelRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                model_id,
                model_version,
                dimensions,
                max_input_tokens,
                content_types_json,
                status,
                introduced_at,
                compatible_with_json
            FROM ai_embedding_models
            ORDER BY model_id ASC, model_version ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let content_types_json: String = row.get("content_types_json");
            let compatible_with_json: String = row.get("compatible_with_json");
            let content_types: Vec<String> = serde_json::from_str(&content_types_json)?;
            let compatible_with: Vec<String> = serde_json::from_str(&compatible_with_json)?;

            out.push(EmbeddingModelRecord {
                model_id: row.get("model_id"),
                model_version: row.get("model_version"),
                dimensions: map_i64_from_i32(&row, "dimensions") as u32,
                max_input_tokens: map_i64_from_i32(&row, "max_input_tokens") as u32,
                content_types,
                status: crate::ai_ready_data::records::EmbeddingModelStatus::from_str(
                    row.get::<String, _>("status").as_str(),
                )
                .map_err(|_| StorageError::Validation("invalid embedding model status"))?,
                introduced_at: map_timestamp(&row, "introduced_at"),
                compatible_with,
            });
        }

        Ok(out)
    }

    async fn set_ai_embedding_default_model(
        &self,
        ctx: &WriteContext,
        model_id: &str,
        model_version: &str,
    ) -> StorageResult<()> {
        self.guard
            .validate_write(ctx, "ai_embedding_registry")
            .await?;
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO ai_embedding_registry (
                id, current_default_model_id, current_default_model_version, updated_at
            )
            VALUES ('global', $1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
                current_default_model_id = excluded.current_default_model_id,
                current_default_model_version = excluded.current_default_model_version,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(model_id)
        .bind(model_version)
        .bind(now.naive_utc())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_ai_embedding_registry(&self) -> StorageResult<Option<EmbeddingRegistry>> {
        let row = sqlx::query(
            r#"
            SELECT
                current_default_model_id,
                current_default_model_version,
                updated_at
            FROM ai_embedding_registry
            WHERE id = 'global'
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(EmbeddingRegistry {
            current_default_model_id: row.get("current_default_model_id"),
            current_default_model_version: row.get("current_default_model_version"),
            updated_at: map_timestamp(&row, "updated_at"),
        }))
    }

    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_ai_job(row),
            None => Err(StorageError::NotFound("ai_job")),
        }
    }

    async fn list_ai_jobs(&self, filter: AiJobListFilter) -> StorageResult<Vec<AiJob>> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            r#"
            SELECT
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            FROM ai_jobs
            "#,
        );

        let mut has_where = false;
        let mut push_clause = |builder: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if has_where {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                has_where = true;
            }
        };

        if let Some(status) = filter.status {
            push_clause(&mut qb);
            qb.push("status = ").push_bind(status.as_str());
        }
        if let Some(kind) = filter.job_kind {
            push_clause(&mut qb);
            qb.push("job_kind = ").push_bind(kind.as_str());
        }
        if let Some(wsid) = filter.wsid {
            push_clause(&mut qb);
            qb.push(
                "EXISTS (SELECT 1 FROM jsonb_array_elements(entity_refs::jsonb) AS elem WHERE elem->>'entity_kind' = 'workspace' AND elem->>'entity_id' = ",
            )
            .push_bind(wsid)
            .push(")");
        }
        if let Some(from) = filter.from {
            push_clause(&mut qb);
            qb.push("created_at >= ").push_bind(from);
        }
        if let Some(to) = filter.to {
            push_clause(&mut qb);
            qb.push("created_at <= ").push_bind(to);
        }

        qb.push(" ORDER BY created_at DESC LIMIT ");
        qb.push_bind(200_i64);

        let rows = qb.build().fetch_all(&self.pool).await?;
        rows.into_iter().map(map_ai_job).collect()
    }

    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob> {
        validate_job_contract(&job.job_kind, &job.profile_id, &job.protocol_id)?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let job_inputs = job.job_inputs.clone().map(|value| value.to_string());
        let metrics_json = serde_json::to_string(&job.metrics)?;
        let entity_refs_json = serde_json::to_string(&job.entity_refs)?;
        let planned_ops_json = serde_json::to_string(&job.planned_operations)?;

        let row = sqlx::query(
            r#"
            INSERT INTO ai_jobs (
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            "#,
        )
        .bind(&id)
        .bind(job.trace_id.to_string())
        .bind(Option::<String>::None)
        .bind(job.job_kind.as_str())
        .bind(JobState::Queued.as_str())
        .bind(&job.status_reason)
        .bind(&job.protocol_id)
        .bind(&job.profile_id)
        .bind(&job.capability_profile_id)
        .bind(job.access_mode.as_str())
        .bind(job.safety_mode.as_str())
        .bind(entity_refs_json)
        .bind(planned_ops_json)
        .bind(metrics_json)
        .bind(&job_inputs)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        map_ai_job(row)
    }

    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob> {
        let job_outputs = update.job_outputs.as_ref().map(|val| val.to_string());
        let metrics_json = update
            .metrics
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            UPDATE ai_jobs
            SET status = $1,
                status_reason = $2,
                metrics = COALESCE($3, metrics),
                workflow_run_id = COALESCE($4, workflow_run_id),
                trace_id = COALESCE($5, trace_id),
                error_message = COALESCE($6, error_message),
                job_outputs = COALESCE($7, job_outputs),
                updated_at = $8
            WHERE id = $9
            RETURNING
                id,
                trace_id,
                workflow_run_id,
                job_kind,
                status,
                status_reason,
                error_message,
                protocol_id,
                profile_id,
                capability_profile_id,
                access_mode,
                safety_mode,
                entity_refs,
                planned_operations,
                metrics,
                job_inputs,
                job_outputs,
                created_at,
                updated_at
            "#,
        )
        .bind(update.state.as_str())
        .bind(&update.status_reason)
        .bind(metrics_json)
        .bind(update.workflow_run_id.map(|id| id.to_string()))
        .bind(update.trace_id.map(|id| id.to_string()))
        .bind(update.error_message.clone())
        .bind(&job_outputs)
        .bind(now)
        .bind(update.job_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => map_ai_job(row),
            None => Err(StorageError::NotFound("ai_job")),
        }
    }

    async fn set_job_outputs(
        &self,
        job_id: &str,
        outputs: Option<serde_json::Value>,
    ) -> StorageResult<()> {
        let now = Utc::now();
        let outputs = outputs.map(|val| val.to_string());
        sqlx::query(
            r#"
            UPDATE ai_jobs
            SET job_outputs = $1,
                updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(&outputs)
        .bind(now)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_workflow_run(
        &self,
        job_id: Uuid,
        status: JobState,
        last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    ) -> StorageResult<WorkflowRun> {
        let id = Uuid::new_v4();
        let heartbeat = last_heartbeat.unwrap_or_else(Utc::now);
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO workflow_runs (id, job_id, status, last_heartbeat, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, job_id, status, last_heartbeat, created_at, updated_at
            "#,
        )
        .bind(id.to_string())
        .bind(job_id.to_string())
        .bind(status.as_str())
        .bind(heartbeat)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        map_workflow_run(row)
    }

    async fn update_workflow_run_status(
        &self,
        run_id: Uuid,
        status: JobState,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowRun> {
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            UPDATE workflow_runs
            SET status = $1,
                updated_at = $2
            WHERE id = $3
            RETURNING id, job_id, status, last_heartbeat, created_at, updated_at
            "#,
        )
        .bind(status.as_str())
        .bind(now)
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(StorageError::NotFound("workflow_run"))?;

        if let Some(message) = error_message.clone() {
            sqlx::query(
                r#"
                UPDATE ai_jobs
                SET error_message = $1,
                    updated_at = $2
                WHERE id = (SELECT job_id FROM workflow_runs WHERE id = $3)
                "#,
            )
            .bind(&message)
            .bind(now)
            .bind(run_id.to_string())
            .execute(&self.pool)
            .await?;
        }

        map_workflow_run(row)
    }

    async fn heartbeat_workflow(
        &self,
        run_id: Uuid,
        at: chrono::DateTime<chrono::Utc>,
    ) -> StorageResult<()> {
        sqlx::query(
            r#"
            UPDATE workflow_runs
            SET last_heartbeat = $1,
                updated_at = $1
            WHERE id = $2
            "#,
        )
        .bind(at)
        .bind(run_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_workflow_node_execution(
        &self,
        exec: NewNodeExecution,
    ) -> StorageResult<WorkflowNodeExecution> {
        let id = Uuid::new_v4();
        let input_payload = exec.input_payload.as_ref().map(|v| v.to_string());
        let row = sqlx::query(
            r#"
            INSERT INTO workflow_node_executions (
                id, workflow_run_id, node_id, node_type, status, sequence, input_payload, started_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, workflow_run_id, node_id, node_type, status, sequence, input_payload,
                output_payload, error_message, started_at, finished_at, created_at, updated_at
            "#,
        )
        .bind(id.to_string())
        .bind(exec.workflow_run_id.to_string())
        .bind(exec.node_id)
        .bind(exec.node_type)
        .bind(exec.status.as_str())
        .bind(exec.sequence)
        .bind(input_payload)
        .bind(exec.started_at)
        .fetch_one(&self.pool)
        .await?;

        map_workflow_node_execution(row)
    }

    async fn update_workflow_node_execution_status(
        &self,
        exec_id: Uuid,
        status: JobState,
        output: Option<Value>,
        error_message: Option<String>,
    ) -> StorageResult<WorkflowNodeExecution> {
        let now = Utc::now();
        let output_payload = output.as_ref().map(|v| v.to_string());
        let row = sqlx::query(
            r#"
            UPDATE workflow_node_executions
            SET status = $1,
                output_payload = COALESCE($2, output_payload),
                error_message = COALESCE($3, error_message),
                finished_at = CASE WHEN $1 IN ('completed','failed','cancelled','stalled') THEN $4 ELSE finished_at END,
                updated_at = $4
            WHERE id = $5
            RETURNING
                id, workflow_run_id, node_id, node_type, status, sequence, input_payload,
                output_payload, error_message, started_at, finished_at, created_at, updated_at
            "#,
        )
        .bind(status.as_str())
        .bind(output_payload)
        .bind(error_message)
        .bind(now)
        .bind(exec_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(StorageError::NotFound("workflow_node_execution"))?;
        map_workflow_node_execution(row)
    }

    async fn list_workflow_node_executions(
        &self,
        run_id: Uuid,
    ) -> StorageResult<Vec<WorkflowNodeExecution>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, workflow_run_id, node_id, node_type, status, sequence,
                input_payload, output_payload, error_message, started_at,
                finished_at, created_at, updated_at
            FROM workflow_node_executions
            WHERE workflow_run_id = $1
            ORDER BY sequence ASC
            "#,
        )
        .bind(run_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_workflow_node_execution).collect()
    }

    async fn find_stalled_workflows(&self, threshold_secs: u64) -> StorageResult<Vec<WorkflowRun>> {
        let cutoff = Utc::now() - chrono::Duration::seconds(threshold_secs as i64);
        let rows = sqlx::query(
            r#"
            SELECT id, job_id, status, last_heartbeat, created_at, updated_at
            FROM workflow_runs
            WHERE status = 'running'
              AND last_heartbeat < $1
            "#,
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_workflow_run).collect()
    }

    async fn validate_write_with_guard(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata> {
        let metadata = self.guard.validate_write(ctx, resource_id).await?;
        Ok(metadata)
    }

    async fn prune_ai_jobs(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
        min_versions: u32,
        dry_run: bool,
    ) -> StorageResult<super::PruneReport> {
        let mut report = super::PruneReport::new();

        let scan_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COALESCE(SUM(CASE WHEN is_pinned = 1 THEN 1 ELSE 0 END), 0) as pinned
            FROM ai_jobs
            WHERE status IN ('completed', 'failed')
              AND created_at < $1
            "#,
        )
        .bind(cutoff)
        .fetch_one(&self.pool)
        .await?;

        let total_eligible: i64 = scan_row.get("total");
        let pinned_count: i64 = scan_row.get("pinned");

        let total_eligible = total_eligible.max(0) as u32;
        let pinned_count = pinned_count.max(0) as u32;
        let deletable_count = total_eligible.saturating_sub(pinned_count);

        report.items_scanned += total_eligible;
        report.items_spared_pinned += pinned_count;

        let non_pinned_row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM ai_jobs
            WHERE is_pinned = 0
              AND status IN ('completed', 'failed')
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let total_non_pinned: i64 = non_pinned_row.get("count");
        let total_non_pinned = total_non_pinned.max(0) as u32;

        let max_deletable = total_non_pinned.saturating_sub(min_versions);
        let actual_to_delete = deletable_count.min(max_deletable);

        if actual_to_delete == 0 {
            report.items_spared_window += deletable_count;
            return Ok(report);
        }

        if dry_run {
            report.items_pruned += actual_to_delete;
            report.items_spared_window += deletable_count.saturating_sub(actual_to_delete);
            return Ok(report);
        }

        let mut deleted = 0u32;
        let batch_size = 1000i64;

        while deleted < actual_to_delete {
            let remaining = (actual_to_delete - deleted) as i64;
            let limit = remaining.min(batch_size);

            let result = sqlx::query(
                r#"
                DELETE FROM ai_jobs
                WHERE id IN (
                    SELECT id FROM ai_jobs
                    WHERE status IN ('completed', 'failed')
                      AND created_at < $1
                      AND is_pinned = 0
                    ORDER BY created_at ASC
                    LIMIT $2
                )
                "#,
            )
            .bind(cutoff)
            .bind(limit)
            .execute(&self.pool)
            .await?;

            let batch_deleted = result.rows_affected() as u32;
            if batch_deleted == 0 {
                break;
            }
            deleted += batch_deleted;
        }

        report.items_pruned += deleted;
        report.items_spared_window += deletable_count.saturating_sub(deleted);
        Ok(report)
    }
}
