//! WP-KERNEL-012 MT-045 (Locus resolve): read-only endpoints that turn a Locus
//! work-packet / microtask id into a minimal display record for the frontend
//! `LocusRecordWire` (`{title, summary?, status?}`, all serde-default so a
//! minimal `{"title":"…"}` still decodes).
//!
//! Data source: the canonical Locus tables `work_packets` / `micro_tasks`
//! (migration 0016), read directly over the shared storage handle. These rows
//! are keyed globally by `wp_id` / `mt_id` (not workspace-scoped), so the
//! `:workspace_id` segment is validated for path consistency but the lookup is
//! by id. Read-only, single-store authority, no new store.
//!
//! Locus records are resolved directly from the embedded SurrealDB authority.
//!
//! Reverse lookup (record -> referencing blocks) already works through
//! `loom/search-v2`; it is intentionally NOT duplicated here.

use crate::models::ErrorResponse;
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use surrealdb::types::SurrealValue;

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

fn not_found(code: &'static str) -> ApiError {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}

fn internal_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core::locus_api", error = %err, "locus_api_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-LOCUS",
        }),
    )
}

async fn ensure_workspace_exists(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(internal_error(err)),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/locus/work-packets/:record_id",
            get(resolve_work_packet),
        )
        .route(
            "/workspaces/:workspace_id/locus/microtasks/:record_id",
            get(resolve_micro_task),
        )
        .with_state(state)
}

/// The minimal Locus display record. Fields are optional beyond `title` so a
/// caller can decode a partial record; `summary`/`status` are omitted when
/// absent rather than serialized as null.
#[derive(Debug, Serialize)]
struct LocusRecordWire {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[derive(SurrealValue)]
struct LocusIdBindings {
    record_id: String,
}

#[derive(SurrealValue)]
struct WorkPacketLocusRow {
    title: String,
    description: Option<String>,
    status: String,
}

#[derive(SurrealValue)]
struct MicroTaskLocusRow {
    name: String,
    status: String,
}

async fn resolve_work_packet(
    State(state): State<AppState>,
    Path((workspace_id, record_id)): Path<(String, String)>,
) -> ApiResult<Json<LocusRecordWire>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let row: Option<WorkPacketLocusRow> = state
        .surreal
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT title, description, status FROM work_packets \
                         WHERE wp_id = $record_id LIMIT 1;",
                        LocusIdBindings { record_id },
                    )
                    .await
            })
        })
        .await
        .map_err(internal_error)?;
    match row {
        Some(row) => Ok(Json(LocusRecordWire {
            title: row.title,
            summary: row.description.filter(|value| !value.trim().is_empty()),
            status: Some(row.status),
        })),
        None => Err(not_found("locus_work_packet_not_found")),
    }
}

async fn resolve_micro_task(
    State(state): State<AppState>,
    Path((workspace_id, record_id)): Path<(String, String)>,
) -> ApiResult<Json<LocusRecordWire>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    // `micro_tasks` carries `name` + `status` (no dedicated description column;
    // richer detail lives in `metadata`). Title = name; summary is left None.
    let row: Option<MicroTaskLocusRow> = state
        .surreal
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT name, status FROM micro_tasks \
                         WHERE mt_id = $record_id LIMIT 1;",
                        LocusIdBindings { record_id },
                    )
                    .await
            })
        })
        .await
        .map_err(internal_error)?;
    match row {
        Some(row) => Ok(Json(LocusRecordWire {
            title: row.name,
            summary: None,
            status: Some(row.status),
        })),
        None => Err(not_found("locus_micro_task_not_found")),
    }
}
