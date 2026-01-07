use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::{
    models::{
        BlockResponse, CreateDocumentRequest, CreateWorkspaceRequest, DocumentResponse,
        DocumentWithBlocksResponse, ErrorResponse, UpsertBlocksRequest, WorkspaceResponse,
    },
    storage::{Block, NewBlock, NewDocument, NewWorkspace, StorageError, WriteContext},
    AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/workspaces", post(create_workspace).get(list_workspaces))
        .route(
            "/workspaces/:workspace_id/documents",
            post(create_document).get(list_documents),
        )
        .route(
            "/documents/:document_id",
            get(get_document).delete(delete_document),
        )
        .route("/documents/:document_id/blocks", put(replace_blocks))
        .route("/workspaces/:workspace_id", delete(delete_workspace))
        .with_state(state)
}

async fn create_workspace(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = WriteContext::human(None);
    let workspace = state
        .storage
        .create_workspace(
            &ctx,
            NewWorkspace {
                name: payload.name.clone(),
            },
        )
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces", status = "created", workspace_id = %workspace.id, "workspace created");

    Ok((
        StatusCode::CREATED,
        Json(WorkspaceResponse {
            id: workspace.id,
            name: workspace.name,
            created_at: workspace.created_at,
            updated_at: workspace.updated_at,
        }),
    ))
}

async fn list_workspaces(
    State(state): State<AppState>,
) -> Result<Json<Vec<WorkspaceResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let rows = state
        .storage
        .list_workspaces()
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces", status = "ok", count = rows.len(), "list workspaces");

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

async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = WriteContext::human(None);
    state
        .storage
        .delete_workspace(&ctx, &workspace_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id", status = "deleted", workspace_id = %workspace_id, "workspace deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn create_document(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);

    let document = state
        .storage
        .create_document(
            &ctx,
            NewDocument {
                workspace_id: workspace_id.clone(),
                title: payload.title.clone(),
            },
        )
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id/documents", status = "created", workspace_id = %workspace_id, document_id = %document.id, "document created");

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
            id: document.id,
            workspace_id: document.workspace_id,
            title: document.title,
            created_at: document.created_at,
            updated_at: document.updated_at,
        }),
    ))
}

async fn list_documents(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<DocumentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let rows = state
        .storage
        .list_documents(&workspace_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id/documents", status = "ok", workspace_id = %workspace_id, count = rows.len(), "list documents");

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
    let document = state
        .storage
        .get_document(&document_id)
        .await
        .map_err(map_storage_error)?;

    let blocks = state
        .storage
        .get_blocks(&document_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/documents/:document_id", status = "ok", document_id = %document.id, "get document");

    let blocks: Vec<BlockResponse> = blocks.into_iter().map(block_to_response).collect();

    Ok(Json(DocumentWithBlocksResponse {
        id: document.id,
        workspace_id: document.workspace_id,
        title: document.title,
        created_at: document.created_at,
        updated_at: document.updated_at,
        blocks,
    }))
}

async fn delete_document(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = WriteContext::human(None);
    state
        .storage
        .delete_document(&ctx, &document_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/documents/:document_id", status = "deleted", document_id = %document_id, "document deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn replace_blocks(
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Json(payload): Json<UpsertBlocksRequest>,
) -> Result<Json<Vec<BlockResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Ensure document exists first to provide 404 instead of recreating.
    let ctx = WriteContext::human(None);
    let _doc = state
        .storage
        .get_document(&document_id)
        .await
        .map_err(map_storage_error)?;

    let incoming_blocks: Vec<NewBlock> = payload
        .blocks
        .into_iter()
        .map(|incoming| NewBlock {
            id: incoming.id,
            document_id: document_id.clone(),
            kind: incoming.kind,
            sequence: incoming.sequence,
            raw_content: incoming.raw_content.clone(),
            display_content: incoming.display_content,
            derived_content: incoming.derived_content,
            sensitivity: None,
            exportable: None,
        })
        .collect();

    let result_blocks = state
        .storage
        .replace_blocks(&ctx, &document_id, incoming_blocks)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/documents/:document_id/blocks", status = "ok", document_id = %document_id, blocks = result_blocks.len(), "replace blocks");

    Ok(Json(
        result_blocks.into_iter().map(block_to_response).collect(),
    ))
}

fn block_to_response(block: Block) -> BlockResponse {
    BlockResponse {
        id: block.id,
        kind: block.kind,
        sequence: block.sequence,
        raw_content: block.raw_content,
        display_content: block.display_content,
        derived_content: block.derived_content,
        created_at: block.created_at,
        updated_at: block.updated_at,
    }
}

async fn ensure_workspace_exists(
    state: &AppState,
    workspace_id: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(map_storage_error(err)),
    }
}

fn map_storage_error(err: StorageError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        StorageError::NotFound(code) => not_found(code),
        StorageError::Guard(_) | StorageError::Validation("HSK-403-SILENT-EDIT") => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "HSK-403-SILENT-EDIT",
            }),
        ),
        StorageError::Validation(_) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "bad_request",
            }),
        ),
        _ => internal_error(err),
    }
}

fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(target: "handshake_core", error = %err, "db_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: "db_error" }),
    )
}

fn not_found(code: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}
