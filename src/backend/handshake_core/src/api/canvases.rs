use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{
    models::{
        CanvasEdgeResponse, CanvasNodeResponse, CanvasResponse, CanvasWithGraphResponse,
        CreateCanvasRequest, ErrorResponse,
    },
    AppState,
};

#[derive(sqlx::FromRow)]
struct CanvasRow {
    id: String,
    workspace_id: String,
    title: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct CanvasNodeRow {
    id: String,
    canvas_id: String,
    kind: String,
    position_x: f64,
    position_y: f64,
    data: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(sqlx::FromRow)]
struct CanvasEdgeRow {
    id: String,
    canvas_id: String,
    from_node_id: String,
    to_node_id: String,
    kind: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

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
    let existing = sqlx::query_scalar!(
        r#"SELECT COUNT(1) as "count!: i64" FROM canvases WHERE id = ?1"#,
        canvas_id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(super::workspaces::internal_error)?;

    if existing == 0 {
        return Err(super::workspaces::not_found("canvas_not_found"));
    }

    sqlx::query!(r#"DELETE FROM canvases WHERE id = ?1"#, canvas_id)
        .execute(&state.pool)
        .await
        .map_err(super::workspaces::internal_error)?;

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "deleted", canvas_id = %canvas_id, "canvas deleted");

    Ok(StatusCode::NO_CONTENT)
}

async fn create_canvas(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateCanvasRequest>,
) -> Result<(StatusCode, Json<CanvasResponse>), (StatusCode, Json<ErrorResponse>)> {
    super::workspaces::ensure_workspace(&state.pool, &workspace_id).await?;

    let now = Utc::now();
    let id = Uuid::new_v4().to_string();

    sqlx::query!(
        r#"
        INSERT INTO canvases (id, workspace_id, title, created_at, updated_at)
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
    .map_err(super::workspaces::internal_error)?;

    tracing::info!(target: "handshake_core", route = "/workspaces/:workspace_id/canvases", status = "created", workspace_id = %workspace_id, canvas_id = %id, "canvas created");

    Ok((
        StatusCode::CREATED,
        Json(CanvasResponse {
            id,
            workspace_id,
            title: payload.title,
            created_at: now,
            updated_at: now,
        }),
    ))
}

async fn list_canvases(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<CanvasResponse>>, (StatusCode, Json<ErrorResponse>)> {
    super::workspaces::ensure_workspace(&state.pool, &workspace_id).await?;

    let rows = sqlx::query_as!(
        CanvasRow,
        r#"
        SELECT
            id as "id!: String",
            workspace_id as "workspace_id!: String",
            title as "title!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM canvases
        WHERE workspace_id = ?1
        ORDER BY created_at ASC
        "#,
        workspace_id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(super::workspaces::internal_error)?;

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
    let canvas = sqlx::query_as!(
        CanvasRow,
        r#"
        SELECT
            id as "id!: String",
            workspace_id as "workspace_id!: String",
            title as "title!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM canvases
        WHERE id = ?1
        "#,
        canvas_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(super::workspaces::internal_error)?;

    let canvas = match canvas {
        Some(row) => row,
        None => return Err(super::workspaces::not_found("canvas_not_found")),
    };

    let nodes = sqlx::query_as!(
        CanvasNodeRow,
        r#"
        SELECT
            id as "id!: String",
            canvas_id as "canvas_id!: String",
            kind as "kind!: String",
            position_x as "position_x!: f64",
            position_y as "position_y!: f64",
            data as "data!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM canvas_nodes
        WHERE canvas_id = ?1
        ORDER BY created_at ASC
        "#,
        canvas.id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(super::workspaces::internal_error)?;

    let edges = sqlx::query_as!(
        CanvasEdgeRow,
        r#"
        SELECT
            id as "id!: String",
            canvas_id as "canvas_id!: String",
            from_node_id as "from_node_id!: String",
            to_node_id as "to_node_id!: String",
            kind as "kind!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM canvas_edges
        WHERE canvas_id = ?1
        ORDER BY created_at ASC
        "#,
        canvas.id
    )
    .fetch_all(&state.pool)
    .await
    .map_err(super::workspaces::internal_error)?;

    let nodes = nodes
        .into_iter()
        .map(|n| CanvasNodeResponse {
            id: n.id,
            canvas_id: n.canvas_id,
            kind: n.kind,
            position_x: n.position_x,
            position_y: n.position_y,
            data: serde_json::from_str(&n.data).unwrap_or(Value::Object(Default::default())),
            created_at: n.created_at,
            updated_at: n.updated_at,
        })
        .collect();

    let edges = edges
        .into_iter()
        .map(|e| CanvasEdgeResponse {
            id: e.id,
            canvas_id: e.canvas_id,
            from_node_id: e.from_node_id,
            to_node_id: e.to_node_id,
            kind: e.kind,
            created_at: e.created_at,
            updated_at: e.updated_at,
        })
        .collect();

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "ok", canvas_id = %canvas.id, "get canvas");

    Ok(Json(CanvasWithGraphResponse {
        id: canvas.id,
        workspace_id: canvas.workspace_id,
        title: canvas.title,
        created_at: canvas.created_at,
        updated_at: canvas.updated_at,
        nodes,
        edges,
    }))
}

async fn update_canvas_graph(
    State(state): State<AppState>,
    Path(canvas_id): Path<String>,
    Json(payload): Json<UpdateCanvasGraphRequest>,
) -> Result<Json<CanvasWithGraphResponse>, (StatusCode, Json<ErrorResponse>)> {
    let canvas = sqlx::query_as!(
        CanvasRow,
        r#"
        SELECT
            id as "id!: String",
            workspace_id as "workspace_id!: String",
            title as "title!: String",
            created_at as "created_at: chrono::DateTime<chrono::Utc>",
            updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
        FROM canvases
        WHERE id = ?1
        "#,
        canvas_id
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(super::workspaces::internal_error)?;

    let canvas = match canvas {
        Some(row) => row,
        None => return Err(super::workspaces::not_found("canvas_not_found")),
    };

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(super::workspaces::internal_error)?;

    // Edges depend on nodes; clear edges first, then nodes.
    sqlx::query!(
        r#"DELETE FROM canvas_edges WHERE canvas_id = ?1"#,
        canvas.id
    )
    .execute(&mut *tx)
    .await
    .map_err(super::workspaces::internal_error)?;

    sqlx::query!(
        r#"DELETE FROM canvas_nodes WHERE canvas_id = ?1"#,
        canvas.id
    )
    .execute(&mut *tx)
    .await
    .map_err(super::workspaces::internal_error)?;

    let now = Utc::now();
    let mut inserted_nodes = Vec::with_capacity(payload.nodes.len());

    for incoming in payload.nodes.into_iter() {
        let node_id = incoming.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let data = incoming
            .data
            .unwrap_or_else(|| Value::Object(Default::default()));
        let data_str = data.to_string();

        sqlx::query!(
            r#"
            INSERT INTO canvas_nodes (
                id, canvas_id, kind, position_x, position_y, data, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            node_id,
            canvas.id,
            incoming.kind,
            incoming.position_x,
            incoming.position_y,
            data_str,
            now,
            now
        )
        .execute(&mut *tx)
        .await
        .map_err(super::workspaces::internal_error)?;

        inserted_nodes.push(CanvasNodeResponse {
            id: node_id,
            canvas_id: canvas.id.clone(),
            kind: incoming.kind,
            position_x: incoming.position_x,
            position_y: incoming.position_y,
            data,
            created_at: now,
            updated_at: now,
        });
    }

    let mut inserted_edges = Vec::with_capacity(payload.edges.len());

    for incoming in payload.edges.into_iter() {
        let edge_id = incoming.id.unwrap_or_else(|| Uuid::new_v4().to_string());

        sqlx::query!(
            r#"
            INSERT INTO canvas_edges (
                id, canvas_id, from_node_id, to_node_id, kind, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            edge_id,
            canvas.id,
            incoming.from_node_id,
            incoming.to_node_id,
            incoming.kind,
            now,
            now
        )
        .execute(&mut *tx)
        .await
        .map_err(super::workspaces::internal_error)?;

        inserted_edges.push(CanvasEdgeResponse {
            id: edge_id,
            canvas_id: canvas.id.clone(),
            from_node_id: incoming.from_node_id,
            to_node_id: incoming.to_node_id,
            kind: incoming.kind,
            created_at: now,
            updated_at: now,
        });
    }

    tx.commit()
        .await
        .map_err(super::workspaces::internal_error)?;

    tracing::info!(target: "handshake_core", route = "/canvases/:canvas_id", status = "ok", canvas_id = %canvas.id, nodes = inserted_nodes.len(), edges = inserted_edges.len(), "update canvas graph");

    Ok(Json(CanvasWithGraphResponse {
        id: canvas.id,
        workspace_id: canvas.workspace_id,
        title: canvas.title,
        created_at: canvas.created_at,
        updated_at: now,
        nodes: inserted_nodes,
        edges: inserted_edges,
    }))
}
