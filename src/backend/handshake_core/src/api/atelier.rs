//! Atelier read/navigation HTTP surface (WP-KERNEL-005).
//!
//! Exposes the WP-KERNEL-005 atelier store over the existing Axum
//! server so a React panel can navigate it: a store overview, intake batches +
//! items, the command-corpus catalog, and the stealth-window registry. The
//! routes mirror the conventions in `api/workspaces.rs` exactly: a `routes`
//! builder, `State(AppState)` handlers, and a private `ErrorResponse` with
//! `internal_error` / `bad_request` helpers.
//!
//! Storage authority is the embedded SurrealDB store. Read handlers use the
//! typed `AtelierStore` surface or its lease-bound Surreal data context; no
//! relational fallback exists.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use crate::atelier::intake::{
    IntakeBatchMode, IntakeItemLoomProjection, IntakeLaneCounts, IntakeProfileMode, NewIntakeBatch,
    NewIntakeItem,
};
use crate::atelier::search::{
    AiTagSuggestion, AiTagSuggestionDecision, AiTagSuggestionStatus, NewAiTagSuggestion,
};
use crate::atelier::stealth_window::ResolvedContentRef;
use crate::atelier::{
    AtelierStore, BulkOperationReceipt, ClipboardImageImportRequest, DeletionArchiveRequest,
    DeletionImpactPreview, DeletionImpactPreviewRequest, DeletionRestoreRequest, DeletionTargetRef,
    ImageImportRecord, UrlImageImportRequest,
};
use crate::AppState;

const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/atelier/overview", get(overview))
        .route(
            "/atelier/intake/batches",
            get(list_intake_batches).post(create_intake_batch),
        )
        .route(
            "/atelier/intake/batches/:batch_id/items",
            get(list_intake_batch_items).post(create_intake_item),
        )
        .route(
            "/atelier/intake/items/:item_id/loom-projection",
            put(link_intake_item_loom_projection),
        )
        .route("/atelier/command-corpus", get(list_command_corpus))
        .route(
            "/atelier/filesystem-health/checks",
            post(run_filesystem_health_check),
        )
        .route(
            "/atelier/filesystem-health/checks/:check_id/findings",
            get(list_filesystem_health_findings),
        )
        .route(
            "/atelier/deletion/impact-preview",
            post(preview_deletion_impact),
        )
        .route("/atelier/deletion/archive", post(archive_deletion_targets))
        .route("/atelier/deletion/restore", post(restore_deletion_targets))
        .route(
            "/atelier/image-import/clipboard",
            post(import_clipboard_image),
        )
        .route("/atelier/image-import/url", post(record_url_image_import))
        .route(
            "/atelier/ai-tag-suggestions",
            post(record_ai_tag_suggestion),
        )
        .route(
            "/atelier/ai-tag-suggestions/characters/:character_internal_id",
            get(list_ai_tag_suggestions_for_character),
        )
        .route(
            "/atelier/ai-tag-suggestions/:suggestion_id/accept",
            post(accept_ai_tag_suggestion),
        )
        .route(
            "/atelier/ai-tag-suggestions/:suggestion_id/reject",
            post(reject_ai_tag_suggestion),
        )
        .route(
            "/atelier/ai-tag-suggestions/:suggestion_id/apply",
            post(apply_ai_tag_suggestion),
        )
        .route("/atelier/stealth/windows", get(list_stealth_windows))
        .route(
            "/atelier/stealth/windows/:window_ref_id/refs/:ref_id",
            get(resolve_stealth_ref),
        )
        .with_state(state)
}

/// Curated atelier tables surfaced by the overview row-count projection. This is
/// a fixed allowlist paired with literal SurrealQL count statements, so no
/// caller input reaches the query text.
const OVERVIEW_TABLES: &[&str] = &[
    "atelier_character",
    "atelier_media_asset",
    "atelier_media_source_provenance_ref",
    "atelier_media_sidecar",
    "atelier_bulk_operation_receipt",
    "atelier_trash_marker",
    "atelier_filesystem_health_check",
    "atelier_filesystem_health_finding",
    "atelier_image_import_request",
    "atelier_intake_batch",
    "atelier_intake_item",
    "atelier_pose_rig",
    "atelier_comfy_intake_output",
    "atelier_sourcing_spec",
    "atelier_transcript_artifact",
    "atelier_md_download_session",
    "atelier_command_corpus_entry",
    "atelier_ai_tag_suggestion",
    "atelier_stealth_window",
];

fn overview_count_statement(table: &str) -> &'static str {
    match table {
        "atelier_character" => "RETURN count(SELECT id FROM atelier_character);",
        "atelier_media_asset" => "RETURN count(SELECT id FROM atelier_media_asset);",
        "atelier_media_source_provenance_ref" => {
            "RETURN count(SELECT id FROM atelier_media_source_provenance_ref);"
        }
        "atelier_media_sidecar" => "RETURN count(SELECT id FROM atelier_media_sidecar);",
        "atelier_bulk_operation_receipt" => {
            "RETURN count(SELECT id FROM atelier_bulk_operation_receipt);"
        }
        "atelier_trash_marker" => "RETURN count(SELECT id FROM atelier_trash_marker);",
        "atelier_filesystem_health_check" => {
            "RETURN count(SELECT id FROM atelier_filesystem_health_check);"
        }
        "atelier_filesystem_health_finding" => {
            "RETURN count(SELECT id FROM atelier_filesystem_health_finding);"
        }
        "atelier_image_import_request" => {
            "RETURN count(SELECT id FROM atelier_image_import_request);"
        }
        "atelier_intake_batch" => "RETURN count(SELECT id FROM atelier_intake_batch);",
        "atelier_intake_item" => "RETURN count(SELECT id FROM atelier_intake_item);",
        "atelier_pose_rig" => "RETURN count(SELECT id FROM atelier_pose_rig);",
        "atelier_comfy_intake_output" => {
            "RETURN count(SELECT id FROM atelier_comfy_intake_output);"
        }
        "atelier_sourcing_spec" => "RETURN count(SELECT id FROM atelier_sourcing_spec);",
        "atelier_transcript_artifact" => {
            "RETURN count(SELECT id FROM atelier_transcript_artifact);"
        }
        "atelier_md_download_session" => {
            "RETURN count(SELECT id FROM atelier_md_download_session);"
        }
        "atelier_command_corpus_entry" => {
            "RETURN count(SELECT id FROM atelier_command_corpus_entry);"
        }
        "atelier_ai_tag_suggestion" => "RETURN count(SELECT id FROM atelier_ai_tag_suggestion);",
        "atelier_stealth_window" => "RETURN count(SELECT id FROM atelier_stealth_window);",
        _ => unreachable!("overview table must come from OVERVIEW_TABLES"),
    }
}

fn atelier_store(state: &AppState) -> AtelierStore {
    AtelierStore::with_observability(
        state.surreal.clone(),
        state.storage.clone(),
        state.flight_recorder.clone(),
    )
}

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

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn calling_actor(headers: &HeaderMap) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    header_str(headers, HSK_HEADER_ACTOR_ID)
        .map(ToOwned::to_owned)
        .ok_or((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "missing_actor",
            }),
        ))
}

/// Map an `AtelierError` to an HTTP status, mirroring `workspaces.rs`
/// `map_storage_error`: a missing aggregate is 404, a semantically-bad input is
/// 400, and infra/storage failures are 500. (Malformed `Path<Uuid>` / JSON body
/// inputs are already rejected with a 400 by Axum's extractors before a handler
/// runs.) The body never leaks internals — it is a fixed `&'static str` code.
fn atelier_error(err: crate::atelier::AtelierError) -> (StatusCode, Json<ErrorResponse>) {
    use crate::atelier::AtelierError;
    match err {
        AtelierError::NotFound(detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "not_found");
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse { error: "not_found" }),
            )
        }
        AtelierError::Validation(detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "bad_request");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "bad_request",
                }),
            )
        }
        AtelierError::Conflict(detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "conflict");
            (
                StatusCode::CONFLICT,
                Json(ErrorResponse { error: "conflict" }),
            )
        }
        other => internal_error(other),
    }
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

#[derive(SurrealValue)]
struct NoBindings {}

#[derive(SurrealValue)]
struct EventFamilyCountRow {
    event_family: String,
    count: i64,
}

/// GET /atelier/overview — row counts for the curated atelier tables plus
/// per-family atelier event counts.
async fn overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);

    let mut tables = Vec::with_capacity(OVERVIEW_TABLES.len());
    for name in OVERVIEW_TABLES {
        let statement = overview_count_statement(name);
        let rows = store
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first::<i64, _>(statement, NoBindings {}).await })
            })
            .await
            .map_err(atelier_error)?;
        let rows = rows.unwrap_or(0);
        tables.push(TableCount { name, rows });
    }

    let family_rows: Vec<EventFamilyCountRow> = store
        .with_data(|ctx| {
            Box::pin(async move {
                ctx.query_values(
                    "SELECT event_family, count() AS count FROM atelier_event \
                     GROUP BY event_family ORDER BY event_family;",
                    NoBindings {},
                )
                .await
            })
        })
        .await
        .map_err(atelier_error)?;

    let event_families = family_rows
        .into_iter()
        .map(|row| EventFamilyCount {
            family: row.event_family,
            count: row.count,
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
    source_ref: String,
    mode: String,
    profile_mode: String,
    target_character_id: Option<Uuid>,
    target_sheet_version_id: Option<Uuid>,
    target_collection_id: Option<Uuid>,
    status: String,
    resume_cursor: Option<String>,
    resumed_at_utc: Option<DateTime<Utc>>,
    created_at_utc: DateTime<Utc>,
}

fn intake_batch_response(batch: crate::atelier::intake::IntakeBatch) -> IntakeBatchResponse {
    IntakeBatchResponse {
        batch_id: batch.batch_id,
        idempotency_key: batch.idempotency_key,
        source_label: batch.source_label,
        source_ref: batch.source_ref,
        mode: batch.mode.as_str().to_string(),
        profile_mode: batch.profile_mode.as_str().to_string(),
        target_character_id: batch.target_character_id,
        target_sheet_version_id: batch.target_sheet_version_id,
        target_collection_id: batch.target_collection_id,
        status: batch.status.as_str().to_string(),
        resume_cursor: batch.resume_cursor,
        resumed_at_utc: batch.resumed_at_utc,
        created_at_utc: batch.created_at_utc,
    }
}

/// GET /atelier/intake/batches — newest first, capped.
async fn list_intake_batches(
    State(state): State<AppState>,
) -> Result<Json<Vec<IntakeBatchResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let batches = store
        .list_intake_batches(None, LIST_CAP)
        .await
        .map_err(atelier_error)?;

    let out = batches.into_iter().map(intake_batch_response).collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches", status = "ok", "list intake batches");

    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
struct CreateIntakeBatchRequest {
    idempotency_key: String,
    source_label: String,
    source_ref: Option<String>,
    mode: Option<String>,
    profile_mode: Option<String>,
    target_character_id: Option<Uuid>,
    target_sheet_version_id: Option<Uuid>,
    target_collection_id: Option<Uuid>,
    resume_cursor: Option<String>,
}

/// POST /atelier/intake/batches — open (idempotently) an intake batch.
async fn create_intake_batch(
    State(state): State<AppState>,
    Json(payload): Json<CreateIntakeBatchRequest>,
) -> Result<(StatusCode, Json<IntakeBatchResponse>), (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let mode = match payload.mode.as_deref().unwrap_or("manual") {
        "manual" => IntakeBatchMode::Manual,
        "folder_scan" => IntakeBatchMode::FolderScan,
        "sourcing_run" => IntakeBatchMode::SourcingRun,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_mode",
                }),
            ));
        }
    };
    let profile_mode = match payload.profile_mode.as_deref().unwrap_or("loose_profile") {
        "loose_profile" => IntakeProfileMode::LooseProfile,
        "character_linked" => IntakeProfileMode::CharacterLinked,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "invalid_profile_mode",
                }),
            ));
        }
    };
    let batch = store
        .open_intake_batch(&NewIntakeBatch {
            idempotency_key: payload.idempotency_key,
            source_label: payload.source_label,
            source_ref: payload.source_ref,
            mode,
            profile_mode,
            character_internal_id: payload.target_character_id,
            target_character_id: payload.target_character_id,
            target_sheet_version_id: payload.target_sheet_version_id,
            target_collection_id: payload.target_collection_id,
            resume_cursor: payload.resume_cursor,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches", status = "created", batch_id = %batch.batch_id, "open intake batch");

    Ok((StatusCode::CREATED, Json(intake_batch_response(batch))))
}

#[derive(Debug, Deserialize)]
struct RunFilesystemHealthCheckRequest {
    scope_label: Option<String>,
}

async fn run_filesystem_health_check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RunFilesystemHealthCheckRequest>,
) -> Result<
    (StatusCode, Json<crate::atelier::FilesystemHealthReport>),
    (StatusCode, Json<ErrorResponse>),
> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let report = store
        .run_filesystem_health_check(&crate::atelier::FilesystemHealthCheckRequest {
            requested_by: actor.clone(),
            scope_label: payload.scope_label,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/filesystem-health/checks",
        status = "created",
        check_id = %report.check.check_id,
        actor = %actor,
        "run filesystem health check"
    );

    Ok((StatusCode::CREATED, Json(report)))
}

async fn list_filesystem_health_findings(
    State(state): State<AppState>,
    Path(check_id): Path<Uuid>,
) -> Result<Json<Vec<crate::atelier::FilesystemHealthFinding>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let findings = store
        .list_filesystem_health_findings(check_id)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/filesystem-health/checks/:check_id/findings",
        status = "ok",
        check_id = %check_id,
        "list filesystem health findings"
    );

    Ok(Json(findings))
}

#[derive(Debug, Deserialize)]
struct DeletionControlsRequest {
    targets: Vec<DeletionTargetRef>,
    reason: String,
}

async fn preview_deletion_impact(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DeletionControlsRequest>,
) -> Result<Json<DeletionImpactPreview>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let preview = store
        .preview_deletion_impact(&DeletionImpactPreviewRequest {
            targets: payload.targets,
            requested_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/deletion/impact-preview",
        status = "ok",
        actor = %actor,
        target_count = preview.target_count,
        "preview deletion impact"
    );

    Ok(Json(preview))
}

async fn archive_deletion_targets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DeletionControlsRequest>,
) -> Result<(StatusCode, Json<BulkOperationReceipt>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let receipt = store
        .archive_deletion_targets(&DeletionArchiveRequest {
            targets: payload.targets,
            requested_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/deletion/archive",
        status = "created",
        actor = %actor,
        receipt_id = %receipt.receipt_id,
        "archive deletion targets"
    );

    Ok((StatusCode::CREATED, Json(receipt)))
}

async fn restore_deletion_targets(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DeletionControlsRequest>,
) -> Result<(StatusCode, Json<BulkOperationReceipt>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let receipt = store
        .restore_deletion_targets(&DeletionRestoreRequest {
            targets: payload.targets,
            requested_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/deletion/restore",
        status = "created",
        actor = %actor,
        receipt_id = %receipt.receipt_id,
        "restore deletion targets"
    );

    Ok((StatusCode::CREATED, Json(receipt)))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipboardImageImportApiRequest {
    idempotency_key: String,
    mime: String,
    content_hash: String,
    byte_len: i64,
    artifact_ref: String,
    source_application: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UrlImageImportApiRequest {
    idempotency_key: String,
    source_url: String,
    expected_mime: Option<String>,
    source_label: Option<String>,
    capability_profile_id: String,
    capability_grant_ref: String,
}

async fn import_clipboard_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ClipboardImageImportApiRequest>,
) -> Result<(StatusCode, Json<ImageImportRecord>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let record = store
        .import_clipboard_image(&ClipboardImageImportRequest {
            idempotency_key: payload.idempotency_key,
            mime: payload.mime,
            content_hash: payload.content_hash,
            byte_len: payload.byte_len,
            artifact_ref: payload.artifact_ref,
            source_application: payload.source_application,
            requested_by: actor.clone(),
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/image-import/clipboard",
        status = "created",
        actor = %actor,
        import_id = %record.import_id,
        "import clipboard image"
    );

    Ok((StatusCode::CREATED, Json(record)))
}

async fn record_url_image_import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UrlImageImportApiRequest>,
) -> Result<(StatusCode, Json<ImageImportRecord>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let record = store
        .record_url_image_import(&UrlImageImportRequest {
            idempotency_key: payload.idempotency_key,
            source_url: payload.source_url,
            expected_mime: payload.expected_mime,
            source_label: payload.source_label,
            capability_profile_id: payload.capability_profile_id,
            capability_grant_ref: payload.capability_grant_ref,
            requested_by: actor.clone(),
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/image-import/url",
        status = "created",
        actor = %actor,
        import_id = %record.import_id,
        "record URL image import"
    );

    Ok((StatusCode::CREATED, Json(record)))
}

#[derive(Debug, Serialize)]
struct IntakeLaneCountsResponse {
    pending: i64,
    accepted: i64,
    rejected: i64,
    deferred: i64,
    skipped: i64,
    failed: i64,
}

impl From<IntakeLaneCounts> for IntakeLaneCountsResponse {
    fn from(c: IntakeLaneCounts) -> Self {
        Self {
            pending: c.pending,
            accepted: c.accepted,
            rejected: c.rejected,
            deferred: c.deferred,
            skipped: c.skipped,
            failed: c.failed,
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
    loom_block_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct IntakeBatchItemsResponse {
    lane_counts: IntakeLaneCountsResponse,
    items: Vec<IntakeItemResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateIntakeItemRequest {
    source_path: String,
    file_name: String,
    byte_len: i64,
    content_hash: Option<String>,
}

/// POST /atelier/intake/batches/:batch_id/items — register one source item.
/// Replaying the same `(batch_id, source_path)` converges on the canonical item.
async fn create_intake_item(
    State(state): State<AppState>,
    Path(batch_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<CreateIntakeItemRequest>,
) -> Result<(StatusCode, Json<IntakeItemResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let item = atelier_store(&state)
        .add_intake_item(
            batch_id,
            &NewIntakeItem {
                source_path: payload.source_path,
                file_name: payload.file_name,
                byte_len: payload.byte_len,
                content_hash: payload.content_hash,
            },
        )
        .await
        .map_err(atelier_error)?;
    let response = IntakeItemResponse {
        item_id: item.item_id,
        source_path: item.source_path,
        file_name: item.file_name,
        lane: item.lane.as_str().to_owned(),
        byte_len: item.byte_len,
        loom_block_id: None,
    };

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/batches/:batch_id/items",
        status = "created",
        actor = %actor,
        batch_id = %batch_id,
        item_id = %response.item_id,
        "register intake batch item"
    );

    Ok((StatusCode::CREATED, Json(response)))
}

#[derive(SurrealValue)]
struct IntakeProjectionBinding {
    batch_ref: RecordId,
    limit: i64,
}

#[derive(SurrealValue)]
struct IntakeProjectionRow {
    item_id: SurrealUuid,
    loom_block_id: String,
}

/// GET /atelier/intake/batches/:batch_id/items — lane counts + items for a batch.
async fn list_intake_batch_items(
    State(state): State<AppState>,
    Path(batch_id): Path<Uuid>,
) -> Result<Json<IntakeBatchItemsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);

    let lane_counts = store
        .intake_lane_counts(batch_id)
        .await
        .map_err(atelier_error)?;

    let projections: Vec<IntakeProjectionRow> = store
        .with_data(move |ctx| {
            Box::pin(async move {
                ctx.query_values(
                    "SELECT record::id(item_id) AS item_id, \
                     record::id(loom_block_id) AS loom_block_id \
                     FROM atelier_intake_item_loom_projection \
                     WHERE item_id IN (SELECT VALUE id FROM atelier_intake_item \
                     WHERE batch_id = $batch_ref \
                     ORDER BY created_at_utc ASC LIMIT $limit);",
                    IntakeProjectionBinding {
                        batch_ref: RecordId::new(
                            "atelier_intake_batch",
                            SurrealUuid::from(batch_id),
                        ),
                        limit: LIST_CAP,
                    },
                )
                .await
            })
        })
        .await
        .map_err(atelier_error)?;
    let projections = projections
        .into_iter()
        .map(|row| (Uuid::from(row.item_id), row.loom_block_id))
        .collect::<std::collections::HashMap<_, _>>();
    let items = store
        .list_intake_items_limited(batch_id, None, LIST_CAP)
        .await
        .map_err(atelier_error)?
        .into_iter()
        .map(|item| IntakeItemResponse {
            loom_block_id: projections.get(&item.item_id).cloned(),
            item_id: item.item_id,
            source_path: item.source_path,
            file_name: item.file_name,
            lane: item.lane.as_str().to_owned(),
            byte_len: item.byte_len,
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches/:batch_id/items", status = "ok", batch_id = %batch_id, "list intake batch items");

    Ok(Json(IntakeBatchItemsResponse {
        lane_counts: lane_counts.into(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkIntakeItemLoomProjectionRequest {
    loom_block_id: String,
}

/// PUT /atelier/intake/items/:item_id/loom-projection — publish the durable
/// canonical Loom identity consumed by editor/canvas drag payloads.
async fn link_intake_item_loom_projection(
    State(state): State<AppState>,
    Path(item_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<LinkIntakeItemLoomProjectionRequest>,
) -> Result<Json<IntakeItemLoomProjection>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let projection = atelier_store(&state)
        .link_intake_item_loom_projection(item_id, &payload.loom_block_id, &actor)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/items/:item_id/loom-projection",
        status = "ok",
        item_id = %item_id,
        loom_block_id = %projection.loom_block_id,
        "linked intake item Loom projection"
    );
    Ok(Json(projection))
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
    let out = atelier_store(&state)
        .list_command_corpus_entries_limited(None, LIST_CAP)
        .await
        .map_err(atelier_error)?
        .into_iter()
        .map(|entry| CommandCorpusEntryResponse {
            entry_id: entry.entry_id,
            action_id: entry.action_id,
            owner: entry.owner,
            execution_class: entry.execution_class.as_token().to_owned(),
            foreground_flag: entry.foreground_flag,
            manual_anchor: entry.manual_anchor,
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/command-corpus", status = "ok", "list command corpus");

    Ok(Json(out))
}

#[derive(Debug, Serialize)]
struct AiTagSuggestionResponse {
    suggestion_id: Uuid,
    character_internal_id: Uuid,
    asset_id: Option<Uuid>,
    tag_text: String,
    confidence: Option<f64>,
    model_receipt_ref: String,
    tool_receipt_ref: String,
    suggested_by: String,
    status: String,
    decided_by: Option<String>,
    decision_reason: Option<String>,
    applied_tag_id: Option<Uuid>,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct RecordAiTagSuggestionRequest {
    character_internal_id: Uuid,
    asset_id: Option<Uuid>,
    tag_text: String,
    confidence: Option<f64>,
    model_receipt_ref: String,
    tool_receipt_ref: String,
    suggested_by: String,
}

#[derive(Debug, Deserialize)]
struct AiTagSuggestionDecisionRequest {
    reason: Option<String>,
}

fn ai_tag_suggestion_status_token(status: AiTagSuggestionStatus) -> &'static str {
    match status {
        AiTagSuggestionStatus::Proposed => "proposed",
        AiTagSuggestionStatus::Accepted => "accepted",
        AiTagSuggestionStatus::Rejected => "rejected",
        AiTagSuggestionStatus::Applied => "applied",
    }
}

fn ai_tag_suggestion_response(suggestion: AiTagSuggestion) -> AiTagSuggestionResponse {
    AiTagSuggestionResponse {
        suggestion_id: suggestion.suggestion_id,
        character_internal_id: suggestion.character_internal_id,
        asset_id: suggestion.asset_id,
        tag_text: suggestion.tag_text,
        confidence: suggestion.confidence,
        model_receipt_ref: suggestion.model_receipt_ref,
        tool_receipt_ref: suggestion.tool_receipt_ref,
        suggested_by: suggestion.suggested_by,
        status: ai_tag_suggestion_status_token(suggestion.status).to_string(),
        decided_by: suggestion.decided_by,
        decision_reason: suggestion.decision_reason,
        applied_tag_id: suggestion.applied_tag_id,
        created_at_utc: suggestion.created_at_utc,
        updated_at_utc: suggestion.updated_at_utc,
    }
}

async fn record_ai_tag_suggestion(
    State(state): State<AppState>,
    Json(payload): Json<RecordAiTagSuggestionRequest>,
) -> Result<(StatusCode, Json<AiTagSuggestionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let suggestion = store
        .record_ai_tag_suggestion(&NewAiTagSuggestion {
            character_internal_id: payload.character_internal_id,
            asset_id: payload.asset_id,
            tag_text: payload.tag_text,
            confidence: payload.confidence,
            model_receipt_ref: payload.model_receipt_ref,
            tool_receipt_ref: payload.tool_receipt_ref,
            suggested_by: payload.suggested_by,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions",
        status = "created",
        suggestion_id = %suggestion.suggestion_id,
        "record AI tag suggestion"
    );

    Ok((
        StatusCode::CREATED,
        Json(ai_tag_suggestion_response(suggestion)),
    ))
}

async fn list_ai_tag_suggestions_for_character(
    State(state): State<AppState>,
    Path(character_internal_id): Path<Uuid>,
) -> Result<Json<Vec<AiTagSuggestionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let suggestions = store
        .list_ai_tag_suggestions_for_character(character_internal_id)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/characters/:character_internal_id",
        status = "ok",
        character_internal_id = %character_internal_id,
        "list AI tag suggestions"
    );

    Ok(Json(
        suggestions
            .into_iter()
            .map(ai_tag_suggestion_response)
            .collect(),
    ))
}

async fn accept_ai_tag_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(suggestion_id): Path<Uuid>,
    Json(payload): Json<AiTagSuggestionDecisionRequest>,
) -> Result<Json<AiTagSuggestionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let suggestion = store
        .accept_ai_tag_suggestion(&AiTagSuggestionDecision {
            suggestion_id,
            decided_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/:suggestion_id/accept",
        status = "ok",
        suggestion_id = %suggestion_id,
        actor = %actor,
        "accept AI tag suggestion"
    );

    Ok(Json(ai_tag_suggestion_response(suggestion)))
}

async fn reject_ai_tag_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(suggestion_id): Path<Uuid>,
    Json(payload): Json<AiTagSuggestionDecisionRequest>,
) -> Result<Json<AiTagSuggestionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let suggestion = store
        .reject_ai_tag_suggestion(&AiTagSuggestionDecision {
            suggestion_id,
            decided_by: actor.clone(),
            reason: payload.reason,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/:suggestion_id/reject",
        status = "ok",
        suggestion_id = %suggestion_id,
        actor = %actor,
        "reject AI tag suggestion"
    );

    Ok(Json(ai_tag_suggestion_response(suggestion)))
}

async fn apply_ai_tag_suggestion(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(suggestion_id): Path<Uuid>,
) -> Result<Json<AiTagSuggestionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let suggestion = store
        .apply_ai_tag_suggestion(suggestion_id, &actor)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ai-tag-suggestions/:suggestion_id/apply",
        status = "ok",
        suggestion_id = %suggestion_id,
        actor = %actor,
        "apply AI tag suggestion"
    );

    Ok(Json(ai_tag_suggestion_response(suggestion)))
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

/// GET /atelier/stealth/windows — registry entries visible to the calling actor,
/// newest first, capped.
async fn list_stealth_windows(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<StealthWindowResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let windows = store
        .list_stealth_windows(&actor, None, LIST_CAP)
        .await
        .map_err(atelier_error)?;

    let out = windows
        .into_iter()
        .map(|window| StealthWindowResponse {
            window_ref_id: window.window_ref_id,
            owner_actor: window.owner_actor,
            title: window.title,
            visibility: window.visibility.as_token().to_string(),
            status: window.status.as_token().to_string(),
            revision: window.revision,
        })
        .collect();

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/stealth/windows",
        status = "ok",
        actor = %actor,
        "list stealth windows"
    );

    Ok(Json(out))
}

/// GET /atelier/stealth/windows/:window_ref_id/refs/:ref_id — governed,
/// redacted single-reference view. This is a read-only projection over
/// durable authority and never includes raw payload fields.
async fn resolve_stealth_ref(
    State(state): State<AppState>,
    Path((window_ref_id, ref_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ResolvedContentRef>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let resolved = store
        .resolve_stealth_ref(window_ref_id, ref_id)
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/stealth/windows/:window_ref_id/refs/:ref_id",
        status = "ok",
        window_ref_id = %window_ref_id,
        ref_id = %ref_id,
        "resolve stealth ref"
    );

    Ok(Json(resolved))
}
