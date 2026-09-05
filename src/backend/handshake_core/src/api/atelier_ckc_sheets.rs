//! WP-CKC-posekit-overhaul SurrealDB port — CKC `sheets` lane router.
//!
//! Characters, append-only sheet versions (guarded by `expected_parent_version_id`),
//! bundled v2.00 template import/export, sheet-field suggestions, reusable sheet artifact
//! links, character documents (story/moodboard/note) with story cards and beats, and native
//! moodboard snapshots. Route paths, JSON shapes and error codes follow the reference branch
//! (`api/atelier.rs` on `feat/WP-CKC-posekit-overhaul`); storage authority is the embedded
//! SurrealDB store through `AtelierStore`, no relational fallback exists.
//!
//! Shared helpers come from `super::atelier` (`atelier_store`, `atelier_error`, `internal_error`,
//! `calling_actor`, `header_str`, `ErrorResponse`, `LIST_CAP`). The model-operation lease guard
//! the reference kept in the shared router lives here for this lane: every guarded mutation
//! requires either an active `atelier_model_coordination_lease` claim (`x-hsk-model-lease-id` +
//! `x-hsk-session-id`) bound to the mutation's coordination thread, or the explicit operator
//! declaration `x-hsk-actor-kind: operator` with `x-hsk-actor-id: operator`.

use std::collections::HashSet;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::atelier::{
    atelier_error as shared_atelier_error, atelier_store, calling_actor, header_str,
    internal_error, ErrorResponse, HSK_HEADER_ACTOR_ID, LIST_CAP,
};
use crate::atelier::core::{Character, NewCharacter};
use crate::atelier::documents::{
    AppendCharacterDocumentVersion, CharacterDocument, CharacterDocumentType,
    CharacterDocumentVersion, NewCharacterDocument, NewStoryBeat, NewStoryCard, StoryBeat,
    StoryCard,
};
use crate::atelier::model_lease::ModelLeaseRecord;
use crate::atelier::moodboards::{MoodboardDocument, MoodboardSnapshot, NewMoodboardSnapshot};
use crate::atelier::refs::{character_ref, sheet_version_ref};
use crate::atelier::sheet::{
    sheet_field_id_from_line, sheet_field_values, sheet_line_looks_like_field, NewSheetVersion,
    SheetFieldSuggestion, SheetVersion,
};
use crate::atelier::sheet_artifacts::{NewSheetArtifactLink, SheetArtifactKind, SheetArtifactLink};
use crate::atelier::sheet_templates::{
    builtin_character_sheet_template, builtin_safe_subset, default_character_sheet_text,
    text_hash, BuiltInSafeSubset, BuiltInSheetTemplate, CHARACTER_SHEET_V2_TEMPLATE_VERSION,
    DEFAULT_SHEET_TOOL,
};
use crate::atelier::AtelierError;
use crate::kernel::role_mailbox_claim_lease::{ClaimLeaseState, RoleMailboxClaimMode};
use crate::AppState;

/// Declares the caller's actor kind. `operator` (together with `x-hsk-actor-id: operator`)
/// is the only value that exempts a guarded mutation from the lease requirement.
pub(crate) const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
/// Claim id of an active model-operation lease (`atelier_model_coordination_lease`).
pub(crate) const HSK_HEADER_MODEL_LEASE_ID: &str = "x-hsk-model-lease-id";
/// Session id the lease was claimed under; must accompany the lease id.
pub(crate) const HSK_HEADER_SESSION_ID: &str = "x-hsk-session-id";

pub fn routes(state: AppState) -> Router {
    Router::new()
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
            "/atelier/characters/:character_internal_id/sheet-versions/import",
            post(import_sheet_version),
        )
        .route(
            "/atelier/characters/:character_internal_id/documents",
            get(list_character_documents).post(create_character_document),
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
            "/atelier/sheet-versions/:version_id",
            get(get_sheet_version),
        )
        .route(
            "/atelier/sheet-versions/:version_id/artifact-links",
            get(list_sheet_artifact_links).post(attach_sheet_artifact_link),
        )
        .route(
            "/atelier/sheet-artifact-links/:link_id",
            get(get_sheet_artifact_link).delete(detach_sheet_artifact_link),
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
        .with_state(state)
}

type ApiError = (StatusCode, Json<ErrorResponse>);

/// The reference branch answered [`AtelierError::ForbiddenStorage`] (a caller pointing at
/// SQLite/Electron/CKC/localhost/machine-local storage) with `400 bad_request`; the shared
/// `super::atelier::atelier_error` on this tree falls through to `500 db_error` for it. This lane
/// keeps the reference contract for its own routes and delegates everything else unchanged.
fn atelier_error(err: AtelierError) -> ApiError {
    match err {
        AtelierError::ForbiddenStorage(detail) => {
            tracing::warn!(target: "handshake_core::atelier", %detail, "bad_request");
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "bad_request",
                }),
            )
        }
        other => shared_atelier_error(other),
    }
}

// ---------------------------------------------------------------------------------------------
// Request / response shapes (reference `api/atelier.rs` names and fields, unchanged).
// ---------------------------------------------------------------------------------------------

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
struct AttachSheetArtifactLinkRequest {
    artifact_kind: String,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct SheetArtifactLinkResponse {
    link_id: Uuid,
    character_internal_id: Uuid,
    character_ref: String,
    sheet_version_id: Uuid,
    sheet_version_ref: String,
    typed_ref: String,
    artifact_kind: SheetArtifactKind,
    artifact_ref: String,
    manifest_ref: Option<String>,
    source_ref: Option<String>,
    label: Option<String>,
    reuse_role: Option<String>,
    linked_by: String,
    metadata: serde_json::Value,
    created_at_utc: DateTime<Utc>,
    detached_at_utc: Option<DateTime<Utc>>,
    detached_by: Option<String>,
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
    moodboard: MoodboardDocument,
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

// ---------------------------------------------------------------------------------------------
// Ref builders and response mappers.
// ---------------------------------------------------------------------------------------------

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

fn sheet_artifact_link_response(link: SheetArtifactLink) -> SheetArtifactLinkResponse {
    SheetArtifactLinkResponse {
        link_id: link.link_id,
        character_internal_id: link.character_internal_id,
        character_ref: character_ref(link.character_internal_id),
        sheet_version_id: link.sheet_version_id,
        sheet_version_ref: link.sheet_version_ref,
        typed_ref: link.typed_ref,
        artifact_kind: link.artifact_kind,
        artifact_ref: link.artifact_ref,
        manifest_ref: link.manifest_ref,
        source_ref: link.source_ref,
        label: link.label,
        reuse_role: link.reuse_role,
        linked_by: link.linked_by,
        metadata: link.metadata,
        created_at_utc: link.created_at_utc,
        detached_at_utc: link.detached_at_utc,
        detached_by: link.detached_by,
    }
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
    store: &crate::atelier::AtelierStore,
    document: CharacterDocument,
) -> Result<CharacterDocumentResponse, ApiError> {
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

// ---------------------------------------------------------------------------------------------
// Model-operation lease guard (reference `validate_model_operation_lease_if_present`).
// ---------------------------------------------------------------------------------------------

/// Guard a model-operation mutation. With `x-hsk-model-lease-id` present the lease must be
/// active, held by this actor+session, mutating (`exclusive_lease` / `handoff_reservation`) and
/// bound to `expected_thread_id`. Without it, only the explicit operator declaration
/// (`x-hsk-actor-kind: operator` for `x-hsk-actor-id: operator`) passes; every other caller is
/// refused with `bad_request`.
async fn validate_model_operation_lease_if_present(
    state: &AppState,
    headers: &HeaderMap,
    actor: &str,
    expected_thread_id: Option<&str>,
) -> Result<Option<ModelLeaseRecord>, ApiError> {
    if header_str(headers, HSK_HEADER_MODEL_LEASE_ID).is_some() {
        return validate_model_operation_lease_required(state, headers, actor, expected_thread_id)
            .await
            .map(Some);
    }
    match header_str(headers, HSK_HEADER_ACTOR_KIND) {
        Some("operator") if actor == "operator" => Ok(None),
        Some("operator") => Err(atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_ACTOR_KIND}=operator is reserved for {HSK_HEADER_ACTOR_ID}=operator"
        )))),
        Some(other) => Err(atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation guarded mutations unless {HSK_HEADER_ACTOR_KIND}=operator; got {other}"
        )))),
        None => Err(atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation guarded mutations unless {HSK_HEADER_ACTOR_KIND}=operator"
        )))),
    }
}

async fn validate_model_operation_lease_required(
    state: &AppState,
    headers: &HeaderMap,
    actor: &str,
    expected_thread_id: Option<&str>,
) -> Result<ModelLeaseRecord, ApiError> {
    let raw_claim_id = header_str(headers, HSK_HEADER_MODEL_LEASE_ID).ok_or_else(|| {
        atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} is required for model-operation mutations"
        )))
    })?;
    let session_id = header_str(headers, HSK_HEADER_SESSION_ID).ok_or_else(|| {
        atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_SESSION_ID} is required with {HSK_HEADER_MODEL_LEASE_ID}"
        )))
    })?;
    let claim_id = Uuid::parse_str(raw_claim_id).map_err(|_| {
        atelier_error(AtelierError::Validation(format!(
            "{HSK_HEADER_MODEL_LEASE_ID} must be a UUID"
        )))
    })?;
    let store = atelier_store(state);
    let record = store
        .get_model_lease(claim_id)
        .await
        .map_err(atelier_error)?;
    if record.actor_id != actor || record.session_id != session_id {
        return Err(atelier_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} is held by actor={} session={}",
            record.actor_id, record.session_id
        ))));
    }
    if record.effective_state != ClaimLeaseState::Active || record.lease_expired {
        return Err(atelier_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} is not active: state={:?} expired={}",
            record.effective_state, record.lease_expired
        ))));
    }
    if !matches!(
        record.claim_mode,
        RoleMailboxClaimMode::ExclusiveLease | RoleMailboxClaimMode::HandoffReservation
    ) {
        return Err(atelier_error(AtelierError::Conflict(format!(
            "model-operation lease {claim_id} has non-mutating claim_mode={:?}",
            record.claim_mode
        ))));
    }
    if let Some(expected_thread_id) = expected_thread_id {
        if record.thread_id != expected_thread_id {
            return Err(atelier_error(AtelierError::Conflict(format!(
                "model-operation lease {claim_id} targets thread_id={} but mutation requires thread_id={expected_thread_id}",
                record.thread_id
            ))));
        }
    }
    Ok(record)
}

/// Coordination thread ids the guarded mutations of this lane bind their leases to
/// (reference `ckc_*_model_operation_thread_id`). Public so tests and other lanes can claim a
/// lease for exactly the thread a route will demand.
pub fn ckc_character_create_model_operation_thread_id(public_id: &str) -> String {
    format!("atelier.ckc.character.public.{}", text_hash(public_id))
}

pub fn ckc_character_model_operation_thread_id(character_internal_id: Uuid) -> String {
    format!("atelier.ckc.character.{character_internal_id}")
}

pub fn ckc_document_model_operation_thread_id(document_id: Uuid) -> String {
    format!("atelier.ckc.document.{document_id}")
}

pub fn ckc_sheet_artifacts_model_operation_thread_id(version_id: Uuid) -> String {
    format!("atelier.ckc.sheet-version.{version_id}.artifacts")
}

// ---------------------------------------------------------------------------------------------
// Sheet text helpers (owner check, import decoding, exports).
// ---------------------------------------------------------------------------------------------

fn safe_subset_sheet_text(raw_text: &str) -> Result<String, ApiError> {
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
) -> Result<String, ApiError> {
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

/// Accept raw sheet text, or a JSON export (`raw_text`, or a `content` wrapper) produced by
/// `/export?format=json`, so an export can be re-imported as the next guarded version.
fn import_sheet_raw_text(raw_text: &str) -> Result<String, ApiError> {
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

/// A full CKC sheet write must carry exactly one `CHAR-ID-001` line and it must name the
/// character the route targets, so ownership never depends on the URL alone.
fn validate_ckc_sheet_owner(
    character: &Character,
    raw_text: &str,
    require_character_id: bool,
) -> Result<(), ApiError> {
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

fn parse_character_document_type(raw: &str) -> Result<CharacterDocumentType, ApiError> {
    CharacterDocumentType::from_token(raw.trim()).map_err(atelier_error)
}

// ---------------------------------------------------------------------------------------------
// Characters and sheet versions.
// ---------------------------------------------------------------------------------------------

/// GET /atelier/characters — stable CKC character list for model/operator selection.
async fn list_characters(
    State(state): State<AppState>,
) -> Result<Json<Vec<CharacterResponse>>, ApiError> {
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
) -> Result<(StatusCode, Json<CharacterResponse>), ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = ckc_character_create_model_operation_thread_id(&payload.public_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
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
) -> Result<Json<CharacterResponse>, ApiError> {
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
) -> Result<Json<Vec<SheetVersionResponse>>, ApiError> {
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

/// POST /atelier/characters/:character_internal_id/sheet-versions — append a guarded sheet edit.
async fn append_sheet_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Json(payload): Json<AppendSheetVersionRequest>,
) -> Result<Response, ApiError> {
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
) -> Result<Response, ApiError> {
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
) -> Result<Response, ApiError> {
    let actor = calling_actor(headers)?;
    let expected_thread_id = ckc_character_model_operation_thread_id(character_internal_id);
    validate_model_operation_lease_if_present(
        state,
        headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
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
) -> Result<Json<SheetVersionResponse>, ApiError> {
    let store = atelier_store(&state);
    let version = store
        .get_sheet_version(version_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(sheet_version_response(version)))
}

/// GET /atelier/sheet-versions/:version_id/export?format=txt|json|safe-txt|safe-json.
async fn export_sheet_version(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
    Query(query): Query<SheetVersionExportQuery>,
) -> Result<Json<SheetVersionExportResponse>, ApiError> {
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
async fn get_default_sheet_template() -> Result<Json<BuiltInSheetTemplate>, ApiError> {
    builtin_character_sheet_template()
        .map(Json)
        .map_err(atelier_error)
}

/// GET /atelier/sheet-templates/default/safe-subset — original LLM-safe v2.00 field whitelist.
async fn get_default_sheet_template_safe_subset() -> Result<Json<BuiltInSafeSubset>, ApiError> {
    builtin_safe_subset().map(Json).map_err(atelier_error)
}

/// GET /atelier/sheet-field-suggestions?field_id=...&limit=... — prior values for one Field ID.
async fn list_sheet_field_suggestions(
    State(state): State<AppState>,
    Query(query): Query<SheetFieldSuggestionsQuery>,
) -> Result<Json<Vec<SheetFieldSuggestion>>, ApiError> {
    let store = atelier_store(&state);
    store
        .sheet_field_suggestions(&query.field_id, query.limit.unwrap_or(20))
        .await
        .map(Json)
        .map_err(atelier_error)
}

// ---------------------------------------------------------------------------------------------
// Sheet artifact links.
// ---------------------------------------------------------------------------------------------

/// GET /atelier/sheet-versions/:version_id/artifact-links — active reusable refs for one version.
async fn list_sheet_artifact_links(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
) -> Result<Json<Vec<SheetArtifactLinkResponse>>, ApiError> {
    let store = atelier_store(&state);
    store
        .get_sheet_version(version_id)
        .await
        .map_err(atelier_error)?;
    let links = store
        .list_sheet_artifacts(version_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(
        links
            .into_iter()
            .map(sheet_artifact_link_response)
            .collect(),
    ))
}

/// GET /atelier/sheet-artifact-links/:link_id — resolve one active typed ref.
async fn get_sheet_artifact_link(
    State(state): State<AppState>,
    Path(link_id): Path<Uuid>,
) -> Result<Json<SheetArtifactLinkResponse>, ApiError> {
    let store = atelier_store(&state);
    let link = store
        .get_sheet_artifact(link_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(sheet_artifact_link_response(link)))
}

/// POST /atelier/sheet-versions/:version_id/artifact-links — attach a reusable artifact ref.
/// 201 on a new link, 200 when the same `(kind, artifact_ref)` is already active.
async fn attach_sheet_artifact_link(
    State(state): State<AppState>,
    Path(version_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<AttachSheetArtifactLinkRequest>,
) -> Result<(StatusCode, Json<SheetArtifactLinkResponse>), ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = ckc_sheet_artifacts_model_operation_thread_id(version_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let store = atelier_store(&state);
    let version = store
        .get_sheet_version(version_id)
        .await
        .map_err(atelier_error)?;
    let artifact_kind =
        SheetArtifactKind::from_token(payload.artifact_kind.trim()).map_err(atelier_error)?;
    let outcome = store
        .link_sheet_artifact_with_status(&NewSheetArtifactLink {
            character_internal_id: version.character_internal_id,
            sheet_version_id: version.version_id,
            artifact_kind,
            artifact_ref: payload.artifact_ref,
            manifest_ref: payload.manifest_ref,
            source_ref: payload.source_ref,
            label: payload.label,
            reuse_role: payload.reuse_role,
            linked_by: actor,
            metadata: payload.metadata.unwrap_or_else(|| serde_json::json!({})),
        })
        .await
        .map_err(atelier_error)?;
    let status = if outcome.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(sheet_artifact_link_response(outcome.link))))
}

/// DELETE /atelier/sheet-artifact-links/:link_id — soft-detach a sheet artifact link.
async fn detach_sheet_artifact_link(
    State(state): State<AppState>,
    Path(link_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<SheetArtifactLinkResponse>, ApiError> {
    let actor = calling_actor(&headers)?;
    let store = atelier_store(&state);
    let current = store
        .get_sheet_artifact(link_id)
        .await
        .map_err(atelier_error)?;
    let expected_thread_id =
        ckc_sheet_artifacts_model_operation_thread_id(current.sheet_version_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
    let link = store
        .detach_sheet_artifact(link_id, &actor)
        .await
        .map_err(atelier_error)?;
    Ok(Json(sheet_artifact_link_response(link)))
}

// ---------------------------------------------------------------------------------------------
// Character documents, story cards/beats, moodboard snapshots.
// ---------------------------------------------------------------------------------------------

/// GET /atelier/characters/:character_internal_id/documents?doc_type=story|moodboard|note.
async fn list_character_documents(
    State(state): State<AppState>,
    Path(character_internal_id): Path<Uuid>,
    Query(query): Query<CharacterDocumentsQuery>,
) -> Result<Json<Vec<CharacterDocumentResponse>>, ApiError> {
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

/// POST /atelier/characters/:character_internal_id/documents — create a story/moodboard/note.
async fn create_character_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Json(payload): Json<CreateCharacterDocumentRequest>,
) -> Result<(StatusCode, Json<CharacterDocumentResponse>), ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = ckc_character_model_operation_thread_id(character_internal_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
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
) -> Result<Json<CharacterDocumentResponse>, ApiError> {
    let store = atelier_store(&state);
    let document = store
        .get_character_document(document_id)
        .await
        .map_err(atelier_error)?;
    Ok(Json(character_document_response(&store, document).await?))
}

/// GET /atelier/character-documents/:document_id/versions — append-only document history.
async fn list_character_document_versions(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<Vec<CharacterDocumentVersionResponse>>, ApiError> {
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

/// POST /atelier/character-documents/:document_id/versions — guarded document append.
async fn append_character_document_version(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<AppendCharacterDocumentVersionRequest>,
) -> Result<Response, ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = ckc_document_model_operation_thread_id(document_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
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
) -> Result<Json<Vec<StoryCardResponse>>, ApiError> {
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
) -> Result<(StatusCode, Json<StoryCardResponse>), ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = ckc_document_model_operation_thread_id(document_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
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
) -> Result<Json<Vec<StoryBeatResponse>>, ApiError> {
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
) -> Result<(StatusCode, Json<StoryBeatResponse>), ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = ckc_document_model_operation_thread_id(document_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
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

/// POST /atelier/character-documents/:document_id/moodboard/snapshots — record a native snapshot.
async fn record_moodboard_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<RecordMoodboardSnapshotRequest>,
) -> Result<Response, ApiError> {
    let actor = calling_actor(&headers)?;
    let expected_thread_id = ckc_document_model_operation_thread_id(document_id);
    validate_model_operation_lease_if_present(
        &state,
        &headers,
        &actor,
        Some(expected_thread_id.as_str()),
    )
    .await?;
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

/// GET /atelier/character-documents/:document_id/moodboard/latest — latest native snapshot.
async fn latest_moodboard_snapshot(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<MoodboardSnapshotResponse>, ApiError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_accepts_raw_text_and_json_exports() {
        assert_eq!(
            import_sheet_raw_text("CHAR-ID-001 - Character_ID: x").unwrap(),
            "CHAR-ID-001 - Character_ID: x"
        );
        let export = serde_json::json!({ "raw_text": "CHAR-ID-001 - Character_ID: y" }).to_string();
        assert_eq!(
            import_sheet_raw_text(&export).unwrap(),
            "CHAR-ID-001 - Character_ID: y"
        );
        let wrapped = serde_json::json!({ "content": export }).to_string();
        assert_eq!(
            import_sheet_raw_text(&wrapped).unwrap(),
            "CHAR-ID-001 - Character_ID: y"
        );
        assert!(import_sheet_raw_text("   ").is_err());
        assert!(import_sheet_raw_text("{not json").is_err());
        assert!(import_sheet_raw_text(r#"{"other": 1}"#).is_err());
    }

    #[test]
    fn safe_subset_export_keeps_only_whitelisted_field_lines() {
        let raw = "CHARACTER SHEET\nCHAR-ID-001 \u{2014} Character_ID: x\nCHAR-SEX-001\u{2014}Sex_Model: y\nfree prose line\n";
        let safe = safe_subset_sheet_text(raw).expect("safe subset");
        assert!(safe.contains("CHAR-ID-001"));
        assert!(!safe.contains("CHAR-SEX-001"));
        assert!(safe.contains("CHARACTER SHEET"));
        assert!(safe.contains("free prose line"));
    }

    #[test]
    fn thread_ids_are_stable_per_target() {
        let id = Uuid::now_v7();
        assert_eq!(
            ckc_character_model_operation_thread_id(id),
            format!("atelier.ckc.character.{id}")
        );
        assert_eq!(
            ckc_sheet_artifacts_model_operation_thread_id(id),
            format!("atelier.ckc.sheet-version.{id}.artifacts")
        );
        assert_eq!(
            ckc_character_create_model_operation_thread_id("a b"),
            ckc_character_create_model_operation_thread_id("a b")
        );
        assert_ne!(
            ckc_character_create_model_operation_thread_id("a"),
            ckc_character_create_model_operation_thread_id("b")
        );
    }
}
