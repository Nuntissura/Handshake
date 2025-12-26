use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    models::{
        CanvasEdgeResponse, CanvasNodeResponse, CanvasResponse, CanvasWithGraphResponse,
        CreateCanvasRequest, ErrorResponse,
    },
    storage::{
        CanvasEdge, CanvasGraph, CanvasNode, NewCanvas, NewCanvasEdge, NewCanvasNode, StorageError,
        WriteContext,
    },
    AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/canvases",
            post(create_canvas).get(list_canvases),
        )
        .route(
            "/canvases/:canvas_id",
            get(get_canvas)
                .put(update_canvas_graph)
                .delete(delete_canvas),
        )
        .with_state(state)
}

#[derive(Deserialize)]
struct UpdateCanvasGraphRequest {
    nodes: Vec<IncomingCanvasNode>,
    edges: Vec<IncomingCanvasEdge>,
}

#[derive(Deserialize)]
struct IncomingCanvasNode {
    id: Option<String>,
    kind: String,
    position_x: f64,
    position_y: f64,
    data: Option<Value>,
}

#[derive(Deserialize)]
struct IncomingCanvasEdge {
    id: Option<String>,
    from_node_id: String,
    to_node_id: String,
    kind: String,
}

async fn delete_canvas(
    State(state): State<AppState>,
    Path(canvas_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = WriteContext::human(None);
    state
        .storage
        .delete_canvas(&ctx, &canvas_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "deleted", canvas_id = %canvas_id, "canvas deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn create_canvas(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateCanvasRequest>,
) -> Result<(StatusCode, Json<CanvasResponse>), (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let ctx = WriteContext::human(None);

    let canvas = state
        .storage
        .create_canvas(
            &ctx,
            NewCanvas {
                workspace_id: workspace_id.clone(),
                title: payload.title.clone(),
            },
        )
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id/canvases", status = "created", workspace_id = %workspace_id, canvas_id = %canvas.id, "canvas created");

    Ok((
        StatusCode::CREATED,
        Json(CanvasResponse {
            id: canvas.id,
            workspace_id: canvas.workspace_id,
            title: canvas.title,
            created_at: canvas.created_at,
            updated_at: canvas.updated_at,
        }),
    ))
}

async fn list_canvases(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<CanvasResponse>>, (StatusCode, Json<ErrorResponse>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;

    let rows = state
        .storage
        .list_canvases(&workspace_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id/canvases", status = "ok", workspace_id = %workspace_id, count = rows.len(), "list canvases");

    let canvases = rows
        .into_iter()
        .map(|row| CanvasResponse {
            id: row.id,
            workspace_id: row.workspace_id,
            title: row.title,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(Json(canvases))
}

async fn get_canvas(
    State(state): State<AppState>,
    Path(canvas_id): Path<String>,
) -> Result<Json<CanvasWithGraphResponse>, (StatusCode, Json<ErrorResponse>)> {
    let graph = state
        .storage
        .get_canvas_with_graph(&canvas_id)
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "ok", canvas_id = %canvas_id, "get canvas");

    Ok(Json(graph_to_response(graph)))
}

async fn update_canvas_graph(
    State(state): State<AppState>,
    Path(canvas_id): Path<String>,
    Json(payload): Json<UpdateCanvasGraphRequest>,
) -> Result<Json<CanvasWithGraphResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = WriteContext::human(None);
    let graph = state
        .storage
        .update_canvas_graph(
            &ctx,
            &canvas_id,
            payload
                .nodes
                .into_iter()
                .map(|incoming| NewCanvasNode {
                    id: incoming.id,
                    kind: incoming.kind,
                    position_x: incoming.position_x,
                    position_y: incoming.position_y,
                    data: incoming.data,
                })
                .collect(),
            payload
                .edges
                .into_iter()
                .map(|incoming| NewCanvasEdge {
                    id: incoming.id,
                    from_node_id: incoming.from_node_id,
                    to_node_id: incoming.to_node_id,
                    kind: incoming.kind,
                })
                .collect(),
        )
        .await
        .map_err(map_storage_error)?;

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "ok", canvas_id = %canvas_id, nodes = graph.nodes.len(), edges = graph.edges.len(), "update canvas graph");

    Ok(Json(graph_to_response(graph)))
}

fn graph_to_response(graph: CanvasGraph) -> CanvasWithGraphResponse {
    CanvasWithGraphResponse {
        id: graph.canvas.id,
        workspace_id: graph.canvas.workspace_id,
        title: graph.canvas.title,
        created_at: graph.canvas.created_at,
        updated_at: graph.canvas.updated_at,
        nodes: graph.nodes.into_iter().map(node_to_response).collect(),
        edges: graph.edges.into_iter().map(edge_to_response).collect(),
    }
}

fn node_to_response(node: CanvasNode) -> CanvasNodeResponse {
    CanvasNodeResponse {
        id: node.id,
        canvas_id: node.canvas_id,
        kind: node.kind,
        position_x: node.position_x,
        position_y: node.position_y,
        data: node.data,
        created_at: node.created_at,
        updated_at: node.updated_at,
    }
}

fn edge_to_response(edge: CanvasEdge) -> CanvasEdgeResponse {
    CanvasEdgeResponse {
        id: edge.id,
        canvas_id: edge.canvas_id,
        from_node_id: edge.from_node_id,
        to_node_id: edge.to_node_id,
        kind: edge.kind,
        created_at: edge.created_at,
        updated_at: edge.updated_at,
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
