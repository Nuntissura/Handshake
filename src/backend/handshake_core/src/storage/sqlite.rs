use super::{
    AccessMode, AiJob, AiJobListFilter, Block, BlockUpdate, Canvas, CanvasEdge, CanvasGraph,
    CanvasNode, DefaultStorageGuard, Document, EntityRef, JobKind, JobMetrics, JobState,
    JobStatusUpdate, MutationMetadata, NewAiJob, NewBlock, NewCanvas, NewCanvasEdge, NewCanvasNode,
    NewDocument, NewNodeExecution, NewWorkspace, PlannedOperation, SafetyMode, StorageError,
    StorageGuard, StorageResult, WorkflowNodeExecution, WorkflowRun, Workspace, WriteContext,
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;
/// SQLite-backed implementation of the Database trait.
pub struct SqliteDatabase {
    pool: SqlitePool,
    guard: Arc<dyn StorageGuard>,
}

#[cfg(test)]
impl SqliteDatabase {
    /// Expose pool for test-only SQL adjustments (kept out of production interfaces).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[derive(sqlx::FromRow)]
struct AiJobRow {
    id: String,
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
    entity_refs: String,
    planned_operations: String,
    metrics: String,
    job_inputs: Option<String>,
    job_outputs: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl SqliteDatabase {
    fn map_ai_job_row(&self, row: AiJobRow) -> StorageResult<AiJob> {
        let job_state = JobState::try_from(row.status.as_str())?;
        let job_kind = JobKind::from_str(row.job_kind.as_str())?;
        let access_mode = AccessMode::try_from(row.access_mode.as_str())?;
        let safety_mode = SafetyMode::try_from(row.safety_mode.as_str())?;
        let entity_refs: Vec<EntityRef> = serde_json::from_str(&row.entity_refs)?;
        let planned_operations: Vec<PlannedOperation> =
            serde_json::from_str(&row.planned_operations)?;
        let metrics: JobMetrics = serde_json::from_str(&row.metrics)?;

        Ok(AiJob {
            job_id: Uuid::parse_str(&row.id)
                .map_err(|_| StorageError::Validation("invalid job_id uuid"))?,
            trace_id: Uuid::parse_str(&row.trace_id)
                .map_err(|_| StorageError::Validation("invalid trace_id uuid"))?,
            workflow_run_id: row
                .workflow_run_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(|_| StorageError::Validation("invalid workflow_run_id uuid"))?,
            job_kind,
            state: job_state,
            error_message: row.error_message,
            protocol_id: row.protocol_id,
            profile_id: row.profile_id,
            capability_profile_id: row.capability_profile_id,
            access_mode,
            safety_mode,
            entity_refs,
            planned_operations,
            metrics,
            status_reason: row.status_reason,
            job_inputs: row
                .job_inputs
                .map(|val| serde_json::from_str::<Value>(val.as_str()))
                .transpose()?,
            job_outputs: row
                .job_outputs
                .map(|val| serde_json::from_str::<Value>(val.as_str()))
                .transpose()?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    fn map_workflow_run_row(row: sqlx::sqlite::SqliteRow) -> StorageResult<super::WorkflowRun> {
        Ok(super::WorkflowRun {
            id: Uuid::parse_str(row.get::<String, _>("id").as_str())
                .map_err(|_| StorageError::Validation("invalid workflow_run uuid"))?,
            job_id: Uuid::parse_str(row.get::<String, _>("job_id").as_str())
                .map_err(|_| StorageError::Validation("invalid job_id uuid"))?,
            status: JobState::try_from(row.get::<String, _>("status").as_str())?,
            last_heartbeat: row.get("last_heartbeat"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    fn map_node_exec_row(row: sqlx::sqlite::SqliteRow) -> StorageResult<WorkflowNodeExecution> {
        let input_value = row
            .get::<Option<String>, _>("input_payload")
            .map(|val| serde_json::from_str(&val))
            .transpose()?;
        let output_value = row
            .get::<Option<String>, _>("output_payload")
            .map(|val| serde_json::from_str(&val))
            .transpose()?;

        Ok(WorkflowNodeExecution {
            id: Uuid::parse_str(row.get::<String, _>("id").as_str())
                .map_err(|_| StorageError::Validation("invalid node execution uuid"))?,
            workflow_run_id: Uuid::parse_str(row.get::<String, _>("workflow_run_id").as_str())
                .map_err(|_| StorageError::Validation("invalid workflow_run_id uuid"))?,
            node_id: row.get("node_id"),
            node_type: row.get("node_type"),
            status: JobState::try_from(row.get::<String, _>("status").as_str())?,
            sequence: row.get("sequence"),
            input_payload: input_value,
            output_payload: output_value,
            error_message: row.get("error_message"),
            started_at: row.get("started_at"),
            finished_at: row.get("finished_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn connect(db_url: &str, max_connections: u32) -> StorageResult<Self> {
        let guard: Arc<dyn StorageGuard> = Arc::new(DefaultStorageGuard);
        Self::connect_with_guard(db_url, max_connections, guard).await
    }

    pub async fn connect_with_guard(
        db_url: &str,
        max_connections: u32,
        guard: Arc<dyn StorageGuard>,
    ) -> StorageResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect(db_url)
            .await?;
        Ok(Self { pool, guard })
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            guard: Arc::new(DefaultStorageGuard),
        }
    }

    pub fn new_with_guard(pool: SqlitePool, guard: Arc<dyn StorageGuard>) -> Self {
        Self { pool, guard }
    }

    pub fn into_arc(self) -> Arc<dyn super::Database> {
        Arc::new(self)
    }
}

#[async_trait]
impl super::Database for SqliteDatabase {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn run_migrations(&self) -> StorageResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    async fn ping(&self) -> StorageResult<()> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }

    async fn list_workspaces(&self) -> StorageResult<Vec<Workspace>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                name as "name!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM workspaces
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Workspace {
                id: row.id,
                name: row.name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
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
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let inserted = sqlx::query!(
            r#"
            INSERT INTO workspaces (
                id, name, last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id as "id!: String",
                name as "name!: String",
                last_actor_kind as "last_actor_kind!: String",
                last_actor_id as "last_actor_id: String",
                last_job_id as "last_job_id: String",
                last_workflow_id as "last_workflow_id: String",
                edit_event_id as "edit_event_id!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            workspace.name,
            actor_kind,
            actor_id_ref,
            job_id,
            workflow_id,
            edit_event_id,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Workspace {
            id: inserted.id,
            name: inserted.name,
            created_at: inserted.created_at,
            updated_at: inserted.updated_at,
        })
    }

    async fn delete_workspace(&self, ctx: &WriteContext, id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, id).await?;
        let rows = sqlx::query!(r#"DELETE FROM workspaces WHERE id = $1"#, id)
            .execute(&self.pool)
            .await?;
        if rows.rows_affected() == 0 {
            return Err(StorageError::NotFound("workspace"));
        }
        Ok(())
    }

    async fn get_workspace(&self, id: &str) -> StorageResult<Option<Workspace>> {
        let row = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                name as "name!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM workspaces
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Workspace {
            id: r.id,
            name: r.name,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }))
    }
    async fn list_documents(&self, workspace_id: &str) -> StorageResult<Vec<Document>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM documents
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
            workspace_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Document {
                id: row.id,
                workspace_id: row.workspace_id,
                title: row.title,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn get_document(&self, doc_id: &str) -> StorageResult<Document> {
        let row = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM documents
            WHERE id = $1
            "#,
            doc_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Document {
                id: row.id,
                workspace_id: row.workspace_id,
                title: row.title,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }),
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
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let inserted = sqlx::query!(
            r#"
            INSERT INTO documents (
                id, workspace_id, title, last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                last_actor_kind as "last_actor_kind!: String",
                last_actor_id as "last_actor_id: String",
                last_job_id as "last_job_id: String",
                last_workflow_id as "last_workflow_id: String",
                edit_event_id as "edit_event_id!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            doc.workspace_id,
            doc.title,
            actor_kind,
            actor_id_ref,
            job_id,
            workflow_id,
            edit_event_id,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Document {
            id: inserted.id,
            workspace_id: inserted.workspace_id,
            title: inserted.title,
            created_at: inserted.created_at,
            updated_at: inserted.updated_at,
        })
    }

    async fn delete_document(&self, ctx: &WriteContext, doc_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, doc_id).await?;
        let res = sqlx::query!(r#"DELETE FROM documents WHERE id = $1"#, doc_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("document"));
        }
        Ok(())
    }
    async fn get_blocks(&self, doc_id: &str) -> StorageResult<Vec<Block>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                document_id as "document_id!: String",
                kind as "kind!: String",
                sequence as "sequence!: i64",
                raw_content as "raw_content!: String",
                display_content as "display_content!: String",
                derived_content as "derived_content!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>",
                sensitivity as "sensitivity: String",
                exportable as "exportable: i32"
            FROM blocks
            WHERE document_id = $1
            ORDER BY sequence ASC
            "#,
            doc_id
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let derived = serde_json::from_str(&row.derived_content)?;
                Ok(Block {
                    id: row.id,
                    document_id: row.document_id,
                    kind: row.kind,
                    sequence: row.sequence,
                    raw_content: row.raw_content,
                    display_content: row.display_content,
                    derived_content: derived,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    sensitivity: row.sensitivity,
                    exportable: row.exportable.map(|v| v != 0),
                })
            })
            .collect()
    }

    async fn get_block(&self, block_id: &str) -> StorageResult<Block> {
        let row = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                document_id as "document_id!: String",
                kind as "kind!: String",
                sequence as "sequence!: i64",
                raw_content as "raw_content!: String",
                display_content as "display_content!: String",
                derived_content as "derived_content!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>",
                sensitivity as "sensitivity: String",
                exportable as "exportable: i32"
            FROM blocks
            WHERE id = $1
            "#,
            block_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let derived = serde_json::from_str(&row.derived_content)?;
                Ok(Block {
                    id: row.id,
                    document_id: row.document_id,
                    kind: row.kind,
                    sequence: row.sequence,
                    raw_content: row.raw_content,
                    display_content: row.display_content,
                    derived_content: derived,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    sensitivity: row.sensitivity,
                    exportable: row.exportable.map(|v| v != 0),
                })
            }
            None => Err(StorageError::NotFound("block")),
        }
    }
    async fn create_block(&self, ctx: &WriteContext, block: NewBlock) -> StorageResult<Block> {
        let now = Utc::now();
        let id = block.id.map_or_else(|| Uuid::new_v4().to_string(), |v| v);
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();
        let display_content = block
            .display_content
            .map_or_else(|| block.raw_content.clone(), |v| v);
        let derived_content = block
            .derived_content
            .map_or_else(|| Value::Object(Default::default()), |v| v)
            .to_string();

        let row = sqlx::query!(
            r#"
            INSERT INTO blocks (
                id, document_id, kind, sequence, raw_content, display_content, derived_content, last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING
                id as "id!: String",
                document_id as "document_id!: String",
                kind as "kind!: String",
                sequence as "sequence!: i64",
                raw_content as "raw_content!: String",
                display_content as "display_content!: String",
                derived_content as "derived_content!: String",
                last_actor_kind as "last_actor_kind!: String",
                last_actor_id as "last_actor_id: String",
                last_job_id as "last_job_id: String",
                last_workflow_id as "last_workflow_id: String",
                edit_event_id as "edit_event_id!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>",
                sensitivity as "sensitivity: String",
                exportable as "exportable: i32"
            "#,
            id,
            block.document_id,
            block.kind,
            block.sequence,
            block.raw_content,
            display_content,
            derived_content,
            actor_kind,
            actor_id_ref,
            job_id,
            workflow_id,
            edit_event_id,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        let derived = serde_json::from_str(&row.derived_content)?;
        Ok(Block {
            id: row.id,
            document_id: row.document_id,
            kind: row.kind,
            sequence: row.sequence,
            raw_content: row.raw_content,
            display_content: row.display_content,
            derived_content: derived,
            created_at: row.created_at,
            updated_at: row.updated_at,
            sensitivity: row.sensitivity,
            exportable: row.exportable.map(|v| v != 0),
        })
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
        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        sqlx::query!(
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
            block.kind,
            block.sequence,
            block.raw_content,
            block.display_content,
            derived_content,
            actor_kind,
            actor_id_ref,
            job_id,
            workflow_id,
            edit_event_id,
            now,
            block.id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_block(&self, ctx: &WriteContext, block_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, block_id).await?;
        let res = sqlx::query!(r#"DELETE FROM blocks WHERE id = $1"#, block_id)
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
        sqlx::query!(r#"DELETE FROM blocks WHERE document_id = $1"#, document_id)
            .execute(&mut *tx)
            .await?;

        let mut inserted = Vec::with_capacity(blocks.len());
        for block in blocks {
            let now = Utc::now();
            let id = block.id.map_or_else(|| Uuid::new_v4().to_string(), |v| v);
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let actor_id_ref = actor_id.as_deref();
            let job_id = metadata.job_id.map(|id| id.to_string());
            let workflow_id = metadata.workflow_id.map(|id| id.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();
            let display_content = block
                .display_content
                .map_or_else(|| block.raw_content.clone(), |v| v);
            let derived_content = block
                .derived_content
                .map_or_else(|| Value::Object(Default::default()), |v| v)
                .to_string();

            let row = sqlx::query!(
                r#"
                INSERT INTO blocks (
                    id, document_id, kind, sequence, raw_content, display_content, derived_content, last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                RETURNING
                    id as "id!: String",
                    document_id as "document_id!: String",
                    kind as "kind!: String",
                    sequence as "sequence!: i64",
                    raw_content as "raw_content!: String",
                    display_content as "display_content!: String",
                    derived_content as "derived_content!: String",
                    last_actor_kind as "last_actor_kind!: String",
                    last_actor_id as "last_actor_id: String",
                    last_job_id as "last_job_id: String",
                    last_workflow_id as "last_workflow_id: String",
                    edit_event_id as "edit_event_id!: String",
                    created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                    updated_at as "updated_at!: chrono::DateTime<chrono::Utc>",
                    sensitivity as "sensitivity: String",
                    exportable as "exportable: i32"
                "#,
                id,
                document_id,
                block.kind,
                block.sequence,
                block.raw_content,
                display_content,
                derived_content,
                actor_kind,
                actor_id_ref,
                job_id,
                workflow_id,
                edit_event_id,
                now,
                now
            )
            .fetch_one(&mut *tx)
            .await?;

            let derived = serde_json::from_str(&row.derived_content)?;
            inserted.push(Block {
                id: row.id,
                document_id: row.document_id,
                kind: row.kind,
                sequence: row.sequence,
                raw_content: row.raw_content,
                display_content: row.display_content,
                derived_content: derived,
                created_at: row.created_at,
                updated_at: row.updated_at,
                sensitivity: row.sensitivity,
                exportable: row.exportable.map(|v| v != 0),
            });
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
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let row = sqlx::query!(
            r#"
            INSERT INTO canvases (id, workspace_id, title, last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                last_actor_kind as "last_actor_kind!: String",
                last_actor_id as "last_actor_id: String",
                last_job_id as "last_job_id: String",
                last_workflow_id as "last_workflow_id: String",
                edit_event_id as "edit_event_id!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            canvas.workspace_id,
            canvas.title,
            actor_kind,
            actor_id_ref,
            job_id,
            workflow_id,
            edit_event_id,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Canvas {
            id: row.id,
            workspace_id: row.workspace_id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn list_canvases(&self, workspace_id: &str) -> StorageResult<Vec<Canvas>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM canvases
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
            workspace_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Canvas {
                id: row.id,
                workspace_id: row.workspace_id,
                title: row.title,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn get_canvas_with_graph(&self, canvas_id: &str) -> StorageResult<CanvasGraph> {
        let canvas_row = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM canvases
            WHERE id = $1
            "#,
            canvas_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let canvas_row = match canvas_row {
            Some(row) => row,
            None => return Err(StorageError::NotFound("canvas")),
        };

        let nodes = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                canvas_id as "canvas_id!: String",
                kind as "kind!: String",
                position_x as "position_x!: f64",
                position_y as "position_y!: f64",
                data as "data!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM canvas_nodes
            WHERE canvas_id = $1
            ORDER BY created_at ASC
            "#,
            canvas_id
        )
        .fetch_all(&self.pool)
        .await?;

        let edges = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                canvas_id as "canvas_id!: String",
                from_node_id as "from_node_id!: String",
                to_node_id as "to_node_id!: String",
                kind as "kind!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM canvas_edges
            WHERE canvas_id = $1
            ORDER BY created_at ASC
            "#,
            canvas_id
        )
        .fetch_all(&self.pool)
        .await?;

        let parsed_nodes = nodes
            .into_iter()
            .map(|n| {
                let data = serde_json::from_str(&n.data)?;
                Ok(CanvasNode {
                    id: n.id,
                    canvas_id: n.canvas_id,
                    kind: n.kind,
                    position_x: n.position_x,
                    position_y: n.position_y,
                    data,
                    created_at: n.created_at,
                    updated_at: n.updated_at,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        let parsed_edges = edges
            .into_iter()
            .map(|e| {
                Ok(CanvasEdge {
                    id: e.id,
                    canvas_id: e.canvas_id,
                    from_node_id: e.from_node_id,
                    to_node_id: e.to_node_id,
                    kind: e.kind,
                    created_at: e.created_at,
                    updated_at: e.updated_at,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        Ok(CanvasGraph {
            canvas: Canvas {
                id: canvas_row.id,
                workspace_id: canvas_row.workspace_id,
                title: canvas_row.title,
                created_at: canvas_row.created_at,
                updated_at: canvas_row.updated_at,
            },
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
        let mut tx = self.pool.begin().await?;

        let canvas_row = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM canvases
            WHERE id = $1
            "#,
            canvas_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        let canvas_row = match canvas_row {
            Some(row) => row,
            None => return Err(StorageError::NotFound("canvas")),
        };

        sqlx::query!(
            r#"DELETE FROM canvas_edges WHERE canvas_id = $1"#,
            canvas_id
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            r#"DELETE FROM canvas_nodes WHERE canvas_id = $1"#,
            canvas_id
        )
        .execute(&mut *tx)
        .await?;

        let now = Utc::now();
        let mut inserted_nodes = Vec::with_capacity(nodes.len());
        for node in nodes {
            let id = node.id.map_or_else(|| Uuid::new_v4().to_string(), |v| v);
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let actor_id_ref = actor_id.as_deref();
            let job_id = metadata.job_id.map(|id| id.to_string());
            let workflow_id = metadata.workflow_id.map(|id| id.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();
            let data = node
                .data
                .map_or_else(|| Value::Object(Default::default()), |v| v)
                .to_string();

            let row = sqlx::query!(
                r#"
                INSERT INTO canvas_nodes (
                    id, canvas_id, kind, position_x, position_y, data, last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                RETURNING
                    id as "id!: String",
                    canvas_id as "canvas_id!: String",
                    kind as "kind!: String",
                    position_x as "position_x!: f64",
                    position_y as "position_y!: f64",
                    data as "data!: String",
                    last_actor_kind as "last_actor_kind!: String",
                    last_actor_id as "last_actor_id: String",
                    last_job_id as "last_job_id: String",
                    last_workflow_id as "last_workflow_id: String",
                    edit_event_id as "edit_event_id!: String",
                    created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                    updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
                "#,
                id,
                canvas_id,
                node.kind,
                node.position_x,
                node.position_y,
                data,
                actor_kind,
                actor_id_ref,
                job_id,
                workflow_id,
                edit_event_id,
                now,
                now
            )
            .fetch_one(&mut *tx)
            .await?;

            let parsed = serde_json::from_str(&row.data)?;
            inserted_nodes.push(CanvasNode {
                id: row.id,
                canvas_id: row.canvas_id,
                kind: row.kind,
                position_x: row.position_x,
                position_y: row.position_y,
                data: parsed,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        let mut inserted_edges = Vec::with_capacity(edges.len());
        for edge in edges {
            let id = edge.id.map_or_else(|| Uuid::new_v4().to_string(), |v| v);
            let metadata = self.guard.validate_write(ctx, &id).await?;
            let actor_kind = metadata.actor_kind.as_str();
            let actor_id = metadata.actor_id.clone();
            let actor_id_ref = actor_id.as_deref();
            let job_id = metadata.job_id.map(|id| id.to_string());
            let workflow_id = metadata.workflow_id.map(|id| id.to_string());
            let edit_event_id = metadata.edit_event_id.to_string();

            let row = sqlx::query!(
                r#"
                INSERT INTO canvas_edges (
                    id, canvas_id, from_node_id, to_node_id, kind, last_actor_kind, last_actor_id, last_job_id, last_workflow_id, edit_event_id, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                RETURNING
                    id as "id!: String",
                    canvas_id as "canvas_id!: String",
                    from_node_id as "from_node_id!: String",
                    to_node_id as "to_node_id!: String",
                    kind as "kind!: String",
                    last_actor_kind as "last_actor_kind!: String",
                    last_actor_id as "last_actor_id: String",
                    last_job_id as "last_job_id: String",
                    last_workflow_id as "last_workflow_id: String",
                    edit_event_id as "edit_event_id!: String",
                    created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                    updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
                "#,
                id,
                canvas_id,
                edge.from_node_id,
                edge.to_node_id,
                edge.kind,
                actor_kind,
                actor_id_ref,
                job_id,
                workflow_id,
                edit_event_id,
                now,
                now
            )
            .fetch_one(&mut *tx)
            .await?;

            inserted_edges.push(CanvasEdge {
                id: row.id,
                canvas_id: row.canvas_id,
                from_node_id: row.from_node_id,
                to_node_id: row.to_node_id,
                kind: row.kind,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        tx.commit().await?;

        Ok(CanvasGraph {
            canvas: Canvas {
                id: canvas_row.id,
                workspace_id: canvas_row.workspace_id,
                title: canvas_row.title,
                created_at: canvas_row.created_at,
                updated_at: canvas_row.updated_at,
            },
            nodes: inserted_nodes,
            edges: inserted_edges,
        })
    }

    async fn delete_canvas(&self, ctx: &WriteContext, canvas_id: &str) -> StorageResult<()> {
        self.guard.validate_write(ctx, canvas_id).await?;
        let res = sqlx::query!(r#"DELETE FROM canvases WHERE id = $1"#, canvas_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("canvas"));
        }
        Ok(())
    }
    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob> {
        let row = sqlx::query_as!(
            AiJobRow,
            r#"
            SELECT
                id as "id!: String",
                trace_id as "trace_id!: String",
                workflow_run_id as "workflow_run_id?",
                job_kind as "job_kind!: String",
                status as "status!: String",
                status_reason as "status_reason!: String",
                error_message as "error_message?",
                protocol_id as "protocol_id!: String",
                profile_id as "profile_id!: String",
                capability_profile_id as "capability_profile_id!: String",
                access_mode as "access_mode!: String",
                safety_mode as "safety_mode!: String",
                entity_refs as "entity_refs!: String",
                planned_operations as "planned_operations!: String",
                metrics as "metrics!: String",
                job_inputs as "job_inputs?",
                job_outputs as "job_outputs?",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM ai_jobs
            WHERE id = $1
            "#,
            job_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => self.map_ai_job_row(row),
            None => Err(StorageError::NotFound("ai_job")),
        }
    }

    async fn list_ai_jobs(&self, filter: AiJobListFilter) -> StorageResult<Vec<AiJob>> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            r#"
            SELECT
                id as "id!: String",
                trace_id as "trace_id!: String",
                workflow_run_id as "workflow_run_id?",
                job_kind as "job_kind!: String",
                status as "status!: String",
                status_reason as "status_reason!: String",
                error_message as "error_message?",
                protocol_id as "protocol_id!: String",
                profile_id as "profile_id!: String",
                capability_profile_id as "capability_profile_id!: String",
                access_mode as "access_mode!: String",
                safety_mode as "safety_mode!: String",
                entity_refs as "entity_refs!: String",
                planned_operations as "planned_operations!: String",
                metrics as "metrics!: String",
                job_inputs as "job_inputs?",
                job_outputs as "job_outputs?",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM ai_jobs
            "#,
        );

        let mut has_where = false;
        let mut push_clause = |builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>| {
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
                "EXISTS (SELECT 1 FROM json_each(entity_refs) AS elem WHERE json_extract(elem.value, '$.entity_kind') = 'workspace' AND json_extract(elem.value, '$.entity_id') = ",
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

        let rows = qb
            .build_query_as::<AiJobRow>()
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|row| self.map_ai_job_row(row))
            .collect()
    }

    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let job_inputs = job.job_inputs.clone().map(|value| value.to_string());
        let metrics_json = serde_json::to_string(&job.metrics)?;
        let entity_refs_json = serde_json::to_string(&job.entity_refs)?;
        let planned_ops_json = serde_json::to_string(&job.planned_operations)?;
        let id_str = id.to_string();
        let trace_id = job.trace_id.to_string();
        let job_kind = job.job_kind.as_str().to_string();
        let status_reason = job.status_reason;
        let protocol_id = job.protocol_id;
        let profile_id = job.profile_id;
        let capability_profile_id = job.capability_profile_id;
        let access_mode = job.access_mode.as_str().to_string();
        let safety_mode = job.safety_mode.as_str().to_string();
        let queued_status = JobState::Queued.as_str().to_string();

        let row = sqlx::query_as!(
            AiJobRow,
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
                id as "id!: String",
                trace_id as "trace_id!: String",
                workflow_run_id as "workflow_run_id?",
                job_kind as "job_kind!: String",
                status as "status!: String",
                status_reason as "status_reason!: String",
                error_message as "error_message?",
                protocol_id as "protocol_id!: String",
                profile_id as "profile_id!: String",
                capability_profile_id as "capability_profile_id!: String",
                access_mode as "access_mode!: String",
                safety_mode as "safety_mode!: String",
                entity_refs as "entity_refs!: String",
                planned_operations as "planned_operations!: String",
                metrics as "metrics!: String",
                job_inputs as "job_inputs?",
                job_outputs as "job_outputs?",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id_str,
            trace_id,
            Option::<String>::None,
            job_kind,
            queued_status,
            status_reason,
            protocol_id,
            profile_id,
            capability_profile_id,
            access_mode,
            safety_mode,
            entity_refs_json,
            planned_ops_json,
            metrics_json,
            job_inputs,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        self.map_ai_job_row(row)
    }

    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<AiJob> {
        let job_outputs = update.job_outputs.as_ref().map(|val| val.to_string());
        let metrics_json = update
            .metrics
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let status = update.state.as_str().to_string();
        let status_reason = update.status_reason.clone();
        let workflow_run_id = update.workflow_run_id.map(|id| id.to_string());
        let trace_id = update.trace_id.map(|id| id.to_string());
        let error_message = update.error_message.clone();
        let job_id = update.job_id.to_string();
        let now = Utc::now();
        let row = sqlx::query_as!(
            AiJobRow,
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
                id as "id!: String",
                trace_id as "trace_id!: String",
                workflow_run_id as "workflow_run_id?",
                job_kind as "job_kind!: String",
                status as "status!: String",
                status_reason as "status_reason!: String",
                error_message as "error_message?",
                protocol_id as "protocol_id!: String",
                profile_id as "profile_id!: String",
                capability_profile_id as "capability_profile_id!: String",
                access_mode as "access_mode!: String",
                safety_mode as "safety_mode!: String",
                entity_refs as "entity_refs!: String",
                planned_operations as "planned_operations!: String",
                metrics as "metrics!: String",
                job_inputs as "job_inputs?",
                job_outputs as "job_outputs?",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            status,
            status_reason,
            metrics_json,
            workflow_run_id,
            trace_id,
            error_message,
            job_outputs,
            now,
            job_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => self.map_ai_job_row(row),
            None => Err(StorageError::NotFound("ai_job")),
        }
    }

    async fn set_job_outputs(&self, job_id: &str, outputs: Option<Value>) -> StorageResult<()> {
        let outputs_json = outputs.as_ref().map(|val| val.to_string());
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE ai_jobs
            SET job_outputs = $1,
                updated_at = $2
            WHERE id = $3
            "#,
            outputs_json,
            now,
            job_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    async fn create_workflow_run(
        &self,
        job_id: Uuid,
        status: JobState,
        last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    ) -> StorageResult<super::WorkflowRun> {
        let id = Uuid::new_v4();
        let heartbeat = last_heartbeat.unwrap_or_else(Utc::now);
        let row = sqlx::query(
            r#"
            INSERT INTO workflow_runs (id, job_id, status, last_heartbeat)
            VALUES ($1, $2, $3, $4)
            RETURNING
                id,
                job_id,
                status,
                last_heartbeat,
                created_at,
                updated_at
            "#,
        )
        .bind(id.to_string())
        .bind(job_id.to_string())
        .bind(status.as_str())
        .bind(heartbeat)
        .fetch_one(&self.pool)
        .await?;

        Ok(Self::map_workflow_run_row(row)?)
    }

    async fn update_workflow_run_status(
        &self,
        run_id: Uuid,
        status: JobState,
        error_message: Option<String>,
    ) -> StorageResult<super::WorkflowRun> {
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            UPDATE workflow_runs
            SET status = $1,
                updated_at = $2
            WHERE id = $3
            RETURNING
                id,
                job_id,
                status,
                last_heartbeat,
                created_at,
                updated_at
            "#,
        )
        .bind(status.as_str())
        .bind(now)
        .bind(run_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if row.is_none() {
            return Err(StorageError::NotFound("workflow_run"));
        }

        if let Some(message) = error_message {
            sqlx::query(
                r#"
                UPDATE ai_jobs
                SET error_message = $1,
                    updated_at = $2
                WHERE id = (SELECT job_id FROM workflow_runs WHERE id = $3)
                "#,
            )
            .bind(message)
            .bind(now)
            .bind(run_id.to_string())
            .execute(&self.pool)
            .await?;
        }

        let Some(row) = row else {
            return Err(StorageError::NotFound("workflow_run"));
        };
        Ok(Self::map_workflow_run_row(row)?)
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
                id,
                workflow_run_id,
                node_id,
                node_type,
                status,
                sequence,
                input_payload,
                output_payload,
                error_message,
                started_at,
                finished_at,
                created_at,
                updated_at
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

        Ok(Self::map_node_exec_row(row)?)
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
                id,
                workflow_run_id,
                node_id,
                node_type,
                status,
                sequence,
                input_payload,
                output_payload,
                error_message,
                started_at,
                finished_at,
                created_at,
                updated_at
            "#,
        )
        .bind(status.as_str())
        .bind(output_payload)
        .bind(error_message)
        .bind(now)
        .bind(exec_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(StorageError::NotFound("workflow_node_execution"));
        };
        Ok(Self::map_node_exec_row(row)?)
    }

    async fn list_workflow_node_executions(
        &self,
        run_id: Uuid,
    ) -> StorageResult<Vec<WorkflowNodeExecution>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                workflow_run_id,
                node_id,
                node_type,
                status,
                sequence,
                input_payload,
                output_payload,
                error_message,
                started_at,
                finished_at,
                created_at,
                updated_at
            FROM workflow_node_executions
            WHERE workflow_run_id = $1
            ORDER BY sequence ASC
            "#,
        )
        .bind(run_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::map_node_exec_row).collect()
    }

    async fn find_stalled_workflows(&self, threshold_secs: u64) -> StorageResult<Vec<WorkflowRun>> {
        let cutoff = Utc::now() - chrono::Duration::seconds(threshold_secs as i64);
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                job_id,
                status,
                last_heartbeat,
                created_at,
                updated_at
            FROM workflow_runs
            WHERE status = 'running'
              AND last_heartbeat < $1
            "#,
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::map_workflow_run_row).collect()
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
        let cutoff_str = cutoff.to_rfc3339();

        // Count total eligible items (completed/failed jobs older than cutoff)
        let scan_result = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total!: i64",
                SUM(CASE WHEN is_pinned = 1 THEN 1 ELSE 0 END) as "pinned!: i64"
            FROM ai_jobs
            WHERE status IN ('completed', 'failed')
              AND created_at < $1
            "#,
            cutoff_str
        )
        .fetch_one(&self.pool)
        .await?;

        let total_eligible = scan_result.total as u32;
        let pinned_count = scan_result.pinned as u32;
        let deletable_count = total_eligible.saturating_sub(pinned_count);

        report.items_scanned += total_eligible;
        report.items_spared_pinned += pinned_count;

        // Respect min_versions: keep the N most recent even if expired
        let total_non_pinned = sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!: i64"
            FROM ai_jobs
            WHERE is_pinned = 0
              AND status IN ('completed', 'failed')
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        let total_non_pinned = total_non_pinned.count as u32;

        let max_deletable = total_non_pinned.saturating_sub(min_versions);

        let actual_to_delete = deletable_count.min(max_deletable);

        if actual_to_delete == 0 {
            report.items_spared_window += deletable_count;
            return Ok(report);
        }

        if dry_run {
            report.items_pruned += actual_to_delete;
            return Ok(report);
        }

        let mut deleted = 0u32;
        let batch_size = 1000i64; // Default batch size

        while deleted < actual_to_delete {
            let remaining = (actual_to_delete - deleted) as i64;
            let limit = remaining.min(batch_size);

            let result = sqlx::query!(
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
                cutoff_str,
                limit
            )
            .execute(&self.pool)
            .await?;

            let batch_deleted = result.rows_affected() as u32;
            if batch_deleted == 0 {
                break;
            }
            deleted += batch_deleted;
        }

        report.items_pruned += deleted;
        let spared_window = deletable_count.saturating_sub(deleted);
        report.items_spared_window += spared_window;

        Ok(report)
    }
}
