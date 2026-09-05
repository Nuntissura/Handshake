//! Intake / inbox sorting (MT-016): persistent intake batches and per-item
//! accept / reject / defer / skip / fail lanes for the operator's
//! "IntakeSorterView" flow.
//!
//! legacy source source: `app/backend/library.js` (`createIntakeBatch`,
//! `listIntakeBatches`, `getIntakeBatch`, `updateIntakeBatchItem`,
//! `classifyIntakeBatch`, `_normalizeIntakeStatus`) and `app/backend/db.js`
//! (`IntakeBatch` / `IntakeBatchItem` tables). Schema/behavior INTENT only;
//! the SQLite originals are not copied. Storage authority is the single
//! embedded SurrealDB Handshake store, with no legacy database fallback.
//!
//! Translated contract (the load-bearing invariants from legacy source):
//!   * Persistent batches: a scan produces a durable `atelier_intake_batch`
//!     plus one `atelier_intake_item` per source file; nothing is ephemeral.
//!   * Pending / accept / reject / defer / skip / fail lanes: legacy source's
//!     `pending` review lane becomes the untriaged `Pending` state. Rejected,
//!     skipped, and failed states also write idempotent audit rows.
//!   * Idempotency: re-scanning the same source is safe. A batch carries a
//!     unique `idempotency_key`; re-creating with the same key returns the
//!     existing batch. Items are unique per `(batch, source_path)`; re-adding
//!     the same source returns the existing item instead of duplicating it.
//!   * Source preservation / no silent deletes: rejecting an item only moves
//!     its lane; the row and its `source_path` are always retained so the
//!     original is never lost. There is no delete path in this module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use surrealdb::types::{RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{
    atelier_event_sql, collections::collections_event_family, event_ref_for_text,
    media::MEDIA_ORIGINAL_RETENTION_CLASS, reject_legacy_runtime_ref, search::normalize_tag,
    AtelierError, AtelierResult, AtelierStore, PreparedAtelierEvent, RecordEventBindings,
};

struct IntakeRow(serde_json::Map<String, serde_json::Value>);

impl IntakeRow {
    fn get<T, I>(&self, field: I) -> T
    where
        T: serde::de::DeserializeOwned,
        I: AsRef<str>,
    {
        let field = field.as_ref();
        serde_json::from_value(
            self.0
                .get(field)
                .unwrap_or_else(|| panic!("missing persisted intake field {field}"))
                .clone(),
        )
        .unwrap_or_else(|err| panic!("invalid persisted intake field {field}: {err}"))
    }
}

fn intake_row(value: serde_json::Value) -> AtelierResult<IntakeRow> {
    value
        .as_object()
        .cloned()
        .map(IntakeRow)
        .ok_or_else(|| AtelierError::Internal("intake query returned a non-object row".to_owned()))
}

pub const ORPHAN_MANIFEST_SCHEMA_ID: &str = "hsk.atelier.orphan_manifest@1";

/// Intake event families (extends the MT-005 coverage set). The parent wires
/// these into [`super::event_family::ALL`].
pub mod intake_event_family {
    /// A persistent intake batch was opened.
    pub const INTAKE_BATCH_CREATED: &str = "atelier.intake.batch_created";
    /// A source file was registered as an item in a batch.
    pub const INTAKE_ITEM_ADDED: &str = "atelier.intake.item_added";
    /// An intake item acquired its immutable canonical Loom projection.
    pub const INTAKE_ITEM_LOOM_PROJECTION_LINKED: &str =
        "atelier.intake.item_loom_projection_linked";
    /// An item moved into a lifecycle state.
    pub const INTAKE_ITEM_CLASSIFIED: &str = "atelier.intake.item_classified";
    /// A negative item state wrote a durable rejection audit row.
    pub const INTAKE_ITEM_REJECTION_AUDITED: &str = "atelier.intake.item_rejection_audited";
    /// A batch was closed after its items were triaged.
    pub const INTAKE_BATCH_CLOSED: &str = "atelier.intake.batch_closed";
    /// A batch was resumed and marked in progress with a durable cursor.
    pub const INTAKE_BATCH_RESUMED: &str = "atelier.intake.batch_resumed";
    /// A configured inbox folder scan completed with summary counts.
    pub const INTAKE_FOLDER_SCAN_COMPLETED: &str = "atelier.intake.folder_scan_completed";
    /// A reset mode was executed with recoverable counts.
    pub const RESET_RECORDED: &str = "atelier.intake.reset_recorded";
    /// A full reset preserved original media in an orphan manifest.
    pub const ORPHAN_MANIFEST_RECORDED: &str = "atelier.intake.orphan_manifest_recorded";
    /// A retained orphan manifest item was adopted back into intake.
    pub const ORPHAN_MANIFEST_ITEM_ADOPTED: &str = "atelier.intake.orphan_manifest_item_adopted";

    /// Intake event families, exported for parity/coverage proofs.
    pub const ALL: &[&str] = &[
        INTAKE_BATCH_CREATED,
        INTAKE_ITEM_ADDED,
        INTAKE_ITEM_LOOM_PROJECTION_LINKED,
        INTAKE_ITEM_CLASSIFIED,
        INTAKE_ITEM_REJECTION_AUDITED,
        INTAKE_BATCH_CLOSED,
        INTAKE_BATCH_RESUMED,
        INTAKE_FOLDER_SCAN_COMPLETED,
        RESET_RECORDED,
        ORPHAN_MANIFEST_RECORDED,
        ORPHAN_MANIFEST_ITEM_ADOPTED,
    ];
}

const MAX_INBOX_FOLDER_SCAN_FILES: usize = 1000;

/// Lifecycle lane for an intake item. `Pending` is the untriaged inbox lane.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IntakeLane {
    Pending,
    Accepted,
    Rejected,
    Deferred,
    Skipped,
    Failed,
}

impl IntakeLane {
    /// Canonical lowercase database token.
    pub fn as_str(self) -> &'static str {
        match self {
            IntakeLane::Pending => "pending",
            IntakeLane::Accepted => "accepted",
            IntakeLane::Rejected => "rejected",
            IntakeLane::Deferred => "deferred",
            IntakeLane::Skipped => "skipped",
            IntakeLane::Failed => "failed",
        }
    }

    /// Parse a lane token, accepting legacy aliases (`new` -> pending,
    /// `pass`/`accept` -> accepted, `reject` -> rejected).
    pub fn parse(raw: &str) -> AtelierResult<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pending" | "new" => Ok(IntakeLane::Pending),
            "accepted" | "accept" | "pass" => Ok(IntakeLane::Accepted),
            "rejected" | "reject" => Ok(IntakeLane::Rejected),
            "deferred" | "defer" => Ok(IntakeLane::Deferred),
            "skipped" | "skip" => Ok(IntakeLane::Skipped),
            "failed" | "fail" => Ok(IntakeLane::Failed),
            other => Err(AtelierError::Validation(format!(
                "intake lane must be pending/accepted/rejected/deferred/skipped/failed, got {other:?}"
            ))),
        }
    }

    fn requires_rejection_audit(self) -> bool {
        matches!(
            self,
            IntakeLane::Rejected | IntakeLane::Skipped | IntakeLane::Failed
        )
    }
}

/// Lifecycle status of a persistent intake batch.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Open and accepting triage decisions.
    Open,
    /// A route/session has resumed work on the batch.
    InProgress,
    /// All triage complete with no leftover `New` items.
    Closed,
}

impl BatchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BatchStatus::Open => "open",
            BatchStatus::InProgress => "in_progress",
            BatchStatus::Closed => "closed",
        }
    }

    fn parse(raw: &str) -> BatchStatus {
        match raw.trim().to_ascii_lowercase().as_str() {
            "in_progress" => BatchStatus::InProgress,
            "closed" => BatchStatus::Closed,
            _ => BatchStatus::Open,
        }
    }
}

/// Source mode for an intake batch. This keeps "loose manual batch" and
/// folder/sourcing-run imports distinguishable after reconnect.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntakeBatchMode {
    Manual,
    FolderScan,
    SourcingRun,
}

impl IntakeBatchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            IntakeBatchMode::Manual => "manual",
            IntakeBatchMode::FolderScan => "folder_scan",
            IntakeBatchMode::SourcingRun => "sourcing_run",
        }
    }

    fn parse(raw: &str) -> IntakeBatchMode {
        match raw.trim().to_ascii_lowercase().as_str() {
            "folder_scan" => IntakeBatchMode::FolderScan,
            "sourcing_run" => IntakeBatchMode::SourcingRun,
            _ => IntakeBatchMode::Manual,
        }
    }
}

/// Linkage mode for where accepted intake items should land. This is separate
/// from [`IntakeBatchMode`], which describes the source mechanism.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IntakeProfileMode {
    LooseProfile,
    CharacterLinked,
}

impl IntakeProfileMode {
    pub fn as_str(self) -> &'static str {
        match self {
            IntakeProfileMode::LooseProfile => "loose_profile",
            IntakeProfileMode::CharacterLinked => "character_linked",
        }
    }

    fn parse(raw: &str) -> IntakeProfileMode {
        match raw.trim().to_ascii_lowercase().as_str() {
            "character_linked" | "linked" | "character" => IntakeProfileMode::CharacterLinked,
            _ => IntakeProfileMode::LooseProfile,
        }
    }
}

/// A persistent intake batch produced by a scan of a source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeBatch {
    pub batch_id: Uuid,
    /// Stable operator-facing label for the source scan (e.g. a folder path or
    /// sourcing-run id). Unique; doubles as the idempotency key.
    pub idempotency_key: String,
    /// Human-facing description of where the batch came from.
    pub source_label: String,
    /// Stable source reference used for resume/replay without persisting raw
    /// local paths.
    pub source_ref: String,
    pub mode: IntakeBatchMode,
    pub profile_mode: IntakeProfileMode,
    /// Optional owning character (FK to `atelier_character.internal_id`).
    pub character_internal_id: Option<Uuid>,
    pub target_character_id: Option<Uuid>,
    pub target_sheet_version_id: Option<Uuid>,
    pub target_collection_id: Option<Uuid>,
    pub status: BatchStatus,
    pub resume_cursor: Option<String>,
    pub resumed_at_utc: Option<DateTime<Utc>>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// One source file registered inside a batch. Always retains `source_path` so
/// the original is never lost (no silent deletes).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeItem {
    pub item_id: Uuid,
    pub batch_id: Uuid,
    /// Preserved path/URI of the original source; never mutated by triage.
    pub source_path: String,
    pub file_name: String,
    pub byte_len: i64,
    /// Optional content hash for cross-item dedup hints.
    pub content_hash: Option<String>,
    pub lane: IntakeLane,
    /// Free-form reason captured when an item is rejected/deferred.
    pub lane_reason: Option<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Durable identity bridge consumed by native editor/canvas drag payloads.
/// The relation is immutable: an Atelier item cannot silently move to a
/// different Loom block after a document or canvas has stored the reference.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeItemLoomProjection {
    pub item_id: Uuid,
    pub loom_block_id: String,
    pub workspace_id: String,
    pub linked_by: String,
    pub linked_at_utc: DateTime<Utc>,
}

/// Durable audit row for negative intake lifecycle outcomes. Rejected,
/// skipped, and failed states are auditable and idempotent by
/// `(item_id, lane, reason)`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeItemRejectionAudit {
    pub audit_id: Uuid,
    pub item_id: Uuid,
    pub batch_id: Uuid,
    pub lane: IntakeLane,
    pub reason: String,
    pub source_path_ref: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to open (or idempotently re-open) a batch.
#[derive(Clone, Debug)]
pub struct NewIntakeBatch {
    pub idempotency_key: String,
    pub source_label: String,
    pub source_ref: Option<String>,
    pub mode: IntakeBatchMode,
    pub profile_mode: IntakeProfileMode,
    pub character_internal_id: Option<Uuid>,
    pub target_character_id: Option<Uuid>,
    pub target_sheet_version_id: Option<Uuid>,
    pub target_collection_id: Option<Uuid>,
    pub resume_cursor: Option<String>,
}

/// Input to register a source file as an intake item.
#[derive(Clone, Debug)]
pub struct NewIntakeItem {
    pub source_path: String,
    pub file_name: String,
    pub byte_len: i64,
    pub content_hash: Option<String>,
}

/// Transactional intake decision apply. This promotes the low-level lane
/// change into the media-facing workflow: accepted items resolve their media
/// asset by `content_hash` and are attached to the batch target collection when
/// one exists.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeClassificationContactSheetMetadata {
    pub rows: Option<i64>,
    pub columns: Option<i64>,
    pub dpi: Option<i64>,
    pub cells: Option<i64>,
}

/// Dataset-mining metadata attached to an intake decision (CKC MT-017). The
/// durable copy lives on `atelier_intake_item_metadata`, keyed by the item.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeClassificationMetadata {
    pub request_id: Option<String>,
    pub batch_id: Option<String>,
    pub dataset_ref: Option<String>,
    pub character_ref: Option<String>,
    #[serde(default)]
    pub link_passed: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub event: Option<String>,
    pub date: Option<String>,
    pub location: Option<String>,
    pub facial_profile: Option<String>,
    pub loaded_item_count: Option<i64>,
    pub contact_sheet: Option<IntakeClassificationContactSheetMetadata>,
}

#[derive(Clone, Debug)]
pub struct ApplyIntakeClassificationRequest {
    pub item_id: Uuid,
    pub lane: IntakeLane,
    pub reason: Option<String>,
    pub requested_by: Option<String>,
    pub metadata: Option<IntakeClassificationMetadata>,
}

#[derive(Clone, Debug)]
pub struct ApplyIntakeBatchClassificationOverride {
    pub item_id: Uuid,
    pub lane: IntakeLane,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ApplyIntakeBatchClassificationsRequest {
    pub batch_id: Uuid,
    pub default_lane: IntakeLane,
    pub default_reason: Option<String>,
    pub requested_by: String,
    pub metadata: Option<IntakeClassificationMetadata>,
    pub overrides: Vec<ApplyIntakeBatchClassificationOverride>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeClassificationApplyResult {
    pub item: IntakeItem,
    pub asset_id: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    pub collection_inserted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeBatchClassificationFailure {
    pub item_id: Uuid,
    pub index: usize,
    pub error: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeBatchClassificationApplyResult {
    pub batch_id: Uuid,
    pub total_item_count: usize,
    pub applied: Vec<IntakeClassificationApplyResult>,
    pub failed: Option<IntakeBatchClassificationFailure>,
}

/// The durable per-item dataset-mining metadata row
/// (`atelier_intake_item_metadata`), read back for proofs and recovery.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeItemMetadata {
    pub item_id: Uuid,
    pub batch_id: Uuid,
    pub asset_id: Option<Uuid>,
    pub request_id: String,
    pub dataset_ref: Option<String>,
    pub character_ref: Option<String>,
    pub link_passed: bool,
    pub tags: Vec<String>,
    pub note: Option<String>,
    pub event_label: Option<String>,
    pub event_date: Option<String>,
    pub location: Option<String>,
    pub facial_profile: Option<String>,
    pub loaded_item_count: Option<i64>,
    pub contact_sheet: Option<IntakeClassificationContactSheetMetadata>,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AtelierResetMode {
    PreferencesOnly,
    FullPreserveOriginalMedia,
}

impl AtelierResetMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AtelierResetMode::PreferencesOnly => "preferences_only",
            AtelierResetMode::FullPreserveOriginalMedia => "full_preserve_original_media",
        }
    }

    fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "preferences_only" => Ok(AtelierResetMode::PreferencesOnly),
            "full_preserve_original_media" => Ok(AtelierResetMode::FullPreserveOriginalMedia),
            other => Err(AtelierError::Validation(format!(
                "unsupported atelier reset mode: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AtelierResetRequest {
    pub mode: AtelierResetMode,
    pub requested_by: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AtelierResetRecord {
    pub reset_id: Uuid,
    pub mode: AtelierResetMode,
    pub requested_by: String,
    pub reason: String,
    pub preferences_deleted_count: i64,
    pub original_media_preserved_count: i64,
    pub orphan_manifest_id: Option<Uuid>,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OrphanAdoptionStatus {
    Orphaned,
    Adopted,
}

impl OrphanAdoptionStatus {
    fn as_str(self) -> &'static str {
        match self {
            OrphanAdoptionStatus::Orphaned => "orphaned",
            OrphanAdoptionStatus::Adopted => "adopted",
        }
    }

    fn parse(raw: &str) -> AtelierResult<Self> {
        match raw {
            "orphaned" => Ok(OrphanAdoptionStatus::Orphaned),
            "adopted" => Ok(OrphanAdoptionStatus::Adopted),
            other => Err(AtelierError::Validation(format!(
                "unsupported orphan adoption status: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanManifestItem {
    pub manifest_item_id: Uuid,
    pub manifest_id: Uuid,
    pub asset_id: Uuid,
    pub content_hash: String,
    pub artifact_ref: String,
    pub mime: String,
    pub byte_len: i64,
    pub retention_class: String,
    pub adoption_status: OrphanAdoptionStatus,
    pub adopted_batch_id: Option<Uuid>,
    pub adopted_item_id: Option<Uuid>,
    pub adopted_by: Option<String>,
    pub adopted_at_utc: Option<DateTime<Utc>>,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct OrphanAdoptionRequest {
    pub manifest_item_id: Uuid,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanAdoptionResult {
    pub manifest_item: OrphanManifestItem,
    pub batch: IntakeBatch,
    pub item: IntakeItem,
}

/// Request to scan a configured inbox directory and register image files as
/// pending intake items. `inbox_root` is an operator-provided runtime input and
/// is not persisted as a raw local path; persisted rows use portable
/// `source://operator-inbox/...` refs derived from the batch key and file name.
#[derive(Clone, Debug)]
pub struct InboxFolderScanRequest {
    pub idempotency_key: String,
    pub inbox_root: PathBuf,
    pub source_label: String,
    pub character_internal_id: Option<Uuid>,
    pub max_files: usize,
    pub requested_by: String,
}

/// Summary returned after one folder scan action.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InboxFolderScanResult {
    pub batch: IntakeBatch,
    pub items: Vec<IntakeItem>,
    pub requested_max_files: i64,
    pub effective_max_files: i64,
    pub image_candidate_count: i64,
    pub imported_count: i64,
    pub duplicate_skipped_count: i64,
    pub skipped_over_max_count: i64,
    pub skipped_non_image_count: i64,
    pub skipped_subdir_count: i64,
    pub skipped_special_count: i64,
}

/// Per-lane counts for a batch, used by the sorter view header.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IntakeLaneCounts {
    pub pending: i64,
    pub accepted: i64,
    pub rejected: i64,
    pub deferred: i64,
    pub skipped: i64,
    pub failed: i64,
}

struct PendingInboxFile {
    file_name: String,
    source_path: String,
    byte_len: i64,
    content_hash: String,
}

struct InboxFolderEnumeration {
    root_path_ref: String,
    files: Vec<PendingInboxFile>,
    image_candidate_count: i64,
    skipped_non_image_count: i64,
    skipped_subdir_count: i64,
    skipped_special_count: i64,
}

#[derive(Clone, Copy, Debug)]
struct NormalizedBatchTargets {
    character_internal_id: Option<Uuid>,
    target_character_id: Option<Uuid>,
    target_sheet_version_id: Option<Uuid>,
    target_collection_id: Option<Uuid>,
}

fn require_scan_text<'a>(field: &str, value: &'a str) -> AtelierResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(trimmed)
}

fn inbox_io_error(field: &str, path: &Path, error: std::io::Error) -> AtelierError {
    AtelierError::Validation(format!("{field} {}: {error}", path.display()))
}

fn image_mime_for_path(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn advisory_lock_key(scope: &str) -> i64 {
    let digest = Sha256::digest(scope.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

fn checked_usize_to_i64(field: &str, value: usize) -> AtelierResult<i64> {
    i64::try_from(value).map_err(|_| AtelierError::Validation(format!("{field} exceeds i64 range")))
}

fn scan_source_path(root_path_ref: &str, idempotency_key: &str, file_name: &str) -> String {
    let root_ref = root_path_ref
        .strip_prefix("sha256:")
        .unwrap_or(root_path_ref)
        .to_string();
    let batch_ref = sha256_hex(idempotency_key.as_bytes());
    let file_ref = sha256_hex(file_name.as_bytes());
    format!(
        "source://operator-inbox/{}/{}/{}",
        &root_ref[..16],
        &batch_ref[..16],
        file_ref
    )
}

fn normalize_optional_batch_ref(
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

fn normalize_lane_reason(lane: IntakeLane, reason: Option<&str>) -> AtelierResult<Option<String>> {
    match reason {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed != raw {
                return Err(AtelierError::Validation(
                    "lane_reason must not be empty or padded".into(),
                ));
            }
            reject_legacy_runtime_ref("lane_reason", raw)?;
            Ok(Some(raw.to_string()))
        }
        None if lane.requires_rejection_audit() => Err(AtelierError::Validation(format!(
            "{} intake items require a rejection audit reason",
            lane.as_str()
        ))),
        None => Ok(None),
    }
}

fn normalize_batch_source_refs(new: &NewIntakeBatch) -> AtelierResult<(Option<String>, String)> {
    let explicit_source_ref = normalize_optional_batch_ref("source_ref", &new.source_ref)?;
    let effective_source_ref = explicit_source_ref
        .clone()
        .unwrap_or_else(|| event_ref_for_text(&new.source_label));
    Ok((explicit_source_ref, effective_source_ref))
}

fn normalize_batch_targets(new: &NewIntakeBatch) -> AtelierResult<NormalizedBatchTargets> {
    match new.profile_mode {
        IntakeProfileMode::LooseProfile => {
            if new.character_internal_id.is_some()
                || new.target_character_id.is_some()
                || new.target_sheet_version_id.is_some()
                || new.target_collection_id.is_some()
            {
                return Err(AtelierError::Validation(
                    "loose_profile intake batches must not carry character/sheet/collection targets"
                        .into(),
                ));
            }
            Ok(NormalizedBatchTargets {
                character_internal_id: None,
                target_character_id: None,
                target_sheet_version_id: None,
                target_collection_id: None,
            })
        }
        IntakeProfileMode::CharacterLinked => {
            let target_character_id = new
                .target_character_id
                .or(new.character_internal_id)
                .ok_or_else(|| {
                    AtelierError::Validation(
                        "character_linked intake batches require target_character_id".into(),
                    )
                })?;
            if let Some(character_internal_id) = new.character_internal_id {
                if character_internal_id != target_character_id {
                    return Err(AtelierError::Validation(
                        "character_internal_id must match target_character_id".into(),
                    ));
                }
            }
            Ok(NormalizedBatchTargets {
                character_internal_id: Some(target_character_id),
                target_character_id: Some(target_character_id),
                target_sheet_version_id: new.target_sheet_version_id,
                target_collection_id: new.target_collection_id,
            })
        }
    }
}

fn validate_intake_batch_reopen_contract(
    existing: &IntakeBatch,
    new: &NewIntakeBatch,
    explicit_source_ref: Option<&str>,
    targets: NormalizedBatchTargets,
) -> AtelierResult<()> {
    let mut mismatches = Vec::new();
    if let Some(source_ref) = explicit_source_ref {
        if existing.source_ref != source_ref {
            mismatches.push("source_ref");
        }
    }
    if existing.mode != new.mode {
        mismatches.push("mode");
    }
    if existing.profile_mode != new.profile_mode {
        mismatches.push("profile_mode");
    }
    if existing.character_internal_id != targets.character_internal_id {
        mismatches.push("character_internal_id");
    }
    if existing.target_character_id != targets.target_character_id {
        mismatches.push("target_character_id");
    }
    if existing.target_sheet_version_id != targets.target_sheet_version_id {
        mismatches.push("target_sheet_version_id");
    }
    if existing.target_collection_id != targets.target_collection_id {
        mismatches.push("target_collection_id");
    }
    if mismatches.is_empty() {
        return Ok(());
    }
    if mismatches.len() == 1
        && mismatches[0] == "source_ref"
        && existing.mode == IntakeBatchMode::FolderScan
        && new.mode == IntakeBatchMode::FolderScan
    {
        return Err(AtelierError::Validation(
            "inbox_root does not match the previous folder scan for this idempotency_key".into(),
        ));
    }
    Err(AtelierError::Validation(format!(
        "incompatible intake batch idempotency_key {}: {}",
        existing.idempotency_key,
        mismatches.join(", ")
    )))
}

fn is_surreal_unique_index_conflict(error: &AtelierError, index_name: &str) -> bool {
    let text = error.to_string();
    text.contains("Database index")
        && text.contains(index_name)
        && text.contains("already contains")
}

const SURREAL_TRANSACTION_MAX_ATTEMPTS: usize = 10;
const SURREAL_TRANSACTION_BACKOFF_CAP_MS: u64 = 32;

fn is_surreal_retryable_transaction_conflict(error: &AtelierError) -> bool {
    matches!(
        error,
        AtelierError::Database(crate::storage::surreal::SurrealStorageError::Database(source))
            if source
                .to_string()
                .contains("Transaction conflict: Resource busy. This transaction can be retried")
    )
}

fn surreal_transaction_retry_delay(seed: Uuid, failed_attempt: usize) -> Duration {
    let exponential_cap = 1_u64
        .checked_shl(failed_attempt.min(5) as u32)
        .unwrap_or(SURREAL_TRANSACTION_BACKOFF_CAP_MS)
        .min(SURREAL_TRANSACTION_BACKOFF_CAP_MS);
    let seed = seed.as_u128();
    let mut mixed = (seed as u64)
        ^ ((seed >> 64) as u64)
        ^ (failed_attempt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed ^= mixed >> 30;
    mixed = mixed.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed ^= mixed >> 27;
    mixed = mixed.wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^= mixed >> 31;
    Duration::from_millis(mixed % (exponential_cap + 1))
}

async fn wait_before_surreal_transaction_retry(seed: Uuid, failed_attempt: usize) {
    tokio::time::sleep(surreal_transaction_retry_delay(seed, failed_attempt)).await;
}

fn enumerate_inbox_folder(
    request: &InboxFolderScanRequest,
    effective_max_files: usize,
) -> AtelierResult<InboxFolderEnumeration> {
    let metadata = fs::metadata(&request.inbox_root)
        .map_err(|error| inbox_io_error("inbox_root", &request.inbox_root, error))?;
    if !metadata.is_dir() {
        return Err(AtelierError::Validation(format!(
            "inbox_root {} must be a directory",
            request.inbox_root.display()
        )));
    }
    let canonical_root = fs::canonicalize(&request.inbox_root)
        .map_err(|error| inbox_io_error("inbox_root", &request.inbox_root, error))?;
    let root_path_display = canonical_root.display().to_string();
    let root_path_ref = event_ref_for_text(&root_path_display);

    let mut image_paths = Vec::new();
    let mut skipped_non_image_count = 0_i64;
    let mut skipped_subdir_count = 0_i64;
    let mut skipped_special_count = 0_i64;

    for entry in fs::read_dir(&request.inbox_root)
        .map_err(|error| inbox_io_error("inbox_root", &request.inbox_root, error))?
    {
        let entry =
            entry.map_err(|error| inbox_io_error("inbox_root", &request.inbox_root, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| inbox_io_error("inbox_root entry", &entry.path(), error))?;
        if file_type.is_dir() {
            skipped_subdir_count += 1;
            continue;
        }
        if !file_type.is_file() {
            skipped_special_count += 1;
            continue;
        }
        if image_mime_for_path(&entry.path()).is_none() {
            skipped_non_image_count += 1;
            continue;
        }
        image_paths.push(entry.path());
    }

    image_paths.sort_by(|left, right| {
        let left_name = left
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let right_name = right
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        left_name
            .cmp(&right_name)
            .then_with(|| left.file_name().cmp(&right.file_name()))
    });

    let image_candidate_count = image_paths.len() as i64;
    let mut files = Vec::new();
    for path in image_paths.into_iter().take(effective_max_files) {
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| {
                AtelierError::Validation(format!(
                    "inbox file {} must have a file name",
                    path.display()
                ))
            })?;
        let bytes = fs::read(&path).map_err(|error| inbox_io_error("inbox file", &path, error))?;
        let byte_len = i64::try_from(bytes.len()).map_err(|_| {
            AtelierError::Validation(format!(
                "inbox file {} exceeds i64 byte length",
                path.display()
            ))
        })?;
        files.push(PendingInboxFile {
            source_path: scan_source_path(&root_path_ref, &request.idempotency_key, &file_name),
            file_name,
            byte_len,
            content_hash: format!("sha256:{}", sha256_hex(&bytes)),
        });
    }

    Ok(InboxFolderEnumeration {
        root_path_ref,
        files,
        image_candidate_count,
        skipped_non_image_count,
        skipped_subdir_count,
        skipped_special_count,
    })
}

fn batch_from_row(row: &IntakeRow) -> IntakeBatch {
    let status: String = row.get("status");
    let mode: String = row.get("mode");
    let profile_mode: String = row.get("profile_mode");
    IntakeBatch {
        batch_id: row.get("batch_id"),
        idempotency_key: row.get("idempotency_key"),
        source_label: row.get("source_label"),
        source_ref: row.get("source_ref"),
        mode: IntakeBatchMode::parse(&mode),
        profile_mode: IntakeProfileMode::parse(&profile_mode),
        character_internal_id: row.get("character_internal_id"),
        target_character_id: row.get("target_character_id"),
        target_sheet_version_id: row.get("target_sheet_version_id"),
        target_collection_id: row.get("target_collection_id"),
        status: BatchStatus::parse(&status),
        resume_cursor: row.get("resume_cursor"),
        resumed_at_utc: row.get("resumed_at_utc"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn item_from_row(row: &IntakeRow) -> AtelierResult<IntakeItem> {
    let lane: String = row.get("lane");
    Ok(IntakeItem {
        item_id: row.get("item_id"),
        batch_id: row.get("batch_id"),
        source_path: row.get("source_path"),
        file_name: row.get("file_name"),
        byte_len: row.get("byte_len"),
        content_hash: row.get("content_hash"),
        lane: IntakeLane::parse(&lane)?,
        lane_reason: row.get("lane_reason"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn rejection_audit_from_row(row: &IntakeRow) -> AtelierResult<IntakeItemRejectionAudit> {
    let lane: String = row.get("lane");
    Ok(IntakeItemRejectionAudit {
        audit_id: row.get("audit_id"),
        item_id: row.get("item_id"),
        batch_id: row.get("batch_id"),
        lane: IntakeLane::parse(&lane)?,
        reason: row.get("reason"),
        source_path_ref: row.get("source_path_ref"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn reset_from_row(row: &IntakeRow) -> AtelierResult<AtelierResetRecord> {
    let mode: String = row.get("mode");
    Ok(AtelierResetRecord {
        reset_id: row.get("reset_id"),
        mode: AtelierResetMode::parse(&mode)?,
        requested_by: row.get("requested_by"),
        reason: row.get("reason"),
        preferences_deleted_count: row.get("preferences_deleted_count"),
        original_media_preserved_count: row.get("original_media_preserved_count"),
        orphan_manifest_id: row.get("orphan_manifest_id"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn orphan_manifest_item_from_row(row: &IntakeRow) -> AtelierResult<OrphanManifestItem> {
    let adoption_status: String = row.get("adoption_status");
    Ok(OrphanManifestItem {
        manifest_item_id: row.get("manifest_item_id"),
        manifest_id: row.get("manifest_id"),
        asset_id: row.get("asset_id"),
        content_hash: row.get("content_hash"),
        artifact_ref: row.get("artifact_ref"),
        mime: row.get("mime"),
        byte_len: row.get("byte_len"),
        retention_class: row.get("retention_class"),
        adoption_status: OrphanAdoptionStatus::parse(&adoption_status)?,
        adopted_batch_id: row.get("adopted_batch_id"),
        adopted_item_id: row.get("adopted_item_id"),
        adopted_by: row.get("adopted_by"),
        adopted_at_utc: row.get("adopted_at_utc"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn orphan_intake_file_name(content_hash: &str, mime: &str) -> String {
    let hash = content_hash.strip_prefix("sha256:").unwrap_or(content_hash);
    let short_hash: String = hash.chars().take(12).collect();
    let extension = match mime {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "png",
    };
    format!("orphan-original-{short_hash}.{extension}")
}

#[derive(SurrealValue)]
struct NoBindings {}

#[derive(SurrealValue)]
struct IdempotencyKeyBinding {
    idempotency_key: String,
}

#[derive(SurrealValue)]
struct BatchRefBinding {
    batch_ref: RecordId,
}

#[derive(SurrealValue)]
struct BatchListBinding {
    batch_ref: RecordId,
    lane: Option<String>,
    limit: i64,
}

#[derive(SurrealValue)]
struct ItemRefBinding {
    item_ref: RecordId,
}

#[derive(SurrealValue)]
struct ItemLookupBinding {
    batch_ref: RecordId,
    source_path: String,
}

#[derive(SurrealValue)]
struct ManifestRefBinding {
    manifest_ref: RecordId,
}

#[derive(SurrealValue)]
struct OrphanItemRefBinding {
    manifest_item_ref: RecordId,
}

#[derive(SurrealValue)]
struct ContentHashBinding {
    content_hash: String,
}

#[derive(SurrealValue)]
struct LoomBlockBinding {
    loom_block_ref: RecordId,
}

#[derive(SurrealValue)]
struct ProjectionRefBinding {
    projection_ref: RecordId,
}

#[derive(SurrealValue)]
struct AuditLookupBinding {
    item_ref: RecordId,
    lane: String,
    reason: String,
}

#[derive(Clone, SurrealValue)]
struct LoomProjectionBindings {
    projection_ref: RecordId,
    item_ref: RecordId,
    loom_block_ref: RecordId,
    workspace_ref: RecordId,
    linked_by: String,
}

#[derive(Clone, SurrealValue)]
struct BatchWriteBindings {
    batch_ref: RecordId,
    batch_id: SurrealUuid,
    idempotency_key: String,
    source_label: String,
    source_ref: String,
    mode: String,
    profile_mode: String,
    character_internal_id: Option<RecordId>,
    target_character_id: Option<RecordId>,
    target_sheet_version_id: Option<RecordId>,
    target_collection_id: Option<RecordId>,
    resume_cursor: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct BatchResumeBindings {
    batch_ref: RecordId,
    resume_cursor: String,
}

#[derive(Clone, SurrealValue)]
struct ItemWriteBindings {
    item_ref: RecordId,
    item_id: SurrealUuid,
    batch_ref: RecordId,
    source_path: String,
    file_name: String,
    byte_len: i64,
    content_hash: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct ItemClassificationBindings {
    item_ref: RecordId,
    batch_ref: RecordId,
    lane: String,
    lane_reason: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct RejectionAuditBindings {
    audit_ref: RecordId,
    audit_id: SurrealUuid,
    item_ref: RecordId,
    batch_ref: RecordId,
    lane: String,
    reason: String,
    source_path_ref: String,
}

#[derive(Clone, SurrealValue)]
struct OrphanAdoptionBindings {
    manifest_item_ref: RecordId,
    batch_ref: RecordId,
    item_ref: RecordId,
    adopted_by: String,
}

#[derive(Clone, SurrealValue)]
struct ResetBindings {
    reset_ref: RecordId,
    reset_id: SurrealUuid,
    mode: String,
    requested_by: String,
    reason: String,
    preserve_original_media: bool,
    manifest_ref: RecordId,
    manifest_id: SurrealUuid,
    manifest_json: serde_json::Value,
    preferences_deleted_count: i64,
    original_media_preserved_count: i64,
    media_items: Vec<ResetMediaInput>,
}

#[derive(Clone, SurrealValue)]
struct ResetMediaInput {
    manifest_item_ref: RecordId,
    manifest_item_id: SurrealUuid,
    asset_ref: RecordId,
    content_hash: String,
    artifact_ref: String,
    mime: String,
    byte_len: i64,
    retention_class: String,
}

macro_rules! batch_columns {
    () => {
        "batch_id, idempotency_key, source_label, source_ref, mode, profile_mode, \
         IF character_internal_id = NONE { NONE } ELSE { record::id(character_internal_id) } AS character_internal_id, \
         IF target_character_id = NONE { NONE } ELSE { record::id(target_character_id) } AS target_character_id, \
         IF target_sheet_version_id = NONE { NONE } ELSE { record::id(target_sheet_version_id) } AS target_sheet_version_id, \
         IF target_collection_id = NONE { NONE } ELSE { record::id(target_collection_id) } AS target_collection_id, status, resume_cursor, \
         resumed_at_utc, created_at_utc, updated_at_utc"
    };
}

macro_rules! item_columns {
    () => {
        "item_id, record::id(batch_id) AS batch_id, source_path, file_name, byte_len, \
         content_hash, lane, lane_reason, created_at_utc, updated_at_utc"
    };
}

macro_rules! audit_columns {
    () => {
        "audit_id, record::id(item_id) AS item_id, record::id(batch_id) AS batch_id, lane, \
         reason, source_path_ref, created_at_utc"
    };
}

macro_rules! orphan_item_columns {
    () => {
        "manifest_item_id, record::id(manifest_id) AS manifest_id, record::id(asset_id) AS asset_id, \
         content_hash, artifact_ref, mime, byte_len, retention_class, adoption_status, \
         IF adopted_batch_id = NONE { NONE } ELSE { record::id(adopted_batch_id) } AS adopted_batch_id, \
         IF adopted_item_id = NONE { NONE } ELSE { record::id(adopted_item_id) } AS adopted_item_id, \
         adopted_by, adopted_at_utc, created_at_utc"
    };
}

const GET_BATCH_BY_KEY_STATEMENT: &str = concat!(
    "SELECT ",
    batch_columns!(),
    " FROM atelier_intake_batch WHERE idempotency_key = $idempotency_key LIMIT 1;"
);
const GET_BATCH_STATEMENT: &str = concat!("SELECT ", batch_columns!(), " FROM $batch_ref LIMIT 1;");
const LIST_BATCHES_STATEMENT: &str = concat!(
    "SELECT ",
    batch_columns!(),
    " FROM atelier_intake_batch ORDER BY updated_at_utc DESC;"
);
const WRITE_BATCH_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.batch_ref; \
       CREATE $rid CONTENT { batch_id: $domain.batch_id, idempotency_key: $domain.idempotency_key, \
         source_label: $domain.source_label, source_ref: $domain.source_ref, mode: $domain.mode, \
         profile_mode: $domain.profile_mode, character_internal_id: $domain.character_internal_id, \
         target_character_id: $domain.target_character_id, \
         target_sheet_version_id: $domain.target_sheet_version_id, \
         target_collection_id: $domain.target_collection_id, status: 'open', \
         resume_cursor: $domain.resume_cursor }; ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    batch_columns!(),
    " FROM $rid)[0]; };"
);
const RESUME_BATCH_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.batch_ref; \
       IF (SELECT VALUE status FROM $rid LIMIT 1)[0] IN [NONE, 'closed'] { \
         THROW 'open intake batch not found'; }; \
       UPDATE $rid SET status = 'in_progress', resume_cursor = $domain.resume_cursor, \
         resumed_at_utc = time::now(), updated_at_utc = time::now(); ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    batch_columns!(),
    " FROM $rid)[0]; };"
);
const CLOSE_BATCH_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.batch_ref; \
       IF !record::exists($rid) { THROW 'intake batch not found'; }; \
       UPDATE $rid SET status = 'closed', updated_at_utc = time::now(); ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    batch_columns!(),
    " FROM $rid)[0]; };"
);

const GET_ITEM_STATEMENT: &str = concat!(
    "SELECT ",
    item_columns!(),
    " FROM atelier_intake_item WHERE batch_id = $batch_ref AND source_path = $source_path LIMIT 1;"
);
const GET_ITEM_BY_REF_STATEMENT: &str =
    concat!("SELECT ", item_columns!(), " FROM $item_ref LIMIT 1;");
const LIST_ITEMS_STATEMENT: &str = concat!(
    "SELECT ",
    item_columns!(),
    " FROM atelier_intake_item WHERE batch_id = $batch_ref ORDER BY created_at_utc ASC, item_id ASC;"
);
const LIST_ITEMS_LIMITED_STATEMENT: &str = concat!(
    "SELECT ",
    item_columns!(),
    " FROM atelier_intake_item WHERE batch_id = $batch_ref \
     AND ($lane = NONE OR lane = $lane) \
     ORDER BY created_at_utc ASC LIMIT $limit;"
);
const WRITE_ITEM_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.item_ref; \
       IF !record::exists($domain.batch_ref) { THROW 'intake batch not found'; }; \
       CREATE $rid CONTENT { item_id: $domain.item_id, batch_id: $domain.batch_ref, \
         source_path: $domain.source_path, file_name: $domain.file_name, byte_len: $domain.byte_len, \
         content_hash: $domain.content_hash, lane: 'pending' }; \
       UPDATE $domain.batch_ref SET updated_at_utc = time::now(); ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    item_columns!(),
    " FROM $rid)[0]; };"
);
const CLASSIFY_ITEM_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.item_ref; \
       IF !record::exists($rid) { THROW 'intake item not found'; }; \
       UPDATE $rid SET lane = $domain.lane, lane_reason = $domain.lane_reason, \
         updated_at_utc = time::now(); \
       UPDATE $domain.batch_ref SET updated_at_utc = time::now(); ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    item_columns!(),
    " FROM $rid)[0]; };"
);

const LIST_AUDITS_STATEMENT: &str = concat!(
    "SELECT ",
    audit_columns!(),
    " FROM atelier_intake_item_rejection_audit WHERE batch_id = $batch_ref \
       ORDER BY created_at_utc ASC, audit_id ASC;"
);
const FIND_AUDIT_STATEMENT: &str = concat!(
    "SELECT ",
    audit_columns!(),
    " FROM atelier_intake_item_rejection_audit WHERE item_id = $item_ref \
       AND lane = $lane AND reason = $reason LIMIT 1;"
);
const WRITE_AUDIT_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.audit_ref; \
       CREATE $rid CONTENT { audit_id: $domain.audit_id, item_id: $domain.item_ref, \
         batch_id: $domain.batch_ref, lane: $domain.lane, reason: $domain.reason, \
         source_path_ref: $domain.source_path_ref }; ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    audit_columns!(),
    " FROM $rid)[0]; };"
);

const GET_ORPHAN_ITEM_STATEMENT: &str = concat!(
    "SELECT ",
    orphan_item_columns!(),
    " FROM $manifest_item_ref LIMIT 1;"
);
const LIST_ORPHAN_ITEMS_STATEMENT: &str = concat!(
    "SELECT ",
    orphan_item_columns!(),
    " FROM atelier_orphan_manifest_item WHERE manifest_id = $manifest_ref \
       ORDER BY created_at_utc ASC, manifest_item_id ASC;"
);
const ADOPT_ORPHAN_ITEM_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.manifest_item_ref; \
       IF (SELECT VALUE adoption_status FROM $rid LIMIT 1)[0] != 'orphaned' { \
         THROW 'orphan manifest item is no longer orphaned'; }; \
       UPDATE $rid SET adoption_status = 'adopted', adopted_batch_id = $domain.batch_ref, \
         adopted_item_id = $domain.item_ref, adopted_by = $domain.adopted_by, \
         adopted_at_utc = time::now(); ",
    atelier_event_sql!(),
    " RETURN (SELECT ",
    orphan_item_columns!(),
    " FROM $rid)[0]; };"
);

const GET_LOOM_BLOCK_STATEMENT: &str = "SELECT record::id(workspace_id) AS workspace_id, \
            (document_id != NONE OR asset_id != NONE) AS has_source \
     FROM $loom_block_ref LIMIT 1;";
const GET_LOOM_PROJECTION_STATEMENT: &str =
    "SELECT record::id(item_id) AS item_id, record::id(loom_block_id) AS loom_block_id, \
            record::id(workspace_id) AS workspace_id, linked_by, linked_at_utc \
     FROM $projection_ref LIMIT 1;";
const FIND_LOOM_PROJECTION_BY_BLOCK_STATEMENT: &str =
    "SELECT record::id(item_id) AS item_id FROM atelier_intake_item_loom_projection \
     WHERE loom_block_id = $loom_block_ref LIMIT 1;";
const WRITE_LOOM_PROJECTION_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.projection_ref; \
       CREATE $rid CONTENT { item_id: $domain.item_ref, loom_block_id: $domain.loom_block_ref, \
         workspace_id: $domain.workspace_ref, linked_by: $domain.linked_by }; ",
    atelier_event_sql!(),
    " RETURN (SELECT record::id(item_id) AS item_id, \
       record::id(loom_block_id) AS loom_block_id, record::id(workspace_id) AS workspace_id, \
       linked_by, linked_at_utc FROM $rid)[0]; };"
);

const COUNT_PREFERENCES_STATEMENT: &str =
    "RETURN array::len((SELECT VALUE id FROM atelier_preference));";
const LIST_ORIGINAL_MEDIA_STATEMENT: &str =
    "SELECT id AS asset_ref, content_hash, artifact_ref, mime, byte_len, retention_class \
     FROM atelier_media_asset WHERE retention_class = $retention_class \
       AND string::trim(content_hash) = content_hash AND content_hash != '' \
       AND string::trim(artifact_ref) = artifact_ref AND artifact_ref != '' \
       AND string::trim(mime) = mime AND mime != '' AND byte_len > 0 \
       AND string::trim(retention_class) = retention_class AND retention_class != '' \
     ORDER BY created_at_utc ASC, asset_id ASC;";

#[derive(SurrealValue)]
struct RetentionClassBinding {
    retention_class: String,
}

macro_rules! reset_columns {
    () => {
        "reset_id, mode, requested_by, reason, preferences_deleted_count, \
         original_media_preserved_count, orphan_manifest_id, created_at_utc"
    };
}

const WRITE_RESET_STATEMENT: &str = concat!(
    "RETURN { LET $rid = $domain.reset_ref; \
       DELETE atelier_preference; \
       CREATE $rid CONTENT { reset_id: $domain.reset_id, mode: $domain.mode, \
         requested_by: $domain.requested_by, reason: $domain.reason, \
         preferences_deleted_count: $domain.preferences_deleted_count, \
         original_media_preserved_count: $domain.original_media_preserved_count, \
         orphan_manifest_id: IF $domain.preserve_original_media { $domain.manifest_id } \
           ELSE { NONE } END }; \
       IF $domain.preserve_original_media { \
         CREATE $domain.manifest_ref CONTENT { manifest_id: $domain.manifest_id, reset_id: $rid, \
           manifest_json: $domain.manifest_json, item_count: $domain.original_media_preserved_count }; \
         FOR $item IN $domain.media_items { \
           CREATE $item.manifest_item_ref CONTENT { manifest_item_id: $item.manifest_item_id, \
             manifest_id: $domain.manifest_ref, asset_id: $item.asset_ref, \
             content_hash: $item.content_hash, artifact_ref: $item.artifact_ref, mime: $item.mime, \
             byte_len: $item.byte_len, retention_class: $item.retention_class }; \
         }; \
       }; ",
    atelier_event_sql!(),
    " RETURN (SELECT ", reset_columns!(), " FROM $rid)[0]; };"
);

// ---------------------------------------------------------------------------
// CKC MT-017 / MT-031: classification apply with dataset-mining metadata.
//
// PostgreSQL ran every item of a batch inside one `BEGIN ... COMMIT` with
// `FOR UPDATE` row locks and two advisory locks. On the embedded store the
// whole plan (lane changes, collection membership, rejection audits, metadata
// upserts and every event they emit) is ONE statement, so a batch commits
// together or not at all; the optimistic write-write conflict SurrealDB raises
// instead of a PG deadlock is retried a bounded number of times
// (MT-056 INTAKE_DEADLOCK_RETRY).
// ---------------------------------------------------------------------------

fn normalize_metadata_text(
    field: &str,
    value: &Option<String>,
    reject_runtime_ref: bool,
) -> AtelierResult<Option<String>> {
    match value.as_deref() {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed != raw {
                return Err(AtelierError::Validation(format!(
                    "metadata.{field} must not be empty or padded"
                )));
            }
            if reject_runtime_ref {
                reject_legacy_runtime_ref(&format!("metadata.{field}"), raw)?;
            }
            Ok(Some(raw.to_string()))
        }
    }
}

fn normalize_metadata_request_id(
    metadata: &IntakeClassificationMetadata,
    required: bool,
) -> AtelierResult<Option<String>> {
    let normalized = normalize_metadata_text("request_id", &metadata.request_id, true)?;
    if required && normalized.is_none() {
        return Err(AtelierError::Validation(
            "metadata.request_id is required for batch intake classification".into(),
        ));
    }
    Ok(normalized)
}

fn normalize_metadata_loaded_count(value: Option<i64>) -> AtelierResult<Option<i64>> {
    if let Some(count) = value {
        if count < 0 {
            return Err(AtelierError::Validation(
                "metadata.loaded_item_count must not be negative".into(),
            ));
        }
    }
    Ok(value)
}

fn normalize_contact_sheet_metadata(
    value: &Option<IntakeClassificationContactSheetMetadata>,
) -> AtelierResult<Option<IntakeClassificationContactSheetMetadata>> {
    let Some(value) = value else {
        return Ok(None);
    };
    for (field, candidate) in [
        ("rows", value.rows),
        ("columns", value.columns),
        ("dpi", value.dpi),
        ("cells", value.cells),
    ] {
        if let Some(candidate) = candidate {
            if candidate <= 0 {
                return Err(AtelierError::Validation(format!(
                    "metadata.contact_sheet.{field} must be positive"
                )));
            }
        }
    }
    Ok(Some(value.clone()))
}

fn normalize_classification_metadata(
    metadata: Option<&IntakeClassificationMetadata>,
    expected_batch_id: Option<Uuid>,
    canonical_item_count: Option<i64>,
    require_request_id: bool,
) -> AtelierResult<(Option<IntakeClassificationMetadata>, Option<String>)> {
    let Some(metadata) = metadata else {
        if require_request_id {
            return Err(AtelierError::Validation(
                "metadata with request_id is required for batch intake classification".into(),
            ));
        }
        return Ok((None, None));
    };

    let request_id = normalize_metadata_request_id(metadata, require_request_id)?;
    let mut batch_id = normalize_metadata_text("batch_id", &metadata.batch_id, true)?;
    if let Some(expected_batch_id) = expected_batch_id {
        let expected = expected_batch_id.to_string();
        if let Some(actual) = batch_id.as_deref() {
            if actual != expected {
                return Err(AtelierError::Validation(format!(
                    "metadata.batch_id {actual} does not match intake batch {expected}"
                )));
            }
        } else {
            batch_id = Some(expected);
        }
    }

    let mut tags = Vec::new();
    let mut seen_tags = BTreeSet::new();
    for raw_tag in &metadata.tags {
        let normalized = normalize_tag(raw_tag);
        if normalized.is_empty() {
            continue;
        }
        if seen_tags.insert(normalized.clone()) {
            tags.push(normalized);
        }
    }

    let loaded_item_count = match canonical_item_count {
        Some(count) => Some(count),
        None => normalize_metadata_loaded_count(metadata.loaded_item_count)?,
    };

    Ok((
        Some(IntakeClassificationMetadata {
            request_id: request_id.clone(),
            batch_id,
            dataset_ref: normalize_metadata_text("dataset_ref", &metadata.dataset_ref, true)?,
            character_ref: normalize_metadata_text("character_ref", &metadata.character_ref, true)?,
            link_passed: metadata.link_passed,
            tags,
            note: normalize_metadata_text("note", &metadata.note, false)?,
            event: normalize_metadata_text("event", &metadata.event, false)?,
            date: normalize_metadata_text("date", &metadata.date, false)?,
            location: normalize_metadata_text("location", &metadata.location, false)?,
            facial_profile: normalize_metadata_text(
                "facial_profile",
                &metadata.facial_profile,
                false,
            )?,
            loaded_item_count,
            contact_sheet: normalize_contact_sheet_metadata(&metadata.contact_sheet)?,
        }),
        request_id,
    ))
}

fn normalize_requested_by(value: Option<&str>) -> AtelierResult<Option<String>> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed != raw {
                return Err(AtelierError::Validation(
                    "requested_by must not be empty or padded".into(),
                ));
            }
            reject_legacy_runtime_ref("requested_by", raw)?;
            Some(raw.to_string()).map(Ok).transpose()
        }
    }
}

fn metadata_tags_from_json(value: Option<serde_json::Value>) -> Vec<String> {
    let Some(serde_json::Value::Array(values)) = value else {
        return Vec::new();
    };
    values
        .into_iter()
        .filter_map(|value| match value {
            serde_json::Value::String(text) => Some(normalize_tag(&text)),
            _ => None,
        })
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn intake_item_metadata_from_row(row: &IntakeRow) -> AtelierResult<IntakeItemMetadata> {
    let tags_json: Option<serde_json::Value> = row.get("tags_json");
    let contact_sheet_json: Option<serde_json::Value> = row.get("contact_sheet_json");
    let contact_sheet = contact_sheet_json
        .filter(|value| !value.is_null())
        .map(serde_json::from_value::<IntakeClassificationContactSheetMetadata>)
        .transpose()
        .map_err(|err| {
            AtelierError::Internal(format!(
                "persisted intake item metadata contact_sheet_json is malformed: {err}"
            ))
        })?;
    Ok(IntakeItemMetadata {
        item_id: row.get("item_id"),
        batch_id: row.get("batch_id"),
        asset_id: row.get("asset_id"),
        request_id: row.get("request_id"),
        dataset_ref: row.get("dataset_ref"),
        character_ref: row.get("character_ref"),
        link_passed: row.get("link_passed"),
        tags: metadata_tags_from_json(tags_json),
        note: row.get("note"),
        event_label: row.get("event_label"),
        event_date: row.get("event_date"),
        location: row.get("location"),
        facial_profile: row.get("facial_profile"),
        loaded_item_count: row.get("loaded_item_count"),
        contact_sheet,
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

#[derive(SurrealValue)]
struct MetadataRefBinding {
    metadata_ref: RecordId,
}

#[derive(SurrealValue)]
struct RequestBatchConflictBinding {
    request_id: String,
    batch_ref: RecordId,
}

#[derive(SurrealValue)]
struct RecordedRequestBinding {
    event_family: String,
    aggregate_id: String,
    request_id: String,
}

#[derive(SurrealValue)]
struct CollectionRefBinding {
    collection_ref: RecordId,
}

#[derive(SurrealValue)]
struct CollectionMemberLookupBinding {
    pair_key: Vec<SurrealUuid>,
}

#[derive(Clone, SurrealValue)]
struct ClassificationMemberInput {
    collection_ref: RecordId,
    asset_ref: RecordId,
    pair_key: Vec<SurrealUuid>,
    source_path_ref: String,
    actor: String,
}

#[derive(Clone, SurrealValue)]
struct ClassificationAuditInput {
    audit_ref: RecordId,
    audit_id: SurrealUuid,
    reason: String,
    source_path_ref: String,
}

#[derive(Clone, SurrealValue)]
struct ClassificationMetadataInput {
    metadata_ref: RecordId,
    asset_ref: Option<RecordId>,
    request_id: String,
    dataset_ref: Option<String>,
    character_ref: Option<String>,
    link_passed: bool,
    tags_json: Vec<String>,
    note: Option<String>,
    event_label: Option<String>,
    event_date: Option<String>,
    location: Option<String>,
    facial_profile: Option<String>,
    loaded_item_count: Option<i64>,
    contact_sheet_json: Option<serde_json::Value>,
    requested_by: String,
}

#[derive(Clone, SurrealValue)]
struct ClassificationItemInput {
    item_ref: RecordId,
    batch_ref: RecordId,
    changed: bool,
    touch_batch: bool,
    lane: String,
    lane_reason: Option<String>,
    member: Option<ClassificationMemberInput>,
    audit: Option<ClassificationAuditInput>,
    metadata: Option<ClassificationMetadataInput>,
}

impl ClassificationItemInput {
    fn writes_anything(&self) -> bool {
        self.changed
            || self.touch_batch
            || self.member.is_some()
            || self.audit.is_some()
            || self.metadata.is_some()
    }
}

#[derive(Clone, SurrealValue)]
struct ClassificationApplyBindings {
    items: Vec<ClassificationItemInput>,
    item_refs: Vec<RecordId>,
    events: Vec<RecordEventBindings>,
}

const GET_ITEM_METADATA_STATEMENT: &str = "SELECT record::id(item_id) AS item_id, \
     record::id(batch_id) AS batch_id, \
     IF asset_id = NONE { NONE } ELSE { record::id(asset_id) } AS asset_id, request_id, \
     dataset_ref, character_ref, link_passed, tags_json, note, event_label, event_date, location, \
     facial_profile, loaded_item_count, contact_sheet_json, requested_by, created_at_utc, \
     updated_at_utc FROM $metadata_ref LIMIT 1;";
const FIND_CONFLICTING_REQUEST_BATCH_STATEMENT: &str =
    "SELECT VALUE record::id(batch_id) FROM atelier_intake_item_metadata \
     WHERE request_id = $request_id AND batch_id != $batch_ref LIMIT 1;";
const RECORDED_REQUEST_PAYLOAD_STATEMENT: &str = "SELECT VALUE payload FROM atelier_event \
     WHERE event_family = $event_family AND aggregate_type = 'atelier_intake_item' \
       AND aggregate_id = $aggregate_id AND payload.metadata.request_id = $request_id \
     ORDER BY created_at_utc ASC LIMIT 1;";
const GET_COLLECTION_TARGET_STATEMENT: &str = "SELECT collection_id, \
     IF character_internal_id = NONE { NONE } ELSE { record::id(character_internal_id) } AS character_internal_id, \
     IF sheet_version_id = NONE { NONE } ELSE { record::id(sheet_version_id) } AS sheet_version_id \
     FROM $collection_ref LIMIT 1;";
const GET_COLLECTION_MEMBER_STATEMENT: &str = "RETURN { \
       LET $rid = type::record('atelier_collection_item', $pair_key); \
       RETURN { exists: record::exists($rid), \
                source_path_ref: (SELECT VALUE source_path_ref FROM $rid)[0] }; };";

/// One atomic statement for a whole classification plan. Every domain write
/// and every event it emits commits together; the kernel ledger + atelier
/// projection rows are written per element of `$events` with the same
/// idempotent shape `atelier_event_sql!` uses for a single event.
const APPLY_CLASSIFICATIONS_STATEMENT: &str = concat!(
    "RETURN { \
       FOR $it IN $items { \
         IF !record::exists($it.item_ref) { THROW 'intake item not found'; }; \
         IF $it.changed { \
           UPDATE $it.item_ref SET lane = $it.lane, lane_reason = $it.lane_reason, \
             updated_at_utc = time::now(); \
         }; \
         IF $it.touch_batch { UPDATE $it.batch_ref SET updated_at_utc = time::now(); }; \
         IF $it.member != NONE { \
           LET $member_rid = type::record('atelier_collection_item', $it.member.pair_key); \
           IF !record::exists($member_rid) { \
             LET $next_order = (array::max((SELECT VALUE sort_order FROM atelier_collection_item \
                                 WHERE collection_id = $it.member.collection_ref)) ?? -1) + 1; \
             CREATE $member_rid CONTENT { collection_id: $it.member.collection_ref, \
               asset_id: $it.member.asset_ref, sort_order: $next_order, \
               source_path_ref: $it.member.source_path_ref, linked_by: $it.member.actor, \
               updated_by: $it.member.actor }; \
             UPDATE $it.member.collection_ref SET updated_at_utc = time::now(); \
           } ELSE { \
             IF (SELECT VALUE source_path_ref FROM $member_rid)[0] != $it.member.source_path_ref { \
               UPDATE $member_rid SET source_path_ref = $it.member.source_path_ref, \
                 updated_by = $it.member.actor, updated_at_utc = time::now(); \
               UPDATE $it.member.collection_ref SET updated_at_utc = time::now(); \
             }; \
           }; \
         }; \
         IF $it.audit != NONE { \
           IF count(SELECT id FROM atelier_intake_item_rejection_audit \
                    WHERE item_id = $it.item_ref AND lane = $it.lane \
                      AND reason = $it.audit.reason) = 0 { \
             CREATE $it.audit.audit_ref CONTENT { audit_id: $it.audit.audit_id, \
               item_id: $it.item_ref, batch_id: $it.batch_ref, lane: $it.lane, \
               reason: $it.audit.reason, source_path_ref: $it.audit.source_path_ref }; \
           }; \
         }; \
         IF $it.metadata != NONE { \
           UPSERT $it.metadata.metadata_ref SET item_id = $it.item_ref, batch_id = $it.batch_ref, \
             asset_id = $it.metadata.asset_ref, request_id = $it.metadata.request_id, \
             dataset_ref = $it.metadata.dataset_ref, character_ref = $it.metadata.character_ref, \
             link_passed = $it.metadata.link_passed, tags_json = $it.metadata.tags_json, \
             note = $it.metadata.note, event_label = $it.metadata.event_label, \
             event_date = $it.metadata.event_date, location = $it.metadata.location, \
             facial_profile = $it.metadata.facial_profile, \
             loaded_item_count = $it.metadata.loaded_item_count, \
             contact_sheet_json = $it.metadata.contact_sheet_json, \
             requested_by = $it.metadata.requested_by, updated_at_utc = time::now(); \
         }; \
       }; \
       FOR $e IN $events { \
         LET $existing = (SELECT VALUE id FROM kernel_event_ledger \
           WHERE idempotency_key = $e.idempotency_key LIMIT 1)[0]; \
         IF $existing IS NONE { \
           CREATE $e.ledger_id CONTENT { \
             event_id: $e.kernel_event_id, event_version: $e.event_version, \
             kernel_task_run_id: $e.kernel_task_run_id, session_run_id: $e.session_run_id, \
             aggregate_type: $e.kernel_aggregate_type, aggregate_id: $e.kernel_aggregate_id, \
             idempotency_key: $e.idempotency_key, event_type: $e.event_type, \
             actor_kind: $e.actor_kind, actor_id: $e.actor_id, causation_id: $e.causation_id, \
             correlation_id: $e.correlation_id, payload_hash: $e.payload_hash, \
             source_component: $e.source_component, payload: $e.ledger_payload, \
             created_at: $e.created_at }; \
         }; \
         LET $ledger_row = (SELECT event_id, event_sequence FROM kernel_event_ledger \
           WHERE idempotency_key = $e.idempotency_key LIMIT 1)[0]; \
         CREATE $e.atelier_id CONTENT { event_id: $e.atelier_event_uuid, \
           event_family: $e.event_family, aggregate_type: $e.kernel_aggregate_type, \
           aggregate_id: $e.kernel_aggregate_id, kernel_event_id: $ledger_row.event_id, \
           kernel_event_sequence: $ledger_row.event_sequence, payload: $e.atelier_payload }; \
       }; \
       RETURN (SELECT ",
    item_columns!(),
    " FROM $item_refs ORDER BY created_at_utc ASC, item_id ASC); };"
);

#[derive(SurrealValue)]
struct LedgerIdempotencyBinding {
    idempotency_key: String,
}

#[derive(SurrealValue)]
struct LedgerReceiptRow {
    event_id: String,
    event_sequence: i64,
}

/// The fully resolved write plan for one intake item.
struct ClassificationPlan {
    item_id: Uuid,
    asset_id: Option<Uuid>,
    collection_id: Option<Uuid>,
    collection_inserted: bool,
    input: ClassificationItemInput,
    events: Vec<PreparedAtelierEvent>,
}

/// Per-plan bookkeeping so two items in one batch that resolve to the same
/// `(collection, asset)` pair predict membership the way the statement will
/// observe it.
#[derive(Default)]
struct BatchPlanState {
    planned_members: BTreeMap<(Uuid, Uuid), String>,
}

impl AtelierStore {
    /// Read the durable dataset-mining metadata row for an intake item.
    pub async fn get_intake_item_metadata(
        &self,
        item_id: Uuid,
    ) -> AtelierResult<Option<IntakeItemMetadata>> {
        let binding = MetadataRefBinding {
            metadata_ref: RecordId::new("atelier_intake_item_metadata", SurrealUuid::from(item_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_ITEM_METADATA_STATEMENT, binding).await })
            })
            .await?;
        row.map(intake_row)
            .transpose()?
            .map(|row| intake_item_metadata_from_row(&row))
            .transpose()
    }

    async fn recorded_request_payload(
        &self,
        item_id: Uuid,
        request_id: Option<&str>,
    ) -> AtelierResult<Option<serde_json::Value>> {
        let Some(request_id) = request_id.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let binding = RecordedRequestBinding {
            event_family: intake_event_family::INTAKE_ITEM_CLASSIFIED.to_owned(),
            aggregate_id: item_id.to_string(),
            request_id: request_id.to_owned(),
        };
        self.with_data(move |ctx| {
            Box::pin(async move {
                ctx.query_first(RECORDED_REQUEST_PAYLOAD_STATEMENT, binding)
                    .await
            })
        })
        .await
    }

    async fn resolve_asset_id_by_content_hash(
        &self,
        content_hash: &str,
    ) -> AtelierResult<Option<Uuid>> {
        let content_hash = content_hash.to_owned();
        self.with_data(move |ctx| {
            Box::pin(async move {
                ctx.query_first(
                    "SELECT VALUE asset_id FROM atelier_media_asset \
                     WHERE content_hash = $content_hash LIMIT 1;",
                    ContentHashBinding { content_hash },
                )
                .await
            })
        })
        .await
    }

    /// Build the write plan for one item. Every validation the PostgreSQL
    /// transaction performed under `FOR UPDATE` happens here, before anything
    /// is written, so a failing item aborts the whole plan with nothing
    /// persisted.
    async fn plan_intake_classification(
        &self,
        existing: &IntakeItem,
        batch: &IntakeBatch,
        lane: IntakeLane,
        reason: Option<&str>,
        requested_by: Option<&str>,
        metadata: Option<&IntakeClassificationMetadata>,
        state: &mut BatchPlanState,
    ) -> AtelierResult<ClassificationPlan> {
        let normalized_reason = normalize_lane_reason(lane, reason)?;
        let (metadata, request_id) =
            normalize_classification_metadata(metadata, Some(existing.batch_id), None, false)?;
        let requested_by = normalize_requested_by(requested_by)?;
        let changed = existing.lane != lane || existing.lane_reason != normalized_reason;
        let has_request_context = requested_by.is_some() || metadata.is_some();
        let recorded_request_payload = self
            .recorded_request_payload(existing.item_id, request_id.as_deref())
            .await?;
        let duplicate_request_id = recorded_request_payload.is_some();
        if let Some(recorded_payload) = recorded_request_payload.as_ref() {
            let recorded_lane = recorded_payload
                .get("lane")
                .and_then(|value| value.as_str());
            let recorded_reason = recorded_payload
                .get("reason")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned);
            if recorded_lane != Some(lane.as_str()) || recorded_reason != normalized_reason {
                return Err(AtelierError::Validation(
                    "duplicate intake classification request_id conflicts with recorded lane/reason"
                        .into(),
                ));
            }
        }
        if duplicate_request_id && changed {
            return Err(AtelierError::Validation(
                "duplicate intake classification request_id is stale against current lane/reason"
                    .into(),
            ));
        }

        let item_ref = RecordId::new("atelier_intake_item", SurrealUuid::from(existing.item_id));
        let batch_ref = RecordId::new("atelier_intake_batch", SurrealUuid::from(existing.batch_id));
        let source_path_ref = event_ref_for_text(&existing.source_path);
        let mut events = Vec::new();
        let mut asset_id = None;
        let mut collection_id = None;
        let mut collection_inserted = false;
        let mut member = None;

        if lane == IntakeLane::Accepted {
            let content_hash = existing.content_hash.as_deref().ok_or_else(|| {
                AtelierError::Validation(
                    "accepted intake item requires target media asset content_hash".into(),
                )
            })?;
            let resolved_asset_id = self
                .resolve_asset_id_by_content_hash(content_hash)
                .await?
                .ok_or_else(|| {
                    AtelierError::NotFound(format!(
                        "target media asset for intake item {}",
                        existing.item_id
                    ))
                })?;
            asset_id = Some(resolved_asset_id);

            if let Some(target_collection_id) = batch.target_collection_id {
                let collection_ref =
                    RecordId::new("atelier_collection", SurrealUuid::from(target_collection_id));
                let collection_row: Option<serde_json::Value> = self
                    .with_data({
                        let collection_ref = collection_ref.clone();
                        move |ctx| {
                            Box::pin(async move {
                                ctx.query_first(
                                    GET_COLLECTION_TARGET_STATEMENT,
                                    CollectionRefBinding { collection_ref },
                                )
                                .await
                            })
                        }
                    })
                    .await?;
                let collection_row = intake_row(collection_row.ok_or_else(|| {
                    AtelierError::NotFound(format!("target collection {target_collection_id}"))
                })?)?;
                let collection_character_id: Option<Uuid> =
                    collection_row.get("character_internal_id");
                let collection_sheet_version_id: Option<Uuid> =
                    collection_row.get("sheet_version_id");
                if let Some(target_character_id) = batch.target_character_id {
                    if collection_character_id != Some(target_character_id) {
                        return Err(AtelierError::Validation(format!(
                            "target collection {target_collection_id} does not belong to intake batch target_character_id {target_character_id}"
                        )));
                    }
                }
                if let Some(target_sheet_version_id) = batch.target_sheet_version_id {
                    if collection_sheet_version_id != Some(target_sheet_version_id) {
                        return Err(AtelierError::Validation(format!(
                            "target collection {target_collection_id} does not belong to intake batch target_sheet_version_id {target_sheet_version_id}"
                        )));
                    }
                }

                let pair_key = vec![
                    SurrealUuid::from(target_collection_id),
                    SurrealUuid::from(resolved_asset_id),
                ];
                let (member_exists, existing_source_path_ref): (bool, Option<String>) =
                    match state
                        .planned_members
                        .get(&(target_collection_id, resolved_asset_id))
                    {
                        Some(planned_ref) => (true, Some(planned_ref.clone())),
                        None => {
                            let lookup: Option<serde_json::Value> = self
                                .with_data({
                                    let pair_key = pair_key.clone();
                                    move |ctx| {
                                        Box::pin(async move {
                                            ctx.query_first(
                                                GET_COLLECTION_MEMBER_STATEMENT,
                                                CollectionMemberLookupBinding { pair_key },
                                            )
                                            .await
                                        })
                                    }
                                })
                                .await?;
                            let lookup = intake_row(lookup.ok_or_else(|| {
                                AtelierError::Internal(
                                    "collection membership lookup returned no row".to_owned(),
                                )
                            })?)?;
                            (lookup.get("exists"), lookup.get("source_path_ref"))
                        }
                    };
                let inserted = !member_exists;
                let updated_refs = member_exists
                    && existing_source_path_ref.as_deref() != Some(source_path_ref.as_str());
                collection_id = Some(target_collection_id);
                collection_inserted = inserted;
                state.planned_members.insert(
                    (target_collection_id, resolved_asset_id),
                    source_path_ref.clone(),
                );
                if inserted || updated_refs {
                    member = Some(ClassificationMemberInput {
                        collection_ref,
                        asset_ref: RecordId::new(
                            "atelier_media_asset",
                            SurrealUuid::from(resolved_asset_id),
                        ),
                        pair_key,
                        source_path_ref: source_path_ref.clone(),
                        actor: requested_by
                            .clone()
                            .unwrap_or_else(|| "system".to_owned()),
                    });
                    events.push(self.prepare_event(
                        collections_event_family::COLLECTION_IMAGES_ADDED,
                        "atelier_collection",
                        &target_collection_id.to_string(),
                        serde_json::json!({
                            "requested": 1,
                            "inserted": if inserted { 1 } else { 0 },
                            "updated_refs": if updated_refs { 1 } else { 0 },
                            "asset_id": resolved_asset_id,
                            "intake_item_id": existing.item_id,
                            "source_path_ref": source_path_ref,
                            "requested_by": requested_by.as_deref(),
                            "request_id": request_id.as_deref(),
                        }),
                    )?);
                }
            }
        }

        let mut audit = None;
        if changed && lane.requires_rejection_audit() {
            let audit_reason = normalized_reason.clone().ok_or_else(|| {
                AtelierError::Validation(format!(
                    "{} intake items require a rejection audit reason",
                    lane.as_str()
                ))
            })?;
            let lookup = AuditLookupBinding {
                item_ref: item_ref.clone(),
                lane: lane.as_str().to_owned(),
                reason: audit_reason.clone(),
            };
            let existing_audit: Option<serde_json::Value> = self
                .with_data(move |ctx| {
                    Box::pin(async move { ctx.query_first(FIND_AUDIT_STATEMENT, lookup).await })
                })
                .await?;
            if existing_audit.is_none() {
                let audit_id = Uuid::now_v7();
                audit = Some(ClassificationAuditInput {
                    audit_ref: RecordId::new(
                        "atelier_intake_item_rejection_audit",
                        SurrealUuid::from(audit_id),
                    ),
                    audit_id: SurrealUuid::from(audit_id),
                    reason: audit_reason.clone(),
                    source_path_ref: source_path_ref.clone(),
                });
                events.push(self.prepare_event(
                    intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
                    "atelier_intake_item",
                    &existing.item_id.to_string(),
                    serde_json::json!({
                        "audit_id": audit_id,
                        "batch_id": existing.batch_id,
                        "lane": lane,
                        "reason_ref": event_ref_for_text(&audit_reason),
                        "source_path_ref": source_path_ref,
                    }),
                )?);
            }
        }

        let emit_classified = (changed || has_request_context) && !duplicate_request_id;
        if emit_classified {
            events.push(self.prepare_event(
                intake_event_family::INTAKE_ITEM_CLASSIFIED,
                "atelier_intake_item",
                &existing.item_id.to_string(),
                serde_json::json!({
                    "batch_id": existing.batch_id,
                    "lane": lane,
                    "reason": normalized_reason,
                    "source_path_ref": source_path_ref,
                    "asset_id": asset_id,
                    "collection_id": collection_id,
                    "apply_workflow": true,
                    "changed": changed,
                    "requested_by": requested_by.as_deref(),
                    "metadata": metadata.as_ref(),
                }),
            )?);
        }

        let mut metadata_input = None;
        if let (Some(requested_by), Some(metadata), Some(request_id)) = (
            requested_by.as_deref(),
            metadata.as_ref(),
            metadata.as_ref().and_then(|value| value.request_id.as_deref()),
        ) {
            let previous = self.get_intake_item_metadata(existing.item_id).await?;
            let mut merged_tags = Vec::new();
            let mut seen_tags = BTreeSet::new();
            if let Some(previous) = previous.as_ref() {
                for tag in &previous.tags {
                    if seen_tags.insert(tag.clone()) {
                        merged_tags.push(tag.clone());
                    }
                }
            }
            for tag in &metadata.tags {
                let normalized = normalize_tag(tag);
                if normalized.is_empty() {
                    continue;
                }
                if seen_tags.insert(normalized.clone()) {
                    merged_tags.push(normalized);
                }
            }
            let coalesce = |incoming: &Option<String>, prior: Option<&Option<String>>| {
                incoming
                    .clone()
                    .or_else(|| prior.and_then(|value| value.clone()))
            };
            let contact_sheet = metadata
                .contact_sheet
                .clone()
                .or_else(|| previous.as_ref().and_then(|row| row.contact_sheet.clone()));
            let contact_sheet_json = contact_sheet
                .map(|value| serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({})));
            metadata_input = Some(ClassificationMetadataInput {
                metadata_ref: RecordId::new(
                    "atelier_intake_item_metadata",
                    SurrealUuid::from(existing.item_id),
                ),
                asset_ref: asset_id
                    .or_else(|| previous.as_ref().and_then(|row| row.asset_id))
                    .map(|id| RecordId::new("atelier_media_asset", SurrealUuid::from(id))),
                request_id: request_id.to_owned(),
                dataset_ref: coalesce(
                    &metadata.dataset_ref,
                    previous.as_ref().map(|row| &row.dataset_ref),
                ),
                character_ref: coalesce(
                    &metadata.character_ref,
                    previous.as_ref().map(|row| &row.character_ref),
                ),
                link_passed: metadata.link_passed,
                tags_json: merged_tags,
                note: previous
                    .as_ref()
                    .and_then(|row| row.note.clone())
                    .or_else(|| metadata.note.clone()),
                event_label: coalesce(
                    &metadata.event,
                    previous.as_ref().map(|row| &row.event_label),
                ),
                event_date: coalesce(&metadata.date, previous.as_ref().map(|row| &row.event_date)),
                location: coalesce(
                    &metadata.location,
                    previous.as_ref().map(|row| &row.location),
                ),
                facial_profile: coalesce(
                    &metadata.facial_profile,
                    previous.as_ref().map(|row| &row.facial_profile),
                ),
                loaded_item_count: metadata
                    .loaded_item_count
                    .or_else(|| previous.as_ref().and_then(|row| row.loaded_item_count)),
                contact_sheet_json,
                requested_by: requested_by.to_owned(),
            });
        }

        Ok(ClassificationPlan {
            item_id: existing.item_id,
            asset_id,
            collection_id,
            collection_inserted,
            input: ClassificationItemInput {
                item_ref,
                batch_ref,
                changed,
                touch_batch: emit_classified,
                lane: lane.as_str().to_owned(),
                lane_reason: normalized_reason,
                member,
                audit,
                metadata: metadata_input,
            },
            events,
        })
    }

    /// Commit a set of plans in one statement, then mirror their events onto
    /// the Flight Recorder exactly as `write_with_event` does.
    async fn execute_classification_plans(
        &self,
        plans: Vec<ClassificationPlan>,
    ) -> AtelierResult<Vec<IntakeClassificationApplyResult>> {
        let mut inputs = Vec::with_capacity(plans.len());
        let mut item_refs = Vec::with_capacity(plans.len());
        let mut prepared_events = Vec::new();
        let mut event_bindings = Vec::new();
        let mut needs_write = false;
        for plan in &plans {
            needs_write |= plan.input.writes_anything() || !plan.events.is_empty();
            inputs.push(plan.input.clone());
            item_refs.push(plan.input.item_ref.clone());
        }
        let mut summaries = Vec::with_capacity(plans.len());
        for plan in plans {
            summaries.push((
                plan.item_id,
                plan.asset_id,
                plan.collection_id,
                plan.collection_inserted,
            ));
            for event in plan.events {
                event_bindings.push(event.bindings.clone());
                prepared_events.push(event);
            }
        }

        let rows: Vec<serde_json::Value> = if needs_write {
            let bindings = ClassificationApplyBindings {
                items: inputs,
                item_refs,
                events: event_bindings,
            };
            self.with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(APPLY_CLASSIFICATIONS_STATEMENT, bindings)
                        .await
                })
            })
            .await?
        } else {
            self.with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        concat!(
                            "SELECT ",
                            item_columns!(),
                            " FROM $item_refs ORDER BY created_at_utc ASC, item_id ASC;"
                        ),
                        ItemRefsBinding { item_refs },
                    )
                    .await
                })
            })
            .await?
        };

        for prepared in prepared_events {
            let key = prepared.bindings.idempotency_key.clone();
            let recorded: Option<LedgerReceiptRow> = self
                .with_data(move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            "SELECT event_id, event_sequence FROM kernel_event_ledger \
                             WHERE idempotency_key = $idempotency_key LIMIT 1;",
                            LedgerIdempotencyBinding {
                                idempotency_key: key,
                            },
                        )
                        .await
                    })
                })
                .await?;
            self.finish_event(
                prepared,
                recorded.map(|row| super::RecordedLedgerRow {
                    event_id: row.event_id,
                    event_sequence: row.event_sequence,
                }),
            )
            .await?;
        }

        let mut items_by_id = BTreeMap::new();
        for row in rows {
            let item = item_from_row(&intake_row(row)?)?;
            items_by_id.insert(item.item_id, item);
        }
        summaries
            .into_iter()
            .map(|(item_id, asset_id, collection_id, collection_inserted)| {
                let item = items_by_id.remove(&item_id).ok_or_else(|| {
                    AtelierError::Internal(format!(
                        "applying intake classification returned no row for item {item_id}"
                    ))
                })?;
                Ok(IntakeClassificationApplyResult {
                    item,
                    asset_id,
                    collection_id,
                    collection_inserted,
                })
            })
            .collect()
    }

    async fn apply_intake_classification_once(
        &self,
        request: &ApplyIntakeClassificationRequest,
    ) -> AtelierResult<IntakeClassificationApplyResult> {
        let existing = self
            .get_intake_item_by_id(request.item_id)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("intake item {}", request.item_id)))?;
        let batch = self
            .get_intake_batch_by_id(existing.batch_id)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("intake batch {}", existing.batch_id)))?;
        let mut state = BatchPlanState::default();
        let plan = self
            .plan_intake_classification(
                &existing,
                &batch,
                request.lane,
                request.reason.as_deref(),
                request.requested_by.as_deref(),
                request.metadata.as_ref(),
                &mut state,
            )
            .await?;
        let mut applied = self.execute_classification_plans(vec![plan]).await?;
        applied.pop().ok_or_else(|| {
            AtelierError::Internal("applying intake classification returned no result".to_owned())
        })
    }

    async fn apply_intake_batch_classifications_once(
        &self,
        request: &ApplyIntakeBatchClassificationsRequest,
        requested_by: &str,
    ) -> AtelierResult<IntakeBatchClassificationApplyResult> {
        let batch = self
            .get_intake_batch_by_id(request.batch_id)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("intake batch {}", request.batch_id)))?;
        let items = self.list_intake_items(request.batch_id, None).await?;
        let canonical_item_count = checked_usize_to_i64("intake batch item count", items.len())?;
        let (metadata, request_id) = normalize_classification_metadata(
            request.metadata.as_ref(),
            Some(request.batch_id),
            Some(canonical_item_count),
            true,
        )?;
        let request_id = request_id.ok_or_else(|| {
            AtelierError::Validation(
                "metadata.request_id is required for batch intake classification".into(),
            )
        })?;
        let conflict_binding = RequestBatchConflictBinding {
            request_id: request_id.clone(),
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(request.batch_id)),
        };
        let conflicting_batch_id: Option<Uuid> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(FIND_CONFLICTING_REQUEST_BATCH_STATEMENT, conflict_binding)
                        .await
                })
            })
            .await?;
        if let Some(conflicting_batch_id) = conflicting_batch_id {
            return Err(AtelierError::Validation(format!(
                "metadata.request_id {request_id} is already bound to intake batch {conflicting_batch_id}"
            )));
        }

        let item_ids: BTreeSet<Uuid> = items.iter().map(|item| item.item_id).collect();
        let mut overrides = BTreeMap::new();
        for override_row in &request.overrides {
            if !item_ids.contains(&override_row.item_id) {
                return Err(AtelierError::Validation(format!(
                    "intake batch override item_id {} does not belong to batch {}",
                    override_row.item_id, request.batch_id
                )));
            }
            if overrides
                .insert(
                    override_row.item_id,
                    (override_row.lane, override_row.reason.clone()),
                )
                .is_some()
            {
                return Err(AtelierError::Validation(format!(
                    "duplicate intake batch override item_id {}",
                    override_row.item_id
                )));
            }
        }

        let mut planned = Vec::with_capacity(items.len());
        for item in &items {
            let (lane, reason) = overrides
                .get(&item.item_id)
                .cloned()
                .unwrap_or((request.default_lane, request.default_reason.clone()));
            let normalized_reason = normalize_lane_reason(lane, reason.as_deref())?;
            planned.push((item, lane, normalized_reason));
        }

        let mut state = BatchPlanState::default();
        let mut plans = Vec::with_capacity(planned.len());
        for (item, lane, reason) in planned {
            plans.push(
                self.plan_intake_classification(
                    item,
                    &batch,
                    lane,
                    reason.as_deref(),
                    Some(requested_by),
                    metadata.as_ref(),
                    &mut state,
                )
                .await?,
            );
        }
        let applied = self.execute_classification_plans(plans).await?;
        Ok(IntakeBatchClassificationApplyResult {
            batch_id: request.batch_id,
            total_item_count: items.len(),
            applied,
            failed: None,
        })
    }

    /// Persist the authoritative Atelier item -> Loom block relation.
    ///
    /// The target block must already be a real Loom authority row carrying a
    /// source document or asset. Repeating the same link is idempotent; trying
    /// to repoint an already-linked item fails closed.
    pub async fn link_intake_item_loom_projection(
        &self,
        item_id: Uuid,
        loom_block_id: &str,
        linked_by: &str,
    ) -> AtelierResult<IntakeItemLoomProjection> {
        let loom_block_id = require_scan_text("loom_block_id", loom_block_id)?;
        let linked_by = require_scan_text("linked_by", linked_by)?;
        reject_legacy_runtime_ref("loom_block_id", loom_block_id)?;
        reject_legacy_runtime_ref("linked_by", linked_by)?;

        let item_ref = RecordId::new("atelier_intake_item", SurrealUuid::from(item_id));
        let item: Option<serde_json::Value> = self
            .with_data({
                let item_ref = item_ref.clone();
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(GET_ITEM_BY_REF_STATEMENT, ItemRefBinding { item_ref })
                            .await
                    })
                }
            })
            .await?;
        if item.is_none() {
            return Err(AtelierError::NotFound(format!("intake item {item_id}")));
        }

        let loom_block_ref = RecordId::new("loom_blocks", loom_block_id.to_owned());
        let block: Option<serde_json::Value> = self
            .with_data({
                let loom_block_ref = loom_block_ref.clone();
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            GET_LOOM_BLOCK_STATEMENT,
                            LoomBlockBinding { loom_block_ref },
                        )
                        .await
                    })
                }
            })
            .await?;
        let block = intake_row(
            block.ok_or_else(|| AtelierError::NotFound(format!("Loom block {loom_block_id}")))?,
        )?;
        if !block.get::<bool, _>("has_source") {
            return Err(AtelierError::Validation(format!(
                "Loom block {loom_block_id} has no source document or asset"
            )));
        }
        let workspace_id: String = block.get("workspace_id");
        // `query_first` crosses the store boundary through `serde_json::Value`, where a Surreal
        // RecordId is represented as its display string and cannot be deserialized back into the
        // SDK's typed `RecordId`. Keep the public UUID projection and rebuild the schema-constrained
        // workspaces relation instead of asking the JSON row mapper to recover the lost type tag.
        let workspace_ref = RecordId::new("workspaces", workspace_id.clone());
        let projection_ref = RecordId::new(
            "atelier_intake_item_loom_projection",
            SurrealUuid::from(item_id),
        );
        let existing: Option<serde_json::Value> = self
            .with_data({
                let projection_ref = projection_ref.clone();
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            GET_LOOM_PROJECTION_STATEMENT,
                            ProjectionRefBinding { projection_ref },
                        )
                        .await
                    })
                }
            })
            .await?;
        if let Some(existing) = existing {
            let row = intake_row(existing)?;
            let existing_block_id: String = row.get("loom_block_id");
            if existing_block_id != loom_block_id {
                return Err(AtelierError::Conflict(format!(
                    "intake item {item_id} is already linked to Loom block {existing_block_id}"
                )));
            }
            return Ok(IntakeItemLoomProjection {
                item_id: row.get("item_id"),
                loom_block_id: existing_block_id,
                workspace_id: row.get("workspace_id"),
                linked_by: row.get("linked_by"),
                linked_at_utc: row.get("linked_at_utc"),
            });
        }
        let occupied: Option<serde_json::Value> = self
            .with_data({
                let loom_block_ref = loom_block_ref.clone();
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            FIND_LOOM_PROJECTION_BY_BLOCK_STATEMENT,
                            LoomBlockBinding { loom_block_ref },
                        )
                        .await
                    })
                }
            })
            .await?;
        if occupied.is_some() {
            return Err(AtelierError::Conflict(format!(
                "Loom block {loom_block_id} already has a canonical Atelier intake item"
            )));
        }
        let bindings = LoomProjectionBindings {
            projection_ref,
            item_ref,
            loom_block_ref,
            workspace_ref,
            linked_by: linked_by.to_owned(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_LOOM_PROJECTION_STATEMENT,
                bindings,
                intake_event_family::INTAKE_ITEM_LOOM_PROJECTION_LINKED,
                "atelier_intake_item",
                &item_id.to_string(),
                serde_json::json!({
                    "loom_block_id": loom_block_id,
                    "workspace_id": workspace_id,
                    "linked_by": linked_by,
                }),
            )
            .await?;
        let row = intake_row(row.ok_or_else(|| {
            AtelierError::Internal("linking an intake Loom projection returned no row".to_owned())
        })?)?;
        Ok(IntakeItemLoomProjection {
            item_id: row.get("item_id"),
            loom_block_id: row.get("loom_block_id"),
            workspace_id: row.get("workspace_id"),
            linked_by: row.get("linked_by"),
            linked_at_utc: row.get("linked_at_utc"),
        })
    }

    /// Scan a configured inbox directory and register direct child image files
    /// as pending intake items. The scan is read-only with respect to
    /// `inbox_root`: it reads directory entries and file bytes for hashing but
    /// never writes, moves, renames, or deletes source files.
    pub async fn scan_inbox_folder_import(
        &self,
        request: &InboxFolderScanRequest,
    ) -> AtelierResult<InboxFolderScanResult> {
        require_scan_text("idempotency_key", &request.idempotency_key)?;
        require_scan_text("source_label", &request.source_label)?;
        require_scan_text("requested_by", &request.requested_by)?;
        reject_legacy_runtime_ref("source_label", &request.source_label)?;
        reject_legacy_runtime_ref("requested_by", &request.requested_by)?;
        if request.max_files == 0 {
            return Err(AtelierError::Validation(
                "max_files must be greater than zero".into(),
            ));
        }
        if request.max_files > MAX_INBOX_FOLDER_SCAN_FILES {
            return Err(AtelierError::Validation(format!(
                "max_files must be <= {MAX_INBOX_FOLDER_SCAN_FILES}"
            )));
        }

        let requested_max_files = checked_usize_to_i64("max_files", request.max_files)?;
        let effective_max_files = request.max_files;
        let effective_max_files_i64 = checked_usize_to_i64("max_files", effective_max_files)?;
        let enumeration = enumerate_inbox_folder(request, effective_max_files)?;
        self.scan_inbox_folder_import_embedded(
            request,
            enumeration,
            requested_max_files,
            effective_max_files_i64,
        )
        .await
    }

    async fn scan_inbox_folder_import_embedded(
        &self,
        request: &InboxFolderScanRequest,
        enumeration: InboxFolderEnumeration,
        requested_max_files: i64,
        effective_max_files: i64,
    ) -> AtelierResult<InboxFolderScanResult> {
        let scan_profile_mode = if request.character_internal_id.is_some() {
            IntakeProfileMode::CharacterLinked
        } else {
            IntakeProfileMode::LooseProfile
        };
        let scan_resume_cursor = format!(
            "cursor://atelier/intake/{}/folder-scan",
            sha256_hex(request.idempotency_key.as_bytes())
        );
        let (batch, batch_inserted) = self
            .open_intake_batch_inner(&NewIntakeBatch {
                idempotency_key: request.idempotency_key.clone(),
                source_label: request.source_label.clone(),
                source_ref: Some(enumeration.root_path_ref.clone()),
                mode: IntakeBatchMode::FolderScan,
                profile_mode: scan_profile_mode,
                character_internal_id: request.character_internal_id,
                target_character_id: request.character_internal_id,
                target_sheet_version_id: None,
                target_collection_id: None,
                resume_cursor: Some(scan_resume_cursor),
            })
            .await?;
        if !batch_inserted {
            if batch.source_ref != enumeration.root_path_ref
                && batch.mode == IntakeBatchMode::FolderScan
                && batch.profile_mode == scan_profile_mode
                && batch.character_internal_id == request.character_internal_id
                && batch.target_character_id == request.character_internal_id
                && batch.target_sheet_version_id.is_none()
                && batch.target_collection_id.is_none()
            {
                return Err(AtelierError::Validation(
                    "inbox_root does not match the previous folder scan for this idempotency_key"
                        .into(),
                ));
            }
            if batch.mode != IntakeBatchMode::FolderScan
                || batch.source_ref != enumeration.root_path_ref
                || batch.profile_mode != scan_profile_mode
                || batch.character_internal_id != request.character_internal_id
                || batch.target_character_id != request.character_internal_id
                || batch.target_sheet_version_id.is_some()
                || batch.target_collection_id.is_some()
            {
                return Err(AtelierError::Validation(
                    "idempotency_key is already bound to a different folder scan intake contract"
                        .into(),
                ));
            }
        }

        let mut imported_count = 0_i64;
        let mut duplicate_skipped_count = 0_i64;
        let mut items = Vec::new();
        for file in enumeration.files {
            let (item, inserted) = self
                .insert_intake_item_inner(
                    batch.batch_id,
                    &NewIntakeItem {
                        source_path: file.source_path,
                        file_name: file.file_name,
                        byte_len: file.byte_len,
                        content_hash: Some(file.content_hash),
                    },
                )
                .await?;
            if inserted {
                imported_count += 1;
            } else {
                duplicate_skipped_count += 1;
            }
            items.push(item);
        }

        let skipped_over_max_count = enumeration
            .image_candidate_count
            .saturating_sub(effective_max_files);
        let result = InboxFolderScanResult {
            batch,
            items,
            requested_max_files,
            effective_max_files,
            image_candidate_count: enumeration.image_candidate_count,
            imported_count,
            duplicate_skipped_count,
            skipped_over_max_count,
            skipped_non_image_count: enumeration.skipped_non_image_count,
            skipped_subdir_count: enumeration.skipped_subdir_count,
            skipped_special_count: enumeration.skipped_special_count,
        };

        self.record_event(
            intake_event_family::INTAKE_FOLDER_SCAN_COMPLETED,
            "atelier_intake_batch",
            &result.batch.batch_id.to_string(),
            serde_json::json!({
                "batch_id": result.batch.batch_id,
                "idempotency_key": &request.idempotency_key,
                "source_label": &request.source_label,
                "root_path_ref": enumeration.root_path_ref,
                "requested_by": &request.requested_by,
                "requested_max_files": result.requested_max_files,
                "effective_max_files": result.effective_max_files,
                "image_candidate_count": result.image_candidate_count,
                "imported_count": result.imported_count,
                "duplicate_skipped_count": result.duplicate_skipped_count,
                "skipped_over_max_count": result.skipped_over_max_count,
                "skipped_non_image_count": result.skipped_non_image_count,
                "skipped_subdir_count": result.skipped_subdir_count,
                "skipped_special_count": result.skipped_special_count,
            }),
        )
        .await?;
        Ok(result)
    }

    async fn open_intake_batch_inner(
        &self,
        new: &NewIntakeBatch,
    ) -> AtelierResult<(IntakeBatch, bool)> {
        let (explicit_source_ref, source_ref) = normalize_batch_source_refs(new)?;
        let resume_cursor = normalize_optional_batch_ref("resume_cursor", &new.resume_cursor)?;
        let targets = normalize_batch_targets(new)?;
        if let Some(batch) = self.get_intake_batch_by_key(&new.idempotency_key).await? {
            validate_intake_batch_reopen_contract(
                &batch,
                new,
                explicit_source_ref.as_deref(),
                targets,
            )?;
            return Ok((batch, false));
        }
        let batch_id = Uuid::now_v7();
        let record_ref = |table: &'static str, value: Option<Uuid>| {
            value.map(|id| RecordId::new(table, SurrealUuid::from(id)))
        };
        let bindings = BatchWriteBindings {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
            batch_id: SurrealUuid::from(batch_id),
            idempotency_key: new.idempotency_key.clone(),
            source_label: new.source_label.clone(),
            source_ref: source_ref.clone(),
            mode: new.mode.as_str().to_owned(),
            profile_mode: new.profile_mode.as_str().to_owned(),
            character_internal_id: record_ref("atelier_character", targets.character_internal_id),
            target_character_id: record_ref("atelier_character", targets.target_character_id),
            target_sheet_version_id: record_ref(
                "atelier_sheet_version",
                targets.target_sheet_version_id,
            ),
            target_collection_id: record_ref("atelier_collection", targets.target_collection_id),
            resume_cursor: resume_cursor.clone(),
        };
        let payload = serde_json::json!({
            "batch_id": batch_id,
            "idempotency_key": new.idempotency_key,
            "source_label": new.source_label,
            "source_ref": source_ref,
            "mode": new.mode,
            "profile_mode": new.profile_mode,
            "resume_cursor": resume_cursor,
            "character_scoped": targets.character_internal_id.is_some(),
        });
        let aggregate_id = batch_id.to_string();
        let mut attempt = 1;
        let row = loop {
            let row_result: AtelierResult<Option<serde_json::Value>> = self
                .write_with_event(
                    WRITE_BATCH_STATEMENT,
                    bindings.clone(),
                    intake_event_family::INTAKE_BATCH_CREATED,
                    "atelier_intake_batch",
                    &aggregate_id,
                    payload.clone(),
                )
                .await;
            match row_result {
                Ok(Some(row)) => break row,
                Ok(None) => {
                    return Err(AtelierError::Internal(
                        "opening an intake batch returned no row".to_owned(),
                    ));
                }
                Err(error)
                    if is_surreal_unique_index_conflict(&error, "uq_atelier_intake_batch_1")
                        || is_surreal_retryable_transaction_conflict(&error) =>
                {
                    match self.get_intake_batch_by_key(&new.idempotency_key).await {
                        Ok(Some(batch)) => {
                            validate_intake_batch_reopen_contract(
                                &batch,
                                new,
                                explicit_source_ref.as_deref(),
                                targets,
                            )?;
                            return Ok((batch, false));
                        }
                        Ok(None) => {}
                        Err(read_error)
                            if is_surreal_retryable_transaction_conflict(&read_error) => {}
                        Err(read_error) => {
                            return Err(AtelierError::Internal(format!(
                                "opening an intake batch failed: {error}; canonical idempotency reread also failed: {read_error}"
                            )));
                        }
                    }
                    if attempt >= SURREAL_TRANSACTION_MAX_ATTEMPTS {
                        return Err(error);
                    }
                    wait_before_surreal_transaction_retry(batch_id, attempt).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        };
        let row = intake_row(row)?;
        Ok((batch_from_row(&row), true))
    }

    async fn insert_intake_item_inner(
        &self,
        batch_id: Uuid,
        new: &NewIntakeItem,
    ) -> AtelierResult<(IntakeItem, bool)> {
        if let Some(existing) = self.get_intake_item(batch_id, &new.source_path).await? {
            return Ok((existing, false));
        }
        let item_id = Uuid::now_v7();
        let bindings = ItemWriteBindings {
            item_ref: RecordId::new("atelier_intake_item", SurrealUuid::from(item_id)),
            item_id: SurrealUuid::from(item_id),
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
            source_path: new.source_path.clone(),
            file_name: new.file_name.clone(),
            byte_len: new.byte_len,
            content_hash: new.content_hash.clone(),
        };
        let payload = serde_json::json!({
            "batch_id": batch_id,
            "source_path_ref": event_ref_for_text(&new.source_path),
            "file_name_ref": event_ref_for_text(&new.file_name),
            "byte_len": new.byte_len,
        });
        let aggregate_id = item_id.to_string();
        let mut attempt = 1;
        let row = loop {
            let row_result: AtelierResult<Option<serde_json::Value>> = self
                .write_with_event(
                    WRITE_ITEM_STATEMENT,
                    bindings.clone(),
                    intake_event_family::INTAKE_ITEM_ADDED,
                    "atelier_intake_item",
                    &aggregate_id,
                    payload.clone(),
                )
                .await;
            match row_result {
                Ok(Some(row)) => break row,
                Ok(None) => {
                    return Err(AtelierError::Internal(
                        "adding an intake item returned no row".to_owned(),
                    ));
                }
                Err(error)
                    if is_surreal_unique_index_conflict(&error, "uq_atelier_intake_item_1")
                        || is_surreal_retryable_transaction_conflict(&error) =>
                {
                    match self.get_intake_item(batch_id, &new.source_path).await {
                        Ok(Some(existing)) => return Ok((existing, false)),
                        Ok(None) => {}
                        Err(read_error)
                            if is_surreal_retryable_transaction_conflict(&read_error) => {}
                        Err(read_error) => {
                            return Err(AtelierError::Internal(format!(
                                "adding an intake item failed: {error}; canonical idempotency reread also failed: {read_error}"
                            )));
                        }
                    }
                    if attempt >= SURREAL_TRANSACTION_MAX_ATTEMPTS {
                        return Err(error);
                    }
                    wait_before_surreal_transaction_retry(item_id, attempt).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        };
        let row = intake_row(row)?;
        Ok((item_from_row(&row)?, true))
    }

    async fn insert_rejection_audit(
        &self,
        item: &IntakeItem,
    ) -> AtelierResult<Option<(IntakeItemRejectionAudit, bool)>> {
        if !item.lane.requires_rejection_audit() {
            return Ok(None);
        }
        let reason = item.lane_reason.as_deref().ok_or_else(|| {
            AtelierError::Validation(format!(
                "{} intake items require a rejection audit reason",
                item.lane.as_str()
            ))
        })?;
        let source_path_ref = event_ref_for_text(&item.source_path);
        let item_ref = RecordId::new("atelier_intake_item", SurrealUuid::from(item.item_id));
        let lookup = AuditLookupBinding {
            item_ref: item_ref.clone(),
            lane: item.lane.as_str().to_owned(),
            reason: reason.to_owned(),
        };
        let existing: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(FIND_AUDIT_STATEMENT, lookup).await })
            })
            .await?;
        if let Some(existing) = existing {
            return Ok(Some((
                rejection_audit_from_row(&intake_row(existing)?)?,
                false,
            )));
        }
        let audit_id = Uuid::now_v7();
        let bindings = RejectionAuditBindings {
            audit_ref: RecordId::new(
                "atelier_intake_item_rejection_audit",
                SurrealUuid::from(audit_id),
            ),
            audit_id: SurrealUuid::from(audit_id),
            item_ref,
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(item.batch_id)),
            lane: item.lane.as_str().to_owned(),
            reason: reason.to_owned(),
            source_path_ref: source_path_ref.clone(),
        };
        let payload = serde_json::json!({
            "audit_id": audit_id,
            "batch_id": item.batch_id,
            "lane": item.lane,
            "reason_ref": event_ref_for_text(reason),
            "source_path_ref": source_path_ref,
        });
        let aggregate_id = item.item_id.to_string();
        let mut attempt = 1;
        let row = loop {
            let row_result: AtelierResult<Option<serde_json::Value>> = self
                .write_with_event(
                    WRITE_AUDIT_STATEMENT,
                    bindings.clone(),
                    intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
                    "atelier_intake_item",
                    &aggregate_id,
                    payload.clone(),
                )
                .await;
            match row_result {
                Ok(Some(row)) => break row,
                Ok(None) => {
                    return Err(AtelierError::Internal(
                        "recording an intake rejection audit returned no row".to_owned(),
                    ));
                }
                Err(error)
                    if is_surreal_unique_index_conflict(
                        &error,
                        "uq_atelier_intake_item_rejection_audit_1",
                    ) || is_surreal_retryable_transaction_conflict(&error) =>
                {
                    let lookup = AuditLookupBinding {
                        item_ref: RecordId::new(
                            "atelier_intake_item",
                            SurrealUuid::from(item.item_id),
                        ),
                        lane: item.lane.as_str().to_owned(),
                        reason: reason.to_owned(),
                    };
                    let existing: AtelierResult<Option<serde_json::Value>> = self
                        .with_data(move |ctx| {
                            Box::pin(
                                async move { ctx.query_first(FIND_AUDIT_STATEMENT, lookup).await },
                            )
                        })
                        .await;
                    match existing {
                        Ok(Some(existing)) => {
                            return Ok(Some((
                                rejection_audit_from_row(&intake_row(existing)?)?,
                                false,
                            )));
                        }
                        Ok(None) => {}
                        Err(read_error)
                            if is_surreal_retryable_transaction_conflict(&read_error) => {}
                        Err(read_error) => {
                            return Err(AtelierError::Internal(format!(
                                "recording an intake rejection audit failed: {error}; canonical idempotency reread also failed: {read_error}"
                            )));
                        }
                    }
                    if attempt >= SURREAL_TRANSACTION_MAX_ATTEMPTS {
                        return Err(error);
                    }
                    wait_before_surreal_transaction_retry(audit_id, attempt).await;
                    attempt += 1;
                }
                Err(error) => return Err(error),
            }
        };
        let row = intake_row(row)?;
        Ok(Some((rejection_audit_from_row(&row)?, true)))
    }

    #[cfg(any(test, feature = "surreal-test-support"))]
    pub(crate) async fn mt136_insert_rejection_audit_for_proof(
        &self,
        item: &IntakeItem,
    ) -> AtelierResult<Option<(IntakeItemRejectionAudit, bool)>> {
        self.insert_rejection_audit(item).await
    }

    /// Open a persistent intake batch, or return the existing one for the same
    /// `idempotency_key`. Re-scanning the same source is therefore safe and
    /// never creates a duplicate batch (legacy source `createIntakeBatch` intent).
    pub async fn open_intake_batch(&self, new: &NewIntakeBatch) -> AtelierResult<IntakeBatch> {
        if new.idempotency_key.trim().is_empty()
            || new.idempotency_key.trim() != new.idempotency_key
        {
            return Err(AtelierError::Validation(
                "idempotency_key must not be empty or padded".into(),
            ));
        }
        require_scan_text("source_label", &new.source_label)?;
        reject_legacy_runtime_ref("source_label", &new.source_label)?;
        self.open_intake_batch_inner(new)
            .await
            .map(|(batch, _)| batch)
    }

    /// Fetch a batch by its stable idempotency key.
    pub async fn get_intake_batch_by_key(
        &self,
        idempotency_key: &str,
    ) -> AtelierResult<Option<IntakeBatch>> {
        let binding = IdempotencyKeyBinding {
            idempotency_key: idempotency_key.to_owned(),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_BATCH_BY_KEY_STATEMENT, binding).await })
            })
            .await?;
        row.map(intake_row)
            .transpose()
            .map(|row| row.as_ref().map(batch_from_row))
    }

    async fn get_intake_batch_by_id(&self, batch_id: Uuid) -> AtelierResult<Option<IntakeBatch>> {
        let binding = BatchRefBinding {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_BATCH_STATEMENT, binding).await })
            })
            .await?;
        row.map(intake_row)
            .transpose()
            .map(|row| row.as_ref().map(batch_from_row))
    }

    /// List batches, newest first, optionally filtered by status.
    pub async fn list_intake_batches(
        &self,
        status: Option<BatchStatus>,
        limit: i64,
    ) -> AtelierResult<Vec<IntakeBatch>> {
        let capped = limit.clamp(1, 1000);
        let rows: Vec<serde_json::Value> = self
            .with_data(|ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_BATCHES_STATEMENT, NoBindings {})
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(intake_row)
            .map(|row| row.map(|row| batch_from_row(&row)))
            .filter(|result| {
                result
                    .as_ref()
                    .map(|batch| status.map(|wanted| batch.status == wanted).unwrap_or(true))
                    .unwrap_or(true)
            })
            .take(capped as usize)
            .collect()
    }

    /// List batches by profile linkage mode, newest first. `None` returns all
    /// profile modes while preserving the same cap behavior as status listing.
    pub async fn list_intake_batches_by_profile_mode(
        &self,
        profile_mode: Option<IntakeProfileMode>,
        limit: i64,
    ) -> AtelierResult<Vec<IntakeBatch>> {
        let capped = limit.clamp(1, 1000);
        let rows: Vec<serde_json::Value> = self
            .with_data(|ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_BATCHES_STATEMENT, NoBindings {})
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(intake_row)
            .map(|row| row.map(|row| batch_from_row(&row)))
            .filter(|result| {
                result
                    .as_ref()
                    .map(|batch| {
                        profile_mode
                            .map(|wanted| batch.profile_mode == wanted)
                            .unwrap_or(true)
                    })
                    .unwrap_or(true)
            })
            .take(capped as usize)
            .collect()
    }

    /// Mark a persistent intake batch as actively resumed and store the cursor
    /// another route/session can use to continue without hidden UI state.
    pub async fn mark_intake_batch_in_progress(
        &self,
        batch_id: Uuid,
        resume_cursor: &str,
        requested_by: &str,
    ) -> AtelierResult<IntakeBatch> {
        let cursor =
            normalize_optional_batch_ref("resume_cursor", &Some(resume_cursor.to_string()))?
                .expect("resume_cursor validation returns Some for Some input");
        let requested_by = require_scan_text("requested_by", requested_by)?;
        reject_legacy_runtime_ref("requested_by", requested_by)?;

        let existing = self.get_intake_batch_by_id(batch_id).await?;
        if existing
            .as_ref()
            .map(|batch| batch.status == BatchStatus::Closed)
            .unwrap_or(true)
        {
            return Err(AtelierError::NotFound(format!(
                "open intake batch {batch_id}"
            )));
        }
        let bindings = BatchResumeBindings {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
            resume_cursor: cursor.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                RESUME_BATCH_STATEMENT,
                bindings,
                intake_event_family::INTAKE_BATCH_RESUMED,
                "atelier_intake_batch",
                &batch_id.to_string(),
                serde_json::json!({
                    "batch_id": batch_id,
                    "source_ref": existing.as_ref().map(|batch| &batch.source_ref),
                    "mode": existing.as_ref().map(|batch| batch.mode),
                    "status": BatchStatus::InProgress,
                    "resume_cursor": cursor,
                    "requested_by": requested_by,
                }),
            )
            .await?;
        let row = intake_row(row.ok_or_else(|| {
            AtelierError::Internal("resuming an intake batch returned no row".to_owned())
        })?)?;
        Ok(batch_from_row(&row))
    }

    /// Register a source file in a batch, idempotently. Re-adding the same
    /// `(batch, source_path)` returns the existing item rather than creating a
    /// duplicate, and never mutates its lane (source preservation). Items always
    /// enter the `Pending` lane.
    pub async fn add_intake_item(
        &self,
        batch_id: Uuid,
        new: &NewIntakeItem,
    ) -> AtelierResult<IntakeItem> {
        if self.get_intake_batch_by_id(batch_id).await?.is_none() {
            return Err(AtelierError::NotFound(format!(
                "intake batch {batch_id}"
            )));
        }
        if new.source_path.trim().is_empty() {
            return Err(AtelierError::Validation(
                "source_path must not be empty".into(),
            ));
        }
        if new.file_name.trim().is_empty() {
            return Err(AtelierError::Validation(
                "file_name must not be empty".into(),
            ));
        }
        if new.byte_len < 0 {
            return Err(AtelierError::Validation(
                "byte_len must not be negative".into(),
            ));
        }
        reject_legacy_runtime_ref("source_path", &new.source_path)?;
        self.insert_intake_item_inner(batch_id, new)
            .await
            .map(|(item, _)| item)
    }

    /// Fetch a single item by its preserved source path within a batch.
    pub async fn get_intake_item(
        &self,
        batch_id: Uuid,
        source_path: &str,
    ) -> AtelierResult<Option<IntakeItem>> {
        let binding = ItemLookupBinding {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
            source_path: source_path.to_owned(),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_ITEM_STATEMENT, binding).await })
            })
            .await?;
        row.map(intake_row)
            .transpose()?
            .map(|row| item_from_row(&row))
            .transpose()
    }

    /// Fetch a single item by its id.
    pub async fn get_intake_item_by_id(
        &self,
        item_id: Uuid,
    ) -> AtelierResult<Option<IntakeItem>> {
        let binding = ItemRefBinding {
            item_ref: RecordId::new("atelier_intake_item", SurrealUuid::from(item_id)),
        };
        let row: Option<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_ITEM_BY_REF_STATEMENT, binding).await })
            })
            .await?;
        row.map(intake_row)
            .transpose()?
            .map(|row| item_from_row(&row))
            .transpose()
    }

    /// List the items in a batch (creation order), optionally filtered to a lane.
    pub async fn list_intake_items(
        &self,
        batch_id: Uuid,
        lane: Option<IntakeLane>,
    ) -> AtelierResult<Vec<IntakeItem>> {
        let binding = BatchRefBinding {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_ITEMS_STATEMENT, binding).await })
            })
            .await?;
        rows.into_iter()
            .map(intake_row)
            .map(|row| row.and_then(|row| item_from_row(&row)))
            .filter(|result| {
                result
                    .as_ref()
                    .map(|item| lane.map(|wanted| item.lane == wanted).unwrap_or(true))
                    .unwrap_or(true)
            })
            .collect()
    }

    /// List at most `limit` items without materializing the rest of the batch.
    /// The lane predicate and cap both execute inside SurrealDB.
    pub async fn list_intake_items_limited(
        &self,
        batch_id: Uuid,
        lane: Option<IntakeLane>,
        limit: i64,
    ) -> AtelierResult<Vec<IntakeItem>> {
        if limit <= 0 {
            return Ok(Vec::new());
        }
        let binding = BatchListBinding {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
            lane: lane.map(|value| value.as_str().to_owned()),
            limit,
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_ITEMS_LIMITED_STATEMENT, binding)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(intake_row)
            .map(|row| row.and_then(|row| item_from_row(&row)))
            .collect()
    }

    /// List durable negative-lifecycle audit rows for a batch.
    pub async fn list_intake_rejection_audits(
        &self,
        batch_id: Uuid,
    ) -> AtelierResult<Vec<IntakeItemRejectionAudit>> {
        let binding = BatchRefBinding {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_AUDITS_STATEMENT, binding).await })
            })
            .await?;
        rows.into_iter()
            .map(intake_row)
            .map(|row| row.and_then(|row| rejection_audit_from_row(&row)))
            .collect()
    }

    pub async fn record_atelier_reset(
        &self,
        request: &AtelierResetRequest,
    ) -> AtelierResult<AtelierResetRecord> {
        let requested_by = require_scan_text("requested_by", &request.requested_by)?;
        let reason = require_scan_text("reason", &request.reason)?;
        reject_legacy_runtime_ref("requested_by", requested_by)?;
        reject_legacy_runtime_ref("reason", reason)?;

        let reset_id = Uuid::now_v7();
        let preferences_deleted_count: Option<i64> = self
            .with_data(|ctx| {
                Box::pin(async move {
                    ctx.query_first(COUNT_PREFERENCES_STATEMENT, NoBindings {})
                        .await
                })
            })
            .await?;
        let preferences_deleted_count = preferences_deleted_count.unwrap_or(0);
        let preserve_original_media = request.mode == AtelierResetMode::FullPreserveOriginalMedia;
        let media_rows: Vec<serde_json::Value> = if preserve_original_media {
            self.with_data(|ctx| {
                Box::pin(async move {
                    ctx.query_values(
                        LIST_ORIGINAL_MEDIA_STATEMENT,
                        RetentionClassBinding {
                            retention_class: MEDIA_ORIGINAL_RETENTION_CLASS.to_owned(),
                        },
                    )
                    .await
                })
            })
            .await?
        } else {
            Vec::new()
        };
        let original_media_preserved_count =
            checked_usize_to_i64("original_media_preserved_count", media_rows.len())?;
        let manifest_id = Uuid::now_v7();
        let mut media_items = Vec::with_capacity(media_rows.len());
        for row in media_rows {
            let row = intake_row(row)?;
            let manifest_item_id = Uuid::now_v7();
            media_items.push(ResetMediaInput {
                manifest_item_ref: RecordId::new(
                    "atelier_orphan_manifest_item",
                    SurrealUuid::from(manifest_item_id),
                ),
                manifest_item_id: SurrealUuid::from(manifest_item_id),
                asset_ref: row.get("asset_ref"),
                content_hash: row.get("content_hash"),
                artifact_ref: row.get("artifact_ref"),
                mime: row.get("mime"),
                byte_len: row.get("byte_len"),
                retention_class: row.get("retention_class"),
            });
        }
        let orphan_manifest_id = preserve_original_media.then_some(manifest_id);
        let manifest_json = serde_json::json!({
            "schema_id": ORPHAN_MANIFEST_SCHEMA_ID,
            "reset_id": reset_id,
            "reset_mode": request.mode.as_str(),
            "item_count": original_media_preserved_count,
            "retention_class": MEDIA_ORIGINAL_RETENTION_CLASS,
        });
        let bindings = ResetBindings {
            reset_ref: RecordId::new("atelier_reset_operation", SurrealUuid::from(reset_id)),
            reset_id: SurrealUuid::from(reset_id),
            mode: request.mode.as_str().to_owned(),
            requested_by: requested_by.to_owned(),
            reason: reason.to_owned(),
            preserve_original_media,
            manifest_ref: RecordId::new("atelier_orphan_manifest", SurrealUuid::from(manifest_id)),
            manifest_id: SurrealUuid::from(manifest_id),
            manifest_json,
            preferences_deleted_count,
            original_media_preserved_count,
            media_items,
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                WRITE_RESET_STATEMENT,
                bindings,
                intake_event_family::RESET_RECORDED,
                "atelier_reset_operation",
                &reset_id.to_string(),
                serde_json::json!({
                    "mode": request.mode.as_str(),
                    "requested_by": requested_by,
                    "reason_ref": event_ref_for_text(reason),
                    "preferences_deleted_count": preferences_deleted_count,
                    "original_media_preserved_count": original_media_preserved_count,
                    "orphan_manifest_id": orphan_manifest_id,
                }),
            )
            .await?;
        let row = intake_row(row.ok_or_else(|| {
            AtelierError::Internal("recording an Atelier reset returned no row".to_owned())
        })?)?;
        let reset = reset_from_row(&row)?;
        if preserve_original_media {
            self.record_event(
                intake_event_family::ORPHAN_MANIFEST_RECORDED,
                "atelier_orphan_manifest",
                &manifest_id.to_string(),
                serde_json::json!({
                    "reset_id": reset_id,
                    "schema_id": ORPHAN_MANIFEST_SCHEMA_ID,
                    "item_count": original_media_preserved_count,
                    "retention_class": MEDIA_ORIGINAL_RETENTION_CLASS,
                }),
            )
            .await?;
        }
        Ok(reset)
    }

    pub async fn list_orphan_manifest_items(
        &self,
        manifest_id: Uuid,
    ) -> AtelierResult<Vec<OrphanManifestItem>> {
        let binding = ManifestRefBinding {
            manifest_ref: RecordId::new("atelier_orphan_manifest", SurrealUuid::from(manifest_id)),
        };
        let rows: Vec<serde_json::Value> = self
            .with_data(move |ctx| {
                Box::pin(
                    async move { ctx.query_values(LIST_ORPHAN_ITEMS_STATEMENT, binding).await },
                )
            })
            .await?;
        rows.into_iter()
            .map(intake_row)
            .map(|row| row.and_then(|row| orphan_manifest_item_from_row(&row)))
            .collect()
    }

    pub async fn adopt_orphan_manifest_item(
        &self,
        request: &OrphanAdoptionRequest,
    ) -> AtelierResult<OrphanAdoptionResult> {
        let requested_by = require_scan_text("requested_by", &request.requested_by)?;
        reject_legacy_runtime_ref("requested_by", requested_by)?;
        let manifest_item_ref = RecordId::new(
            "atelier_orphan_manifest_item",
            SurrealUuid::from(request.manifest_item_id),
        );
        let row: Option<serde_json::Value> = self
            .with_data({
                let manifest_item_ref = manifest_item_ref.clone();
                move |ctx| {
                    Box::pin(async move {
                        ctx.query_first(
                            GET_ORPHAN_ITEM_STATEMENT,
                            OrphanItemRefBinding { manifest_item_ref },
                        )
                        .await
                    })
                }
            })
            .await?;
        let manifest_item = orphan_manifest_item_from_row(&intake_row(row.ok_or_else(|| {
            AtelierError::NotFound(format!("orphan manifest item {}", request.manifest_item_id))
        })?)?)?;
        if manifest_item.adoption_status == OrphanAdoptionStatus::Adopted {
            let batch_id = manifest_item.adopted_batch_id.ok_or_else(|| {
                AtelierError::Validation(
                    "adopted orphan manifest item is missing adopted_batch_id".into(),
                )
            })?;
            let item_id = manifest_item.adopted_item_id.ok_or_else(|| {
                AtelierError::Validation(
                    "adopted orphan manifest item is missing adopted_item_id".into(),
                )
            })?;
            let batch = self
                .get_intake_batch_by_id(batch_id)
                .await?
                .ok_or_else(|| AtelierError::NotFound(format!("intake batch {batch_id}")))?;
            let item = self
                .get_intake_item_by_id(item_id)
                .await?
                .ok_or_else(|| AtelierError::NotFound(format!("intake item {item_id}")))?;
            return Ok(OrphanAdoptionResult {
                manifest_item,
                batch,
                item,
            });
        }

        reject_legacy_runtime_ref("artifact_ref", &manifest_item.artifact_ref)?;
        let batch_request = NewIntakeBatch {
            idempotency_key: format!("orphan-adoption:{}", manifest_item.manifest_id),
            source_label: format!("orphan-adoption:{}", manifest_item.manifest_id),
            source_ref: Some(format!("orphan-manifest://{}", manifest_item.manifest_id)),
            mode: IntakeBatchMode::Manual,
            profile_mode: IntakeProfileMode::LooseProfile,
            character_internal_id: None,
            target_character_id: None,
            target_sheet_version_id: None,
            target_collection_id: None,
            resume_cursor: None,
        };
        let (batch, _) = self.open_intake_batch_inner(&batch_request).await?;
        let item_request = NewIntakeItem {
            source_path: manifest_item.artifact_ref.clone(),
            file_name: orphan_intake_file_name(&manifest_item.content_hash, &manifest_item.mime),
            byte_len: manifest_item.byte_len,
            content_hash: Some(manifest_item.content_hash.clone()),
        };
        let (item, _) = self
            .insert_intake_item_inner(batch.batch_id, &item_request)
            .await?;
        let bindings = OrphanAdoptionBindings {
            manifest_item_ref,
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch.batch_id)),
            item_ref: RecordId::new("atelier_intake_item", SurrealUuid::from(item.item_id)),
            adopted_by: requested_by.to_owned(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                ADOPT_ORPHAN_ITEM_STATEMENT,
                bindings,
                intake_event_family::ORPHAN_MANIFEST_ITEM_ADOPTED,
                "atelier_orphan_manifest_item",
                &manifest_item.manifest_item_id.to_string(),
                serde_json::json!({
                    "manifest_id": manifest_item.manifest_id,
                    "asset_id": manifest_item.asset_id,
                    "content_hash": manifest_item.content_hash,
                    "adopted_batch_id": batch.batch_id,
                    "adopted_item_id": item.item_id,
                    "requested_by": requested_by,
                }),
            )
            .await?;
        let updated_manifest_item =
            orphan_manifest_item_from_row(&intake_row(row.ok_or_else(|| {
                AtelierError::Internal("adopting an orphan item returned no row".to_owned())
            })?)?)?;
        Ok(OrphanAdoptionResult {
            manifest_item: updated_manifest_item,
            batch,
            item,
        })
    }

    /// Move an item into a lifecycle lane. This is the only state change
    /// triage performs: the source row is preserved and never deleted, only
    /// its lane and reason change. Rejected/skipped/failed states also write
    /// an idempotent audit row.
    pub async fn classify_intake_item(
        &self,
        item_id: Uuid,
        lane: IntakeLane,
        reason: Option<&str>,
    ) -> AtelierResult<IntakeItem> {
        let normalized_reason = normalize_lane_reason(lane, reason)?;
        let existing = self
            .get_intake_item_by_id(item_id)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("intake item {item_id}")))?;
        if existing.lane == lane && existing.lane_reason == normalized_reason {
            return Ok(existing);
        }
        let bindings = ItemClassificationBindings {
            item_ref: RecordId::new("atelier_intake_item", SurrealUuid::from(item_id)),
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(existing.batch_id)),
            lane: lane.as_str().to_owned(),
            lane_reason: normalized_reason.clone(),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                CLASSIFY_ITEM_STATEMENT,
                bindings,
                intake_event_family::INTAKE_ITEM_CLASSIFIED,
                "atelier_intake_item",
                &item_id.to_string(),
                serde_json::json!({
                    "batch_id": existing.batch_id,
                    "lane": lane,
                    "reason": normalized_reason,
                    "source_path_ref": event_ref_for_text(&existing.source_path),
                }),
            )
            .await?;
        let item = item_from_row(&intake_row(row.ok_or_else(|| {
            AtelierError::Internal("classifying an intake item returned no row".to_owned())
        })?)?)?;
        self.insert_rejection_audit(&item).await?;
        Ok(item)
    }

    /// Apply an intake lane decision to the media workflow. Accepted decisions
    /// resolve the item's `content_hash` to an existing media asset and attach
    /// that asset to the batch target collection when configured. All writes
    /// happen in one transaction so invalid targets roll back the lane change.
    pub async fn apply_intake_classification(
        &self,
        request: &ApplyIntakeClassificationRequest,
    ) -> AtelierResult<IntakeClassificationApplyResult> {
        let normalized_reason = normalize_lane_reason(request.lane, request.reason.as_deref())?;
        let existing = self
            .get_intake_item_by_id(request.item_id)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("intake item {}", request.item_id)))?;
        let batch = self
            .get_intake_batch_by_id(existing.batch_id)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("intake batch {}", existing.batch_id)))?;

        let mut asset_id = None;
        let mut collection_id = None;
        let mut collection_inserted = false;
        if request.lane == IntakeLane::Accepted {
            let content_hash = existing.content_hash.as_deref().ok_or_else(|| {
                AtelierError::Validation(
                    "accepted intake item requires target media asset content_hash".into(),
                )
            })?;
            let resolved_asset_id: Option<Uuid> = self
                .with_data({
                    let content_hash = content_hash.to_owned();
                    move |ctx| {
                        Box::pin(async move {
                            ctx.query_first(
                                "SELECT VALUE asset_id FROM atelier_media_asset \
                                 WHERE content_hash = $content_hash LIMIT 1;",
                                ContentHashBinding { content_hash },
                            )
                            .await
                        })
                    }
                })
                .await?;
            let resolved_asset_id = resolved_asset_id.ok_or_else(|| {
                AtelierError::NotFound(format!(
                    "target media asset for intake item {}",
                    existing.item_id
                ))
            })?;
            asset_id = Some(resolved_asset_id);

            if let Some(target_collection_id) = batch.target_collection_id {
                let inserted = self
                    .add_images_to_collection(target_collection_id, &[resolved_asset_id])
                    .await?;
                collection_id = Some(target_collection_id);
                collection_inserted = inserted > 0;
            }
        }

        let mut item = existing.clone();
        let changed = existing.lane != request.lane || existing.lane_reason != normalized_reason;
        if changed {
            let bindings = ItemClassificationBindings {
                item_ref: RecordId::new("atelier_intake_item", SurrealUuid::from(request.item_id)),
                batch_ref: RecordId::new(
                    "atelier_intake_batch",
                    SurrealUuid::from(existing.batch_id),
                ),
                lane: request.lane.as_str().to_owned(),
                lane_reason: normalized_reason.clone(),
            };
            let row: Option<serde_json::Value> = self
                .write_with_event(
                    CLASSIFY_ITEM_STATEMENT,
                    bindings,
                    intake_event_family::INTAKE_ITEM_CLASSIFIED,
                    "atelier_intake_item",
                    &request.item_id.to_string(),
                    serde_json::json!({
                        "batch_id": existing.batch_id,
                        "lane": request.lane,
                        "reason": normalized_reason,
                        "source_path_ref": event_ref_for_text(&existing.source_path),
                        "asset_id": asset_id,
                        "collection_id": collection_id,
                        "apply_workflow": true,
                    }),
                )
                .await?;
            item = item_from_row(&intake_row(row.ok_or_else(|| {
                AtelierError::Internal("applying intake classification returned no row".to_owned())
            })?)?)?;
            self.insert_rejection_audit(&item).await?;
        }
        Ok(IntakeClassificationApplyResult {
            item,
            asset_id,
            collection_id,
            collection_inserted,
        })
    }

    /// Per-lane counts for the sorter header.
    pub async fn intake_lane_counts(&self, batch_id: Uuid) -> AtelierResult<IntakeLaneCounts> {
        let mut counts = IntakeLaneCounts::default();
        for item in self.list_intake_items(batch_id, None).await? {
            match item.lane {
                IntakeLane::Pending => counts.pending += 1,
                IntakeLane::Accepted => counts.accepted += 1,
                IntakeLane::Rejected => counts.rejected += 1,
                IntakeLane::Deferred => counts.deferred += 1,
                IntakeLane::Skipped => counts.skipped += 1,
                IntakeLane::Failed => counts.failed += 1,
            }
        }
        Ok(counts)
    }

    /// Close a batch once triage is done. Refuses to close while any item is
    /// still in the `Pending` lane, so nothing is silently dropped. Returns the
    /// updated batch.
    pub async fn close_intake_batch(&self, batch_id: Uuid) -> AtelierResult<IntakeBatch> {
        let counts = self.intake_lane_counts(batch_id).await?;
        if counts.pending > 0 {
            return Err(AtelierError::Validation(format!(
                "cannot close intake batch {batch_id}: {} item(s) still in the pending lane",
                counts.pending
            )));
        }

        if self.get_intake_batch_by_id(batch_id).await?.is_none() {
            return Err(AtelierError::NotFound(format!("intake batch {batch_id}")));
        }
        let bindings = BatchRefBinding {
            batch_ref: RecordId::new("atelier_intake_batch", SurrealUuid::from(batch_id)),
        };
        let row: Option<serde_json::Value> = self
            .write_with_event(
                CLOSE_BATCH_STATEMENT,
                bindings,
                intake_event_family::INTAKE_BATCH_CLOSED,
                "atelier_intake_batch",
                &batch_id.to_string(),
                serde_json::json!({
                    "batch_id": batch_id,
                    "accepted": counts.accepted,
                    "rejected": counts.rejected,
                    "deferred": counts.deferred,
                    "skipped": counts.skipped,
                    "failed": counts.failed,
                }),
            )
            .await?;
        let row = intake_row(row.ok_or_else(|| {
            AtelierError::Internal("closing an intake batch returned no row".to_owned())
        })?)?;
        Ok(batch_from_row(&row))
    }
}
