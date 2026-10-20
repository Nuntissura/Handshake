//! WP-CKC-posekit-overhaul SurrealDB port — CKC `media` lane router (MT-010 / MT-011 /
//! MT-033 / MT-034 / MT-035 / MT-036 on the embedded store).
//!
//! Character-scoped media albums over existing catalog assets (create/list/page), album
//! membership with link-scoped folder / source-URL provenance (add, unlink with a durable
//! receipt, full dense reorder, link-ref edit), the per-image notes/tags save, and the CKC
//! fuzzy / vector / combined search plus rich tag notes.
//!
//! Shared helpers come from `super::atelier` (`atelier_store`, `atelier_error`, `atelier_account`, `as_account`,
//! `ErrorResponse`, `LIST_CAP`). Storage authority is the embedded SurrealDB store through
//! `AtelierStore` under the authenticated account session; every mutation is one store call whose statement commits the domain rows and
//! their EventLedger event together. Route paths, request/response JSON shapes and error codes
//! mirror the reference `api/atelier.rs` handlers of the same names.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::atelier::collections::{
    Collection, CollectionItemLinkRefUpdate, CollectionItemReorder, CollectionMemberDetail,
    MediaAlbumMemberEnrichment, MediaNotesTagsUpdate, NewCollection,
    ALBUM_MUTATION_CONCURRENCY_POLICY, ALBUM_REORDER_CONCURRENCY_POLICY,
};
use crate::atelier::refs::{character_ref, collection_ref, media_asset_ref, sheet_version_ref};
use crate::atelier::search::{
    CkcSearchMode, CkcSearchRequest, CkcSearchResponse, CkcTagNote, UpsertCkcTagNote,
};
use crate::atelier::source_evidence::{
    optional_ckc_source_ref_readout, portable_optional_ckc_source_ref, validate_ckc_source_ref,
    CkcSourceRefKind, CkcSourceRefReadout,
};
use crate::atelier::{AtelierError, AtelierStore};
use crate::AppState;

use super::atelier::{as_account, atelier_account, atelier_error, atelier_store, ErrorResponse, LIST_CAP};
use crate::storage::surreal::resource_authority::ResourceAction;

const ALBUM_LIST_DEFAULT_LIMIT: i64 = 50;
const ALBUM_MEMBER_PREVIEW_DEFAULT_LIMIT: i64 = 50;

type ApiError = (StatusCode, Json<ErrorResponse>);

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/atelier/characters/:character_internal_id/media-albums",
            get(list_character_media_albums).post(create_character_media_album),
        )
        .route(
            "/atelier/media-albums/:collection_id/items",
            get(list_media_album_items).post(add_media_album_items),
        )
        .route(
            "/atelier/media-albums/:collection_id/items/reorder",
            patch(reorder_media_album_items),
        )
        .route(
            "/atelier/media-albums/:collection_id/items/:asset_id",
            delete(unlink_media_album_item).patch(update_media_album_item_link),
        )
        .route(
            "/atelier/media-assets/:asset_id/notes-tags",
            post(update_media_notes_tags),
        )
        .route("/atelier/ckc/search", post(search_ckc))
        .route("/atelier/ckc/tag-notes", post(upsert_ckc_tag_note))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::api::authority::require_authenticated_session,
        ))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Request / response shapes (identical to the reference handlers).
// ---------------------------------------------------------------------------

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
struct UpdateMediaAlbumItemLinkRequest {
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    clear_source_path_ref: Option<bool>,
    clear_source_url_ref: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ReorderMediaAlbumItemRequest {
    asset_id: Uuid,
    sort_order: i64,
}

#[derive(Debug, Deserialize)]
struct ReorderMediaAlbumItemsRequest {
    items: Vec<ReorderMediaAlbumItemRequest>,
}

#[derive(Debug, Deserialize)]
struct MediaAlbumItemsQuery {
    cursor: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct MediaAlbumListQuery {
    cursor: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
    member_limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct AddMediaAlbumItemsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    traversal_order: Option<&'static str>,
    collection_id: Uuid,
    collection_ref: String,
    requested: usize,
    inserted: i64,
    offset: i64,
    limit: i64,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<MediaAlbumMemberResponse>,
}

#[derive(Debug, Serialize)]
struct MediaAlbumItemsMutationResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    traversal_order: Option<&'static str>,
    collection_id: Uuid,
    collection_ref: String,
    mutation: &'static str,
    actor_id: String,
    removed_by: Option<String>,
    unlink_receipt_id: Option<Uuid>,
    unlinked_at_utc: Option<DateTime<Utc>>,
    asset_id: Option<Uuid>,
    media_ref: Option<String>,
    requested: usize,
    inserted: i64,
    removed: i64,
    updated: i64,
    reordered: i64,
    concurrency_policy: &'static str,
    offset: i64,
    limit: i64,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<MediaAlbumMemberResponse>,
}

#[derive(Debug, Serialize)]
struct MediaAlbumItemsPageResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    traversal_order: Option<&'static str>,
    collection_id: Uuid,
    collection_ref: String,
    offset: i64,
    limit: i64,
    member_count: usize,
    members_next_offset: Option<i64>,
    members: Vec<MediaAlbumMemberResponse>,
}

#[derive(Debug, Serialize)]
struct MediaAlbumListPageResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    traversal_order: Option<&'static str>,
    character_internal_id: Uuid,
    character_ref: String,
    offset: i64,
    limit: i64,
    member_limit: i64,
    album_count: usize,
    albums_next_offset: Option<i64>,
    albums: Vec<MediaAlbumResponse>,
}

#[derive(Debug, Serialize)]
struct MediaAlbumResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    traversal_order: Option<&'static str>,
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
    created_by: String,
    updated_by: String,
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
    notes_updated_by: Option<String>,
    notes_updated_at_utc: Option<DateTime<Utc>>,
    review_status: Option<String>,
    tags: Vec<String>,
    source_path_ref: Option<String>,
    source_url_ref: Option<String>,
    source_path_ref_kind: Option<&'static str>,
    source_url_ref_kind: Option<&'static str>,
    source_path_ref_readout: Option<CkcSourceRefReadout>,
    source_url_ref_readout: Option<CkcSourceRefReadout>,
    link_source_path_ref: Option<String>,
    link_source_url_ref: Option<String>,
    link_source_path_ref_status: &'static str,
    link_source_url_ref_status: &'static str,
    asset_source_path_ref_status: &'static str,
    asset_source_url_ref_status: &'static str,
    source_path_ref_origin: &'static str,
    source_url_ref_origin: &'static str,
    linked_by: String,
    member_updated_by: String,
    member_updated_at_utc: DateTime<Utc>,
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
    updated_by: String,
    updated_at_utc: DateTime<Utc>,
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

// ---------------------------------------------------------------------------
// Validation helpers.
// ---------------------------------------------------------------------------

fn normalize_offset_limit(
    offset: Option<i64>,
    limit: Option<i64>,
    default_limit: i64,
) -> Result<(i64, i64), ApiError> {
    let offset = offset.unwrap_or(0);
    if offset < 0 {
        return Err(atelier_error(AtelierError::Validation(
            "offset must be >= 0".to_owned(),
        )));
    }
    let limit = limit.unwrap_or(default_limit);
    if limit < 1 {
        return Err(atelier_error(AtelierError::Validation(
            "limit must be >= 1".to_owned(),
        )));
    }
    Ok((offset, limit.min(LIST_CAP)))
}

fn normalize_media_album_items_page_query(query: MediaAlbumItemsQuery) -> Result<(i64, i64), ApiError> {
    normalize_offset_limit(query.offset, query.limit, LIST_CAP)
}

fn normalize_media_album_list_query(
    query: MediaAlbumListQuery,
) -> Result<(i64, i64, i64), ApiError> {
    let (offset, limit) =
        normalize_offset_limit(query.offset, query.limit, ALBUM_LIST_DEFAULT_LIMIT)?;
    let member_limit = query
        .member_limit
        .unwrap_or(ALBUM_MEMBER_PREVIEW_DEFAULT_LIMIT);
    if member_limit < 1 {
        return Err(atelier_error(AtelierError::Validation(
            "member_limit must be >= 1".to_owned(),
        )));
    }
    Ok((offset, limit, member_limit.min(LIST_CAP)))
}

struct MemberSeekQuery {
    limit: i64,
    after: Option<Uuid>,
}

struct AlbumSeekQuery {
    limit: i64,
    member_limit: i64,
    after: Option<(DateTime<Utc>, Uuid)>,
}

fn invalid_media_cursor() -> ApiError {
    atelier_error(AtelierError::Validation("invalid media album cursor".to_owned()))
}

fn checked_cursor_limit(value: i64) -> Result<i64, ApiError> {
    if !(1..=LIST_CAP).contains(&value) || value.checked_add(1).is_none() {
        return Err(invalid_media_cursor());
    }
    Ok(value)
}

fn cursor_limit_field(raw: &str) -> Result<i64, ApiError> {
    let value = raw.parse::<i64>().map_err(|_| invalid_media_cursor())?;
    if value.to_string() != raw { return Err(invalid_media_cursor()); }
    checked_cursor_limit(value)
}

fn cursor_uuid_field(raw: &str) -> Result<Uuid, ApiError> {
    let value = Uuid::parse_str(raw).map_err(|_| invalid_media_cursor())?;
    if value.to_string() != raw { return Err(invalid_media_cursor()); }
    Ok(value)
}

fn require_bound_cursor_limit(supplied: Option<i64>, bound: i64) -> Result<(), ApiError> {
    if supplied.is_some_and(|value| value != bound) { return Err(invalid_media_cursor()); }
    Ok(())
}

fn normalize_member_seek_query(query: &MediaAlbumItemsQuery, collection_id: Uuid) -> Result<MemberSeekQuery, ApiError> {
    if query.offset.is_some() { return Err(invalid_media_cursor()); }
    let raw = query.cursor.as_deref().ok_or_else(invalid_media_cursor)?;
    if raw == "start" {
        return Ok(MemberSeekQuery { limit: checked_cursor_limit(query.limit.unwrap_or(LIST_CAP))?, after: None });
    }
    if raw.len() > 256 { return Err(invalid_media_cursor()); }
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() != 5 || parts[0] != "v1" || parts[1] != "m"
        || cursor_uuid_field(parts[2])? != collection_id {
        return Err(invalid_media_cursor());
    }
    let limit = cursor_limit_field(parts[3])?;
    require_bound_cursor_limit(query.limit, limit)?;
    Ok(MemberSeekQuery { limit, after: Some(cursor_uuid_field(parts[4])?) })
}

fn media_cursor_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}

fn cursor_created_field(raw: &str) -> Result<DateTime<Utc>, ApiError> {
    if raw.is_empty() || raw.len() > 128 || raw.len() % 2 != 0 { return Err(invalid_media_cursor()); }
    fn digit(byte: u8) -> Option<u8> {
        match byte { b'0'..=b'9' => Some(byte - b'0'), b'a'..=b'f' => Some(byte - b'a' + 10), _ => None }
    }
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    for pair in raw.as_bytes().chunks_exact(2) {
        let high = digit(pair[0]).ok_or_else(invalid_media_cursor)?;
        let low = digit(pair[1]).ok_or_else(invalid_media_cursor)?;
        bytes.push((high << 4) | low);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| invalid_media_cursor())?;
    let created = DateTime::parse_from_rfc3339(text).map_err(|_| invalid_media_cursor())?.with_timezone(&Utc);
    if created.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true) != text { return Err(invalid_media_cursor()); }
    Ok(created)
}

fn normalize_album_seek_query(query: &MediaAlbumListQuery, character_id: Uuid) -> Result<AlbumSeekQuery, ApiError> {
    if query.offset.is_some() { return Err(invalid_media_cursor()); }
    let raw = query.cursor.as_deref().ok_or_else(invalid_media_cursor)?;
    if raw == "start" {
        return Ok(AlbumSeekQuery {
            limit: checked_cursor_limit(query.limit.unwrap_or(ALBUM_LIST_DEFAULT_LIMIT))?,
            member_limit: checked_cursor_limit(query.member_limit.unwrap_or(ALBUM_MEMBER_PREVIEW_DEFAULT_LIMIT))?,
            after: None,
        });
    }
    if raw.len() > 512 { return Err(invalid_media_cursor()); }
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() != 7 || parts[0] != "v1" || parts[1] != "a"
        || cursor_uuid_field(parts[2])? != character_id {
        return Err(invalid_media_cursor());
    }
    let limit = cursor_limit_field(parts[3])?;
    let member_limit = cursor_limit_field(parts[4])?;
    require_bound_cursor_limit(query.limit, limit)?;
    require_bound_cursor_limit(query.member_limit, member_limit)?;
    Ok(AlbumSeekQuery {
        limit, member_limit,
        after: Some((cursor_created_field(parts[5])?, cursor_uuid_field(parts[6])?)),
    })
}

fn member_seek_cursor(collection_id: Uuid, limit: i64, asset_id: Uuid) -> String {
    format!("v1:m:{collection_id}:{limit}:{asset_id}")
}

fn album_seek_cursor(character_id: Uuid, limit: i64, member_limit: i64, collection: &Collection) -> String {
    let timestamp = collection.created_at_utc.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true);
    format!("v1:a:{character_id}:{limit}:{member_limit}:{}:{}", media_cursor_hex(timestamp.as_bytes()), collection.collection_id)
}

/// A forbidden machine-local ref is a caller error on this surface (400), not a
/// storage-backend failure.
fn validate_optional_ckc_source_ref_for_api(
    kind: CkcSourceRefKind,
    field: &str,
    value: Option<&str>,
) -> Result<(), ApiError> {
    match value {
        None => Ok(()),
        Some(raw) => match validate_ckc_source_ref(kind, field, raw) {
            Ok(()) => Ok(()),
            Err(AtelierError::ForbiddenStorage(message)) => {
                Err(atelier_error(AtelierError::Validation(message)))
            }
            Err(err) => Err(atelier_error(err)),
        },
    }
}

/// Map the `ForbiddenStorage` the store raises for a machine-local ref onto the
/// 400 the API contract promises for a bad provenance value.
fn media_store_error(err: AtelierError) -> ApiError {
    match err {
        AtelierError::ForbiddenStorage(message) => atelier_error(AtelierError::Validation(message)),
        other => atelier_error(other),
    }
}

enum AlbumMutationApiError {
    Standard(ApiError),
    Unknown {
        operation: crate::storage::surreal::AlbumMutationOperationReference,
    },
}

impl From<ApiError> for AlbumMutationApiError {
    fn from(error: ApiError) -> Self {
        Self::Standard(error)
    }
}

impl axum::response::IntoResponse for AlbumMutationApiError {
    fn into_response(self) -> axum::response::Response {
        use axum::response::IntoResponse;
        match self {
            Self::Standard(error) => error.into_response(),
            Self::Unknown { operation } => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "album_mutation_outcome_unknown",
                    "operation": operation,
                })),
            ).into_response(),
        }
    }
}

fn album_mutation_error(err: AtelierError) -> AlbumMutationApiError {
    use crate::storage::surreal::SurrealStorageError;
    match err {
        AtelierError::Database(SurrealStorageError::AlbumMutationUnknown { operation, source }) => {
            tracing::warn!(operation_id = %operation.operation_id, error = %source,
                "album membership outcome requires authenticated settlement observation");
            AlbumMutationApiError::Unknown { operation }
        }
        AtelierError::Database(
            SurrealStorageError::TransactionSettlementPending { .. }
            | SurrealStorageError::TransactionCommit(_)
            | SurrealStorageError::TransactionCancel(_)
            | SurrealStorageError::TransactionWorker(_),
        ) => AlbumMutationApiError::Standard((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse { error: "album_mutation_outcome_unknown" }),
        )),
        other => AlbumMutationApiError::Standard(media_store_error(other)),
    }
}

/// CKC review-status aliases accepted on the wire, normalised to the canonical
/// `atelier_media_review_metadata.review_status` token.
fn normalize_media_review_status_for_api(status: Option<&str>) -> Result<Option<String>, ApiError> {
    match status
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("pass" | "passed" | "approve" | "approved") => Ok(Some("approved".to_owned())),
        Some("reject" | "rejected") => Ok(Some("rejected".to_owned())),
        Some("unsure" | "hold" | "defer" | "deferred") => Ok(Some("deferred".to_owned())),
        Some("review") => Ok(Some("review".to_owned())),
        Some("unreviewed") => Ok(Some("unreviewed".to_owned())),
        None => Ok(None),
        Some(other) => Err(atelier_error(AtelierError::Validation(format!(
            "unsupported review_status: {other}"
        )))),
    }
}

fn provenance_ref_status(raw: Option<&str>, portable: Option<&str>) -> &'static str {
    match (raw, portable) {
        (None, _) => "none",
        (Some(_), Some(_)) => "present",
        (Some(_), None) => "redacted_invalid",
    }
}

fn visible_media_album_ref_origin(
    link_status: &'static str,
    asset_status: &'static str,
) -> &'static str {
    match (link_status, asset_status) {
        ("present", _) => "link",
        ("redacted_invalid", _) => "link_redacted_invalid",
        (_, "present") => "asset_fallback",
        (_, "redacted_invalid") => "asset_fallback_redacted_invalid",
        _ => "none",
    }
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

fn validate_link_ref_update_request(
    payload: &UpdateMediaAlbumItemLinkRequest,
) -> Result<CollectionItemLinkRefUpdate, ApiError> {
    let clear_path = payload.clear_source_path_ref.unwrap_or(false);
    let clear_url = payload.clear_source_url_ref.unwrap_or(false);
    if clear_path && payload.source_path_ref.is_some() {
        return Err(atelier_error(AtelierError::Validation(
            "source_path_ref cannot be supplied when clear_source_path_ref is true".to_owned(),
        )));
    }
    if clear_url && payload.source_url_ref.is_some() {
        return Err(atelier_error(AtelierError::Validation(
            "source_url_ref cannot be supplied when clear_source_url_ref is true".to_owned(),
        )));
    }
    validate_optional_ckc_source_ref_for_api(
        CkcSourceRefKind::Folder,
        "source_path_ref",
        payload.source_path_ref.as_deref(),
    )?;
    validate_optional_ckc_source_ref_for_api(
        CkcSourceRefKind::SourceUrl,
        "source_url_ref",
        payload.source_url_ref.as_deref(),
    )?;
    let set_path = clear_path || payload.source_path_ref.is_some();
    let set_url = clear_url || payload.source_url_ref.is_some();
    if !set_path && !set_url {
        return Err(atelier_error(AtelierError::Validation(
            "at least one link-scoped provenance field must be set or cleared".to_owned(),
        )));
    }
    Ok(CollectionItemLinkRefUpdate {
        set_source_path_ref: set_path,
        source_path_ref: payload.source_path_ref.clone(),
        set_source_url_ref: set_url,
        source_url_ref: payload.source_url_ref.clone(),
    })
}

fn validate_reorder_media_album_items_request(
    payload: &ReorderMediaAlbumItemsRequest,
) -> Result<Vec<CollectionItemReorder>, ApiError> {
    if payload.items.is_empty() {
        return Err(atelier_error(AtelierError::Validation(
            "reorder items must not be empty".to_owned(),
        )));
    }
    let mut seen = std::collections::HashSet::new();
    let mut seen_orders = std::collections::HashSet::new();
    let mut items = Vec::with_capacity(payload.items.len());
    for item in &payload.items {
        if item.sort_order < 0 {
            return Err(atelier_error(AtelierError::Validation(format!(
                "sort_order for asset_id={} must be >= 0",
                item.asset_id
            ))));
        }
        if !seen.insert(item.asset_id) {
            return Err(atelier_error(AtelierError::Validation(format!(
                "duplicate asset_id={} in reorder request",
                item.asset_id
            ))));
        }
        if !seen_orders.insert(item.sort_order) {
            return Err(atelier_error(AtelierError::Validation(format!(
                "duplicate sort_order={} in reorder request",
                item.sort_order
            ))));
        }
        items.push(CollectionItemReorder {
            asset_id: item.asset_id,
            sort_order: item.sort_order,
        });
    }
    for expected_order in 0..payload.items.len() as i64 {
        if !seen_orders.contains(&expected_order) {
            return Err(atelier_error(AtelierError::Validation(format!(
                "reorder sort_order values must be dense from 0; missing {expected_order}"
            ))));
        }
    }
    Ok(items)
}

fn parse_ckc_search_modes(raw_modes: Option<Vec<String>>) -> Result<Vec<CkcSearchMode>, ApiError> {
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

// ---------------------------------------------------------------------------
// Store-backed response assembly.
// ---------------------------------------------------------------------------

async fn ensure_sheet_version_matches_character(
    store: &AtelierStore,
    character_internal_id: Uuid,
    sheet_version_id: Option<Uuid>,
) -> Result<(), ApiError> {
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

async fn require_ckc_media_album(
    store: &AtelierStore,
    collection_id: Uuid,
) -> Result<Collection, ApiError> {
    let collection = store
        .get_collection(collection_id)
        .await
        .map_err(atelier_error)?;
    if collection.character_internal_id.is_none() {
        return Err(atelier_error(AtelierError::Validation(format!(
            "collection_id={collection_id} is not a CKC character album"
        ))));
    }
    Ok(collection)
}

fn media_album_member_response(
    member: CollectionMemberDetail,
    enrichment: Option<&MediaAlbumMemberEnrichment>,
) -> Result<MediaAlbumMemberResponse, ApiError> {
    let (source_path, source_url) = split_media_source_provenance(member.source_provenance.as_deref());
    let metadata = enrichment.and_then(|row| row.metadata.as_ref());
    let provenance = enrichment.and_then(|row| row.provenance.as_ref());
    let tags = enrichment.map(|row| row.tags.clone()).unwrap_or_default();

    let raw_link_source_path_ref = member.source_path_ref.clone();
    let raw_link_source_url_ref = member.source_url_ref.clone();
    let raw_asset_source_path_ref = provenance.and_then(|row| row.source_path_ref.clone());
    let raw_asset_source_url_ref = provenance.and_then(|row| row.source_url_ref.clone());
    let link_source_path_ref = portable_optional_ckc_source_ref(
        CkcSourceRefKind::Folder,
        "source_path_ref",
        raw_link_source_path_ref.clone(),
    );
    let link_source_url_ref = portable_optional_ckc_source_ref(
        CkcSourceRefKind::SourceUrl,
        "source_url_ref",
        raw_link_source_url_ref.clone(),
    );
    let asset_source_path_ref = portable_optional_ckc_source_ref(
        CkcSourceRefKind::Folder,
        "source_path_ref",
        raw_asset_source_path_ref.clone(),
    );
    let asset_source_url_ref = portable_optional_ckc_source_ref(
        CkcSourceRefKind::SourceUrl,
        "source_url_ref",
        raw_asset_source_url_ref.clone(),
    );
    let link_source_path_ref_status = provenance_ref_status(
        raw_link_source_path_ref.as_deref(),
        link_source_path_ref.as_deref(),
    );
    let link_source_url_ref_status = provenance_ref_status(
        raw_link_source_url_ref.as_deref(),
        link_source_url_ref.as_deref(),
    );
    let asset_source_path_ref_status = provenance_ref_status(
        raw_asset_source_path_ref.as_deref(),
        asset_source_path_ref.as_deref(),
    );
    let asset_source_url_ref_status = provenance_ref_status(
        raw_asset_source_url_ref.as_deref(),
        asset_source_url_ref.as_deref(),
    );
    let source_path_ref_origin =
        visible_media_album_ref_origin(link_source_path_ref_status, asset_source_path_ref_status);
    let source_url_ref_origin =
        visible_media_album_ref_origin(link_source_url_ref_status, asset_source_url_ref_status);
    let source_path_ref = link_source_path_ref
        .clone()
        .or_else(|| asset_source_path_ref.clone());
    let source_url_ref = link_source_url_ref
        .clone()
        .or_else(|| asset_source_url_ref.clone());
    let source_path_ref_readout =
        optional_ckc_source_ref_readout(CkcSourceRefKind::Folder, source_path_ref.as_deref())
            .map_err(atelier_error)?;
    let source_url_ref_readout =
        optional_ckc_source_ref_readout(CkcSourceRefKind::SourceUrl, source_url_ref.as_deref())
            .map_err(atelier_error)?;
    Ok(MediaAlbumMemberResponse {
        asset_id: member.asset_id,
        media_ref: media_asset_ref(member.asset_id),
        file_name: media_display_file_name(member.source_provenance.as_deref(), &member.content_hash),
        content_hash: member.content_hash,
        content_type: member.mime,
        source_path,
        source_url,
        sort_order: member.sort_order,
        added_at_utc: member.added_at_utc,
        notes: metadata.and_then(|row| row.notes.clone()),
        notes_updated_by: metadata.map(|row| row.updated_by.clone()),
        notes_updated_at_utc: metadata.map(|row| row.updated_at_utc),
        review_status: metadata.map(|row| row.review_status.clone()),
        tags,
        source_path_ref,
        source_url_ref,
        source_path_ref_kind: source_path_ref_readout
            .as_ref()
            .map(|readout| readout.ref_kind),
        source_url_ref_kind: source_url_ref_readout
            .as_ref()
            .map(|readout| readout.ref_kind),
        source_path_ref_readout,
        source_url_ref_readout,
        link_source_path_ref,
        link_source_url_ref,
        link_source_path_ref_status,
        link_source_url_ref_status,
        asset_source_path_ref_status,
        asset_source_url_ref_status,
        source_path_ref_origin,
        source_url_ref_origin,
        linked_by: member.linked_by,
        member_updated_by: member.updated_by,
        member_updated_at_utc: member.updated_at_utc,
    })
}

/// One page of album members as the API renders them, plus the canonical
/// member count and the next offset derived from that count (never from the
/// rendered page size).
async fn media_album_members_page_response(
    store: &AtelierStore,
    collection_id: Uuid,
    offset: i64,
    limit: i64,
) -> Result<(Vec<MediaAlbumMemberResponse>, usize, Option<i64>), ApiError> {
    let page = store
        .list_collection_member_page(collection_id, offset, limit)
        .await
        .map_err(atelier_error)?;
    let asset_ids: Vec<Uuid> = page.members.iter().map(|member| member.asset_id).collect();
    let enrichment: HashMap<Uuid, MediaAlbumMemberEnrichment> = store
        .media_album_member_enrichment(&asset_ids)
        .await
        .map_err(atelier_error)?
        .into_iter()
        .map(|row| (row.asset_id, row))
        .collect();
    let mut members = Vec::with_capacity(page.members.len());
    for member in page.members {
        let asset_id = member.asset_id;
        members.push(media_album_member_response(member, enrichment.get(&asset_id))?);
    }
    let next_offset = offset.saturating_add(members.len() as i64);
    let members_next_offset = if page.total_count > next_offset {
        Some(next_offset)
    } else {
        None
    };
    Ok((members, page.total_count.max(0) as usize, members_next_offset))
}

async fn media_album_members_seek_response(
    store: &AtelierStore,
    collection_id: Uuid,
    query: MemberSeekQuery,
) -> Result<MediaAlbumItemsPageResponse, ApiError> {
    let page = store.list_collection_member_seek_page(collection_id, query.after, query.limit)
        .await.map_err(atelier_error)?;
    let next_cursor = if page.has_more {
        page.members.last().map(|member| member_seek_cursor(collection_id, query.limit, member.asset_id))
    } else { None };
    let asset_ids: Vec<Uuid> = page.members.iter().map(|member| member.asset_id).collect();
    let enrichment: HashMap<Uuid, MediaAlbumMemberEnrichment> = store.media_album_member_enrichment(&asset_ids)
        .await.map_err(atelier_error)?.into_iter().map(|row| (row.asset_id, row)).collect();
    let mut members = Vec::with_capacity(page.members.len());
    for member in page.members {
        let asset_id = member.asset_id;
        members.push(media_album_member_response(member, enrichment.get(&asset_id))?);
    }
    Ok(MediaAlbumItemsPageResponse {
        collection_id,
        collection_ref: collection_ref(collection_id),
        offset: 0,
        limit: query.limit,
        member_count: page.total_count.max(0) as usize,
        members_next_offset: None,
        next_cursor,
        traversal_order: Some("asset_id_asc"),
        members,
    })
}

async fn character_media_albums_seek_response(
    store: &AtelierStore,
    character_id: Uuid,
    query: AlbumSeekQuery,
) -> Result<MediaAlbumListPageResponse, ApiError> {
    let page = store.list_character_collections_seek_page(character_id, query.after, query.limit)
        .await.map_err(atelier_error)?;
    let next_cursor = if page.has_more {
        page.collections.last().map(|collection| album_seek_cursor(character_id, query.limit, query.member_limit, collection))
    } else { None };
    let mut albums = Vec::with_capacity(page.collections.len());
    for collection in page.collections {
        albums.push(media_album_response(store, collection, query.member_limit).await?);
    }
    Ok(MediaAlbumListPageResponse {
        character_internal_id: character_id,
        character_ref: character_ref(character_id),
        offset: 0,
        limit: query.limit,
        member_limit: query.member_limit,
        album_count: page.total_count.max(0) as usize,
        albums_next_offset: None,
        next_cursor,
        traversal_order: Some("created_at_desc_collection_id_asc"),
        albums,
    })
}

async fn media_album_response(
    store: &AtelierStore,
    collection: Collection,
    member_limit: i64,
) -> Result<MediaAlbumResponse, ApiError> {
    let Some(character_internal_id) = collection.character_internal_id else {
        return Err(atelier_error(AtelierError::Validation(format!(
            "collection_id={} is not linked to a CKC character",
            collection.collection_id
        ))));
    };
    let (members, member_count, members_next_offset) =
        media_album_members_page_response(store, collection.collection_id, 0, member_limit).await?;
    Ok(MediaAlbumResponse {
            next_cursor: None,
            traversal_order: None,
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
        created_by: collection.created_by,
        updated_by: collection.updated_by,
        created_at_utc: collection.created_at_utc,
        updated_at_utc: collection.updated_at_utc,
    })
}

#[allow(clippy::too_many_arguments)]
async fn media_album_items_mutation_response(
    store: &AtelierStore,
    collection_id: Uuid,
    mutation: &'static str,
    actor_id: String,
    asset_id: Option<Uuid>,
    requested: usize,
    inserted: i64,
    removed: i64,
    updated: i64,
    reordered: i64,
    unlink_receipt_id: Option<Uuid>,
    unlinked_at_utc: Option<DateTime<Utc>>,
) -> Result<MediaAlbumItemsMutationResponse, ApiError> {
    let (members, member_count, members_next_offset) =
        media_album_members_page_response(store, collection_id, 0, LIST_CAP).await?;
    Ok(MediaAlbumItemsMutationResponse {
            next_cursor: None,
            traversal_order: None,
        collection_id,
        collection_ref: collection_ref(collection_id),
        mutation,
        removed_by: (mutation == "unlink").then(|| actor_id.clone()),
        unlink_receipt_id,
        unlinked_at_utc,
        actor_id,
        asset_id,
        media_ref: asset_id.map(media_asset_ref),
        requested,
        inserted,
        removed,
        updated,
        reordered,
        concurrency_policy: match mutation {
            "reorder" => ALBUM_REORDER_CONCURRENCY_POLICY,
            _ => ALBUM_MUTATION_CONCURRENCY_POLICY,
        },
        offset: 0,
        limit: LIST_CAP,
        member_count,
        members_next_offset,
        members,
    })
}

// ---------------------------------------------------------------------------
// Handlers.
// ---------------------------------------------------------------------------

/// GET /atelier/characters/:character_internal_id/media-albums — character-scoped CKC albums.
async fn list_character_media_albums(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Query(query): Query<MediaAlbumListQuery>,
) -> Result<Json<MediaAlbumListPageResponse>, ApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Read).await?;
    as_account(&state, &account, async {
        let store = atelier_store(&state);
        store
            .get_character_by_internal_id(character_internal_id)
            .await
            .map_err(atelier_error)?;
        if query.cursor.is_some() {
            let seek = normalize_album_seek_query(&query, character_internal_id)?;
            return Ok(Json(character_media_albums_seek_response(&store, character_internal_id, seek).await?));
        }
        let (offset, limit, member_limit) = normalize_media_album_list_query(query)?;
        let page = store
            .list_character_collections_page(character_internal_id, offset, limit)
            .await
            .map_err(atelier_error)?;
        let next_offset = offset.saturating_add(page.collections.len() as i64);
        let albums_next_offset = if page.total_count > next_offset {
            Some(next_offset)
        } else {
            None
        };
        let mut albums = Vec::with_capacity(page.collections.len());
        for collection in page.collections {
            albums.push(media_album_response(&store, collection, member_limit).await?);
        }
        Ok(Json(MediaAlbumListPageResponse {
            next_cursor: None,
            traversal_order: None,
            character_internal_id,
            character_ref: character_ref(character_internal_id),
            offset,
            limit,
            member_limit,
            album_count: page.total_count.max(0) as usize,
            albums_next_offset,
            albums,
        }))
    }).await
}

/// POST /atelier/characters/:character_internal_id/media-albums — create a CKC album over existing media.
async fn create_character_media_album(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(character_internal_id): Path<Uuid>,
    Json(payload): Json<CreateMediaAlbumRequest>,
) -> Result<(StatusCode, Json<MediaAlbumResponse>), ApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Create).await?;
    as_account(&state, &account, async {
        let actor = account.actor_id.clone();
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
            .map_err(media_store_error)?;
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
            Json(media_album_response(&store, collection, LIST_CAP).await?),
        ))
    }).await
}

/// GET /atelier/media-albums/:collection_id/items — fetch a page of album media members.
async fn list_media_album_items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection_id): Path<Uuid>,
    Query(query): Query<MediaAlbumItemsQuery>,
) -> Result<Json<MediaAlbumItemsPageResponse>, ApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Read).await?;
    as_account(&state, &account, async {
        let store = atelier_store(&state);
        require_ckc_media_album(&store, collection_id).await?;
        if query.cursor.is_some() {
            let seek = normalize_member_seek_query(&query, collection_id)?;
            return Ok(Json(media_album_members_seek_response(&store, collection_id, seek).await?));
        }
        let (offset, limit) = normalize_media_album_items_page_query(query)?;
        let (members, member_count, members_next_offset) =
            media_album_members_page_response(&store, collection_id, offset, limit).await?;
        Ok(Json(MediaAlbumItemsPageResponse {
            next_cursor: None,
            traversal_order: None,
            collection_id,
            collection_ref: collection_ref(collection_id),
            offset,
            limit,
            member_count,
            members_next_offset,
            members,
        }))
    }).await
}

/// POST /atelier/media-albums/:collection_id/items — append existing media assets to an album.
async fn add_media_album_items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection_id): Path<Uuid>,
    Json(payload): Json<AddMediaAlbumItemsRequest>,
) -> Result<Json<AddMediaAlbumItemsResponse>, ApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Update).await?;
    as_account(&state, &account, async {
        let actor = account.actor_id.clone();
        let store = atelier_store(&state);
        require_ckc_media_album(&store, collection_id).await?;
        validate_optional_ckc_source_ref_for_api(
            CkcSourceRefKind::Folder,
            "source_path_ref",
            payload.source_path_ref.as_deref(),
        )?;
        validate_optional_ckc_source_ref_for_api(
            CkcSourceRefKind::SourceUrl,
            "source_url_ref",
            payload.source_url_ref.as_deref(),
        )?;
        let requested = payload.asset_ids.len();
        let added = store
            .add_images_to_collection_with_link_refs_attributed(
                collection_id,
                &payload.asset_ids,
                payload.source_path_ref.as_deref(),
                payload.source_url_ref.as_deref(),
                &actor,
            )
            .await
            .map_err(media_store_error)?;
        let (members, member_count, members_next_offset) =
            media_album_members_page_response(&store, collection_id, 0, LIST_CAP).await?;
        tracing::info!(
            target: "handshake_core::atelier",
            route = "/atelier/media-albums/:collection_id/items",
            status = "ok",
            actor = %actor,
            collection_id = %collection_id,
            requested = requested,
            inserted = added.inserted,
            updated_refs = added.updated_refs,
            "add CKC media album items"
        );
        Ok(Json(AddMediaAlbumItemsResponse {
            next_cursor: None,
            traversal_order: None,
            collection_id,
            collection_ref: collection_ref(collection_id),
            requested,
            inserted: added.inserted,
            offset: 0,
            limit: LIST_CAP,
            member_count,
            members_next_offset,
            members,
        }))
    }).await
}

/// DELETE /atelier/media-albums/:collection_id/items/:asset_id — unlink one album membership only.
async fn unlink_media_album_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection_id, asset_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MediaAlbumItemsMutationResponse>, AlbumMutationApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Delete).await?;
    as_account(&state, &account, async {
        let actor = account.actor_id.clone();
        let store = atelier_store(&state);
        require_ckc_media_album(&store, collection_id).await?;
        let receipt = store
            .unlink_media_album_item(collection_id, asset_id, &actor)
            .await
            .map_err(album_mutation_error)?;
        let response = media_album_items_mutation_response(
            &store,
            collection_id,
            "unlink",
            actor.clone(),
            Some(asset_id),
            1,
            0,
            1,
            0,
            0,
            Some(receipt.unlink_receipt_id),
            Some(receipt.unlinked_at_utc),
        )
        .await?;
        tracing::info!(
            target: "handshake_core::atelier",
            route = "/atelier/media-albums/:collection_id/items/:asset_id",
            status = "ok",
            actor = %actor,
            collection_id = %collection_id,
            asset_id = %asset_id,
            unlink_receipt_id = %receipt.unlink_receipt_id,
            "unlink CKC media album item"
        );
        Ok(Json(response))
    }).await
}

/// PATCH /atelier/media-albums/:collection_id/items/:asset_id — edit link-scoped provenance.
async fn update_media_album_item_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((collection_id, asset_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateMediaAlbumItemLinkRequest>,
) -> Result<Json<MediaAlbumItemsMutationResponse>, AlbumMutationApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Update).await?;
    as_account(&state, &account, async {
        let actor = account.actor_id.clone();
        let update = validate_link_ref_update_request(&payload)?;
        let store = atelier_store(&state);
        require_ckc_media_album(&store, collection_id).await?;
        let updated = store
            .update_media_album_item_link_refs(collection_id, asset_id, &update, &actor)
            .await
            .map_err(album_mutation_error)?;
        let response = media_album_items_mutation_response(
            &store,
            collection_id,
            "link_ref_edit",
            actor.clone(),
            Some(asset_id),
            1,
            0,
            0,
            updated,
            0,
            None,
            None,
        )
        .await?;
        tracing::info!(
            target: "handshake_core::atelier",
            route = "/atelier/media-albums/:collection_id/items/:asset_id",
            status = "ok",
            actor = %actor,
            collection_id = %collection_id,
            asset_id = %asset_id,
            updated = updated,
            "update CKC media album item link refs"
        );
        Ok(Json(response))
    }).await
}

/// PATCH /atelier/media-albums/:collection_id/items/reorder — set explicit album-member order.
async fn reorder_media_album_items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(collection_id): Path<Uuid>,
    Json(payload): Json<ReorderMediaAlbumItemsRequest>,
) -> Result<Json<MediaAlbumItemsMutationResponse>, AlbumMutationApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Update).await?;
    as_account(&state, &account, async {
        let actor = account.actor_id.clone();
        let items = validate_reorder_media_album_items_request(&payload)?;
        let store = atelier_store(&state);
        require_ckc_media_album(&store, collection_id).await?;
        let changed = store
            .reorder_media_album_items(collection_id, &items, &actor)
            .await
            .map_err(album_mutation_error)?;
        let response = media_album_items_mutation_response(
            &store,
            collection_id,
            "reorder",
            actor.clone(),
            None,
            items.len(),
            0,
            0,
            changed,
            items.len() as i64,
            None,
            None,
        )
        .await?;
        tracing::info!(
            target: "handshake_core::atelier",
            route = "/atelier/media-albums/:collection_id/items/reorder",
            status = "ok",
            actor = %actor,
            collection_id = %collection_id,
            requested = items.len(),
            changed = changed,
            "reorder CKC media album items"
        );
        Ok(Json(response))
    }).await
}

/// POST /atelier/media-assets/:asset_id/notes-tags — save image notes/tags separate from sheet text.
async fn update_media_notes_tags(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(asset_id): Path<Uuid>,
    Json(payload): Json<MediaNotesTagsRequest>,
) -> Result<Json<MediaNotesTagsResponse>, ApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Update).await?;
    as_account(&state, &account, async {
        let actor = account.actor_id.clone();
        let store = atelier_store(&state);
        validate_optional_ckc_source_ref_for_api(
            CkcSourceRefKind::Folder,
            "source_path_ref",
            payload.source_path_ref.as_deref(),
        )?;
        validate_optional_ckc_source_ref_for_api(
            CkcSourceRefKind::SourceUrl,
            "source_url_ref",
            payload.source_url_ref.as_deref(),
        )?;
        let review_status = normalize_media_review_status_for_api(payload.review_status.as_deref())?;
        let result = store
            .apply_media_notes_tags(&MediaNotesTagsUpdate {
                asset_id,
                notes: payload.notes,
                tags: payload.tags,
                review_status,
                source_path_ref: payload.source_path_ref,
                source_url_ref: payload.source_url_ref,
                updated_by: actor.clone(),
            })
            .await
            .map_err(media_store_error)?;
        tracing::info!(
            target: "handshake_core::atelier",
            route = "/atelier/media-assets/:asset_id/notes-tags",
            status = "ok",
            actor = %actor,
            asset_id = %asset_id,
            added_tags = result.added_tags.len(),
            removed_tags = result.removed_tags.len(),
            "update CKC media notes/tags"
        );
        Ok(Json(MediaNotesTagsResponse {
            asset_id,
            media_ref: media_asset_ref(asset_id),
            notes: result.metadata.notes,
            review_status: result.metadata.review_status,
            tags: result.tags,
            source_path_ref: result
                .provenance
                .as_ref()
                .and_then(|row| row.source_path_ref.clone()),
            source_url_ref: result
                .provenance
                .as_ref()
                .and_then(|row| row.source_url_ref.clone()),
            updated_by: result.metadata.updated_by,
            updated_at_utc: result.metadata.updated_at_utc,
        }))
    }).await
}

/// POST /atelier/ckc/search — fuzzy/vector/combined CKC search over characters, sheets, albums, media, tags, and tag notes.
async fn search_ckc(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CkcSearchApiRequest>,
) -> Result<Json<CkcSearchResponse>, ApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Read).await?;
    as_account(&state, &account, async {
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
    }).await
}

/// POST /atelier/ckc/tag-notes — rich tag note round-trip, separate from sheet/media/album notes.
async fn upsert_ckc_tag_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CkcTagNoteRequest>,
) -> Result<Json<CkcTagNote>, ApiError> {
    let account = atelier_account(&state, &headers, ResourceAction::Update).await?;
    as_account(&state, &account, async {
        let actor = account.actor_id.clone();
        let store = atelier_store(&state);
        let tag_note = store
            .upsert_ckc_tag_note(&UpsertCkcTagNote {
                tag_text: payload.tag_text,
                scope_ref: payload.scope_ref,
                note: payload.note,
                updated_by: actor.clone(),
            })
            .await
            .map_err(media_store_error)?;
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
    }).await
}
