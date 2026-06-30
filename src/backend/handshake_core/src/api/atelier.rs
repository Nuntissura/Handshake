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
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashSet;
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::atelier::collections::{Collection, NewCollection};
use crate::atelier::documents::{
    AppendCharacterDocumentVersion, CharacterDocument, CharacterDocumentType,
    CharacterDocumentVersion, NewCharacterDocument, NewStoryBeat, NewStoryCard, StoryBeat,
    StoryCard,
};
use crate::atelier::intake::{
    ApplyIntakeClassificationRequest, IntakeBatchMode, IntakeClassificationMetadata, IntakeLane,
    IntakeLaneCounts, IntakeProfileMode, NewIntakeBatch,
};
use crate::atelier::moodboards::{MoodboardSnapshot, NewMoodboardSnapshot};
use crate::atelier::pose::{
    generate_posekit_openpose_export, generate_posekit_openpose_export_from_keypoints,
    NewPoseSidecar, PoseSidecar, PoseSidecarKind, PoseSidecarStatus, PosekitExportFraming,
    PosekitMarkerLayers, PosekitOpenPoseExport, PosekitOpenPoseExportRequest,
    POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID,
};
use crate::atelier::search::{
    normalize_tag, AiTagSuggestion, AiTagSuggestionDecision, AiTagSuggestionStatus, CkcSearchMode,
    CkcSearchRequest, CkcSearchResponse, CkcTagNote, NewAiTagSuggestion, UpsertCkcTagNote,
};
use crate::atelier::sheet::{
    sheet_field_id_from_line, sheet_field_values, sheet_line_looks_like_field,
};
use crate::atelier::stealth_window::ResolvedContentRef;
use crate::atelier::{
    builtin_character_sheet_template, builtin_safe_subset, character_ref, collection_ref,
    default_character_sheet_text, event_ref_for_text, media_asset_ref, reject_legacy_runtime_ref,
    sheet_version_ref, text_hash, AtelierError, AtelierStore, BulkOperationReceipt, Character,
    ClipboardImageImportRequest, DeletionArchiveRequest, DeletionImpactPreview,
    DeletionImpactPreviewRequest, DeletionRestoreRequest, DeletionTargetRef, ImageImportRecord,
    NewCharacter, NewSheetVersion, SheetVersion, UrlImageImportRequest,
    CHARACTER_SHEET_V2_TEMPLATE_VERSION, DEFAULT_SHEET_TOOL,
};
use crate::storage::artifacts::{
    artifact_root_rel, resolve_workspace_root, validate_artifact_content_hash, write_file_artifact,
    ArtifactClassification, ArtifactLayer, ArtifactManifest, ArtifactPayloadKind,
};
use crate::storage::EntityRef;
use crate::AppState;

const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/atelier/overview", get(overview))
        .route(
            "/atelier/characters",
            get(list_characters).post(create_character),
        )
        .route(
            "/atelier/characters/:character_internal_id",
            get(get_character),
        )
        .route(
            "/atelier/characters/:character_internal_id/sheet-versions",
            get(list_sheet_versions).post(append_sheet_version),
        )
        .route(
            "/atelier/characters/:character_internal_id/media-albums",
            get(list_character_media_albums).post(create_character_media_album),
        )
        .route(
            "/atelier/characters/:character_internal_id/documents",
            get(list_character_documents).post(create_character_document),
        )
        .route(
            "/atelier/characters/:character_internal_id/sheet-versions/import",
            post(import_sheet_version),
        )
        .route(
            "/atelier/character-documents/:document_id",
            get(get_character_document),
        )
        .route(
            "/atelier/character-documents/:document_id/versions",
            get(list_character_document_versions).post(append_character_document_version),
        )
        .route(
            "/atelier/character-documents/:document_id/story-cards",
            get(list_story_cards).post(add_story_card),
        )
        .route(
            "/atelier/character-documents/:document_id/story-beats",
            get(list_story_beats).post(add_story_beat),
        )
        .route(
            "/atelier/character-documents/:document_id/moodboard/snapshots",
            post(record_moodboard_snapshot),
        )
        .route(
            "/atelier/character-documents/:document_id/moodboard/latest",
            get(latest_moodboard_snapshot),
        )
        .route(
            "/atelier/media-albums/:collection_id/items",
            get(list_media_album_items).post(add_media_album_items),
        )
        .route(
            "/atelier/media-assets/:asset_id/notes-tags",
            post(update_media_notes_tags),
        )
        .route("/atelier/ckc/search", post(search_ckc))
        .route("/atelier/ckc/tag-notes", post(upsert_ckc_tag_note))
        .route(
            "/atelier/posekit/openpose-export",
            post(export_posekit_openpose),
        )
        .route(
            "/atelier/sheet-versions/:version_id",
            get(get_sheet_version),
        )
        .route(
            "/atelier/sheet-versions/:version_id/export",
            get(export_sheet_version),
        )
        .route(
            "/atelier/sheet-templates/default",
            get(get_default_sheet_template),
        )
        .route(
            "/atelier/sheet-templates/default/safe-subset",
            get(get_default_sheet_template_safe_subset),
        )
        .route(
            "/atelier/sheet-field-suggestions",
            get(list_sheet_field_suggestions),
        )
        .route(
            "/atelier/intake/batches",
            get(list_intake_batches).post(create_intake_batch),
        )
        .route(
            "/atelier/intake/batches/:batch_id/items",
            get(list_intake_batch_items),
        )
        .route(
            "/atelier/intake/items/:item_id/classification",
            post(apply_intake_item_classification),
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
/// a fixed allowlist: only these literal identifiers are ever placed into a
/// `SELECT count(*)` statement, so no caller input reaches SQL.
const OVERVIEW_TABLES: &[&str] = &[
    "atelier_character",
    "atelier_media_asset",
    "atelier_collection",
    "atelier_collection_item",
    "atelier_media_asset_tag",
    "atelier_tag_note",
    "atelier_ckc_search_projection",
    "atelier_media_review_metadata",
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

fn atelier_store(state: &AppState) -> AtelierStore {
    AtelierStore::with_observability(
        state.postgres_pool.clone(),
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
struct CharacterResponse {
    internal_id: Uuid,
    public_id: String,
    display_name: String,
    character_ref: String,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateCharacterRequest {
    public_id: String,
    display_name: String,
    create_default_sheet: Option<bool>,
}

#[derive(Debug, Serialize)]
struct SheetVersionResponse {
    version_id: Uuid,
    character_internal_id: Uuid,
    parent_version_id: Option<Uuid>,
    seq: i64,
    raw_text: String,
    author: String,
    tool: Option<String>,
    character_ref: String,
    sheet_version_ref: String,
    created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SheetVersionConflictResponse {
    error: &'static str,
    character_internal_id: Uuid,
    character_ref: String,
    expected_parent_version_id: Option<Uuid>,
    expected_parent_sheet_version_ref: Option<String>,
    expected_sheet_version_ref: Option<String>,
    current_head_version_id: Option<Uuid>,
    current_head_sheet_version_ref: Option<String>,
    current_parent_version_id: Option<Uuid>,
    current_sheet_version_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppendSheetVersionRequest {
    raw_text: String,
    expected_parent_version_id: Option<Uuid>,
    tool: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SheetVersionExportQuery {
    format: Option<String>,
}

#[derive(Debug, Serialize)]
struct SheetVersionExportResponse {
    version_id: Uuid,
    character_internal_id: Uuid,
    format: String,
    file_name: String,
    content_hash: String,
    content: String,
    character_ref: String,
    sheet_version_ref: String,
}

#[derive(Debug, Deserialize)]
struct SheetFieldSuggestionsQuery {
    field_id: String,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CharacterDocumentsQuery {
    doc_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateCharacterDocumentRequest {
    doc_type: String,
    title: String,
    body_raw_text: String,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct AppendCharacterDocumentVersionRequest {
    title: String,
    body_raw_text: String,
    tags: Option<Vec<String>>,
    expected_parent_version_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct CharacterDocumentVersionResponse {
    version_id: Uuid,
    document_id: Uuid,
    document_ref: String,
    version_seq: i64,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    author: String,
    parent_version_id: Option<Uuid>,
    created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct CharacterDocumentVersionConflictResponse {
    error: &'static str,
    document_id: Uuid,
    document_ref: String,
    expected_parent_version_id: Option<Uuid>,
    expected_parent_document_version_ref: Option<String>,
    expected_document_version_ref: Option<String>,
    current_head_version_id: Option<Uuid>,
    current_head_document_version_ref: Option<String>,
    current_parent_version_id: Option<Uuid>,
    current_document_version_ref: Option<String>,
}

#[derive(Debug, Serialize)]
struct CharacterDocumentResponse {
    document_id: Uuid,
    document_ref: String,
    character_internal_id: Uuid,
    character_ref: String,
    doc_type: String,
    title: String,
    tags: Vec<String>,
    current_version_id: Uuid,
    current_version_seq: i64,
    current_version: Option<CharacterDocumentVersionResponse>,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct AddStoryCardRequest {
    title: String,
    body_raw_text: String,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct StoryCardResponse {
    card_id: Uuid,
    card_ref: String,
    story_document_id: Uuid,
    story_document_ref: String,
    seq: i64,
    title: String,
    body_raw_text: String,
    tags: Vec<String>,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct AddStoryBeatRequest {
    card_id: Option<Uuid>,
    beat_text: String,
}

#[derive(Debug, Serialize)]
struct StoryBeatResponse {
    beat_id: Uuid,
    beat_ref: String,
    story_document_id: Uuid,
    story_document_ref: String,
    card_id: Option<Uuid>,
    card_ref: Option<String>,
    seq: i64,
    beat_text: String,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct RecordMoodboardSnapshotRequest {
    raw_json_text: String,
    expected_document_version_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct MoodboardSnapshotResponse {
    snapshot_id: Uuid,
    moodboard_ref: String,
    document_id: Uuid,
    document_ref: String,
    document_version_id: Uuid,
    schema_id: String,
    schema_version: i64,
    raw_json_text: String,
    moodboard_json: serde_json::Value,
    moodboard: crate::atelier::moodboards::MoodboardDocument,
    content_sha256: String,
    author: String,
    created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MoodboardSnapshotConflictResponse {
    error: &'static str,
    document_id: Uuid,
    document_ref: String,
    expected_document_version_id: Option<Uuid>,
    expected_document_version_ref: Option<String>,
    current_head_version_id: Option<Uuid>,
    current_head_document_version_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateMediaAlbumRequest {
    name: String,
    notes: Option<String>,
    tags: Option<Vec<String>>,
    sheet_version_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct AddMediaAlbumItemsRequest {
    asset_ids: Vec<Uuid>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaAlbumItemsQuery {
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct AddMediaAlbumItemsResponse {
    collection_id: Uuid,
    collection_ref: String,
    requested: usize,
    inserted: i64,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<MediaAlbumMemberResponse>,
}

#[derive(Debug, Serialize)]
struct MediaAlbumItemsPageResponse {
    collection_id: Uuid,
    collection_ref: String,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<MediaAlbumMemberResponse>,
}

#[derive(Debug, Serialize)]
struct MediaAlbumResponse {
    collection_id: Uuid,
    collection_ref: String,
    name: String,
    notes: String,
    tags: Vec<String>,
    character_internal_id: Uuid,
    character_ref: String,
    sheet_version_id: Option<Uuid>,
    sheet_version_ref: Option<String>,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<MediaAlbumMemberResponse>,
    created_at_utc: DateTime<Utc>,
    updated_at_utc: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MediaAlbumMemberResponse {
    asset_id: Uuid,
    media_ref: String,
    content_hash: String,
    file_name: String,
    content_type: String,
    source_path: Option<String>,
    source_url: Option<String>,
    sort_order: i64,
    added_at_utc: DateTime<Utc>,
    notes: Option<String>,
    review_status: Option<String>,
    tags: Vec<String>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MediaNotesTagsRequest {
    notes: Option<String>,
    tags: Option<Vec<String>>,
    review_status: Option<String>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
}

#[derive(Debug, Serialize)]
struct MediaNotesTagsResponse {
    asset_id: Uuid,
    media_ref: String,
    notes: Option<String>,
    review_status: String,
    tags: Vec<String>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CkcSearchApiRequest {
    query: Option<String>,
    modes: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    character_internal_id: Option<Uuid>,
    collection_id: Option<Uuid>,
    media_asset_id: Option<Uuid>,
    similar_to_asset_id: Option<Uuid>,
    similar_to_dhash_hex: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CkcTagNoteRequest {
    tag_text: String,
    scope_ref: Option<String>,
    note: String,
}

#[derive(Debug, Serialize)]
struct PosekitArtifactResponse {
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
    byte_len: u64,
    mime: String,
    file_name: String,
}

#[derive(Debug, Serialize)]
struct PosekitSidecarResponse {
    sidecar_id: Uuid,
    rig_id: Uuid,
    kind: String,
    artifact_ref: String,
    manifest_ref: String,
    content_hash: String,
}

#[derive(Debug, Serialize)]
struct PosekitOpenPoseExportResponse {
    schema_id: String,
    source_ref: String,
    rig_id: Option<Uuid>,
    yaw_deg: i32,
    pitch_deg: i32,
    zoom_percent: i32,
    framing: PosekitExportFraming,
    marker_layers: PosekitMarkerLayers,
    applied_marker_edit_count: usize,
    width: i32,
    height: i32,
    openpose_json: serde_json::Value,
    openpose_json_sha256: String,
    openpose_png_sha256: String,
    content_hash: String,
    receipt_ref: String,
    openpose_png_artifact: PosekitArtifactResponse,
    openpose_json_artifact: PosekitArtifactResponse,
    sidecars: Vec<PosekitSidecarResponse>,
}

fn character_response(character: Character) -> CharacterResponse {
    CharacterResponse {
        internal_id: character.internal_id,
        public_id: character.public_id,
        display_name: character.display_name,
        character_ref: character_ref(character.internal_id),
        created_at_utc: character.created_at_utc,
        updated_at_utc: character.updated_at_utc,
    }
}

fn sheet_version_response(version: SheetVersion) -> SheetVersionResponse {
    SheetVersionResponse {
        version_id: version.version_id,
        character_internal_id: version.character_internal_id,
        parent_version_id: version.parent_version_id,
        seq: version.seq,
        raw_text: version.raw_text,
        author: version.author,
        tool: version.tool,
        character_ref: character_ref(version.character_internal_id),
        sheet_version_ref: sheet_version_ref(version.character_internal_id, version.version_id),
        created_at_utc: version.created_at_utc,
    }
}

fn character_document_ref(document_id: Uuid) -> String {
    format!("atelier://document/{document_id}")
}

fn character_document_version_ref(document_id: Uuid, version_id: Uuid) -> String {
    format!("atelier://document/{document_id}/version/{version_id}")
}

fn story_card_ref(card_id: Uuid) -> String {
    format!("atelier://story-card/{card_id}")
}

fn story_beat_ref(beat_id: Uuid) -> String {
    format!("atelier://story-beat/{beat_id}")
}

fn moodboard_ref(snapshot_id: Uuid) -> String {
    format!("atelier://moodboard/{snapshot_id}")
}

fn character_document_version_response(
    version: CharacterDocumentVersion,
) -> CharacterDocumentVersionResponse {
    CharacterDocumentVersionResponse {
        version_id: version.version_id,
        document_id: version.document_id,
        document_ref: character_document_ref(version.document_id),
        version_seq: version.version_seq,
        title: version.title,
        body_raw_text: version.body_raw_text,
        tags: version.tags,
        author: version.author,
        parent_version_id: version.parent_version_id,
        created_at_utc: version.created_at_utc,
    }
}

fn character_document_response_with_current_version(
    document: CharacterDocument,
    current_version: CharacterDocumentVersion,
) -> CharacterDocumentResponse {
    let current_version_id = current_version.version_id;
    let current_version_seq = current_version.version_seq;
    CharacterDocumentResponse {
        document_id: document.document_id,
        document_ref: character_document_ref(document.document_id),
        character_internal_id: document.character_internal_id,
        character_ref: character_ref(document.character_internal_id),
        doc_type: document.doc_type.as_token().to_owned(),
        title: document.title,
        tags: document.tags,
        current_version_id,
        current_version_seq,
        current_version: Some(character_document_version_response(current_version)),
        created_at_utc: document.created_at_utc,
        updated_at_utc: document.updated_at_utc,
    }
}

async fn character_document_response(
    store: &AtelierStore,
    document: CharacterDocument,
) -> Result<CharacterDocumentResponse, (StatusCode, Json<ErrorResponse>)> {
    let current_version = store
        .latest_character_document_version(document.document_id)
        .await
        .map_err(atelier_error)?
        .map(character_document_version_response);
    Ok(CharacterDocumentResponse {
        document_id: document.document_id,
        document_ref: character_document_ref(document.document_id),
        character_internal_id: document.character_internal_id,
        character_ref: character_ref(document.character_internal_id),
        doc_type: document.doc_type.as_token().to_owned(),
        title: document.title,
        tags: document.tags,
        current_version_id: document.current_version_id,
        current_version_seq: document.current_version_seq,
        current_version,
        created_at_utc: document.created_at_utc,
        updated_at_utc: document.updated_at_utc,
    })
}

fn story_card_response(card: StoryCard) -> StoryCardResponse {
    StoryCardResponse {
        card_id: card.card_id,
        card_ref: story_card_ref(card.card_id),
        story_document_id: card.story_document_id,
        story_document_ref: character_document_ref(card.story_document_id),
        seq: card.seq,
        title: card.title,
        body_raw_text: card.body_raw_text,
        tags: card.tags,
        created_at_utc: card.created_at_utc,
        updated_at_utc: card.updated_at_utc,
    }
}

fn story_beat_response(beat: StoryBeat) -> StoryBeatResponse {
    StoryBeatResponse {
        beat_id: beat.beat_id,
        beat_ref: story_beat_ref(beat.beat_id),
        story_document_id: beat.story_document_id,
        story_document_ref: character_document_ref(beat.story_document_id),
        card_id: beat.card_id,
        card_ref: beat.card_id.map(story_card_ref),
        seq: beat.seq,
        beat_text: beat.beat_text,
        created_at_utc: beat.created_at_utc,
        updated_at_utc: beat.updated_at_utc,
    }
}

fn moodboard_snapshot_response(snapshot: MoodboardSnapshot) -> MoodboardSnapshotResponse {
    MoodboardSnapshotResponse {
        snapshot_id: snapshot.snapshot_id,
        moodboard_ref: moodboard_ref(snapshot.snapshot_id),
        document_id: snapshot.document_id,
        document_ref: character_document_ref(snapshot.document_id),
        document_version_id: snapshot.document_version_id,
        schema_id: snapshot.schema_id,
        schema_version: snapshot.schema_version,
        raw_json_text: snapshot.raw_json_text,
        moodboard_json: snapshot.moodboard_json,
        moodboard: snapshot.moodboard,
        content_sha256: snapshot.content_sha256,
        author: snapshot.author,
        created_at_utc: snapshot.created_at_utc,
    }
}

fn sheet_version_conflict_response(
    character_internal_id: Uuid,
    expected_parent_version_id: Option<Uuid>,
    current: Option<SheetVersion>,
) -> SheetVersionConflictResponse {
    let current_parent_version_id = current.as_ref().map(|version| version.version_id);
    SheetVersionConflictResponse {
        error: "stale_sheet_version",
        character_internal_id,
        character_ref: character_ref(character_internal_id),
        expected_parent_version_id,
        expected_parent_sheet_version_ref: expected_parent_version_id
            .map(|version_id| sheet_version_ref(character_internal_id, version_id)),
        expected_sheet_version_ref: expected_parent_version_id
            .map(|version_id| sheet_version_ref(character_internal_id, version_id)),
        current_head_version_id: current_parent_version_id,
        current_head_sheet_version_ref: current_parent_version_id
            .map(|version_id| sheet_version_ref(character_internal_id, version_id)),
        current_parent_version_id,
        current_sheet_version_ref: current_parent_version_id
            .map(|version_id| sheet_version_ref(character_internal_id, version_id)),
    }
}

fn character_document_version_conflict_response(
    document_id: Uuid,
    expected_parent_version_id: Option<Uuid>,
    current: Option<CharacterDocumentVersion>,
) -> CharacterDocumentVersionConflictResponse {
    let current_parent_version_id = current.as_ref().map(|version| version.version_id);
    CharacterDocumentVersionConflictResponse {
        error: "stale_character_document_version",
        document_id,
        document_ref: character_document_ref(document_id),
        expected_parent_version_id,
        expected_parent_document_version_ref: expected_parent_version_id
            .map(|version_id| character_document_version_ref(document_id, version_id)),
        expected_document_version_ref: expected_parent_version_id
            .map(|version_id| character_document_version_ref(document_id, version_id)),
        current_head_version_id: current_parent_version_id,
        current_head_document_version_ref: current_parent_version_id
            .map(|version_id| character_document_version_ref(document_id, version_id)),
        current_parent_version_id,
        current_document_version_ref: current_parent_version_id
            .map(|version_id| character_document_version_ref(document_id, version_id)),
    }
}

fn moodboard_snapshot_conflict_response(
    document_id: Uuid,
    expected_document_version_id: Option<Uuid>,
    current_head_version_id: Option<Uuid>,
) -> MoodboardSnapshotConflictResponse {
    MoodboardSnapshotConflictResponse {
        error: "stale_moodboard_document_version",
        document_id,
        document_ref: character_document_ref(document_id),
        expected_document_version_id,
        expected_document_version_ref: expected_document_version_id
            .map(|version_id| character_document_version_ref(document_id, version_id)),
        current_head_version_id,
        current_head_document_version_ref: current_head_version_id
            .map(|version_id| character_document_version_ref(document_id, version_id)),
    }
}

fn collection_tags_from_row(row: &sqlx::postgres::PgRow) -> Vec<String> {
    let tags_json: serde_json::Value = row.get("tags_json");
    tags_json
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn collection_from_api_row(row: &sqlx::postgres::PgRow) -> Collection {
    Collection {
        collection_id: row.get("collection_id"),
        name: row.get("name"),
        notes: row.get("notes"),
        tags: collection_tags_from_row(row),
        character_internal_id: row.get("character_internal_id"),
        sheet_version_id: row.get("sheet_version_id"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn normalize_media_tags_for_api(tags: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = normalize_tag(tag);
        if tag.is_empty() {
            continue;
        }
        if !normalized.iter().any(|existing| existing == &tag) {
            normalized.push(tag);
        }
    }
    normalized
}

fn normalize_media_review_status_for_api(
    status: Option<&str>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    match status
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("pass" | "passed" | "approve" | "approved") => Ok("approved".to_owned()),
        Some("reject" | "rejected") => Ok("rejected".to_owned()),
        Some("unsure" | "hold" | "defer" | "deferred") => Ok("deferred".to_owned()),
        Some("review") => Ok("review".to_owned()),
        Some("unreviewed") | None => Ok("unreviewed".to_owned()),
        Some(other) => Err(atelier_error(AtelierError::Validation(format!(
            "unsupported review_status: {other}"
        )))),
    }
}

fn media_notes_ref_for_api(notes: &str) -> String {
    format!("sha256:{}", text_hash(notes))
}

fn validate_optional_provenance_ref_for_api(
    field: &str,
    value: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(raw) = value else {
        return Ok(());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed != raw {
        return Err(atelier_error(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        ))));
    }
    match reject_legacy_runtime_ref(field, raw) {
        Ok(()) => Ok(()),
        Err(AtelierError::ForbiddenStorage(message)) => {
            Err(atelier_error(AtelierError::Validation(message)))
        }
        Err(err) => Err(atelier_error(err)),
    }
}

fn portable_optional_provenance_ref_for_api(field: &str, value: Option<String>) -> Option<String> {
    value.filter(|raw| reject_legacy_runtime_ref(field, raw).is_ok())
}

async fn ensure_sheet_version_matches_character(
    store: &AtelierStore,
    character_internal_id: Uuid,
    sheet_version_id: Option<Uuid>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if let Some(version_id) = sheet_version_id {
        let version = store
            .get_sheet_version(version_id)
            .await
            .map_err(atelier_error)?;
        if version.character_internal_id != character_internal_id {
            return Err(atelier_error(AtelierError::Validation(format!(
                "sheet_version_id={version_id} does not belong to character_internal_id={character_internal_id}"
            ))));
        }
    }
    Ok(())
}

async fn list_character_collections(
    store: &AtelierStore,
    character_internal_id: Uuid,
) -> Result<Vec<Collection>, (StatusCode, Json<ErrorResponse>)> {
    let rows = sqlx::query(
        r#"SELECT collection_id, name, notes, tags_json,
                  character_internal_id, sheet_version_id,
                  created_at_utc, updated_at_utc
           FROM atelier_collection
           WHERE character_internal_id = $1
           ORDER BY updated_at_utc DESC, collection_id ASC
           LIMIT $2"#,
    )
    .bind(character_internal_id)
    .bind(LIST_CAP)
    .fetch_all(store.pool())
    .await
    .map_err(internal_error)?;
    Ok(rows.iter().map(collection_from_api_row).collect())
}

async fn media_album_members_response(
    store: &AtelierStore,
    collection_id: Uuid,
) -> Result<(Vec<MediaAlbumMemberResponse>, usize, Option<i64>), (StatusCode, Json<ErrorResponse>)>
{
    media_album_members_page_response(store, collection_id, 0, LIST_CAP).await
}

async fn media_album_members_page_response(
    store: &AtelierStore,
    collection_id: Uuid,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MediaAlbumMemberResponse>, usize, Option<i64>), (StatusCode, Json<ErrorResponse>)>
{
    let total_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM atelier_collection_item WHERE collection_id = $1")
            .bind(collection_id)
            .fetch_one(store.pool())
            .await
            .map_err(internal_error)?;
    let rows = sqlx::query(
        r#"SELECT ci.asset_id, ma.content_hash, ma.mime, ma.source_provenance,
                  ci.sort_order, ci.added_at_utc,
                  ci.source_path_ref AS link_source_path_ref,
                  ci.source_url_ref AS link_source_url_ref
           FROM atelier_collection_item ci
           JOIN atelier_media_asset ma ON ma.asset_id = ci.asset_id
           WHERE ci.collection_id = $1
           ORDER BY ci.sort_order ASC, ci.added_at_utc ASC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(collection_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(store.pool())
    .await
    .map_err(internal_error)?;

    let mut members = Vec::with_capacity(rows.len());
    for row in rows {
        let asset_id: Uuid = row.get("asset_id");
        let content_hash: String = row.get("content_hash");
        let source_provenance: Option<String> = row.get("source_provenance");
        let (source_path, source_url) = split_media_source_provenance(source_provenance.as_deref());
        let metadata = store
            .get_media_review_metadata(asset_id)
            .await
            .map_err(atelier_error)?;
        let tags = store
            .list_media_asset_tags(asset_id)
            .await
            .map_err(atelier_error)?
            .into_iter()
            .map(|tag| tag.text)
            .collect();
        let provenance = store
            .get_media_source_provenance_refs(asset_id)
            .await
            .map_err(atelier_error)?;
        let link_source_path_ref: Option<String> = portable_optional_provenance_ref_for_api(
            "source_path_ref",
            row.get("link_source_path_ref"),
        );
        let link_source_url_ref: Option<String> = portable_optional_provenance_ref_for_api(
            "source_url_ref",
            row.get("link_source_url_ref"),
        );
        members.push(MediaAlbumMemberResponse {
            asset_id,
            media_ref: media_asset_ref(asset_id),
            file_name: media_display_file_name(source_provenance.as_deref(), &content_hash),
            content_hash,
            content_type: row.get("mime"),
            source_path,
            source_url,
            sort_order: row.get("sort_order"),
            added_at_utc: row.get("added_at_utc"),
            notes: metadata.as_ref().and_then(|row| row.notes.clone()),
            review_status: metadata.as_ref().map(|row| row.review_status.clone()),
            tags,
            source_path_ref: link_source_path_ref.or_else(|| {
                provenance.as_ref().and_then(|row| {
                    portable_optional_provenance_ref_for_api(
                        "source_path_ref",
                        row.source_path_ref.clone(),
                    )
                })
            }),
            source_url_ref: link_source_url_ref.or_else(|| {
                provenance.as_ref().and_then(|row| {
                    portable_optional_provenance_ref_for_api(
                        "source_url_ref",
                        row.source_url_ref.clone(),
                    )
                })
            }),
        });
    }
    let next_offset = offset.saturating_add(members.len() as i64);
    let members_next_offset = if total_count > next_offset {
        Some(next_offset)
    } else {
        None
    };
    Ok((members, total_count.max(0) as usize, members_next_offset))
}

fn normalize_media_album_items_page_query(
    query: MediaAlbumItemsQuery,
) -> Result<(i64, i64), (StatusCode, Json<ErrorResponse>)> {
    let offset = query.offset.unwrap_or(0);
    if offset < 0 {
        return Err(atelier_error(AtelierError::Validation(
            "offset must be >= 0".to_owned(),
        )));
    }
    let limit = query.limit.unwrap_or(LIST_CAP);
    if limit < 1 {
        return Err(atelier_error(AtelierError::Validation(
            "limit must be >= 1".to_owned(),
        )));
    }
    Ok((offset, limit.min(LIST_CAP)))
}

async fn media_album_response(
    store: &AtelierStore,
    collection: Collection,
) -> Result<MediaAlbumResponse, (StatusCode, Json<ErrorResponse>)> {
    let Some(character_internal_id) = collection.character_internal_id else {
        return Err(atelier_error(AtelierError::Validation(format!(
            "collection_id={} is not linked to a CKC character",
            collection.collection_id
        ))));
    };
    let (members, member_count, members_next_offset) =
        media_album_members_response(store, collection.collection_id).await?;
    Ok(MediaAlbumResponse {
        collection_id: collection.collection_id,
        collection_ref: collection_ref(collection.collection_id),
        name: collection.name,
        notes: collection.notes,
        tags: collection.tags,
        character_internal_id,
        character_ref: character_ref(character_internal_id),
        sheet_version_id: collection.sheet_version_id,
        sheet_version_ref: collection
            .sheet_version_id
            .map(|version_id| sheet_version_ref(character_internal_id, version_id)),
        member_count,
        members_next_offset,
        members,
        created_at_utc: collection.created_at_utc,
        updated_at_utc: collection.updated_at_utc,
    })
}

fn split_media_source_provenance(source: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(source) = source.map(str::trim).filter(|value| !value.is_empty()) else {
        return (None, None);
    };
    if source.starts_with("http://") || source.starts_with("https://") {
        (None, Some(source.to_owned()))
    } else {
        (Some(source.to_owned()), None)
    }
}

fn media_display_file_name(source: Option<&str>, content_hash: &str) -> String {
    if let Some(source) = source.map(str::trim).filter(|value| !value.is_empty()) {
        if let Some(name) = source
            .trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return name.to_owned();
        }
    }
    let suffix: String = content_hash
        .trim_start_matches("sha256:")
        .chars()
        .take(12)
        .collect();
    if suffix.is_empty() {
        "media".to_owned()
    } else {
        format!("media-{suffix}")
    }
}

fn safe_subset_sheet_text(raw_text: &str) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let safe_subset = builtin_safe_subset().map_err(atelier_error)?;
    let safe_ids = safe_subset
        .field_ids
        .into_iter()
        .map(|field_id| field_id.to_ascii_uppercase())
        .collect::<HashSet<_>>();
    let mut out = String::with_capacity(raw_text.len());
    for segment in raw_text.split_inclusive('\n') {
        let trimmed_line = segment.trim_end_matches(['\r', '\n']);
        match sheet_field_id_from_line(trimmed_line) {
            Some(field_id) if safe_ids.contains(&field_id) => out.push_str(segment),
            Some(_) => {}
            None if sheet_line_looks_like_field(trimmed_line) => {}
            None => out.push_str(segment),
        }
    }
    Ok(out)
}

fn export_sheet_json(
    version: &SheetVersion,
    raw_text: &str,
    export_format: &str,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    serde_json::to_string_pretty(&serde_json::json!({
        "export_format": export_format,
        "template_version": CHARACTER_SHEET_V2_TEMPLATE_VERSION,
        "version_id": version.version_id,
        "character_internal_id": version.character_internal_id,
        "parent_version_id": version.parent_version_id,
        "seq": version.seq,
        "author": &version.author,
        "tool": &version.tool,
        "character_ref": character_ref(version.character_internal_id),
        "sheet_version_ref": sheet_version_ref(
            version.character_internal_id,
            version.version_id,
        ),
        "raw_text": raw_text,
        "created_at_utc": version.created_at_utc,
    }))
    .map_err(|err| internal_error(format!("serialize CKC sheet export JSON failed: {err}")))
}

fn raw_text_from_export_json(value: &serde_json::Value) -> Option<String> {
    if let Some(raw_text) = value.get("raw_text").and_then(|value| value.as_str()) {
        return Some(raw_text.to_owned());
    }
    let content = value.get("content").and_then(|value| value.as_str())?;
    if content.trim_start().starts_with('{') {
        serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|nested| raw_text_from_export_json(&nested))
            .or_else(|| Some(content.to_owned()))
    } else {
        Some(content.to_owned())
    }
}

fn import_sheet_raw_text(raw_text: &str) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let trimmed = raw_text.trim();
    if trimmed.is_empty() {
        return Err(atelier_error(AtelierError::Validation(
            "CKC sheet import raw_text must not be empty".to_owned(),
        )));
    }
    if trimmed.starts_with('{') {
        let value = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|err| {
            atelier_error(AtelierError::Validation(format!(
                "CKC sheet import JSON is invalid: {err}"
            )))
        })?;
        return raw_text_from_export_json(&value).ok_or_else(|| {
            atelier_error(AtelierError::Validation(
                "CKC sheet import JSON must contain raw_text or content".to_owned(),
            ))
        });
    }
    Ok(raw_text.to_owned())
}

fn validate_ckc_sheet_owner(
    character: &Character,
    raw_text: &str,
    require_character_id: bool,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let character_ids = sheet_field_values(raw_text, "CHAR-ID-001");
    if require_character_id && character_ids.is_empty() {
        return Err(atelier_error(AtelierError::Validation(
            "CKC sheet write must include CHAR-ID-001 for character ownership".to_owned(),
        )));
    }
    if character_ids.len() > 1 {
        return Err(atelier_error(AtelierError::Validation(format!(
            "CKC sheet write must include exactly one CHAR-ID-001 for character ownership; found {}",
            character_ids.len()
        ))));
    }
    if let Some(character_id) = character_ids.into_iter().next() {
        if character_id != character.public_id {
            return Err(atelier_error(AtelierError::Validation(format!(
                "CKC sheet CHAR-ID-001={character_id} does not match character public_id={}",
                character.public_id
            ))));
        }
    }
    Ok(())
}

/// GET /atelier/characters — stable CKC character list for model/operator selection.
async fn list_characters(
    State(state): State<AppState>,
) -> Result<Json<Vec<CharacterResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let rows = store
        .list_characters(LIST_CAP)
        .await
        .map_err(atelier_error)?;
    Ok(Json(rows.into_iter().map(character_response).collect()))
}

/// POST /atelier/characters — create a CKC character identity.
async fn create_character(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateCharacterRequest>,
) -> Result<(StatusCode, Json<CharacterResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let display_name = payload.display_name;
    let character = store
        .create_character(&NewCharacter {
            public_id: payload.public_id,
            display_name: display_name.clone(),
        })
        .await
        .map_err(atelier_error)?;
    if payload.create_default_sheet.unwrap_or(false) {
        let raw_text = default_character_sheet_text(&character.public_id, &character.display_name);
        store
            .append_sheet_version_if_current(
                &NewSheetVersion {
                    character_internal_id: character.internal_id,
                    raw_text,
                    author: actor.clone(),
                    tool: Some(DEFAULT_SHEET_TOOL.to_owned()),
                },
                None,
            )
            .await
            .map_err(atelier_error)?;
    }
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/characters",
        status = "created",
        actor = %actor,
        character_internal_id = %character.internal_id,
        public_id = %character.public_id,
        "create CKC character"
    );
    Ok((StatusCode::CREATED, Json(character_response(character))))
}

/// GET /atelier/characters/:character_internal_id — read one CKC character identity.
async fn get_character(
    State(state): State<AppState>,
    Path(character_internal_id): Path<Uuid>,
) -> Result<Json<CharacterResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let character = store
        .get_character_by_internal_id(character_internal_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(character_response(character)))
}

/// GET /atelier/characters/:character_internal_id/sheet-versions — append-only version history.
async fn list_sheet_versions(
    State(state): State<AppState>,
    Path(character_internal_id): Path<Uuid>,
) -> Result<Json<Vec<SheetVersionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    store
        .get_character_by_internal_id(character_internal_id)
        .await
        .map_err(atelier_error)?;
    let versions = store
        .sheet_version_history(character_internal_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(
        versions.into_iter().map(sheet_version_response).collect(),
    ))
}

fn parse_character_document_type(
    raw: &str,
) -> Result<CharacterDocumentType, (StatusCode, Json<ErrorResponse>)> {
    CharacterDocumentType::from_token(raw.trim()).map_err(atelier_error)
}

/// GET /atelier/characters/:character_internal_id/documents — CKC story/moodboard/note document refs.
async fn list_character_documents(
    State(state): State<AppState>,
    Path(character_internal_id): Path<Uuid>,
    Query(query): Query<CharacterDocumentsQuery>,
) -> Result<Json<Vec<CharacterDocumentResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    store
        .get_character_by_internal_id(character_internal_id)
        .await
        .map_err(atelier_error)?;
    let doc_type = match query.doc_type {
        Some(raw) => Some(parse_character_document_type(&raw)?),
        None => None,
    };
    let documents = store
        .list_character_documents(character_internal_id, doc_type)
        .await
        .map_err(atelier_error)?;
    let mut out = Vec::with_capacity(documents.len());
    for document in documents {
        out.push(character_document_response(&store, document).await?);
    }
    Ok(Json(out))
}

/// POST /atelier/characters/:character_internal_id/documents — create a CKC story/moodboard/note document.
async fn create_character_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Json(payload): Json<CreateCharacterDocumentRequest>,
) -> Result<(StatusCode, Json<CharacterDocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    store
        .get_character_by_internal_id(character_internal_id)
        .await
        .map_err(atelier_error)?;
    let doc_type = parse_character_document_type(&payload.doc_type)?;
    let version = store
        .create_character_document(&NewCharacterDocument {
            character_internal_id,
            doc_type,
            title: payload.title,
            body_raw_text: payload.body_raw_text,
            tags: payload.tags.unwrap_or_default(),
            author: actor.clone(),
        })
        .await
        .map_err(atelier_error)?;
    let document = store
        .get_character_document(version.document_id)
        .await
        .map_err(atelier_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/characters/:character_internal_id/documents",
        status = "created",
        actor = %actor,
        character_internal_id = %character_internal_id,
        document_id = %document.document_id,
        doc_type = %document.doc_type.as_token(),
        "create CKC character document"
    );
    Ok((
        StatusCode::CREATED,
        Json(character_document_response(&store, document).await?),
    ))
}

/// GET /atelier/character-documents/:document_id — read one CKC character document.
async fn get_character_document(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<CharacterDocumentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let document = store
        .get_character_document(document_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(character_document_response(&store, document).await?))
}

/// GET /atelier/character-documents/:document_id/versions — append-only story/moodboard/note history.
async fn list_character_document_versions(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<Vec<CharacterDocumentVersionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    store
        .get_character_document(document_id)
        .await
        .map_err(atelier_error)?;
    let versions = store
        .character_document_history(document_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(
        versions
            .into_iter()
            .map(character_document_version_response)
            .collect(),
    ))
}

/// POST /atelier/character-documents/:document_id/versions — append story/moodboard/note text.
async fn append_character_document_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<AppendCharacterDocumentVersionRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let expected_parent_version_id = payload.expected_parent_version_id;
    let (document, appended_version) = match store
        .append_character_document_version_and_document_if_current(
            document_id,
            &AppendCharacterDocumentVersion {
                title: payload.title,
                body_raw_text: payload.body_raw_text,
                tags: payload.tags.unwrap_or_default(),
                author: actor.clone(),
            },
            expected_parent_version_id,
        )
        .await
    {
        Ok((document, version)) => (document, version),
        Err(AtelierError::Conflict(detail)) => {
            tracing::warn!(
                target: "handshake_core::atelier",
                %detail,
                document_id = %document_id,
                "stale CKC character document version write"
            );
            let current = store
                .latest_character_document_version(document_id)
                .await
                .map_err(atelier_error)?;
            let response = character_document_version_conflict_response(
                document_id,
                expected_parent_version_id,
                current,
            );
            return Ok((StatusCode::CONFLICT, Json(response)).into_response());
        }
        Err(err) => return Err(atelier_error(err)),
    };
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/character-documents/:document_id/versions",
        status = "ok",
        actor = %actor,
        document_id = %document_id,
        "append CKC character document version"
    );
    Ok((
        StatusCode::CREATED,
        Json(character_document_response_with_current_version(
            document,
            appended_version,
        )),
    )
        .into_response())
}

/// GET /atelier/character-documents/:document_id/story-cards — list reusable story cards.
async fn list_story_cards(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<Vec<StoryCardResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let cards = store
        .list_story_cards(document_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(cards.into_iter().map(story_card_response).collect()))
}

/// POST /atelier/character-documents/:document_id/story-cards — add a story card.
async fn add_story_card(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<AddStoryCardRequest>,
) -> Result<(StatusCode, Json<StoryCardResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let card = store
        .add_story_card(&NewStoryCard {
            story_document_id: document_id,
            title: payload.title,
            body_raw_text: payload.body_raw_text,
            tags: payload.tags.unwrap_or_default(),
        })
        .await
        .map_err(atelier_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/character-documents/:document_id/story-cards",
        status = "created",
        actor = %actor,
        document_id = %document_id,
        card_id = %card.card_id,
        "add CKC story card"
    );
    Ok((StatusCode::CREATED, Json(story_card_response(card))))
}

/// GET /atelier/character-documents/:document_id/story-beats — list reusable story beats.
async fn list_story_beats(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<Vec<StoryBeatResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let beats = store
        .list_story_beats(document_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(beats.into_iter().map(story_beat_response).collect()))
}

/// POST /atelier/character-documents/:document_id/story-beats — add a story beat.
async fn add_story_beat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<AddStoryBeatRequest>,
) -> Result<(StatusCode, Json<StoryBeatResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let beat = store
        .add_story_beat(&NewStoryBeat {
            story_document_id: document_id,
            card_id: payload.card_id,
            beat_text: payload.beat_text,
        })
        .await
        .map_err(atelier_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/character-documents/:document_id/story-beats",
        status = "created",
        actor = %actor,
        document_id = %document_id,
        beat_id = %beat.beat_id,
        "add CKC story beat"
    );
    Ok((StatusCode::CREATED, Json(story_beat_response(beat))))
}

/// POST /atelier/character-documents/:document_id/moodboard/snapshots — record a native moodboard snapshot.
async fn record_moodboard_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<RecordMoodboardSnapshotRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let expected_document_version_id = payload.expected_document_version_id;
    let snapshot = match store
        .record_moodboard_snapshot(&NewMoodboardSnapshot {
            document_id,
            raw_json_text: payload.raw_json_text,
            expected_document_version_id,
            author: actor.clone(),
        })
        .await
    {
        Ok(snapshot) => snapshot,
        Err(AtelierError::Conflict(detail)) => {
            tracing::warn!(
                target: "handshake_core::atelier",
                %detail,
                document_id = %document_id,
                "stale CKC moodboard snapshot write"
            );
            let current_head_version_id = store
                .latest_character_document_version(document_id)
                .await
                .map_err(atelier_error)?
                .map(|version| version.version_id);
            let response = moodboard_snapshot_conflict_response(
                document_id,
                expected_document_version_id,
                current_head_version_id,
            );
            return Ok((StatusCode::CONFLICT, Json(response)).into_response());
        }
        Err(err) => return Err(atelier_error(err)),
    };
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/character-documents/:document_id/moodboard/snapshots",
        status = "created",
        actor = %actor,
        document_id = %document_id,
        snapshot_id = %snapshot.snapshot_id,
        "record CKC moodboard snapshot"
    );
    Ok((
        StatusCode::CREATED,
        Json(moodboard_snapshot_response(snapshot)),
    )
        .into_response())
}

/// GET /atelier/character-documents/:document_id/moodboard/latest — open the latest native moodboard snapshot.
async fn latest_moodboard_snapshot(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<MoodboardSnapshotResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let document = store
        .get_character_document(document_id)
        .await
        .map_err(atelier_error)?;
    if document.doc_type != CharacterDocumentType::Moodboard {
        return Err(atelier_error(AtelierError::Validation(format!(
            "document {document_id} is {}, expected moodboard",
            document.doc_type.as_token()
        ))));
    }
    let snapshot = store
        .latest_moodboard_snapshot(document_id)
        .await
        .map_err(atelier_error)?
        .ok_or_else(|| atelier_error(AtelierError::NotFound(format!("moodboard {document_id}"))))?;
    Ok(Json(moodboard_snapshot_response(snapshot)))
}

/// GET /atelier/characters/:character_internal_id/media-albums — character-scoped CKC albums.
async fn list_character_media_albums(
    State(state): State<AppState>,
    Path(character_internal_id): Path<Uuid>,
) -> Result<Json<Vec<MediaAlbumResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    store
        .get_character_by_internal_id(character_internal_id)
        .await
        .map_err(atelier_error)?;
    let collections = list_character_collections(&store, character_internal_id).await?;
    let mut out = Vec::with_capacity(collections.len());
    for collection in collections {
        out.push(media_album_response(&store, collection).await?);
    }
    Ok(Json(out))
}

/// POST /atelier/characters/:character_internal_id/media-albums — create a CKC album over existing media.
async fn create_character_media_album(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Json(payload): Json<CreateMediaAlbumRequest>,
) -> Result<(StatusCode, Json<MediaAlbumResponse>), (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    store
        .get_character_by_internal_id(character_internal_id)
        .await
        .map_err(atelier_error)?;
    ensure_sheet_version_matches_character(&store, character_internal_id, payload.sheet_version_id)
        .await?;
    let collection = store
        .create_collection_attributed(
            &NewCollection {
                name: payload.name,
                notes: payload.notes.unwrap_or_default(),
                tags: payload.tags.unwrap_or_default(),
                character_internal_id: Some(character_internal_id),
                sheet_version_id: payload.sheet_version_id,
            },
            &actor,
        )
        .await
        .map_err(atelier_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/characters/:character_internal_id/media-albums",
        status = "created",
        actor = %actor,
        character_internal_id = %character_internal_id,
        collection_id = %collection.collection_id,
        "create CKC media album"
    );
    Ok((
        StatusCode::CREATED,
        Json(media_album_response(&store, collection).await?),
    ))
}

/// GET /atelier/media-albums/:collection_id/items — fetch a page of album media members.
async fn list_media_album_items(
    State(state): State<AppState>,
    Path(collection_id): Path<Uuid>,
    Query(query): Query<MediaAlbumItemsQuery>,
) -> Result<Json<MediaAlbumItemsPageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let collection = store
        .get_collection(collection_id)
        .await
        .map_err(atelier_error)?;
    if collection.character_internal_id.is_none() {
        return Err(atelier_error(AtelierError::Validation(format!(
            "collection_id={collection_id} is not a CKC character album"
        ))));
    }
    let (offset, limit) = normalize_media_album_items_page_query(query)?;
    let (members, member_count, members_next_offset) =
        media_album_members_page_response(&store, collection_id, offset, limit).await?;
    Ok(Json(MediaAlbumItemsPageResponse {
        collection_id,
        collection_ref: collection_ref(collection_id),
        member_count,
        members_next_offset,
        members,
    }))
}

/// POST /atelier/media-albums/:collection_id/items — append existing media assets to an album.
async fn add_media_album_items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection_id): Path<Uuid>,
    Json(payload): Json<AddMediaAlbumItemsRequest>,
) -> Result<Json<AddMediaAlbumItemsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let collection = store
        .get_collection(collection_id)
        .await
        .map_err(atelier_error)?;
    if collection.character_internal_id.is_none() {
        return Err(atelier_error(AtelierError::Validation(format!(
            "collection_id={collection_id} is not a CKC character album"
        ))));
    }
    validate_optional_provenance_ref_for_api(
        "source_path_ref",
        payload.source_path_ref.as_deref(),
    )?;
    validate_optional_provenance_ref_for_api("source_url_ref", payload.source_url_ref.as_deref())?;
    let requested = payload.asset_ids.len();
    let inserted = store
        .add_images_to_collection_with_link_refs_attributed(
            collection_id,
            &payload.asset_ids,
            payload.source_path_ref.as_deref(),
            payload.source_url_ref.as_deref(),
            &actor,
        )
        .await
        .map_err(atelier_error)?;
    let (members, member_count, members_next_offset) =
        media_album_members_response(&store, collection_id).await?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/media-albums/:collection_id/items",
        status = "ok",
        actor = %actor,
        collection_id = %collection_id,
        requested = requested,
        inserted = inserted,
        "add CKC media album items"
    );
    Ok(Json(AddMediaAlbumItemsResponse {
        collection_id,
        collection_ref: collection_ref(collection_id),
        requested,
        inserted,
        member_count,
        members_next_offset,
        members,
    }))
}

/// POST /atelier/media-assets/:asset_id/notes-tags — save image notes/tags separate from sheet text.
async fn update_media_notes_tags(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(asset_id): Path<Uuid>,
    Json(payload): Json<MediaNotesTagsRequest>,
) -> Result<Json<MediaNotesTagsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    validate_optional_provenance_ref_for_api(
        "source_path_ref",
        payload.source_path_ref.as_deref(),
    )?;
    validate_optional_provenance_ref_for_api("source_url_ref", payload.source_url_ref.as_deref())?;
    reject_legacy_runtime_ref("source provenance updated_by", &actor).map_err(atelier_error)?;
    let MediaNotesTagsRequest {
        notes,
        tags,
        review_status,
        source_path_ref,
        source_url_ref,
    } = payload;
    let desired_tags = tags.as_ref().map(|tags| normalize_media_tags_for_api(tags));

    let mut tx = store
        .pool()
        .begin()
        .await
        .map_err(|err| atelier_error(AtelierError::Database(err)))?;
    let asset_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM atelier_media_asset WHERE asset_id = $1)")
            .bind(asset_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| atelier_error(AtelierError::Database(err)))?;
    if !asset_exists {
        return Err(atelier_error(AtelierError::NotFound(format!(
            "media asset_id={asset_id}"
        ))));
    }

    let existing_metadata = sqlx::query(
        r#"SELECT asset_id, favorite, rating, frontpage, carousel, notes,
                  review_status, updated_by, updated_at_utc
           FROM atelier_media_review_metadata
           WHERE asset_id = $1
           FOR UPDATE"#,
    )
    .bind(asset_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| atelier_error(AtelierError::Database(err)))?;
    let favorite: bool = existing_metadata
        .as_ref()
        .map(|row| row.get("favorite"))
        .unwrap_or(false);
    let rating: i16 = existing_metadata
        .as_ref()
        .map(|row| row.get("rating"))
        .unwrap_or(0);
    let frontpage: bool = existing_metadata
        .as_ref()
        .map(|row| row.get("frontpage"))
        .unwrap_or(false);
    let carousel: bool = existing_metadata
        .as_ref()
        .map(|row| row.get("carousel"))
        .unwrap_or(false);
    let existing_notes: Option<String> =
        existing_metadata.as_ref().and_then(|row| row.get("notes"));
    let existing_review_status: Option<String> = existing_metadata
        .as_ref()
        .map(|row| row.get("review_status"));
    let notes = notes.or(existing_notes);
    let review_status = normalize_media_review_status_for_api(
        review_status
            .as_deref()
            .or(existing_review_status.as_deref()),
    )?;
    let metadata_row = sqlx::query(
        r#"INSERT INTO atelier_media_review_metadata (
               asset_id, favorite, rating, frontpage, carousel, notes,
               review_status, updated_by, updated_at_utc
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
           ON CONFLICT (asset_id) DO UPDATE SET
               favorite = EXCLUDED.favorite,
               rating = EXCLUDED.rating,
               frontpage = EXCLUDED.frontpage,
               carousel = EXCLUDED.carousel,
               notes = EXCLUDED.notes,
               review_status = EXCLUDED.review_status,
               updated_by = EXCLUDED.updated_by,
               updated_at_utc = NOW()
           RETURNING asset_id, favorite, rating, frontpage, carousel, notes,
                     review_status, updated_by, updated_at_utc"#,
    )
    .bind(asset_id)
    .bind(favorite)
    .bind(rating)
    .bind(frontpage)
    .bind(carousel)
    .bind(&notes)
    .bind(&review_status)
    .bind(&actor)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| atelier_error(AtelierError::Database(err)))?;
    let notes: Option<String> = metadata_row.get("notes");
    let review_status: String = metadata_row.get("review_status");
    store
        .record_event_in_tx(
            &mut tx,
            crate::atelier::event_family::MEDIA_REVIEW_METADATA_UPDATED,
            "atelier_media_review_metadata",
            &asset_id.to_string(),
            serde_json::json!({
                "asset_id": asset_id,
                "favorite": favorite,
                "rating": rating,
                "frontpage": frontpage,
                "carousel": carousel,
                "review_status": &review_status,
                "notes_present": notes.is_some(),
                "notes_ref": notes.as_deref().map(media_notes_ref_for_api),
                "requested_by": &actor,
            }),
        )
        .await
        .map_err(atelier_error)?;

    if let Some(desired) = desired_tags {
        let existing_tag_rows = sqlx::query(
            r#"SELECT mat.tag_id, t.text
               FROM atelier_media_asset_tag mat
               JOIN atelier_tag t ON t.tag_id = mat.tag_id
               WHERE mat.asset_id = $1
               FOR UPDATE OF mat"#,
        )
        .bind(asset_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|err| atelier_error(AtelierError::Database(err)))?;
        for row in existing_tag_rows {
            let tag_id: Uuid = row.get("tag_id");
            let tag_text: String = row.get("text");
            if desired.iter().any(|value| value == &tag_text) {
                continue;
            }
            sqlx::query("DELETE FROM atelier_media_asset_tag WHERE asset_id = $1 AND tag_id = $2")
                .bind(asset_id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| atelier_error(AtelierError::Database(err)))?;
            store
                .record_event_in_tx(
                    &mut tx,
                    crate::atelier::collections::collections_event_family::MEDIA_ASSET_UNTAGGED,
                    "atelier_media_asset_tag",
                    &event_ref_for_text(&format!("media-asset-untag:{}:{}", asset_id, tag_text)),
                    serde_json::json!({
                        "asset_id": asset_id,
                        "text": tag_text,
                    }),
                )
                .await
                .map_err(atelier_error)?;
        }
        for tag_text in &desired {
            let tag_row = sqlx::query(
                r#"INSERT INTO atelier_tag (text)
                   VALUES ($1)
                   ON CONFLICT (text) DO UPDATE SET text = EXCLUDED.text
                   RETURNING tag_id, text"#,
            )
            .bind(tag_text)
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| atelier_error(AtelierError::Database(err)))?;
            let tag_id: Uuid = tag_row.get("tag_id");
            let persisted_text: String = tag_row.get("text");
            sqlx::query(
                r#"INSERT INTO atelier_media_asset_tag (asset_id, tag_id, source)
                   VALUES ($1, $2, $3)
                   ON CONFLICT (asset_id, tag_id)
                     DO UPDATE SET source = EXCLUDED.source"#,
            )
            .bind(asset_id)
            .bind(tag_id)
            .bind(&actor)
            .execute(&mut *tx)
            .await
            .map_err(|err| atelier_error(AtelierError::Database(err)))?;
            store
                .record_event_in_tx(
                    &mut tx,
                    crate::atelier::collections::collections_event_family::MEDIA_ASSET_TAGGED,
                    "atelier_media_asset_tag",
                    &event_ref_for_text(&format!("media-asset-tag:{}:{}", asset_id, tag_id)),
                    serde_json::json!({
                        "asset_id": asset_id,
                        "tag_id": tag_id,
                        "text": persisted_text,
                        "tag_source_ref": event_ref_for_text(&actor),
                    }),
                )
                .await
                .map_err(atelier_error)?;
        }
    }

    let existing_refs = sqlx::query(
        r#"SELECT asset_id, source_url_ref, source_path_ref, source_note_ref,
                  contact_sheet_ref, task_ref, run_ref, updated_by, updated_at_utc
           FROM atelier_media_source_provenance_ref
           WHERE asset_id = $1
           FOR UPDATE"#,
    )
    .bind(asset_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| atelier_error(AtelierError::Database(err)))?;
    let mut final_source_url_ref: Option<String> = existing_refs
        .as_ref()
        .and_then(|row| row.get("source_url_ref"));
    let mut final_source_path_ref: Option<String> = existing_refs
        .as_ref()
        .and_then(|row| row.get("source_path_ref"));
    let source_note_ref: Option<String> = existing_refs
        .as_ref()
        .and_then(|row| row.get("source_note_ref"));
    let contact_sheet_ref: Option<String> = existing_refs
        .as_ref()
        .and_then(|row| row.get("contact_sheet_ref"));
    let task_ref: Option<String> = existing_refs.as_ref().and_then(|row| row.get("task_ref"));
    let run_ref: Option<String> = existing_refs.as_ref().and_then(|row| row.get("run_ref"));
    if source_url_ref.is_some() || source_path_ref.is_some() {
        if source_url_ref.is_some() {
            final_source_url_ref = source_url_ref;
        }
        if source_path_ref.is_some() {
            final_source_path_ref = source_path_ref;
        }
        validate_optional_provenance_ref_for_api(
            "source_url_ref",
            final_source_url_ref.as_deref(),
        )?;
        validate_optional_provenance_ref_for_api(
            "source_path_ref",
            final_source_path_ref.as_deref(),
        )?;
        validate_optional_provenance_ref_for_api("source_note_ref", source_note_ref.as_deref())?;
        validate_optional_provenance_ref_for_api(
            "contact_sheet_ref",
            contact_sheet_ref.as_deref(),
        )?;
        validate_optional_provenance_ref_for_api("task_ref", task_ref.as_deref())?;
        validate_optional_provenance_ref_for_api("run_ref", run_ref.as_deref())?;
        sqlx::query(
            r#"INSERT INTO atelier_media_source_provenance_ref
                 (asset_id, source_url_ref, source_path_ref, source_note_ref,
                  contact_sheet_ref, task_ref, run_ref, updated_by, updated_at_utc)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
               ON CONFLICT (asset_id)
               DO UPDATE SET
                   source_url_ref = EXCLUDED.source_url_ref,
                   source_path_ref = EXCLUDED.source_path_ref,
                   source_note_ref = EXCLUDED.source_note_ref,
                   contact_sheet_ref = EXCLUDED.contact_sheet_ref,
                   task_ref = EXCLUDED.task_ref,
                   run_ref = EXCLUDED.run_ref,
                   updated_by = EXCLUDED.updated_by,
                   updated_at_utc = NOW()"#,
        )
        .bind(asset_id)
        .bind(&final_source_url_ref)
        .bind(&final_source_path_ref)
        .bind(&source_note_ref)
        .bind(&contact_sheet_ref)
        .bind(&task_ref)
        .bind(&run_ref)
        .bind(&actor)
        .execute(&mut *tx)
        .await
        .map_err(|err| atelier_error(AtelierError::Database(err)))?;
        store
            .record_event_in_tx(
                &mut tx,
                crate::atelier::event_family::MEDIA_SOURCE_PROVENANCE_REFS_SET,
                "atelier_media_asset",
                &asset_id.to_string(),
                serde_json::json!({
                    "asset_id": asset_id,
                    "source_url_ref": &final_source_url_ref,
                    "source_path_ref": &final_source_path_ref,
                    "source_note_ref": &source_note_ref,
                    "contact_sheet_ref": &contact_sheet_ref,
                    "task_ref": &task_ref,
                    "run_ref": &run_ref,
                    "updated_by": &actor,
                }),
            )
            .await
            .map_err(atelier_error)?;
    }

    let tag_rows = sqlx::query(
        r#"SELECT t.text
           FROM atelier_media_asset_tag mat
           JOIN atelier_tag t ON t.tag_id = mat.tag_id
           WHERE mat.asset_id = $1
           ORDER BY t.text ASC"#,
    )
    .bind(asset_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|err| atelier_error(AtelierError::Database(err)))?;
    let tags = tag_rows.iter().map(|row| row.get("text")).collect();
    tx.commit()
        .await
        .map_err(|err| atelier_error(AtelierError::Database(err)))?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/media-assets/:asset_id/notes-tags",
        status = "ok",
        actor = %actor,
        asset_id = %asset_id,
        "update CKC media notes/tags"
    );
    Ok(Json(MediaNotesTagsResponse {
        asset_id,
        media_ref: media_asset_ref(asset_id),
        notes,
        review_status,
        tags,
        source_path_ref: final_source_path_ref,
        source_url_ref: final_source_url_ref,
    }))
}

fn parse_ckc_search_modes(
    raw_modes: Option<Vec<String>>,
) -> Result<Vec<CkcSearchMode>, (StatusCode, Json<ErrorResponse>)> {
    let Some(raw_modes) = raw_modes else {
        return Ok(Vec::new());
    };
    let mut modes = Vec::new();
    for raw in raw_modes {
        let Some(mode) = CkcSearchMode::parse(&raw) else {
            return Err(atelier_error(AtelierError::Validation(format!(
                "unknown CKC search mode: {raw}"
            ))));
        };
        if !modes.contains(&mode) {
            modes.push(mode);
        }
    }
    Ok(modes)
}

/// POST /atelier/ckc/search — fuzzy/vector/combined CKC search over characters, sheets, albums, media, tags, and tag notes.
async fn search_ckc(
    State(state): State<AppState>,
    Json(payload): Json<CkcSearchApiRequest>,
) -> Result<Json<CkcSearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let request = CkcSearchRequest {
        query: payload.query.unwrap_or_default(),
        modes: parse_ckc_search_modes(payload.modes)?,
        tags: payload.tags.unwrap_or_default(),
        character_internal_id: payload.character_internal_id,
        collection_id: payload.collection_id,
        media_asset_id: payload.media_asset_id,
        similar_to_asset_id: payload.similar_to_asset_id,
        similar_to_dhash_hex: payload.similar_to_dhash_hex,
        limit: payload.limit.unwrap_or(25),
    };
    let response = store
        .ckc_search(request, Some(state.llm_client.as_ref()))
        .await
        .map_err(atelier_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ckc/search",
        status = "ok",
        result_count = response.result_count,
        semantic_available = response.semantic_available,
        "search CKC"
    );
    Ok(Json(response))
}

/// POST /atelier/ckc/tag-notes — rich tag note round-trip, separate from sheet/media/album notes.
async fn upsert_ckc_tag_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CkcTagNoteRequest>,
) -> Result<Json<CkcTagNote>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let tag_note = store
        .upsert_ckc_tag_note(&UpsertCkcTagNote {
            tag_text: payload.tag_text,
            scope_ref: payload.scope_ref,
            note: payload.note,
            updated_by: actor.clone(),
        })
        .await
        .map_err(atelier_error)?;
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/ckc/tag-notes",
        status = "ok",
        actor = %actor,
        tag_note_id = %tag_note.tag_note_id,
        tag_text = %tag_note.tag_text,
        "upsert CKC tag note"
    );
    Ok(Json(tag_note))
}

/// POST /atelier/posekit/openpose-export — native Rust Posekit/OpenRepose export into ArtifactStore.
async fn export_posekit_openpose(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PosekitOpenPoseExportRequest>,
) -> Result<Json<PosekitOpenPoseExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let rig = if let Some(rig_id) = payload.rig_id {
        let rig = store.get_pose_rig(rig_id).await.map_err(atelier_error)?;
        if rig.source_ref != payload.source_ref {
            return Err(atelier_error(AtelierError::Validation(format!(
                "Posekit OpenPose export source_ref must match rig source_ref: request={} rig={}",
                payload.source_ref, rig.source_ref
            ))));
        }
        Some(rig)
    } else {
        None
    };
    let export = if let Some(rig) = rig.as_ref() {
        generate_posekit_openpose_export_from_keypoints(&payload, &rig.keypoints_json)
    } else {
        generate_posekit_openpose_export(&payload)
    }
    .map_err(atelier_error)?;
    let png_artifact = write_posekit_export_artifact(
        &export,
        &export.openpose_png_bytes,
        &export.openpose_png_sha256,
        "image/png",
        "posekit-openpose.png",
        &actor,
        payload.rig_id,
        &[],
    )?;
    let json_artifact = write_posekit_export_artifact(
        &export,
        &export.openpose_json_bytes,
        &export.openpose_json_sha256,
        "application/json",
        "posekit-openpose.json",
        &actor,
        payload.rig_id,
        &[],
    )?;
    let receipt_artifact = write_posekit_export_receipt_artifact(
        &export,
        &png_artifact,
        &json_artifact,
        &actor,
        payload.rig_id,
    )?;
    let sidecars = if let Some(rig_id) = payload.rig_id {
        let sidecars = store
            .record_pose_sidecars(&[
                NewPoseSidecar {
                    rig_id,
                    kind: PoseSidecarKind::OpenPoseJson,
                    artifact_ref: json_artifact.artifact_ref.clone(),
                    manifest_ref: json_artifact.manifest_ref.clone(),
                    content_hash: json_artifact.content_hash.clone(),
                    byte_len: json_artifact.byte_len as i64,
                    mime: json_artifact.mime.clone(),
                    width: export.width,
                    height: export.height,
                    status: PoseSidecarStatus::Rendered,
                    error_message: None,
                },
                NewPoseSidecar {
                    rig_id,
                    kind: PoseSidecarKind::OpenPosePng,
                    artifact_ref: png_artifact.artifact_ref.clone(),
                    manifest_ref: png_artifact.manifest_ref.clone(),
                    content_hash: png_artifact.content_hash.clone(),
                    byte_len: png_artifact.byte_len as i64,
                    mime: png_artifact.mime.clone(),
                    width: export.width,
                    height: export.height,
                    status: PoseSidecarStatus::Rendered,
                    error_message: None,
                },
            ])
            .await
            .map_err(atelier_error)?;
        sidecars.into_iter().map(posekit_sidecar_response).collect()
    } else {
        Vec::new()
    };
    let response = PosekitOpenPoseExportResponse {
        schema_id: POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID.to_owned(),
        source_ref: export.source_ref.clone(),
        rig_id: payload.rig_id,
        yaw_deg: export.yaw_deg,
        pitch_deg: export.pitch_deg,
        zoom_percent: export.zoom_percent,
        framing: export.framing,
        marker_layers: export.marker_layers.clone(),
        applied_marker_edit_count: export.applied_marker_edit_count,
        width: export.width,
        height: export.height,
        openpose_json: export.openpose_json.clone(),
        openpose_json_sha256: export.openpose_json_sha256.clone(),
        openpose_png_sha256: export.openpose_png_sha256.clone(),
        content_hash: export.content_hash.clone(),
        receipt_ref: receipt_artifact.artifact_ref.clone(),
        openpose_png_artifact: png_artifact,
        openpose_json_artifact: json_artifact,
        sidecars,
    };
    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/posekit/openpose-export",
        status = "ok",
        actor = %actor,
        source_ref = %response.source_ref,
        yaw_deg = response.yaw_deg,
        png_artifact_ref = %response.openpose_png_artifact.artifact_ref,
        json_artifact_ref = %response.openpose_json_artifact.artifact_ref,
        sidecar_count = response.sidecars.len(),
        receipt_ref = %response.receipt_ref,
        "export Posekit OpenPose"
    );
    Ok(Json(response))
}

fn posekit_sidecar_response(sidecar: PoseSidecar) -> PosekitSidecarResponse {
    PosekitSidecarResponse {
        sidecar_id: sidecar.sidecar_id,
        rig_id: sidecar.rig_id,
        kind: sidecar.kind.as_token().to_owned(),
        artifact_ref: sidecar.artifact_ref,
        manifest_ref: sidecar.manifest_ref,
        content_hash: sidecar.content_hash,
    }
}

fn write_posekit_export_artifact(
    export: &PosekitOpenPoseExport,
    payload_bytes: &[u8],
    content_hash: &str,
    mime: &str,
    file_name: &str,
    actor: &str,
    rig_id: Option<Uuid>,
    source_artifact_refs: &[ArtifactHandle],
) -> Result<PosekitArtifactResponse, (StatusCode, Json<ErrorResponse>)> {
    let workspace_root = resolve_workspace_root().map_err(internal_error)?;
    let artifact_id = Uuid::now_v7();
    let manifest = ArtifactManifest {
        artifact_id,
        layer: ArtifactLayer::L1,
        kind: ArtifactPayloadKind::File,
        mime: mime.to_owned(),
        filename_hint: Some(file_name.to_owned()),
        created_at: Utc::now(),
        created_by_job_id: None,
        source_entity_refs: posekit_export_source_entity_refs(export, actor, rig_id),
        source_artifact_refs: source_artifact_refs.to_vec(),
        content_hash: content_hash.to_owned(),
        size_bytes: payload_bytes.len() as u64,
        classification: ArtifactClassification::Low,
        exportable: true,
        retention_ttl_days: None,
        pinned: Some(true),
        hash_basis: Some(format!(
            "{}|{}|yaw={}|pitch={}|zoom={}|{}",
            POSEKIT_OPENPOSE_EXPORT_SCHEMA_ID,
            export.source_ref,
            export.yaw_deg,
            export.pitch_deg,
            export.zoom_percent,
            export.content_hash
        )),
        hash_exclude_paths: Vec::new(),
    };
    write_file_artifact(&workspace_root, &manifest, payload_bytes).map_err(internal_error)?;
    validate_artifact_content_hash(&workspace_root, ArtifactLayer::L1, artifact_id)
        .map_err(internal_error)?;
    let root = artifact_root_rel(ArtifactLayer::L1, artifact_id);
    Ok(PosekitArtifactResponse {
        artifact_ref: format!("artifact://{root}/payload"),
        manifest_ref: format!("artifact://{root}/artifact.json"),
        content_hash: content_hash.to_owned(),
        byte_len: payload_bytes.len() as u64,
        mime: mime.to_owned(),
        file_name: file_name.to_owned(),
    })
}

fn write_posekit_export_receipt_artifact(
    export: &PosekitOpenPoseExport,
    png_artifact: &PosekitArtifactResponse,
    json_artifact: &PosekitArtifactResponse,
    actor: &str,
    rig_id: Option<Uuid>,
) -> Result<PosekitArtifactResponse, (StatusCode, Json<ErrorResponse>)> {
    let receipt = serde_json::json!({
        "schema_id": "hsk.atelier.posekit.openpose_export_receipt@1",
        "export_schema_id": export.schema_id.clone(),
        "source_ref": export.source_ref.clone(),
        "rig_id": rig_id,
        "actor_ref": format!("actor://sha256/{}", text_hash(actor)),
        "yaw_deg": export.yaw_deg,
        "pitch_deg": export.pitch_deg,
        "zoom_percent": export.zoom_percent,
        "framing": export.framing,
        "marker_layers": export.marker_layers.clone(),
        "applied_marker_edit_count": export.applied_marker_edit_count,
        "width": export.width,
        "height": export.height,
        "content_hash": export.content_hash.clone(),
        "openpose_png_artifact_ref": png_artifact.artifact_ref.clone(),
        "openpose_png_manifest_ref": png_artifact.manifest_ref.clone(),
        "openpose_png_sha256": export.openpose_png_sha256.clone(),
        "openpose_json_artifact_ref": json_artifact.artifact_ref.clone(),
        "openpose_json_manifest_ref": json_artifact.manifest_ref.clone(),
        "openpose_json_sha256": export.openpose_json_sha256.clone(),
    });
    let payload_bytes = serde_json::to_vec(&receipt).map_err(internal_error)?;
    let content_hash = text_hash(
        std::str::from_utf8(&payload_bytes).map_err(|err| internal_error(err.to_string()))?,
    );
    write_posekit_export_artifact(
        export,
        &payload_bytes,
        &content_hash,
        "application/json",
        "posekit-openpose-export-receipt.json",
        actor,
        rig_id,
        &[
            posekit_artifact_handle(png_artifact),
            posekit_artifact_handle(json_artifact),
        ],
    )
}

fn posekit_export_source_entity_refs(
    export: &PosekitOpenPoseExport,
    actor: &str,
    rig_id: Option<Uuid>,
) -> Vec<EntityRef> {
    let mut refs = vec![
        EntityRef {
            entity_kind: "posekit_source_ref".to_owned(),
            entity_id: export.source_ref.clone(),
        },
        EntityRef {
            entity_kind: "actor_sha256".to_owned(),
            entity_id: text_hash(actor),
        },
        EntityRef {
            entity_kind: "posekit_openpose_export".to_owned(),
            entity_id: export.content_hash.clone(),
        },
    ];
    if let Some(rig_id) = rig_id {
        refs.push(EntityRef {
            entity_kind: "pose_rig".to_owned(),
            entity_id: rig_id.to_string(),
        });
    }
    refs
}

fn posekit_artifact_handle(artifact: &PosekitArtifactResponse) -> ArtifactHandle {
    let artifact_id = artifact
        .artifact_ref
        .strip_prefix("artifact://.handshake/artifacts/L1/")
        .and_then(|value| value.strip_suffix("/payload"))
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::now_v7);
    ArtifactHandle::new(artifact_id, artifact.artifact_ref.clone())
}

/// POST /atelier/characters/:character_internal_id/sheet-versions — append a guarded sheet edit.
async fn append_sheet_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Json(payload): Json<AppendSheetVersionRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    append_sheet_version_for_character(
        &state,
        &headers,
        character_internal_id,
        payload,
        "/atelier/characters/:character_internal_id/sheet-versions",
    )
    .await
}

/// POST /atelier/characters/:character_internal_id/sheet-versions/import — import raw sheet text.
async fn import_sheet_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Json(payload): Json<AppendSheetVersionRequest>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    append_sheet_version_for_character(
        &state,
        &headers,
        character_internal_id,
        payload,
        "/atelier/characters/:character_internal_id/sheet-versions/import",
    )
    .await
}

async fn append_sheet_version_for_character(
    state: &AppState,
    headers: &HeaderMap,
    character_internal_id: Uuid,
    payload: AppendSheetVersionRequest,
    route: &'static str,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(headers)?;
    let store = atelier_store(state);
    let character = store
        .get_character_by_internal_id(character_internal_id)
        .await
        .map_err(atelier_error)?;
    let raw_text = if route.ends_with("/import") {
        let raw_text = import_sheet_raw_text(&payload.raw_text)?;
        validate_ckc_sheet_owner(&character, &raw_text, true)?;
        raw_text
    } else {
        validate_ckc_sheet_owner(&character, &payload.raw_text, true)?;
        payload.raw_text
    };
    let expected_parent_version_id = payload.expected_parent_version_id;
    let version = match store
        .append_sheet_version_if_current(
            &NewSheetVersion {
                character_internal_id,
                raw_text,
                author: actor.clone(),
                tool: payload.tool,
            },
            expected_parent_version_id,
        )
        .await
    {
        Ok(version) => version,
        Err(AtelierError::Conflict(detail)) => {
            tracing::warn!(
                target: "handshake_core::atelier",
                %detail,
                character_internal_id = %character_internal_id,
                "stale CKC sheet version write"
            );
            let current = store
                .latest_sheet_version(character_internal_id)
                .await
                .map_err(atelier_error)?;
            let response = sheet_version_conflict_response(
                character_internal_id,
                expected_parent_version_id,
                current,
            );
            return Ok((StatusCode::CONFLICT, Json(response)).into_response());
        }
        Err(err) => return Err(atelier_error(err)),
    };
    tracing::info!(
        target: "handshake_core::atelier",
        route = route,
        status = "created",
        actor = %actor,
        character_internal_id = %character_internal_id,
        version_id = %version.version_id,
        seq = version.seq,
        "append CKC sheet version"
    );
    Ok((StatusCode::CREATED, Json(sheet_version_response(version))).into_response())
}

/// GET /atelier/sheet-versions/:version_id — read one stable CKC sheet/version ref.
async fn get_sheet_version(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
) -> Result<Json<SheetVersionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let version = store
        .get_sheet_version(version_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(sheet_version_response(version)))
}

/// GET /atelier/sheet-versions/:version_id/export?format=txt|json — deterministic CKC sheet export.
async fn export_sheet_version(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
    Query(query): Query<SheetVersionExportQuery>,
) -> Result<Json<SheetVersionExportResponse>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    let version = store
        .get_sheet_version(version_id)
        .await
        .map_err(atelier_error)?;
    let format = query
        .format
        .as_deref()
        .unwrap_or("txt")
        .trim()
        .to_ascii_lowercase();
    let (format_label, file_ext, content) = match format.as_str() {
        "txt" | "text" => ("txt", "txt", version.raw_text.clone()),
        "json" => (
            "json",
            "json",
            export_sheet_json(&version, &version.raw_text, "ckc-sheet-export.v1")?,
        ),
        "safe-txt" | "safe_text" | "safe_txt" => {
            let safe_text = safe_subset_sheet_text(&version.raw_text)?;
            ("safe-txt", "safe.txt", safe_text)
        }
        "safe-json" | "safe_json" => {
            let safe_text = safe_subset_sheet_text(&version.raw_text)?;
            (
                "safe-json",
                "safe.json",
                export_sheet_json(&version, &safe_text, "ckc-sheet-safe-export.v1")?,
            )
        }
        _ => {
            return Err(atelier_error(AtelierError::Validation(format!(
                "unsupported CKC sheet export format={format}"
            ))));
        }
    };
    let content_hash = text_hash(&content);
    Ok(Json(SheetVersionExportResponse {
        version_id: version.version_id,
        character_internal_id: version.character_internal_id,
        format: format_label.to_owned(),
        file_name: format!("ckc-sheet-{}.{}", version.version_id, file_ext),
        content_hash,
        content,
        character_ref: character_ref(version.character_internal_id),
        sheet_version_ref: sheet_version_ref(version.character_internal_id, version.version_id),
    }))
}

/// GET /atelier/sheet-templates/default — bundled CKC v2.00 template metadata + raw text.
async fn get_default_sheet_template(
) -> Result<Json<crate::atelier::BuiltInSheetTemplate>, (StatusCode, Json<ErrorResponse>)> {
    builtin_character_sheet_template()
        .map(Json)
        .map_err(atelier_error)
}

/// GET /atelier/sheet-templates/default/safe-subset — original LLM-safe v2.00 field whitelist.
async fn get_default_sheet_template_safe_subset(
) -> Result<Json<crate::atelier::BuiltInSafeSubset>, (StatusCode, Json<ErrorResponse>)> {
    builtin_safe_subset().map(Json).map_err(atelier_error)
}

/// GET /atelier/sheet-field-suggestions?field_id=... — prior values for one CKC Field ID.
async fn list_sheet_field_suggestions(
    State(state): State<AppState>,
    Query(query): Query<SheetFieldSuggestionsQuery>,
) -> Result<Json<Vec<crate::atelier::SheetFieldSuggestion>>, (StatusCode, Json<ErrorResponse>)> {
    let store = atelier_store(&state);
    store
        .sheet_field_suggestions(&query.field_id, query.limit.unwrap_or(20))
        .await
        .map(Json)
        .map_err(atelier_error)
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
    if let Some(target_character_id) = payload.target_character_id {
        ensure_sheet_version_matches_character(
            &store,
            target_character_id,
            payload.target_sheet_version_id,
        )
        .await?;
    }
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
}

fn intake_item_response(item: crate::atelier::intake::IntakeItem) -> IntakeItemResponse {
    IntakeItemResponse {
        item_id: item.item_id,
        source_path: item.source_path,
        file_name: item.file_name,
        lane: item.lane.as_str().to_owned(),
        byte_len: item.byte_len,
    }
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
    let store = atelier_store(&state);

    let lane_counts = store
        .intake_lane_counts(batch_id)
        .await
        .map_err(atelier_error)?;

    // The typed `list_intake_items` is uncapped; a batch can hold tens of
    // thousands of items, so use a direct capped query (LIST_CAP) like the
    // other list routes. The lane_counts header carries the true totals.
    let rows = sqlx::query(
        r#"SELECT item_id, source_path, file_name, lane, byte_len
           FROM atelier_intake_item
           WHERE batch_id = $1
           ORDER BY created_at_utc ASC
           LIMIT $2"#,
    )
    .bind(batch_id)
    .bind(LIST_CAP)
    .fetch_all(&state.postgres_pool)
    .await
    .map_err(internal_error)?;

    let items = rows
        .iter()
        .map(|row| IntakeItemResponse {
            item_id: row.get("item_id"),
            source_path: row.get("source_path"),
            file_name: row.get("file_name"),
            lane: row.get("lane"),
            byte_len: row.get("byte_len"),
        })
        .collect();

    tracing::info!(target: "handshake_core::atelier", route = "/atelier/intake/batches/:batch_id/items", status = "ok", batch_id = %batch_id, "list intake batch items");

    Ok(Json(IntakeBatchItemsResponse {
        lane_counts: lane_counts.into(),
        items,
    }))
}

#[derive(Debug, Deserialize)]
struct ApplyIntakeItemClassificationRequest {
    lane: String,
    reason: Option<String>,
    metadata: Option<IntakeClassificationMetadata>,
}

#[derive(Debug, Serialize)]
struct IntakeClassificationApplyResponse {
    item: IntakeItemResponse,
    asset_id: Option<Uuid>,
    media_ref: Option<String>,
    collection_id: Option<Uuid>,
    collection_ref: Option<String>,
    collection_inserted: bool,
    requested_by: String,
}

/// POST /atelier/intake/items/:item_id/classification — persist one item triage decision.
async fn apply_intake_item_classification(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
    Json(payload): Json<ApplyIntakeItemClassificationRequest>,
) -> Result<Json<IntakeClassificationApplyResponse>, (StatusCode, Json<ErrorResponse>)> {
    let actor = calling_actor(&headers)?;
    let lane = IntakeLane::parse(&payload.lane).map_err(atelier_error)?;
    let store = atelier_store(&state);
    let applied = store
        .apply_intake_classification(&ApplyIntakeClassificationRequest {
            item_id,
            lane,
            reason: payload.reason,
            requested_by: Some(actor.clone()),
            metadata: payload.metadata,
        })
        .await
        .map_err(atelier_error)?;

    tracing::info!(
        target: "handshake_core::atelier",
        route = "/atelier/intake/items/:item_id/classification",
        status = "ok",
        actor = %actor,
        item_id = %applied.item.item_id,
        lane = applied.item.lane.as_str(),
        "apply intake item classification"
    );

    Ok(Json(IntakeClassificationApplyResponse {
        item: intake_item_response(applied.item),
        asset_id: applied.asset_id,
        media_ref: applied.asset_id.map(media_asset_ref),
        collection_id: applied.collection_id,
        collection_ref: applied.collection_id.map(collection_ref),
        collection_inserted: applied.collection_inserted,
        requested_by: actor,
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
/// PostgreSQL and never includes raw payload fields.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_review_status_normalizer_accepts_operator_aliases() {
        let cases = [
            (None, "unreviewed"),
            (Some("pass"), "approved"),
            (Some("approved"), "approved"),
            (Some("reject"), "rejected"),
            (Some("unsure"), "deferred"),
            (Some("review"), "review"),
        ];

        for (input, expected) in cases {
            assert_eq!(
                normalize_media_review_status_for_api(input).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn media_review_status_normalizer_rejects_unknown_status_before_storage() {
        let err = normalize_media_review_status_for_api(Some("ship-it")).unwrap_err();

        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1 .0.error, "bad_request");
    }

    #[test]
    fn media_notes_ref_uses_store_hash_prefix_contract() {
        let notes_ref = media_notes_ref_for_api("same note");

        assert!(notes_ref.starts_with("sha256:"));
        assert_eq!(
            notes_ref,
            format!("sha256:{}", crate::atelier::text_hash("same note"))
        );
    }
}
