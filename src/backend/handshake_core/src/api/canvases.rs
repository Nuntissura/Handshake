use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
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
        .route("/canvases/:canvas_id", get(get_canvas))
        .with_state(state)
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
