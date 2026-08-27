//! Media-downloader-v2 governed records (MT-204, WP-KERNEL-005 legacy source fold-in).
//!
//! Spec authority: master-spec-v02.189 Section 6.10 "Media Downloader v2 Depth"
//! (6.10.2 OutputRootConfigV2, 6.10.3 MdDownloadSessionV2 + MdItemStateV2 staged
//! resumable sessions + checkpoints, 6.10.4 MdAuthContextV2 + MdAllowlistPolicyV2,
//! 6.10.5 MdSessionReceiptV2 sanitized telemetry/receipts).
//!
//! legacy source (intent only): legacy source `app backend Media-Downloader-v2`.
//! The SQLite/Electron/localhost/polling originals are NOT copied; only the
//! governed DATA + RECEIPT contract is translated. Storage authority is the
//! single Handshake store + EventLedger only (see
//! the embedded SurrealDB authority and EventLedger only (MT-004/MT-138).
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
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{
    atelier_event_sql, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
};

fn stable_downloader_uuid(kind: &str, natural_key: &str) -> Uuid {
    let digest = Sha256::digest(format!("atelier.downloader:{kind}:{natural_key}").as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

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

#[derive(SurrealValue)]
struct OutputRootRow {
    root_id: SurrealUuid,
    configured_root: String,
    materialization_mode: String,
    per_mode_subdirs: serde_json::Value,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}
fn output_root_from_row(row: OutputRootRow) -> AtelierResult<OutputRootConfig> {
    Ok(OutputRootConfig {
        root_id: row.root_id.into(),
        configured_root: row.configured_root,
        materialization_mode: MaterializationMode::from_token(&row.materialization_mode)?,
        per_mode_subdirs: row.per_mode_subdirs,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

#[derive(SurrealValue)]
struct AllowlistRow {
    allowlist_policy_id: SurrealUuid,
    name: String,
    allowed_domains: serde_json::Value,
    explicit_url_lists: serde_json::Value,
    default_decision: String,
    rate_limit: serde_json::Value,
    max_pages: i64,
    robots_posture: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}
fn allowlist_from_row(row: AllowlistRow) -> AllowlistPolicy {
    AllowlistPolicy {
        allowlist_policy_id: row.allowlist_policy_id.into(),
        name: row.name,
        allowed_domains: row.allowed_domains,
        explicit_url_lists: row.explicit_url_lists,
        default_decision: row.default_decision,
        rate_limit: row.rate_limit,
        max_pages: row.max_pages,
        robots_posture: row.robots_posture,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    }
}

#[derive(SurrealValue)]
struct AuthRow {
    auth_context_ref: SurrealUuid,
    label: String,
    auth_mode: String,
    session_ref: Option<String>,
    cookie_jar_artifact_ref: Option<String>,
    header_secret_refs: serde_json::Value,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}
fn auth_from_row(row: AuthRow) -> AtelierResult<AuthContext> {
    Ok(AuthContext {
        auth_context_ref: row.auth_context_ref.into(),
        label: row.label,
        auth_mode: AuthMode::from_token(&row.auth_mode)?,
        session_ref: row.session_ref,
        cookie_jar_artifact_ref: row.cookie_jar_artifact_ref,
        header_secret_refs: row.header_secret_refs,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

#[derive(SurrealValue)]
struct SessionRow {
    session_id: SurrealUuid,
    parent_job_id: String,
    idempotency_key: String,
    source_kind: String,
    auth_context_ref: Option<SurrealUuid>,
    allowlist_policy_id: SurrealUuid,
    output_root_id: SurrealUuid,
    protocol_id: String,
    capability_profile_id: String,
    capability_grant_ref: String,
    stage: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}
fn session_from_row(row: SessionRow) -> AtelierResult<DownloadSession> {
    Ok(DownloadSession {
        session_id: row.session_id.into(),
        parent_job_id: row.parent_job_id,
        idempotency_key: row.idempotency_key,
        source_kind: SourceKind::from_token(&row.source_kind)?,
        auth_context_ref: row.auth_context_ref.map(Into::into),
        allowlist_policy_id: row.allowlist_policy_id.into(),
        output_root_id: row.output_root_id.into(),
        protocol_id: row.protocol_id,
        capability_profile_id: row.capability_profile_id,
        capability_grant_ref: row.capability_grant_ref,
        stage: SessionStage::from_token(&row.stage)?,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

#[derive(SurrealValue)]
struct ItemRow {
    item_id: SurrealUuid,
    session_id: SurrealUuid,
    normalized_url: String,
    stable_source_id: Option<String>,
    content_hash: Option<String>,
    stage: String,
    bytes_downloaded: i64,
    bytes_total: Option<i64>,
    part_path_ref: Option<String>,
    attempt_count: i64,
    last_error_code: Option<String>,
    resume_token: Option<String>,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}
fn item_from_row(row: ItemRow) -> AtelierResult<ItemState> {
    Ok(ItemState {
        item_id: row.item_id.into(),
        session_id: row.session_id.into(),
        normalized_url: row.normalized_url,
        stable_source_id: row.stable_source_id,
        content_hash: row.content_hash,
        stage: ItemStage::from_token(&row.stage)?,
        bytes_downloaded: row.bytes_downloaded,
        bytes_total: row.bytes_total,
        part_path_ref: row.part_path_ref,
        attempt_count: row.attempt_count,
        last_error_code: row.last_error_code,
        resume_token: row.resume_token,
        created_at_utc: row.created_at_utc.into(),
        updated_at_utc: row.updated_at_utc.into(),
    })
}

#[derive(SurrealValue)]
struct CheckpointRow {
    checkpoint_id: SurrealUuid,
    session_id: SurrealUuid,
    item_id: Option<SurrealUuid>,
    stage: String,
    bytes_downloaded: i64,
    bytes_total: Option<i64>,
    resume_token: Option<String>,
    created_at_utc: Datetime,
}
fn checkpoint_from_row(row: CheckpointRow) -> Checkpoint {
    Checkpoint {
        checkpoint_id: row.checkpoint_id.into(),
        session_id: row.session_id.into(),
        item_id: row.item_id.map(Into::into),
        stage: row.stage,
        bytes_downloaded: row.bytes_downloaded,
        bytes_total: row.bytes_total,
        resume_token: row.resume_token,
        created_at_utc: row.created_at_utc.into(),
    }
}

#[derive(SurrealValue)]
struct ReceiptRow {
    receipt_id: SurrealUuid,
    session_id: SurrealUuid,
    parent_job_id: String,
    source_kind: String,
    auth_context_ref: Option<SurrealUuid>,
    allowlist_policy_id: SurrealUuid,
    output_root_id: SurrealUuid,
    item_count: i64,
    succeeded: i64,
    failed: i64,
    skipped_deduped: i64,
    materialized_paths: serde_json::Value,
    manifest_artifact_ref: Option<String>,
    started_at_utc: Option<Datetime>,
    ended_at_utc: Option<Datetime>,
    terminal_stage: String,
    created_at_utc: Datetime,
}
fn receipt_from_row(row: ReceiptRow) -> AtelierResult<SessionReceipt> {
    Ok(SessionReceipt {
        receipt_id: row.receipt_id.into(),
        session_id: row.session_id.into(),
        parent_job_id: row.parent_job_id,
        source_kind: SourceKind::from_token(&row.source_kind)?,
        auth_context_ref: row.auth_context_ref.map(Into::into),
        allowlist_policy_id: row.allowlist_policy_id.into(),
        output_root_id: row.output_root_id.into(),
        item_count: row.item_count,
        succeeded: row.succeeded,
        failed: row.failed,
        skipped_deduped: row.skipped_deduped,
        materialized_paths: row.materialized_paths,
        manifest_artifact_ref: row.manifest_artifact_ref,
        started_at_utc: row.started_at_utc.map(Into::into),
        ended_at_utc: row.ended_at_utc.map(Into::into),
        terminal_stage: TerminalStage::from_token(&row.terminal_stage)?,
        created_at_utc: row.created_at_utc.into(),
    })
}

#[derive(Clone, SurrealValue)]
struct UuidBinding {
    value: SurrealUuid,
}
#[derive(Clone, SurrealValue)]
struct StringBinding {
    value: String,
}
#[derive(Clone, SurrealValue)]
struct SessionUrlBinding {
    download_session: RecordId,
    normalized_url: String,
}
#[derive(Clone, SurrealValue)]
struct OutputRootWrite {
    record: RecordId,
    root_id: SurrealUuid,
    configured_root: String,
    materialization_mode: String,
    per_mode_subdirs: serde_json::Value,
}
#[derive(Clone, SurrealValue)]
struct AllowlistWrite {
    record: RecordId,
    allowlist_policy_id: SurrealUuid,
    name: String,
    allowed_domains: serde_json::Value,
    explicit_url_lists: serde_json::Value,
    default_decision: String,
    rate_limit: serde_json::Value,
    max_pages: i64,
    robots_posture: String,
}
#[derive(Clone, SurrealValue)]
struct AuthWrite {
    record: RecordId,
    auth_context_ref: SurrealUuid,
    label: String,
    auth_mode: String,
    session_ref: Option<String>,
    cookie_jar_artifact_ref: Option<String>,
    header_secret_refs: serde_json::Value,
}
#[derive(Clone, SurrealValue)]
struct SessionWrite {
    record: RecordId,
    session_id: SurrealUuid,
    parent_job_id: String,
    idempotency_key: String,
    source_kind: String,
    auth_context_ref: Option<RecordId>,
    allowlist_policy_id: RecordId,
    output_root_id: RecordId,
    protocol_id: String,
    capability_profile_id: String,
    capability_grant_ref: String,
}
#[derive(Clone, SurrealValue)]
struct StageWrite {
    download_session: RecordId,
    checkpoint: RecordId,
    checkpoint_id: SurrealUuid,
    stage: String,
    resume_token: Option<String>,
}
#[derive(Clone, SurrealValue)]
struct ItemWrite {
    record: RecordId,
    item_id: SurrealUuid,
    download_session: RecordId,
    normalized_url: String,
    stable_source_id: Option<String>,
}
#[derive(Clone, SurrealValue)]
struct ItemListBinding {
    download_session: RecordId,
    stage: Option<String>,
}
#[derive(Clone, SurrealValue)]
struct CheckpointWrite {
    checkpoint: RecordId,
    checkpoint_id: SurrealUuid,
    download_session: RecordId,
    item: Option<RecordId>,
    stage: String,
    bytes_downloaded: i64,
    bytes_total: Option<i64>,
    resume_token: Option<String>,
}
#[derive(Clone, SurrealValue)]
struct CheckpointLookup {
    download_session: RecordId,
    item: Option<RecordId>,
}
#[derive(Clone, SurrealValue)]
struct ReceiptLookup {
    download_session: RecordId,
    terminal_stage: String,
}
#[derive(Clone, SurrealValue)]
struct ReceiptWrite {
    record: RecordId,
    receipt_id: SurrealUuid,
    session: RecordId,
    parent_job_id: String,
    source_kind: String,
    auth_context_ref: Option<RecordId>,
    allowlist_policy_id: RecordId,
    output_root_id: RecordId,
    item_count: i64,
    succeeded: i64,
    failed: i64,
    skipped_deduped: i64,
    materialized_paths: serde_json::Value,
    manifest_artifact_ref: Option<String>,
    started_at_utc: Option<Datetime>,
    ended_at_utc: Option<Datetime>,
    terminal_stage: String,
}

const WRITE_OUTPUT_ROOT: &str = concat!("RETURN { LET $row = (UPSERT $domain.record CONTENT { root_id: $domain.root_id, configured_root: $domain.configured_root, materialization_mode: $domain.materialization_mode, per_mode_subdirs: $domain.per_mode_subdirs } RETURN AFTER)[0]; ", atelier_event_sql!(), " RETURN $row; };");
const WRITE_ALLOWLIST: &str = concat!("RETURN { LET $row = (UPSERT $domain.record CONTENT { allowlist_policy_id: $domain.allowlist_policy_id, name: $domain.name, allowed_domains: $domain.allowed_domains, explicit_url_lists: $domain.explicit_url_lists, default_decision: $domain.default_decision, rate_limit: $domain.rate_limit, max_pages: $domain.max_pages, robots_posture: $domain.robots_posture } RETURN AFTER)[0]; ", atelier_event_sql!(), " RETURN $row; };");
const WRITE_AUTH: &str = concat!("RETURN { LET $row = (UPSERT $domain.record CONTENT { auth_context_ref: $domain.auth_context_ref, label: $domain.label, auth_mode: $domain.auth_mode, session_ref: $domain.session_ref, cookie_jar_artifact_ref: $domain.cookie_jar_artifact_ref, header_secret_refs: $domain.header_secret_refs } RETURN AFTER)[0]; ", atelier_event_sql!(), " RETURN $row; };");
const WRITE_SESSION: &str = concat!("RETURN { LET $existing = (SELECT protocol_id, capability_profile_id, capability_grant_ref FROM ONLY $domain.record); IF $existing != NONE AND ($existing.protocol_id != $domain.protocol_id OR $existing.capability_profile_id != $domain.capability_profile_id OR $existing.capability_grant_ref != $domain.capability_grant_ref) { THROW 'HSK-MD-IDEMPOTENCY-CAPABILITY-MISMATCH'; }; IF $existing = NONE { CREATE $domain.record CONTENT { session_id: $domain.session_id, parent_job_id: $domain.parent_job_id, idempotency_key: $domain.idempotency_key, source_kind: $domain.source_kind, auth_context_ref: $domain.auth_context_ref, allowlist_policy_id: $domain.allowlist_policy_id, output_root_id: $domain.output_root_id, protocol_id: $domain.protocol_id, capability_profile_id: $domain.capability_profile_id, capability_grant_ref: $domain.capability_grant_ref, stage: 'resolving' } RETURN NONE; }; ", atelier_event_sql!(), " RETURN (SELECT session_id, parent_job_id, idempotency_key, source_kind, IF auth_context_ref = NONE { NONE } ELSE { record::id(auth_context_ref) } AS auth_context_ref, record::id(allowlist_policy_id) AS allowlist_policy_id, record::id(output_root_id) AS output_root_id, protocol_id, capability_profile_id, capability_grant_ref, stage, created_at_utc, updated_at_utc FROM $domain.record)[0]; };");
const ADVANCE_SESSION: &str = "RETURN { LET $updated = (UPDATE $download_session SET stage = $stage, updated_at_utc = time::now() RETURN AFTER)[0]; IF $updated = NONE { RETURN NONE; }; CREATE $checkpoint CONTENT { checkpoint_id: $checkpoint_id, session_id: $download_session, item_id: NONE, stage: $stage, bytes_downloaded: 0, bytes_total: NONE, resume_token: $resume_token } RETURN NONE; RETURN (SELECT session_id, parent_job_id, idempotency_key, source_kind, IF auth_context_ref = NONE { NONE } ELSE { record::id(auth_context_ref) } AS auth_context_ref, record::id(allowlist_policy_id) AS allowlist_policy_id, record::id(output_root_id) AS output_root_id, protocol_id, capability_profile_id, capability_grant_ref, stage, created_at_utc, updated_at_utc FROM $download_session)[0]; };";
const CREATE_ITEM: &str = "RETURN { IF !record::exists($download_session) { RETURN NONE; }; LET $existing = (SELECT VALUE id FROM atelier_md_item_state WHERE session_id = $download_session AND normalized_url = $normalized_url LIMIT 1)[0]; LET $target = IF $existing = NONE { $record } ELSE { $existing }; IF $existing = NONE { CREATE $target CONTENT { item_id: $item_id, session_id: $download_session, normalized_url: $normalized_url, stable_source_id: $stable_source_id, content_hash: NONE, stage: 'enqueued', bytes_downloaded: 0, bytes_total: NONE, part_path_ref: NONE, attempt_count: 0, last_error_code: NONE, resume_token: NONE } RETURN NONE; }; UPDATE $download_session SET updated_at_utc = time::now() RETURN NONE; RETURN (SELECT item_id, record::id(session_id) AS session_id, normalized_url, stable_source_id, content_hash, stage, bytes_downloaded, bytes_total, part_path_ref, attempt_count, last_error_code, resume_token, created_at_utc, updated_at_utc FROM $target)[0]; };";
const WRITE_CHECKPOINT: &str = "RETURN { IF !record::exists($download_session) { RETURN NONE; }; IF $item != NONE { LET $updated = (UPDATE $item SET stage = $stage, bytes_downloaded = $bytes_downloaded, bytes_total = $bytes_total, resume_token = $resume_token, updated_at_utc = time::now() WHERE session_id = $download_session RETURN AFTER); IF array::len($updated) = 0 { THROW 'HSK-MD-ITEM-NOT-FOUND'; }; }; CREATE $checkpoint CONTENT { checkpoint_id: $checkpoint_id, session_id: $download_session, item_id: $item, stage: $stage, bytes_downloaded: $bytes_downloaded, bytes_total: $bytes_total, resume_token: $resume_token } RETURN NONE; RETURN (SELECT checkpoint_id, record::id(session_id) AS session_id, IF item_id = NONE { NONE } ELSE { record::id(item_id) } AS item_id, stage, bytes_downloaded, bytes_total, resume_token, created_at_utc FROM $checkpoint)[0]; };";
const WRITE_RECEIPT: &str = concat!("RETURN { IF record::exists($domain.record) { UPDATE $domain.record SET item_count = $domain.item_count RETURN NONE; } ELSE { CREATE $domain.record CONTENT { receipt_id: $domain.receipt_id, session_id: $domain.session, parent_job_id: $domain.parent_job_id, source_kind: $domain.source_kind, auth_context_ref: $domain.auth_context_ref, allowlist_policy_id: $domain.allowlist_policy_id, output_root_id: $domain.output_root_id, item_count: $domain.item_count, succeeded: $domain.succeeded, failed: $domain.failed, skipped_deduped: $domain.skipped_deduped, materialized_paths: $domain.materialized_paths, manifest_artifact_ref: $domain.manifest_artifact_ref, started_at_utc: $domain.started_at_utc, ended_at_utc: $domain.ended_at_utc, terminal_stage: $domain.terminal_stage } RETURN NONE; }; ", atelier_event_sql!(), " RETURN (SELECT receipt_id, record::id(session_id) AS session_id, parent_job_id, source_kind, IF auth_context_ref = NONE { NONE } ELSE { record::id(auth_context_ref) } AS auth_context_ref, record::id(allowlist_policy_id) AS allowlist_policy_id, record::id(output_root_id) AS output_root_id, item_count, succeeded, failed, skipped_deduped, materialized_paths, manifest_artifact_ref, started_at_utc, ended_at_utc, terminal_stage, created_at_utc FROM $domain.record)[0]; };");

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

        let key = input.configured_root.clone();
        let existing: Option<SurrealUuid> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT VALUE root_id FROM atelier_md_output_root WHERE configured_root = $value LIMIT 1;", StringBinding { value: key }).await })).await?;
        let root_id: Uuid = existing
            .map(Into::into)
            .unwrap_or_else(|| stable_downloader_uuid("output-root", &input.configured_root));
        let row: Option<OutputRootRow> = self
            .write_with_event(
                WRITE_OUTPUT_ROOT,
                OutputRootWrite {
                    record: RecordId::new("atelier_md_output_root", SurrealUuid::from(root_id)),
                    root_id: root_id.into(),
                    configured_root: input.configured_root.clone(),
                    materialization_mode: input.materialization_mode.as_token().to_owned(),
                    per_mode_subdirs: input.per_mode_subdirs.clone(),
                },
                OUTPUT_ROOT_CONFIGURED,
                "atelier_md_output_root",
                &root_id.to_string(),
                serde_json::json!({
                    "root_id": root_id,
                    "configured_root": input.configured_root,
                    "materialization_mode": input.materialization_mode.as_token(),
                }),
            )
            .await?;
        row.map(output_root_from_row)
            .transpose()?
            .ok_or_else(|| AtelierError::Internal("output root write returned no row".to_owned()))
    }

    /// Fetch an output-root config by id.
    pub async fn get_output_root_config(&self, root_id: Uuid) -> AtelierResult<OutputRootConfig> {
        let row: Option<OutputRootRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT root_id, configured_root, materialization_mode, per_mode_subdirs, created_at_utc, updated_at_utc FROM atelier_md_output_root WHERE root_id = $value LIMIT 1;", UuidBinding { value: root_id.into() }).await })).await?;
        row.map(output_root_from_row)
            .transpose()?
            .ok_or_else(|| AtelierError::NotFound(format!("output_root_id={root_id}")))
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

        let key = input.name.clone();
        let existing: Option<SurrealUuid> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT VALUE allowlist_policy_id FROM atelier_md_allowlist_policy WHERE name = $value LIMIT 1;", StringBinding { value: key }).await })).await?;
        let id: Uuid = existing
            .map(Into::into)
            .unwrap_or_else(|| stable_downloader_uuid("allowlist", &input.name));
        let row: Option<AllowlistRow> = self.write_with_event(WRITE_ALLOWLIST, AllowlistWrite { record: RecordId::new("atelier_md_allowlist_policy", SurrealUuid::from(id)), allowlist_policy_id: id.into(), name: input.name.clone(), allowed_domains: input.allowed_domains.clone(), explicit_url_lists: input.explicit_url_lists.clone(), default_decision: input.default_decision.clone(), rate_limit: input.rate_limit.clone(), max_pages, robots_posture: input.robots_posture.clone() }, ALLOWLIST_POLICY_SET, "atelier_md_allowlist_policy", &id.to_string(), serde_json::json!({ "allowlist_policy_id": id, "name": input.name, "default_decision": input.default_decision, "max_pages": max_pages, "robots_posture": input.robots_posture })).await?;
        row.map(allowlist_from_row)
            .ok_or_else(|| AtelierError::Internal("allowlist write returned no row".to_owned()))
    }

    /// Fetch an allowlist policy by id.
    pub async fn get_allowlist_policy(
        &self,
        allowlist_policy_id: Uuid,
    ) -> AtelierResult<AllowlistPolicy> {
        let row: Option<AllowlistRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT allowlist_policy_id, name, allowed_domains, explicit_url_lists, default_decision, rate_limit, max_pages, robots_posture, created_at_utc, updated_at_utc FROM atelier_md_allowlist_policy WHERE allowlist_policy_id = $value LIMIT 1;", UuidBinding { value: allowlist_policy_id.into() }).await })).await?;
        row.map(allowlist_from_row).ok_or_else(|| {
            AtelierError::NotFound(format!("allowlist_policy_id={allowlist_policy_id}"))
        })
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

        let header_ref_count = header_refs.as_array().map(|a| a.len()).unwrap_or(0);
        let key = input.label.clone();
        let existing: Option<SurrealUuid> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT VALUE auth_context_ref FROM atelier_md_auth_context WHERE label = $value LIMIT 1;", StringBinding { value: key }).await })).await?;
        let id: Uuid = existing
            .map(Into::into)
            .unwrap_or_else(|| stable_downloader_uuid("auth-context", &input.label));
        let row: Option<AuthRow> = self
            .write_with_event(
                WRITE_AUTH,
                AuthWrite {
                    record: RecordId::new("atelier_md_auth_context", SurrealUuid::from(id)),
                    auth_context_ref: id.into(),
                    label: input.label.clone(),
                    auth_mode: input.auth_mode.as_token().to_owned(),
                    session_ref: input.session_ref.clone(),
                    cookie_jar_artifact_ref: input.cookie_jar_artifact_ref.clone(),
                    header_secret_refs: header_refs,
                },
                AUTH_CONTEXT_REGISTERED,
                "atelier_md_auth_context",
                &id.to_string(),
                serde_json::json!({
                    "auth_context_ref": id,
                    "label": input.label,
                    "auth_mode": input.auth_mode.as_token(),
                    "has_session_ref": input.session_ref.is_some(),
                    "has_cookie_jar": input.cookie_jar_artifact_ref.is_some(),
                    "header_secret_ref_count": header_ref_count,
                    "secret_values": REDACTED_PLACEHOLDER,
                }),
            )
            .await?;
        row.map(auth_from_row)
            .transpose()?
            .ok_or_else(|| AtelierError::Internal("auth context write returned no row".to_owned()))
    }

    /// Fetch an auth context by ref. Auth material remains by-reference; no
    /// secret value is stored to return.
    pub async fn get_auth_context(&self, auth_context_ref: Uuid) -> AtelierResult<AuthContext> {
        let row: Option<AuthRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT auth_context_ref, label, auth_mode, session_ref, cookie_jar_artifact_ref, header_secret_refs, created_at_utc, updated_at_utc FROM atelier_md_auth_context WHERE auth_context_ref = $value LIMIT 1;", UuidBinding { value: auth_context_ref.into() }).await })).await?;
        row.map(auth_from_row)
            .transpose()?
            .ok_or_else(|| AtelierError::NotFound(format!("auth_context_ref={auth_context_ref}")))
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

        let session_id = stable_downloader_uuid("session", &input.idempotency_key);
        let row: Option<SessionRow> = self
            .write_with_event(
                WRITE_SESSION,
                SessionWrite {
                    record: RecordId::new(
                        "atelier_md_download_session",
                        SurrealUuid::from(session_id),
                    ),
                    session_id: session_id.into(),
                    parent_job_id: input.parent_job_id.clone(),
                    idempotency_key: input.idempotency_key.clone(),
                    source_kind: input.source_kind.as_token().to_owned(),
                    auth_context_ref: input
                        .auth_context_ref
                        .map(|id| RecordId::new("atelier_md_auth_context", SurrealUuid::from(id))),
                    allowlist_policy_id: RecordId::new(
                        "atelier_md_allowlist_policy",
                        SurrealUuid::from(input.allowlist_policy_id),
                    ),
                    output_root_id: RecordId::new(
                        "atelier_md_output_root",
                        SurrealUuid::from(input.output_root_id),
                    ),
                    protocol_id: input.protocol_id.clone(),
                    capability_profile_id: input.capability_profile_id.clone(),
                    capability_grant_ref: input.capability_grant_ref.clone(),
                },
                SESSION_OPENED,
                "atelier_md_download_session",
                &session_id.to_string(),
                serde_json::json!({
                    "session_id": session_id,
                    "parent_job_id": input.parent_job_id,
                    "source_kind": input.source_kind.as_token(),
                    "allowlist_policy_id": input.allowlist_policy_id,
                    "output_root_id": input.output_root_id,
                    "auth_context_ref": input.auth_context_ref,
                    "protocol_id": input.protocol_id,
                    "capability_profile_id": input.capability_profile_id,
                    "capability_grant_ref": input.capability_grant_ref,
                    "required_capabilities": required_capabilities,
                    "stage": SessionStage::Resolving.as_token(),
                }),
            )
            .await?;
        let session = row.map(session_from_row).transpose()?.ok_or_else(|| {
            AtelierError::Internal("download session write returned no row".to_owned())
        })?;
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
        let value = idempotency_key.to_owned();
        let row: Option<SessionRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT session_id, parent_job_id, idempotency_key, source_kind, IF auth_context_ref = NONE { NONE } ELSE { record::id(auth_context_ref) } AS auth_context_ref, record::id(allowlist_policy_id) AS allowlist_policy_id, record::id(output_root_id) AS output_root_id, protocol_id, capability_profile_id, capability_grant_ref, stage, created_at_utc, updated_at_utc FROM atelier_md_download_session WHERE idempotency_key = $value LIMIT 1;", StringBinding { value }).await })).await?;
        row.map(session_from_row).transpose()
    }

    /// Fetch a session by id.
    pub async fn get_download_session(&self, session_id: Uuid) -> AtelierResult<DownloadSession> {
        let row: Option<SessionRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT session_id, parent_job_id, idempotency_key, source_kind, IF auth_context_ref = NONE { NONE } ELSE { record::id(auth_context_ref) } AS auth_context_ref, record::id(allowlist_policy_id) AS allowlist_policy_id, record::id(output_root_id) AS output_root_id, protocol_id, capability_profile_id, capability_grant_ref, stage, created_at_utc, updated_at_utc FROM atelier_md_download_session WHERE session_id = $value LIMIT 1;", UuidBinding { value: session_id.into() }).await })).await?;
        row.map(session_from_row)
            .transpose()?
            .ok_or_else(|| AtelierError::NotFound(format!("session_id={session_id}")))
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
        let checkpoint_id = Uuid::now_v7();
        let bindings = StageWrite {
            download_session: RecordId::new(
                "atelier_md_download_session",
                SurrealUuid::from(session_id),
            ),
            checkpoint: RecordId::new("atelier_md_checkpoint", SurrealUuid::from(checkpoint_id)),
            checkpoint_id: checkpoint_id.into(),
            stage: stage.as_token().to_owned(),
            resume_token: resume_token.map(ToOwned::to_owned),
        };
        let row: Option<SessionRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(ADVANCE_SESSION, bindings).await })
            })
            .await?;
        let session = row
            .map(session_from_row)
            .transpose()?
            .ok_or_else(|| AtelierError::NotFound(format!("session_id={session_id}")))?;

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

        let item_id =
            stable_downloader_uuid("item", &format!("{session_id}:{}", input.normalized_url));
        let bindings = ItemWrite {
            record: RecordId::new("atelier_md_item_state", SurrealUuid::from(item_id)),
            item_id: item_id.into(),
            download_session: RecordId::new(
                "atelier_md_download_session",
                SurrealUuid::from(session_id),
            ),
            normalized_url: input.normalized_url.clone(),
            stable_source_id: input.stable_source_id.clone(),
        };
        let row: Option<ItemRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(CREATE_ITEM, bindings).await })
            })
            .await?;
        let item = row
            .map(item_from_row)
            .transpose()?
            .ok_or_else(|| AtelierError::NotFound(format!("session_id={session_id}")))?;
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
        let bindings = SessionUrlBinding {
            download_session: RecordId::new(
                "atelier_md_download_session",
                SurrealUuid::from(session_id),
            ),
            normalized_url: normalized_url.to_owned(),
        };
        let row: Option<ItemRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT item_id, record::id(session_id) AS session_id, normalized_url, stable_source_id, content_hash, stage, bytes_downloaded, bytes_total, part_path_ref, attempt_count, last_error_code, resume_token, created_at_utc, updated_at_utc FROM atelier_md_item_state WHERE session_id = $download_session AND normalized_url = $normalized_url LIMIT 1;", bindings).await })).await?;
        row.map(item_from_row).transpose()
    }

    /// List items in a session in enqueue order, optionally filtered by stage.
    pub async fn list_session_items(
        &self,
        session_id: Uuid,
        stage: Option<ItemStage>,
    ) -> AtelierResult<Vec<ItemState>> {
        let bindings = ItemListBinding {
            download_session: RecordId::new(
                "atelier_md_download_session",
                SurrealUuid::from(session_id),
            ),
            stage: stage.map(|value| value.as_token().to_owned()),
        };
        let rows: Vec<ItemRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_values("SELECT item_id, record::id(session_id) AS session_id, normalized_url, stable_source_id, content_hash, stage, bytes_downloaded, bytes_total, part_path_ref, attempt_count, last_error_code, resume_token, created_at_utc, updated_at_utc FROM atelier_md_item_state WHERE session_id = $download_session AND ($stage = NONE OR stage = $stage) ORDER BY created_at_utc ASC;", bindings).await })).await?;
        rows.into_iter().map(item_from_row).collect()
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

        let checkpoint_id = Uuid::now_v7();
        let bindings = CheckpointWrite {
            checkpoint: RecordId::new("atelier_md_checkpoint", SurrealUuid::from(checkpoint_id)),
            checkpoint_id: checkpoint_id.into(),
            download_session: RecordId::new(
                "atelier_md_download_session",
                SurrealUuid::from(session_id),
            ),
            item: input
                .item_id
                .map(|id| RecordId::new("atelier_md_item_state", SurrealUuid::from(id))),
            stage: input.stage.clone(),
            bytes_downloaded: input.bytes_downloaded,
            bytes_total: input.bytes_total,
            resume_token: input.resume_token.clone(),
        };
        let row: Option<CheckpointRow> = self
            .with_data(move |ctx| {
                Box::pin(async move { ctx.query_first(WRITE_CHECKPOINT, bindings).await })
            })
            .await?;
        let checkpoint = row
            .map(checkpoint_from_row)
            .ok_or_else(|| AtelierError::NotFound(format!("session_id={session_id}")))?;
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
        let bindings = CheckpointLookup {
            download_session: RecordId::new(
                "atelier_md_download_session",
                SurrealUuid::from(session_id),
            ),
            item: item_id.map(|id| RecordId::new("atelier_md_item_state", SurrealUuid::from(id))),
        };
        let row: Option<CheckpointRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT checkpoint_id, record::id(session_id) AS session_id, IF item_id = NONE { NONE } ELSE { record::id(item_id) } AS item_id, stage, bytes_downloaded, bytes_total, resume_token, created_at_utc FROM atelier_md_checkpoint WHERE session_id = $download_session AND item_id = $item ORDER BY created_at_utc DESC LIMIT 1;", bindings).await })).await?;
        Ok(row.map(checkpoint_from_row))
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

        let existing = self
            .get_session_receipt(session_id, input.terminal_stage)
            .await?;
        let receipt_id = existing
            .as_ref()
            .map(|row| row.receipt_id)
            .unwrap_or_else(|| {
                stable_downloader_uuid(
                    "receipt",
                    &format!("{session_id}:{}", input.terminal_stage.as_token()),
                )
            });
        let row: Option<ReceiptRow> = self
            .write_with_event(
                WRITE_RECEIPT,
                ReceiptWrite {
                    record: RecordId::new(
                        "atelier_md_session_receipt",
                        SurrealUuid::from(receipt_id),
                    ),
                    receipt_id: receipt_id.into(),
                    session: RecordId::new(
                        "atelier_md_download_session",
                        SurrealUuid::from(session_id),
                    ),
                    parent_job_id: session.parent_job_id.clone(),
                    source_kind: session.source_kind.as_token().to_owned(),
                    auth_context_ref: session
                        .auth_context_ref
                        .map(|id| RecordId::new("atelier_md_auth_context", SurrealUuid::from(id))),
                    allowlist_policy_id: RecordId::new(
                        "atelier_md_allowlist_policy",
                        SurrealUuid::from(session.allowlist_policy_id),
                    ),
                    output_root_id: RecordId::new(
                        "atelier_md_output_root",
                        SurrealUuid::from(session.output_root_id),
                    ),
                    item_count: input.item_count,
                    succeeded: input.succeeded,
                    failed: input.failed,
                    skipped_deduped: input.skipped_deduped,
                    materialized_paths: input.materialized_paths.clone(),
                    manifest_artifact_ref: input.manifest_artifact_ref.clone(),
                    started_at_utc: input.started_at_utc.map(Datetime::from),
                    ended_at_utc: input.ended_at_utc.map(Datetime::from),
                    terminal_stage: input.terminal_stage.as_token().to_owned(),
                },
                SESSION_RECEIPT_EMITTED,
                "atelier_md_session_receipt",
                &receipt_id.to_string(),
                serde_json::json!({
                    "receipt_id": receipt_id,
                    "session_id": session_id,
                    "parent_job_id": session.parent_job_id,
                    "source_kind": session.source_kind.as_token(),
                    "item_count": input.item_count,
                    "succeeded": input.succeeded,
                    "failed": input.failed,
                    "skipped_deduped": input.skipped_deduped,
                    "terminal_stage": input.terminal_stage.as_token(),
                    // Auth carried by ref only; never a secret value.
                    "auth_context_ref": session.auth_context_ref,
                    "secret_values": REDACTED_PLACEHOLDER,
                }),
            )
            .await?;
        row.map(receipt_from_row).transpose()?.ok_or_else(|| {
            AtelierError::Internal("session receipt write returned no row".to_owned())
        })
    }

    /// Fetch a session's terminal receipt for a given terminal stage, if emitted.
    pub async fn get_session_receipt(
        &self,
        session_id: Uuid,
        terminal_stage: TerminalStage,
    ) -> AtelierResult<Option<SessionReceipt>> {
        let bindings = ReceiptLookup {
            download_session: RecordId::new(
                "atelier_md_download_session",
                SurrealUuid::from(session_id),
            ),
            terminal_stage: terminal_stage.as_token().to_owned(),
        };
        let row: Option<ReceiptRow> = self.with_data(move |ctx| Box::pin(async move { ctx.query_first("SELECT receipt_id, record::id(session_id) AS session_id, parent_job_id, source_kind, IF auth_context_ref = NONE { NONE } ELSE { record::id(auth_context_ref) } AS auth_context_ref, record::id(allowlist_policy_id) AS allowlist_policy_id, record::id(output_root_id) AS output_root_id, item_count, succeeded, failed, skipped_deduped, materialized_paths, manifest_artifact_ref, started_at_utc, ended_at_utc, terminal_stage, created_at_utc FROM atelier_md_session_receipt WHERE session_id = $download_session AND terminal_stage = $terminal_stage LIMIT 1;", bindings).await })).await?;
        row.map(receipt_from_row).transpose()
    }
}
