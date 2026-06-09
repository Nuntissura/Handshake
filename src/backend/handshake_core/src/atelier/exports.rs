//! Character-sheet exports + share packs (MT-199, legacy source fold-in).
//!
//! Translates legacy source `library.js` export surface
//! (`exportBundle`, `exportSharePack`, `_createSheetVersion` export columns,
//! and the web-portfolio manifest model) into the Handshake atelier domain.
//!
//! legacy source source: app/backend/library.js (`exportBundle` ~L6530, `exportSharePack`
//! ~L6603, `_createSheetVersion` export_format/export_relative_path ~L5592),
//! app/backend/backup.js (manifest intent). legacy source stores exports as files on disk
//! under `exports/`; Handshake instead records each export as a durable
//! request -> result -> manifest-entry graph in PostgreSQL, where rendered bytes
//! live in the ArtifactStore (`artifact_ref`) and are never written to random
//! filesystem paths or `.GOV`. SQLite is forbidden (MT-004).
//!
//! Data contract (MT-199):
//!   * `atelier_export_request`  - an operator/model ask to export a character,
//!     pinned to an immutable append-only sheet version (reuses MT-012) and a
//!     requested format. This is the unit operators retry / audit.
//!   * `atelier_export_result`   - the rendered output for a request: the
//!     ArtifactStore ref + content hash + byte length of the produced file.
//!   * `atelier_export_manifest_entry` - the share-pack manifest: the ordered
//!     set of items (sheet, media assets) bundled under one export, mirroring
//!     legacy source `manifest.json`. Reuses `atelier_media_asset` for image entries.
//!
//! Events (MT-005): every mutation emits a new atelier event family defined in
//! this module so MT-005 coverage extends to the export seam.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{cmp::Ordering, collections::HashSet};
use uuid::Uuid;

use super::{
    event_ref_for_text, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};

/// Atelier export event families (MT-199). Defined here, surfaced to MT-005
/// coverage by the parent appending these to `event_family::ALL`.
pub mod export_event_family {
    /// An export was requested for a character + pinned sheet version.
    pub const EXPORT_REQUESTED: &str = "atelier.export.requested";
    /// A requested export was rendered into an ArtifactStore-backed result.
    pub const EXPORT_RENDERED: &str = "atelier.export.rendered";
    /// An item (sheet/media) was attached to a share-pack manifest.
    pub const EXPORT_MANIFEST_ITEM_ADDED: &str = "atelier.export.manifest_item_added";
    /// An intake item/batch target link was attached to an export.
    pub const EXPORT_INTAKE_LINK_ATTACHED: &str = "atelier.export.intake_link_attached";
    /// A PNG/JPG contact-sheet raster export was explicitly planned/deferred.
    pub const CONTACT_SHEET_RASTER_EXPORT_PLANNED: &str =
        "atelier.export.contact_sheet_raster_planned";
    /// A web portfolio export was requested for a source collection.
    pub const WEB_PORTFOLIO_EXPORT_REQUESTED: &str = "atelier.export.web_portfolio_requested";
    /// A web portfolio export was rendered into an ArtifactStore-backed manifest.
    pub const WEB_PORTFOLIO_EXPORT_RENDERED: &str = "atelier.export.web_portfolio_rendered";
    /// A backup manifest was recorded with version/checksum traceability.
    pub const BACKUP_MANIFEST_RECORDED: &str = "atelier.export.backup_manifest_recorded";
    /// A backup restore preflight accepted or refused a manifest.
    pub const BACKUP_RESTORE_PREFLIGHT_RECORDED: &str =
        "atelier.export.backup_restore_preflight_recorded";

    /// All export event families (parity/coverage helper, mirrors
    /// `event_family::ALL` shape).
    pub const ALL: &[&str] = &[
        EXPORT_REQUESTED,
        EXPORT_RENDERED,
        EXPORT_MANIFEST_ITEM_ADDED,
        EXPORT_INTAKE_LINK_ATTACHED,
        CONTACT_SHEET_RASTER_EXPORT_PLANNED,
        WEB_PORTFOLIO_EXPORT_REQUESTED,
        WEB_PORTFOLIO_EXPORT_RENDERED,
        BACKUP_MANIFEST_RECORDED,
        BACKUP_RESTORE_PREFLIGHT_RECORDED,
    ];
}

/// Re-export at module root so callers can write `exports::EXPORT_REQUESTED`.
pub use export_event_family::{
    BACKUP_MANIFEST_RECORDED, BACKUP_RESTORE_PREFLIGHT_RECORDED,
    CONTACT_SHEET_RASTER_EXPORT_PLANNED, EXPORT_INTAKE_LINK_ATTACHED, EXPORT_MANIFEST_ITEM_ADDED,
    EXPORT_RENDERED, EXPORT_REQUESTED, WEB_PORTFOLIO_EXPORT_RENDERED,
    WEB_PORTFOLIO_EXPORT_REQUESTED,
};

const CONTACT_SHEET_RASTER_EXPORT_DEFERRED_REASON: &str = "deferred: PNG/JPG contact sheet raster export is planned; no raster renderer is implemented and no output artifact was produced";
pub const WEB_PORTFOLIO_MANIFEST_SCHEMA_ID: &str = "hsk.atelier.web_portfolio_export_manifest@1";
pub const BACKUP_MANIFEST_SCHEMA_ID: &str = "hsk.atelier.backup_manifest@1";
pub const LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID: &str = "hsk.atelier.llm_evidence_pack_manifest@1";

/// Output format for a character-sheet export (MT-199).
///
/// Mirrors legacy source `exportBundle` (txt/md/pdf) plus a structured JSON form. Stored
/// as a stable lowercase token; PDF rendering itself happens out-of-band (legacy source
/// renders PDF in the Electron main process) -- the request/result graph is
/// format-agnostic and only pins which format was asked for and produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    PlainText,
    Markdown,
    Json,
    Pdf,
}

impl ExportFormat {
    /// Stable DB token (also the legacy source `export_format` value where it overlaps).
    pub fn as_token(self) -> &'static str {
        match self {
            ExportFormat::PlainText => "txt",
            ExportFormat::Markdown => "md",
            ExportFormat::Json => "json",
            ExportFormat::Pdf => "pdf",
        }
    }

    /// Parse a stored token back into a format. Unknown tokens are a validation
    /// error rather than a silent default, so a corrupt row never masquerades
    /// as plain text.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "txt" => Ok(ExportFormat::PlainText),
            "md" => Ok(ExportFormat::Markdown),
            "json" => Ok(ExportFormat::Json),
            "pdf" => Ok(ExportFormat::Pdf),
            other => Err(AtelierError::Validation(format!(
                "unknown export format token: {other}"
            ))),
        }
    }
}

/// Lifecycle status of an export request (MT-199).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    /// Requested but not yet rendered.
    Pending,
    /// A result row was recorded for this request.
    Rendered,
}

impl ExportStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            ExportStatus::Pending => "pending",
            ExportStatus::Rendered => "rendered",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "pending" => Ok(ExportStatus::Pending),
            "rendered" => Ok(ExportStatus::Rendered),
            other => Err(AtelierError::Validation(format!(
                "unknown export status token: {other}"
            ))),
        }
    }
}

/// Planned raster output format for contact-sheet exports (MT-037).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactSheetRasterExportFormat {
    Png,
    Jpg,
}

impl ContactSheetRasterExportFormat {
    pub fn as_token(self) -> &'static str {
        match self {
            ContactSheetRasterExportFormat::Png => "png",
            ContactSheetRasterExportFormat::Jpg => "jpg",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "png" => Ok(ContactSheetRasterExportFormat::Png),
            "jpg" | "jpeg" => Ok(ContactSheetRasterExportFormat::Jpg),
            other => Err(AtelierError::Validation(format!(
                "unknown contact sheet raster export format token: {other}"
            ))),
        }
    }
}

/// The only valid lifecycle for MT-037: raster export is known and planned, but
/// no PNG/JPG bytes are produced by this module.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactSheetRasterExportStatus {
    Planned,
}

impl ContactSheetRasterExportStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            ContactSheetRasterExportStatus::Planned => "planned",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "planned" => Ok(ContactSheetRasterExportStatus::Planned),
            other => Err(AtelierError::Validation(format!(
                "unknown contact sheet raster export status token: {other}"
            ))),
        }
    }
}

/// Durable marker that PNG/JPG contact-sheet export is an explicit planned
/// capability. There is intentionally no artifact ref, content hash, or byte
/// length here; those belong only to real rendered output.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactSheetRasterExportPlan {
    pub plan_id: Uuid,
    pub sheet_id: Uuid,
    pub format: ContactSheetRasterExportFormat,
    pub status: ContactSheetRasterExportStatus,
    pub reason: String,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// A durable export request pinned to an immutable sheet version (MT-199).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportRequest {
    pub export_id: Uuid,
    pub character_internal_id: Uuid,
    /// Pinned append-only sheet version (MT-012); the export reflects exactly
    /// this snapshot even if the sheet is edited later.
    pub sheet_version_id: Uuid,
    pub format: ExportFormat,
    pub status: ExportStatus,
    /// Operator-facing label for the share pack / bundle (e.g. legacy source setName).
    pub label: Option<String>,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Parameters to open a new export request.
#[derive(Clone, Debug)]
pub struct NewExportRequest {
    pub character_internal_id: Uuid,
    pub sheet_version_id: Uuid,
    pub format: ExportFormat,
    pub label: Option<String>,
    pub requested_by: String,
}

/// The rendered artifact for an export request (MT-199).
///
/// Bytes live in the ArtifactStore behind `artifact_ref`; `content_hash` lets a
/// share pack be deduped/verified the same way legacy source stored `sheet_bytes_hash`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportResult {
    pub result_id: Uuid,
    pub export_id: Uuid,
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub created_at_utc: DateTime<Utc>,
}

/// Kind of item bundled into a share-pack manifest (MT-199).
///
/// Mirrors legacy source `manifest.json` sections (`includeSheet`, `images`, `docs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestItemKind {
    /// The exported character sheet itself.
    Sheet,
    /// A media asset (image) from the DAM (MT-015).
    Media,
    /// Generated README/usage instructions for a portable share pack.
    UsageReadme,
}

impl ManifestItemKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ManifestItemKind::Sheet => "sheet",
            ManifestItemKind::Media => "media",
            ManifestItemKind::UsageReadme => "usage_readme",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "sheet" => Ok(ManifestItemKind::Sheet),
            "media" => Ok(ManifestItemKind::Media),
            "usage_readme" => Ok(ManifestItemKind::UsageReadme),
            other => Err(AtelierError::Validation(format!(
                "unknown manifest item kind token: {other}"
            ))),
        }
    }
}

/// One ordered entry in a share-pack manifest (MT-199).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEntry {
    pub entry_id: Uuid,
    pub export_id: Uuid,
    pub seq: i64,
    pub kind: ManifestItemKind,
    /// ArtifactStore ref for the bundled bytes (sheet result or media asset).
    pub artifact_ref: String,
    /// Stable relative path inside the share pack (mirrors legacy source manifest paths,
    /// e.g. `sheet/character.txt`, `images/<id>__name.png`).
    pub pack_path: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharePackSubsetSelector {
    pub include_sheet: bool,
    pub media_asset_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharePackUsageReadmeArtifact {
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
}

#[derive(Clone, Debug)]
pub struct SharePackBuildRequest {
    pub export_id: Uuid,
    pub selector: SharePackSubsetSelector,
    pub usage_readme: SharePackUsageReadmeArtifact,
    pub requested_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharePackBuildResult {
    pub export_id: Uuid,
    pub entries: Vec<ManifestEntry>,
    pub selected_media_count: i64,
}

/// Required file roles in a deterministic model-consumable evidence pack (MT-072).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmEvidencePackFileKind {
    Readme,
    Evidence,
    RedactionReport,
    SourceIndex,
}

impl LlmEvidencePackFileKind {
    pub fn as_token(self) -> &'static str {
        match self {
            LlmEvidencePackFileKind::Readme => "readme",
            LlmEvidencePackFileKind::Evidence => "evidence",
            LlmEvidencePackFileKind::RedactionReport => "redaction_report",
            LlmEvidencePackFileKind::SourceIndex => "source_index",
        }
    }
}

/// Source anchor carried by every file in an LLM evidence pack.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmEvidenceSourceAnchor {
    pub source_id: String,
    pub source_path: String,
    pub source_range: String,
    pub content_hash: String,
}

/// One file in the MT-072 evidence-pack manifest.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmEvidencePackFile {
    pub kind: LlmEvidencePackFileKind,
    pub pack_path: String,
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub source_anchors: Vec<LlmEvidenceSourceAnchor>,
    pub redaction_required: bool,
    pub redacted: bool,
}

/// Strict, deterministic evidence-pack manifest for no-context model ingestion.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmEvidencePackManifest {
    pub schema_id: String,
    pub pack_id: Uuid,
    pub requested_by: String,
    pub files: Vec<LlmEvidencePackFile>,
}

/// A collection-backed web portfolio export request (MT-048).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebPortfolioExportRequest {
    pub portfolio_export_id: Uuid,
    pub source_collection_id: Uuid,
    pub slug: String,
    pub title: String,
    pub status: ExportStatus,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Parameters to open a portable web portfolio export request.
#[derive(Clone, Debug)]
pub struct NewWebPortfolioExportRequest {
    pub source_collection_id: Uuid,
    pub slug: String,
    pub title: String,
    pub requested_by: String,
}

/// One ArtifactStore-backed item in the web portfolio manifest.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebPortfolioManifestItem {
    pub asset_id: Uuid,
    pub artifact_ref: String,
    pub pack_path: String,
    pub content_hash: String,
    pub byte_len: i64,
}

/// Rendered web portfolio output and its machine-readable manifest (MT-048).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebPortfolioExportResult {
    pub result_id: Uuid,
    pub portfolio_export_id: Uuid,
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub manifest_json: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
}

/// One file/checksum row inside a backup manifest (MT-049).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifestFile {
    pub logical_path: String,
    pub content_hash: String,
    pub byte_len: i64,
}

/// Parameters to record a backup manifest.
#[derive(Clone, Debug)]
pub struct NewBackupManifest {
    pub app_version: String,
    pub spec_version: String,
    pub schema_version: i32,
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub files: Vec<BackupManifestFile>,
    pub created_by: String,
}

/// Durable backup manifest record with version and checksum traceability.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifestRecord {
    pub backup_id: Uuid,
    pub app_version: String,
    pub spec_version: String,
    pub schema_version: i32,
    pub artifact_ref: String,
    pub content_hash: String,
    pub byte_len: i64,
    pub manifest_hash: String,
    pub manifest_json: serde_json::Value,
    pub created_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Restore preflight verdict for a backup manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupRestorePreflightStatus {
    Accepted,
    Refused,
}

impl BackupRestorePreflightStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            BackupRestorePreflightStatus::Accepted => "accepted",
            BackupRestorePreflightStatus::Refused => "refused",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "accepted" => Ok(BackupRestorePreflightStatus::Accepted),
            "refused" => Ok(BackupRestorePreflightStatus::Refused),
            other => Err(AtelierError::Validation(format!(
                "unknown backup restore preflight status token: {other}"
            ))),
        }
    }
}

/// Parameters for restore compatibility preflight.
#[derive(Clone, Debug)]
pub struct BackupRestorePreflightRequest {
    pub backup_id: Uuid,
    pub current_app_version: String,
    pub current_spec_version: String,
    pub current_schema_version: i32,
    pub requested_by: String,
}

/// Recorded restore preflight decision.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupRestorePreflight {
    pub preflight_id: Uuid,
    pub backup_id: Uuid,
    pub current_app_version: String,
    pub current_spec_version: String,
    pub current_schema_version: i32,
    pub status: BackupRestorePreflightStatus,
    pub refusal_reason: Option<String>,
    pub requested_by: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Link between an export and an intake item target context (MT-032).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportIntakeLink {
    pub link_id: Uuid,
    pub export_id: Uuid,
    pub batch_id: Uuid,
    pub item_id: Uuid,
    pub target_character_id: Option<Uuid>,
    pub target_sheet_version_id: Option<Uuid>,
    pub target_collection_id: Option<Uuid>,
    pub version_agnostic: bool,
    pub created_at_utc: DateTime<Utc>,
}

fn request_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ExportRequest> {
    let format_token: String = row.get("format");
    let status_token: String = row.get("status");
    Ok(ExportRequest {
        export_id: row.get("export_id"),
        character_internal_id: row.get("character_internal_id"),
        sheet_version_id: row.get("sheet_version_id"),
        format: ExportFormat::from_token(&format_token)?,
        status: ExportStatus::from_token(&status_token)?,
        label: row.get("label"),
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn result_from_row(row: &sqlx::postgres::PgRow) -> ExportResult {
    ExportResult {
        result_id: row.get("result_id"),
        export_id: row.get("export_id"),
        artifact_ref: row.get("artifact_ref"),
        content_hash: row.get("content_hash"),
        byte_len: row.get("byte_len"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn entry_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ManifestEntry> {
    let kind_token: String = row.get("kind");
    Ok(ManifestEntry {
        entry_id: row.get("entry_id"),
        export_id: row.get("export_id"),
        seq: row.get("seq"),
        kind: ManifestItemKind::from_token(&kind_token)?,
        artifact_ref: row.get("artifact_ref"),
        pack_path: row.get("pack_path"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn web_portfolio_request_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<WebPortfolioExportRequest> {
    let status_token: String = row.get("status");
    Ok(WebPortfolioExportRequest {
        portfolio_export_id: row.get("portfolio_export_id"),
        source_collection_id: row.get("source_collection_id"),
        slug: row.get("slug"),
        title: row.get("title"),
        status: ExportStatus::from_token(&status_token)?,
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn web_portfolio_result_from_row(row: &sqlx::postgres::PgRow) -> WebPortfolioExportResult {
    WebPortfolioExportResult {
        result_id: row.get("result_id"),
        portfolio_export_id: row.get("portfolio_export_id"),
        artifact_ref: row.get("artifact_ref"),
        content_hash: row.get("content_hash"),
        byte_len: row.get("byte_len"),
        manifest_json: row.get("manifest_json"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn backup_manifest_from_row(row: &sqlx::postgres::PgRow) -> BackupManifestRecord {
    BackupManifestRecord {
        backup_id: row.get("backup_id"),
        app_version: row.get("app_version"),
        spec_version: row.get("spec_version"),
        schema_version: row.get("schema_version"),
        artifact_ref: row.get("artifact_ref"),
        content_hash: row.get("content_hash"),
        byte_len: row.get("byte_len"),
        manifest_hash: row.get("manifest_hash"),
        manifest_json: row.get("manifest_json"),
        created_by: row.get("created_by"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn backup_preflight_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<BackupRestorePreflight> {
    let status_token: String = row.get("status");
    Ok(BackupRestorePreflight {
        preflight_id: row.get("preflight_id"),
        backup_id: row.get("backup_id"),
        current_app_version: row.get("current_app_version"),
        current_spec_version: row.get("current_spec_version"),
        current_schema_version: row.get("current_schema_version"),
        status: BackupRestorePreflightStatus::from_token(&status_token)?,
        refusal_reason: row.get("refusal_reason"),
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn normalize_web_portfolio_slug(raw: &str) -> AtelierResult<String> {
    let slug = raw.trim();
    if slug.is_empty() {
        return Err(AtelierError::Validation("slug must not be empty".into()));
    }
    if slug.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(
            "slug must use no-space portable naming".into(),
        ));
    }
    if !slug.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
    }) {
        return Err(AtelierError::Validation(
            "slug must contain only lowercase ASCII letters, digits, hyphens, or underscores"
                .into(),
        ));
    }
    if !slug
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(AtelierError::Validation(
            "slug must start with a lowercase ASCII letter or digit".into(),
        ));
    }
    reject_legacy_runtime_ref("slug", slug)?;
    Ok(slug.to_string())
}

fn trimmed_nonempty(field: &str, value: &str) -> AtelierResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed != value {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty or padded"
        )));
    }
    Ok(trimmed.to_string())
}

fn validate_content_hash(field: &str, value: &str) -> AtelierResult<()> {
    trimmed_nonempty(field, value)?;
    if value.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(format!(
            "{field} must not contain blank spaces"
        )));
    }
    Ok(())
}

fn validate_positive_byte_len(field: &str, byte_len: i64) -> AtelierResult<()> {
    if byte_len <= 0 {
        return Err(AtelierError::Validation(format!(
            "{field} must be positive"
        )));
    }
    Ok(())
}

fn validate_artifact_store_ref(field: &str, artifact_ref: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref(field, artifact_ref)?;
    if !artifact_ref.starts_with("artifact://") {
        return Err(AtelierError::Validation(format!(
            "{field} must be an ArtifactStore artifact:// ref"
        )));
    }
    if artifact_ref.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(format!(
            "{field} must use no-space portable naming"
        )));
    }
    Ok(())
}

fn validate_web_portfolio_pack_path(pack_path: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref("pack_path", pack_path)?;
    if pack_path.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(
            "pack_path must use no-space portable naming".into(),
        ));
    }
    if pack_path.contains('\\') {
        return Err(AtelierError::Validation(
            "pack_path must use portable forward slashes".into(),
        ));
    }
    if pack_path.contains("//") {
        return Err(AtelierError::Validation(
            "pack_path must not contain empty path segments".into(),
        ));
    }
    Ok(())
}

fn media_pack_path(asset_id: Uuid, mime: &str) -> String {
    let extension = match mime {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };
    format!("images/{asset_id}.{extension}")
}

fn sheet_pack_path(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::PlainText => "sheet/character.txt",
        ExportFormat::Markdown => "sheet/character.md",
        ExportFormat::Json => "sheet/character.json",
        ExportFormat::Pdf => "sheet/character.pdf",
    }
}

fn llm_evidence_required_pack_path(kind: LlmEvidencePackFileKind) -> &'static str {
    match kind {
        LlmEvidencePackFileKind::Readme => "README.md",
        LlmEvidencePackFileKind::Evidence => "evidence.json",
        LlmEvidencePackFileKind::RedactionReport => "redactions.json",
        LlmEvidencePackFileKind::SourceIndex => "source-index.json",
    }
}

fn llm_evidence_file_order(kind: LlmEvidencePackFileKind) -> usize {
    match kind {
        LlmEvidencePackFileKind::Readme => 0,
        LlmEvidencePackFileKind::Evidence => 1,
        LlmEvidencePackFileKind::RedactionReport => 2,
        LlmEvidencePackFileKind::SourceIndex => 3,
    }
}

fn validate_llm_evidence_anchor(anchor: &LlmEvidenceSourceAnchor) -> AtelierResult<()> {
    let source_id = trimmed_nonempty("source_anchor.source_id", &anchor.source_id)?;
    let source_path = trimmed_nonempty("source_anchor.source_path", &anchor.source_path)?;
    let source_range = trimmed_nonempty("source_anchor.source_range", &anchor.source_range)?;
    reject_legacy_runtime_ref("source_anchor.source_id", &source_id)?;
    validate_web_portfolio_pack_path(&source_path)?;
    reject_legacy_runtime_ref("source_anchor.source_range", &source_range)?;
    validate_content_hash("source_anchor.content_hash", &anchor.content_hash)?;
    Ok(())
}

fn validate_llm_evidence_file(file: &LlmEvidencePackFile) -> AtelierResult<()> {
    let expected_path = llm_evidence_required_pack_path(file.kind);
    if file.pack_path != expected_path {
        return Err(AtelierError::Validation(format!(
            "{} file must use pack_path {expected_path}",
            file.kind.as_token()
        )));
    }
    validate_web_portfolio_pack_path(&file.pack_path)?;
    validate_artifact_store_ref("artifact_ref", &file.artifact_ref)?;
    validate_content_hash("content_hash", &file.content_hash)?;
    validate_positive_byte_len("byte_len", file.byte_len)?;
    if file.source_anchors.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{} must include source_anchors",
            file.pack_path
        )));
    }
    for anchor in &file.source_anchors {
        validate_llm_evidence_anchor(anchor)?;
    }
    if file.redaction_required && !file.redacted {
        return Err(AtelierError::Validation(format!(
            "{} is redaction_required but not marked redacted",
            file.pack_path
        )));
    }
    Ok(())
}

pub fn validate_llm_evidence_pack_manifest(
    manifest: &LlmEvidencePackManifest,
) -> AtelierResult<()> {
    if manifest.schema_id != LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID {
        return Err(AtelierError::Validation(format!(
            "schema_id must be {LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID}"
        )));
    }
    if manifest.pack_id.is_nil() {
        return Err(AtelierError::Validation("pack_id must not be nil".into()));
    }
    let requested_by = trimmed_nonempty("requested_by", &manifest.requested_by)?;
    reject_legacy_runtime_ref("requested_by", &requested_by)?;

    let mut seen_kinds = HashSet::new();
    let mut seen_paths = HashSet::new();
    let mut expected_order = Vec::with_capacity(manifest.files.len());
    for file in &manifest.files {
        validate_llm_evidence_file(file)?;
        if !seen_kinds.insert(file.kind) {
            return Err(AtelierError::Validation(format!(
                "duplicate evidence-pack file kind {}",
                file.kind.as_token()
            )));
        }
        if !seen_paths.insert(file.pack_path.as_str()) {
            return Err(AtelierError::Validation(format!(
                "duplicate evidence-pack pack_path {}",
                file.pack_path
            )));
        }
        expected_order.push(llm_evidence_file_order(file.kind));
    }

    for kind in [
        LlmEvidencePackFileKind::Readme,
        LlmEvidencePackFileKind::Evidence,
        LlmEvidencePackFileKind::RedactionReport,
        LlmEvidencePackFileKind::SourceIndex,
    ] {
        let path = llm_evidence_required_pack_path(kind);
        if !seen_paths.contains(path) {
            return Err(AtelierError::Validation(format!(
                "LLM evidence pack missing required file {path}"
            )));
        }
    }

    let mut sorted_order = expected_order.clone();
    sorted_order.sort_unstable();
    if expected_order != sorted_order {
        return Err(AtelierError::Validation(
            "LLM evidence pack files must be in deterministic order".into(),
        ));
    }
    Ok(())
}

pub fn build_llm_evidence_pack_manifest(
    pack_id: Uuid,
    requested_by: String,
    mut files: Vec<LlmEvidencePackFile>,
) -> AtelierResult<LlmEvidencePackManifest> {
    files.sort_by(|left, right| {
        llm_evidence_file_order(left.kind)
            .cmp(&llm_evidence_file_order(right.kind))
            .then_with(|| left.pack_path.cmp(&right.pack_path))
    });
    let manifest = LlmEvidencePackManifest {
        schema_id: LLM_EVIDENCE_PACK_MANIFEST_SCHEMA_ID.to_string(),
        pack_id,
        requested_by,
        files,
    };
    validate_llm_evidence_pack_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_web_portfolio_manifest_items(items: &[WebPortfolioManifestItem]) -> AtelierResult<()> {
    if items.is_empty() {
        return Err(AtelierError::Validation(
            "web portfolio manifest must include at least one item".into(),
        ));
    }

    let mut seen_assets = HashSet::new();
    let mut seen_paths = HashSet::new();
    for item in items {
        if !seen_assets.insert(item.asset_id) {
            return Err(AtelierError::Validation(format!(
                "duplicate web portfolio asset_id {}",
                item.asset_id
            )));
        }
        if !seen_paths.insert(item.pack_path.as_str()) {
            return Err(AtelierError::Validation(format!(
                "duplicate web portfolio pack_path {}",
                item.pack_path
            )));
        }
        validate_artifact_store_ref("artifact_ref", &item.artifact_ref)?;
        validate_web_portfolio_pack_path(&item.pack_path)?;
        validate_content_hash("content_hash", &item.content_hash)?;
        validate_positive_byte_len("byte_len", item.byte_len)?;
    }
    Ok(())
}

fn web_portfolio_manifest_json(
    request: &WebPortfolioExportRequest,
    artifact_ref: &str,
    content_hash: &str,
    byte_len: i64,
    items: &[WebPortfolioManifestItem],
) -> serde_json::Value {
    serde_json::json!({
        "schema_id": WEB_PORTFOLIO_MANIFEST_SCHEMA_ID,
        "portfolio_export_id": request.portfolio_export_id,
        "source_collection_id": request.source_collection_id,
        "slug": request.slug,
        "title": request.title,
        "output": {
            "artifact_ref": artifact_ref,
            "content_hash": content_hash,
            "byte_len": byte_len,
        },
        "items": items
            .iter()
            .map(|item| {
                serde_json::json!({
                    "asset_id": item.asset_id,
                    "artifact_ref": item.artifact_ref,
                    "pack_path": item.pack_path,
                    "content_hash": item.content_hash,
                    "byte_len": item.byte_len,
                })
            })
            .collect::<Vec<_>>(),
    })
}

fn validate_backup_version_token(field: &str, value: &str) -> AtelierResult<String> {
    let token = trimmed_nonempty(field, value)?;
    if token.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(format!(
            "{field} must not contain blank spaces"
        )));
    }
    Ok(token)
}

fn validate_backup_schema_version(schema_version: i32) -> AtelierResult<()> {
    if schema_version <= 0 {
        return Err(AtelierError::Validation(
            "schema_version must be positive".into(),
        ));
    }
    Ok(())
}

fn validate_backup_logical_path(path: &str) -> AtelierResult<()> {
    reject_legacy_runtime_ref("logical_path", path)?;
    if path.chars().any(char::is_whitespace) {
        return Err(AtelierError::Validation(
            "logical_path must use no-space portable naming".into(),
        ));
    }
    if path.contains('\\') || path.contains("//") {
        return Err(AtelierError::Validation(
            "logical_path must use portable relative path segments".into(),
        ));
    }
    Ok(())
}

fn validate_backup_manifest_files(files: &[BackupManifestFile]) -> AtelierResult<()> {
    if files.is_empty() {
        return Err(AtelierError::Validation(
            "backup manifest must include at least one file".into(),
        ));
    }
    let mut seen_paths = HashSet::new();
    for file in files {
        if !seen_paths.insert(file.logical_path.as_str()) {
            return Err(AtelierError::Validation(format!(
                "duplicate backup manifest logical_path {}",
                file.logical_path
            )));
        }
        validate_backup_logical_path(&file.logical_path)?;
        validate_content_hash("content_hash", &file.content_hash)?;
        validate_positive_byte_len("byte_len", file.byte_len)?;
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn backup_manifest_json(
    backup_id: Uuid,
    app_version: &str,
    spec_version: &str,
    schema_version: i32,
    artifact_ref: &str,
    content_hash: &str,
    byte_len: i64,
    files: &[BackupManifestFile],
    created_by: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema_id": BACKUP_MANIFEST_SCHEMA_ID,
        "backup_id": backup_id,
        "app_version": app_version,
        "spec_version": spec_version,
        "schema_version": schema_version,
        "artifact": {
            "artifact_ref": artifact_ref,
            "content_hash": content_hash,
            "byte_len": byte_len,
        },
        "files": files
            .iter()
            .map(|file| {
                serde_json::json!({
                    "logical_path": file.logical_path,
                    "content_hash": file.content_hash,
                    "byte_len": file.byte_len,
                })
            })
            .collect::<Vec<_>>(),
        "created_by": created_by,
    })
}

fn version_segments(version: &str) -> Option<Vec<u64>> {
    let mut segments = Vec::new();
    for segment in version.split(|ch: char| matches!(ch, '.' | '-' | '_')) {
        if segment.is_empty() {
            return None;
        }
        let numeric_prefix: String = segment
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if numeric_prefix.is_empty() {
            return None;
        }
        let parsed = numeric_prefix.parse::<u64>().ok()?;
        segments.push(parsed);
    }
    Some(segments)
}

fn compare_version_tokens(left: &str, right: &str) -> Ordering {
    match (version_segments(left), version_segments(right)) {
        (Some(left_segments), Some(right_segments)) => {
            let max_len = left_segments.len().max(right_segments.len());
            for index in 0..max_len {
                let left_value = left_segments.get(index).copied().unwrap_or(0);
                let right_value = right_segments.get(index).copied().unwrap_or(0);
                match left_value.cmp(&right_value) {
                    Ordering::Equal => {}
                    ordering => return ordering,
                }
            }
            Ordering::Equal
        }
        _ => left.cmp(right),
    }
}

fn backup_restore_refusal_reason(
    backup: &BackupManifestRecord,
    current_app_version: &str,
    current_spec_version: &str,
    current_schema_version: i32,
) -> Option<String> {
    if compare_version_tokens(&backup.app_version, current_app_version) == Ordering::Greater {
        return Some(format!(
            "newer app backup {} cannot be restored by current app {}",
            backup.app_version, current_app_version
        ));
    }
    if compare_version_tokens(&backup.spec_version, current_spec_version) == Ordering::Greater {
        return Some(format!(
            "newer spec backup {} cannot be restored by current spec {}",
            backup.spec_version, current_spec_version
        ));
    }
    if backup.schema_version > current_schema_version {
        return Some(format!(
            "newer schema backup {} cannot be restored by current schema {}",
            backup.schema_version, current_schema_version
        ));
    }
    None
}

fn intake_link_from_row(row: &sqlx::postgres::PgRow) -> ExportIntakeLink {
    ExportIntakeLink {
        link_id: row.get("link_id"),
        export_id: row.get("export_id"),
        batch_id: row.get("batch_id"),
        item_id: row.get("item_id"),
        target_character_id: row.get("target_character_id"),
        target_sheet_version_id: row.get("target_sheet_version_id"),
        target_collection_id: row.get("target_collection_id"),
        version_agnostic: row.get("version_agnostic"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn raster_export_plan_from_row(
    row: &sqlx::postgres::PgRow,
) -> AtelierResult<ContactSheetRasterExportPlan> {
    let format_token: String = row.get("format");
    let status_token: String = row.get("status");
    Ok(ContactSheetRasterExportPlan {
        plan_id: row.get("plan_id"),
        sheet_id: row.get("sheet_id"),
        format: ContactSheetRasterExportFormat::from_token(&format_token)?,
        status: ContactSheetRasterExportStatus::from_token(&status_token)?,
        reason: row.get("reason"),
        requested_by: row.get("requested_by"),
        created_at_utc: row.get("created_at_utc"),
    })
}

impl AtelierStore {
    /// Open an export request pinned to a specific sheet version (MT-199).
    ///
    /// The sheet version is validated to belong to the named character so an
    /// export can never silently bundle another character's snapshot. Reuses
    /// the append-only sheet history (MT-012): the export reflects exactly the
    /// pinned version even if the sheet is edited afterwards.
    pub async fn request_sheet_export(
        &self,
        new: &NewExportRequest,
    ) -> AtelierResult<ExportRequest> {
        if new.requested_by.trim().is_empty() {
            return Err(AtelierError::Validation(
                "requested_by must not be empty".into(),
            ));
        }

        // Pin validation: the sheet version must exist and belong to the
        // character being exported.
        let owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT character_internal_id FROM atelier_sheet_version WHERE version_id = $1",
        )
        .bind(new.sheet_version_id)
        .fetch_optional(self.pool())
        .await?;
        match owner {
            None => {
                return Err(AtelierError::NotFound(format!(
                    "sheet version_id={}",
                    new.sheet_version_id
                )));
            }
            Some(owner_id) if owner_id != new.character_internal_id => {
                return Err(AtelierError::Validation(format!(
                    "sheet version {} does not belong to character {}",
                    new.sheet_version_id, new.character_internal_id
                )));
            }
            Some(_) => {}
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_export_request
                 (character_internal_id, sheet_version_id, format, status, label, requested_by)
               VALUES ($1, $2, $3, 'pending', $4, $5)
               RETURNING export_id, character_internal_id, sheet_version_id, format,
                         status, label, requested_by, created_at_utc"#,
        )
        .bind(new.character_internal_id)
        .bind(new.sheet_version_id)
        .bind(new.format.as_token())
        .bind(&new.label)
        .bind(&new.requested_by)
        .fetch_one(self.pool())
        .await?;
        let request = request_from_row(&row)?;

        self.record_event(
            EXPORT_REQUESTED,
            "atelier_export_request",
            &request.export_id.to_string(),
            serde_json::json!({
                "sheet_version_id": request.sheet_version_id,
                "format": request.format.as_token(),
                "requested_by": request.requested_by,
            }),
        )
        .await?;
        Ok(request)
    }

    /// Fetch an export request by id.
    pub async fn get_export_request(&self, export_id: Uuid) -> AtelierResult<ExportRequest> {
        let row = sqlx::query(
            r#"SELECT export_id, character_internal_id, sheet_version_id, format,
                      status, label, requested_by, created_at_utc
               FROM atelier_export_request WHERE export_id = $1"#,
        )
        .bind(export_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("export_id={export_id}")))?;
        request_from_row(&row)
    }

    /// Record the rendered artifact for a pending export and flip it to
    /// `rendered` (MT-199).
    ///
    /// Idempotent on `(export_id, content_hash)`: re-recording identical bytes
    /// returns the existing result rather than duplicating it, mirroring the
    /// content-hash dedup used by the DAM (MT-015) and legacy source `sheet_bytes_hash`.
    /// Recording a result for an unknown request is a not-found error so a
    /// result can never dangle.
    pub async fn record_export_result(
        &self,
        export_id: Uuid,
        artifact_ref: &str,
        content_hash: &str,
        byte_len: i64,
    ) -> AtelierResult<ExportResult> {
        if artifact_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "artifact_ref must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_ref", artifact_ref)?;
        // Guard: the request must exist (also flips status below).
        let _ = self.get_export_request(export_id).await?;

        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_export_result
                 (export_id, artifact_ref, content_hash, byte_len)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (export_id, content_hash)
                 DO UPDATE SET artifact_ref = EXCLUDED.artifact_ref
               RETURNING result_id, export_id, artifact_ref, content_hash,
                         byte_len, created_at_utc"#,
        )
        .bind(export_id)
        .bind(artifact_ref)
        .bind(content_hash)
        .bind(byte_len)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("UPDATE atelier_export_request SET status = 'rendered' WHERE export_id = $1")
            .bind(export_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let result = result_from_row(&row);
        self.record_event(
            EXPORT_RENDERED,
            "atelier_export_request",
            &export_id.to_string(),
            serde_json::json!({
                "result_id": result.result_id,
                "content_hash": result.content_hash,
                "byte_len": result.byte_len,
            }),
        )
        .await?;
        Ok(result)
    }

    /// The rendered result for an export, if one has been recorded.
    pub async fn get_export_result(&self, export_id: Uuid) -> AtelierResult<Option<ExportResult>> {
        let row = sqlx::query(
            r#"SELECT result_id, export_id, artifact_ref, content_hash, byte_len,
                      created_at_utc
               FROM atelier_export_result WHERE export_id = $1
               ORDER BY created_at_utc DESC LIMIT 1"#,
        )
        .bind(export_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(result_from_row))
    }

    /// Attach an item to an export's share-pack manifest (MT-199).
    ///
    /// Computes the next manifest sequence inside a transaction so concurrent
    /// adds cannot collide on `(export_id, seq)`. Mirrors legacy source `exportSharePack`
    /// assembling `manifest.images` / `manifest.docs`; here every bundled item
    /// is an ArtifactStore ref plus a stable relative pack path.
    pub async fn add_manifest_entry(
        &self,
        export_id: Uuid,
        kind: ManifestItemKind,
        artifact_ref: &str,
        pack_path: &str,
    ) -> AtelierResult<ManifestEntry> {
        if pack_path.trim().is_empty() {
            return Err(AtelierError::Validation(
                "pack_path must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("artifact_ref", artifact_ref)?;
        validate_web_portfolio_pack_path(pack_path)?;
        // Guard: the export must exist.
        let _ = self.get_export_request(export_id).await?;

        let mut tx = self.pool().begin().await?;

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM atelier_export_manifest_entry WHERE export_id = $1",
        )
        .bind(export_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_export_manifest_entry
                 (export_id, seq, kind, artifact_ref, pack_path)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING entry_id, export_id, seq, kind, artifact_ref, pack_path,
                         created_at_utc"#,
        )
        .bind(export_id)
        .bind(next_seq)
        .bind(kind.as_token())
        .bind(artifact_ref)
        .bind(pack_path)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let entry = entry_from_row(&row)?;
        self.record_event(
            EXPORT_MANIFEST_ITEM_ADDED,
            "atelier_export_request",
            &export_id.to_string(),
            serde_json::json!({
                "entry_id": entry.entry_id,
                "seq": entry.seq,
                "kind": entry.kind.as_token(),
                "pack_path": entry.pack_path,
            }),
        )
        .await?;
        Ok(entry)
    }

    /// The ordered share-pack manifest for an export (ascending sequence),
    /// mirroring legacy source `manifest.json` item ordering.
    pub async fn export_manifest(&self, export_id: Uuid) -> AtelierResult<Vec<ManifestEntry>> {
        let rows = sqlx::query(
            r#"SELECT entry_id, export_id, seq, kind, artifact_ref, pack_path,
                      created_at_utc
               FROM atelier_export_manifest_entry
               WHERE export_id = $1
               ORDER BY seq ASC"#,
        )
        .bind(export_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(entry_from_row).collect()
    }

    pub async fn build_share_pack_manifest(
        &self,
        request: &SharePackBuildRequest,
    ) -> AtelierResult<SharePackBuildResult> {
        let requested_by = trimmed_nonempty("requested_by", &request.requested_by)?;
        reject_legacy_runtime_ref("requested_by", &requested_by)?;
        validate_artifact_store_ref(
            "usage_readme.artifact_ref",
            &request.usage_readme.artifact_ref,
        )?;
        validate_content_hash(
            "usage_readme.content_hash",
            &request.usage_readme.content_hash,
        )?;
        validate_positive_byte_len("usage_readme.byte_len", request.usage_readme.byte_len)?;

        if !request.selector.include_sheet && request.selector.media_asset_ids.is_empty() {
            return Err(AtelierError::Validation(
                "share-pack selector must include the sheet or at least one media asset".into(),
            ));
        }

        let export = self.get_export_request(request.export_id).await?;
        let sheet_result = if request.selector.include_sheet {
            Some(
                self.get_export_result(request.export_id)
                    .await?
                    .ok_or_else(|| {
                        AtelierError::Validation(
                            "share-pack selector includes sheet but no export result is recorded"
                                .into(),
                        )
                    })?,
            )
        } else {
            None
        };

        let mut seen_media = HashSet::new();
        let mut media_rows = Vec::new();
        for asset_id in &request.selector.media_asset_ids {
            if !seen_media.insert(*asset_id) {
                continue;
            }
            let row = sqlx::query(
                r#"SELECT asset_id, artifact_ref, content_hash, byte_len, mime
                   FROM atelier_media_asset
                   WHERE asset_id = $1"#,
            )
            .bind(asset_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("media asset_id={asset_id}")))?;
            let artifact_ref: String = row.get("artifact_ref");
            let content_hash: String = row.get("content_hash");
            let byte_len: i64 = row.get("byte_len");
            validate_artifact_store_ref("media.artifact_ref", &artifact_ref)?;
            validate_content_hash("media.content_hash", &content_hash)?;
            validate_positive_byte_len("media.byte_len", byte_len)?;
            media_rows.push(row);
        }

        let mut entries = Vec::new();
        if let Some(sheet_result) = sheet_result {
            entries.push(
                self.add_manifest_entry(
                    request.export_id,
                    ManifestItemKind::Sheet,
                    &sheet_result.artifact_ref,
                    sheet_pack_path(export.format),
                )
                .await?,
            );
        }
        for row in &media_rows {
            let asset_id: Uuid = row.get("asset_id");
            let artifact_ref: String = row.get("artifact_ref");
            let mime: String = row.get("mime");
            entries.push(
                self.add_manifest_entry(
                    request.export_id,
                    ManifestItemKind::Media,
                    &artifact_ref,
                    &media_pack_path(asset_id, &mime),
                )
                .await?,
            );
        }
        entries.push(
            self.add_manifest_entry(
                request.export_id,
                ManifestItemKind::UsageReadme,
                &request.usage_readme.artifact_ref,
                "README.md",
            )
            .await?,
        );

        Ok(SharePackBuildResult {
            export_id: request.export_id,
            entries,
            selected_media_count: media_rows.len() as i64,
        })
    }

    /// Open a collection-backed web portfolio export request (MT-048).
    ///
    /// This is deliberately separate from character-sheet exports: the source is
    /// a collection, the stable operator-facing name is a no-space slug, and the
    /// rendered output is recorded as a manifest JSON artifact.
    pub async fn request_web_portfolio_export(
        &self,
        new: &NewWebPortfolioExportRequest,
    ) -> AtelierResult<WebPortfolioExportRequest> {
        let slug = normalize_web_portfolio_slug(&new.slug)?;
        let title = trimmed_nonempty("title", &new.title)?;
        let requested_by = trimmed_nonempty("requested_by", &new.requested_by)?;
        let _ = self.get_collection(new.source_collection_id).await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_web_portfolio_export_request
                 (source_collection_id, slug, title, status, requested_by)
               VALUES ($1, $2, $3, 'pending', $4)
               RETURNING portfolio_export_id, source_collection_id, slug, title,
                         status, requested_by, created_at_utc, updated_at_utc"#,
        )
        .bind(new.source_collection_id)
        .bind(&slug)
        .bind(&title)
        .bind(&requested_by)
        .fetch_one(self.pool())
        .await?;
        let request = web_portfolio_request_from_row(&row)?;

        self.record_event(
            WEB_PORTFOLIO_EXPORT_REQUESTED,
            "atelier_web_portfolio_export_request",
            &request.portfolio_export_id.to_string(),
            serde_json::json!({
                "source_collection_id": request.source_collection_id,
                "slug": request.slug,
                "requested_by": request.requested_by,
            }),
        )
        .await?;
        Ok(request)
    }

    /// Fetch a web portfolio export request by id.
    pub async fn get_web_portfolio_export_request(
        &self,
        portfolio_export_id: Uuid,
    ) -> AtelierResult<WebPortfolioExportRequest> {
        let row = sqlx::query(
            r#"SELECT portfolio_export_id, source_collection_id, slug, title,
                      status, requested_by, created_at_utc, updated_at_utc
               FROM atelier_web_portfolio_export_request
               WHERE portfolio_export_id = $1"#,
        )
        .bind(portfolio_export_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("portfolio_export_id={portfolio_export_id}"))
        })?;
        web_portfolio_request_from_row(&row)
    }

    /// Record a rendered web portfolio manifest artifact and its manifest item
    /// contract (MT-048).
    pub async fn record_web_portfolio_export_result(
        &self,
        portfolio_export_id: Uuid,
        artifact_ref: &str,
        content_hash: &str,
        byte_len: i64,
        items: &[WebPortfolioManifestItem],
    ) -> AtelierResult<WebPortfolioExportResult> {
        validate_artifact_store_ref("artifact_ref", artifact_ref)?;
        validate_content_hash("content_hash", content_hash)?;
        validate_positive_byte_len("byte_len", byte_len)?;
        validate_web_portfolio_manifest_items(items)?;

        let request = self
            .get_web_portfolio_export_request(portfolio_export_id)
            .await?;
        for item in items {
            let row = sqlx::query(
                r#"SELECT ma.artifact_ref, ma.content_hash, ma.byte_len
                   FROM atelier_collection_item ci
                   JOIN atelier_media_asset ma
                     ON ma.asset_id = ci.asset_id
                   WHERE ci.collection_id = $1
                     AND ci.asset_id = $2"#,
            )
            .bind(request.source_collection_id)
            .bind(item.asset_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or_else(|| {
                AtelierError::Validation(format!(
                    "asset_id {} is not in web portfolio source collection {}",
                    item.asset_id, request.source_collection_id
                ))
            })?;
            let stored_artifact_ref: String = row.get("artifact_ref");
            let stored_content_hash: String = row.get("content_hash");
            let stored_byte_len: i64 = row.get("byte_len");
            if stored_artifact_ref != item.artifact_ref {
                return Err(AtelierError::Validation(format!(
                    "artifact_ref for asset_id {} does not match stored media asset",
                    item.asset_id
                )));
            }
            if stored_content_hash != item.content_hash {
                return Err(AtelierError::Validation(format!(
                    "content_hash for asset_id {} does not match stored media asset",
                    item.asset_id
                )));
            }
            if stored_byte_len != item.byte_len {
                return Err(AtelierError::Validation(format!(
                    "byte_len for asset_id {} does not match stored media asset",
                    item.asset_id
                )));
            }
        }

        let manifest_json =
            web_portfolio_manifest_json(&request, artifact_ref, content_hash, byte_len, items);
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_web_portfolio_export_result
                 (portfolio_export_id, artifact_ref, content_hash, byte_len, manifest_json)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (portfolio_export_id, content_hash)
                 DO UPDATE SET artifact_ref = EXCLUDED.artifact_ref,
                               byte_len = EXCLUDED.byte_len,
                               manifest_json = EXCLUDED.manifest_json
               RETURNING result_id, portfolio_export_id, artifact_ref, content_hash,
                         byte_len, manifest_json, created_at_utc"#,
        )
        .bind(portfolio_export_id)
        .bind(artifact_ref)
        .bind(content_hash)
        .bind(byte_len)
        .bind(&manifest_json)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"UPDATE atelier_web_portfolio_export_request
               SET status = 'rendered', updated_at_utc = NOW()
               WHERE portfolio_export_id = $1"#,
        )
        .bind(portfolio_export_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let result = web_portfolio_result_from_row(&row);
        self.record_event(
            WEB_PORTFOLIO_EXPORT_RENDERED,
            "atelier_web_portfolio_export_request",
            &portfolio_export_id.to_string(),
            serde_json::json!({
                "result_id": result.result_id,
                "content_hash": result.content_hash,
                "byte_len": result.byte_len,
                "item_count": items.len(),
            }),
        )
        .await?;
        Ok(result)
    }

    /// The most recent web portfolio result for a request, if one exists.
    pub async fn get_web_portfolio_export_result(
        &self,
        portfolio_export_id: Uuid,
    ) -> AtelierResult<Option<WebPortfolioExportResult>> {
        let row = sqlx::query(
            r#"SELECT result_id, portfolio_export_id, artifact_ref, content_hash,
                      byte_len, manifest_json, created_at_utc
               FROM atelier_web_portfolio_export_result
               WHERE portfolio_export_id = $1
               ORDER BY created_at_utc DESC, result_id DESC
               LIMIT 1"#,
        )
        .bind(portfolio_export_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(web_portfolio_result_from_row))
    }

    /// Record a backup manifest with app/spec/schema version traceability and
    /// checksums for the backup artifact plus logical files (MT-049).
    pub async fn record_backup_manifest(
        &self,
        new: &NewBackupManifest,
    ) -> AtelierResult<BackupManifestRecord> {
        let app_version = validate_backup_version_token("app_version", &new.app_version)?;
        let spec_version = validate_backup_version_token("spec_version", &new.spec_version)?;
        validate_backup_schema_version(new.schema_version)?;
        validate_artifact_store_ref("artifact_ref", &new.artifact_ref)?;
        validate_content_hash("content_hash", &new.content_hash)?;
        validate_positive_byte_len("byte_len", new.byte_len)?;
        validate_backup_manifest_files(&new.files)?;
        let created_by = trimmed_nonempty("created_by", &new.created_by)?;

        let backup_id = Uuid::new_v4();
        let manifest_json = backup_manifest_json(
            backup_id,
            &app_version,
            &spec_version,
            new.schema_version,
            &new.artifact_ref,
            &new.content_hash,
            new.byte_len,
            &new.files,
            &created_by,
        );
        let manifest_bytes = serde_json::to_vec(&manifest_json).map_err(|err| {
            AtelierError::Validation(format!("backup manifest could not be hashed: {err}"))
        })?;
        let manifest_hash = sha256_hex(&manifest_bytes);

        let row = sqlx::query(
            r#"INSERT INTO atelier_backup_manifest
                 (backup_id, app_version, spec_version, schema_version,
                  artifact_ref, content_hash, byte_len, manifest_hash,
                  manifest_json, created_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING backup_id, app_version, spec_version, schema_version,
                         artifact_ref, content_hash, byte_len, manifest_hash,
                         manifest_json, created_by, created_at_utc"#,
        )
        .bind(backup_id)
        .bind(&app_version)
        .bind(&spec_version)
        .bind(new.schema_version)
        .bind(&new.artifact_ref)
        .bind(&new.content_hash)
        .bind(new.byte_len)
        .bind(&manifest_hash)
        .bind(&manifest_json)
        .bind(&created_by)
        .fetch_one(self.pool())
        .await?;
        let backup = backup_manifest_from_row(&row);

        self.record_event(
            BACKUP_MANIFEST_RECORDED,
            "atelier_backup_manifest",
            &backup.backup_id.to_string(),
            serde_json::json!({
                "app_version": backup.app_version,
                "spec_version": backup.spec_version,
                "schema_version": backup.schema_version,
                "manifest_hash": backup.manifest_hash,
                "file_count": new.files.len(),
            }),
        )
        .await?;
        Ok(backup)
    }

    /// Fetch a recorded backup manifest.
    pub async fn get_backup_manifest(
        &self,
        backup_id: Uuid,
    ) -> AtelierResult<BackupManifestRecord> {
        let row = sqlx::query(
            r#"SELECT backup_id, app_version, spec_version, schema_version,
                      artifact_ref, content_hash, byte_len, manifest_hash,
                      manifest_json, created_by, created_at_utc
               FROM atelier_backup_manifest
               WHERE backup_id = $1"#,
        )
        .bind(backup_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("backup_id={backup_id}")))?;
        Ok(backup_manifest_from_row(&row))
    }

    /// Record restore compatibility preflight for a backup. A manifest created
    /// by a newer app/spec/schema is refused before any restore work can begin.
    pub async fn preflight_backup_restore(
        &self,
        request: &BackupRestorePreflightRequest,
    ) -> AtelierResult<BackupRestorePreflight> {
        let current_app_version =
            validate_backup_version_token("current_app_version", &request.current_app_version)?;
        let current_spec_version =
            validate_backup_version_token("current_spec_version", &request.current_spec_version)?;
        validate_backup_schema_version(request.current_schema_version)?;
        let requested_by = trimmed_nonempty("requested_by", &request.requested_by)?;
        let backup = self.get_backup_manifest(request.backup_id).await?;

        let refusal_reason = backup_restore_refusal_reason(
            &backup,
            &current_app_version,
            &current_spec_version,
            request.current_schema_version,
        );
        let status = if refusal_reason.is_some() {
            BackupRestorePreflightStatus::Refused
        } else {
            BackupRestorePreflightStatus::Accepted
        };

        let row = sqlx::query(
            r#"INSERT INTO atelier_backup_restore_preflight
                 (backup_id, current_app_version, current_spec_version,
                  current_schema_version, status, refusal_reason, requested_by)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING preflight_id, backup_id, current_app_version,
                         current_spec_version, current_schema_version, status,
                         refusal_reason, requested_by, created_at_utc"#,
        )
        .bind(request.backup_id)
        .bind(&current_app_version)
        .bind(&current_spec_version)
        .bind(request.current_schema_version)
        .bind(status.as_token())
        .bind(&refusal_reason)
        .bind(&requested_by)
        .fetch_one(self.pool())
        .await?;
        let preflight = backup_preflight_from_row(&row)?;

        self.record_event(
            BACKUP_RESTORE_PREFLIGHT_RECORDED,
            "atelier_backup_manifest",
            &backup.backup_id.to_string(),
            serde_json::json!({
                "preflight_id": preflight.preflight_id,
                "status": preflight.status.as_token(),
                "refusal_reason": preflight.refusal_reason,
                "current_app_version": preflight.current_app_version,
                "current_spec_version": preflight.current_spec_version,
                "current_schema_version": preflight.current_schema_version,
            }),
        )
        .await?;
        Ok(preflight)
    }

    /// Attach the target refs from an intake item/batch to an export (MT-032).
    ///
    /// The row is intentionally separate from the share-pack manifest: manifest
    /// entries describe bundled bytes, while this link preserves where an
    /// accepted intake item was intended to land. A missing
    /// `target_sheet_version_id` is stored as `version_agnostic = true`.
    pub async fn attach_intake_link_to_export(
        &self,
        export_id: Uuid,
        batch_id: Uuid,
        item_id: Uuid,
    ) -> AtelierResult<ExportIntakeLink> {
        let export = self.get_export_request(export_id).await?;
        let batch_row = sqlx::query(
            r#"SELECT character_internal_id, target_character_id,
                      target_sheet_version_id, target_collection_id
               FROM atelier_intake_batch
               WHERE batch_id = $1"#,
        )
        .bind(batch_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("intake batch {batch_id}")))?;
        let batch_character_id: Option<Uuid> = batch_row.get("character_internal_id");
        let target_character_id: Option<Uuid> = batch_row.get("target_character_id");
        let target_sheet_version_id: Option<Uuid> = batch_row.get("target_sheet_version_id");
        let target_collection_id: Option<Uuid> = batch_row.get("target_collection_id");
        let effective_target_character_id = target_character_id.or(batch_character_id);

        if let Some(target_character_id) = effective_target_character_id {
            if target_character_id != export.character_internal_id {
                return Err(AtelierError::Validation(format!(
                    "intake batch {batch_id} targets character {target_character_id}, not export character {}",
                    export.character_internal_id
                )));
            }
        }
        if let Some(target_sheet_version_id) = target_sheet_version_id {
            if target_sheet_version_id != export.sheet_version_id {
                return Err(AtelierError::Validation(format!(
                    "intake batch {batch_id} targets sheet version {target_sheet_version_id}, not export sheet version {}",
                    export.sheet_version_id
                )));
            }
        }

        let item_batch_id: Option<Uuid> =
            sqlx::query_scalar("SELECT batch_id FROM atelier_intake_item WHERE item_id = $1")
                .bind(item_id)
                .fetch_optional(self.pool())
                .await?;
        match item_batch_id {
            None => {
                return Err(AtelierError::NotFound(format!("intake item {item_id}")));
            }
            Some(owner_batch_id) if owner_batch_id != batch_id => {
                return Err(AtelierError::Validation(format!(
                    "intake item {item_id} belongs to batch {owner_batch_id}, not {batch_id}"
                )));
            }
            Some(_) => {}
        }

        if let Some(target_collection_id) = target_collection_id {
            let collection_row = sqlx::query(
                r#"SELECT character_internal_id, sheet_version_id
                   FROM atelier_collection
                   WHERE collection_id = $1"#,
            )
            .bind(target_collection_id)
            .fetch_optional(self.pool())
            .await?
            .ok_or_else(|| {
                AtelierError::NotFound(format!("target collection {target_collection_id}"))
            })?;
            let collection_character_id: Option<Uuid> = collection_row.get("character_internal_id");
            let collection_sheet_version_id: Option<Uuid> = collection_row.get("sheet_version_id");
            if let Some(collection_character_id) = collection_character_id {
                if collection_character_id != export.character_internal_id {
                    return Err(AtelierError::Validation(format!(
                        "target collection {target_collection_id} belongs to character {collection_character_id}, not export character {}",
                        export.character_internal_id
                    )));
                }
            }
            if let (Some(collection_sheet_version_id), Some(target_sheet_version_id)) =
                (collection_sheet_version_id, target_sheet_version_id)
            {
                if collection_sheet_version_id != target_sheet_version_id {
                    return Err(AtelierError::Validation(format!(
                        "target collection {target_collection_id} belongs to sheet version {collection_sheet_version_id}, not intake target sheet version {target_sheet_version_id}"
                    )));
                }
            }
        }

        let version_agnostic = target_sheet_version_id.is_none();
        let row = sqlx::query(
            r#"WITH inserted AS (
                 INSERT INTO atelier_export_intake_link
                   (export_id, batch_id, item_id, target_character_id,
                    target_sheet_version_id, target_collection_id,
                    version_agnostic)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (export_id, batch_id, item_id) DO NOTHING
                 RETURNING link_id, export_id, batch_id, item_id,
                           target_character_id, target_sheet_version_id,
                           target_collection_id, version_agnostic,
                           created_at_utc
               )
               SELECT TRUE AS inserted, link_id, export_id, batch_id, item_id,
                      target_character_id, target_sheet_version_id,
                      target_collection_id, version_agnostic, created_at_utc
               FROM inserted
               UNION ALL
               SELECT FALSE AS inserted, link_id, export_id, batch_id, item_id,
                      target_character_id, target_sheet_version_id,
                      target_collection_id, version_agnostic, created_at_utc
               FROM atelier_export_intake_link
               WHERE export_id = $1 AND batch_id = $2 AND item_id = $3
               LIMIT 1"#,
        )
        .bind(export_id)
        .bind(batch_id)
        .bind(item_id)
        .bind(effective_target_character_id)
        .bind(target_sheet_version_id)
        .bind(target_collection_id)
        .bind(version_agnostic)
        .fetch_one(self.pool())
        .await?;
        let inserted: bool = row.get("inserted");
        let link = intake_link_from_row(&row);

        if inserted {
            self.record_event(
                EXPORT_INTAKE_LINK_ATTACHED,
                "atelier_export_request",
                &export_id.to_string(),
                serde_json::json!({
                    "link_id": link.link_id,
                    "batch_id": link.batch_id,
                    "item_id": link.item_id,
                    "version_agnostic": link.version_agnostic,
                    "target_character_ref": link
                        .target_character_id
                        .map(|id| event_ref_for_text(&id.to_string())),
                    "target_sheet_version_ref": link
                        .target_sheet_version_id
                        .map(|id| event_ref_for_text(&id.to_string())),
                    "target_collection_ref": link
                        .target_collection_id
                        .map(|id| event_ref_for_text(&id.to_string())),
                }),
            )
            .await?;
        }

        Ok(link)
    }

    /// Export-linked intake target refs, ordered by durable attach time.
    pub async fn export_intake_links(
        &self,
        export_id: Uuid,
    ) -> AtelierResult<Vec<ExportIntakeLink>> {
        let _ = self.get_export_request(export_id).await?;
        let rows = sqlx::query(
            r#"SELECT link_id, export_id, batch_id, item_id,
                      target_character_id, target_sheet_version_id,
                      target_collection_id, version_agnostic, created_at_utc
               FROM atelier_export_intake_link
               WHERE export_id = $1
               ORDER BY created_at_utc ASC, link_id ASC"#,
        )
        .bind(export_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(intake_link_from_row).collect())
    }

    /// Preserve PNG/JPG contact-sheet export as an explicit planned capability
    /// without producing fake raster bytes. Idempotent on `(sheet_id, format)`.
    pub async fn plan_contact_sheet_raster_export(
        &self,
        sheet_id: Uuid,
        format: ContactSheetRasterExportFormat,
        requested_by: &str,
    ) -> AtelierResult<ContactSheetRasterExportPlan> {
        let requested_by = requested_by.trim();
        if requested_by.is_empty() {
            return Err(AtelierError::Validation(
                "requested_by must not be empty".into(),
            ));
        }
        let _ = self.get_contact_sheet(sheet_id).await?;
        let status = ContactSheetRasterExportStatus::Planned;
        let row = sqlx::query(
            r#"WITH inserted AS (
                 INSERT INTO atelier_contact_sheet_raster_export_plan
                   (sheet_id, format, status, reason, requested_by)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (sheet_id, format) DO NOTHING
                 RETURNING plan_id, sheet_id, format, status, reason,
                           requested_by, created_at_utc
               )
               SELECT TRUE AS inserted, plan_id, sheet_id, format, status, reason,
                      requested_by, created_at_utc
               FROM inserted
               UNION ALL
               SELECT FALSE AS inserted, plan_id, sheet_id, format, status, reason,
                      requested_by, created_at_utc
               FROM atelier_contact_sheet_raster_export_plan
               WHERE sheet_id = $1 AND format = $2
               LIMIT 1"#,
        )
        .bind(sheet_id)
        .bind(format.as_token())
        .bind(status.as_token())
        .bind(CONTACT_SHEET_RASTER_EXPORT_DEFERRED_REASON)
        .bind(requested_by)
        .fetch_one(self.pool())
        .await?;
        let inserted: bool = row.get("inserted");
        let plan = raster_export_plan_from_row(&row)?;

        if inserted {
            self.record_event(
                CONTACT_SHEET_RASTER_EXPORT_PLANNED,
                "atelier_contact_sheet",
                &sheet_id.to_string(),
                serde_json::json!({
                    "plan_id": plan.plan_id,
                    "sheet_id": plan.sheet_id,
                    "format": plan.format.as_token(),
                    "status": plan.status.as_token(),
                    "reason": plan.reason,
                    "requested_by": plan.requested_by,
                    "output_artifact": "not_produced",
                }),
            )
            .await?;
        }

        Ok(plan)
    }

    /// List planned PNG/JPG raster export markers for a contact sheet.
    pub async fn list_contact_sheet_raster_export_plans(
        &self,
        sheet_id: Uuid,
    ) -> AtelierResult<Vec<ContactSheetRasterExportPlan>> {
        let _ = self.get_contact_sheet(sheet_id).await?;
        let rows = sqlx::query(
            r#"SELECT plan_id, sheet_id, format, status, reason,
                      requested_by, created_at_utc
               FROM atelier_contact_sheet_raster_export_plan
               WHERE sheet_id = $1
               ORDER BY created_at_utc ASC, format ASC"#,
        )
        .bind(sheet_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(raster_export_plan_from_row).collect()
    }
}
