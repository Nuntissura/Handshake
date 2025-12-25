use super::{
    AiJob, Block, BlockUpdate, Canvas, CanvasEdge, CanvasGraph, CanvasNode, Document,
    JobStatusUpdate, MutationMetadata, NewAiJob, NewBlock, NewCanvas, NewCanvasEdge, NewCanvasNode,
    NewDocument, NewWorkspace, StorageError, StorageResult, Workspace, WriteActor,
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::sync::Arc;
use uuid::Uuid;

/// SQLite-backed implementation of the Database trait.
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn connect(db_url: &str, max_connections: u32) -> StorageResult<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect(db_url)
            .await?;
        Ok(Self { pool })
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn into_arc(self) -> Arc<dyn super::Database> {
        Arc::new(self)
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn run_migrations(&self) -> StorageResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl super::Database for SqliteDatabase {
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

    async fn create_workspace(&self, workspace: NewWorkspace) -> StorageResult<Workspace> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        let inserted = sqlx::query!(
            r#"
            INSERT INTO workspaces (id, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4)
            RETURNING
                id as "id!: String",
                name as "name!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            workspace.name,
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

    async fn delete_workspace(&self, id: &str) -> StorageResult<()> {
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

    async fn create_document(&self, doc: NewDocument) -> StorageResult<Document> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        let inserted = sqlx::query!(
            r#"
            INSERT INTO documents (id, workspace_id, title, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            doc.workspace_id,
            doc.title,
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

    async fn delete_document(&self, doc_id: &str) -> StorageResult<()> {
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
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
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
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
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
                })
            }
            None => Err(StorageError::NotFound("block")),
        }
    }
    async fn create_block(&self, block: NewBlock) -> StorageResult<Block> {
        let now = Utc::now();
        let id = block.id.map_or_else(|| Uuid::new_v4().to_string(), |v| v);
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
                id, document_id, kind, sequence, raw_content, display_content, derived_content, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id as "id!: String",
                document_id as "document_id!: String",
                kind as "kind!: String",
                sequence as "sequence!: i64",
                raw_content as "raw_content!: String",
                display_content as "display_content!: String",
                derived_content as "derived_content!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            block.document_id,
            block.kind,
            block.sequence,
            block.raw_content,
            display_content,
            derived_content,
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
        })
    }

    async fn update_block(&self, block_id: &str, data: BlockUpdate) -> StorageResult<()> {
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

        sqlx::query!(
            r#"
            UPDATE blocks
            SET kind = $1, sequence = $2, raw_content = $3, display_content = $4, derived_content = $5, updated_at = $6
            WHERE id = $7
            "#,
            block.kind,
            block.sequence,
            block.raw_content,
            block.display_content,
            derived_content,
            now,
            block.id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_block(&self, block_id: &str) -> StorageResult<()> {
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
                    id, document_id, kind, sequence, raw_content, display_content, derived_content, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING
                    id as "id!: String",
                    document_id as "document_id!: String",
                    kind as "kind!: String",
                    sequence as "sequence!: i64",
                    raw_content as "raw_content!: String",
                    display_content as "display_content!: String",
                    derived_content as "derived_content!: String",
                    created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                    updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
                "#,
                id,
                document_id,
                block.kind,
                block.sequence,
                block.raw_content,
                display_content,
                derived_content,
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
            });
        }

        tx.commit().await?;
        Ok(inserted)
    }
    async fn create_canvas(&self, canvas: NewCanvas) -> StorageResult<Canvas> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        let row = sqlx::query!(
            r#"
            INSERT INTO canvases (id, workspace_id, title, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id as "id!: String",
                workspace_id as "workspace_id!: String",
                title as "title!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            canvas.workspace_id,
            canvas.title,
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
            let data = node
                .data
                .map_or_else(|| Value::Object(Default::default()), |v| v)
                .to_string();

            let row = sqlx::query!(
                r#"
                INSERT INTO canvas_nodes (
                    id, canvas_id, kind, position_x, position_y, data, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING
                    id as "id!: String",
                    canvas_id as "canvas_id!: String",
                    kind as "kind!: String",
                    position_x as "position_x!: f64",
                    position_y as "position_y!: f64",
                    data as "data!: String",
                    created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                    updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
                "#,
                id,
                canvas_id,
                node.kind,
                node.position_x,
                node.position_y,
                data,
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

            let row = sqlx::query!(
                r#"
                INSERT INTO canvas_edges (
                    id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING
                    id as "id!: String",
                    canvas_id as "canvas_id!: String",
                    from_node_id as "from_node_id!: String",
                    to_node_id as "to_node_id!: String",
                    kind as "kind!: String",
                    created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                    updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
                "#,
                id,
                canvas_id,
                edge.from_node_id,
                edge.to_node_id,
                edge.kind,
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

    async fn delete_canvas(&self, canvas_id: &str) -> StorageResult<()> {
        let res = sqlx::query!(r#"DELETE FROM canvases WHERE id = $1"#, canvas_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound("canvas"));
        }
        Ok(())
    }
    async fn get_ai_job(&self, job_id: &str) -> StorageResult<AiJob> {
        let row = sqlx::query!(
            r#"
            SELECT
                id as "id!: String",
                job_kind as "job_kind!: String",
                status as "status!: String",
                error_message as "error_message: String",
                protocol_id as "protocol_id!: String",
                profile_id as "profile_id!: String",
                capability_profile_id as "capability_profile_id!: String",
                access_mode as "access_mode!: String",
                safety_mode as "safety_mode!: String",
                job_inputs as "job_inputs: String",
                job_outputs as "job_outputs: String",
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
            Some(row) => Ok(AiJob {
                id: row.id,
                job_kind: row.job_kind,
                status: row.status,
                error_message: row.error_message,
                protocol_id: row.protocol_id,
                profile_id: row.profile_id,
                capability_profile_id: row.capability_profile_id,
                access_mode: row.access_mode,
                safety_mode: row.safety_mode,
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
            }),
            None => Err(StorageError::NotFound("ai_job")),
        }
    }

    async fn create_ai_job(&self, job: NewAiJob) -> StorageResult<AiJob> {
        let id = Uuid::new_v4().to_string();
        let status = "queued".to_string();
        let now = Utc::now();
        let job_inputs = job.job_inputs.clone().map(|value| value.to_string());

        let row = sqlx::query!(
            r#"
            INSERT INTO ai_jobs (
                id, job_kind, status, protocol_id, profile_id, capability_profile_id, access_mode, safety_mode, job_inputs, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id as "id!: String",
                job_kind as "job_kind!: String",
                status as "status!: String",
                error_message as "error_message: String",
                protocol_id as "protocol_id!: String",
                profile_id as "profile_id!: String",
                capability_profile_id as "capability_profile_id!: String",
                access_mode as "access_mode!: String",
                safety_mode as "safety_mode!: String",
                job_inputs as "job_inputs: String",
                job_outputs as "job_outputs: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            job.job_kind,
            status,
            job.protocol_id,
            job.profile_id,
            job.capability_profile_id,
            job.access_mode,
            job.safety_mode,
            job_inputs,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(AiJob {
            id: row.id,
            job_kind: row.job_kind,
            status: row.status,
            error_message: row.error_message,
            protocol_id: row.protocol_id,
            profile_id: row.profile_id,
            capability_profile_id: row.capability_profile_id,
            access_mode: row.access_mode,
            safety_mode: row.safety_mode,
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

    async fn update_ai_job_status(&self, update: JobStatusUpdate) -> StorageResult<()> {
        let job_outputs = update.job_outputs.as_ref().map(|val| val.to_string());
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE ai_jobs
            SET status = $1,
                error_message = $2,
                job_outputs = COALESCE($3, job_outputs),
                updated_at = $4
            WHERE id = $5
            "#,
            update.status,
            update.error_message,
            job_outputs,
            now,
            update.job_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
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
        job_id: &str,
        status: &str,
    ) -> StorageResult<super::WorkflowRun> {
        let id = Uuid::new_v4().to_string();
        let row = sqlx::query!(
            r#"
            INSERT INTO workflow_runs (id, job_id, status)
            VALUES ($1, $2, $3)
            RETURNING
                id as "id!: String",
                job_id as "job_id!: String",
                status as "status!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            id,
            job_id,
            status
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(super::WorkflowRun {
            id: row.id,
            job_id: row.job_id,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn update_workflow_run_status(
        &self,
        run_id: &str,
        status: &str,
        error_message: Option<String>,
    ) -> StorageResult<super::WorkflowRun> {
        let now = Utc::now();
        let row = sqlx::query!(
            r#"
            UPDATE workflow_runs
            SET status = $1,
                updated_at = $2
            WHERE id = $3
            RETURNING
                id as "id!: String",
                job_id as "job_id!: String",
                status as "status!: String",
                created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at!: chrono::DateTime<chrono::Utc>"
            "#,
            status,
            now,
            run_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if row.is_none() {
            return Err(StorageError::NotFound("workflow_run"));
        }

        if let Some(message) = error_message {
            sqlx::query!(
                r#"
                UPDATE ai_jobs
                SET error_message = $1,
                    updated_at = $2
                WHERE id = (SELECT job_id FROM workflow_runs WHERE id = $3)
                "#,
                message,
                now,
                run_id
            )
            .execute(&self.pool)
            .await?;
        }

        let Some(row) = row else {
            return Err(StorageError::NotFound("workflow_run"));
        };
        Ok(super::WorkflowRun {
            id: row.id,
            job_id: row.job_id,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn validate_write_with_guard(
        &self,
        actor: &WriteActor,
        job_id: Option<uuid::Uuid>,
        resource_id: &str,
    ) -> StorageResult<MutationMetadata> {
        // Records metadata to preserve traceability; enforcement hooks will be added when guard rules are finalized.
        Ok(MutationMetadata {
            actor: actor.clone(),
            job_id,
            resource_id: resource_id.to_string(),
            timestamp: Utc::now(),
        })
    }

    fn sqlite_pool(&self) -> Option<&SqlitePool> {
        Some(&self.pool)
    }
}
