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
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_document(row: PgRow) -> Document {
    Document {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_canvas(row: PgRow) -> Canvas {
    Canvas {
        id: row.get("id"),
        workspace_id: row.get("workspace_id"),
        title: row.get("title"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_canvas_edge(row: PgRow) -> CanvasEdge {
    CanvasEdge {
        id: row.get("id"),
        canvas_id: row.get("canvas_id"),
        from_node_id: row.get("from_node_id"),
        to_node_id: row.get("to_node_id"),
        kind: row.get("kind"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
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
        sequence: row.get("sequence"),
        raw_content: row.get("raw_content"),
        display_content: row.get("display_content"),
        derived_content: derived,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
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
        position_x: row.get("position_x"),
        position_y: row.get("position_y"),
        data,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
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
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn map_workflow_run(row: PgRow) -> StorageResult<WorkflowRun> {
    Ok(WorkflowRun {
        id: Uuid::parse_str(row.get::<String, _>("id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run id"))?,
        job_id: Uuid::parse_str(row.get::<String, _>("job_id").as_str())
            .map_err(|_| StorageError::Validation("invalid workflow_run job_id"))?,
        status: JobState::try_from(row.get::<String, _>("status").as_str())?,
        last_heartbeat: row.get("last_heartbeat"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
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
        sequence: row.get("sequence"),
        input_payload,
        output_payload,
        error_message: row.get("error_message"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
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
            INSERT INTO workspaces (id, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&workspace.name)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE workspaces
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5
            WHERE id = $6
            "#,
        )
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(&id)
        .execute(&self.pool)
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
            INSERT INTO documents (id, workspace_id, title, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, workspace_id, title, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&doc.workspace_id)
        .bind(&doc.title)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE documents
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5
            WHERE id = $6
            "#,
        )
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(&id)
        .execute(&self.pool)
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
                id, document_id, kind, sequence, raw_content, display_content, derived_content,
                created_at, updated_at, sensitivity, exportable
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
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
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE blocks
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5
            WHERE id = $6
            "#,
        )
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(&id)
        .execute(&self.pool)
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
                    id, document_id, kind, sequence, raw_content, display_content, derived_content,
                    created_at, updated_at, sensitivity, exportable
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
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
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                UPDATE blocks
                SET last_actor_kind = $1,
                    last_actor_id = $2,
                    last_job_id = $3,
                    last_workflow_id = $4,
                    edit_event_id = $5
                WHERE id = $6
                "#,
            )
            .bind(actor_kind)
            .bind(actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .bind(&id)
            .execute(&mut *tx)
            .await?;

            inserted.push(map_block(row)?);
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
            INSERT INTO canvases (id, workspace_id, title, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, workspace_id, title, created_at, updated_at
            "#,
        )
        .bind(&id)
        .bind(&canvas.workspace_id)
        .bind(&canvas.title)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        sqlx::query(
            r#"
            UPDATE canvases
            SET last_actor_kind = $1,
                last_actor_id = $2,
                last_job_id = $3,
                last_workflow_id = $4,
                edit_event_id = $5
            WHERE id = $6
            "#,
        )
        .bind(actor_kind)
        .bind(actor_id)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(&id)
        .execute(&self.pool)
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
        self.guard.validate_write(ctx, canvas_id).await?;
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
                    id, canvas_id, kind, position_x, position_y, data, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
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
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                UPDATE canvas_nodes
                SET last_actor_kind = $1,
                    last_actor_id = $2,
                    last_job_id = $3,
                    last_workflow_id = $4,
                    edit_event_id = $5
                WHERE id = $6
                "#,
            )
            .bind(actor_kind)
            .bind(actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .bind(&id)
            .execute(&mut *tx)
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
                    id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
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
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                UPDATE canvas_edges
                SET last_actor_kind = $1,
                    last_actor_id = $2,
                    last_job_id = $3,
                    last_workflow_id = $4,
                    edit_event_id = $5
                WHERE id = $6
                "#,
            )
            .bind(actor_kind)
            .bind(actor_id)
            .bind(job_id)
            .bind(workflow_id)
            .bind(edit_event_id)
            .bind(&id)
            .execute(&mut *tx)
            .await?;

            inserted_edges.push(map_canvas_edge(row));
        }

        tx.commit().await?;

        Ok(CanvasGraph {
            canvas: map_canvas(canvas_row),
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
        _cutoff: chrono::DateTime<chrono::Utc>,
        _min_versions: u32,
        _dry_run: bool,
    ) -> StorageResult<super::PruneReport> {
        Err(StorageError::NotImplemented("postgres pruning"))
    }
}
