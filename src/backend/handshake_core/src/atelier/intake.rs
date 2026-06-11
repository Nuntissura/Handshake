//! Intake / inbox sorting (MT-016): persistent intake batches and per-item
//! accept / reject / defer / skip / fail lanes for the operator's
//! "IntakeSorterView" flow.
//!
//! legacy source source: `app/backend/library.js` (`createIntakeBatch`,
//! `listIntakeBatches`, `getIntakeBatch`, `updateIntakeBatchItem`,
//! `classifyIntakeBatch`, `_normalizeIntakeStatus`) and `app/backend/db.js`
//! (`IntakeBatch` / `IntakeBatchItem` tables). Schema/behavior INTENT only;
//! the SQLite originals are not copied. Storage authority is PostgreSQL.
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
use sqlx::{Postgres, Row, Transaction};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::{
    collections::collections_event_family, event_ref_for_text,
    media::MEDIA_ORIGINAL_RETENTION_CLASS, reject_legacy_runtime_ref, AtelierError, AtelierResult,
    AtelierStore,
};

pub const ORPHAN_MANIFEST_SCHEMA_ID: &str = "hsk.atelier.orphan_manifest@1";

/// Intake event families (extends the MT-005 coverage set). The parent wires
/// these into [`super::event_family::ALL`].
pub mod intake_event_family {
    /// A persistent intake batch was opened.
    pub const INTAKE_BATCH_CREATED: &str = "atelier.intake.batch_created";
    /// A source file was registered as an item in a batch.
    pub const INTAKE_ITEM_ADDED: &str = "atelier.intake.item_added";
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
#[derive(Clone, Debug)]
pub struct ApplyIntakeClassificationRequest {
    pub item_id: Uuid,
    pub lane: IntakeLane,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntakeClassificationApplyResult {
    pub item: IntakeItem,
    pub asset_id: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    pub collection_inserted: bool,
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

fn batch_from_row(row: &sqlx::postgres::PgRow) -> IntakeBatch {
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

fn item_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<IntakeItem> {
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

fn rejection_audit_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<IntakeItemRejectionAudit> {
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

fn reset_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<AtelierResetRecord> {
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

fn orphan_manifest_item_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<OrphanManifestItem> {
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

impl AtelierStore {
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
        let mut tx = self.pool().begin().await?;
        let result = self
            .scan_inbox_folder_import_in_tx(
                request,
                enumeration,
                requested_max_files,
                effective_max_files_i64,
                &mut tx,
            )
            .await;
        match result {
            Ok(result) => {
                tx.commit().await?;
                Ok(result)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    async fn scan_inbox_folder_import_in_tx(
        &self,
        request: &InboxFolderScanRequest,
        enumeration: InboxFolderEnumeration,
        requested_max_files: i64,
        effective_max_files: i64,
        tx: &mut Transaction<'_, Postgres>,
    ) -> AtelierResult<InboxFolderScanResult> {
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(advisory_lock_key(&request.idempotency_key))
            .execute(&mut **tx)
            .await?;

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
            .open_intake_batch_in_tx(
                tx,
                &NewIntakeBatch {
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
                },
            )
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

        let previous_root_ref: Option<String> = sqlx::query_scalar(
            r#"SELECT payload->>'root_path_ref'
               FROM atelier_event
               WHERE event_family = $1
                 AND aggregate_type = 'atelier_intake_batch'
                 AND aggregate_id = $2
               ORDER BY created_at_utc ASC
               LIMIT 1"#,
        )
        .bind(intake_event_family::INTAKE_FOLDER_SCAN_COMPLETED)
        .bind(batch.batch_id.to_string())
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(previous_root_ref) = previous_root_ref {
            if previous_root_ref != enumeration.root_path_ref {
                return Err(AtelierError::Validation(
                    "inbox_root does not match the previous folder scan for this idempotency_key"
                        .into(),
                ));
            }
        }

        if batch_inserted {
            self.record_event_in_tx(
                tx,
                intake_event_family::INTAKE_BATCH_CREATED,
                "atelier_intake_batch",
                &batch.batch_id.to_string(),
                serde_json::json!({
                    "batch_id": batch.batch_id,
                    "idempotency_key": batch.idempotency_key,
                    "source_label": batch.source_label,
                    "source_ref": batch.source_ref,
                    "mode": batch.mode,
                    "resume_cursor": batch.resume_cursor,
                    "character_scoped": batch.character_internal_id.is_some(),
                }),
            )
            .await?;
        }

        let mut imported_count = 0_i64;
        let mut duplicate_skipped_count = 0_i64;
        let mut items = Vec::new();
        for file in enumeration.files {
            let (item, inserted) = self
                .insert_intake_item_in_tx(
                    tx,
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
                self.record_event_in_tx(
                    tx,
                    intake_event_family::INTAKE_ITEM_ADDED,
                    "atelier_intake_item",
                    &item.item_id.to_string(),
                    serde_json::json!({
                        "batch_id": item.batch_id,
                        "source_path_ref": event_ref_for_text(&item.source_path),
                        "file_name_ref": event_ref_for_text(&item.file_name),
                        "byte_len": item.byte_len,
                    }),
                )
                .await?;
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

        self.record_event_in_tx(
            tx,
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

    async fn open_intake_batch_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        new: &NewIntakeBatch,
    ) -> AtelierResult<(IntakeBatch, bool)> {
        let (explicit_source_ref, source_ref) = normalize_batch_source_refs(new)?;
        let resume_cursor = normalize_optional_batch_ref("resume_cursor", &new.resume_cursor)?;
        let targets = normalize_batch_targets(new)?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(advisory_lock_key(&format!(
                "atelier-intake-batch:{}",
                new.idempotency_key
            )))
            .execute(&mut **tx)
            .await?;

        let row = sqlx::query(
            r#"WITH inserted AS (
                 INSERT INTO atelier_intake_batch
                   (idempotency_key, source_label, source_ref, mode,
                    profile_mode, character_internal_id, target_character_id,
                    target_sheet_version_id, target_collection_id, status, resume_cursor)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'open', $10)
                 ON CONFLICT (idempotency_key) DO NOTHING
                 RETURNING batch_id, idempotency_key, source_label,
                           source_ref, mode, profile_mode, character_internal_id,
                           target_character_id, target_sheet_version_id,
                           target_collection_id, status, resume_cursor,
                           resumed_at_utc, created_at_utc, updated_at_utc
               )
               SELECT TRUE AS inserted, batch_id, idempotency_key, source_label,
                      source_ref, mode, profile_mode, character_internal_id,
                      target_character_id, target_sheet_version_id,
                      target_collection_id, status, resume_cursor,
                      resumed_at_utc, created_at_utc, updated_at_utc
               FROM inserted
               UNION ALL
               SELECT FALSE AS inserted, batch_id, idempotency_key, source_label,
                      source_ref, mode, profile_mode, character_internal_id,
                      target_character_id, target_sheet_version_id,
                      target_collection_id, status, resume_cursor,
                      resumed_at_utc, created_at_utc, updated_at_utc
               FROM atelier_intake_batch
               WHERE idempotency_key = $1
               LIMIT 1"#,
        )
        .bind(&new.idempotency_key)
        .bind(&new.source_label)
        .bind(&source_ref)
        .bind(new.mode.as_str())
        .bind(new.profile_mode.as_str())
        .bind(targets.character_internal_id)
        .bind(targets.target_character_id)
        .bind(targets.target_sheet_version_id)
        .bind(targets.target_collection_id)
        .bind(&resume_cursor)
        .fetch_one(&mut **tx)
        .await?;
        let inserted: bool = row.get("inserted");
        let batch = batch_from_row(&row);
        if !inserted {
            validate_intake_batch_reopen_contract(
                &batch,
                new,
                explicit_source_ref.as_deref(),
                targets,
            )?;
        }
        Ok((batch, inserted))
    }

    async fn insert_intake_item_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        batch_id: Uuid,
        new: &NewIntakeItem,
    ) -> AtelierResult<(IntakeItem, bool)> {
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(advisory_lock_key(&format!(
                "atelier-intake-item:{batch_id}:{}",
                new.source_path
            )))
            .execute(&mut **tx)
            .await?;

        let batch_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT batch_id FROM atelier_intake_batch WHERE batch_id = $1")
                .bind(batch_id)
                .fetch_optional(&mut **tx)
                .await?;
        if batch_exists.is_none() {
            return Err(AtelierError::NotFound(format!("intake batch {batch_id}")));
        }

        let row = sqlx::query(
            r#"WITH inserted AS (
                 INSERT INTO atelier_intake_item
                   (batch_id, source_path, file_name, byte_len, content_hash, lane)
                 VALUES ($1, $2, $3, $4, $5, 'pending')
                 ON CONFLICT (batch_id, source_path) DO NOTHING
                 RETURNING item_id, batch_id, source_path, file_name, byte_len,
                           content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               )
               SELECT TRUE AS inserted, item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM inserted
               UNION ALL
               SELECT FALSE AS inserted, item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM atelier_intake_item
               WHERE batch_id = $1 AND source_path = $2
               LIMIT 1"#,
        )
        .bind(batch_id)
        .bind(&new.source_path)
        .bind(&new.file_name)
        .bind(new.byte_len)
        .bind(&new.content_hash)
        .fetch_one(&mut **tx)
        .await?;
        let inserted: bool = row.get("inserted");
        if inserted {
            sqlx::query(
                "UPDATE atelier_intake_batch SET updated_at_utc = NOW() WHERE batch_id = $1",
            )
            .bind(batch_id)
            .execute(&mut **tx)
            .await?;
        }
        Ok((item_from_row(&row)?, inserted))
    }

    async fn insert_rejection_audit_in_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
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
        let row = sqlx::query(
            r#"WITH inserted AS (
                 INSERT INTO atelier_intake_item_rejection_audit
                   (item_id, batch_id, lane, reason, source_path_ref)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (item_id, lane, reason) DO NOTHING
                 RETURNING audit_id, item_id, batch_id, lane, reason,
                           source_path_ref, created_at_utc
               )
               SELECT TRUE AS inserted, audit_id, item_id, batch_id, lane, reason,
                      source_path_ref, created_at_utc
               FROM inserted
               UNION ALL
               SELECT FALSE AS inserted, audit_id, item_id, batch_id, lane, reason,
                      source_path_ref, created_at_utc
               FROM atelier_intake_item_rejection_audit
               WHERE item_id = $1 AND lane = $3 AND reason = $4
               LIMIT 1"#,
        )
        .bind(item.item_id)
        .bind(item.batch_id)
        .bind(item.lane.as_str())
        .bind(reason)
        .bind(&source_path_ref)
        .fetch_one(&mut **tx)
        .await?;
        let inserted: bool = row.get("inserted");
        Ok(Some((rejection_audit_from_row(&row)?, inserted)))
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
        let (explicit_source_ref, source_ref) = normalize_batch_source_refs(new)?;
        let resume_cursor = normalize_optional_batch_ref("resume_cursor", &new.resume_cursor)?;
        let targets = normalize_batch_targets(new)?;

        // Idempotent fast path: an existing batch with this key wins.
        if let Some(existing) = self.get_intake_batch_by_key(&new.idempotency_key).await? {
            validate_intake_batch_reopen_contract(
                &existing,
                new,
                explicit_source_ref.as_deref(),
                targets,
            )?;
            return Ok(existing);
        }

        let mut tx = self.pool().begin().await?;
        let result = async {
            let (batch, inserted) = self.open_intake_batch_in_tx(&mut tx, new).await?;
            if inserted {
                self.record_event_in_tx(
                    &mut tx,
                    intake_event_family::INTAKE_BATCH_CREATED,
                    "atelier_intake_batch",
                    &batch.batch_id.to_string(),
                    serde_json::json!({
                        "batch_id": batch.batch_id,
                        "idempotency_key": batch.idempotency_key,
                        "source_label": batch.source_label,
                        "source_ref": source_ref,
                        "mode": batch.mode,
                        "profile_mode": batch.profile_mode,
                        "resume_cursor": resume_cursor,
                        "character_scoped": batch.character_internal_id.is_some(),
                        "target_character_ref": batch
                            .target_character_id
                            .map(|id| event_ref_for_text(&id.to_string())),
                        "target_sheet_version_ref": batch
                            .target_sheet_version_id
                            .map(|id| event_ref_for_text(&id.to_string())),
                        "target_collection_ref": batch
                            .target_collection_id
                            .map(|id| event_ref_for_text(&id.to_string())),
                    }),
                )
                .await?;
            }
            Ok(batch)
        }
        .await;
        match result {
            Ok(batch) => {
                tx.commit().await?;
                Ok(batch)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    /// Fetch a batch by its stable idempotency key.
    pub async fn get_intake_batch_by_key(
        &self,
        idempotency_key: &str,
    ) -> AtelierResult<Option<IntakeBatch>> {
        let row = sqlx::query(
            r#"SELECT batch_id, idempotency_key, source_label, character_internal_id,
                      source_ref, mode, profile_mode, target_character_id,
                      target_sheet_version_id, target_collection_id,
                      status, resume_cursor, resumed_at_utc,
                      created_at_utc, updated_at_utc
               FROM atelier_intake_batch WHERE idempotency_key = $1"#,
        )
        .bind(idempotency_key)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(batch_from_row))
    }

    /// List batches, newest first, optionally filtered by status.
    pub async fn list_intake_batches(
        &self,
        status: Option<BatchStatus>,
        limit: i64,
    ) -> AtelierResult<Vec<IntakeBatch>> {
        let capped = limit.clamp(1, 1000);
        let rows = sqlx::query(
            r#"SELECT batch_id, idempotency_key, source_label, character_internal_id,
                      source_ref, mode, profile_mode, target_character_id,
                      target_sheet_version_id, target_collection_id,
                      status, resume_cursor, resumed_at_utc,
                      created_at_utc, updated_at_utc
               FROM atelier_intake_batch
               WHERE ($1::TEXT IS NULL OR status = $1)
               ORDER BY updated_at_utc DESC
               LIMIT $2"#,
        )
        .bind(status.map(|s| s.as_str()))
        .bind(capped)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(batch_from_row).collect())
    }

    /// List batches by profile linkage mode, newest first. `None` returns all
    /// profile modes while preserving the same cap behavior as status listing.
    pub async fn list_intake_batches_by_profile_mode(
        &self,
        profile_mode: Option<IntakeProfileMode>,
        limit: i64,
    ) -> AtelierResult<Vec<IntakeBatch>> {
        let capped = limit.clamp(1, 1000);
        let rows = sqlx::query(
            r#"SELECT batch_id, idempotency_key, source_label, character_internal_id,
                      source_ref, mode, profile_mode, target_character_id,
                      target_sheet_version_id, target_collection_id,
                      status, resume_cursor, resumed_at_utc,
                      created_at_utc, updated_at_utc
               FROM atelier_intake_batch
               WHERE ($1::TEXT IS NULL OR profile_mode = $1)
               ORDER BY updated_at_utc DESC
               LIMIT $2"#,
        )
        .bind(profile_mode.map(|mode| mode.as_str()))
        .bind(capped)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(batch_from_row).collect())
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

        let row = sqlx::query(
            r#"UPDATE atelier_intake_batch
               SET status = 'in_progress',
                   resume_cursor = $2,
                   resumed_at_utc = NOW(),
                   updated_at_utc = NOW()
               WHERE batch_id = $1
                 AND status <> 'closed'
               RETURNING batch_id, idempotency_key, source_label, source_ref,
                         mode, profile_mode, character_internal_id,
                         target_character_id, target_sheet_version_id,
                         target_collection_id, status, resume_cursor,
                         resumed_at_utc, created_at_utc, updated_at_utc"#,
        )
        .bind(batch_id)
        .bind(&cursor)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("open intake batch {batch_id}")))?;
        let batch = batch_from_row(&row);

        self.record_event(
            intake_event_family::INTAKE_BATCH_RESUMED,
            "atelier_intake_batch",
            &batch.batch_id.to_string(),
            serde_json::json!({
                "batch_id": batch.batch_id,
                "source_ref": batch.source_ref,
                "mode": batch.mode,
                "status": batch.status,
                "resume_cursor": batch.resume_cursor,
                "requested_by": requested_by,
            }),
        )
        .await?;
        Ok(batch)
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
        if new.source_path.trim().is_empty() {
            return Err(AtelierError::Validation(
                "source_path must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("source_path", &new.source_path)?;
        // Idempotent fast path: existing item for this source in this batch.
        if let Some(existing) = self.get_intake_item(batch_id, &new.source_path).await? {
            return Ok(existing);
        }

        let mut tx = self.pool().begin().await?;
        let result = async {
            let (item, inserted) = self
                .insert_intake_item_in_tx(&mut tx, batch_id, new)
                .await?;
            if inserted {
                self.record_event_in_tx(
                    &mut tx,
                    intake_event_family::INTAKE_ITEM_ADDED,
                    "atelier_intake_item",
                    &item.item_id.to_string(),
                    serde_json::json!({
                        "batch_id": item.batch_id,
                        "source_path_ref": event_ref_for_text(&item.source_path),
                        "file_name_ref": event_ref_for_text(&item.file_name),
                        "byte_len": item.byte_len,
                    }),
                )
                .await?;
            }
            Ok(item)
        }
        .await;
        match result {
            Ok(item) => {
                tx.commit().await?;
                Ok(item)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    /// Fetch a single item by its preserved source path within a batch.
    pub async fn get_intake_item(
        &self,
        batch_id: Uuid,
        source_path: &str,
    ) -> AtelierResult<Option<IntakeItem>> {
        let row = sqlx::query(
            r#"SELECT item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM atelier_intake_item
               WHERE batch_id = $1 AND source_path = $2"#,
        )
        .bind(batch_id)
        .bind(source_path)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(item_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// List the items in a batch (creation order), optionally filtered to a lane.
    pub async fn list_intake_items(
        &self,
        batch_id: Uuid,
        lane: Option<IntakeLane>,
    ) -> AtelierResult<Vec<IntakeItem>> {
        let rows = sqlx::query(
            r#"SELECT item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM atelier_intake_item
               WHERE batch_id = $1 AND ($2::TEXT IS NULL OR lane = $2)
               ORDER BY created_at_utc ASC"#,
        )
        .bind(batch_id)
        .bind(lane.map(|l| l.as_str()))
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(item_from_row).collect()
    }

    /// List durable negative-lifecycle audit rows for a batch.
    pub async fn list_intake_rejection_audits(
        &self,
        batch_id: Uuid,
    ) -> AtelierResult<Vec<IntakeItemRejectionAudit>> {
        let rows = sqlx::query(
            r#"SELECT audit_id, item_id, batch_id, lane, reason,
                      source_path_ref, created_at_utc
               FROM atelier_intake_item_rejection_audit
               WHERE batch_id = $1
               ORDER BY created_at_utc ASC, audit_id ASC"#,
        )
        .bind(batch_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(rejection_audit_from_row).collect()
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
        let mut tx = self.pool().begin().await?;
        let result = async {
            sqlx::query(
                r#"INSERT INTO atelier_reset_operation
                   (reset_id, mode, requested_by, reason)
                   VALUES ($1, $2, $3, $4)"#,
            )
            .bind(reset_id)
            .bind(request.mode.as_str())
            .bind(requested_by)
            .bind(reason)
            .execute(&mut *tx)
            .await?;

            let preference_rows =
                sqlx::query("DELETE FROM atelier_preference RETURNING preference_id")
                    .fetch_all(&mut *tx)
                    .await?;
            let preferences_deleted_count =
                checked_usize_to_i64("preferences_deleted_count", preference_rows.len())?;

            let mut original_media_preserved_count = 0_i64;
            let mut orphan_manifest_id = None;
            if request.mode == AtelierResetMode::FullPreserveOriginalMedia {
                let manifest_id = Uuid::now_v7();
                let media_rows = sqlx::query(
                    r#"SELECT asset_id, content_hash, artifact_ref, mime, byte_len, retention_class
                       FROM atelier_media_asset
                       WHERE retention_class = $1
                         AND btrim(content_hash) = content_hash
                         AND btrim(content_hash) <> ''
                         AND btrim(artifact_ref) = artifact_ref
                         AND btrim(artifact_ref) <> ''
                         AND btrim(mime) = mime
                         AND btrim(mime) <> ''
                         AND byte_len > 0
                         AND btrim(retention_class) = retention_class
                         AND btrim(retention_class) <> ''
                       ORDER BY created_at_utc ASC, asset_id ASC"#,
                )
                .bind(MEDIA_ORIGINAL_RETENTION_CLASS)
                .fetch_all(&mut *tx)
                .await?;
                original_media_preserved_count =
                    checked_usize_to_i64("original_media_preserved_count", media_rows.len())?;
                let manifest_json = serde_json::json!({
                    "schema_id": ORPHAN_MANIFEST_SCHEMA_ID,
                    "reset_id": reset_id,
                    "reset_mode": request.mode.as_str(),
                    "item_count": original_media_preserved_count,
                    "retention_class": MEDIA_ORIGINAL_RETENTION_CLASS,
                });
                sqlx::query(
                    r#"INSERT INTO atelier_orphan_manifest
                       (manifest_id, reset_id, manifest_json, item_count)
                       VALUES ($1, $2, $3, $4)"#,
                )
                .bind(manifest_id)
                .bind(reset_id)
                .bind(manifest_json)
                .bind(original_media_preserved_count)
                .execute(&mut *tx)
                .await?;

                for row in &media_rows {
                    sqlx::query(
                        r#"INSERT INTO atelier_orphan_manifest_item
                           (manifest_item_id, manifest_id, asset_id, content_hash,
                            artifact_ref, mime, byte_len, retention_class)
                           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                           ON CONFLICT (manifest_id, asset_id) DO NOTHING"#,
                    )
                    .bind(Uuid::now_v7())
                    .bind(manifest_id)
                    .bind(row.get::<Uuid, _>("asset_id"))
                    .bind(row.get::<String, _>("content_hash"))
                    .bind(row.get::<String, _>("artifact_ref"))
                    .bind(row.get::<String, _>("mime"))
                    .bind(row.get::<i64, _>("byte_len"))
                    .bind(row.get::<String, _>("retention_class"))
                    .execute(&mut *tx)
                    .await?;
                }

                self.record_event_in_tx(
                    &mut tx,
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
                orphan_manifest_id = Some(manifest_id);
            }

            let row = sqlx::query(
                r#"UPDATE atelier_reset_operation
                   SET preferences_deleted_count = $2,
                       original_media_preserved_count = $3,
                       orphan_manifest_id = $4
                   WHERE reset_id = $1
                   RETURNING reset_id, mode, requested_by, reason,
                             preferences_deleted_count,
                             original_media_preserved_count,
                             orphan_manifest_id, created_at_utc"#,
            )
            .bind(reset_id)
            .bind(preferences_deleted_count)
            .bind(original_media_preserved_count)
            .bind(orphan_manifest_id)
            .fetch_one(&mut *tx)
            .await?;
            let reset = reset_from_row(&row)?;

            self.record_event_in_tx(
                &mut tx,
                intake_event_family::RESET_RECORDED,
                "atelier_reset_operation",
                &reset.reset_id.to_string(),
                serde_json::json!({
                    "mode": reset.mode.as_str(),
                    "requested_by": reset.requested_by,
                    "reason_ref": event_ref_for_text(&reset.reason),
                    "preferences_deleted_count": reset.preferences_deleted_count,
                    "original_media_preserved_count": reset.original_media_preserved_count,
                    "orphan_manifest_id": reset.orphan_manifest_id,
                }),
            )
            .await?;
            Ok(reset)
        }
        .await;
        match result {
            Ok(reset) => {
                tx.commit().await?;
                Ok(reset)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    pub async fn list_orphan_manifest_items(
        &self,
        manifest_id: Uuid,
    ) -> AtelierResult<Vec<OrphanManifestItem>> {
        let rows = sqlx::query(
            r#"SELECT manifest_item_id, manifest_id, asset_id, content_hash,
                      artifact_ref, mime, byte_len, retention_class,
                      adoption_status, adopted_batch_id, adopted_item_id,
                      adopted_by, adopted_at_utc, created_at_utc
               FROM atelier_orphan_manifest_item
               WHERE manifest_id = $1
               ORDER BY created_at_utc ASC, manifest_item_id ASC"#,
        )
        .bind(manifest_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(orphan_manifest_item_from_row).collect()
    }

    pub async fn adopt_orphan_manifest_item(
        &self,
        request: &OrphanAdoptionRequest,
    ) -> AtelierResult<OrphanAdoptionResult> {
        let requested_by = require_scan_text("requested_by", &request.requested_by)?;
        reject_legacy_runtime_ref("requested_by", requested_by)?;

        let mut tx = self.pool().begin().await?;
        let result = async {
            let row = sqlx::query(
                r#"SELECT manifest_item_id, manifest_id, asset_id, content_hash,
                          artifact_ref, mime, byte_len, retention_class,
                          adoption_status, adopted_batch_id, adopted_item_id,
                          adopted_by, adopted_at_utc, created_at_utc
                   FROM atelier_orphan_manifest_item
                   WHERE manifest_item_id = $1
                   FOR UPDATE"#,
            )
            .bind(request.manifest_item_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("orphan manifest item {}", request.manifest_item_id))
            })?;
            let manifest_item = orphan_manifest_item_from_row(&row)?;

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
                let batch_row = sqlx::query(
                    r#"SELECT batch_id, idempotency_key, source_label, source_ref,
                              mode, profile_mode, character_internal_id,
                              target_character_id, target_sheet_version_id,
                              target_collection_id, status, resume_cursor,
                              resumed_at_utc, created_at_utc, updated_at_utc
                       FROM atelier_intake_batch
                       WHERE batch_id = $1"#,
                )
                .bind(batch_id)
                .fetch_one(&mut *tx)
                .await?;
                let item_row = sqlx::query(
                    r#"SELECT item_id, batch_id, source_path, file_name, byte_len,
                              content_hash, lane, lane_reason, created_at_utc, updated_at_utc
                       FROM atelier_intake_item
                       WHERE item_id = $1"#,
                )
                .bind(item_id)
                .fetch_one(&mut *tx)
                .await?;
                return Ok(OrphanAdoptionResult {
                    manifest_item,
                    batch: batch_from_row(&batch_row),
                    item: item_from_row(&item_row)?,
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
            let (batch, batch_inserted) = self
                .open_intake_batch_in_tx(&mut tx, &batch_request)
                .await?;
            if batch_inserted {
                self.record_event_in_tx(
                    &mut tx,
                    intake_event_family::INTAKE_BATCH_CREATED,
                    "atelier_intake_batch",
                    &batch.batch_id.to_string(),
                    serde_json::json!({
                        "batch_id": batch.batch_id,
                        "idempotency_key": batch.idempotency_key,
                        "source_label": batch.source_label,
                        "source_ref": batch.source_ref,
                        "mode": batch.mode,
                        "profile_mode": batch.profile_mode,
                        "orphan_manifest_id": manifest_item.manifest_id,
                    }),
                )
                .await?;
            }

            let item_request = NewIntakeItem {
                source_path: manifest_item.artifact_ref.clone(),
                file_name: orphan_intake_file_name(
                    &manifest_item.content_hash,
                    &manifest_item.mime,
                ),
                byte_len: manifest_item.byte_len,
                content_hash: Some(manifest_item.content_hash.clone()),
            };
            let (item, item_inserted) = self
                .insert_intake_item_in_tx(&mut tx, batch.batch_id, &item_request)
                .await?;
            if item_inserted {
                self.record_event_in_tx(
                    &mut tx,
                    intake_event_family::INTAKE_ITEM_ADDED,
                    "atelier_intake_item",
                    &item.item_id.to_string(),
                    serde_json::json!({
                        "batch_id": item.batch_id,
                        "source_path_ref": event_ref_for_text(&item.source_path),
                        "file_name_ref": event_ref_for_text(&item.file_name),
                        "byte_len": item.byte_len,
                        "orphan_manifest_item_id": manifest_item.manifest_item_id,
                    }),
                )
                .await?;
            }

            let updated_row = sqlx::query(
                r#"UPDATE atelier_orphan_manifest_item
                   SET adoption_status = $2,
                       adopted_batch_id = $3,
                       adopted_item_id = $4,
                       adopted_by = $5,
                       adopted_at_utc = NOW()
                   WHERE manifest_item_id = $1
                   RETURNING manifest_item_id, manifest_id, asset_id, content_hash,
                             artifact_ref, mime, byte_len, retention_class,
                             adoption_status, adopted_batch_id, adopted_item_id,
                             adopted_by, adopted_at_utc, created_at_utc"#,
            )
            .bind(manifest_item.manifest_item_id)
            .bind(OrphanAdoptionStatus::Adopted.as_str())
            .bind(batch.batch_id)
            .bind(item.item_id)
            .bind(requested_by)
            .fetch_one(&mut *tx)
            .await?;
            let updated_manifest_item = orphan_manifest_item_from_row(&updated_row)?;

            self.record_event_in_tx(
                &mut tx,
                intake_event_family::ORPHAN_MANIFEST_ITEM_ADOPTED,
                "atelier_orphan_manifest_item",
                &updated_manifest_item.manifest_item_id.to_string(),
                serde_json::json!({
                    "manifest_id": updated_manifest_item.manifest_id,
                    "asset_id": updated_manifest_item.asset_id,
                    "content_hash": updated_manifest_item.content_hash,
                    "adopted_batch_id": batch.batch_id,
                    "adopted_item_id": item.item_id,
                    "requested_by": requested_by,
                }),
            )
            .await?;

            Ok(OrphanAdoptionResult {
                manifest_item: updated_manifest_item,
                batch,
                item,
            })
        }
        .await;
        match result {
            Ok(adoption) => {
                tx.commit().await?;
                Ok(adoption)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
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
        let mut tx = self.pool().begin().await?;

        let existing_row = sqlx::query(
            r#"SELECT item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM atelier_intake_item
               WHERE item_id = $1
               FOR UPDATE"#,
        )
        .bind(item_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake item {item_id}")))?;
        let existing = item_from_row(&existing_row)?;
        if existing.lane == lane && existing.lane_reason == normalized_reason {
            tx.commit().await?;
            return Ok(existing);
        }

        let row = sqlx::query(
            r#"UPDATE atelier_intake_item
               SET lane = $2, lane_reason = $3, updated_at_utc = NOW()
               WHERE item_id = $1
               RETURNING item_id, batch_id, source_path, file_name, byte_len,
                         content_hash, lane, lane_reason, created_at_utc, updated_at_utc"#,
        )
        .bind(item_id)
        .bind(lane.as_str())
        .bind(normalized_reason.as_deref())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake item {item_id}")))?;

        let item = item_from_row(&row)?;
        let audit = self.insert_rejection_audit_in_tx(&mut tx, &item).await?;

        sqlx::query("UPDATE atelier_intake_batch SET updated_at_utc = NOW() WHERE batch_id = $1")
            .bind(item.batch_id)
            .execute(&mut *tx)
            .await?;

        self.record_event_in_tx(
            &mut tx,
            intake_event_family::INTAKE_ITEM_CLASSIFIED,
            "atelier_intake_item",
            &item.item_id.to_string(),
            serde_json::json!({
                "batch_id": item.batch_id,
                "lane": item.lane,
                "reason": item.lane_reason,
                "source_path_ref": event_ref_for_text(&item.source_path),
            }),
        )
        .await?;

        if let Some((audit, inserted)) = audit {
            if inserted {
                self.record_event_in_tx(
                    &mut tx,
                    intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
                    "atelier_intake_item",
                    &item.item_id.to_string(),
                    serde_json::json!({
                        "audit_id": audit.audit_id,
                        "batch_id": audit.batch_id,
                        "lane": audit.lane,
                        "reason_ref": event_ref_for_text(&audit.reason),
                        "source_path_ref": audit.source_path_ref,
                    }),
                )
                .await?;
            }
        }

        tx.commit().await?;
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
        let mut tx = self.pool().begin().await?;

        let existing_row = sqlx::query(
            r#"SELECT item_id, batch_id, source_path, file_name, byte_len,
                      content_hash, lane, lane_reason, created_at_utc, updated_at_utc
               FROM atelier_intake_item
               WHERE item_id = $1
               FOR UPDATE"#,
        )
        .bind(request.item_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake item {}", request.item_id)))?;
        let existing = item_from_row(&existing_row)?;

        let batch_row = sqlx::query(
            r#"SELECT batch_id, target_collection_id
               FROM atelier_intake_batch
               WHERE batch_id = $1
               FOR UPDATE"#,
        )
        .bind(existing.batch_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake batch {}", existing.batch_id)))?;
        let target_collection_id: Option<Uuid> = batch_row.get("target_collection_id");

        let mut asset_id = None;
        let mut collection_id = None;
        let mut collection_inserted = false;
        if request.lane == IntakeLane::Accepted {
            let content_hash = existing.content_hash.as_deref().ok_or_else(|| {
                AtelierError::Validation(
                    "accepted intake item requires target media asset content_hash".into(),
                )
            })?;
            let resolved_asset_id: Uuid = sqlx::query_scalar(
                "SELECT asset_id FROM atelier_media_asset WHERE content_hash = $1",
            )
            .bind(content_hash)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!(
                    "target media asset for intake item {}",
                    existing.item_id
                ))
            })?;
            asset_id = Some(resolved_asset_id);

            if let Some(target_collection_id) = target_collection_id {
                let collection_exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM atelier_collection WHERE collection_id = $1)",
                )
                .bind(target_collection_id)
                .fetch_one(&mut *tx)
                .await?;
                if !collection_exists {
                    return Err(AtelierError::NotFound(format!(
                        "target collection {target_collection_id}"
                    )));
                }

                let next_order: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM atelier_collection_item WHERE collection_id = $1",
                )
                .bind(target_collection_id)
                .fetch_one(&mut *tx)
                .await?;
                let inserted = sqlx::query(
                    r#"INSERT INTO atelier_collection_item (collection_id, asset_id, sort_order)
                       VALUES ($1, $2, $3)
                       ON CONFLICT (collection_id, asset_id) DO NOTHING"#,
                )
                .bind(target_collection_id)
                .bind(resolved_asset_id)
                .bind(next_order)
                .execute(&mut *tx)
                .await?
                .rows_affected()
                    > 0;
                collection_id = Some(target_collection_id);
                collection_inserted = inserted;
                if inserted {
                    sqlx::query(
                        "UPDATE atelier_collection SET updated_at_utc = NOW() WHERE collection_id = $1",
                    )
                    .bind(target_collection_id)
                    .execute(&mut *tx)
                    .await?;
                    self.record_event_in_tx(
                        &mut tx,
                        collections_event_family::COLLECTION_IMAGES_ADDED,
                        "atelier_collection",
                        &target_collection_id.to_string(),
                        serde_json::json!({
                            "requested": 1,
                            "inserted": 1,
                            "asset_id": resolved_asset_id,
                            "intake_item_id": existing.item_id,
                        }),
                    )
                    .await?;
                }
            }
        }

        let mut item = existing.clone();
        let changed = existing.lane != request.lane || existing.lane_reason != normalized_reason;
        if changed {
            let row = sqlx::query(
                r#"UPDATE atelier_intake_item
                   SET lane = $2, lane_reason = $3, updated_at_utc = NOW()
                   WHERE item_id = $1
                   RETURNING item_id, batch_id, source_path, file_name, byte_len,
                             content_hash, lane, lane_reason, created_at_utc, updated_at_utc"#,
            )
            .bind(request.item_id)
            .bind(request.lane.as_str())
            .bind(normalized_reason.as_deref())
            .fetch_one(&mut *tx)
            .await?;

            item = item_from_row(&row)?;
            let audit = self.insert_rejection_audit_in_tx(&mut tx, &item).await?;

            sqlx::query(
                "UPDATE atelier_intake_batch SET updated_at_utc = NOW() WHERE batch_id = $1",
            )
            .bind(item.batch_id)
            .execute(&mut *tx)
            .await?;

            self.record_event_in_tx(
                &mut tx,
                intake_event_family::INTAKE_ITEM_CLASSIFIED,
                "atelier_intake_item",
                &item.item_id.to_string(),
                serde_json::json!({
                    "batch_id": item.batch_id,
                    "lane": item.lane,
                    "reason": item.lane_reason,
                    "source_path_ref": event_ref_for_text(&item.source_path),
                    "asset_id": asset_id,
                    "collection_id": collection_id,
                    "apply_workflow": true,
                }),
            )
            .await?;

            if let Some((audit, inserted)) = audit {
                if inserted {
                    self.record_event_in_tx(
                        &mut tx,
                        intake_event_family::INTAKE_ITEM_REJECTION_AUDITED,
                        "atelier_intake_item",
                        &item.item_id.to_string(),
                        serde_json::json!({
                            "audit_id": audit.audit_id,
                            "batch_id": audit.batch_id,
                            "lane": audit.lane,
                            "reason_ref": event_ref_for_text(&audit.reason),
                            "source_path_ref": audit.source_path_ref,
                        }),
                    )
                    .await?;
                }
            }
        }

        tx.commit().await?;
        Ok(IntakeClassificationApplyResult {
            item,
            asset_id,
            collection_id,
            collection_inserted,
        })
    }

    /// Per-lane counts for the sorter header.
    pub async fn intake_lane_counts(&self, batch_id: Uuid) -> AtelierResult<IntakeLaneCounts> {
        let rows = sqlx::query(
            r#"SELECT lane, COUNT(*) AS n
               FROM atelier_intake_item
               WHERE batch_id = $1
               GROUP BY lane"#,
        )
        .bind(batch_id)
        .fetch_all(self.pool())
        .await?;

        let mut counts = IntakeLaneCounts::default();
        for row in &rows {
            let lane: String = row.get("lane");
            let n: i64 = row.get("n");
            match IntakeLane::parse(&lane)? {
                IntakeLane::Pending => counts.pending = n,
                IntakeLane::Accepted => counts.accepted = n,
                IntakeLane::Rejected => counts.rejected = n,
                IntakeLane::Deferred => counts.deferred = n,
                IntakeLane::Skipped => counts.skipped = n,
                IntakeLane::Failed => counts.failed = n,
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

        let row = sqlx::query(
            r#"UPDATE atelier_intake_batch
               SET status = 'closed', updated_at_utc = NOW()
               WHERE batch_id = $1
               RETURNING batch_id, idempotency_key, source_label, source_ref,
                         mode, profile_mode, character_internal_id,
                         target_character_id, target_sheet_version_id,
                         target_collection_id, status, resume_cursor,
                         resumed_at_utc, created_at_utc, updated_at_utc"#,
        )
        .bind(batch_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake batch {batch_id}")))?;
        let batch = batch_from_row(&row);

        self.record_event(
            intake_event_family::INTAKE_BATCH_CLOSED,
            "atelier_intake_batch",
            &batch.batch_id.to_string(),
            serde_json::json!({
                "batch_id": batch.batch_id,
                "accepted": counts.accepted,
                "rejected": counts.rejected,
                "deferred": counts.deferred,
                "skipped": counts.skipped,
                "failed": counts.failed,
            }),
        )
        .await?;
        Ok(batch)
    }
}
