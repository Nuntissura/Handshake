//! Media-downloader-v2 governed records (MT-204, WP-KERNEL-005 legacy source fold-in).
//!
//! Spec authority: master-spec-v02.189 Section 6.10 "Media Downloader v2 Depth"
//! (6.10.2 OutputRootConfigV2, 6.10.3 MdDownloadSessionV2 + MdItemStateV2 staged
//! resumable sessions + checkpoints, 6.10.4 MdAuthContextV2 + MdAllowlistPolicyV2,
//! 6.10.5 MdSessionReceiptV2 sanitized telemetry/receipts).
//!
//! legacy source (intent only): legacy source `app backend Media-Downloader-v2`.
//! The SQLite/Electron/localhost/polling originals are NOT copied; only the
//! governed DATA + RECEIPT contract is translated. Storage authority is
//! PostgreSQL + EventLedger only (see [`super::assert_postgres_url`], MT-004).
//!
//! IMPORTANT BOUNDARY (Section 6.10.1 LAW-MDV2-EXEC-001..003): this module is a
//! pure governed records/receipt repository. It NEVER opens a socket, spawns a
//! process, or calls an external endpoint. Actual URL expansion, fetch, probe,
//! merge, and materialization run as a Workflow-Engine job elsewhere; that job
//! writes its session/item/checkpoint/receipt rows THROUGH the methods here and
//! the canonical state is reconstructable from these rows + the EventLedger.
//!
//! REDACTION BOUNDARY (Section 6.10.4 LAW-MDV2-AUTH-001..002): secrets, cookies,
//! header tokens, and Authorization values are carried by reference (`*_ref`)
//! ONLY. No inline secret material is ever persisted, and every stored record /
//! event payload is redacted (follows `settings.rs` redaction style). A persisted
//! or event-leaked raw secret is a hard violation, so the auth-context API
//! accepts only refs and never raw values.
//!
//! Microtasks: MT-204 (media-downloader-v2 records), MT-005 (event coverage).

use crate::capabilities::CapabilityRegistry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

/// Media-downloader-v2 event families (MT-204, extends the MT-005 coverage set).
///
/// Defined here so the parent folds these into [`super::event_family::ALL`] and
/// the MT-005 coverage proof picks up every downloader mutation (Section 6.10.3
/// LAW-MDV2-RESUME-003: every stage transition leaves a checkpoint event;
/// Section 6.10.5 LAW-MDV2-TEL-003: every session produces a receipt).
pub mod downloader_event_family {
    /// An OutputRootDir configuration was set/updated (6.10.2).
    pub const OUTPUT_ROOT_CONFIGURED: &str = "atelier.downloader.output_root_configured";
    /// An allowlist policy was set/updated (6.10.4 LAW-MDV2-CAP-001).
    pub const ALLOWLIST_POLICY_SET: &str = "atelier.downloader.allowlist_policy_set";
    /// A redacted auth context was registered (6.10.4 LAW-MDV2-AUTH-001).
    pub const AUTH_CONTEXT_REGISTERED: &str = "atelier.downloader.auth_context_registered";
    /// A staged download session was opened (6.10.3).
    pub const SESSION_OPENED: &str = "atelier.downloader.session_opened";
    /// A session moved to a new stage (6.10.3 staged lifecycle).
    pub const SESSION_STAGE_CHANGED: &str = "atelier.downloader.session_stage_changed";
    /// An item was enqueued into a session (6.10.3 MdItemStateV2).
    pub const ITEM_ENQUEUED: &str = "atelier.downloader.item_enqueued";
    /// A resumable checkpoint was recorded for an item/session (6.10.3
    /// LAW-MDV2-RESUME-003 MdCheckpointV2).
    pub const ITEM_CHECKPOINTED: &str = "atelier.downloader.item_checkpointed";
    /// A recoverable session receipt was produced (6.10.5 LAW-MDV2-TEL-003
    /// MdSessionReceiptV2).
    pub const SESSION_RECEIPT_EMITTED: &str = "atelier.downloader.session_receipt_emitted";
    /// Canonical leak-safe job-state telemetry (6.10.5 LAW-MDV2-TEL-001).
    pub const MEDIA_DOWNLOADER_JOB_STATE: &str = "media_downloader.job_state";
    /// Canonical leak-safe byte-progress telemetry (6.10.5 LAW-MDV2-TEL-001).
    pub const MEDIA_DOWNLOADER_PROGRESS: &str = "media_downloader.progress";
    /// Canonical leak-safe per-item terminal result telemetry (6.10.5
    /// LAW-MDV2-TEL-001).
    pub const MEDIA_DOWNLOADER_ITEM_RESULT: &str = "media_downloader.item_result";

    /// All downloader event families (parity/coverage helper).
    pub const ALL: &[&str] = &[
        OUTPUT_ROOT_CONFIGURED,
        ALLOWLIST_POLICY_SET,
        AUTH_CONTEXT_REGISTERED,
        SESSION_OPENED,
        SESSION_STAGE_CHANGED,
        ITEM_ENQUEUED,
        ITEM_CHECKPOINTED,
        SESSION_RECEIPT_EMITTED,
        MEDIA_DOWNLOADER_JOB_STATE,
        MEDIA_DOWNLOADER_PROGRESS,
        MEDIA_DOWNLOADER_ITEM_RESULT,
    ];
}

/// Re-export so callers can write `downloader::SESSION_OPENED`.
pub use downloader_event_family::{
    ALLOWLIST_POLICY_SET, AUTH_CONTEXT_REGISTERED, ITEM_CHECKPOINTED, ITEM_ENQUEUED,
    MEDIA_DOWNLOADER_ITEM_RESULT, MEDIA_DOWNLOADER_JOB_STATE, MEDIA_DOWNLOADER_PROGRESS,
    OUTPUT_ROOT_CONFIGURED, SESSION_OPENED, SESSION_RECEIPT_EMITTED, SESSION_STAGE_CHANGED,
};

/// Marker substituted for any value that looks like inline secret material.
const REDACTED_PLACEHOLDER: &str = "[REDACTED]";
pub const MEDIA_DOWNLOADER_JOB_KIND: &str = "media_downloader";
pub const MEDIA_DOWNLOADER_BATCH_PROTOCOL_ID: &str = "hsk.media_downloader.batch.v0";
const MEDIA_DOWNLOADER_GRANT_PREFIX: &str = "capgrant://media_downloader/";

/// Heuristic guard backing the redaction boundary (Section 6.10.4
/// LAW-MDV2-AUTH-001/002). Auth material MUST be carried by reference only;
/// this rejects inputs that smell like an inline cookie/token/Authorization
/// value so a raw secret can never be persisted in a record or event.
fn reject_inline_secret(field: &str, value: &str) -> AtelierResult<()> {
    let lowered = value.to_ascii_lowercase();
    let looks_inline = lowered.contains("authorization:")
        || lowered.contains("set-cookie")
        || lowered.starts_with("cookie:")
        || lowered.starts_with("bearer ")
        || lowered.contains("sessionid=")
        || lowered.contains("token=")
        || lowered.contains("password=")
        || lowered.contains("secret=");
    if looks_inline {
        return Err(AtelierError::Validation(format!(
            "{field} must be a secret-store reference, not inline secret material \
             (Section 6.10.4 LAW-MDV2-AUTH-001)"
        )));
    }
    Ok(())
}

fn reject_legacy_runtime_refs_in_json(field: &str, value: &serde_json::Value) -> AtelierResult<()> {
    match value {
        serde_json::Value::String(text) => reject_legacy_runtime_ref(field, text),
        serde_json::Value::Array(items) => {
            for item in items {
                reject_legacy_runtime_refs_in_json(field, item)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                reject_legacy_runtime_refs_in_json(field, item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// How a downloaded canonical artifact is materialized under the resolved root
/// (Section 6.10.2 `materialization_mode`). "hardlink" preferred where the
/// filesystem supports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterializationMode {
    Copy,
    Hardlink,
    Symlink,
}

impl MaterializationMode {
    pub fn as_token(self) -> &'static str {
        match self {
            MaterializationMode::Copy => "copy",
            MaterializationMode::Hardlink => "hardlink",
            MaterializationMode::Symlink => "symlink",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "copy" => Ok(MaterializationMode::Copy),
            "hardlink" => Ok(MaterializationMode::Hardlink),
            "symlink" => Ok(MaterializationMode::Symlink),
            other => Err(AtelierError::Validation(format!(
                "unknown materialization_mode token: {other}"
            ))),
        }
    }
}

/// Source provider for a download session (Section 6.10.3 `source_kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Youtube,
    Instagram,
    Forumcrawler,
    Videodownloader,
}

impl SourceKind {
    pub fn as_token(self) -> &'static str {
        match self {
            SourceKind::Youtube => "youtube",
            SourceKind::Instagram => "instagram",
            SourceKind::Forumcrawler => "forumcrawler",
            SourceKind::Videodownloader => "videodownloader",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "youtube" => Ok(SourceKind::Youtube),
            "instagram" => Ok(SourceKind::Instagram),
            "forumcrawler" => Ok(SourceKind::Forumcrawler),
            "videodownloader" => Ok(SourceKind::Videodownloader),
            other => Err(AtelierError::Validation(format!(
                "unknown source_kind token: {other}"
            ))),
        }
    }
}

/// Auth mode for a session (Section 6.10.4 PRIM-MdAuthMode). Secrets are always
/// by reference; this only records WHICH mode is in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    None,
    Session,
    CookieJar,
    Header,
}

impl AuthMode {
    pub fn as_token(self) -> &'static str {
        match self {
            AuthMode::None => "none",
            AuthMode::Session => "session",
            AuthMode::CookieJar => "cookie_jar",
            AuthMode::Header => "header",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "none" => Ok(AuthMode::None),
            "session" => Ok(AuthMode::Session),
            "cookie_jar" => Ok(AuthMode::CookieJar),
            "header" => Ok(AuthMode::Header),
            other => Err(AtelierError::Validation(format!(
                "unknown auth_mode token: {other}"
            ))),
        }
    }
}

/// Staged session lifecycle (Section 6.10.3). Linear progression
/// `resolving -> enqueued -> fetching -> probing -> merging -> materializing ->
/// finalized` with terminal branches `paused`, `failed`, `cancelled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStage {
    Resolving,
    Enqueued,
    Fetching,
    Probing,
    Merging,
    Materializing,
    Finalized,
    Paused,
    Failed,
    Cancelled,
}

impl SessionStage {
    pub fn as_token(self) -> &'static str {
        match self {
            SessionStage::Resolving => "resolving",
            SessionStage::Enqueued => "enqueued",
            SessionStage::Fetching => "fetching",
            SessionStage::Probing => "probing",
            SessionStage::Merging => "merging",
            SessionStage::Materializing => "materializing",
            SessionStage::Finalized => "finalized",
            SessionStage::Paused => "paused",
            SessionStage::Failed => "failed",
            SessionStage::Cancelled => "cancelled",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "resolving" => Ok(SessionStage::Resolving),
            "enqueued" => Ok(SessionStage::Enqueued),
            "fetching" => Ok(SessionStage::Fetching),
            "probing" => Ok(SessionStage::Probing),
            "merging" => Ok(SessionStage::Merging),
            "materializing" => Ok(SessionStage::Materializing),
            "finalized" => Ok(SessionStage::Finalized),
            "paused" => Ok(SessionStage::Paused),
            "failed" => Ok(SessionStage::Failed),
            "cancelled" => Ok(SessionStage::Cancelled),
            other => Err(AtelierError::Validation(format!(
                "unknown session stage token: {other}"
            ))),
        }
    }

    /// Whether this is a terminal stage (no further transitions expected).
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            SessionStage::Finalized | SessionStage::Failed | SessionStage::Cancelled
        )
    }
}

/// Per-item stage (Section 6.10.3 `MdItemStateV2.stage`). Items move through a
/// subset of the session lifecycle plus a `skipped` lane for dedupe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStage {
    Enqueued,
    Fetching,
    Probing,
    Merging,
    Materializing,
    Finalized,
    Skipped,
    Failed,
}

impl ItemStage {
    pub fn as_token(self) -> &'static str {
        match self {
            ItemStage::Enqueued => "enqueued",
            ItemStage::Fetching => "fetching",
            ItemStage::Probing => "probing",
            ItemStage::Merging => "merging",
            ItemStage::Materializing => "materializing",
            ItemStage::Finalized => "finalized",
            ItemStage::Skipped => "skipped",
            ItemStage::Failed => "failed",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "enqueued" => Ok(ItemStage::Enqueued),
            "fetching" => Ok(ItemStage::Fetching),
            "probing" => Ok(ItemStage::Probing),
            "merging" => Ok(ItemStage::Merging),
            "materializing" => Ok(ItemStage::Materializing),
            "finalized" => Ok(ItemStage::Finalized),
            "skipped" => Ok(ItemStage::Skipped),
            "failed" => Ok(ItemStage::Failed),
            other => Err(AtelierError::Validation(format!(
                "unknown item stage token: {other}"
            ))),
        }
    }
}

/// Terminal stage recorded on a session receipt (Section 6.10.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStage {
    Finalized,
    Failed,
    Cancelled,
}

impl TerminalStage {
    pub fn as_token(self) -> &'static str {
        match self {
            TerminalStage::Finalized => "finalized",
            TerminalStage::Failed => "failed",
            TerminalStage::Cancelled => "cancelled",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "finalized" => Ok(TerminalStage::Finalized),
            "failed" => Ok(TerminalStage::Failed),
            "cancelled" => Ok(TerminalStage::Cancelled),
            other => Err(AtelierError::Validation(format!(
                "unknown terminal stage token: {other}"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// OutputRootConfigV2 (Section 6.10.2)
// ---------------------------------------------------------------------------

/// OutputRootDir configuration record (Section 6.10.2 OutputRootConfigV2).
///
/// `configured_root` MUST be stored in portable form only (LAW-MDV2-OUT-001);
/// resolution to an absolute path happens at job time and is not persisted here.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputRootConfig {
    pub root_id: Uuid,
    /// Operator-set portable base (e.g. `media_downloader/`). Never a drive
    /// letter / user-profile / absolute machine path.
    pub configured_root: String,
    pub materialization_mode: MaterializationMode,
    /// Map of `source_kind -> relative subpath` (defaults per 10.14.6).
    pub per_mode_subdirs: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to set/update an output-root config (idempotent on `configured_root`).
#[derive(Clone, Debug)]
pub struct SetOutputRootConfig {
    pub configured_root: String,
    pub materialization_mode: MaterializationMode,
    pub per_mode_subdirs: serde_json::Value,
}

// ---------------------------------------------------------------------------
// MdAllowlistPolicyV2 (Section 6.10.4 LAW-MDV2-CAP-001)
// ---------------------------------------------------------------------------

/// Allowlist + capability-gating policy (Section 6.10.4 MdAllowlistPolicyV2).
/// Every external fetch must pass an allowlist decision before any network call;
/// the actual decision/fetch happens in the Workflow-Engine job, this stores the
/// governed policy the job must honor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AllowlistPolicy {
    pub allowlist_policy_id: Uuid,
    /// Stable operator-facing name; doubles as the idempotency key.
    pub name: String,
    pub allowed_domains: serde_json::Value,
    pub explicit_url_lists: serde_json::Value,
    /// Default "deny" for non-allowlisted domains under a crawl posture.
    pub default_decision: String,
    pub rate_limit: serde_json::Value,
    /// Crawler bound: default 1500, hard cap 5000 (10.14.9).
    pub max_pages: i64,
    pub robots_posture: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to set/update an allowlist policy (idempotent on `name`).
#[derive(Clone, Debug)]
pub struct SetAllowlistPolicy {
    pub name: String,
    pub allowed_domains: serde_json::Value,
    pub explicit_url_lists: serde_json::Value,
    pub default_decision: String,
    pub rate_limit: serde_json::Value,
    pub max_pages: i64,
    pub robots_posture: String,
}

// ---------------------------------------------------------------------------
// MdAuthContextV2 (Section 6.10.4 LAW-MDV2-AUTH-001)
// ---------------------------------------------------------------------------

/// Redacted auth context (Section 6.10.4 MdAuthContextV2).
///
/// Secrets are NEVER stored. Only references are kept: a cookie-jar artifact ref
/// (the jar lives in ArtifactStore, classification "high", `exportable=false`),
/// a session record ref, and an array of header-secret refs. Inline secret
/// material is rejected at the API boundary.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthContext {
    pub auth_context_ref: Uuid,
    /// Stable operator-facing label; doubles as the idempotency key.
    pub label: String,
    pub auth_mode: AuthMode,
    /// PRIM-MdSessionRecordV0 reference (only when `auth_mode = session`).
    pub session_ref: Option<String>,
    /// ArtifactStore ref to a Netscape cookies.txt jar (cookie_jar mode).
    pub cookie_jar_artifact_ref: Option<String>,
    /// References to secret-store entries for custom headers (header mode).
    /// NEVER inline header values.
    pub header_secret_refs: serde_json::Value,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to register/update an auth context. All fields are refs; raw secrets
/// are rejected.
#[derive(Clone, Debug)]
pub struct RegisterAuthContext {
    pub label: String,
    pub auth_mode: AuthMode,
    pub session_ref: Option<String>,
    pub cookie_jar_artifact_ref: Option<String>,
    pub header_secret_refs: serde_json::Value,
}

// ---------------------------------------------------------------------------
// MdDownloadSessionV2 + MdItemStateV2 (Section 6.10.3)
// ---------------------------------------------------------------------------

/// A staged, resumable download session (Section 6.10.3 MdDownloadSessionV2).
/// Canonical session state lives here + the EventLedger; an in-memory queue is a
/// projection reconstructable from these rows (LAW-MDV2-EXEC-002).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadSession {
    pub session_id: Uuid,
    /// Workflow-Engine job id (opaque; the engine owns execution, not this module).
    pub parent_job_id: String,
    /// Stable idempotency key so re-opening the same job is safe.
    pub idempotency_key: String,
    pub source_kind: SourceKind,
    pub auth_context_ref: Option<Uuid>,
    pub allowlist_policy_id: Uuid,
    pub output_root_id: Uuid,
    /// Workflow protocol whose capability set was validated before opening.
    pub protocol_id: String,
    /// Capability profile used by the Workflow-Engine job.
    pub capability_profile_id: String,
    /// Opaque capability grant evidence ref. Never a secret value.
    pub capability_grant_ref: String,
    pub stage: SessionStage,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to open (or idempotently re-open) a download session.
#[derive(Clone, Debug)]
pub struct OpenDownloadSession {
    pub parent_job_id: String,
    pub idempotency_key: String,
    pub source_kind: SourceKind,
    pub auth_context_ref: Option<Uuid>,
    pub allowlist_policy_id: Uuid,
    pub output_root_id: Uuid,
    pub protocol_id: String,
    pub capability_profile_id: String,
    pub capability_grant_ref: String,
}

/// Per-item download state (Section 6.10.3 MdItemStateV2). `resume_token` is an
/// opaque per-item offset/range cursor used to continue after restart
/// (LAW-MDV2-RESUME-002).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemState {
    pub item_id: Uuid,
    pub session_id: Uuid,
    /// Allowlist-checked URL with query secrets stripped (telemetry-safe).
    pub normalized_url: String,
    pub stable_source_id: Option<String>,
    pub content_hash: Option<String>,
    pub stage: ItemStage,
    pub bytes_downloaded: i64,
    pub bytes_total: Option<i64>,
    /// The `.part` staging artifact ref (10.14.8); never a filesystem path.
    pub part_path_ref: Option<String>,
    pub attempt_count: i64,
    pub last_error_code: Option<String>,
    /// Opaque resume cursor (byte/offset/range), per LAW-MDV2-RESUME-002.
    pub resume_token: Option<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to enqueue an item into a session.
#[derive(Clone, Debug)]
pub struct EnqueueItem {
    pub normalized_url: String,
    pub stable_source_id: Option<String>,
}

/// A resumable checkpoint (Section 6.10.3 LAW-MDV2-RESUME-003 MdCheckpointV2),
/// emitted at every stage transition and at bounded progress intervals during
/// fetching. Checkpoints are the recovery anchor.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Checkpoint {
    pub checkpoint_id: Uuid,
    pub session_id: Uuid,
    /// Null for a session-level checkpoint.
    pub item_id: Option<Uuid>,
    pub stage: String,
    pub bytes_downloaded: i64,
    pub bytes_total: Option<i64>,
    pub resume_token: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to record a checkpoint and advance item progress/resume cursor.
#[derive(Clone, Debug)]
pub struct RecordCheckpoint {
    /// None for a session-level checkpoint.
    pub item_id: Option<Uuid>,
    /// The stage at the moment of the checkpoint (session or item stage token).
    pub stage: String,
    pub bytes_downloaded: i64,
    pub bytes_total: Option<i64>,
    pub resume_token: Option<String>,
}

// ---------------------------------------------------------------------------
// MdSessionReceiptV2 (Section 6.10.5 LAW-MDV2-TEL-003)
// ---------------------------------------------------------------------------

/// Recoverable session receipt (Section 6.10.5 MdSessionReceiptV2). Sufficient
/// to reconstruct what was attempted, fetched, deduped, and materialized for
/// replay/audit. Contains NO secret material (auth carried by ref only).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionReceipt {
    pub receipt_id: Uuid,
    pub session_id: Uuid,
    pub parent_job_id: String,
    pub source_kind: SourceKind,
    pub auth_context_ref: Option<Uuid>,
    pub allowlist_policy_id: Uuid,
    pub output_root_id: Uuid,
    pub item_count: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub skipped_deduped: i64,
    /// Portable materialized references only (no machine-local absolute paths).
    pub materialized_paths: serde_json::Value,
    /// ArtifactStore ref to the per-item manifest.
    pub manifest_artifact_ref: Option<String>,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub terminal_stage: TerminalStage,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to emit a session receipt.
#[derive(Clone, Debug)]
pub struct EmitSessionReceipt {
    pub item_count: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub skipped_deduped: i64,
    pub materialized_paths: serde_json::Value,
    pub manifest_artifact_ref: Option<String>,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub ended_at_utc: Option<DateTime<Utc>>,
    pub terminal_stage: TerminalStage,
}

pub(crate) fn validate_media_downloader_capability_grant(
    protocol_id: &str,
    capability_profile_id: &str,
    capability_grant_ref: &str,
) -> AtelierResult<Vec<String>> {
    let protocol_id = protocol_id.trim();
    let capability_profile_id = capability_profile_id.trim();
    let capability_grant_ref = capability_grant_ref.trim();

    if protocol_id.is_empty() {
        return Err(AtelierError::Validation(
            "protocol_id must not be empty".into(),
        ));
    }
    if capability_profile_id.is_empty() {
        return Err(AtelierError::Validation(
            "capability_profile_id must not be empty".into(),
        ));
    }

    let rest = capability_grant_ref
        .strip_prefix(MEDIA_DOWNLOADER_GRANT_PREFIX)
        .ok_or_else(|| {
            AtelierError::Validation(format!(
                "capability_grant_ref must start with {MEDIA_DOWNLOADER_GRANT_PREFIX}"
            ))
        })?;
    let (grant_profile_id, evidence_ref) = rest.split_once('/').ok_or_else(|| {
        AtelierError::Validation(
            "capability_grant_ref must include profile/evidence for media_downloader".into(),
        )
    })?;
    if grant_profile_id.trim().is_empty() || evidence_ref.trim().is_empty() {
        return Err(AtelierError::Validation(
            "capability_grant_ref must include non-empty profile/evidence for media_downloader"
                .into(),
        ));
    }
    reject_legacy_runtime_ref("capability_grant_ref evidence_ref", evidence_ref)?;
    if grant_profile_id != capability_profile_id {
        return Err(AtelierError::Validation(format!(
            "capability_grant_ref profile {grant_profile_id} must match capability_profile_id {capability_profile_id}"
        )));
    }

    let registry = CapabilityRegistry::new();
    let required = registry
        .required_capabilities_for_job_request(MEDIA_DOWNLOADER_JOB_KIND, protocol_id)
        .map_err(|err| AtelierError::Validation(err.to_string()))?;
    for capability in &required {
        match registry.profile_can(capability_profile_id, capability) {
            Ok(true) => {}
            Ok(false) => {
                return Err(AtelierError::Validation(format!(
                    "capability profile {capability_profile_id} is not granted required media_downloader capability {capability}"
                )));
            }
            Err(err) => {
                return Err(AtelierError::Validation(format!(
                    "capability profile {capability_profile_id} cannot grant required media_downloader capability {capability}: {err}"
                )));
            }
        }
    }

    Ok(required)
}

// ---------------------------------------------------------------------------
// Row mappers
// ---------------------------------------------------------------------------

fn output_root_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<OutputRootConfig> {
    let mode: String = row.get("materialization_mode");
    Ok(OutputRootConfig {
        root_id: row.get("root_id"),
        configured_root: row.get("configured_root"),
        materialization_mode: MaterializationMode::from_token(&mode)?,
        per_mode_subdirs: row.get("per_mode_subdirs"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn allowlist_from_row(row: &sqlx::postgres::PgRow) -> AllowlistPolicy {
    AllowlistPolicy {
        allowlist_policy_id: row.get("allowlist_policy_id"),
        name: row.get("name"),
        allowed_domains: row.get("allowed_domains"),
        explicit_url_lists: row.get("explicit_url_lists"),
        default_decision: row.get("default_decision"),
        rate_limit: row.get("rate_limit"),
        max_pages: row.get("max_pages"),
        robots_posture: row.get("robots_posture"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn auth_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<AuthContext> {
    let mode: String = row.get("auth_mode");
    Ok(AuthContext {
        auth_context_ref: row.get("auth_context_ref"),
        label: row.get("label"),
        auth_mode: AuthMode::from_token(&mode)?,
        session_ref: row.get("session_ref"),
        cookie_jar_artifact_ref: row.get("cookie_jar_artifact_ref"),
        header_secret_refs: row.get("header_secret_refs"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn session_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<DownloadSession> {
    let source_kind: String = row.get("source_kind");
    let stage: String = row.get("stage");
    Ok(DownloadSession {
        session_id: row.get("session_id"),
        parent_job_id: row.get("parent_job_id"),
        idempotency_key: row.get("idempotency_key"),
        source_kind: SourceKind::from_token(&source_kind)?,
        auth_context_ref: row.get("auth_context_ref"),
        allowlist_policy_id: row.get("allowlist_policy_id"),
        output_root_id: row.get("output_root_id"),
        protocol_id: row.get("protocol_id"),
        capability_profile_id: row.get("capability_profile_id"),
        capability_grant_ref: row.get("capability_grant_ref"),
        stage: SessionStage::from_token(&stage)?,
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn item_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ItemState> {
    let stage: String = row.get("stage");
    Ok(ItemState {
        item_id: row.get("item_id"),
        session_id: row.get("session_id"),
        normalized_url: row.get("normalized_url"),
        stable_source_id: row.get("stable_source_id"),
        content_hash: row.get("content_hash"),
        stage: ItemStage::from_token(&stage)?,
        bytes_downloaded: row.get("bytes_downloaded"),
        bytes_total: row.get("bytes_total"),
        part_path_ref: row.get("part_path_ref"),
        attempt_count: row.get("attempt_count"),
        last_error_code: row.get("last_error_code"),
        resume_token: row.get("resume_token"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn checkpoint_from_row(row: &sqlx::postgres::PgRow) -> Checkpoint {
    Checkpoint {
        checkpoint_id: row.get("checkpoint_id"),
        session_id: row.get("session_id"),
        item_id: row.get("item_id"),
        stage: row.get("stage"),
        bytes_downloaded: row.get("bytes_downloaded"),
        bytes_total: row.get("bytes_total"),
        resume_token: row.get("resume_token"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn receipt_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<SessionReceipt> {
    let source_kind: String = row.get("source_kind");
    let terminal: String = row.get("terminal_stage");
    Ok(SessionReceipt {
        receipt_id: row.get("receipt_id"),
        session_id: row.get("session_id"),
        parent_job_id: row.get("parent_job_id"),
        source_kind: SourceKind::from_token(&source_kind)?,
        auth_context_ref: row.get("auth_context_ref"),
        allowlist_policy_id: row.get("allowlist_policy_id"),
        output_root_id: row.get("output_root_id"),
        item_count: row.get("item_count"),
        succeeded: row.get("succeeded"),
        failed: row.get("failed"),
        skipped_deduped: row.get("skipped_deduped"),
        materialized_paths: row.get("materialized_paths"),
        manifest_artifact_ref: row.get("manifest_artifact_ref"),
        started_at_utc: row.get("started_at_utc"),
        ended_at_utc: row.get("ended_at_utc"),
        terminal_stage: TerminalStage::from_token(&terminal)?,
        created_at_utc: row.get("created_at_utc"),
    })
}

const OUTPUT_ROOT_COLUMNS: &str = "root_id, configured_root, materialization_mode, \
                                   per_mode_subdirs, created_at_utc, updated_at_utc";
const ALLOWLIST_COLUMNS: &str = "allowlist_policy_id, name, allowed_domains, explicit_url_lists, \
                                 default_decision, rate_limit, max_pages, robots_posture, \
                                 created_at_utc, updated_at_utc";
const AUTH_COLUMNS: &str = "auth_context_ref, label, auth_mode, session_ref, \
                            cookie_jar_artifact_ref, header_secret_refs, created_at_utc, \
                            updated_at_utc";
const SESSION_COLUMNS: &str = "session_id, parent_job_id, idempotency_key, source_kind, \
                               auth_context_ref, allowlist_policy_id, output_root_id, \
                               protocol_id, capability_profile_id, capability_grant_ref, stage, \
                               created_at_utc, updated_at_utc";
const ITEM_COLUMNS: &str = "item_id, session_id, normalized_url, stable_source_id, content_hash, \
                            stage, bytes_downloaded, bytes_total, part_path_ref, attempt_count, \
                            last_error_code, resume_token, created_at_utc, updated_at_utc";
const RECEIPT_COLUMNS: &str = "receipt_id, session_id, parent_job_id, source_kind, \
                               auth_context_ref, allowlist_policy_id, output_root_id, item_count, \
                               succeeded, failed, skipped_deduped, materialized_paths, \
                               manifest_artifact_ref, started_at_utc, ended_at_utc, \
                               terminal_stage, created_at_utc";

impl AtelierStore {
    // -----------------------------------------------------------------------
    // OutputRootConfigV2 (6.10.2)
    // -----------------------------------------------------------------------

    /// Set (create or update) an output-root config keyed by portable
    /// `configured_root` (Section 6.10.2). Rejects machine-local absolute paths
    /// (LAW-MDV2-OUT-001). Emits `OUTPUT_ROOT_CONFIGURED`.
    pub async fn set_output_root_config(
        &self,
        input: &SetOutputRootConfig,
    ) -> AtelierResult<OutputRootConfig> {
        if input.configured_root.trim().is_empty() {
            return Err(AtelierError::Validation(
                "configured_root must not be empty".into(),
            ));
        }
        reject_legacy_runtime_ref("configured_root", &input.configured_root)?;
        reject_legacy_runtime_refs_in_json("per_mode_subdirs", &input.per_mode_subdirs)?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_md_output_root
                 (configured_root, materialization_mode, per_mode_subdirs)
               VALUES ($1, $2, $3)
               ON CONFLICT (configured_root) DO UPDATE
                 SET materialization_mode = EXCLUDED.materialization_mode,
                     per_mode_subdirs     = EXCLUDED.per_mode_subdirs,
                     updated_at_utc       = NOW()
               RETURNING {OUTPUT_ROOT_COLUMNS}"#
        ))
        .bind(&input.configured_root)
        .bind(input.materialization_mode.as_token())
        .bind(&input.per_mode_subdirs)
        .fetch_one(self.pool())
        .await?;
        let config = output_root_from_row(&row)?;

        self.record_event(
            OUTPUT_ROOT_CONFIGURED,
            "atelier_md_output_root",
            &config.root_id.to_string(),
            serde_json::json!({
                "root_id": config.root_id,
                "configured_root": config.configured_root,
                "materialization_mode": config.materialization_mode.as_token(),
            }),
        )
        .await?;
        Ok(config)
    }

    /// Fetch an output-root config by id.
    pub async fn get_output_root_config(&self, root_id: Uuid) -> AtelierResult<OutputRootConfig> {
        let row = sqlx::query(&format!(
            "SELECT {OUTPUT_ROOT_COLUMNS} FROM atelier_md_output_root WHERE root_id = $1"
        ))
        .bind(root_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("output_root_id={root_id}")))?;
        output_root_from_row(&row)
    }

    // -----------------------------------------------------------------------
    // MdAllowlistPolicyV2 (6.10.4)
    // -----------------------------------------------------------------------

    /// Set (create or update) an allowlist policy keyed by `name` (Section
    /// 6.10.4 LAW-MDV2-CAP-001). `max_pages` is clamped to the hard cap 5000
    /// (10.14.9). Emits `ALLOWLIST_POLICY_SET`.
    pub async fn set_allowlist_policy(
        &self,
        input: &SetAllowlistPolicy,
    ) -> AtelierResult<AllowlistPolicy> {
        if input.name.trim().is_empty() {
            return Err(AtelierError::Validation(
                "allowlist policy name must not be empty".into(),
            ));
        }
        if input.default_decision != "deny" && input.default_decision != "allow" {
            return Err(AtelierError::Validation(format!(
                "default_decision must be 'deny' or 'allow', got {:?}",
                input.default_decision
            )));
        }
        let max_pages = input.max_pages.clamp(1, 5000);

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_md_allowlist_policy
                 (name, allowed_domains, explicit_url_lists, default_decision,
                  rate_limit, max_pages, robots_posture)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (name) DO UPDATE
                 SET allowed_domains    = EXCLUDED.allowed_domains,
                     explicit_url_lists = EXCLUDED.explicit_url_lists,
                     default_decision   = EXCLUDED.default_decision,
                     rate_limit         = EXCLUDED.rate_limit,
                     max_pages          = EXCLUDED.max_pages,
                     robots_posture     = EXCLUDED.robots_posture,
                     updated_at_utc     = NOW()
               RETURNING {ALLOWLIST_COLUMNS}"#
        ))
        .bind(&input.name)
        .bind(&input.allowed_domains)
        .bind(&input.explicit_url_lists)
        .bind(&input.default_decision)
        .bind(&input.rate_limit)
        .bind(max_pages)
        .bind(&input.robots_posture)
        .fetch_one(self.pool())
        .await?;
        let policy = allowlist_from_row(&row);

        self.record_event(
            ALLOWLIST_POLICY_SET,
            "atelier_md_allowlist_policy",
            &policy.allowlist_policy_id.to_string(),
            serde_json::json!({
                "allowlist_policy_id": policy.allowlist_policy_id,
                "name": policy.name,
                "default_decision": policy.default_decision,
                "max_pages": policy.max_pages,
                "robots_posture": policy.robots_posture,
            }),
        )
        .await?;
        Ok(policy)
    }

    /// Fetch an allowlist policy by id.
    pub async fn get_allowlist_policy(
        &self,
        allowlist_policy_id: Uuid,
    ) -> AtelierResult<AllowlistPolicy> {
        let row = sqlx::query(&format!(
            "SELECT {ALLOWLIST_COLUMNS} FROM atelier_md_allowlist_policy WHERE allowlist_policy_id = $1"
        ))
        .bind(allowlist_policy_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("allowlist_policy_id={allowlist_policy_id}"))
        })?;
        Ok(allowlist_from_row(&row))
    }

    // -----------------------------------------------------------------------
    // MdAuthContextV2 (6.10.4 LAW-MDV2-AUTH-001/002)
    // -----------------------------------------------------------------------

    /// Register (create or update) a redacted auth context keyed by `label`
    /// (Section 6.10.4). All auth material is carried by reference; inline secret
    /// material is rejected before persistence so a raw secret can never reach a
    /// record or event (LAW-MDV2-AUTH-001/002). The event payload carries the
    /// `auth_context_ref` and mode only, never any ref contents. Emits
    /// `AUTH_CONTEXT_REGISTERED`.
    pub async fn register_auth_context(
        &self,
        input: &RegisterAuthContext,
    ) -> AtelierResult<AuthContext> {
        if input.label.trim().is_empty() {
            return Err(AtelierError::Validation(
                "auth context label must not be empty".into(),
            ));
        }

        // Redaction boundary: reject any field that smells like inline secrets.
        if let Some(session_ref) = &input.session_ref {
            reject_inline_secret("session_ref", session_ref)?;
            reject_legacy_runtime_ref("session_ref", session_ref)?;
        }
        if let Some(jar) = &input.cookie_jar_artifact_ref {
            reject_inline_secret("cookie_jar_artifact_ref", jar)?;
            reject_legacy_runtime_ref("cookie_jar_artifact_ref", jar)?;
        }
        // header_secret_refs must be an array of reference strings, never inline
        // header values.
        match &input.header_secret_refs {
            serde_json::Value::Array(items) => {
                for entry in items {
                    if let Some(text) = entry.as_str() {
                        reject_inline_secret("header_secret_refs", text)?;
                        reject_legacy_runtime_ref("header_secret_refs", text)?;
                    } else {
                        return Err(AtelierError::Validation(
                            "header_secret_refs entries must be reference strings".into(),
                        ));
                    }
                }
            }
            serde_json::Value::Null => {}
            _ => {
                return Err(AtelierError::Validation(
                    "header_secret_refs must be a JSON array of reference strings".into(),
                ));
            }
        }

        // Mode/field consistency (LAW-MDV2-AUTH-001): a mode must carry its ref.
        match input.auth_mode {
            AuthMode::Session if input.session_ref.is_none() => {
                return Err(AtelierError::Validation(
                    "auth_mode=session requires session_ref".into(),
                ));
            }
            AuthMode::CookieJar if input.cookie_jar_artifact_ref.is_none() => {
                return Err(AtelierError::Validation(
                    "auth_mode=cookie_jar requires cookie_jar_artifact_ref".into(),
                ));
            }
            _ => {}
        }

        let header_refs = if input.header_secret_refs.is_null() {
            serde_json::json!([])
        } else {
            input.header_secret_refs.clone()
        };

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_md_auth_context
                 (label, auth_mode, session_ref, cookie_jar_artifact_ref, header_secret_refs)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (label) DO UPDATE
                 SET auth_mode               = EXCLUDED.auth_mode,
                     session_ref             = EXCLUDED.session_ref,
                     cookie_jar_artifact_ref = EXCLUDED.cookie_jar_artifact_ref,
                     header_secret_refs      = EXCLUDED.header_secret_refs,
                     updated_at_utc          = NOW()
               RETURNING {AUTH_COLUMNS}"#
        ))
        .bind(&input.label)
        .bind(input.auth_mode.as_token())
        .bind(&input.session_ref)
        .bind(&input.cookie_jar_artifact_ref)
        .bind(&header_refs)
        .fetch_one(self.pool())
        .await?;
        let context = auth_from_row(&row)?;

        // Event payload carries refs/mode only; never any secret value. The
        // header-ref COUNT is surfaced, not the refs themselves.
        let header_ref_count = context
            .header_secret_refs
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        self.record_event(
            AUTH_CONTEXT_REGISTERED,
            "atelier_md_auth_context",
            &context.auth_context_ref.to_string(),
            serde_json::json!({
                "auth_context_ref": context.auth_context_ref,
                "label": context.label,
                "auth_mode": context.auth_mode.as_token(),
                "has_session_ref": context.session_ref.is_some(),
                "has_cookie_jar": context.cookie_jar_artifact_ref.is_some(),
                "header_secret_ref_count": header_ref_count,
                "secret_values": REDACTED_PLACEHOLDER,
            }),
        )
        .await?;
        Ok(context)
    }

    /// Fetch an auth context by ref. Auth material remains by-reference; no
    /// secret value is stored to return.
    pub async fn get_auth_context(&self, auth_context_ref: Uuid) -> AtelierResult<AuthContext> {
        let row = sqlx::query(&format!(
            "SELECT {AUTH_COLUMNS} FROM atelier_md_auth_context WHERE auth_context_ref = $1"
        ))
        .bind(auth_context_ref)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("auth_context_ref={auth_context_ref}")))?;
        auth_from_row(&row)
    }

    // -----------------------------------------------------------------------
    // MdDownloadSessionV2 (6.10.3)
    // -----------------------------------------------------------------------

    /// Open a staged download session, or return the existing one for the same
    /// `idempotency_key` (Section 6.10.3). FK targets (allowlist policy, output
    /// root, optional auth context) are validated so a session never dangles.
    /// Sessions start in the `resolving` stage. Emits `SESSION_OPENED`.
    pub async fn open_download_session(
        &self,
        input: &OpenDownloadSession,
    ) -> AtelierResult<DownloadSession> {
        if input.idempotency_key.trim().is_empty() {
            return Err(AtelierError::Validation(
                "idempotency_key must not be empty".into(),
            ));
        }
        if input.parent_job_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "parent_job_id must not be empty".into(),
            ));
        }
        let required_capabilities = validate_media_downloader_capability_grant(
            &input.protocol_id,
            &input.capability_profile_id,
            &input.capability_grant_ref,
        )?;

        // Idempotent fast path.
        if let Some(existing) = self
            .get_download_session_by_key(&input.idempotency_key)
            .await?
        {
            if existing.protocol_id.as_str() != input.protocol_id.as_str()
                || existing.capability_profile_id.as_str() != input.capability_profile_id.as_str()
                || existing.capability_grant_ref.as_str() != input.capability_grant_ref.as_str()
            {
                return Err(AtelierError::Validation(
                    "idempotency_key is already bound to a different media_downloader capability grant"
                        .into(),
                ));
            }
            return Ok(existing);
        }

        // Guard FK targets explicitly for clean validation errors.
        let _ = self.get_allowlist_policy(input.allowlist_policy_id).await?;
        let _ = self.get_output_root_config(input.output_root_id).await?;
        if let Some(auth_ref) = input.auth_context_ref {
            let _ = self.get_auth_context(auth_ref).await?;
        }

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_md_download_session
                 (parent_job_id, idempotency_key, source_kind, auth_context_ref,
                  allowlist_policy_id, output_root_id, protocol_id,
                  capability_profile_id, capability_grant_ref, stage)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'resolving')
               ON CONFLICT (idempotency_key) DO UPDATE
                 SET idempotency_key = EXCLUDED.idempotency_key
               RETURNING {SESSION_COLUMNS}"#
        ))
        .bind(&input.parent_job_id)
        .bind(&input.idempotency_key)
        .bind(input.source_kind.as_token())
        .bind(input.auth_context_ref)
        .bind(input.allowlist_policy_id)
        .bind(input.output_root_id)
        .bind(&input.protocol_id)
        .bind(&input.capability_profile_id)
        .bind(&input.capability_grant_ref)
        .fetch_one(self.pool())
        .await?;
        let session = session_from_row(&row)?;

        self.record_event(
            SESSION_OPENED,
            "atelier_md_download_session",
            &session.session_id.to_string(),
            serde_json::json!({
                "session_id": session.session_id,
                "parent_job_id": session.parent_job_id,
                "source_kind": session.source_kind.as_token(),
                "allowlist_policy_id": session.allowlist_policy_id,
                "output_root_id": session.output_root_id,
                "auth_context_ref": session.auth_context_ref,
                "protocol_id": session.protocol_id.clone(),
                "capability_profile_id": session.capability_profile_id.clone(),
                "capability_grant_ref": session.capability_grant_ref.clone(),
                "required_capabilities": required_capabilities,
                "stage": session.stage.as_token(),
            }),
        )
        .await?;
        self.record_event(
            MEDIA_DOWNLOADER_JOB_STATE,
            "atelier_md_download_session",
            &session.session_id.to_string(),
            serde_json::json!({
                "session_id": session.session_id,
                "source_kind": session.source_kind.as_token(),
                "state": session.stage.as_token(),
                "stage": session.stage.as_token(),
                "is_terminal": session.stage.is_terminal(),
                "protocol_id": session.protocol_id.clone(),
                "capability_profile_id": session.capability_profile_id.clone(),
            }),
        )
        .await?;
        Ok(session)
    }

    /// Fetch a session by its stable idempotency key.
    pub async fn get_download_session_by_key(
        &self,
        idempotency_key: &str,
    ) -> AtelierResult<Option<DownloadSession>> {
        let row = sqlx::query(&format!(
            "SELECT {SESSION_COLUMNS} FROM atelier_md_download_session WHERE idempotency_key = $1"
        ))
        .bind(idempotency_key)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(session_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// Fetch a session by id.
    pub async fn get_download_session(&self, session_id: Uuid) -> AtelierResult<DownloadSession> {
        let row = sqlx::query(&format!(
            "SELECT {SESSION_COLUMNS} FROM atelier_md_download_session WHERE session_id = $1"
        ))
        .bind(session_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("session_id={session_id}")))?;
        session_from_row(&row)
    }

    /// Advance a session to a new stage (Section 6.10.3 staged lifecycle). Every
    /// transition records a session-level checkpoint in the same transaction so
    /// the recovery anchor invariant (LAW-MDV2-RESUME-003: a stage transition
    /// without a checkpoint is a violation) holds. Emits `SESSION_STAGE_CHANGED`.
    pub async fn advance_session_stage(
        &self,
        session_id: Uuid,
        stage: SessionStage,
        resume_token: Option<&str>,
    ) -> AtelierResult<DownloadSession> {
        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(&format!(
            r#"UPDATE atelier_md_download_session
               SET stage = $2, updated_at_utc = NOW()
               WHERE session_id = $1
               RETURNING {SESSION_COLUMNS}"#
        ))
        .bind(session_id)
        .bind(stage.as_token())
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("session_id={session_id}")))?;
        let session = session_from_row(&row)?;

        // Bundled session-level checkpoint (recovery anchor).
        sqlx::query(
            r#"INSERT INTO atelier_md_checkpoint
                 (session_id, item_id, stage, bytes_downloaded, bytes_total, resume_token)
               VALUES ($1, NULL, $2, 0, NULL, $3)"#,
        )
        .bind(session_id)
        .bind(stage.as_token())
        .bind(resume_token)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.record_event(
            SESSION_STAGE_CHANGED,
            "atelier_md_download_session",
            &session.session_id.to_string(),
            serde_json::json!({
                "session_id": session.session_id,
                "stage": session.stage.as_token(),
                "is_terminal": session.stage.is_terminal(),
            }),
        )
        .await?;
        self.record_event(
            MEDIA_DOWNLOADER_JOB_STATE,
            "atelier_md_download_session",
            &session.session_id.to_string(),
            serde_json::json!({
                "session_id": session.session_id,
                "source_kind": session.source_kind.as_token(),
                "state": session.stage.as_token(),
                "stage": session.stage.as_token(),
                "is_terminal": session.stage.is_terminal(),
                "protocol_id": session.protocol_id.clone(),
                "capability_profile_id": session.capability_profile_id.clone(),
            }),
        )
        .await?;
        Ok(session)
    }

    // -----------------------------------------------------------------------
    // MdItemStateV2 (6.10.3)
    // -----------------------------------------------------------------------

    /// Enqueue an item into a session, idempotently on `(session, normalized_url)`
    /// (Section 6.10.3). Re-enqueuing the same normalized URL returns the existing
    /// item without resetting its progress, so dedupe across runs is safe
    /// (LAW-MDV2-RESUME-004). Items enter the `enqueued` stage. Emits
    /// `ITEM_ENQUEUED`.
    pub async fn enqueue_item(
        &self,
        session_id: Uuid,
        input: &EnqueueItem,
    ) -> AtelierResult<ItemState> {
        if input.normalized_url.trim().is_empty() {
            return Err(AtelierError::Validation(
                "normalized_url must not be empty".into(),
            ));
        }

        // Idempotent fast path.
        if let Some(existing) = self
            .get_item_by_url(session_id, &input.normalized_url)
            .await?
        {
            return Ok(existing);
        }

        let mut tx = self.pool().begin().await?;

        // Guard the FK so a bad session_id is a clean validation error.
        let session_exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT session_id FROM atelier_md_download_session WHERE session_id = $1",
        )
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await?;
        if session_exists.is_none() {
            return Err(AtelierError::NotFound(format!("session_id={session_id}")));
        }

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_md_item_state
                 (session_id, normalized_url, stable_source_id, stage)
               VALUES ($1, $2, $3, 'enqueued')
               ON CONFLICT (session_id, normalized_url) DO UPDATE
                 SET normalized_url = EXCLUDED.normalized_url
               RETURNING {ITEM_COLUMNS}"#
        ))
        .bind(session_id)
        .bind(&input.normalized_url)
        .bind(&input.stable_source_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE atelier_md_download_session SET updated_at_utc = NOW() WHERE session_id = $1",
        )
        .bind(session_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let item = item_from_row(&row)?;
        self.record_event(
            ITEM_ENQUEUED,
            "atelier_md_item_state",
            &item.item_id.to_string(),
            serde_json::json!({
                "session_id": item.session_id,
                "item_id": item.item_id,
                "normalized_url": item.normalized_url,
                "stable_source_id": item.stable_source_id,
            }),
        )
        .await?;
        Ok(item)
    }

    /// Fetch an item by its normalized URL within a session.
    pub async fn get_item_by_url(
        &self,
        session_id: Uuid,
        normalized_url: &str,
    ) -> AtelierResult<Option<ItemState>> {
        let row = sqlx::query(&format!(
            r#"SELECT {ITEM_COLUMNS} FROM atelier_md_item_state
               WHERE session_id = $1 AND normalized_url = $2"#
        ))
        .bind(session_id)
        .bind(normalized_url)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(item_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// List items in a session in enqueue order, optionally filtered by stage.
    pub async fn list_session_items(
        &self,
        session_id: Uuid,
        stage: Option<ItemStage>,
    ) -> AtelierResult<Vec<ItemState>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {ITEM_COLUMNS} FROM atelier_md_item_state
               WHERE session_id = $1 AND ($2::TEXT IS NULL OR stage = $2)
               ORDER BY created_at_utc ASC"#
        ))
        .bind(session_id)
        .bind(stage.map(|s| s.as_token()))
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(item_from_row).collect()
    }

    // -----------------------------------------------------------------------
    // MdCheckpointV2 (6.10.3 LAW-MDV2-RESUME-003)
    // -----------------------------------------------------------------------

    /// Record a resumable checkpoint and advance the item's progress/resume
    /// cursor (Section 6.10.3 MdCheckpointV2). One call appends an immutable
    /// checkpoint row AND updates the live item's `bytes_downloaded`,
    /// `bytes_total`, `resume_token`, and `stage`, inside a single transaction so
    /// the live state and the recovery anchor never diverge. On resume the
    /// Workflow-Engine job reads the latest checkpoint plus the `.part` artifact
    /// to continue from the recorded offset (LAW-MDV2-RESUME-002). Emits
    /// `ITEM_CHECKPOINTED`.
    pub async fn record_checkpoint(
        &self,
        session_id: Uuid,
        input: &RecordCheckpoint,
    ) -> AtelierResult<Checkpoint> {
        // Validate the stage token is one of the known item stages so a corrupt
        // value cannot enter the recovery anchor.
        let item_stage = ItemStage::from_token(&input.stage)?;

        let mut tx = self.pool().begin().await?;

        // Guard the session FK.
        let session_exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT session_id FROM atelier_md_download_session WHERE session_id = $1",
        )
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await?;
        if session_exists.is_none() {
            return Err(AtelierError::NotFound(format!("session_id={session_id}")));
        }

        // If item-scoped, advance the live item state in the same transaction.
        if let Some(item_id) = input.item_id {
            let updated: Option<Uuid> = sqlx::query_scalar(
                r#"UPDATE atelier_md_item_state
                   SET stage           = $3,
                       bytes_downloaded = $4,
                       bytes_total      = $5,
                       resume_token     = $6,
                       updated_at_utc   = NOW()
                   WHERE item_id = $1 AND session_id = $2
                   RETURNING item_id"#,
            )
            .bind(item_id)
            .bind(session_id)
            .bind(&input.stage)
            .bind(input.bytes_downloaded)
            .bind(input.bytes_total)
            .bind(&input.resume_token)
            .fetch_optional(&mut *tx)
            .await?;
            if updated.is_none() {
                return Err(AtelierError::NotFound(format!(
                    "item_id={item_id} in session_id={session_id}"
                )));
            }
        }

        let row = sqlx::query(
            r#"INSERT INTO atelier_md_checkpoint
                 (session_id, item_id, stage, bytes_downloaded, bytes_total, resume_token)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING checkpoint_id, session_id, item_id, stage, bytes_downloaded,
                         bytes_total, resume_token, created_at_utc"#,
        )
        .bind(session_id)
        .bind(input.item_id)
        .bind(&input.stage)
        .bind(input.bytes_downloaded)
        .bind(input.bytes_total)
        .bind(&input.resume_token)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let checkpoint = checkpoint_from_row(&row);
        self.record_event(
            ITEM_CHECKPOINTED,
            "atelier_md_checkpoint",
            &checkpoint.checkpoint_id.to_string(),
            serde_json::json!({
                "session_id": checkpoint.session_id,
                "item_id": checkpoint.item_id,
                "stage": checkpoint.stage.clone(),
                "bytes_downloaded": checkpoint.bytes_downloaded,
                "bytes_total": checkpoint.bytes_total,
                "has_resume_token": checkpoint.resume_token.is_some(),
            }),
        )
        .await?;
        let telemetry_aggregate_type = if checkpoint.item_id.is_some() {
            "atelier_md_item_state"
        } else {
            "atelier_md_download_session"
        };
        let telemetry_aggregate_id = checkpoint
            .item_id
            .unwrap_or(checkpoint.session_id)
            .to_string();
        self.record_event(
            MEDIA_DOWNLOADER_PROGRESS,
            telemetry_aggregate_type,
            &telemetry_aggregate_id,
            serde_json::json!({
                "session_id": checkpoint.session_id,
                "item_id": checkpoint.item_id,
                "stage": checkpoint.stage,
                "progress": {
                    "bytes_downloaded": checkpoint.bytes_downloaded,
                    "bytes_total": checkpoint.bytes_total,
                },
            }),
        )
        .await?;
        if let Some(item_id) = checkpoint.item_id {
            let result = match item_stage {
                ItemStage::Finalized => Some("succeeded"),
                ItemStage::Skipped => Some("skipped"),
                ItemStage::Failed => Some("failed"),
                _ => None,
            };
            if let Some(result) = result {
                self.record_event(
                    MEDIA_DOWNLOADER_ITEM_RESULT,
                    "atelier_md_item_state",
                    &item_id.to_string(),
                    serde_json::json!({
                        "session_id": checkpoint.session_id,
                        "item_id": item_id,
                        "stage": item_stage.as_token(),
                        "result": result,
                        "progress": {
                            "bytes_downloaded": checkpoint.bytes_downloaded,
                            "bytes_total": checkpoint.bytes_total,
                        },
                    }),
                )
                .await?;
            }
        }
        Ok(checkpoint)
    }

    /// The latest checkpoint for an item (or session-level when `item_id` is
    /// None), used as the resume anchor after a process restart.
    pub async fn latest_checkpoint(
        &self,
        session_id: Uuid,
        item_id: Option<Uuid>,
    ) -> AtelierResult<Option<Checkpoint>> {
        let row = sqlx::query(
            r#"SELECT checkpoint_id, session_id, item_id, stage, bytes_downloaded,
                      bytes_total, resume_token, created_at_utc
               FROM atelier_md_checkpoint
               WHERE session_id = $1
                 AND item_id IS NOT DISTINCT FROM $2::uuid
               ORDER BY created_at_utc DESC
               LIMIT 1"#,
        )
        .bind(session_id)
        .bind(item_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(checkpoint_from_row))
    }

    // -----------------------------------------------------------------------
    // MdSessionReceiptV2 (6.10.5 LAW-MDV2-TEL-003)
    // -----------------------------------------------------------------------

    /// Emit a recoverable session receipt at finalize/fail/cancel (Section
    /// 6.10.5). Idempotent on `(session_id, terminal_stage)`: re-emitting the
    /// same terminal receipt returns the existing one rather than duplicating.
    /// The receipt denormalizes session provenance (parent job, source kind,
    /// auth ref, allowlist, output root) so it stays a self-contained replay
    /// unit, and carries NO secret material (auth by ref only). Emits
    /// `SESSION_RECEIPT_EMITTED`.
    pub async fn emit_session_receipt(
        &self,
        session_id: Uuid,
        input: &EmitSessionReceipt,
    ) -> AtelierResult<SessionReceipt> {
        // Denormalize from the session so the receipt is self-contained.
        let session = self.get_download_session(session_id).await?;
        reject_legacy_runtime_refs_in_json("materialized_paths", &input.materialized_paths)?;
        if let Some(manifest_artifact_ref) = &input.manifest_artifact_ref {
            reject_legacy_runtime_ref("manifest_artifact_ref", manifest_artifact_ref)?;
        }

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_md_session_receipt
                 (session_id, parent_job_id, source_kind, auth_context_ref,
                  allowlist_policy_id, output_root_id, item_count, succeeded, failed,
                  skipped_deduped, materialized_paths, manifest_artifact_ref,
                  started_at_utc, ended_at_utc, terminal_stage)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
               ON CONFLICT (session_id, terminal_stage) DO UPDATE
                 SET item_count = EXCLUDED.item_count
               RETURNING {RECEIPT_COLUMNS}"#
        ))
        .bind(session_id)
        .bind(&session.parent_job_id)
        .bind(session.source_kind.as_token())
        .bind(session.auth_context_ref)
        .bind(session.allowlist_policy_id)
        .bind(session.output_root_id)
        .bind(input.item_count)
        .bind(input.succeeded)
        .bind(input.failed)
        .bind(input.skipped_deduped)
        .bind(&input.materialized_paths)
        .bind(&input.manifest_artifact_ref)
        .bind(input.started_at_utc)
        .bind(input.ended_at_utc)
        .bind(input.terminal_stage.as_token())
        .fetch_one(self.pool())
        .await?;
        let receipt = receipt_from_row(&row)?;

        self.record_event(
            SESSION_RECEIPT_EMITTED,
            "atelier_md_session_receipt",
            &receipt.receipt_id.to_string(),
            serde_json::json!({
                "receipt_id": receipt.receipt_id,
                "session_id": receipt.session_id,
                "parent_job_id": receipt.parent_job_id,
                "source_kind": receipt.source_kind.as_token(),
                "item_count": receipt.item_count,
                "succeeded": receipt.succeeded,
                "failed": receipt.failed,
                "skipped_deduped": receipt.skipped_deduped,
                "terminal_stage": receipt.terminal_stage.as_token(),
                // Auth carried by ref only; never a secret value.
                "auth_context_ref": receipt.auth_context_ref,
                "secret_values": REDACTED_PLACEHOLDER,
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// Fetch a session's terminal receipt for a given terminal stage, if emitted.
    pub async fn get_session_receipt(
        &self,
        session_id: Uuid,
        terminal_stage: TerminalStage,
    ) -> AtelierResult<Option<SessionReceipt>> {
        let row = sqlx::query(&format!(
            r#"SELECT {RECEIPT_COLUMNS} FROM atelier_md_session_receipt
               WHERE session_id = $1 AND terminal_stage = $2"#
        ))
        .bind(session_id)
        .bind(terminal_stage.as_token())
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(receipt_from_row(&r)?)),
            None => Ok(None),
        }
    }
}
