use super::{
    validate_job_contract, AccessMode, AiJob, AiJobListFilter, Block, BlockUpdate, BronzeRecord,
    AiJobMcpFields, AiJobMcpUpdate, Asset, Canvas, CanvasEdge, CanvasGraph, CanvasNode,
    DefaultStorageGuard, Document, LoomBlock, LoomBlockContentType, LoomBlockDerived,
    LoomBlockSearchResult, LoomBlockUpdate, LoomEdge, LoomEdgeCreatedBy, LoomEdgeType,
    LoomSearchFilters, LoomSourceAnchor, LoomViewFilters, LoomViewGroup, LoomViewResponse,
    LoomViewType, NewAsset, NewLoomBlock, NewLoomEdge, PreviewStatus,
    EmbeddingModelRecord, EmbeddingRegistry, EntityRef, JobKind, JobMetrics, JobState,
    JobStatusUpdate, MutationMetadata, NewAiJob, NewBlock, NewBronzeRecord, NewCanvas,
    NewCanvasEdge, NewCanvasNode, NewDocument, NewNodeExecution, NewSilverRecord, NewWorkspace,
    PlannedOperation, SafetyMode, SilverRecord, StorageError, StorageGuard, StorageResult,
    WorkflowNodeExecution, WorkflowRun, Workspace, WriteContext,
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

impl SqliteDatabase {
    pub(crate) fn pool(&self) -> &SqlitePool {
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

#[derive(sqlx::FromRow)]
struct AssetRow {
    asset_id: String,
    workspace_id: String,
    kind: String,
    mime: String,
    original_filename: Option<String>,
    content_hash: String,
    size_bytes: i64,
    width: Option<i64>,
    height: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    classification: String,
    exportable: i64,
    is_proxy_of: Option<String>,
    proxy_asset_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct LoomBlockRow {
    block_id: String,
    workspace_id: String,
    content_type: String,
    document_id: Option<String>,
    asset_id: Option<String>,
    title: Option<String>,
    original_filename: Option<String>,
    content_hash: Option<String>,
    pinned: i64,
    journal_date: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    imported_at: Option<chrono::DateTime<chrono::Utc>>,
    backlink_count: i64,
    mention_count: i64,
    tag_count: i64,
    derived_json: String,
    preview_status: String,
    thumbnail_asset_id: Option<String>,
    proxy_asset_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct LoomBlockSearchRow {
    score: f64,
    block_id: String,
    workspace_id: String,
    content_type: String,
    document_id: Option<String>,
    asset_id: Option<String>,
    title: Option<String>,
    original_filename: Option<String>,
    content_hash: Option<String>,
    pinned: i64,
    journal_date: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    imported_at: Option<chrono::DateTime<chrono::Utc>>,
    backlink_count: i64,
    mention_count: i64,
    tag_count: i64,
    derived_json: String,
    preview_status: String,
    thumbnail_asset_id: Option<String>,
    proxy_asset_id: Option<String>,
}

#[derive(sqlx::FromRow)]
struct LoomEdgeRow {
    edge_id: String,
    workspace_id: String,
    source_block_id: String,
    target_block_id: String,
    edge_type: String,
    created_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
    crdt_site_id: Option<String>,
    source_document_id: Option<String>,
    source_text_block_id: Option<String>,
    offset_start: Option<i64>,
    offset_end: Option<i64>,
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

    fn map_asset_row(&self, row: AssetRow) -> Asset {
        Asset {
            asset_id: row.asset_id,
            workspace_id: row.workspace_id,
            kind: row.kind,
            mime: row.mime,
            original_filename: row.original_filename,
            content_hash: row.content_hash,
            size_bytes: row.size_bytes,
            width: row.width,
            height: row.height,
            created_at: row.created_at,
            classification: row.classification,
            exportable: row.exportable != 0,
            is_proxy_of: row.is_proxy_of,
            proxy_asset_id: row.proxy_asset_id,
        }
    }

    fn map_loom_block_row(&self, row: LoomBlockRow) -> StorageResult<LoomBlock> {
        let content_type = LoomBlockContentType::from_str(row.content_type.as_str())?;
        let preview_status = PreviewStatus::from_str(row.preview_status.as_str())?;

        let mut derived: LoomBlockDerived =
            serde_json::from_str(&row.derived_json).unwrap_or_default();
        derived.backlink_count = row.backlink_count;
        derived.mention_count = row.mention_count;
        derived.tag_count = row.tag_count;
        derived.preview_status = preview_status;
        derived.thumbnail_asset_id = row.thumbnail_asset_id.clone();
        derived.proxy_asset_id = row.proxy_asset_id.clone();

        Ok(LoomBlock {
            block_id: row.block_id,
            workspace_id: row.workspace_id,
            content_type,
            document_id: row.document_id,
            asset_id: row.asset_id,
            title: row.title,
            original_filename: row.original_filename,
            content_hash: row.content_hash,
            pinned: row.pinned != 0,
            journal_date: row.journal_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
            imported_at: row.imported_at,
            derived,
        })
    }

    fn map_loom_block_search_row(
        &self,
        row: LoomBlockSearchRow,
    ) -> StorageResult<LoomBlockSearchResult> {
        let score = row.score;
        let block = self.map_loom_block_row(LoomBlockRow {
            block_id: row.block_id,
            workspace_id: row.workspace_id,
            content_type: row.content_type,
            document_id: row.document_id,
            asset_id: row.asset_id,
            title: row.title,
            original_filename: row.original_filename,
            content_hash: row.content_hash,
            pinned: row.pinned,
            journal_date: row.journal_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
            imported_at: row.imported_at,
            backlink_count: row.backlink_count,
            mention_count: row.mention_count,
            tag_count: row.tag_count,
            derived_json: row.derived_json,
            preview_status: row.preview_status,
            thumbnail_asset_id: row.thumbnail_asset_id,
            proxy_asset_id: row.proxy_asset_id,
        })?;
        Ok(LoomBlockSearchResult { block, score })
    }

    fn map_loom_edge_row(&self, row: LoomEdgeRow) -> StorageResult<LoomEdge> {
        let edge_type = LoomEdgeType::from_str(row.edge_type.as_str())?;
        let created_by = LoomEdgeCreatedBy::from_str(row.created_by.as_str())?;
        let source_anchor = match (
            row.source_document_id,
            row.source_text_block_id,
            row.offset_start,
            row.offset_end,
        ) {
            (Some(document_id), Some(block_id), Some(offset_start), Some(offset_end)) => {
                Some(LoomSourceAnchor {
                    document_id,
                    block_id,
                    offset_start,
                    offset_end,
                })
            }
            _ => None,
        };

        Ok(LoomEdge {
            edge_id: row.edge_id,
            workspace_id: row.workspace_id,
            source_block_id: row.source_block_id,
            target_block_id: row.target_block_id,
            edge_type,
            created_by,
            created_at: row.created_at,
            crdt_site_id: row.crdt_site_id,
            source_anchor,
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

fn normalize_fts5_query(raw: &str) -> Option<String> {
    let query = raw.trim();
    if query.is_empty() {
        return None;
    }

    let mut tokens: Vec<String> = Vec::new();
    for token in query.split_whitespace() {
        let token = token.trim_matches('"').trim();
        if token.is_empty() {
            continue;
        }

        let safe_word = token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
        let token = token.replace('"', "\"\"");
        if safe_word {
            tokens.push(format!("{token}*"));
        } else {
            tokens.push(format!("\"{token}\""));
        }
    }

    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" AND "))
    }
}

async fn ensure_loom_fts_schema_sqlite(pool: &SqlitePool) -> StorageResult<()> {
    let has_loom_blocks: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='loom_blocks'",
    )
    .fetch_optional(pool)
    .await?;
    if has_loom_blocks.is_none() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS loom_blocks_fts USING fts5(
            workspace_id UNINDEXED,
            block_id UNINDEXED,
            title,
            original_filename,
            full_text_index,
            tokenize = 'unicode61'
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    let loom_block_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM loom_blocks")
        .fetch_one(&mut *tx)
        .await?;
    let fts_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM loom_blocks_fts")
        .fetch_one(&mut *tx)
        .await?;

    if loom_block_count != fts_count {
        sqlx::query("DELETE FROM loom_blocks_fts")
            .execute(&mut *tx)
            .await?;

        #[derive(sqlx::FromRow)]
        struct LoomBlockIndexRow {
            workspace_id: String,
            block_id: String,
            title: Option<String>,
            original_filename: Option<String>,
            derived_json: String,
        }

        #[derive(serde::Deserialize, Default)]
        struct LoomBlockDerivedIndex {
            #[serde(default)]
            full_text_index: Option<String>,
        }

        let rows: Vec<LoomBlockIndexRow> = sqlx::query_as(
            r#"
            SELECT workspace_id, block_id, title, original_filename, derived_json
            FROM loom_blocks
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;

        for row in rows {
            let derived: LoomBlockDerivedIndex =
                serde_json::from_str(&row.derived_json).unwrap_or_default();
            let full_text_index = derived.full_text_index.unwrap_or_default();

            sqlx::query(
                r#"
                INSERT INTO loom_blocks_fts (workspace_id, block_id, title, original_filename, full_text_index)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(row.workspace_id)
            .bind(row.block_id)
            .bind(row.title.unwrap_or_default())
            .bind(row.original_filename.unwrap_or_default())
            .bind(full_text_index)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn upsert_loom_block_fts_sqlite(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    block: &LoomBlock,
) -> StorageResult<()> {
    let title = block.title.as_deref().unwrap_or_default();
    let original_filename = block.original_filename.as_deref().unwrap_or_default();
    let full_text_index = block.derived.full_text_index.as_deref().unwrap_or_default();

    sqlx::query(
        r#"
        DELETE FROM loom_blocks_fts
        WHERE workspace_id = $1 AND block_id = $2
        "#,
    )
    .bind(&block.workspace_id)
    .bind(&block.block_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO loom_blocks_fts (workspace_id, block_id, title, original_filename, full_text_index)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&block.workspace_id)
    .bind(&block.block_id)
    .bind(title)
    .bind(original_filename)
    .bind(full_text_index)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn ensure_locus_schema_sqlite(pool: &SqlitePool) -> StorageResult<()> {
    let mut tx = pool.begin().await?;

    // Spec: Handshake_Master_Spec_v02.123.md §2.3.15.5 (Phase 1: SQLite)
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS work_packets (
            wp_id TEXT PRIMARY KEY,
            version INTEGER NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            priority INTEGER NOT NULL,
            phase TEXT,
            routing TEXT,
            task_packet_path TEXT,
            task_board_status TEXT NOT NULL,
            assignee TEXT,
            reporter TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            vector_clock TEXT NOT NULL,  -- JSON
            metadata TEXT NOT NULL       -- JSON
        );
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_wp_status ON work_packets(status);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_wp_priority ON work_packets(priority);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_wp_task_board_status ON work_packets(task_board_status);"#,
        r#"
        CREATE TABLE IF NOT EXISTS micro_tasks (
            mt_id TEXT PRIMARY KEY,
            wp_id TEXT NOT NULL,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            current_iteration INTEGER,
            escalation_level INTEGER,
            metadata TEXT NOT NULL,  -- JSON
            FOREIGN KEY (wp_id) REFERENCES work_packets(wp_id) ON DELETE CASCADE
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS mt_iterations (
            iteration_id INTEGER PRIMARY KEY AUTOINCREMENT,
            mt_id TEXT NOT NULL,
            iteration INTEGER NOT NULL,
            model_id TEXT NOT NULL,
            lora_id TEXT,
            outcome TEXT NOT NULL,
            validation_passed INTEGER,
            duration_ms INTEGER NOT NULL,
            FOREIGN KEY (mt_id) REFERENCES micro_tasks(mt_id) ON DELETE CASCADE
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS dependencies (
            dependency_id TEXT PRIMARY KEY,
            from_wp_id TEXT NOT NULL,
            to_wp_id TEXT NOT NULL,
            type TEXT NOT NULL,
            created_at TEXT NOT NULL,
            vector_clock TEXT NOT NULL,  -- JSON
            FOREIGN KEY (from_wp_id) REFERENCES work_packets(wp_id) ON DELETE CASCADE,
            FOREIGN KEY (to_wp_id) REFERENCES work_packets(wp_id) ON DELETE CASCADE
        );
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_dep_from ON dependencies(from_wp_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_dep_to ON dependencies(to_wp_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_dep_type ON dependencies(type);"#,
    ];

    for statement in statements {
        sqlx::query(statement).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn ensure_ai_jobs_mcp_columns_sqlite(pool: &SqlitePool) -> StorageResult<()> {
    async fn exec_ignore_duplicate_column(pool: &SqlitePool, sql: &str) -> StorageResult<()> {
        match sqlx::query(sql).execute(pool).await {
            Ok(_) => Ok(()),
            Err(e) => {
                let ignore = matches!(&e, sqlx::Error::Database(db_err) if db_err.message().to_lowercase().contains("duplicate column name"));
                if ignore {
                    Ok(())
                } else {
                    Err(e.into())
                }
            }
        }
    }

    exec_ignore_duplicate_column(pool, "ALTER TABLE ai_jobs ADD COLUMN mcp_server_id TEXT").await?;
    exec_ignore_duplicate_column(pool, "ALTER TABLE ai_jobs ADD COLUMN mcp_call_id TEXT").await?;
    exec_ignore_duplicate_column(
        pool,
        "ALTER TABLE ai_jobs ADD COLUMN mcp_progress_token TEXT",
    )
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_jobs_progress_token ON ai_jobs(mcp_progress_token)")
        .execute(pool)
        .await?;

    Ok(())
}

#[async_trait]
impl super::Database for SqliteDatabase {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn run_migrations(&self) -> StorageResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        ensure_locus_schema_sqlite(&self.pool).await?;
        ensure_ai_jobs_mcp_columns_sqlite(&self.pool).await?;
        ensure_loom_fts_schema_sqlite(&self.pool).await?;
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

        let doc_metadata = self.guard.validate_write(ctx, document_id).await?;
        let doc_actor_kind = doc_metadata.actor_kind.as_str();
        let doc_actor_id = doc_metadata.actor_id.clone();
        let doc_actor_id_ref = doc_actor_id.as_deref();
        let doc_job_id = doc_metadata.job_id.map(|id| id.to_string());
        let doc_workflow_id = doc_metadata.workflow_id.map(|id| id.to_string());
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
        .bind(doc_actor_id_ref)
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

    async fn create_asset(&self, ctx: &WriteContext, asset: NewAsset) -> StorageResult<Asset> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let exportable: i64 = if asset.exportable { 1 } else { 0 };

        let row: AssetRow = sqlx::query_as(
            r#"
            INSERT INTO assets (
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9,
                $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19
            )
            RETURNING
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            "#,
        )
        .bind(&id)
        .bind(&asset.workspace_id)
        .bind(&asset.kind)
        .bind(&asset.mime)
        .bind(&asset.original_filename)
        .bind(&asset.content_hash)
        .bind(asset.size_bytes)
        .bind(asset.width)
        .bind(asset.height)
        .bind(actor_kind)
        .bind(actor_id_ref)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(&asset.classification)
        .bind(exportable)
        .bind(&asset.is_proxy_of)
        .bind(&asset.proxy_asset_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(self.map_asset_row(row))
    }

    async fn get_asset(&self, workspace_id: &str, asset_id: &str) -> StorageResult<Asset> {
        let row: Option<AssetRow> = sqlx::query_as(
            r#"
            SELECT
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            FROM assets
            WHERE workspace_id = $1 AND asset_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(StorageError::NotFound("asset"))?;
        Ok(self.map_asset_row(row))
    }

    async fn find_asset_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<Asset>> {
        let row: Option<AssetRow> = sqlx::query_as(
            r#"
            SELECT
                asset_id,
                workspace_id,
                kind,
                mime,
                original_filename,
                content_hash,
                size_bytes,
                width,
                height,
                created_at,
                classification,
                exportable,
                is_proxy_of,
                proxy_asset_id
            FROM assets
            WHERE workspace_id = $1 AND content_hash = $2
            "#,
        )
        .bind(workspace_id)
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| self.map_asset_row(row)))
    }

    async fn create_loom_block(
        &self,
        ctx: &WriteContext,
        block: NewLoomBlock,
    ) -> StorageResult<LoomBlock> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let id = block.block_id.map_or_else(|| Uuid::new_v4().to_string(), |v| v);
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let derived_json = serde_json::to_string(&block.derived)?;
        let preview_status = block.derived.preview_status.as_str();

        let row: LoomBlockRow = sqlx::query_as(
            r#"
            INSERT INTO loom_blocks (
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                journal_date,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15,
                $16, $17, $18,
                $19, $20, $21,
                $22, $23, $24, $25
            )
            RETURNING
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            "#,
        )
        .bind(&id)
        .bind(&block.workspace_id)
        .bind(block.content_type.as_str())
        .bind(&block.document_id)
        .bind(&block.asset_id)
        .bind(&block.title)
        .bind(&block.original_filename)
        .bind(&block.content_hash)
        .bind(if block.pinned { 1_i64 } else { 0_i64 })
        .bind(&block.journal_date)
        .bind(actor_kind)
        .bind(actor_id_ref)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(now)
        .bind(block.imported_at)
        .bind(block.derived.backlink_count)
        .bind(block.derived.mention_count)
        .bind(block.derived.tag_count)
        .bind(derived_json)
        .bind(preview_status)
        .bind(&block.derived.thumbnail_asset_id)
        .bind(&block.derived.proxy_asset_id)
        .fetch_one(&mut *tx)
        .await?;

        let block = self.map_loom_block_row(row)?;
        upsert_loom_block_fts_sqlite(&mut tx, &block).await?;
        tx.commit().await?;
        Ok(block)
    }

    async fn get_loom_block(&self, workspace_id: &str, block_id: &str) -> StorageResult<LoomBlock> {
        let row: Option<LoomBlockRow> = sqlx::query_as(
            r#"
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM loom_blocks
            WHERE workspace_id = $1 AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or(StorageError::NotFound("loom_block"))?;
        self.map_loom_block_row(row)
    }

    async fn find_loom_block_by_content_hash(
        &self,
        workspace_id: &str,
        content_hash: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        let row: Option<LoomBlockRow> = sqlx::query_as(
            r#"
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM loom_blocks
            WHERE workspace_id = $1 AND content_hash = $2
            "#,
        )
        .bind(workspace_id)
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.map_loom_block_row(row)?)),
            None => Ok(None),
        }
    }

    async fn find_loom_block_by_asset_id(
        &self,
        workspace_id: &str,
        asset_id: &str,
    ) -> StorageResult<Option<LoomBlock>> {
        let row: Option<LoomBlockRow> = sqlx::query_as(
            r#"
            SELECT
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            FROM loom_blocks
            WHERE workspace_id = $1 AND asset_id = $2
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(asset_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.map_loom_block_row(row)?)),
            None => Ok(None),
        }
    }

    async fn update_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        update: LoomBlockUpdate,
    ) -> StorageResult<LoomBlock> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let pinned: Option<i64> = update.pinned.map(|v| if v { 1 } else { 0 });

        let row: Option<LoomBlockRow> = sqlx::query_as(
            r#"
            UPDATE loom_blocks
            SET
                title = COALESCE($1, title),
                pinned = COALESCE($2, pinned),
                journal_date = COALESCE($3, journal_date),
                last_actor_kind = $4,
                last_actor_id = $5,
                last_job_id = $6,
                last_workflow_id = $7,
                edit_event_id = $8,
                updated_at = $9
            WHERE workspace_id = $10 AND block_id = $11
            RETURNING
                block_id,
                workspace_id,
                content_type,
                document_id,
                asset_id,
                title,
                original_filename,
                content_hash,
                pinned,
                journal_date,
                created_at,
                updated_at,
                imported_at,
                backlink_count,
                mention_count,
                tag_count,
                derived_json,
                preview_status,
                thumbnail_asset_id,
                proxy_asset_id
            "#,
        )
        .bind(update.title)
        .bind(pinned)
        .bind(update.journal_date)
        .bind(actor_kind)
        .bind(actor_id_ref)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(workspace_id)
        .bind(block_id)
        .fetch_optional(&mut *tx)
        .await?;

        let row = row.ok_or(StorageError::NotFound("loom_block"))?;
        let block = self.map_loom_block_row(row)?;
        upsert_loom_block_fts_sqlite(&mut tx, &block).await?;
        tx.commit().await?;
        Ok(block)
    }

    async fn set_loom_block_preview(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
        preview_status: PreviewStatus,
        thumbnail_asset_id: Option<String>,
        proxy_asset_id: Option<String>,
    ) -> StorageResult<()> {
        let now = Utc::now();
        let metadata = self.guard.validate_write(ctx, block_id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let res = sqlx::query(
            r#"
            UPDATE loom_blocks
            SET
                preview_status = $1,
                thumbnail_asset_id = $2,
                proxy_asset_id = $3,
                last_actor_kind = $4,
                last_actor_id = $5,
                last_job_id = $6,
                last_workflow_id = $7,
                edit_event_id = $8,
                updated_at = $9
            WHERE workspace_id = $10 AND block_id = $11
            "#,
        )
        .bind(preview_status.as_str())
        .bind(thumbnail_asset_id)
        .bind(proxy_asset_id)
        .bind(actor_kind)
        .bind(actor_id_ref)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(workspace_id)
        .bind(block_id)
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_block"));
        }

        Ok(())
    }

    async fn delete_loom_block(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<()> {
        self.guard.validate_write(ctx, block_id).await?;
        let mut tx = self.pool.begin().await?;
        let res = sqlx::query(
            r#"
            DELETE FROM loom_blocks
            WHERE workspace_id = $1 AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .execute(&mut *tx)
        .await?;

        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("loom_block"));
        }

        sqlx::query(
            r#"
            DELETE FROM loom_blocks_fts
            WHERE workspace_id = $1 AND block_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn create_loom_edge(
        &self,
        ctx: &WriteContext,
        edge: NewLoomEdge,
    ) -> StorageResult<LoomEdge> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let id = edge.edge_id.map_or_else(|| Uuid::new_v4().to_string(), |v| v);
        let metadata = self.guard.validate_write(ctx, &id).await?;
        let actor_kind = metadata.actor_kind.as_str();
        let actor_id = metadata.actor_id.clone();
        let actor_id_ref = actor_id.as_deref();
        let job_id = metadata.job_id.map(|id| id.to_string());
        let workflow_id = metadata.workflow_id.map(|id| id.to_string());
        let edit_event_id = metadata.edit_event_id.to_string();

        let (source_document_id, source_text_block_id, offset_start, offset_end) =
            match edge.source_anchor.clone() {
                Some(anchor) => (
                    Some(anchor.document_id),
                    Some(anchor.block_id),
                    Some(anchor.offset_start),
                    Some(anchor.offset_end),
                ),
                None => (None, None, None, None),
            };

        let row: LoomEdgeRow = sqlx::query_as(
            r#"
            INSERT INTO loom_edges (
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                last_actor_kind,
                last_actor_id,
                last_job_id,
                last_workflow_id,
                edit_event_id,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16, $17
            )
            RETURNING
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            "#,
        )
        .bind(&id)
        .bind(&edge.workspace_id)
        .bind(&edge.source_block_id)
        .bind(&edge.target_block_id)
        .bind(edge.edge_type.as_str())
        .bind(edge.created_by.as_str())
        .bind(actor_kind)
        .bind(actor_id_ref)
        .bind(job_id)
        .bind(workflow_id)
        .bind(edit_event_id)
        .bind(now)
        .bind(edge.crdt_site_id)
        .bind(source_document_id)
        .bind(source_text_block_id)
        .bind(offset_start)
        .bind(offset_end)
        .fetch_one(&mut *tx)
        .await?;

        if matches!(edge.edge_type, LoomEdgeType::Mention | LoomEdgeType::Tag) {
            for block_id in [&edge.source_block_id, &edge.target_block_id] {
                sqlx::query(
                    r#"
                    UPDATE loom_blocks
                    SET
                        mention_count = (SELECT COUNT(*) FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'mention'),
                        tag_count = (SELECT COUNT(*) FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'tag'),
                        backlink_count = (SELECT COUNT(*) FROM loom_edges WHERE workspace_id = $1 AND target_block_id = $2 AND edge_type IN ('mention', 'tag'))
                    WHERE workspace_id = $1 AND block_id = $2
                    "#,
                )
                .bind(&edge.workspace_id)
                .bind(block_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        self.map_loom_edge_row(row)
    }

    async fn delete_loom_edge(
        &self,
        ctx: &WriteContext,
        workspace_id: &str,
        edge_id: &str,
    ) -> StorageResult<LoomEdge> {
        let mut tx = self.pool.begin().await?;
        self.guard.validate_write(ctx, edge_id).await?;

        let existing: Option<LoomEdgeRow> = sqlx::query_as(
            r#"
            SELECT
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            FROM loom_edges
            WHERE workspace_id = $1 AND edge_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(edge_id)
        .fetch_optional(&mut *tx)
        .await?;

        let existing = existing.ok_or(StorageError::NotFound("loom_edge"))?;
        let mapped_existing = self.map_loom_edge_row(existing)?;

        sqlx::query(
            r#"
            DELETE FROM loom_edges
            WHERE workspace_id = $1 AND edge_id = $2
            "#,
        )
        .bind(workspace_id)
        .bind(edge_id)
        .execute(&mut *tx)
        .await?;

        if matches!(
            mapped_existing.edge_type,
            LoomEdgeType::Mention | LoomEdgeType::Tag
        ) {
            for block_id in [&mapped_existing.source_block_id, &mapped_existing.target_block_id] {
                sqlx::query(
                    r#"
                    UPDATE loom_blocks
                    SET
                        mention_count = (SELECT COUNT(*) FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'mention'),
                        tag_count = (SELECT COUNT(*) FROM loom_edges WHERE workspace_id = $1 AND source_block_id = $2 AND edge_type = 'tag'),
                        backlink_count = (SELECT COUNT(*) FROM loom_edges WHERE workspace_id = $1 AND target_block_id = $2 AND edge_type IN ('mention', 'tag'))
                    WHERE workspace_id = $1 AND block_id = $2
                    "#,
                )
                .bind(workspace_id)
                .bind(block_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(mapped_existing)
    }

    async fn list_loom_edges_for_block(
        &self,
        workspace_id: &str,
        block_id: &str,
    ) -> StorageResult<Vec<LoomEdge>> {
        let rows: Vec<LoomEdgeRow> = sqlx::query_as(
            r#"
            SELECT
                edge_id,
                workspace_id,
                source_block_id,
                target_block_id,
                edge_type,
                created_by,
                created_at,
                crdt_site_id,
                source_document_id,
                source_text_block_id,
                offset_start,
                offset_end
            FROM loom_edges
            WHERE workspace_id = $1
              AND (source_block_id = $2 OR target_block_id = $2)
            ORDER BY created_at ASC
            "#,
        )
        .bind(workspace_id)
        .bind(block_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.map_loom_edge_row(row))
            .collect()
    }

    async fn query_loom_view(
        &self,
        workspace_id: &str,
        view_type: LoomViewType,
        filters: LoomViewFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<LoomViewResponse> {
        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;

        let select_blocks = |extra_where: Option<&'static str>| async move {
            let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
                r#"
                SELECT
                    b.block_id,
                    b.workspace_id,
                    b.content_type,
                    b.document_id,
                    b.asset_id,
                    b.title,
                    b.original_filename,
                    b.content_hash,
                    b.pinned,
                    b.journal_date,
                    b.created_at,
                    b.updated_at,
                    b.imported_at,
                    b.backlink_count,
                    b.mention_count,
                    b.tag_count,
                    b.derived_json,
                    b.preview_status,
                    b.thumbnail_asset_id,
                    b.proxy_asset_id
                FROM loom_blocks b
                LEFT JOIN assets a
                  ON a.workspace_id = b.workspace_id AND a.asset_id = b.asset_id
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

            push_clause(&mut qb);
            qb.push("b.workspace_id = ").push_bind(workspace_id);

            if let Some(extra) = extra_where {
                push_clause(&mut qb);
                qb.push(extra);
            }

            if let Some(content_type) = filters.content_type {
                push_clause(&mut qb);
                qb.push("b.content_type = ").push_bind(content_type.as_str());
            }

            if let Some(mime) = filters.mime {
                push_clause(&mut qb);
                qb.push("a.mime = ").push_bind(mime);
            }

            if let Some(from) = filters.date_from {
                push_clause(&mut qb);
                qb.push("b.updated_at >= ").push_bind(from);
            }
            if let Some(to) = filters.date_to {
                push_clause(&mut qb);
                qb.push("b.updated_at <= ").push_bind(to);
            }

            if !filters.tag_ids.is_empty() {
                push_clause(&mut qb);
                qb.push(
                    "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'tag' AND e.target_block_id IN (",
                );
                let mut separated = qb.separated(", ");
                for tag_id in &filters.tag_ids {
                    separated.push_bind(tag_id);
                }
                separated.push_unseparated("))");
            }

            if !filters.mention_ids.is_empty() {
                push_clause(&mut qb);
                qb.push(
                    "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'mention' AND e.target_block_id IN (",
                );
                let mut separated = qb.separated(", ");
                for mention_id in &filters.mention_ids {
                    separated.push_bind(mention_id);
                }
                separated.push_unseparated("))");
            }

            qb.push(" ORDER BY b.updated_at DESC ");
            qb.push(" LIMIT ").push_bind(limit_i64);
            qb.push(" OFFSET ").push_bind(offset_i64);

            let rows: Vec<LoomBlockRow> = qb.build_query_as().fetch_all(&self.pool).await?;
            let blocks: Vec<LoomBlock> = rows
                .into_iter()
                .map(|row| self.map_loom_block_row(row))
                .collect::<StorageResult<Vec<_>>>()?;
            Ok::<_, StorageError>(blocks)
        };

        match view_type {
            LoomViewType::All => {
                let blocks = select_blocks(None).await?;
                Ok(LoomViewResponse::All { blocks })
            }
            LoomViewType::Pins => {
                let blocks = select_blocks(Some("b.pinned != 0")).await?;
                Ok(LoomViewResponse::Pins { blocks })
            }
            LoomViewType::Unlinked => {
                let blocks = select_blocks(Some(
                    r#"
                    NOT EXISTS (
                        SELECT 1
                        FROM loom_edges e
                        WHERE e.workspace_id = b.workspace_id
                          AND (e.source_block_id = b.block_id OR e.target_block_id = b.block_id)
                          AND e.edge_type IN ('mention', 'tag')
                    )
                    "#,
                ))
                .await?;
                Ok(LoomViewResponse::Unlinked { blocks })
            }
            LoomViewType::Sorted => {
                let group_rows: Vec<(String, String)> = sqlx::query_as(
                    r#"
                    SELECT DISTINCT edge_type, target_block_id
                    FROM loom_edges
                    WHERE workspace_id = $1
                      AND edge_type IN ('mention', 'tag')
                    ORDER BY edge_type ASC, target_block_id ASC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(workspace_id)
                .bind(limit_i64)
                .bind(offset_i64)
                .fetch_all(&self.pool)
                .await?;

                let mut groups: Vec<LoomViewGroup> = Vec::new();
                for (edge_type_raw, target_block_id) in group_rows {
                    let edge_type = LoomEdgeType::from_str(edge_type_raw.as_str())?;

                    let rows: Vec<LoomBlockRow> = sqlx::query_as(
                        r#"
                        SELECT
                            b.block_id,
                            b.workspace_id,
                            b.content_type,
                            b.document_id,
                            b.asset_id,
                            b.title,
                            b.original_filename,
                            b.content_hash,
                            b.pinned,
                            b.journal_date,
                            b.created_at,
                            b.updated_at,
                            b.imported_at,
                            b.backlink_count,
                            b.mention_count,
                            b.tag_count,
                            b.derived_json,
                            b.preview_status,
                            b.thumbnail_asset_id,
                            b.proxy_asset_id
                        FROM loom_edges e
                        JOIN loom_blocks b
                          ON b.workspace_id = e.workspace_id AND b.block_id = e.source_block_id
                        LEFT JOIN assets a
                          ON a.workspace_id = b.workspace_id AND a.asset_id = b.asset_id
                        WHERE e.workspace_id = $1
                          AND e.edge_type = $2
                          AND e.target_block_id = $3
                        ORDER BY b.updated_at DESC
                        LIMIT 100
                        "#,
                    )
                    .bind(workspace_id)
                    .bind(edge_type.as_str())
                    .bind(&target_block_id)
                    .fetch_all(&self.pool)
                    .await?;

                    let blocks: Vec<LoomBlock> = rows
                        .into_iter()
                        .map(|row| self.map_loom_block_row(row))
                        .collect::<StorageResult<Vec<_>>>()?;

                    groups.push(LoomViewGroup {
                        edge_type,
                        target_block_id,
                        blocks,
                    });
                }

                Ok(LoomViewResponse::Sorted { groups })
            }
        }
    }

    async fn search_loom_blocks(
        &self,
        workspace_id: &str,
        query: &str,
        filters: LoomSearchFilters,
        limit: u32,
        offset: u32,
    ) -> StorageResult<Vec<LoomBlockSearchResult>> {
        let Some(fts_query) = normalize_fts5_query(query) else {
            return Ok(Vec::new());
        };
        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;

        let mut qb = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            r#"
            SELECT
                bm25(loom_blocks_fts) AS score,
                b.block_id,
                b.workspace_id,
                b.content_type,
                b.document_id,
                b.asset_id,
                b.title,
                b.original_filename,
                b.content_hash,
                b.pinned,
                b.journal_date,
                b.created_at,
                b.updated_at,
                b.imported_at,
                b.backlink_count,
                b.mention_count,
                b.tag_count,
                b.derived_json,
                b.preview_status,
                b.thumbnail_asset_id,
                b.proxy_asset_id
            FROM loom_blocks_fts
            JOIN loom_blocks b
              ON b.workspace_id = loom_blocks_fts.workspace_id AND b.block_id = loom_blocks_fts.block_id
            LEFT JOIN assets a
              ON a.workspace_id = b.workspace_id AND a.asset_id = b.asset_id
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

        push_clause(&mut qb);
        qb.push("loom_blocks_fts.workspace_id = ").push_bind(workspace_id);

        push_clause(&mut qb);
        qb.push("loom_blocks_fts MATCH ").push_bind(fts_query);

        if let Some(content_type) = filters.content_type {
            push_clause(&mut qb);
            qb.push("b.content_type = ").push_bind(content_type.as_str());
        }
        if let Some(mime) = filters.mime {
            push_clause(&mut qb);
            qb.push("a.mime = ").push_bind(mime);
        }

        if !filters.tag_ids.is_empty() {
            push_clause(&mut qb);
            qb.push(
                "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'tag' AND e.target_block_id IN (",
            );
            let mut separated = qb.separated(", ");
            for tag_id in &filters.tag_ids {
                separated.push_bind(tag_id);
            }
            separated.push_unseparated("))");
        }

        if !filters.mention_ids.is_empty() {
            push_clause(&mut qb);
            qb.push(
                "EXISTS (SELECT 1 FROM loom_edges e WHERE e.workspace_id = b.workspace_id AND e.source_block_id = b.block_id AND e.edge_type = 'mention' AND e.target_block_id IN (",
            );
            let mut separated = qb.separated(", ");
            for mention_id in &filters.mention_ids {
                separated.push_bind(mention_id);
            }
            separated.push_unseparated("))");
        }

        qb.push(" ORDER BY score ASC, b.updated_at DESC ");
        qb.push(" LIMIT ").push_bind(limit_i64);
        qb.push(" OFFSET ").push_bind(offset_i64);

        let rows: Vec<LoomBlockSearchRow> = qb.build_query_as().fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|row| self.map_loom_block_search_row(row))
            .collect::<StorageResult<Vec<_>>>()
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

        let canvas_metadata = self.guard.validate_write(ctx, canvas_id).await?;
        let canvas_actor_kind = canvas_metadata.actor_kind.as_str();
        let canvas_actor_id = canvas_metadata.actor_id.clone();
        let canvas_actor_id_ref = canvas_actor_id.as_deref();
        let canvas_job_id = canvas_metadata.job_id.map(|id| id.to_string());
        let canvas_workflow_id = canvas_metadata.workflow_id.map(|id| id.to_string());
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
        .bind(canvas_actor_id_ref)
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

        Ok(CanvasGraph {
            canvas: Canvas {
                id: canvas_row.id,
                workspace_id: canvas_row.workspace_id,
                title: canvas_row.title,
                created_at: canvas_row.created_at,
                updated_at: canvas_updated_at,
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

    async fn create_ai_bronze_record(
        &self,
        ctx: &WriteContext,
        record: NewBronzeRecord,
    ) -> StorageResult<BronzeRecord> {
        let now = Utc::now();
        self.guard.validate_write(ctx, &record.bronze_id).await?;

        let size_bytes_i64 = record.size_bytes as i64;
        let ingestion_source_type = record.ingestion_source_type.as_str();

        let inserted = sqlx::query!(
            r#"
            INSERT INTO ai_bronze_records (
                bronze_id, workspace_id, content_hash, content_type, content_encoding, size_bytes,
                original_filename, artifact_path, ingested_at, ingestion_source_type, ingestion_source_id,
                ingestion_method, external_source_json, is_deleted, deleted_at, retention_policy
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,0,NULL,$14)
            RETURNING
                bronze_id as "bronze_id!: String",
                workspace_id as "workspace_id!: String",
                content_hash as "content_hash!: String",
                content_type as "content_type!: String",
                content_encoding as "content_encoding!: String",
                size_bytes as "size_bytes!: i64",
                original_filename as "original_filename: String",
                artifact_path as "artifact_path!: String",
                ingested_at as "ingested_at!: chrono::DateTime<chrono::Utc>",
                ingestion_source_type as "ingestion_source_type!: String",
                ingestion_source_id as "ingestion_source_id: String",
                ingestion_method as "ingestion_method!: String",
                external_source_json as "external_source_json: String",
                is_deleted as "is_deleted!: i64",
                deleted_at as "deleted_at: chrono::DateTime<chrono::Utc>",
                retention_policy as "retention_policy!: String"
            "#,
            record.bronze_id,
            record.workspace_id,
            record.content_hash,
            record.content_type,
            record.content_encoding,
            size_bytes_i64,
            record.original_filename,
            record.artifact_path,
            now,
            ingestion_source_type,
            record.ingestion_source_id,
            record.ingestion_method,
            record.external_source_json,
            record.retention_policy
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(BronzeRecord {
            bronze_id: inserted.bronze_id,
            workspace_id: inserted.workspace_id,
            content_hash: inserted.content_hash,
            content_type: inserted.content_type,
            content_encoding: inserted.content_encoding,
            size_bytes: inserted.size_bytes as u64,
            original_filename: inserted.original_filename,
            artifact_path: inserted.artifact_path,
            ingested_at: inserted.ingested_at,
            ingestion_source_type: crate::ai_ready_data::records::IngestionSourceType::from_str(
                &inserted.ingestion_source_type,
            )
            .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
            ingestion_source_id: inserted.ingestion_source_id,
            ingestion_method: inserted.ingestion_method,
            external_source_json: inserted.external_source_json,
            is_deleted: inserted.is_deleted != 0,
            deleted_at: inserted.deleted_at,
            retention_policy: inserted.retention_policy,
        })
    }

    async fn get_ai_bronze_record(&self, bronze_id: &str) -> StorageResult<Option<BronzeRecord>> {
        let row = sqlx::query!(
            r#"
            SELECT
                bronze_id as "bronze_id!: String",
                workspace_id as "workspace_id!: String",
                content_hash as "content_hash!: String",
                content_type as "content_type!: String",
                content_encoding as "content_encoding!: String",
                size_bytes as "size_bytes!: i64",
                original_filename as "original_filename: String",
                artifact_path as "artifact_path!: String",
                ingested_at as "ingested_at!: chrono::DateTime<chrono::Utc>",
                ingestion_source_type as "ingestion_source_type!: String",
                ingestion_source_id as "ingestion_source_id: String",
                ingestion_method as "ingestion_method!: String",
                external_source_json as "external_source_json: String",
                is_deleted as "is_deleted!: i64",
                deleted_at as "deleted_at: chrono::DateTime<chrono::Utc>",
                retention_policy as "retention_policy!: String"
            FROM ai_bronze_records
            WHERE bronze_id = $1
            "#,
            bronze_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(BronzeRecord {
            bronze_id: row.bronze_id,
            workspace_id: row.workspace_id,
            content_hash: row.content_hash,
            content_type: row.content_type,
            content_encoding: row.content_encoding,
            size_bytes: row.size_bytes as u64,
            original_filename: row.original_filename,
            artifact_path: row.artifact_path,
            ingested_at: row.ingested_at,
            ingestion_source_type: crate::ai_ready_data::records::IngestionSourceType::from_str(
                &row.ingestion_source_type,
            )
            .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
            ingestion_source_id: row.ingestion_source_id,
            ingestion_method: row.ingestion_method,
            external_source_json: row.external_source_json,
            is_deleted: row.is_deleted != 0,
            deleted_at: row.deleted_at,
            retention_policy: row.retention_policy,
        }))
    }

    async fn list_ai_bronze_records(&self, workspace_id: &str) -> StorageResult<Vec<BronzeRecord>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                bronze_id as "bronze_id!: String",
                workspace_id as "workspace_id!: String",
                content_hash as "content_hash!: String",
                content_type as "content_type!: String",
                content_encoding as "content_encoding!: String",
                size_bytes as "size_bytes!: i64",
                original_filename as "original_filename: String",
                artifact_path as "artifact_path!: String",
                ingested_at as "ingested_at!: chrono::DateTime<chrono::Utc>",
                ingestion_source_type as "ingestion_source_type!: String",
                ingestion_source_id as "ingestion_source_id: String",
                ingestion_method as "ingestion_method!: String",
                external_source_json as "external_source_json: String",
                is_deleted as "is_deleted!: i64",
                deleted_at as "deleted_at: chrono::DateTime<chrono::Utc>",
                retention_policy as "retention_policy!: String"
            FROM ai_bronze_records
            WHERE workspace_id = $1
            ORDER BY ingested_at ASC
            "#,
            workspace_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(BronzeRecord {
                bronze_id: row.bronze_id,
                workspace_id: row.workspace_id,
                content_hash: row.content_hash,
                content_type: row.content_type,
                content_encoding: row.content_encoding,
                size_bytes: row.size_bytes as u64,
                original_filename: row.original_filename,
                artifact_path: row.artifact_path,
                ingested_at: row.ingested_at,
                ingestion_source_type:
                    crate::ai_ready_data::records::IngestionSourceType::from_str(
                        &row.ingestion_source_type,
                    )
                    .map_err(|_| StorageError::Validation("invalid ingestion_source_type"))?,
                ingestion_source_id: row.ingestion_source_id,
                ingestion_method: row.ingestion_method,
                external_source_json: row.external_source_json,
                is_deleted: row.is_deleted != 0,
                deleted_at: row.deleted_at,
                retention_policy: row.retention_policy,
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
        let res = sqlx::query!(
            r#"
            UPDATE ai_bronze_records
            SET is_deleted = 1, deleted_at = $2
            WHERE bronze_id = $1
            "#,
            bronze_id,
            now
        )
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

        let chunk_index_i64 = record.chunk_index as i64;
        let total_chunks_i64 = record.total_chunks as i64;
        let token_count_i64 = record.token_count as i64;
        let byte_start_i64 = record.byte_start as i64;
        let byte_end_i64 = record.byte_end as i64;
        let line_start_i64 = record.line_start as i64;
        let line_end_i64 = record.line_end as i64;
        let embedding_dimensions_i64 = record.embedding_dimensions as i64;
        let embedding_compute_latency_ms_i64 = record.embedding_compute_latency_ms as i64;
        let processing_duration_ms_i64 = record.processing_duration_ms as i64;
        let validation_status = record.validation_status.as_str();

        let inserted = sqlx::query!(
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
                silver_id as "silver_id!: String",
                workspace_id as "workspace_id!: String",
                bronze_ref as "bronze_ref!: String",
                chunk_index as "chunk_index!: i64",
                total_chunks as "total_chunks!: i64",
                token_count as "token_count!: i64",
                content_hash as "content_hash!: String",
                byte_start as "byte_start!: i64",
                byte_end as "byte_end!: i64",
                line_start as "line_start!: i64",
                line_end as "line_end!: i64",
                chunk_artifact_path as "chunk_artifact_path!: String",
                embedding_artifact_path as "embedding_artifact_path!: String",
                embedding_model_id as "embedding_model_id!: String",
                embedding_model_version as "embedding_model_version!: String",
                embedding_dimensions as "embedding_dimensions!: i64",
                embedding_compute_latency_ms as "embedding_compute_latency_ms!: i64",
                chunking_strategy as "chunking_strategy!: String",
                chunking_version as "chunking_version!: String",
                processing_pipeline_version as "processing_pipeline_version!: String",
                processed_at as "processed_at!: chrono::DateTime<chrono::Utc>",
                processing_duration_ms as "processing_duration_ms!: i64",
                metadata_json as "metadata_json!: String",
                validation_status as "validation_status!: String",
                validation_failed_checks_json as "validation_failed_checks_json!: String",
                validated_at as "validated_at!: chrono::DateTime<chrono::Utc>",
                validator_version as "validator_version!: String",
                is_current as "is_current!: i64",
                superseded_by as "superseded_by: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>"
            "#,
            record.silver_id,
            record.workspace_id,
            record.bronze_ref,
            chunk_index_i64,
            total_chunks_i64,
            token_count_i64,
            record.content_hash,
            byte_start_i64,
            byte_end_i64,
            line_start_i64,
            line_end_i64,
            record.chunk_artifact_path,
            record.embedding_artifact_path,
            record.embedding_model_id,
            record.embedding_model_version,
            embedding_dimensions_i64,
            embedding_compute_latency_ms_i64,
            record.chunking_strategy,
            record.chunking_version,
            record.processing_pipeline_version,
            now,
            processing_duration_ms_i64,
            record.metadata_json,
            validation_status,
            record.validation_failed_checks_json,
            now,
            record.validator_version,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SilverRecord {
            silver_id: inserted.silver_id,
            workspace_id: inserted.workspace_id,
            bronze_ref: inserted.bronze_ref,
            chunk_index: inserted.chunk_index as u32,
            total_chunks: inserted.total_chunks as u32,
            token_count: inserted.token_count as u32,
            content_hash: inserted.content_hash,
            byte_start: inserted.byte_start as u64,
            byte_end: inserted.byte_end as u64,
            line_start: inserted.line_start as u32,
            line_end: inserted.line_end as u32,
            chunk_artifact_path: inserted.chunk_artifact_path,
            embedding_artifact_path: inserted.embedding_artifact_path,
            embedding_model_id: inserted.embedding_model_id,
            embedding_model_version: inserted.embedding_model_version,
            embedding_dimensions: inserted.embedding_dimensions as u32,
            embedding_compute_latency_ms: inserted.embedding_compute_latency_ms as u64,
            chunking_strategy: inserted.chunking_strategy,
            chunking_version: inserted.chunking_version,
            processing_pipeline_version: inserted.processing_pipeline_version,
            processed_at: inserted.processed_at,
            processing_duration_ms: inserted.processing_duration_ms as u64,
            metadata_json: inserted.metadata_json,
            validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                &inserted.validation_status,
            )
            .map_err(|_| StorageError::Validation("invalid validation_status"))?,
            validation_failed_checks_json: inserted.validation_failed_checks_json,
            validated_at: inserted.validated_at,
            validator_version: inserted.validator_version,
            is_current: inserted.is_current != 0,
            superseded_by: inserted.superseded_by,
            created_at: inserted.created_at,
        })
    }

    async fn get_ai_silver_record(&self, silver_id: &str) -> StorageResult<Option<SilverRecord>> {
        let row = sqlx::query!(
            r#"
            SELECT
                silver_id as "silver_id!: String",
                workspace_id as "workspace_id!: String",
                bronze_ref as "bronze_ref!: String",
                chunk_index as "chunk_index!: i64",
                total_chunks as "total_chunks!: i64",
                token_count as "token_count!: i64",
                content_hash as "content_hash!: String",
                byte_start as "byte_start!: i64",
                byte_end as "byte_end!: i64",
                line_start as "line_start!: i64",
                line_end as "line_end!: i64",
                chunk_artifact_path as "chunk_artifact_path!: String",
                embedding_artifact_path as "embedding_artifact_path!: String",
                embedding_model_id as "embedding_model_id!: String",
                embedding_model_version as "embedding_model_version!: String",
                embedding_dimensions as "embedding_dimensions!: i64",
                embedding_compute_latency_ms as "embedding_compute_latency_ms!: i64",
                chunking_strategy as "chunking_strategy!: String",
                chunking_version as "chunking_version!: String",
                processing_pipeline_version as "processing_pipeline_version!: String",
                processed_at as "processed_at!: chrono::DateTime<chrono::Utc>",
                processing_duration_ms as "processing_duration_ms!: i64",
                metadata_json as "metadata_json!: String",
                validation_status as "validation_status!: String",
                validation_failed_checks_json as "validation_failed_checks_json!: String",
                validated_at as "validated_at!: chrono::DateTime<chrono::Utc>",
                validator_version as "validator_version!: String",
                is_current as "is_current!: i64",
                superseded_by as "superseded_by: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>"
            FROM ai_silver_records
            WHERE silver_id = $1
            "#,
            silver_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(SilverRecord {
            silver_id: row.silver_id,
            workspace_id: row.workspace_id,
            bronze_ref: row.bronze_ref,
            chunk_index: row.chunk_index as u32,
            total_chunks: row.total_chunks as u32,
            token_count: row.token_count as u32,
            content_hash: row.content_hash,
            byte_start: row.byte_start as u64,
            byte_end: row.byte_end as u64,
            line_start: row.line_start as u32,
            line_end: row.line_end as u32,
            chunk_artifact_path: row.chunk_artifact_path,
            embedding_artifact_path: row.embedding_artifact_path,
            embedding_model_id: row.embedding_model_id,
            embedding_model_version: row.embedding_model_version,
            embedding_dimensions: row.embedding_dimensions as u32,
            embedding_compute_latency_ms: row.embedding_compute_latency_ms as u64,
            chunking_strategy: row.chunking_strategy,
            chunking_version: row.chunking_version,
            processing_pipeline_version: row.processing_pipeline_version,
            processed_at: row.processed_at,
            processing_duration_ms: row.processing_duration_ms as u64,
            metadata_json: row.metadata_json,
            validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                &row.validation_status,
            )
            .map_err(|_| StorageError::Validation("invalid validation_status"))?,
            validation_failed_checks_json: row.validation_failed_checks_json,
            validated_at: row.validated_at,
            validator_version: row.validator_version,
            is_current: row.is_current != 0,
            superseded_by: row.superseded_by,
            created_at: row.created_at,
        }))
    }

    async fn list_ai_silver_records_by_bronze(
        &self,
        bronze_id: &str,
    ) -> StorageResult<Vec<SilverRecord>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                silver_id as "silver_id!: String",
                workspace_id as "workspace_id!: String",
                bronze_ref as "bronze_ref!: String",
                chunk_index as "chunk_index!: i64",
                total_chunks as "total_chunks!: i64",
                token_count as "token_count!: i64",
                content_hash as "content_hash!: String",
                byte_start as "byte_start!: i64",
                byte_end as "byte_end!: i64",
                line_start as "line_start!: i64",
                line_end as "line_end!: i64",
                chunk_artifact_path as "chunk_artifact_path!: String",
                embedding_artifact_path as "embedding_artifact_path!: String",
                embedding_model_id as "embedding_model_id!: String",
                embedding_model_version as "embedding_model_version!: String",
                embedding_dimensions as "embedding_dimensions!: i64",
                embedding_compute_latency_ms as "embedding_compute_latency_ms!: i64",
                chunking_strategy as "chunking_strategy!: String",
                chunking_version as "chunking_version!: String",
                processing_pipeline_version as "processing_pipeline_version!: String",
                processed_at as "processed_at!: chrono::DateTime<chrono::Utc>",
                processing_duration_ms as "processing_duration_ms!: i64",
                metadata_json as "metadata_json!: String",
                validation_status as "validation_status!: String",
                validation_failed_checks_json as "validation_failed_checks_json!: String",
                validated_at as "validated_at!: chrono::DateTime<chrono::Utc>",
                validator_version as "validator_version!: String",
                is_current as "is_current!: i64",
                superseded_by as "superseded_by: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>"
            FROM ai_silver_records
            WHERE bronze_ref = $1
            ORDER BY chunk_index ASC
            "#,
            bronze_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(SilverRecord {
                silver_id: row.silver_id,
                workspace_id: row.workspace_id,
                bronze_ref: row.bronze_ref,
                chunk_index: row.chunk_index as u32,
                total_chunks: row.total_chunks as u32,
                token_count: row.token_count as u32,
                content_hash: row.content_hash,
                byte_start: row.byte_start as u64,
                byte_end: row.byte_end as u64,
                line_start: row.line_start as u32,
                line_end: row.line_end as u32,
                chunk_artifact_path: row.chunk_artifact_path,
                embedding_artifact_path: row.embedding_artifact_path,
                embedding_model_id: row.embedding_model_id,
                embedding_model_version: row.embedding_model_version,
                embedding_dimensions: row.embedding_dimensions as u32,
                embedding_compute_latency_ms: row.embedding_compute_latency_ms as u64,
                chunking_strategy: row.chunking_strategy,
                chunking_version: row.chunking_version,
                processing_pipeline_version: row.processing_pipeline_version,
                processed_at: row.processed_at,
                processing_duration_ms: row.processing_duration_ms as u64,
                metadata_json: row.metadata_json,
                validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                    &row.validation_status,
                )
                .map_err(|_| StorageError::Validation("invalid validation_status"))?,
                validation_failed_checks_json: row.validation_failed_checks_json,
                validated_at: row.validated_at,
                validator_version: row.validator_version,
                is_current: row.is_current != 0,
                superseded_by: row.superseded_by,
                created_at: row.created_at,
            });
        }

        Ok(out)
    }

    async fn list_ai_silver_records(&self, workspace_id: &str) -> StorageResult<Vec<SilverRecord>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                silver_id as "silver_id!: String",
                workspace_id as "workspace_id!: String",
                bronze_ref as "bronze_ref!: String",
                chunk_index as "chunk_index!: i64",
                total_chunks as "total_chunks!: i64",
                token_count as "token_count!: i64",
                content_hash as "content_hash!: String",
                byte_start as "byte_start!: i64",
                byte_end as "byte_end!: i64",
                line_start as "line_start!: i64",
                line_end as "line_end!: i64",
                chunk_artifact_path as "chunk_artifact_path!: String",
                embedding_artifact_path as "embedding_artifact_path!: String",
                embedding_model_id as "embedding_model_id!: String",
                embedding_model_version as "embedding_model_version!: String",
                embedding_dimensions as "embedding_dimensions!: i64",
                embedding_compute_latency_ms as "embedding_compute_latency_ms!: i64",
                chunking_strategy as "chunking_strategy!: String",
                chunking_version as "chunking_version!: String",
                processing_pipeline_version as "processing_pipeline_version!: String",
                processed_at as "processed_at!: chrono::DateTime<chrono::Utc>",
                processing_duration_ms as "processing_duration_ms!: i64",
                metadata_json as "metadata_json!: String",
                validation_status as "validation_status!: String",
                validation_failed_checks_json as "validation_failed_checks_json!: String",
                validated_at as "validated_at!: chrono::DateTime<chrono::Utc>",
                validator_version as "validator_version!: String",
                is_current as "is_current!: i64",
                superseded_by as "superseded_by: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>"
            FROM ai_silver_records
            WHERE workspace_id = $1
            ORDER BY created_at ASC
            "#,
            workspace_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(SilverRecord {
                silver_id: row.silver_id,
                workspace_id: row.workspace_id,
                bronze_ref: row.bronze_ref,
                chunk_index: row.chunk_index as u32,
                total_chunks: row.total_chunks as u32,
                token_count: row.token_count as u32,
                content_hash: row.content_hash,
                byte_start: row.byte_start as u64,
                byte_end: row.byte_end as u64,
                line_start: row.line_start as u32,
                line_end: row.line_end as u32,
                chunk_artifact_path: row.chunk_artifact_path,
                embedding_artifact_path: row.embedding_artifact_path,
                embedding_model_id: row.embedding_model_id,
                embedding_model_version: row.embedding_model_version,
                embedding_dimensions: row.embedding_dimensions as u32,
                embedding_compute_latency_ms: row.embedding_compute_latency_ms as u64,
                chunking_strategy: row.chunking_strategy,
                chunking_version: row.chunking_version,
                processing_pipeline_version: row.processing_pipeline_version,
                processed_at: row.processed_at,
                processing_duration_ms: row.processing_duration_ms as u64,
                metadata_json: row.metadata_json,
                validation_status: crate::ai_ready_data::records::ValidationStatus::from_str(
                    &row.validation_status,
                )
                .map_err(|_| StorageError::Validation("invalid validation_status"))?,
                validation_failed_checks_json: row.validation_failed_checks_json,
                validated_at: row.validated_at,
                validator_version: row.validator_version,
                is_current: row.is_current != 0,
                superseded_by: row.superseded_by,
                created_at: row.created_at,
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

        let res = sqlx::query!(
            r#"
            UPDATE ai_silver_records
            SET is_current = 0, superseded_by = $2
            WHERE silver_id = $1
            "#,
            superseded_silver_id,
            new_silver_id
        )
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
        let dimensions_i64 = model.dimensions as i64;
        let max_input_tokens_i64 = model.max_input_tokens as i64;
        let status = model.status.as_str();

        sqlx::query!(
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
            model.model_id,
            model.model_version,
            dimensions_i64,
            max_input_tokens_i64,
            content_types_json,
            status,
            model.introduced_at,
            compatible_with_json
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_ai_embedding_models(&self) -> StorageResult<Vec<EmbeddingModelRecord>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                model_id as "model_id!: String",
                model_version as "model_version!: String",
                dimensions as "dimensions!: i64",
                max_input_tokens as "max_input_tokens!: i64",
                content_types_json as "content_types_json!: String",
                status as "status!: String",
                introduced_at as "introduced_at!: chrono::DateTime<chrono::Utc>",
                compatible_with_json as "compatible_with_json!: String"
            FROM ai_embedding_models
            ORDER BY model_id ASC, model_version ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let content_types: Vec<String> = serde_json::from_str(&row.content_types_json)?;
            let compatible_with: Vec<String> = serde_json::from_str(&row.compatible_with_json)?;
            out.push(EmbeddingModelRecord {
                model_id: row.model_id,
                model_version: row.model_version,
                dimensions: row.dimensions as u32,
                max_input_tokens: row.max_input_tokens as u32,
                content_types,
                status: crate::ai_ready_data::records::EmbeddingModelStatus::from_str(&row.status)
                    .map_err(|_| StorageError::Validation("invalid embedding model status"))?,
                introduced_at: row.introduced_at,
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

        sqlx::query!(
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
            model_id,
            model_version,
            now
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_ai_embedding_registry(&self) -> StorageResult<Option<EmbeddingRegistry>> {
        let row = sqlx::query!(
            r#"
            SELECT
                current_default_model_id as "current_default_model_id!: String",
                current_default_model_version as "current_default_model_version!: String",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            FROM ai_embedding_registry
            WHERE id = 'global'
            "#
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| EmbeddingRegistry {
            current_default_model_id: r.current_default_model_id,
            current_default_model_version: r.current_default_model_version,
            updated_at: r.updated_at,
        }))
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
        validate_job_contract(&job.job_kind, &job.profile_id, &job.protocol_id)?;

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

    async fn update_ai_job_mcp_fields(
        &self,
        job_id: Uuid,
        update: AiJobMcpUpdate,
    ) -> StorageResult<()> {
        let now = Utc::now();
        let mcp_server_id = update.mcp_server_id.clone();
        let mcp_call_id = update.mcp_call_id.clone();
        let mcp_progress_token = update.mcp_progress_token.clone();
        let job_id = job_id.to_string();

        let result = sqlx::query(
            r#"
            UPDATE ai_jobs
            SET mcp_server_id = COALESCE($1, mcp_server_id),
                mcp_call_id = COALESCE($2, mcp_call_id),
                mcp_progress_token = COALESCE($3, mcp_progress_token),
                updated_at = $4
            WHERE id = $5
            "#,
        )
        .bind(mcp_server_id)
        .bind(mcp_call_id)
        .bind(mcp_progress_token)
        .bind(now)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound("ai_job"));
        }

        Ok(())
    }

    async fn get_ai_job_mcp_fields(&self, job_id: Uuid) -> StorageResult<AiJobMcpFields> {
        let job_id = job_id.to_string();
        let row = sqlx::query(
            r#"
            SELECT mcp_server_id, mcp_call_id, mcp_progress_token
            FROM ai_jobs
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(StorageError::NotFound("ai_job"));
        };

        Ok(AiJobMcpFields {
            mcp_server_id: row.get("mcp_server_id"),
            mcp_call_id: row.get("mcp_call_id"),
            mcp_progress_token: row.get("mcp_progress_token"),
        })
    }

    async fn find_ai_job_id_by_mcp_progress_token(
        &self,
        progress_token: &str,
    ) -> StorageResult<Option<Uuid>> {
        let id: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id
            FROM ai_jobs
            WHERE mcp_progress_token = $1
            LIMIT 1
            "#,
        )
        .bind(progress_token)
        .fetch_optional(&self.pool)
        .await?;

        id.map(|id| Uuid::parse_str(&id).map_err(|_| StorageError::Validation("invalid job_id uuid")))
            .transpose()
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
            report.items_spared_window += deletable_count.saturating_sub(actual_to_delete);
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
