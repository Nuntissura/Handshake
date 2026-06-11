//! Media assets / DAM (MT-015): media identity, provenance, and content-hash
//! dedup. Bytes live in the ArtifactStore (`artifact_ref`), never on random
//! filesystem paths and never in `.GOV`. Identity is stable across file moves.

use crate::storage::artifacts::{
    artifact_root_rel, read_artifact_manifest, resolve_workspace_root,
    validate_artifact_content_hash, ArtifactLayer,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

use super::{
    event_family, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
    BulkOperationReceipt,
};

pub const MEDIA_ARTIFACT_MANIFEST_SCHEMA: &str = "hsk.atelier.media_artifact_manifest@1";
pub const MEDIA_ORIGINAL_RETENTION_CLASS: &str = "atelier.media.original.retained";
const INVALID_LEGACY_ARTIFACT_REF_STATE: &str = "invalid_legacy_artifact_ref";
const INVALID_LEGACY_ARTIFACT_REF_REASON: &str =
    "legacy artifact_ref is not a native ArtifactStore payload handle";
const INVALID_ARTIFACT_STORE_BINDING_STATE: &str = "invalid_artifact_store_binding";
const INVALID_ARTIFACT_STORE_BINDING_REASON: &str =
    "artifact_ref could not be validated against ArtifactStore";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaAsset {
    pub asset_id: Uuid,
    pub content_hash: String,
    pub mime: String,
    pub byte_len: i64,
    pub source_provenance: Option<String>,
    pub artifact_ref: String,
    pub retention_class: String,
    pub artifact_manifest: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaSidecarRelationKind {
    OpenPoseJson,
    WorkflowJson,
}

impl MediaSidecarRelationKind {
    pub fn as_token(self) -> &'static str {
        match self {
            MediaSidecarRelationKind::OpenPoseJson => "openpose_json",
            MediaSidecarRelationKind::WorkflowJson => "workflow_json",
        }
    }

    fn from_token(value: &str) -> AtelierResult<Self> {
        match value {
            "openpose_json" => Ok(MediaSidecarRelationKind::OpenPoseJson),
            "workflow_json" => Ok(MediaSidecarRelationKind::WorkflowJson),
            other => Err(AtelierError::Validation(format!(
                "unsupported media sidecar relation kind: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaSidecar {
    pub sidecar_id: Uuid,
    pub parent_asset_id: Uuid,
    pub sidecar_asset_id: Uuid,
    pub relation_kind: MediaSidecarRelationKind,
    pub hidden_from_gallery: bool,
    pub searchable_by_relation: bool,
    pub created_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewMediaAsset {
    pub content_hash: String,
    pub mime: String,
    pub byte_len: i64,
    pub source_provenance: Option<String>,
    pub artifact_ref: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaSourceProvenanceRefs {
    pub asset_id: Uuid,
    pub source_url_ref: Option<String>,
    pub source_path_ref: Option<String>,
    pub source_note_ref: Option<String>,
    pub contact_sheet_ref: Option<String>,
    pub task_ref: Option<String>,
    pub run_ref: Option<String>,
    pub updated_by: String,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct SetMediaSourceProvenanceRefs {
    pub asset_id: Uuid,
    pub source_url_ref: Option<String>,
    pub source_path_ref: Option<String>,
    pub source_note_ref: Option<String>,
    pub contact_sheet_ref: Option<String>,
    pub task_ref: Option<String>,
    pub run_ref: Option<String>,
    pub updated_by: String,
}

#[derive(Clone, Debug)]
pub struct NewMediaSidecarRelation {
    pub parent_asset_id: Uuid,
    pub sidecar_asset_id: Uuid,
    pub relation_kind: MediaSidecarRelationKind,
    pub created_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaReviewMetadata {
    pub asset_id: Uuid,
    pub favorite: bool,
    pub rating: i16,
    pub frontpage: bool,
    pub carousel: bool,
    pub notes: Option<String>,
    pub review_status: String,
    pub updated_by: String,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct MediaReviewMetadataUpdate {
    pub asset_id: Uuid,
    pub favorite: bool,
    pub rating: i16,
    pub frontpage: bool,
    pub carousel: bool,
    pub notes: Option<String>,
    pub review_status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BulkMediaReviewMetadataResult {
    pub receipt: BulkOperationReceipt,
    pub metadata: Vec<MediaReviewMetadata>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaDerivativeKind {
    Thumbnail,
    Proxy,
    PhotoStudioSkeleton,
}

impl MediaDerivativeKind {
    pub fn as_token(self) -> &'static str {
        match self {
            MediaDerivativeKind::Thumbnail => "thumbnail",
            MediaDerivativeKind::Proxy => "proxy",
            MediaDerivativeKind::PhotoStudioSkeleton => "photo_studio_skeleton",
        }
    }

    fn from_token(value: &str) -> AtelierResult<Self> {
        match value {
            "thumbnail" => Ok(MediaDerivativeKind::Thumbnail),
            "proxy" => Ok(MediaDerivativeKind::Proxy),
            "photo_studio_skeleton" => Ok(MediaDerivativeKind::PhotoStudioSkeleton),
            other => Err(AtelierError::Validation(format!(
                "unsupported media derivative kind: {other}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaDerivativeStatus {
    Pending,
    Generating,
    Generated,
    RetryableError,
    Failed,
}

impl MediaDerivativeStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            MediaDerivativeStatus::Pending => "pending",
            MediaDerivativeStatus::Generating => "generating",
            MediaDerivativeStatus::Generated => "generated",
            MediaDerivativeStatus::RetryableError => "retryable_error",
            MediaDerivativeStatus::Failed => "failed",
        }
    }

    fn from_token(value: &str) -> AtelierResult<Self> {
        match value {
            "pending" => Ok(MediaDerivativeStatus::Pending),
            "generating" => Ok(MediaDerivativeStatus::Generating),
            "generated" => Ok(MediaDerivativeStatus::Generated),
            "retryable_error" => Ok(MediaDerivativeStatus::RetryableError),
            "failed" => Ok(MediaDerivativeStatus::Failed),
            other => Err(AtelierError::Validation(format!(
                "unsupported media derivative status: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaDerivative {
    pub derivative_id: Uuid,
    pub asset_id: Uuid,
    pub derivative_kind: MediaDerivativeKind,
    pub target_width: i32,
    pub target_height: i32,
    pub format: String,
    pub status: MediaDerivativeStatus,
    pub artifact_ref: Option<String>,
    pub artifact_manifest_ref: Option<String>,
    pub mime: Option<String>,
    pub byte_len: Option<i64>,
    pub requested_by: String,
    pub updated_by: String,
    pub attempt_count: i64,
    pub retry_count: i64,
    pub last_error_code: Option<String>,
    pub last_error_ref: Option<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct MediaDerivativeRequest {
    pub asset_id: Uuid,
    pub derivative_kind: MediaDerivativeKind,
    pub target_width: i32,
    pub target_height: i32,
    pub format: String,
    pub requested_by: String,
}

#[derive(Clone, Debug)]
pub struct MediaDerivativeGenerated {
    pub derivative_id: Uuid,
    pub artifact_ref: String,
    pub artifact_manifest_ref: String,
    pub mime: String,
    pub byte_len: i64,
    pub updated_by: String,
}

#[derive(Clone, Debug)]
pub struct MediaDerivativeFailure {
    pub error_code: String,
    pub error_detail: String,
    pub retryable: bool,
    pub updated_by: String,
}

#[derive(Clone, Debug)]
struct NormalizedMediaReviewMetadataUpdate {
    asset_id: Uuid,
    favorite: bool,
    rating: i16,
    frontpage: bool,
    carousel: bool,
    notes: Option<String>,
    notes_ref: Option<String>,
    review_status: String,
}

fn asset_from_row(row: &sqlx::postgres::PgRow) -> MediaAsset {
    MediaAsset {
        asset_id: row.get("asset_id"),
        content_hash: row.get("content_hash"),
        mime: row.get("mime"),
        byte_len: row.get("byte_len"),
        source_provenance: row.get("source_provenance"),
        artifact_ref: row.get("artifact_ref"),
        retention_class: row.get("retention_class"),
        artifact_manifest: row.get("artifact_manifest"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn media_source_provenance_refs_from_row(row: &sqlx::postgres::PgRow) -> MediaSourceProvenanceRefs {
    MediaSourceProvenanceRefs {
        asset_id: row.get("asset_id"),
        source_url_ref: row.get("source_url_ref"),
        source_path_ref: row.get("source_path_ref"),
        source_note_ref: row.get("source_note_ref"),
        contact_sheet_ref: row.get("contact_sheet_ref"),
        task_ref: row.get("task_ref"),
        run_ref: row.get("run_ref"),
        updated_by: row.get("updated_by"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn media_sidecar_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<MediaSidecar> {
    let relation_kind: String = row.get("relation_kind");
    Ok(MediaSidecar {
        sidecar_id: row.get("sidecar_id"),
        parent_asset_id: row.get("parent_asset_id"),
        sidecar_asset_id: row.get("sidecar_asset_id"),
        relation_kind: MediaSidecarRelationKind::from_token(&relation_kind)?,
        hidden_from_gallery: row.get("hidden_from_gallery"),
        searchable_by_relation: row.get("searchable_by_relation"),
        created_by: row.get("created_by"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn review_metadata_from_row(row: &sqlx::postgres::PgRow) -> MediaReviewMetadata {
    MediaReviewMetadata {
        asset_id: row.get("asset_id"),
        favorite: row.get("favorite"),
        rating: row.get("rating"),
        frontpage: row.get("frontpage"),
        carousel: row.get("carousel"),
        notes: row.get("notes"),
        review_status: row.get("review_status"),
        updated_by: row.get("updated_by"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn media_derivative_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<MediaDerivative> {
    let kind: String = row.get("derivative_kind");
    let status: String = row.get("status");
    Ok(MediaDerivative {
        derivative_id: row.get("derivative_id"),
        asset_id: row.get("asset_id"),
        derivative_kind: MediaDerivativeKind::from_token(&kind)?,
        target_width: row.get("target_width"),
        target_height: row.get("target_height"),
        format: row.get("format"),
        status: MediaDerivativeStatus::from_token(&status)?,
        artifact_ref: row.get("artifact_ref"),
        artifact_manifest_ref: row.get("artifact_manifest_ref"),
        mime: row.get("mime"),
        byte_len: row.get("byte_len"),
        requested_by: row.get("requested_by"),
        updated_by: row.get("updated_by"),
        attempt_count: row.get("attempt_count"),
        retry_count: row.get("retry_count"),
        last_error_code: row.get("last_error_code"),
        last_error_ref: row.get("last_error_ref"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn validate_artifact_ref(artifact_ref: &str) -> AtelierResult<()> {
    let trimmed = artifact_ref.trim();
    if trimmed.is_empty() || trimmed != artifact_ref {
        return Err(AtelierError::Validation(
            "artifact_ref must not be empty or padded".into(),
        ));
    }
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("artifact://") {
        return Err(AtelierError::Validation(
            "media artifact_ref must be an ArtifactStore handle (artifact://...)".into(),
        ));
    }

    let body = &trimmed["artifact://".len()..];
    let first_segment = body
        .split('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if lower.contains(".gov")
        || body.is_empty()
        || body.starts_with('/')
        || body.contains(':')
        || body.contains("..")
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_whitespace)
        || first_segment == "localhost"
        || first_segment == "0.0.0.0"
        || first_segment == "::1"
        || first_segment == "[::1]"
        || first_segment.starts_with("127.")
    {
        return Err(AtelierError::Validation(
            "media artifact_ref must be a native ArtifactStore handle, not a filesystem, URL, network host, whitespace, drive-letter, traversal, or .GOV path".into(),
        ));
    }
    Ok(())
}

fn normalized_sha256_hex(content_hash: &str) -> AtelierResult<&str> {
    let hash = content_hash.trim();
    if hash != content_hash {
        return Err(AtelierError::Validation(
            "content_hash must not be padded".into(),
        ));
    }
    let hex = hash.strip_prefix("sha256:").unwrap_or(hash);
    if hex.len() != 64 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(AtelierError::Validation(
            "content_hash must be sha256:<64 hex> or bare 64 hex".into(),
        ));
    }
    Ok(hex)
}

fn validate_sha256_content_hash(content_hash: &str) -> AtelierResult<()> {
    normalized_sha256_hex(content_hash).map(|_| ())
}

fn canonical_sha256_content_hash(content_hash: &str) -> AtelierResult<String> {
    Ok(normalized_sha256_hex(content_hash)?.to_ascii_lowercase())
}

fn validated_source_provenance(source_provenance: &Option<String>) -> AtelierResult<&str> {
    let Some(source) = source_provenance.as_deref() else {
        return Err(AtelierError::Validation(
            "source_provenance is required for media materialization".into(),
        ));
    };
    let trimmed = source.trim();
    if trimmed.is_empty() || trimmed != source {
        return Err(AtelierError::Validation(
            "source_provenance must not be empty or padded".into(),
        ));
    }
    reject_legacy_runtime_ref("source_provenance", source)?;
    Ok(source)
}

fn normalize_optional_provenance_ref(
    field: &str,
    value: &Option<String>,
) -> AtelierResult<Option<String>> {
    match value.as_deref() {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed != raw {
                return Err(AtelierError::Validation(format!(
                    "{field} must not be empty or padded"
                )));
            }
            reject_legacy_runtime_ref(field, raw)?;
            Ok(Some(raw.to_string()))
        }
    }
}

fn validate_media_source_provenance_refs(
    update: &SetMediaSourceProvenanceRefs,
) -> AtelierResult<[Option<String>; 6]> {
    let refs = [
        normalize_optional_provenance_ref("source_url_ref", &update.source_url_ref)?,
        normalize_optional_provenance_ref("source_path_ref", &update.source_path_ref)?,
        normalize_optional_provenance_ref("source_note_ref", &update.source_note_ref)?,
        normalize_optional_provenance_ref("contact_sheet_ref", &update.contact_sheet_ref)?,
        normalize_optional_provenance_ref("task_ref", &update.task_ref)?,
        normalize_optional_provenance_ref("run_ref", &update.run_ref)?,
    ];
    if refs.iter().all(Option::is_none) {
        return Err(AtelierError::Validation(
            "at least one media source provenance ref is required".into(),
        ));
    }
    let updated_by = update.updated_by.trim();
    if updated_by.is_empty() || updated_by != update.updated_by {
        return Err(AtelierError::Validation(
            "source provenance updated_by must not be empty or padded".into(),
        ));
    }
    reject_legacy_runtime_ref("source provenance updated_by", &update.updated_by)?;
    Ok(refs)
}

fn sha256_ref(text: &str) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(text.as_bytes())))
}

fn require_review_metadata_requester(requested_by: &str) -> AtelierResult<&str> {
    let requested_by = requested_by.trim();
    if requested_by.is_empty() {
        return Err(AtelierError::Validation(
            "review metadata requested_by must not be empty".into(),
        ));
    }
    Ok(requested_by)
}

fn require_derivative_actor<'a>(field: &str, value: &'a str) -> AtelierResult<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(value)
}

fn require_sidecar_actor(value: &str) -> AtelierResult<&str> {
    require_derivative_actor("media sidecar created_by", value)
}

fn validate_derivative_dimensions(width: i32, height: i32) -> AtelierResult<()> {
    if !(1..=16384).contains(&width) || !(1..=16384).contains(&height) {
        return Err(AtelierError::Validation(
            "media derivative target dimensions must be between 1 and 16384".into(),
        ));
    }
    Ok(())
}

fn normalize_derivative_format(format: &str) -> AtelierResult<String> {
    let normalized = format.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "png" | "jpeg" => Ok(normalized),
        _ => Err(AtelierError::Validation(format!(
            "unsupported media derivative format: {format}"
        ))),
    }
}

fn normalize_derivative_mime(mime: &str) -> AtelierResult<&str> {
    let mime = mime.trim();
    if mime.is_empty() {
        return Err(AtelierError::Validation(
            "media derivative mime must not be empty".into(),
        ));
    }
    match mime {
        "image/png" | "image/jpeg" => Ok(mime),
        _ => Err(AtelierError::Validation(format!(
            "unsupported media derivative mime: {mime}"
        ))),
    }
}

fn expected_mime_for_derivative_format(format: &str) -> AtelierResult<&'static str> {
    match format {
        "png" => Ok("image/png"),
        "jpeg" => Ok("image/jpeg"),
        other => Err(AtelierError::Validation(format!(
            "unsupported media derivative format: {other}"
        ))),
    }
}

fn normalize_error_code(error_code: &str) -> AtelierResult<String> {
    let normalized = error_code.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || !normalized
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return Err(AtelierError::Validation(
            "media derivative error_code must be a non-empty safe token".into(),
        ));
    }
    Ok(normalized)
}

fn clamp_review_rating(rating: i16) -> i16 {
    rating.clamp(0, 5)
}

fn normalize_review_status(status: &str) -> AtelierResult<String> {
    let normalized = status.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(AtelierError::Validation(
            "review_status must not be empty".into(),
        ));
    }
    match normalized.as_str() {
        "unreviewed" | "review" | "approved" | "rejected" | "deferred" => Ok(normalized),
        _ => Err(AtelierError::Validation(format!(
            "unsupported review_status: {status}"
        ))),
    }
}

fn normalize_review_notes(notes: &Option<String>) -> AtelierResult<Option<String>> {
    Ok(notes.clone())
}

fn normalize_review_metadata_update(
    update: &MediaReviewMetadataUpdate,
) -> AtelierResult<NormalizedMediaReviewMetadataUpdate> {
    let notes = normalize_review_notes(&update.notes)?;
    Ok(NormalizedMediaReviewMetadataUpdate {
        asset_id: update.asset_id,
        favorite: update.favorite,
        rating: clamp_review_rating(update.rating),
        frontpage: update.frontpage,
        carousel: update.carousel,
        notes_ref: notes.as_deref().map(sha256_ref),
        notes,
        review_status: normalize_review_status(&update.review_status)?,
    })
}

fn event_safe_media_artifact_manifest(
    manifest: &serde_json::Value,
    source_provenance: &str,
) -> serde_json::Value {
    let mut safe = manifest.clone();
    if let Some(object) = safe.as_object_mut() {
        object.remove("source");
        object.remove("source_provenance");
        object.insert(
            "source_provenance_ref".to_string(),
            serde_json::Value::String(sha256_ref(source_provenance)),
        );
    }
    safe
}

fn build_media_artifact_manifest_from_parts(
    asset_id: Uuid,
    artifact_ref: &str,
    content_hash: &str,
    mime: &str,
    byte_len: i64,
    source_provenance: &str,
    retention_class: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema": MEDIA_ARTIFACT_MANIFEST_SCHEMA,
        "asset_id": asset_id,
        "artifact_ref": artifact_ref,
        "content_hash": content_hash,
        "mime": mime,
        "byte_len": byte_len,
        "size_bytes": byte_len,
        "source_provenance_ref": sha256_ref(source_provenance),
        "retention_class": retention_class,
        "artifact_store": {
            "handle": artifact_ref,
            "content_hash": content_hash,
            "size_bytes": byte_len,
            "retention_class": retention_class,
        },
    })
}

fn build_media_artifact_manifest(
    asset_id: Uuid,
    new: &NewMediaAsset,
    content_hash: &str,
    source_provenance: &str,
) -> serde_json::Value {
    build_media_artifact_manifest_from_parts(
        asset_id,
        &new.artifact_ref,
        content_hash,
        &new.mime,
        new.byte_len,
        source_provenance,
        MEDIA_ORIGINAL_RETENTION_CLASS,
    )
}

fn source_from_media_asset(asset: &MediaAsset) -> &str {
    asset
        .source_provenance
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("legacy:unknown")
}

fn retention_class_from_media_asset(asset: &MediaAsset) -> &str {
    if asset.retention_class.trim().is_empty() {
        MEDIA_ORIGINAL_RETENTION_CLASS
    } else {
        asset.retention_class.as_str()
    }
}

fn build_invalid_media_manifest(
    asset: &MediaAsset,
    source: &str,
    retention_class: &str,
    validation_state: &str,
    reason: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema": MEDIA_ARTIFACT_MANIFEST_SCHEMA,
        "asset_id": asset.asset_id,
        "content_hash": asset.content_hash,
        "mime": asset.mime,
        "byte_len": asset.byte_len,
        "size_bytes": asset.byte_len,
        "source_provenance_ref": sha256_ref(source),
        "retention_class": retention_class,
        "validation_state": validation_state,
        "artifact_store": {
            "status": "unresolved",
            "reason": reason,
        },
    })
}

fn build_invalid_legacy_media_manifest(
    asset: &MediaAsset,
    source: &str,
    retention_class: &str,
) -> serde_json::Value {
    build_invalid_media_manifest(
        asset,
        source,
        retention_class,
        INVALID_LEGACY_ARTIFACT_REF_STATE,
        INVALID_LEGACY_ARTIFACT_REF_REASON,
    )
}

fn build_invalid_artifact_store_binding_manifest(
    asset: &MediaAsset,
    source: &str,
    retention_class: &str,
) -> serde_json::Value {
    build_invalid_media_manifest(
        asset,
        source,
        retention_class,
        INVALID_ARTIFACT_STORE_BINDING_STATE,
        INVALID_ARTIFACT_STORE_BINDING_REASON,
    )
}

fn parse_native_artifact_payload_ref(artifact_ref: &str) -> AtelierResult<(ArtifactLayer, Uuid)> {
    let body = artifact_ref.strip_prefix("artifact://").ok_or_else(|| {
        AtelierError::Validation("artifact_ref missing artifact:// scheme".into())
    })?;
    let parts: Vec<&str> = body.split('/').collect();
    if parts.len() != 5
        || parts[0] != ".handshake"
        || parts[1] != "artifacts"
        || parts[4] != "payload"
    {
        return Err(AtelierError::Validation(
            "media artifact_ref must point to a native ArtifactStore payload (artifact://.handshake/artifacts/<layer>/<uuid>/payload)".into(),
        ));
    }
    let layer = match parts[2] {
        "L1" => ArtifactLayer::L1,
        "L2" => ArtifactLayer::L2,
        "L3" => ArtifactLayer::L3,
        "L4" => ArtifactLayer::L4,
        other => {
            return Err(AtelierError::Validation(format!(
                "unsupported ArtifactStore layer in media artifact_ref: {other}"
            )));
        }
    };
    let artifact_id = Uuid::parse_str(parts[3]).map_err(|err| {
        AtelierError::Validation(format!("invalid ArtifactStore artifact id: {err}"))
    })?;
    Ok((layer, artifact_id))
}

fn is_native_artifact_payload_ref(artifact_ref: &str) -> bool {
    parse_native_artifact_payload_ref(artifact_ref).is_ok()
        && !artifact_ref.to_ascii_lowercase().contains(".gov")
}

fn has_valid_row_hash(content_hash: &str) -> bool {
    validate_sha256_content_hash(content_hash).is_ok()
}

fn has_valid_row_mime(mime: &str) -> bool {
    let trimmed = mime.trim();
    !trimmed.is_empty() && trimmed == mime
}

fn has_valid_row_retention_class(retention_class: &str) -> bool {
    let trimmed = retention_class.trim();
    !trimmed.is_empty() && trimmed == retention_class
}

fn asset_can_have_full_artifact_manifest(asset: &MediaAsset) -> bool {
    is_native_artifact_payload_ref(&asset.artifact_ref)
        && has_valid_row_hash(&asset.content_hash)
        && asset.byte_len > 0
        && has_valid_row_mime(&asset.mime)
        && has_valid_row_retention_class(retention_class_from_media_asset(asset))
}

fn resolve_media_artifact_root() -> AtelierResult<PathBuf> {
    resolve_workspace_root()
        .map_err(|err| AtelierError::Validation(format!("ArtifactStore root unavailable: {err}")))
}

fn verify_artifact_store_binding(new: &NewMediaAsset) -> AtelierResult<()> {
    let (layer, artifact_id) = parse_native_artifact_payload_ref(&new.artifact_ref)?;
    let workspace_root = resolve_media_artifact_root()?;
    let manifest = read_artifact_manifest(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!("ArtifactStore manifest validation failed: {err}"))
    })?;
    validate_artifact_content_hash(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!(
            "ArtifactStore content hash validation failed: {err}"
        ))
    })?;
    if manifest.artifact_id != artifact_id || manifest.layer != layer {
        return Err(AtelierError::Validation(
            "media ArtifactStore manifest identity mismatch".into(),
        ));
    }
    let requested_hash = normalized_sha256_hex(&new.content_hash)?;
    if !manifest.content_hash.eq_ignore_ascii_case(requested_hash) {
        return Err(AtelierError::Validation(
            "media content_hash does not match ArtifactStore manifest".into(),
        ));
    }
    if manifest.size_bytes != new.byte_len as u64 {
        return Err(AtelierError::Validation(
            "media byte_len does not match ArtifactStore manifest".into(),
        ));
    }
    if manifest.mime != new.mime {
        return Err(AtelierError::Validation(
            "media mime does not match ArtifactStore manifest".into(),
        ));
    }
    Ok(())
}

fn verify_media_asset_artifact_store_binding(asset: &MediaAsset) -> AtelierResult<()> {
    let (layer, artifact_id) = parse_native_artifact_payload_ref(&asset.artifact_ref)?;
    let workspace_root = resolve_media_artifact_root()?;
    let manifest = read_artifact_manifest(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!("ArtifactStore manifest validation failed: {err}"))
    })?;
    validate_artifact_content_hash(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!(
            "ArtifactStore content hash validation failed: {err}"
        ))
    })?;
    if manifest.artifact_id != artifact_id || manifest.layer != layer {
        return Err(AtelierError::Validation(
            "media ArtifactStore manifest identity mismatch".into(),
        ));
    }
    let requested_hash = normalized_sha256_hex(&asset.content_hash)?;
    if !manifest.content_hash.eq_ignore_ascii_case(requested_hash) {
        return Err(AtelierError::Validation(
            "media content_hash does not match ArtifactStore manifest".into(),
        ));
    }
    if manifest.size_bytes != asset.byte_len as u64 {
        return Err(AtelierError::Validation(
            "media byte_len does not match ArtifactStore manifest".into(),
        ));
    }
    if manifest.mime != asset.mime {
        return Err(AtelierError::Validation(
            "media mime does not match ArtifactStore manifest".into(),
        ));
    }
    Ok(())
}

fn expected_artifact_manifest_ref(layer: ArtifactLayer, artifact_id: Uuid) -> String {
    format!(
        "artifact://{}/artifact.json",
        artifact_root_rel(layer, artifact_id)
    )
}

fn resolve_derivative_artifact_root() -> AtelierResult<PathBuf> {
    resolve_workspace_root()
        .map_err(|err| AtelierError::Validation(format!("ArtifactStore root unavailable: {err}")))
}

fn verify_derivative_artifact_binding(
    generated: &MediaDerivativeGenerated,
    normalized_mime: &str,
) -> AtelierResult<(ArtifactLayer, Uuid)> {
    let (layer, artifact_id) = parse_native_artifact_payload_ref(&generated.artifact_ref)?;
    let expected_manifest_ref = expected_artifact_manifest_ref(layer, artifact_id);
    if generated.artifact_manifest_ref != expected_manifest_ref {
        return Err(AtelierError::Validation(format!(
            "media derivative artifact_manifest_ref must point to the same ArtifactStore artifact manifest: expected {expected_manifest_ref}"
        )));
    }
    let workspace_root = resolve_derivative_artifact_root()?;
    let manifest = read_artifact_manifest(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!("ArtifactStore manifest validation failed: {err}"))
    })?;
    validate_artifact_content_hash(&workspace_root, layer, artifact_id).map_err(|err| {
        AtelierError::Validation(format!(
            "ArtifactStore content hash validation failed: {err}"
        ))
    })?;
    if manifest.artifact_id != artifact_id || manifest.layer != layer {
        return Err(AtelierError::Validation(
            "media derivative ArtifactStore manifest identity mismatch".into(),
        ));
    }
    if manifest.size_bytes != generated.byte_len as u64 {
        return Err(AtelierError::Validation(
            "media derivative byte_len does not match ArtifactStore manifest".into(),
        ));
    }
    if manifest.mime != normalized_mime {
        return Err(AtelierError::Validation(
            "media derivative mime does not match ArtifactStore manifest".into(),
        ));
    }
    Ok((layer, artifact_id))
}

impl AtelierStore {
    /// Materialize a media asset, deduplicating on `content_hash`. Re-ingesting
    /// identical bytes returns the existing asset (idempotent) rather than
    /// creating a duplicate row.
    pub async fn materialize_media_asset(&self, new: &NewMediaAsset) -> AtelierResult<MediaAsset> {
        let content_hash = canonical_sha256_content_hash(&new.content_hash)?;
        if new.byte_len <= 0 {
            return Err(AtelierError::Validation(
                "byte_len must be greater than zero".into(),
            ));
        }
        let mime = new.mime.trim();
        if mime.is_empty() || mime != new.mime {
            return Err(AtelierError::Validation(
                "mime must not be empty or padded".into(),
            ));
        }
        validate_artifact_ref(&new.artifact_ref)?;
        verify_artifact_store_binding(new)?;
        let source_provenance = validated_source_provenance(&new.source_provenance)?;

        if let Some(existing) = self.get_media_asset_by_hash(&content_hash).await? {
            let existing_binding_valid = is_native_artifact_payload_ref(&existing.artifact_ref)
                && verify_media_asset_artifact_store_binding(&existing).is_ok();
            if !existing_binding_valid {
                return self
                    .upgrade_media_asset_to_native_manifest(
                        existing,
                        new,
                        &content_hash,
                        source_provenance,
                    )
                    .await;
            }
            return self.repair_media_asset_manifest_if_needed(existing).await;
        }

        let asset_id = Uuid::now_v7();
        let artifact_manifest =
            build_media_artifact_manifest(asset_id, new, &content_hash, source_provenance);
        let mut tx = self.pool().begin().await?;
        let inserted = sqlx::query(
            r#"INSERT INTO atelier_media_asset
                 (asset_id, content_hash, mime, byte_len, source_provenance,
                  artifact_ref, retention_class, artifact_manifest)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (content_hash) DO NOTHING
               RETURNING asset_id, content_hash, mime, byte_len, source_provenance,
                         artifact_ref, retention_class, artifact_manifest, created_at_utc"#,
        )
        .bind(asset_id)
        .bind(&content_hash)
        .bind(&new.mime)
        .bind(new.byte_len)
        .bind(&new.source_provenance)
        .bind(&new.artifact_ref)
        .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
        .bind(artifact_manifest)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = inserted else {
            tx.commit().await?;
            let existing = self
                .get_media_asset_by_hash(&content_hash)
                .await?
                .ok_or_else(|| {
                    AtelierError::NotFound(format!(
                        "media content_hash={} after conflict",
                        content_hash
                    ))
                })?;
            return self.repair_media_asset_manifest_if_needed(existing).await;
        };

        let asset = asset_from_row(&row);
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_ASSET_MATERIALIZED,
            "atelier_media_asset",
            &asset.content_hash,
            serde_json::json!({
                "asset_id": asset.asset_id,
                "mime": asset.mime,
                "byte_len": asset.byte_len,
                "artifact_ref": asset.artifact_ref,
                "retention_class": asset.retention_class,
                "artifact_manifest": event_safe_media_artifact_manifest(
                    &asset.artifact_manifest,
                    source_provenance,
                ),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(asset)
    }

    pub async fn set_media_source_provenance_refs(
        &self,
        update: &SetMediaSourceProvenanceRefs,
    ) -> AtelierResult<MediaSourceProvenanceRefs> {
        let [source_url_ref, source_path_ref, source_note_ref, contact_sheet_ref, task_ref, run_ref] =
            validate_media_source_provenance_refs(update)?;
        let mut tx = self.pool().begin().await?;
        let asset_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM atelier_media_asset WHERE asset_id = $1)",
        )
        .bind(update.asset_id)
        .fetch_one(&mut *tx)
        .await?;
        if !asset_exists {
            return Err(AtelierError::NotFound(format!(
                "media asset_id={}",
                update.asset_id
            )));
        }

        let row = sqlx::query(
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
                   updated_at_utc = NOW()
               RETURNING asset_id, source_url_ref, source_path_ref, source_note_ref,
                         contact_sheet_ref, task_ref, run_ref, updated_by, updated_at_utc"#,
        )
        .bind(update.asset_id)
        .bind(&source_url_ref)
        .bind(&source_path_ref)
        .bind(&source_note_ref)
        .bind(&contact_sheet_ref)
        .bind(&task_ref)
        .bind(&run_ref)
        .bind(&update.updated_by)
        .fetch_one(&mut *tx)
        .await?;
        let refs = media_source_provenance_refs_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_SOURCE_PROVENANCE_REFS_SET,
            "atelier_media_asset",
            &refs.asset_id.to_string(),
            serde_json::json!({
                "asset_id": refs.asset_id,
                "source_url_ref": refs.source_url_ref,
                "source_path_ref": refs.source_path_ref,
                "source_note_ref": refs.source_note_ref,
                "contact_sheet_ref": refs.contact_sheet_ref,
                "task_ref": refs.task_ref,
                "run_ref": refs.run_ref,
                "updated_by": refs.updated_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(refs)
    }

    pub async fn get_media_source_provenance_refs(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<Option<MediaSourceProvenanceRefs>> {
        let row = sqlx::query(
            r#"SELECT asset_id, source_url_ref, source_path_ref, source_note_ref,
                      contact_sheet_ref, task_ref, run_ref, updated_by, updated_at_utc
               FROM atelier_media_source_provenance_ref
               WHERE asset_id = $1"#,
        )
        .bind(asset_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(media_source_provenance_refs_from_row))
    }

    async fn upgrade_media_asset_to_native_manifest(
        &self,
        existing: MediaAsset,
        new: &NewMediaAsset,
        content_hash: &str,
        source_provenance: &str,
    ) -> AtelierResult<MediaAsset> {
        let manifest = build_media_artifact_manifest_from_parts(
            existing.asset_id,
            &new.artifact_ref,
            content_hash,
            &new.mime,
            new.byte_len,
            source_provenance,
            MEDIA_ORIGINAL_RETENTION_CLASS,
        );
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_media_asset
               SET mime = $2,
                   byte_len = $3,
                   source_provenance = $4,
                   artifact_ref = $5,
                   retention_class = $6,
                   artifact_manifest = $7
               WHERE asset_id = $1
                 AND artifact_ref = $8
                 AND (
                       artifact_ref IS DISTINCT FROM $5
                    OR mime IS DISTINCT FROM $2
                    OR byte_len IS DISTINCT FROM $3
                    OR source_provenance IS DISTINCT FROM $4
                    OR retention_class IS DISTINCT FROM $6
                    OR artifact_manifest IS DISTINCT FROM $7
                 )
               RETURNING asset_id, content_hash, mime, byte_len, source_provenance,
                         artifact_ref, retention_class, artifact_manifest, created_at_utc"#,
        )
        .bind(existing.asset_id)
        .bind(&new.mime)
        .bind(new.byte_len)
        .bind(&new.source_provenance)
        .bind(&new.artifact_ref)
        .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
        .bind(manifest)
        .bind(&existing.artifact_ref)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            tx.commit().await?;
            let existing = self
                .get_media_asset_by_hash(content_hash)
                .await?
                .ok_or_else(|| {
                    AtelierError::NotFound(format!(
                        "media content_hash={} after legacy upgrade race",
                        content_hash
                    ))
                })?;
            return self.repair_media_asset_manifest_if_needed(existing).await;
        };
        let asset = asset_from_row(&row);
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_ASSET_MATERIALIZED,
            "atelier_media_asset",
            &asset.content_hash,
            serde_json::json!({
                "asset_id": asset.asset_id,
                "mime": asset.mime,
                "byte_len": asset.byte_len,
                "artifact_ref": asset.artifact_ref,
                "retention_class": asset.retention_class,
                "artifact_manifest": event_safe_media_artifact_manifest(
                    &asset.artifact_manifest,
                    source_provenance,
                ),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(asset)
    }

    async fn repair_media_asset_manifest_if_needed(
        &self,
        asset: MediaAsset,
    ) -> AtelierResult<MediaAsset> {
        let source = source_from_media_asset(&asset);
        let retention_class = retention_class_from_media_asset(&asset);
        let manifest = if asset_can_have_full_artifact_manifest(&asset)
            && verify_media_asset_artifact_store_binding(&asset).is_ok()
        {
            build_media_artifact_manifest_from_parts(
                asset.asset_id,
                &asset.artifact_ref,
                &asset.content_hash,
                &asset.mime,
                asset.byte_len,
                source,
                retention_class,
            )
        } else if is_native_artifact_payload_ref(&asset.artifact_ref)
            && has_valid_row_hash(&asset.content_hash)
            && asset.byte_len > 0
            && has_valid_row_mime(&asset.mime)
            && has_valid_row_retention_class(retention_class)
        {
            build_invalid_artifact_store_binding_manifest(&asset, source, retention_class)
        } else {
            build_invalid_legacy_media_manifest(&asset, source, retention_class)
        };
        if asset.artifact_manifest == manifest && asset.retention_class == retention_class {
            return Ok(asset);
        }
        let row = sqlx::query(
            r#"UPDATE atelier_media_asset
               SET retention_class = $2,
                   artifact_manifest = $3
               WHERE asset_id = $1
               RETURNING asset_id, content_hash, mime, byte_len, source_provenance,
                         artifact_ref, retention_class, artifact_manifest, created_at_utc"#,
        )
        .bind(asset.asset_id)
        .bind(retention_class)
        .bind(manifest)
        .fetch_one(self.pool())
        .await?;
        Ok(asset_from_row(&row))
    }

    pub(crate) async fn repair_media_asset_artifact_manifests(&self) -> AtelierResult<()> {
        let table_exists: bool =
            sqlx::query_scalar("SELECT to_regclass('atelier_media_asset') IS NOT NULL")
                .fetch_one(self.pool())
                .await?;
        if !table_exists {
            return Ok(());
        }

        let columns_ready: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                   SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'atelier_media_asset'
                     AND column_name = 'artifact_manifest'
                     AND table_schema = ANY(current_schemas(false))
               )
               AND EXISTS (
                   SELECT 1 FROM information_schema.columns
                   WHERE table_name = 'atelier_media_asset'
                     AND column_name = 'retention_class'
                     AND table_schema = ANY(current_schemas(false))
               )"#,
        )
        .fetch_one(self.pool())
        .await?;
        if !columns_ready {
            return Ok(());
        }

        sqlx::query(
            r#"UPDATE atelier_media_asset
               SET retention_class = 'atelier.media.original.retained'
               WHERE retention_class IS NULL
                  OR btrim(retention_class) = ''"#,
        )
        .execute(self.pool())
        .await?;

        let rows = sqlx::query(
            r#"SELECT asset_id, content_hash, mime, byte_len, source_provenance,
                      artifact_ref, retention_class, artifact_manifest, created_at_utc
               FROM atelier_media_asset
               WHERE artifact_ref ~ '^artifact://\.handshake/artifacts/L[1-4]/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/payload$'
                  OR artifact_manifest = '{}'::jsonb
                  OR artifact_manifest->>'schema' IS DISTINCT FROM $1
                  OR artifact_manifest->>'asset_id' IS DISTINCT FROM asset_id::text
                  OR artifact_manifest->>'content_hash' IS DISTINCT FROM content_hash
                  OR artifact_manifest->>'mime' IS DISTINCT FROM mime
                  OR artifact_manifest->>'byte_len' IS DISTINCT FROM byte_len::text
                  OR artifact_manifest->>'size_bytes' IS DISTINCT FROM byte_len::text
                  OR artifact_manifest ? 'source_provenance'
                  OR artifact_manifest ? 'source'
                  OR artifact_manifest->>'source_provenance_ref' IS DISTINCT FROM
                     ('sha256:' || encode(sha256(convert_to(COALESCE(NULLIF(btrim(source_provenance), ''), 'legacy:unknown'), 'UTF8')), 'hex'))
                  OR artifact_manifest->>'retention_class' IS DISTINCT FROM retention_class"#,
        )
        .bind(MEDIA_ARTIFACT_MANIFEST_SCHEMA)
        .fetch_all(self.pool())
        .await?;
        for row in rows {
            self.repair_media_asset_manifest_if_needed(asset_from_row(&row))
                .await?;
        }
        Ok(())
    }

    pub async fn get_media_asset_by_hash(
        &self,
        content_hash: &str,
    ) -> AtelierResult<Option<MediaAsset>> {
        let row = sqlx::query(
            r#"SELECT asset_id, content_hash, mime, byte_len, source_provenance,
                      artifact_ref, retention_class, artifact_manifest, created_at_utc
               FROM atelier_media_asset WHERE content_hash = $1"#,
        )
        .bind(content_hash)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(asset_from_row))
    }

    pub async fn get_media_artifact_manifest(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<serde_json::Value> {
        let manifest = sqlx::query_scalar(
            "SELECT artifact_manifest FROM atelier_media_asset WHERE asset_id = $1",
        )
        .bind(asset_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("media asset_id={asset_id}")))?;
        Ok(manifest)
    }

    pub async fn record_media_sidecar_relation(
        &self,
        new: &NewMediaSidecarRelation,
    ) -> AtelierResult<MediaSidecar> {
        if new.parent_asset_id == new.sidecar_asset_id {
            return Err(AtelierError::Validation(
                "media sidecar cannot reference itself as parent".into(),
            ));
        }
        let created_by = require_sidecar_actor(&new.created_by)?;
        let mut tx = self.pool().begin().await?;
        let asset_ids = vec![new.parent_asset_id, new.sidecar_asset_id];
        let existing: Vec<Uuid> =
            sqlx::query_scalar("SELECT asset_id FROM atelier_media_asset WHERE asset_id = ANY($1)")
                .bind(&asset_ids)
                .fetch_all(&mut *tx)
                .await?;
        if existing.len() != asset_ids.len() {
            let existing: HashSet<Uuid> = existing.into_iter().collect();
            let missing: Vec<String> = asset_ids
                .iter()
                .filter(|asset_id| !existing.contains(asset_id))
                .map(Uuid::to_string)
                .collect();
            return Err(AtelierError::NotFound(format!(
                "media sidecar relation assets missing: {}",
                missing.join(", ")
            )));
        }

        let sidecar_id = Uuid::now_v7();
        let row = sqlx::query(
            r#"INSERT INTO atelier_media_sidecar (
                   sidecar_id, parent_asset_id, sidecar_asset_id, relation_kind,
                   hidden_from_gallery, searchable_by_relation, created_by,
                   updated_at_utc
               )
               VALUES ($1, $2, $3, $4, TRUE, TRUE, $5, NOW())
               ON CONFLICT (parent_asset_id, sidecar_asset_id, relation_kind)
               DO UPDATE SET
                   hidden_from_gallery = TRUE,
                   searchable_by_relation = TRUE,
                   created_by = EXCLUDED.created_by,
                   updated_at_utc = NOW()
               RETURNING sidecar_id, parent_asset_id, sidecar_asset_id, relation_kind,
                         hidden_from_gallery, searchable_by_relation, created_by,
                         created_at_utc, updated_at_utc"#,
        )
        .bind(sidecar_id)
        .bind(new.parent_asset_id)
        .bind(new.sidecar_asset_id)
        .bind(new.relation_kind.as_token())
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;
        let sidecar = media_sidecar_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_SIDECAR_RECORDED,
            "atelier_media_sidecar",
            &sidecar.sidecar_id.to_string(),
            serde_json::json!({
                "sidecar_id": sidecar.sidecar_id,
                "parent_asset_id": sidecar.parent_asset_id,
                "sidecar_asset_id": sidecar.sidecar_asset_id,
                "relation_kind": sidecar.relation_kind.as_token(),
                "hidden_from_gallery": sidecar.hidden_from_gallery,
                "searchable_by_relation": sidecar.searchable_by_relation,
                "created_by": created_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(sidecar)
    }

    pub async fn list_media_sidecars_for_asset(
        &self,
        parent_asset_id: Uuid,
        relation_kind: Option<MediaSidecarRelationKind>,
    ) -> AtelierResult<Vec<MediaSidecar>> {
        let relation_kind = relation_kind.map(MediaSidecarRelationKind::as_token);
        let rows = sqlx::query(
            r#"SELECT sidecar_id, parent_asset_id, sidecar_asset_id, relation_kind,
                      hidden_from_gallery, searchable_by_relation, created_by,
                      created_at_utc, updated_at_utc
               FROM atelier_media_sidecar
               WHERE parent_asset_id = $1
                 AND searchable_by_relation
                 AND ($2::text IS NULL OR relation_kind = $2)
               ORDER BY relation_kind, updated_at_utc DESC, sidecar_id"#,
        )
        .bind(parent_asset_id)
        .bind(relation_kind)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(media_sidecar_from_row).collect()
    }

    pub async fn list_media_gallery_assets(&self, limit: i64) -> AtelierResult<Vec<MediaAsset>> {
        if !(1..=500).contains(&limit) {
            return Err(AtelierError::Validation(
                "media gallery limit must be between 1 and 500".into(),
            ));
        }
        let rows = sqlx::query(
            r#"SELECT a.asset_id, a.content_hash, a.mime, a.byte_len, a.source_provenance,
                      a.artifact_ref, a.retention_class, a.artifact_manifest, a.created_at_utc
               FROM atelier_media_asset a
               WHERE NOT EXISTS (
                   SELECT 1
                   FROM atelier_media_sidecar s
                   WHERE s.sidecar_asset_id = a.asset_id
                     AND s.hidden_from_gallery
               )
                 AND NOT EXISTS (
                   SELECT 1
                   FROM atelier_trash_marker t
                   WHERE t.target_type = 'media_asset'
                     AND t.target_id = a.asset_id
               )
               ORDER BY a.created_at_utc DESC, a.asset_id DESC
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(asset_from_row).collect())
    }

    pub async fn request_media_derivative(
        &self,
        request: &MediaDerivativeRequest,
    ) -> AtelierResult<MediaDerivative> {
        validate_derivative_dimensions(request.target_width, request.target_height)?;
        let format = normalize_derivative_format(&request.format)?;
        let requested_by = require_derivative_actor("requested_by", &request.requested_by)?;
        let mut tx = self.pool().begin().await?;
        let asset_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM atelier_media_asset WHERE asset_id = $1)",
        )
        .bind(request.asset_id)
        .fetch_one(&mut *tx)
        .await?;
        if !asset_exists {
            return Err(AtelierError::NotFound(format!(
                "media asset_id={}",
                request.asset_id
            )));
        }

        let derivative_id = Uuid::now_v7();
        let inserted_row = sqlx::query(
            r#"INSERT INTO atelier_media_derivative (
                   derivative_id, asset_id, derivative_kind, target_width,
                   target_height, format, status, requested_by, updated_by
               )
               VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $7)
               ON CONFLICT (asset_id, derivative_kind, target_width, target_height, format)
               DO NOTHING
               RETURNING derivative_id, asset_id, derivative_kind, target_width,
                         target_height, format, status, artifact_ref,
                         artifact_manifest_ref, mime, byte_len, requested_by,
                         updated_by, attempt_count, retry_count, last_error_code,
                         last_error_ref, created_at_utc, updated_at_utc"#,
        )
        .bind(derivative_id)
        .bind(request.asset_id)
        .bind(request.derivative_kind.as_token())
        .bind(request.target_width)
        .bind(request.target_height)
        .bind(&format)
        .bind(requested_by)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = inserted_row else {
            let row = sqlx::query(
                r#"SELECT derivative_id, asset_id, derivative_kind, target_width,
                          target_height, format, status, artifact_ref,
                          artifact_manifest_ref, mime, byte_len, requested_by,
                          updated_by, attempt_count, retry_count, last_error_code,
                          last_error_ref, created_at_utc, updated_at_utc
                   FROM atelier_media_derivative
                   WHERE asset_id = $1
                     AND derivative_kind = $2
                     AND target_width = $3
                     AND target_height = $4
                     AND format = $5"#,
            )
            .bind(request.asset_id)
            .bind(request.derivative_kind.as_token())
            .bind(request.target_width)
            .bind(request.target_height)
            .bind(&format)
            .fetch_one(&mut *tx)
            .await?;
            let derivative = media_derivative_from_row(&row)?;
            tx.commit().await?;
            return Ok(derivative);
        };
        let derivative = media_derivative_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_DERIVATIVE_REQUESTED,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
            serde_json::json!({
                "derivative_id": derivative.derivative_id,
                "asset_id": derivative.asset_id,
                "derivative_kind": derivative.derivative_kind.as_token(),
                "target_width": derivative.target_width,
                "target_height": derivative.target_height,
                "format": derivative.format,
                "status": derivative.status.as_token(),
                "requested_by": requested_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(derivative)
    }

    pub async fn list_media_derivatives(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<Vec<MediaDerivative>> {
        let rows = sqlx::query(
            r#"SELECT derivative_id, asset_id, derivative_kind, target_width,
                      target_height, format, status, artifact_ref,
                      artifact_manifest_ref, mime, byte_len, requested_by,
                      updated_by, attempt_count, retry_count, last_error_code,
                      last_error_ref, created_at_utc, updated_at_utc
               FROM atelier_media_derivative
               WHERE asset_id = $1
               ORDER BY derivative_kind, target_width, target_height, format"#,
        )
        .bind(asset_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(media_derivative_from_row).collect()
    }

    pub async fn mark_media_derivative_generating(
        &self,
        derivative_id: Uuid,
        updated_by: &str,
    ) -> AtelierResult<MediaDerivative> {
        let updated_by = require_derivative_actor("updated_by", updated_by)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_media_derivative
               SET status = 'generating',
                   updated_by = $2,
                   updated_at_utc = NOW()
               WHERE derivative_id = $1
                 AND status = 'pending'
               RETURNING derivative_id, asset_id, derivative_kind, target_width,
                         target_height, format, status, artifact_ref,
                         artifact_manifest_ref, mime, byte_len, requested_by,
                         updated_by, attempt_count, retry_count, last_error_code,
                         last_error_ref, created_at_utc, updated_at_utc"#,
        )
        .bind(derivative_id)
        .bind(updated_by)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AtelierError::Validation(format!(
                "media derivative {derivative_id} is not pending; retryable derivatives must be retried first"
            ))
        })?;
        let derivative = media_derivative_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_DERIVATIVE_GENERATING,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
            serde_json::json!({
                "derivative_id": derivative.derivative_id,
                "asset_id": derivative.asset_id,
                "derivative_kind": derivative.derivative_kind.as_token(),
                "status": derivative.status.as_token(),
                "updated_by": updated_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(derivative)
    }

    pub async fn record_media_derivative_generated(
        &self,
        derivative_id: Uuid,
        artifact_ref: &str,
        artifact_manifest_ref: &str,
        mime: &str,
        byte_len: i64,
        updated_by: &str,
    ) -> AtelierResult<MediaDerivative> {
        self.record_media_derivative_generated_with_artifact(&MediaDerivativeGenerated {
            derivative_id,
            artifact_ref: artifact_ref.to_string(),
            artifact_manifest_ref: artifact_manifest_ref.to_string(),
            mime: mime.to_string(),
            byte_len,
            updated_by: updated_by.to_string(),
        })
        .await
    }

    pub async fn record_media_derivative_generated_with_artifact(
        &self,
        generated: &MediaDerivativeGenerated,
    ) -> AtelierResult<MediaDerivative> {
        validate_artifact_ref(&generated.artifact_ref)?;
        if generated.artifact_manifest_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "artifact_manifest_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_manifest_ref", &generated.artifact_manifest_ref)?;
        let mime = normalize_derivative_mime(&generated.mime)?;
        if generated.byte_len <= 0 {
            return Err(AtelierError::Validation(
                "media derivative byte_len must be greater than zero".into(),
            ));
        }
        verify_derivative_artifact_binding(generated, &mime)?;
        let updated_by = require_derivative_actor("updated_by", &generated.updated_by)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_media_derivative
               SET status = 'generated',
                   artifact_ref = $2,
                   artifact_manifest_ref = $3,
                   mime = $4,
                   byte_len = $5,
                   updated_by = $6,
                   last_error_code = NULL,
                   last_error_ref = NULL,
                   updated_at_utc = NOW()
               WHERE derivative_id = $1
                 AND status = 'generating'
                 AND (
                   (format = 'png' AND $4 = 'image/png')
                   OR (format = 'jpeg' AND $4 = 'image/jpeg')
                 )
               RETURNING derivative_id, asset_id, derivative_kind, target_width,
                         target_height, format, status, artifact_ref,
                         artifact_manifest_ref, mime, byte_len, requested_by,
                         updated_by, attempt_count, retry_count, last_error_code,
                         last_error_ref, created_at_utc, updated_at_utc"#,
        )
        .bind(generated.derivative_id)
        .bind(&generated.artifact_ref)
        .bind(&generated.artifact_manifest_ref)
        .bind(mime)
        .bind(generated.byte_len)
        .bind(updated_by)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            let current = sqlx::query(
                "SELECT status, format FROM atelier_media_derivative WHERE derivative_id = $1",
            )
            .bind(generated.derivative_id)
            .fetch_optional(&mut *tx)
            .await?;
            return match current {
                None => Err(AtelierError::NotFound(format!(
                    "media derivative_id={}",
                    generated.derivative_id
                ))),
                Some(row) => {
                    let status: String = row.get("status");
                    let format: String = row.get("format");
                    if status == "generating" {
                        let expected_mime = expected_mime_for_derivative_format(&format)?;
                        if expected_mime != mime {
                            return Err(AtelierError::Validation(format!(
                                "media derivative format {format} requires mime {expected_mime}, got {mime}"
                            )));
                        }
                    }
                    Err(AtelierError::Validation(format!(
                        "media derivative {} is not active for generated transition (status={status})",
                        generated.derivative_id
                    )))
                }
            };
        };
        let derivative = media_derivative_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_DERIVATIVE_GENERATED,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
            serde_json::json!({
                "derivative_id": derivative.derivative_id,
                "asset_id": derivative.asset_id,
                "derivative_kind": derivative.derivative_kind.as_token(),
                "target_width": derivative.target_width,
                "target_height": derivative.target_height,
                "format": derivative.format,
                "status": derivative.status.as_token(),
                "artifact_ref": generated.artifact_ref,
                "artifact_manifest_ref": generated.artifact_manifest_ref,
                "mime": mime,
                "byte_len": generated.byte_len,
                "updated_by": updated_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(derivative)
    }

    pub async fn record_media_derivative_failure(
        &self,
        derivative_id: Uuid,
        failure: &MediaDerivativeFailure,
    ) -> AtelierResult<MediaDerivative> {
        let error_code = normalize_error_code(&failure.error_code)?;
        let error_detail = failure.error_detail.trim();
        if error_detail.is_empty() {
            return Err(AtelierError::Validation(
                "media derivative error_detail must not be empty".into(),
            ));
        }
        let error_ref = sha256_ref(error_detail);
        let updated_by = require_derivative_actor("updated_by", &failure.updated_by)?;
        let status = if failure.retryable {
            MediaDerivativeStatus::RetryableError
        } else {
            MediaDerivativeStatus::Failed
        };
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_media_derivative
               SET status = $2,
                   updated_by = $3,
                   attempt_count = attempt_count + 1,
                   last_error_code = $4,
                   last_error_ref = $5,
                   artifact_ref = NULL,
                   artifact_manifest_ref = NULL,
                   mime = NULL,
                   byte_len = NULL,
                   updated_at_utc = NOW()
               WHERE derivative_id = $1
                 AND status IN ('pending', 'generating')
               RETURNING derivative_id, asset_id, derivative_kind, target_width,
                         target_height, format, status, artifact_ref,
                         artifact_manifest_ref, mime, byte_len, requested_by,
                         updated_by, attempt_count, retry_count, last_error_code,
                         last_error_ref, created_at_utc, updated_at_utc"#,
        )
        .bind(derivative_id)
        .bind(status.as_token())
        .bind(updated_by)
        .bind(&error_code)
        .bind(&error_ref)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            let current_status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM atelier_media_derivative WHERE derivative_id = $1",
            )
            .bind(derivative_id)
            .fetch_optional(&mut *tx)
            .await?;
            return match current_status {
                None => Err(AtelierError::NotFound(format!(
                    "media derivative_id={derivative_id}"
                ))),
                Some(status) => Err(AtelierError::Validation(format!(
                    "media derivative {derivative_id} is not active for failure transition (status={status})"
                ))),
            };
        };
        let derivative = media_derivative_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_DERIVATIVE_FAILED,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
            serde_json::json!({
                "derivative_id": derivative.derivative_id,
                "asset_id": derivative.asset_id,
                "derivative_kind": derivative.derivative_kind.as_token(),
                "status": derivative.status.as_token(),
                "retryable": failure.retryable,
                "attempt_count": derivative.attempt_count,
                "error_code": error_code,
                "error_ref": error_ref,
                "updated_by": updated_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(derivative)
    }

    pub async fn retry_media_derivative(
        &self,
        derivative_id: Uuid,
        requested_by: &str,
    ) -> AtelierResult<MediaDerivative> {
        let requested_by = require_derivative_actor("requested_by", requested_by)?;
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_media_derivative
               SET status = 'pending',
                   requested_by = $2,
                   updated_by = $2,
                   retry_count = retry_count + 1,
                   artifact_ref = NULL,
                   artifact_manifest_ref = NULL,
                   mime = NULL,
                   byte_len = NULL,
                   updated_at_utc = NOW()
               WHERE derivative_id = $1
                 AND status = 'retryable_error'
               RETURNING derivative_id, asset_id, derivative_kind, target_width,
                         target_height, format, status, artifact_ref,
                         artifact_manifest_ref, mime, byte_len, requested_by,
                         updated_by, attempt_count, retry_count, last_error_code,
                         last_error_ref, created_at_utc, updated_at_utc"#,
        )
        .bind(derivative_id)
        .bind(requested_by)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            let current_status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM atelier_media_derivative WHERE derivative_id = $1",
            )
            .bind(derivative_id)
            .fetch_optional(&mut *tx)
            .await?;
            return match current_status {
                None => Err(AtelierError::NotFound(format!(
                    "media derivative_id={derivative_id}"
                ))),
                Some(status) => Err(AtelierError::Validation(format!(
                    "media derivative {derivative_id} is not retryable (status={status})"
                ))),
            };
        };
        let derivative = media_derivative_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            event_family::MEDIA_DERIVATIVE_RETRIED,
            "atelier_media_derivative",
            &derivative.derivative_id.to_string(),
            serde_json::json!({
                "derivative_id": derivative.derivative_id,
                "asset_id": derivative.asset_id,
                "derivative_kind": derivative.derivative_kind.as_token(),
                "status": derivative.status.as_token(),
                "retry_count": derivative.retry_count,
                "requested_by": requested_by,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(derivative)
    }

    pub async fn get_media_review_metadata(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<Option<MediaReviewMetadata>> {
        let row = sqlx::query(
            r#"SELECT asset_id, favorite, rating, frontpage, carousel, notes,
                      review_status, updated_by, updated_at_utc
               FROM atelier_media_review_metadata
               WHERE asset_id = $1"#,
        )
        .bind(asset_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(review_metadata_from_row))
    }

    pub async fn bulk_update_media_review_metadata(
        &self,
        updates: &[MediaReviewMetadataUpdate],
        requested_by: &str,
    ) -> AtelierResult<BulkMediaReviewMetadataResult> {
        let requested_by = require_review_metadata_requester(requested_by)?;
        if updates.is_empty() {
            return Err(AtelierError::Validation(
                "review metadata bulk update requires at least one target".into(),
            ));
        }

        let mut seen = HashSet::new();
        let mut normalized_updates = Vec::with_capacity(updates.len());
        for update in updates {
            if !seen.insert(update.asset_id) {
                return Err(AtelierError::Validation(format!(
                    "duplicate review metadata asset_id={}",
                    update.asset_id
                )));
            }
            normalized_updates.push(normalize_review_metadata_update(update)?);
        }

        let asset_ids: Vec<Uuid> = normalized_updates
            .iter()
            .map(|update| update.asset_id)
            .collect();
        let mut tx = self.pool().begin().await?;
        let existing: Vec<Uuid> =
            sqlx::query_scalar("SELECT asset_id FROM atelier_media_asset WHERE asset_id = ANY($1)")
                .bind(&asset_ids)
                .fetch_all(&mut *tx)
                .await?;
        if existing.len() != asset_ids.len() {
            let existing: HashSet<Uuid> = existing.into_iter().collect();
            let missing: Vec<String> = asset_ids
                .iter()
                .filter(|asset_id| !existing.contains(asset_id))
                .map(Uuid::to_string)
                .collect();
            return Err(AtelierError::NotFound(format!(
                "review metadata media targets missing: {}",
                missing.join(", ")
            )));
        }

        let mut metadata = Vec::with_capacity(normalized_updates.len());
        for update in &normalized_updates {
            let row = sqlx::query(
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
            .bind(update.asset_id)
            .bind(update.favorite)
            .bind(update.rating)
            .bind(update.frontpage)
            .bind(update.carousel)
            .bind(&update.notes)
            .bind(&update.review_status)
            .bind(requested_by)
            .fetch_one(&mut *tx)
            .await?;
            let persisted = review_metadata_from_row(&row);
            self.record_event_in_tx(
                &mut tx,
                event_family::MEDIA_REVIEW_METADATA_UPDATED,
                "atelier_media_review_metadata",
                &persisted.asset_id.to_string(),
                serde_json::json!({
                    "asset_id": persisted.asset_id,
                    "favorite": persisted.favorite,
                    "rating": persisted.rating,
                    "frontpage": persisted.frontpage,
                    "carousel": persisted.carousel,
                    "review_status": persisted.review_status,
                    "notes_present": persisted.notes.is_some(),
                    "notes_ref": update.notes_ref,
                    "requested_by": requested_by,
                }),
            )
            .await?;
            metadata.push(persisted);
        }

        let receipt = self
            .record_bulk_operation_receipt_in_tx(
                &mut tx,
                "bulk_update_media_review_metadata",
                requested_by,
                normalized_updates.len() as i64,
                metadata.len() as i64,
                serde_json::json!({
                    "asset_ids": asset_ids,
                    "metadata_count": metadata.len(),
                    "review_statuses": metadata
                        .iter()
                        .map(|row| row.review_status.clone())
                        .collect::<Vec<_>>(),
                }),
            )
            .await?;
        tx.commit().await?;
        Ok(BulkMediaReviewMetadataResult { receipt, metadata })
    }
}
