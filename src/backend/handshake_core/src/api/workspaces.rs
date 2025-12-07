use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    models::{
        BlockResponse, CreateDocumentRequest, CreateWorkspaceRequest, DocumentResponse,
        DocumentWithBlocksResponse, ErrorResponse, UpsertBlocksRequest, WorkspaceResponse,
    },
    AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/workspaces", post(create_workspace).get(list_workspaces))
        .route(
            "/workspaces/:workspace_id/documents",
            post(create_document).get(list_documents),
        )
        .route("/documents/:document_id", get(get_document))
        .route("/documents/:document_id/blocks", put(replace_blocks))
        .with_state(state)
}

async fn create_workspace(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();

    sqlx::query!(
        r#"
        INSERT INTO workspaces (id, name, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        id,
        payload.name,
        now,
        now
    )
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(WorkspaceResponse {
            id,
            name: payload.name,
            created_at: now,
            updated_at: now,
        }),
    ))
}

async fn list_workspaces(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let rows = sqlx::query!(
        r#"
        SELECT
            id as "id!: String",
            name as "name!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM workspaces
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let workspaces = rows
        .into_iter()
        .map(|row| WorkspaceResponse {
            id: row.id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(workspaces))
}

async fn create_document(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace(&state.pool, &workspace_id).await?;

    let now = Utc::now();
    let id = Uuid::new_v4().to_string();

    sqlx::query!(
        r#"
        INSERT INTO documents (id, workspace_id, title, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        id,
        workspace_id,
        payload.title,
        now,
        now
    )
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
            id,
            workspace_id,
            title: payload.title,
            created_at: now,
            updated_at: now,
        }),
    ))
}

async fn list_documents(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<DocumentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace(&state.pool, &workspace_id).await?;

    let rows = sqlx::query!(
        r#"
        SELECT
            id as "id!: String",
            workspace_id as "workspace_id!: String",
            title as "title!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM documents
        WHERE workspace_id = ?1
        ORDER BY created_at ASC
        "#,
        workspace_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let docs = rows
        .into_iter()
        .map(|row| DocumentResponse {
            id: row.id,
            workspace_id: row.workspace_id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(docs))
}

async fn get_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Json<DocumentWithBlocksResponse>, (StatusCode, Json<ErrorResponse>)> {
    let doc = sqlx::query!(
        r#"
        SELECT
            id as "id!: String",
            workspace_id as "workspace_id!: String",
            title as "title!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM documents
        WHERE id = ?1
        "#,
        document_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    let doc = match doc {
        Some(row) => row,
        None => return Err(not_found("document_not_found")),
    };

    let blocks = sqlx::query!(
        r#"
        SELECT
            id as "id!: String",
            kind as "kind!: String",
            sequence as "sequence!: i64",
            raw_content as "raw_content!: String",
            display_content as "display_content!: String",
            derived_content as "derived_content!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM blocks
        WHERE document_id = ?1
        ORDER BY sequence ASC
        "#,
        doc.id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    let blocks = blocks
        .into_iter()
        .map(|b| BlockResponse {
            id: b.id,
            kind: b.kind,
            sequence: b.sequence,
            raw_content: b.raw_content,
            display_content: b.display_content,
            derived_content: serde_json::from_str(&b.derived_content)
                .unwrap_or(Value::Object(Default::default())),
            created_at: b.created_at,
            updated_at: b.updated_at,
        })
        .collect();

    Ok(Json(DocumentWithBlocksResponse {
        id: doc.id,
        workspace_id: doc.workspace_id,
        title: doc.title,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
        blocks,
    }))
}

async fn replace_blocks(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Json(payload): Json<UpsertBlocksRequest>,
) -> Result<Json<Vec<BlockResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let doc = sqlx::query!(
        r#"
        SELECT
            id as "id!: String",
            workspace_id as "workspace_id!: String"
        FROM documents
        WHERE id = ?1
        "#,
        document_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    if doc.is_none() {
        return Err(not_found("document_not_found"));
    }

    let mut tx = state.pool.begin().await.map_err(internal_error)?;

    sqlx::query!(r#"DELETE FROM blocks WHERE document_id = ?1"#, document_id)
        .execute(&mut *tx)
        .await
        .map_err(internal_error)?;

    let mut result_blocks = Vec::with_capacity(payload.blocks.len());
    for incoming in payload.blocks.into_iter() {
        let block_id = incoming.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();
        let display_content = incoming
            .display_content
            .unwrap_or_else(|| incoming.raw_content.clone());
        let derived = incoming
            .derived_content
            .unwrap_or_else(|| Value::Object(Default::default()));
        let derived_str = derived.to_string();

        sqlx::query!(
            r#"
            INSERT INTO blocks (
                id, document_id, kind, sequence, raw_content, display_content, derived_content, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            block_id,
            document_id,
            incoming.kind,
            incoming.sequence,
            incoming.raw_content,
            display_content,
            derived_str,
            now,
            now
        )
            .execute(&mut *tx)
            .await
            .map_err(internal_error)?;

        result_blocks.push(BlockResponse {
            id: block_id,
            kind: incoming.kind,
            sequence: incoming.sequence,
            raw_content: incoming.raw_content,
            display_content,
            derived_content: derived,
            created_at: now,
            updated_at: now,
        });
    }

    tx.commit().await.map_err(internal_error)?;

    Ok(Json(result_blocks))
}

pub(super) async fn ensure_workspace(
    pool: &SqlitePool,
    workspace_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT COUNT(1) as "count!: i64"
        FROM workspaces
        WHERE id = ?1
        "#,
        workspace_id
    )
    .fetch_one(pool)
    .await
    .map_err(internal_error)?;

    if exists == 0 {
        Err(not_found("workspace_not_found"))
    } else {
        Ok(())
    }
}

pub(super) fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    eprintln!("db_error: {}", err);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: "db_error" }),
    )
}

pub(super) fn not_found(code: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}
