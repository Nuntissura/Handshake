//! Legacy canvas HTTP surface (`canvases`, `canvas_nodes`, `canvas_edges`).
//!
//! AUTHORITY (MT-154; Master Spec 02-system-architecture.md:2758/2773/2776, LM-RLS-001/002):
//! every route requires an authenticated account session bound to the live native channel and
//! authorizes the canvas's workspace through the ResourceBroker (Read+fs.read for reads,
//! Create/Update+fs.write for writes, Delete+fs.write for deletes) before any protected table
//! access. `/canvases/:canvas_id` carries no workspace, so the canvas's workspace is resolved only
//! through the caller's record-user permissions: a canvas the caller cannot read is answered with
//! the constant denial, never a 404 that would disclose its existence. Every read and write runs
//! as the account's record user (`with_record_user_scope`), the write actor is the session
//! principal (never a header-supplied actor), and a write the table permissions silently drop is
//! answered with the constant denial.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    api::authority::AuthorizedResourceContext,
    models::{
        CanvasEdgeResponse, CanvasNodeResponse, CanvasResponse, CanvasWithGraphResponse,
        CreateCanvasRequest, ErrorResponse,
    },
    storage::{
        surreal::resource_authority::{RecordUserScope, ResourceAction, ResourceKind},
        CanvasEdge, CanvasGraph, CanvasNode, NewCanvas, NewCanvasEdge, NewCanvasNode, StorageError,
        WriteContext,
    },
    AppState,
};

type ApiError = (StatusCode, Json<ErrorResponse>);

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/canvases",
            post(create_canvas).get(list_canvases),
        )
        .route(
            "/canvases/:canvas_id",
            get(get_canvas)
                .patch(rename_canvas)
                .put(update_canvas_graph)
                .delete(delete_canvas),
        )
        // Deny by default before any extractor, body parse or table access (AC-154-2).
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::authority::require_authenticated_session,
        ))
        .with_state(state)
}

/// The constant protected-resource denial (same body as `api::authority::constant_denial`).
fn protected_denial() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: "HSK-403-PROTECTED-RESOURCE",
        }),
    )
}

/// Authorizes `action` on the workspace through the ResourceBroker before any table access.
/// Reads use `fs.read`; create/update/delete use `fs.write`. A missing, foreign or unauthorized
/// workspace is indistinguishable: every failure is the constant denial.
async fn workspace_authority(
    state: &AppState,
    headers: &HeaderMap,
    workspace_id: &str,
    action: ResourceAction,
) -> Result<AuthorizedResourceContext, ApiError> {
    let capability = if matches!(action, ResourceAction::Read) {
        "fs.read"
    } else {
        "fs.write"
    };
    crate::api::authority::authorize_request(
        state,
        headers,
        capability,
        ResourceKind::Workspace,
        workspace_id,
        action,
    )
    .await
    .map_err(|_| protected_denial())
}

/// `/canvases/:canvas_id`: resolve the canvas's workspace ONLY through the caller's record-user
/// permissions (the `canvases` select predicate requires workspace read), so an unreadable canvas
/// is indistinguishable from an absent one, then authorize `action` on that workspace.
async fn canvas_authority(
    state: &AppState,
    headers: &HeaderMap,
    canvas_id: &str,
    action: ResourceAction,
) -> Result<AuthorizedResourceContext, ApiError> {
    let credentials = crate::api::authority::authenticated_session_credentials(state, headers)
        .await
        .map_err(|_| protected_denial())?;
    let lookup = RecordUserScope {
        grant_id: None,
        workspace_id: None,
        session_token: credentials.session_token,
        channel_binding_hash: Some(credentials.channel_binding_hash),
        resource_id: String::new(),
        session_id: credentials.context.session_id,
        capability_id: "fs.read".to_owned(),
        action: ResourceAction::Read,
    };
    let workspace_id = state
        .surreal
        .with_record_user_scope(lookup, state.storage.get_canvas_with_graph(canvas_id))
        .await
        .map(|graph| graph.canvas.workspace_id)
        .map_err(|_| protected_denial())?;
    workspace_authority(state, headers, &workspace_id, action).await
}

/// The write actor is the authenticated session principal, never a header-supplied identity.
fn session_write_context(authority: &AuthorizedResourceContext) -> Result<WriteContext, ApiError> {
    let actor_id = Some(authority.actor_id.clone());
    match authority.actor_kind.as_str() {
        "operator" => Ok(WriteContext::human(actor_id)),
        "system" => Ok(WriteContext::system(actor_id)),
        _ => Err(protected_denial()),
    }
}

#[derive(Deserialize)]
struct UpdateCanvasGraphRequest {
    nodes: Vec<IncomingCanvasNode>,
    edges: Vec<IncomingCanvasEdge>,
}

#[derive(Deserialize)]
struct RenameCanvasRequest {
    title: String,
    #[serde(default)]
    expected_updated_at: Option<DateTime<Utc>>,
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
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let authority = canvas_authority(&state, &headers, &canvas_id, ResourceAction::Delete).await?;
    let ctx = session_write_context(&authority)?;
    state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            state.storage.delete_canvas(&ctx, &canvas_id),
        )
        .await
        .map_err(map_canvas_error)?;

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "deleted", canvas_id = %canvas_id, "canvas deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn create_canvas(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CreateCanvasRequest>,
) -> Result<(StatusCode, Json<CanvasResponse>), ApiError> {
    let authority =
        workspace_authority(&state, &headers, &workspace_id, ResourceAction::Create).await?;
    let ctx = session_write_context(&authority)?;
    let canvas = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            state.storage.create_canvas(
                &ctx,
                NewCanvas {
                    workspace_id: workspace_id.clone(),
                    title: payload.title.clone(),
                },
            ),
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
    headers: HeaderMap,
) -> Result<Json<Vec<CanvasResponse>>, ApiError> {
    let authority =
        workspace_authority(&state, &headers, &workspace_id, ResourceAction::Read).await?;
    let rows = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            state.storage.list_canvases(&workspace_id),
        )
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
    headers: HeaderMap,
) -> Result<Json<CanvasWithGraphResponse>, ApiError> {
    let authority = canvas_authority(&state, &headers, &canvas_id, ResourceAction::Read).await?;
    let graph = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            state.storage.get_canvas_with_graph(&canvas_id),
        )
        .await
        .map_err(map_canvas_error)?;

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "ok", canvas_id = %canvas_id, "get canvas");

    Ok(Json(graph_to_response(graph)))
}

async fn rename_canvas(
    State(state): State<AppState>,
    Path(canvas_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<RenameCanvasRequest>,
) -> Result<Json<CanvasResponse>, ApiError> {
    let authority = canvas_authority(&state, &headers, &canvas_id, ResourceAction::Update).await?;
    if payload.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "canvas_title_empty",
            }),
        ));
    }
    let ctx = session_write_context(&authority)?;
    let canvas = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            state.storage.rename_canvas(
                &ctx,
                &canvas_id,
                payload.title.trim(),
                payload.expected_updated_at,
            ),
        )
        .await
        .map_err(map_canvas_error)?;
    Ok(Json(CanvasResponse {
        id: canvas.id,
        workspace_id: canvas.workspace_id,
        title: canvas.title,
        created_at: canvas.created_at,
        updated_at: canvas.updated_at,
    }))
}

async fn update_canvas_graph(
    State(state): State<AppState>,
    Path(canvas_id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateCanvasGraphRequest>,
) -> Result<Json<CanvasWithGraphResponse>, ApiError> {
    let authority = canvas_authority(&state, &headers, &canvas_id, ResourceAction::Update).await?;
    let ctx = session_write_context(&authority)?;
    let nodes = payload
        .nodes
        .into_iter()
        .map(|incoming| NewCanvasNode {
            id: incoming.id,
            kind: incoming.kind,
            position_x: incoming.position_x,
            position_y: incoming.position_y,
            data: incoming.data,
        })
        .collect();
    let edges = payload
        .edges
        .into_iter()
        .map(|incoming| NewCanvasEdge {
            id: incoming.id,
            from_node_id: incoming.from_node_id,
            to_node_id: incoming.to_node_id,
            kind: incoming.kind,
        })
        .collect();
    let graph = state
        .surreal
        .with_record_user_scope(
            authority.record_user_scope.clone(),
            state
                .storage
                .update_canvas_graph(&ctx, &canvas_id, nodes, edges),
        )
        .await
        .map_err(map_canvas_error)?;

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

/// `/canvases/:canvas_id` errors: the caller already proved it can read the canvas, so a canvas
/// that vanished under the authorized scope is answered like an unreadable one (constant denial,
/// never an existence-disclosing 404).
fn map_canvas_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(_) => protected_denial(),
        other => map_storage_error(other),
    }
}

fn map_storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(code) => not_found(code),
        StorageError::Conflict(code) | StorageError::ConflictDetails { code, .. } => {
            (StatusCode::CONFLICT, Json(ErrorResponse { error: code }))
        }
        // MT-154 silent-deny detection: a record-user write the permissions dropped.
        StorageError::Guard("HSK-403-PROTECTED-RESOURCE") => protected_denial(),
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

fn internal_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core", error = %err, "db_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: "db_error" }),
    )
}

fn not_found(code: &'static str) -> ApiError {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_conflict_maps_to_http_409_with_stable_code() {
        let (status, Json(body)) =
            map_storage_error(StorageError::Conflict("canvas_updated_at_conflict"));
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.error, "canvas_updated_at_conflict");
    }

    #[test]
    fn canvas_not_found_maps_to_http_404() {
        let (status, Json(body)) = map_storage_error(StorageError::NotFound("canvas"));
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.error, "canvas");
    }

    #[test]
    fn mt154_canvas_id_route_never_discloses_existence() {
        let (status, Json(body)) = map_canvas_error(StorageError::NotFound("canvas"));
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.error, "HSK-403-PROTECTED-RESOURCE");
        let (status, Json(body)) =
            map_storage_error(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.error, "HSK-403-PROTECTED-RESOURCE");
    }
}

#[cfg(test)]
mod mt149_conflict_tests {
    use super::*;

    #[test]
    fn mt149_conflict_details_preserve_http_409_and_code() {
        for error in [
            StorageError::Conflict("stable_conflict_code"),
            StorageError::ConflictDetails {
                code: "stable_conflict_code",
                detail: "private diagnostic context".to_owned(),
            },
        ] {
            let (status, Json(body)) = map_storage_error(error);
            assert_eq!(status, StatusCode::CONFLICT);
            assert_eq!(body.error, "stable_conflict_code");
        }
    }
}
