//! Atelier read/navigation HTTP surface (WP-KERNEL-005).
//!
//! Exposes the WP-KERNEL-005 atelier PostgreSQL store over the existing Axum
//! server so a React panel can navigate it: a store overview, intake batches +
//! items, the command-corpus catalog, and the stealth-window registry. The
//! routes mirror the conventions in `api/workspaces.rs` exactly: a `routes`
//! builder, `State(AppState)` handlers, and a private `ErrorResponse` with
//! `internal_error` / `bad_request` helpers.
//!
//! Storage authority is PostgreSQL only (`AtelierStore` over the shared
//! `AppState::postgres_pool`); SQLite is never used. `ensure_schema` is called
//! once at startup, never per-request. Read handlers build an `AtelierStore`
//! per request from the shared pool, or run a direct `sqlx` query against the
//! pool where no typed read method fits the contract. Table names used in count
//! queries come from a fixed allowlist constant; no caller input is ever
//! interpolated into SQL.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::atelier::intake::{IntakeLaneCounts, NewIntakeBatch};
use crate::atelier::AtelierStore;
use crate::AppState;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/atelier/overview", get(overview))
        .route(
            "/atelier/intake/batches",
            get(list_intake_batches).post(create_intake_batch),
        )
        .route(
            "/atelier/intake/batches/:batch_id/items",
            get(list_intake_batch_items),
        )
        .route("/atelier/command-corpus", get(list_command_corpus))
        .route("/atelier/stealth/windows", get(list_stealth_windows))
        .with_state(state)
}

/// Curated atelier tables surfaced by the overview row-count projection. This is
/// a fixed allowlist: only these literal identifiers are ever placed into a
/// `SELECT count(*)` statement, so no caller input reaches SQL.
const OVERVIEW_TABLES: &[&str] = &[
    "atelier_character",
    "atelier_media_asset",
    "atelier_intake_batch",
    "atelier_intake_item",
    "atelier_pose_rig",
    "atelier_comfy_intake_output",
    "atelier_sourcing_spec",
    "atelier_transcript_artifact",
    "atelier_md_download_session",
    "atelier_command_corpus_entry",
    "atelier_stealth_window",
];

/// Cap on list endpoints so a React panel never pulls an unbounded result set.
const LIST_CAP: i64 = 200;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn internal_error(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(target: "handshake_core::atelier", error = %err, "db_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: "db_error" }),
    )
}

// Malformed `Path<Uuid>` / JSON body inputs are rejected with a 400 by Axum's
// own extractors before a handler runs, so this helper is currently uninvoked;
// it is retained as the canonical 400 shape for handlers that need to reject a
// semantically-bad-but-well-typed input.
#[allow(dead_code)]
fn bad_request(err: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(target: "handshake_core::atelier", error = %err, "bad_request");
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "bad_request",
        }),
    )
}

#[derive(Debug, Serialize)]
struct TableCount {
    name: &'static str,
    rows: i64,
}

#[derive(Debug, Serialize)]
struct EventFamilyCount {
    family: String,
    count: i64,
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    tables: Vec<TableCount>,
    event_families: Vec<EventFamilyCount>,
}

/// GET /atelier/overview — row counts for the curated atelier tables plus
/// per-family atelier event counts.
async fn overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    let pool = &state.postgres_pool;

    let mut tables = Vec::with_capacity(OVERVIEW_TABLES.len());
    for name in OVERVIEW_TABLES {
        // `name` is a fixed allowlist literal, never caller input.
        let rows: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {name}"))
            .fetch_one(pool)
            .await
            .map_err(internal_error)?;
        tables.push(TableCount { name, rows });
    }

    let family_rows = sqlx::query(
        r#"SELECT event_family, count(*) AS n
           FROM atelier_event
           GROUP BY event_family
           ORDER BY event_family"#,
    )
    .fetch_all(pool)
    .await
    .map_err(internal_error)?;

    let event_families = family_rows
        .iter()
        .map(|row| EventFamilyCount {
            family: row.get("event_family"),
            count: row.get("n"),
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/overview", status = "ok", "atelier overview");

    Ok(Json(OverviewResponse {
        tables,
        event_families,
    }))
}

#[derive(Debug, Serialize)]
struct IntakeBatchResponse {
    batch_id: Uuid,
    idempotency_key: String,
    source_label: String,
    status: String,
    created_at_utc: DateTime<Utc>,
}

/// GET /atelier/intake/batches — newest first, capped.
async fn list_intake_batches(
    State(state): State<AppState>,
) -> Result<Json<Vec<IntakeBatchResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = AtelierStore::new(state.postgres_pool.clone());
    let batches = store
        .list_intake_batches(None, LIST_CAP)
        .await
        .map_err(internal_error)?;

    let out = batches
        .into_iter()
        .map(|b| IntakeBatchResponse {
            batch_id: b.batch_id,
            idempotency_key: b.idempotency_key,
            source_label: b.source_label,
            status: b.status.as_str().to_string(),
            created_at_utc: b.created_at_utc,
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches", status = "ok", "list intake batches");

    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
struct CreateIntakeBatchRequest {
    idempotency_key: String,
    source_label: String,
}

/// POST /atelier/intake/batches — open (idempotently) an intake batch.
async fn create_intake_batch(
    State(state): State<AppState>,
    Json(payload): Json<CreateIntakeBatchRequest>,
) -> Result<(StatusCode, Json<IntakeBatchResponse>), (StatusCode, Json<ErrorResponse>)> {
    let store = AtelierStore::new(state.postgres_pool.clone());
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: payload.idempotency_key,
            source_label: payload.source_label,
            character_internal_id: None,
        })
        .await
        .map_err(internal_error)?;

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches", status = "created", batch_id = %batch.batch_id, "open intake batch");

    Ok((
        StatusCode::CREATED,
        Json(IntakeBatchResponse {
            batch_id: batch.batch_id,
            idempotency_key: batch.idempotency_key,
            source_label: batch.source_label,
            status: batch.status.as_str().to_string(),
            created_at_utc: batch.created_at_utc,
        }),
    ))
}

#[derive(Debug, Serialize)]
struct IntakeLaneCountsResponse {
    new: i64,
    accepted: i64,
    rejected: i64,
    deferred: i64,
}

impl From<IntakeLaneCounts> for IntakeLaneCountsResponse {
    fn from(c: IntakeLaneCounts) -> Self {
        Self {
            new: c.new,
            accepted: c.accepted,
            rejected: c.rejected,
            deferred: c.deferred,
        }
    }
}

#[derive(Debug, Serialize)]
struct IntakeItemResponse {
    item_id: Uuid,
    source_path: String,
    file_name: String,
    lane: String,
    byte_len: i64,
}

#[derive(Debug, Serialize)]
struct IntakeBatchItemsResponse {
    lane_counts: IntakeLaneCountsResponse,
    items: Vec<IntakeItemResponse>,
}

/// GET /atelier/intake/batches/:batch_id/items — lane counts + items for a batch.
async fn list_intake_batch_items(
    State(state): State<AppState>,
    Path(batch_id): Path<Uuid>,
) -> Result<Json<IntakeBatchItemsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = AtelierStore::new(state.postgres_pool.clone());

    let lane_counts = store
        .intake_lane_counts(batch_id)
        .await
        .map_err(internal_error)?;

    let items = store
        .list_intake_items(batch_id, None)
        .await
        .map_err(internal_error)?;

    let items = items
        .into_iter()
        .map(|i| IntakeItemResponse {
            item_id: i.item_id,
            source_path: i.source_path,
            file_name: i.file_name,
            lane: i.lane.as_str().to_string(),
            byte_len: i.byte_len,
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches/:batch_id/items", status = "ok", batch_id = %batch_id, "list intake batch items");

    Ok(Json(IntakeBatchItemsResponse {
        lane_counts: lane_counts.into(),
        items,
    }))
}

#[derive(Debug, Serialize)]
struct CommandCorpusEntryResponse {
    entry_id: Uuid,
    action_id: String,
    owner: String,
    execution_class: String,
    foreground_flag: bool,
    manual_anchor: String,
}

/// GET /atelier/command-corpus — catalog descriptors ordered by action_id, capped.
async fn list_command_corpus(
    State(state): State<AppState>,
) -> Result<Json<Vec<CommandCorpusEntryResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // No typed list method caps the result set, so a direct, capped query is
    // used. Column projection only; no caller input reaches SQL.
    let rows = sqlx::query(
        r#"SELECT entry_id, action_id, owner, execution_class, foreground_flag, manual_anchor
           FROM atelier_command_corpus_entry
           ORDER BY action_id ASC
           LIMIT $1"#,
    )
    .bind(LIST_CAP)
    .fetch_all(&state.postgres_pool)
    .await
    .map_err(internal_error)?;

    let out = rows
        .iter()
        .map(|row| CommandCorpusEntryResponse {
            entry_id: row.get("entry_id"),
            action_id: row.get("action_id"),
            owner: row.get("owner"),
            execution_class: row.get("execution_class"),
            foreground_flag: row.get("foreground_flag"),
            manual_anchor: row.get("manual_anchor"),
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/command-corpus", status = "ok", "list command corpus");

    Ok(Json(out))
}

#[derive(Debug, Serialize)]
struct StealthWindowResponse {
    window_ref_id: Uuid,
    owner_actor: String,
    title: String,
    visibility: String,
    status: String,
    revision: i64,
}

/// GET /atelier/stealth/windows — registry entries across all owners, newest
/// first, capped.
async fn list_stealth_windows(
    State(state): State<AppState>,
) -> Result<Json<Vec<StealthWindowResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // The typed `list_stealth_windows` filters to a single owner_actor; the
    // panel needs all owners, so a direct, capped query is used. Column
    // projection only; no caller input reaches SQL.
    let rows = sqlx::query(
        r#"SELECT window_ref_id, owner_actor, title, visibility, status, revision
           FROM atelier_stealth_window
           ORDER BY updated_at_utc DESC
           LIMIT $1"#,
    )
    .bind(LIST_CAP)
    .fetch_all(&state.postgres_pool)
    .await
    .map_err(internal_error)?;

    let out = rows
        .iter()
        .map(|row| StealthWindowResponse {
            window_ref_id: row.get("window_ref_id"),
            owner_actor: row.get("owner_actor"),
            title: row.get("title"),
            visibility: row.get("visibility"),
            status: row.get("status"),
            revision: row.get("revision"),
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/stealth/windows", status = "ok", "list stealth windows");

    Ok(Json(out))
}
