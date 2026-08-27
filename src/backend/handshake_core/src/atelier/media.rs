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
use std::collections::HashSet;
use std::path::PathBuf;
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{
    atelier_event_sql, event_family, reject_legacy_runtime_ref, AtelierError, AtelierResult,
    AtelierStore, BulkOperationReceipt,
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

#[derive(SurrealValue)]
struct MediaAssetRow {
    asset_id: SurrealUuid,
    content_hash: String,
    mime: String,
    byte_len: i64,
    source_provenance: Option<String>,
    artifact_ref: String,
    retention_class: String,
    artifact_manifest: serde_json::Value,
    created_at_utc: Datetime,
}

impl From<MediaAssetRow> for MediaAsset {
    fn from(row: MediaAssetRow) -> Self {
        Self {
            asset_id: row.asset_id.into(),
            content_hash: row.content_hash,
            mime: row.mime,
            byte_len: row.byte_len,
            source_provenance: row.source_provenance,
            artifact_ref: row.artifact_ref,
            retention_class: row.retention_class,
            artifact_manifest: row.artifact_manifest,
            created_at_utc: row.created_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct MediaSourceProvenanceRefsRow {
    asset_id: SurrealUuid,
    source_url_ref: Option<String>,
    source_path_ref: Option<String>,
    source_note_ref: Option<String>,
    contact_sheet_ref: Option<String>,
    task_ref: Option<String>,
    run_ref: Option<String>,
    updated_by: String,
    updated_at_utc: Datetime,
}

impl From<MediaSourceProvenanceRefsRow> for MediaSourceProvenanceRefs {
    fn from(row: MediaSourceProvenanceRefsRow) -> Self {
        Self {
            asset_id: row.asset_id.into(),
            source_url_ref: row.source_url_ref,
            source_path_ref: row.source_path_ref,
            source_note_ref: row.source_note_ref,
            contact_sheet_ref: row.contact_sheet_ref,
            task_ref: row.task_ref,
            run_ref: row.run_ref,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct MediaSidecarRow {
    sidecar_id: SurrealUuid,
    parent_asset_id: SurrealUuid,
    sidecar_asset_id: SurrealUuid,
    relation_kind: String,
    hidden_from_gallery: bool,
    searchable_by_relation: bool,
    created_by: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl TryFrom<MediaSidecarRow> for MediaSidecar {
    type Error = AtelierError;

    fn try_from(row: MediaSidecarRow) -> AtelierResult<Self> {
        let relation_kind = row.relation_kind;
        Ok(MediaSidecar {
            sidecar_id: row.sidecar_id.into(),
            parent_asset_id: row.parent_asset_id.into(),
            sidecar_asset_id: row.sidecar_asset_id.into(),
            relation_kind: MediaSidecarRelationKind::from_token(&relation_kind)?,
            hidden_from_gallery: row.hidden_from_gallery,
            searchable_by_relation: row.searchable_by_relation,
            created_by: row.created_by,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct MediaReviewMetadataRow {
    asset_id: SurrealUuid,
    favorite: bool,
    rating: i64,
    frontpage: bool,
    carousel: bool,
    notes: Option<String>,
    review_status: String,
    updated_by: String,
    updated_at_utc: Datetime,
}

impl TryFrom<MediaReviewMetadataRow> for MediaReviewMetadata {
    type Error = AtelierError;

    fn try_from(row: MediaReviewMetadataRow) -> AtelierResult<Self> {
        Ok(Self {
            asset_id: row.asset_id.into(),
            favorite: row.favorite,
            rating: i16::try_from(row.rating).map_err(|_| {
                AtelierError::Internal(format!("persisted media rating {} exceeds i16", row.rating))
            })?,
            frontpage: row.frontpage,
            carousel: row.carousel,
            notes: row.notes,
            review_status: row.review_status,
            updated_by: row.updated_by,
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct MediaDerivativeRow {
    derivative_id: SurrealUuid,
    asset_id: SurrealUuid,
    derivative_kind: String,
    target_width: i64,
    target_height: i64,
    format: String,
    status: String,
    artifact_ref: Option<String>,
    artifact_manifest_ref: Option<String>,
    mime: Option<String>,
    byte_len: Option<i64>,
    requested_by: String,
    updated_by: String,
    attempt_count: i64,
    retry_count: i64,
    last_error_code: Option<String>,
    last_error_ref: Option<String>,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl TryFrom<MediaDerivativeRow> for MediaDerivative {
    type Error = AtelierError;

    fn try_from(row: MediaDerivativeRow) -> AtelierResult<Self> {
        let kind = row.derivative_kind;
        let status = row.status;
        Ok(MediaDerivative {
            derivative_id: row.derivative_id.into(),
            asset_id: row.asset_id.into(),
            derivative_kind: MediaDerivativeKind::from_token(&kind)?,
            target_width: i32::try_from(row.target_width).map_err(|_| {
                AtelierError::Internal("persisted derivative width exceeds i32".into())
            })?,
            target_height: i32::try_from(row.target_height).map_err(|_| {
                AtelierError::Internal("persisted derivative height exceeds i32".into())
            })?,
            format: row.format,
            status: MediaDerivativeStatus::from_token(&status)?,
            artifact_ref: row.artifact_ref,
            artifact_manifest_ref: row.artifact_manifest_ref,
            mime: row.mime,
            byte_len: row.byte_len,
            requested_by: row.requested_by,
            updated_by: row.updated_by,
            attempt_count: row.attempt_count,
            retry_count: row.retry_count,
            last_error_code: row.last_error_code,
            last_error_ref: row.last_error_ref,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

macro_rules! media_asset_select {
    () => {
        "asset_id, content_hash, mime, byte_len, source_provenance, artifact_ref, \
         retention_class, artifact_manifest, created_at_utc"
    };
}

macro_rules! provenance_select {
    () => {
        "record::id(asset_id) AS asset_id, source_url_ref, source_path_ref, source_note_ref, \
         contact_sheet_ref, task_ref, run_ref, updated_by, updated_at_utc"
    };
}

macro_rules! sidecar_select {
    () => {
        "sidecar_id, record::id(parent_asset_id) AS parent_asset_id, \
         record::id(sidecar_asset_id) AS sidecar_asset_id, relation_kind, hidden_from_gallery, \
         searchable_by_relation, created_by, created_at_utc, updated_at_utc"
    };
}

macro_rules! derivative_select {
    () => {
        "derivative_id, record::id(asset_id) AS asset_id, derivative_kind, target_width, \
         target_height, format, status, artifact_ref, artifact_manifest_ref, mime, byte_len, \
         requested_by, updated_by, attempt_count, retry_count, last_error_code, last_error_ref, \
         created_at_utc, updated_at_utc"
    };
}

macro_rules! review_metadata_select {
    () => {
        "record::id(asset_id) AS asset_id, favorite, rating, frontpage, carousel, notes, \
         review_status, updated_by, updated_at_utc"
    };
}

#[derive(SurrealValue)]
struct AssetIdBinding {
    asset_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct AssetRefBinding {
    asset_ref: RecordId,
}

#[derive(SurrealValue)]
struct ContentHashBinding {
    content_hash: String,
}

#[derive(SurrealValue)]
struct LimitBinding {
    limit: i64,
}

#[derive(Clone, SurrealValue)]
struct MaterializeMediaAssetBindings {
    asset_rid: RecordId,
    asset_id: SurrealUuid,
    content_hash: String,
    mime: String,
    byte_len: i64,
    source_provenance: Option<String>,
    artifact_ref: String,
    retention_class: String,
    artifact_manifest: serde_json::Value,
}

#[derive(Clone, SurrealValue)]
struct SetProvenanceBindings {
    provenance_rid: RecordId,
    asset_ref: RecordId,
    source_url_ref: Option<String>,
    source_path_ref: Option<String>,
    source_note_ref: Option<String>,
    contact_sheet_ref: Option<String>,
    task_ref: Option<String>,
    run_ref: Option<String>,
    updated_by: String,
}

#[derive(Clone, SurrealValue)]
struct UpgradeMediaAssetBindings {
    asset_rid: RecordId,
    expected_artifact_ref: String,
    mime: String,
    byte_len: i64,
    source_provenance: Option<String>,
    artifact_ref: String,
    retention_class: String,
    artifact_manifest: serde_json::Value,
}

#[derive(SurrealValue)]
struct RepairManifestBindings {
    asset_rid: RecordId,
    retention_class: String,
    artifact_manifest: serde_json::Value,
}

#[derive(Clone, SurrealValue)]
struct CreateSidecarBindings {
    sidecar_rid: RecordId,
    sidecar_id: SurrealUuid,
    parent_ref: RecordId,
    sidecar_ref: RecordId,
    relation_kind: String,
    created_by: String,
}

#[derive(SurrealValue)]
struct ListSidecarBindings {
    parent_ref: RecordId,
    relation_kind: Option<String>,
}

#[derive(SurrealValue)]
struct ExactSidecarBindings {
    parent_ref: RecordId,
    sidecar_ref: RecordId,
    relation_kind: String,
}

#[derive(Clone, SurrealValue)]
struct RequestDerivativeBindings {
    derivative_rid: RecordId,
    derivative_id: SurrealUuid,
    asset_ref: RecordId,
    derivative_kind: String,
    target_width: i64,
    target_height: i64,
    format: String,
    requested_by: String,
}

#[derive(SurrealValue)]
struct FindDerivativeBindings {
    asset_ref: RecordId,
    derivative_kind: String,
    target_width: i64,
    target_height: i64,
    format: String,
}

#[derive(SurrealValue)]
struct DerivativeAssetBinding {
    asset_ref: RecordId,
}

#[derive(Clone, SurrealValue)]
struct DerivativeTransitionBindings {
    derivative_rid: RecordId,
    expected_statuses: Vec<String>,
    status: String,
    requested_by: Option<String>,
    updated_by: String,
    artifact_ref: Option<String>,
    artifact_manifest_ref: Option<String>,
    mime: Option<String>,
    byte_len: Option<i64>,
    last_error_code: Option<String>,
    last_error_ref: Option<String>,
    increment_attempt: bool,
    increment_retry: bool,
}

#[derive(SurrealValue)]
struct DerivativeIdBinding {
    derivative_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct DerivativeStateRow {
    status: String,
    format: String,
}

#[derive(Clone, SurrealValue)]
struct ReviewUpdateInput {
    metadata_rid: RecordId,
    asset_ref: RecordId,
    favorite: bool,
    rating: i64,
    frontpage: bool,
    carousel: bool,
    notes: Option<String>,
    review_status: String,
}

#[derive(Clone, SurrealValue)]
struct BulkReviewBindings {
    asset_refs: Vec<RecordId>,
    updates: Vec<ReviewUpdateInput>,
    requested_by: String,
}

const MATERIALIZE_MEDIA_ASSET_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " CREATE $domain.asset_rid CONTENT { asset_id: $domain.asset_id, \
       content_hash: $domain.content_hash, mime: $domain.mime, byte_len: $domain.byte_len, \
       source_provenance: $domain.source_provenance, artifact_ref: $domain.artifact_ref, \
       retention_class: $domain.retention_class, artifact_manifest: $domain.artifact_manifest }; \
     RETURN (SELECT ",
    media_asset_select!(),
    " FROM atelier_media_asset WHERE content_hash = $domain.content_hash LIMIT 1); };"
);

const SET_PROVENANCE_STATEMENT: &str = concat!(
    "RETURN { IF !record::exists($domain.asset_ref) { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPSERT $domain.provenance_rid SET asset_id = $domain.asset_ref, \
       source_url_ref = $domain.source_url_ref, source_path_ref = $domain.source_path_ref, \
       source_note_ref = $domain.source_note_ref, contact_sheet_ref = $domain.contact_sheet_ref, \
       task_ref = $domain.task_ref, run_ref = $domain.run_ref, updated_by = $domain.updated_by, \
       updated_at_utc = time::now(); RETURN (SELECT ",
    provenance_select!(),
    " FROM $domain.provenance_rid); };"
);

const UPGRADE_MEDIA_ASSET_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT VALUE artifact_ref FROM $domain.asset_rid)[0]; \
     IF $current != $domain.expected_artifact_ref { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPDATE $domain.asset_rid SET mime = $domain.mime, byte_len = $domain.byte_len, \
       source_provenance = $domain.source_provenance, artifact_ref = $domain.artifact_ref, \
       retention_class = $domain.retention_class, artifact_manifest = $domain.artifact_manifest; \
     RETURN (SELECT ",
    media_asset_select!(),
    " FROM $domain.asset_rid); };"
);

const CREATE_SIDECAR_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " CREATE $domain.sidecar_rid CONTENT { sidecar_id: $domain.sidecar_id, \
       parent_asset_id: $domain.parent_ref, sidecar_asset_id: $domain.sidecar_ref, \
       relation_kind: $domain.relation_kind, hidden_from_gallery: true, \
       searchable_by_relation: true, created_by: $domain.created_by }; \
     RETURN (SELECT ",
    sidecar_select!(),
    " FROM $domain.sidecar_rid); };"
);

const REQUEST_DERIVATIVE_STATEMENT: &str = concat!(
    "RETURN { ",
    atelier_event_sql!(),
    " CREATE $domain.derivative_rid CONTENT { derivative_id: $domain.derivative_id, \
       asset_id: $domain.asset_ref, derivative_kind: $domain.derivative_kind, \
       target_width: $domain.target_width, target_height: $domain.target_height, \
       format: $domain.format, status: 'pending', requested_by: $domain.requested_by, \
       updated_by: $domain.requested_by }; \
     RETURN (SELECT ",
    derivative_select!(),
    " FROM $domain.derivative_rid); };"
);

const MARK_DERIVATIVE_GENERATING_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT VALUE status FROM $domain.derivative_rid)[0]; \
     IF $current NOT IN $domain.expected_statuses { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPDATE $domain.derivative_rid SET status = 'generating', updated_by = $domain.updated_by, \
       updated_at_utc = time::now(); RETURN (SELECT ",
    derivative_select!(),
    " FROM $domain.derivative_rid); };"
);

const MARK_DERIVATIVE_GENERATED_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT status, format FROM $domain.derivative_rid)[0]; \
     IF $current.status != 'generating' { RETURN NONE; }; \
     IF !(( $current.format = 'png' AND $domain.mime = 'image/png') OR \
          ( $current.format = 'jpeg' AND $domain.mime = 'image/jpeg')) { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPDATE $domain.derivative_rid SET status = 'generated', updated_by = $domain.updated_by, \
       artifact_ref = $domain.artifact_ref, artifact_manifest_ref = $domain.artifact_manifest_ref, \
       mime = $domain.mime, byte_len = $domain.byte_len, last_error_code = NONE, \
       last_error_ref = NONE, updated_at_utc = time::now(); RETURN (SELECT ",
    derivative_select!(),
    " FROM $domain.derivative_rid); };"
);

const MARK_DERIVATIVE_FAILED_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT VALUE status FROM $domain.derivative_rid)[0]; \
     IF $current NOT IN ['pending', 'generating'] { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPDATE $domain.derivative_rid SET status = $domain.status, updated_by = $domain.updated_by, \
       attempt_count += 1, last_error_code = $domain.last_error_code, \
       last_error_ref = $domain.last_error_ref, artifact_ref = NONE, \
       artifact_manifest_ref = NONE, mime = NONE, byte_len = NONE, \
       updated_at_utc = time::now(); RETURN (SELECT ",
    derivative_select!(),
    " FROM $domain.derivative_rid); };"
);

const RETRY_DERIVATIVE_STATEMENT: &str = concat!(
    "RETURN { LET $current = (SELECT VALUE status FROM $domain.derivative_rid)[0]; \
     IF $current != 'retryable_error' { RETURN NONE; }; ",
    atelier_event_sql!(),
    " UPDATE $domain.derivative_rid SET status = 'pending', requested_by = $domain.requested_by, \
       updated_by = $domain.updated_by, retry_count += 1, artifact_ref = NONE, \
       artifact_manifest_ref = NONE, mime = NONE, byte_len = NONE, \
       updated_at_utc = time::now(); RETURN (SELECT ",
    derivative_select!(),
    " FROM $domain.derivative_rid); };"
);

const BULK_REVIEW_UPDATE_STATEMENT: &str = concat!(
    "RETURN { LET $existing = (SELECT VALUE id FROM atelier_media_asset \
       WHERE id IN $domain.asset_refs); IF array::len($existing) != array::len($domain.asset_refs) \
       { RETURN []; }; FOR $item IN $domain.updates { UPSERT $item.metadata_rid SET \
       asset_id = $item.asset_ref, favorite = $item.favorite, rating = $item.rating, \
       frontpage = $item.frontpage, carousel = $item.carousel, notes = $item.notes, \
       review_status = $item.review_status, updated_by = $domain.requested_by, \
       updated_at_utc = time::now(); }; RETURN (SELECT ",
    review_metadata_select!(),
    " FROM atelier_media_review_metadata WHERE asset_id IN $domain.asset_refs ORDER BY asset_id); };"
);

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
        let bindings = MaterializeMediaAssetBindings {
            asset_rid: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
            asset_id: SurrealUuid::from(asset_id),
            content_hash: content_hash.clone(),
            mime: new.mime.clone(),
            byte_len: new.byte_len,
            source_provenance: new.source_provenance.clone(),
            artifact_ref: new.artifact_ref.clone(),
            retention_class: MEDIA_ORIGINAL_RETENTION_CLASS.to_owned(),
            artifact_manifest,
        };
        let row: Option<MediaAssetRow> = self
            .write_with_event(
            MATERIALIZE_MEDIA_ASSET_STATEMENT,
            bindings,
            event_family::MEDIA_ASSET_MATERIALIZED,
            "atelier_media_asset",
            &content_hash,
            serde_json::json!({
                "asset_id": asset_id,
                "mime": new.mime,
                "byte_len": new.byte_len,
                "artifact_ref": new.artifact_ref,
                "retention_class": MEDIA_ORIGINAL_RETENTION_CLASS,
                "artifact_manifest": event_safe_media_artifact_manifest(
                    &build_media_artifact_manifest(asset_id, new, &content_hash, source_provenance),
                    source_provenance,
                ),
            }),
        )
        .await?;
        row.map(MediaAsset::from).ok_or_else(|| {
            AtelierError::Internal("materializing a media asset returned no row".to_owned())
        })
    }

    pub async fn set_media_source_provenance_refs(
        &self,
        update: &SetMediaSourceProvenanceRefs,
    ) -> AtelierResult<MediaSourceProvenanceRefs> {
        let [source_url_ref, source_path_ref, source_note_ref, contact_sheet_ref, task_ref, run_ref] =
            validate_media_source_provenance_refs(update)?;
        let asset_ref = RecordId::new("atelier_media_asset", SurrealUuid::from(update.asset_id));
        let bindings = SetProvenanceBindings {
            provenance_rid: RecordId::new(
                "atelier_media_source_provenance_ref",
                SurrealUuid::from(update.asset_id),
            ),
            asset_ref,
            source_url_ref: source_url_ref.clone(),
            source_path_ref: source_path_ref.clone(),
            source_note_ref: source_note_ref.clone(),
            contact_sheet_ref: contact_sheet_ref.clone(),
            task_ref: task_ref.clone(),
            run_ref: run_ref.clone(),
            updated_by: update.updated_by.clone(),
        };
        let row: Option<MediaSourceProvenanceRefsRow> = self
            .write_with_event(
                SET_PROVENANCE_STATEMENT,
                bindings,
                event_family::MEDIA_SOURCE_PROVENANCE_REFS_SET,
                "atelier_media_asset",
                &update.asset_id.to_string(),
                serde_json::json!({
                    "asset_id": update.asset_id,
                    "source_url_ref": source_url_ref,
                    "source_path_ref": source_path_ref,
                    "source_note_ref": source_note_ref,
                    "contact_sheet_ref": contact_sheet_ref,
                    "task_ref": task_ref,
                    "run_ref": run_ref,
                    "updated_by": update.updated_by,
                }),
            )
            .await?;
        row.map(Into::into)
            .ok_or_else(|| AtelierError::NotFound(format!("media asset_id={}", update.asset_id)))
    }

    pub async fn get_media_source_provenance_refs(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<Option<MediaSourceProvenanceRefs>> {
        let bindings = AssetRefBinding {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
        };
        let row: Option<MediaSourceProvenanceRefsRow> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_first(
                    concat!("SELECT ", provenance_select!(), " FROM atelier_media_source_provenance_ref WHERE asset_id = $asset_ref LIMIT 1;"),
                    bindings,
                ).await
            })
        }).await?;
        Ok(row.map(Into::into))
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
        let bindings = UpgradeMediaAssetBindings {
            asset_rid: RecordId::new("atelier_media_asset", SurrealUuid::from(existing.asset_id)),
            expected_artifact_ref: existing.artifact_ref.clone(),
            mime: new.mime.clone(),
            byte_len: new.byte_len,
            source_provenance: new.source_provenance.clone(),
            artifact_ref: new.artifact_ref.clone(),
            retention_class: MEDIA_ORIGINAL_RETENTION_CLASS.to_owned(),
            artifact_manifest: manifest,
        };
        let row: Option<MediaAssetRow> = self.write_with_event(
            UPGRADE_MEDIA_ASSET_STATEMENT,
            bindings,
            event_family::MEDIA_ASSET_MATERIALIZED,
            "atelier_media_asset",
            content_hash,
            serde_json::json!({
                "asset_id": existing.asset_id,
                "mime": new.mime,
                "byte_len": new.byte_len,
                "artifact_ref": new.artifact_ref,
                "retention_class": MEDIA_ORIGINAL_RETENTION_CLASS,
                "artifact_manifest": event_safe_media_artifact_manifest(
                    &build_media_artifact_manifest_from_parts(existing.asset_id, &new.artifact_ref, content_hash, &new.mime, new.byte_len, source_provenance, MEDIA_ORIGINAL_RETENTION_CLASS),
                    source_provenance,
                ),
            }),
        ).await?;
        let Some(row) = row else {
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
        Ok(row.into())
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
        let bindings = RepairManifestBindings {
            asset_rid: RecordId::new("atelier_media_asset", SurrealUuid::from(asset.asset_id)),
            retention_class: retention_class.to_owned(),
            artifact_manifest: manifest,
        };
        let row: Option<MediaAssetRow> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_first(
                    concat!("RETURN { UPDATE $asset_rid SET retention_class = $retention_class, artifact_manifest = $artifact_manifest; RETURN (SELECT ", media_asset_select!(), " FROM $asset_rid); };"),
                    bindings,
                ).await
            })
        }).await?;
        row.map(Into::into)
            .ok_or_else(|| AtelierError::NotFound(format!("media asset_id={}", asset.asset_id)))
    }

    pub(crate) async fn repair_media_asset_artifact_manifests(&self) -> AtelierResult<()> {
        // Embedded SurrealDB is bootstrapped at the current schema and has no
        // PostgreSQL upgrade lineage to repair.
        Ok(())
    }

    pub async fn get_media_asset_by_hash(
        &self,
        content_hash: &str,
    ) -> AtelierResult<Option<MediaAsset>> {
        let bindings = ContentHashBinding {
            content_hash: content_hash.to_owned(),
        };
        let row: Option<MediaAssetRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        concat!(
                            "SELECT ",
                            media_asset_select!(),
                            " FROM atelier_media_asset WHERE content_hash = $content_hash LIMIT 1;"
                        ),
                        bindings,
                    )
                    .await
                })
            })
            .await?;
        Ok(row.map(Into::into))
    }

    pub async fn get_media_artifact_manifest(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<serde_json::Value> {
        let bindings = AssetIdBinding {
            asset_id: asset_id.into(),
        };
        let manifest: Option<serde_json::Value> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_first("SELECT VALUE artifact_manifest FROM atelier_media_asset WHERE asset_id = $asset_id LIMIT 1;", bindings).await
            })
        }).await?;
        let manifest =
            manifest.ok_or_else(|| AtelierError::NotFound(format!("media asset_id={asset_id}")))?;
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
        let asset_ids = vec![new.parent_asset_id, new.sidecar_asset_id];
        #[derive(SurrealValue)]
        struct AssetIdsBinding {
            asset_ids: Vec<SurrealUuid>,
        }
        let existing: Vec<SurrealUuid> = self.store().with_data_operation({
            let bindings = AssetIdsBinding { asset_ids: asset_ids.iter().copied().map(Into::into).collect() };
            move |ctx| Box::pin(async move {
                ctx.query_values("SELECT VALUE asset_id FROM atelier_media_asset WHERE asset_id IN $asset_ids;", bindings).await
            })
        }).await?;
        if existing.len() != asset_ids.len() {
            let existing: HashSet<Uuid> = existing.into_iter().map(Into::into).collect();
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

        let existing_relation: Option<MediaSidecarRow> = self
            .store()
            .with_data_operation({
                let bindings = ExactSidecarBindings {
                    parent_ref: RecordId::new(
                        "atelier_media_asset",
                        SurrealUuid::from(new.parent_asset_id),
                    ),
                    sidecar_ref: RecordId::new(
                        "atelier_media_asset",
                        SurrealUuid::from(new.sidecar_asset_id),
                    ),
                    relation_kind: new.relation_kind.as_token().to_owned(),
                };
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            concat!(
                                "SELECT ",
                                sidecar_select!(),
                                " FROM atelier_media_sidecar WHERE parent_asset_id = $parent_ref \
                                 AND sidecar_asset_id = $sidecar_ref \
                                 AND relation_kind = $relation_kind LIMIT 1;"
                            ),
                            bindings,
                        )
                        .await
                    })
                }
            })
            .await?;
        if let Some(existing_relation) = existing_relation {
            return existing_relation.try_into();
        }

        let sidecar_id = Uuid::now_v7();
        let bindings = CreateSidecarBindings {
            sidecar_rid: RecordId::new("atelier_media_sidecar", SurrealUuid::from(sidecar_id)),
            sidecar_id: sidecar_id.into(),
            parent_ref: RecordId::new(
                "atelier_media_asset",
                SurrealUuid::from(new.parent_asset_id),
            ),
            sidecar_ref: RecordId::new(
                "atelier_media_asset",
                SurrealUuid::from(new.sidecar_asset_id),
            ),
            relation_kind: new.relation_kind.as_token().to_owned(),
            created_by: created_by.to_owned(),
        };
        let row: Option<MediaSidecarRow> = self
            .write_with_event(
                CREATE_SIDECAR_STATEMENT,
                bindings,
                event_family::MEDIA_SIDECAR_RECORDED,
                "atelier_media_sidecar",
                &sidecar_id.to_string(),
                serde_json::json!({
                    "sidecar_id": sidecar_id,
                    "parent_asset_id": new.parent_asset_id,
                    "sidecar_asset_id": new.sidecar_asset_id,
                    "relation_kind": new.relation_kind.as_token(),
                    "hidden_from_gallery": true,
                    "searchable_by_relation": true,
                    "created_by": created_by,
                }),
            )
            .await?;
        row.ok_or_else(|| AtelierError::Internal("recording media sidecar returned no row".into()))?
            .try_into()
    }

    pub async fn list_media_sidecars_for_asset(
        &self,
        parent_asset_id: Uuid,
        relation_kind: Option<MediaSidecarRelationKind>,
    ) -> AtelierResult<Vec<MediaSidecar>> {
        let bindings = ListSidecarBindings {
            parent_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(parent_asset_id)),
            relation_kind: relation_kind.map(|kind| kind.as_token().to_owned()),
        };
        let rows: Vec<MediaSidecarRow> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(concat!("SELECT ", sidecar_select!(), " FROM atelier_media_sidecar WHERE parent_asset_id = $parent_ref AND searchable_by_relation = true AND ($relation_kind = NONE OR relation_kind = $relation_kind) ORDER BY relation_kind, updated_at_utc DESC, sidecar_id;"), bindings).await
            })
        }).await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn list_media_gallery_assets(&self, limit: i64) -> AtelierResult<Vec<MediaAsset>> {
        if !(1..=500).contains(&limit) {
            return Err(AtelierError::Validation(
                "media gallery limit must be between 1 and 500".into(),
            ));
        }
        let rows: Vec<MediaAssetRow> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_values(concat!("SELECT ", media_asset_select!(), " FROM atelier_media_asset WHERE id NOT IN (SELECT VALUE sidecar_asset_id FROM atelier_media_sidecar WHERE hidden_from_gallery = true) AND asset_id NOT IN (SELECT VALUE target_id FROM atelier_trash_marker WHERE target_type = 'media_asset') ORDER BY created_at_utc DESC, asset_id DESC LIMIT $limit;"), LimitBinding { limit }).await
            })
        }).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn request_media_derivative(
        &self,
        request: &MediaDerivativeRequest,
    ) -> AtelierResult<MediaDerivative> {
        validate_derivative_dimensions(request.target_width, request.target_height)?;
        let format = normalize_derivative_format(&request.format)?;
        let requested_by = require_derivative_actor("requested_by", &request.requested_by)?;
        let asset_ref = RecordId::new("atelier_media_asset", SurrealUuid::from(request.asset_id));
        let asset_exists: Option<bool> = self
            .store()
            .with_data_operation({
                let bindings = AssetRefBinding {
                    asset_ref: asset_ref.clone(),
                };
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first("RETURN record::exists($asset_ref);", bindings)
                            .await
                    })
                }
            })
            .await?;
        let asset_exists = asset_exists.unwrap_or(false);
        if !asset_exists {
            return Err(AtelierError::NotFound(format!(
                "media asset_id={}",
                request.asset_id
            )));
        }

        let find_bindings = FindDerivativeBindings {
            asset_ref: asset_ref.clone(),
            derivative_kind: request.derivative_kind.as_token().to_owned(),
            target_width: i64::from(request.target_width),
            target_height: i64::from(request.target_height),
            format: format.clone(),
        };
        let existing: Option<MediaDerivativeRow> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_first(concat!("SELECT ", derivative_select!(), " FROM atelier_media_derivative WHERE asset_id = $asset_ref AND derivative_kind = $derivative_kind AND target_width = $target_width AND target_height = $target_height AND format = $format LIMIT 1;"), find_bindings).await
            })
        }).await?;
        if let Some(existing) = existing {
            return existing.try_into();
        }

        let derivative_id = Uuid::now_v7();
        let bindings = RequestDerivativeBindings {
            derivative_rid: RecordId::new(
                "atelier_media_derivative",
                SurrealUuid::from(derivative_id),
            ),
            derivative_id: derivative_id.into(),
            asset_ref,
            derivative_kind: request.derivative_kind.as_token().to_owned(),
            target_width: i64::from(request.target_width),
            target_height: i64::from(request.target_height),
            format: format.clone(),
            requested_by: requested_by.to_owned(),
        };
        let row: Option<MediaDerivativeRow> = self
            .write_with_event(
                REQUEST_DERIVATIVE_STATEMENT,
                bindings,
                event_family::MEDIA_DERIVATIVE_REQUESTED,
                "atelier_media_derivative",
                &derivative_id.to_string(),
                serde_json::json!({
                    "derivative_id": derivative_id,
                    "asset_id": request.asset_id,
                    "derivative_kind": request.derivative_kind.as_token(),
                    "target_width": request.target_width,
                    "target_height": request.target_height,
                    "format": format,
                    "status": "pending",
                    "requested_by": requested_by,
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("requesting media derivative returned no row".into())
        })?
        .try_into()
    }

    pub async fn list_media_derivatives(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<Vec<MediaDerivative>> {
        let bindings = DerivativeAssetBinding {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
        };
        let rows: Vec<MediaDerivativeRow> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move { ctx.query_values(concat!("SELECT ", derivative_select!(), " FROM atelier_media_derivative WHERE asset_id = $asset_ref ORDER BY derivative_kind, target_width, target_height, format;"), bindings).await })
        }).await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn get_media_derivative_record(
        &self,
        derivative_id: Uuid,
    ) -> AtelierResult<Option<MediaDerivative>> {
        let bindings = DerivativeIdBinding {
            derivative_id: derivative_id.into(),
        };
        let row: Option<MediaDerivativeRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        concat!("SELECT ", derivative_select!(), " FROM atelier_media_derivative WHERE derivative_id = $derivative_id LIMIT 1;"),
                        bindings,
                    )
                    .await
                })
            })
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    pub async fn mark_media_derivative_generating(
        &self,
        derivative_id: Uuid,
        updated_by: &str,
    ) -> AtelierResult<MediaDerivative> {
        let updated_by = require_derivative_actor("updated_by", updated_by)?;
        let current = self
            .get_media_derivative_record(derivative_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("media derivative_id={derivative_id}"))
            })?;
        if current.status != MediaDerivativeStatus::Pending {
            return Err(AtelierError::Validation(format!(
                "media derivative {derivative_id} is not pending; retryable derivatives must be retried first"
            )));
        }
        let bindings = DerivativeTransitionBindings {
            derivative_rid: RecordId::new(
                "atelier_media_derivative",
                SurrealUuid::from(derivative_id),
            ),
            expected_statuses: vec!["pending".to_owned()],
            status: "generating".to_owned(),
            requested_by: None,
            updated_by: updated_by.to_owned(),
            artifact_ref: None,
            artifact_manifest_ref: None,
            mime: None,
            byte_len: None,
            last_error_code: None,
            last_error_ref: None,
            increment_attempt: false,
            increment_retry: false,
        };
        let row: Option<MediaDerivativeRow> = self
            .write_with_event(
                MARK_DERIVATIVE_GENERATING_STATEMENT,
                bindings,
                event_family::MEDIA_DERIVATIVE_GENERATING,
                "atelier_media_derivative",
                &derivative_id.to_string(),
                serde_json::json!({
                    "derivative_id": derivative_id,
                    "asset_id": current.asset_id,
                    "derivative_kind": current.derivative_kind.as_token(),
                    "status": "generating",
                    "updated_by": updated_by,
                }),
            )
            .await?;
        row.ok_or_else(|| AtelierError::Validation(format!("media derivative {derivative_id} is not pending; retryable derivatives must be retried first")))?.try_into()
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
        let current = self
            .get_media_derivative_record(generated.derivative_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("media derivative_id={}", generated.derivative_id))
            })?;
        if current.status != MediaDerivativeStatus::Generating {
            return Err(AtelierError::Validation(format!(
                "media derivative {} is not active for generated transition (status={})",
                generated.derivative_id,
                current.status.as_token()
            )));
        }
        let expected_mime = expected_mime_for_derivative_format(&current.format)?;
        if expected_mime != mime {
            return Err(AtelierError::Validation(format!(
                "media derivative format {} requires mime {expected_mime}, got {mime}",
                current.format
            )));
        }
        let bindings = DerivativeTransitionBindings {
            derivative_rid: RecordId::new(
                "atelier_media_derivative",
                SurrealUuid::from(generated.derivative_id),
            ),
            expected_statuses: vec!["generating".to_owned()],
            status: "generated".to_owned(),
            requested_by: None,
            updated_by: updated_by.to_owned(),
            artifact_ref: Some(generated.artifact_ref.clone()),
            artifact_manifest_ref: Some(generated.artifact_manifest_ref.clone()),
            mime: Some(mime.to_owned()),
            byte_len: Some(generated.byte_len),
            last_error_code: None,
            last_error_ref: None,
            increment_attempt: false,
            increment_retry: false,
        };
        let row: Option<MediaDerivativeRow> = self
            .write_with_event(
                MARK_DERIVATIVE_GENERATED_STATEMENT,
                bindings,
                event_family::MEDIA_DERIVATIVE_GENERATED,
                "atelier_media_derivative",
                &generated.derivative_id.to_string(),
                serde_json::json!({
                    "derivative_id": generated.derivative_id,
                    "asset_id": current.asset_id,
                    "derivative_kind": current.derivative_kind.as_token(),
                    "target_width": current.target_width,
                    "target_height": current.target_height,
                    "format": current.format,
                    "status": "generated",
                    "artifact_ref": generated.artifact_ref,
                    "artifact_manifest_ref": generated.artifact_manifest_ref,
                    "mime": mime,
                    "byte_len": generated.byte_len,
                    "updated_by": updated_by,
                }),
            )
            .await?;
        let Some(row) = row else {
            let current: Option<DerivativeStateRow> = self.store().with_data_operation({
                let bindings = DerivativeIdBinding { derivative_id: generated.derivative_id.into() };
                move |ctx| Box::pin(async move { ctx.query_first("SELECT status, format FROM atelier_media_derivative WHERE derivative_id = $derivative_id LIMIT 1;", bindings).await })
            }).await?;
            return match current {
                None => Err(AtelierError::NotFound(format!(
                    "media derivative_id={}",
                    generated.derivative_id
                ))),
                Some(row) => {
                    let status = row.status;
                    let format = row.format;
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
        row.try_into()
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
        let current = self
            .get_media_derivative_record(derivative_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("media derivative_id={derivative_id}"))
            })?;
        if !matches!(
            current.status,
            MediaDerivativeStatus::Pending | MediaDerivativeStatus::Generating
        ) {
            return Err(AtelierError::Validation(format!(
                "media derivative {derivative_id} is not active for failure transition (status={})",
                current.status.as_token()
            )));
        }
        let bindings = DerivativeTransitionBindings {
            derivative_rid: RecordId::new(
                "atelier_media_derivative",
                SurrealUuid::from(derivative_id),
            ),
            expected_statuses: vec!["pending".to_owned(), "generating".to_owned()],
            status: status.as_token().to_owned(),
            requested_by: None,
            updated_by: updated_by.to_owned(),
            artifact_ref: None,
            artifact_manifest_ref: None,
            mime: None,
            byte_len: None,
            last_error_code: Some(error_code.clone()),
            last_error_ref: Some(error_ref.clone()),
            increment_attempt: true,
            increment_retry: false,
        };
        let row: Option<MediaDerivativeRow> = self
            .write_with_event(
                MARK_DERIVATIVE_FAILED_STATEMENT,
                bindings,
                event_family::MEDIA_DERIVATIVE_FAILED,
                "atelier_media_derivative",
                &derivative_id.to_string(),
                serde_json::json!({
                    "derivative_id": derivative_id,
                    "asset_id": current.asset_id,
                    "derivative_kind": current.derivative_kind.as_token(),
                    "status": status.as_token(),
                    "retryable": failure.retryable,
                    "attempt_count": current.attempt_count + 1,
                    "error_code": error_code,
                    "error_ref": error_ref,
                    "updated_by": updated_by,
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Validation(format!(
                "media derivative {derivative_id} changed state during failure transition"
            ))
        })?
        .try_into()
    }

    pub async fn retry_media_derivative(
        &self,
        derivative_id: Uuid,
        requested_by: &str,
    ) -> AtelierResult<MediaDerivative> {
        let requested_by = require_derivative_actor("requested_by", requested_by)?;
        let current = self
            .get_media_derivative_record(derivative_id)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("media derivative_id={derivative_id}"))
            })?;
        if current.status != MediaDerivativeStatus::RetryableError {
            return Err(AtelierError::Validation(format!(
                "media derivative {derivative_id} is not retryable (status={})",
                current.status.as_token()
            )));
        }
        let bindings = DerivativeTransitionBindings {
            derivative_rid: RecordId::new(
                "atelier_media_derivative",
                SurrealUuid::from(derivative_id),
            ),
            expected_statuses: vec!["retryable_error".to_owned()],
            status: "pending".to_owned(),
            requested_by: Some(requested_by.to_owned()),
            updated_by: requested_by.to_owned(),
            artifact_ref: None,
            artifact_manifest_ref: None,
            mime: None,
            byte_len: None,
            last_error_code: current.last_error_code.clone(),
            last_error_ref: current.last_error_ref.clone(),
            increment_attempt: false,
            increment_retry: true,
        };
        let row: Option<MediaDerivativeRow> = self
            .write_with_event(
                RETRY_DERIVATIVE_STATEMENT,
                bindings,
                event_family::MEDIA_DERIVATIVE_RETRIED,
                "atelier_media_derivative",
                &derivative_id.to_string(),
                serde_json::json!({
                    "derivative_id": derivative_id,
                    "asset_id": current.asset_id,
                    "derivative_kind": current.derivative_kind.as_token(),
                    "status": "pending",
                    "retry_count": current.retry_count + 1,
                    "requested_by": requested_by,
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Validation(format!(
                "media derivative {derivative_id} changed state during retry"
            ))
        })?
        .try_into()
    }

    pub async fn get_media_review_metadata(
        &self,
        asset_id: Uuid,
    ) -> AtelierResult<Option<MediaReviewMetadata>> {
        let bindings = AssetRefBinding {
            asset_ref: RecordId::new("atelier_media_asset", SurrealUuid::from(asset_id)),
        };
        let row: Option<MediaReviewMetadataRow> = self.store().with_data_operation(move |ctx| {
            Box::pin(async move {
                ctx.query_first(concat!("SELECT ", review_metadata_select!(), " FROM atelier_media_review_metadata WHERE asset_id = $asset_ref LIMIT 1;"), bindings).await
            })
        }).await?;
        row.map(TryInto::try_into).transpose()
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
        #[derive(SurrealValue)]
        struct AssetRefsBinding {
            asset_refs: Vec<RecordId>,
        }
        let asset_refs: Vec<RecordId> = asset_ids
            .iter()
            .copied()
            .map(|id| RecordId::new("atelier_media_asset", SurrealUuid::from(id)))
            .collect();
        let existing: Vec<SurrealUuid> = self.store().with_data_operation({
            let bindings = AssetRefsBinding { asset_refs: asset_refs.clone() };
            move |ctx| Box::pin(async move { ctx.query_values("SELECT VALUE asset_id FROM atelier_media_asset WHERE id IN $asset_refs;", bindings).await })
        }).await?;
        if existing.len() != asset_ids.len() {
            let existing: HashSet<Uuid> = existing.into_iter().map(Into::into).collect();
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

        let inputs = normalized_updates
            .iter()
            .map(|update| {
                let asset_ref =
                    RecordId::new("atelier_media_asset", SurrealUuid::from(update.asset_id));
                ReviewUpdateInput {
                    metadata_rid: RecordId::new(
                        "atelier_media_review_metadata",
                        SurrealUuid::from(update.asset_id),
                    ),
                    asset_ref,
                    favorite: update.favorite,
                    rating: i64::from(update.rating),
                    frontpage: update.frontpage,
                    carousel: update.carousel,
                    notes: update.notes.clone(),
                    review_status: update.review_status.clone(),
                }
            })
            .collect();
        let bindings = BulkReviewBindings {
            asset_refs,
            updates: inputs,
            requested_by: requested_by.to_owned(),
        };
        let rows: Vec<MediaReviewMetadataRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(BULK_REVIEW_UPDATE_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        let metadata: Vec<MediaReviewMetadata> = rows
            .into_iter()
            .map(TryInto::try_into)
            .collect::<AtelierResult<_>>()?;
        for persisted in &metadata {
            let update = normalized_updates
                .iter()
                .find(|update| update.asset_id == persisted.asset_id)
                .ok_or_else(|| {
                    AtelierError::Internal(
                        "bulk review result contained an unexpected asset".into(),
                    )
                })?;
            self.record_event(
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
        }

        let receipt = self
            .record_bulk_operation_receipt(
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
        Ok(BulkMediaReviewMetadataResult { receipt, metadata })
    }
}
