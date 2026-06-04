//! Sourcing-spec schema + handler version matrix (MT-201, WP-KERNEL-005).
//!
//! Implements the governed DATA + RECEIPT model for the spec-to-handler binding
//! pipeline defined by master-spec-v02.189 section 6.12 "Sourcing-Spec Schema
//! and Handler Version Matrix". This module is storage-and-evidence only: it
//! records version-pinned sourcing-spec records, the multi-version handler
//! compatibility matrix, deterministic binding decisions, version-mismatch
//! receipts, and idempotent ingestion receipts. It NEVER executes an external
//! tool, opens a socket, spawns a process, or calls a localhost endpoint -- the
//! actual handler run (media-downloader / ASR / external-tool / export /
//! ComfyUI) is a capability-gated Workflow-Engine job that writes its produced
//! artifact refs through these records. The EventLedger is the sole source of
//! truth (S6.12.1, S6.12.6); handler-advertised metadata is advisory only.
//!
//! Spec authority: S6.12.1 (definitions), S6.12.2 (sourcing-spec schema /
//! parse-validate / canonical hashing / secret hygiene), S6.12.3 (handler
//! version matrix / routing law / sunset enforcement), S6.12.4 (version pinning
//! / routing / mismatch rejection with receipt / matrix snapshot), S6.12.5
//! (idempotent ingestion / ingestion key / partial-failure recovery), S6.12.6
//! (evidence / secret scrubbing / storage authority).
//!
//! CKC source (intent only; the SQLite/Electron/localhost original is NOT
//! copied): `app/backend/imageSourcingAdapter.js`
//! (`readSpecVersion`/`resolveHandler` -- the version-keyed handler registry
//! and the hard "No handler registered for spec_version" rejection;
//! `fileSha256Hex` -- content-hash identity; `runIngestion`/`ensureBatch` -- the
//! idempotent ingestion batch keyed by spec + version + dataset). Storage
//! authority here is PostgreSQL + EventLedger only (MT-004).
//!
//! Microtasks: MT-201 (sourcing-spec schema + handler version matrix),
//! MT-005 (event coverage).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

/// Sourcing event families (MT-201, MT-005). Defined here so the parent can
/// fold these into [`super::event_family::ALL`] and the MT-005 coverage check
/// picks up sourcing mutations.
pub mod sourcing_event_family {
    /// A version-pinned sourcing-spec record was registered (S6.12.2).
    pub const SOURCING_SPEC_REGISTERED: &str = "atelier.sourcing.spec_registered";
    /// A handler version matrix entry was published (S6.12.3).
    pub const HANDLER_MATRIX_ENTRY_PUBLISHED: &str = "atelier.sourcing.handler_matrix_published";
    /// A deterministic spec-to-handler binding decision was recorded (S6.12.4).
    pub const SOURCING_BINDING_DECIDED: &str = "atelier.sourcing.binding_decided";
    /// A version-mismatch rejection receipt was produced (S6.12.4).
    pub const VERSION_MISMATCH_REJECTED: &str = "atelier.sourcing.version_mismatch_rejected";
    /// An idempotent ingestion receipt was recorded, fresh or deduped (S6.12.5).
    pub const SOURCING_INGESTION_RECEIPTED: &str = "atelier.sourcing.ingestion_receipted";

    /// All sourcing event families (for parity/coverage folding).
    pub const ALL: &[&str] = &[
        SOURCING_SPEC_REGISTERED,
        HANDLER_MATRIX_ENTRY_PUBLISHED,
        SOURCING_BINDING_DECIDED,
        VERSION_MISMATCH_REJECTED,
        SOURCING_INGESTION_RECEIPTED,
    ];
}

/// Re-export at module root so callers can write `sourcing::SOURCING_BINDING_DECIDED`.
pub use sourcing_event_family::{
    HANDLER_MATRIX_ENTRY_PUBLISHED, SOURCING_BINDING_DECIDED, SOURCING_INGESTION_RECEIPTED,
    SOURCING_SPEC_REGISTERED, VERSION_MISMATCH_REJECTED,
};

/// Side-effect class of a handler version (S6.12.3, consistent with S6.0.2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffect {
    Read,
    Write,
    Execute,
}

impl SideEffect {
    /// Stable uppercase DB token, matching the spec vocabulary.
    pub fn as_token(self) -> &'static str {
        match self {
            SideEffect::Read => "READ",
            SideEffect::Write => "WRITE",
            SideEffect::Execute => "EXECUTE",
        }
    }

    /// Parse a stored token. Unknown tokens are a validation error rather than a
    /// silent default, so a corrupt row never masquerades as `READ`.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "READ" => Ok(SideEffect::Read),
            "WRITE" => Ok(SideEffect::Write),
            "EXECUTE" => Ok(SideEffect::Execute),
            other => Err(AtelierError::Validation(format!(
                "unknown side_effect token: {other}"
            ))),
        }
    }
}

/// Idempotency class of a handler version (S6.12.3, consistent with S6.0.2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyClass {
    Idempotent,
    IdempotentWithKey,
    NonIdempotent,
}

impl IdempotencyClass {
    pub fn as_token(self) -> &'static str {
        match self {
            IdempotencyClass::Idempotent => "IDEMPOTENT",
            IdempotencyClass::IdempotentWithKey => "IDEMPOTENT_WITH_KEY",
            IdempotencyClass::NonIdempotent => "NON_IDEMPOTENT",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "IDEMPOTENT" => Ok(IdempotencyClass::Idempotent),
            "IDEMPOTENT_WITH_KEY" => Ok(IdempotencyClass::IdempotentWithKey),
            "NON_IDEMPOTENT" => Ok(IdempotencyClass::NonIdempotent),
            other => Err(AtelierError::Validation(format!(
                "unknown idempotency token: {other}"
            ))),
        }
    }

    /// Whether a spec-supplied idempotency key MUST be honored for dedupe
    /// (S6.12.5). `IDEMPOTENT_WITH_KEY` requires the key; `IDEMPOTENT` dedupes on
    /// the binding identity alone.
    pub fn requires_key(self) -> bool {
        matches!(self, IdempotencyClass::IdempotentWithKey)
    }
}

/// Lifecycle status of a handler version matrix entry (S6.12.3). Drives sunset
/// enforcement in the routing law (S6.12.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerStatus {
    Active,
    Deprecated,
    Sunset,
}

impl HandlerStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            HandlerStatus::Active => "ACTIVE",
            HandlerStatus::Deprecated => "DEPRECATED",
            HandlerStatus::Sunset => "SUNSET",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "ACTIVE" => Ok(HandlerStatus::Active),
            "DEPRECATED" => Ok(HandlerStatus::Deprecated),
            "SUNSET" => Ok(HandlerStatus::Sunset),
            other => Err(AtelierError::Validation(format!(
                "unknown handler status token: {other}"
            ))),
        }
    }
}

/// Machine-readable reason a binding decision did not bind (S6.12.4 rule 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MismatchReason {
    /// No matrix entry's `handler_version` satisfied the spec pin.
    NoMatchingVersion,
    /// A candidate matched the pin but did not support the spec `schema_version`.
    SchemaUnsupported,
    /// The only satisfying entry was `SUNSET`.
    Sunset,
    /// The capability intersection denied the union of required capabilities.
    CapabilityDenied,
}

impl MismatchReason {
    pub fn as_token(self) -> &'static str {
        match self {
            MismatchReason::NoMatchingVersion => "no_matching_version",
            MismatchReason::SchemaUnsupported => "schema_unsupported",
            MismatchReason::Sunset => "sunset",
            MismatchReason::CapabilityDenied => "capability_denied",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "no_matching_version" => Ok(MismatchReason::NoMatchingVersion),
            "schema_unsupported" => Ok(MismatchReason::SchemaUnsupported),
            "sunset" => Ok(MismatchReason::Sunset),
            "capability_denied" => Ok(MismatchReason::CapabilityDenied),
            other => Err(AtelierError::Validation(format!(
                "unknown mismatch reason token: {other}"
            ))),
        }
    }
}

/// Dedupe outcome of an ingestion receipt (S6.12.5 rule 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionOutcome {
    /// A new ingestion under this identity executed side effects.
    Fresh,
    /// A repeat ingestion under the same identity returned the prior receipt.
    Deduped,
}

impl IngestionOutcome {
    pub fn as_token(self) -> &'static str {
        match self {
            IngestionOutcome::Fresh => "fresh",
            IngestionOutcome::Deduped => "deduped",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "fresh" => Ok(IngestionOutcome::Fresh),
            "deduped" => Ok(IngestionOutcome::Deduped),
            other => Err(AtelierError::Validation(format!(
                "unknown ingestion outcome token: {other}"
            ))),
        }
    }
}

/// Placeholder substituted for any secret/cookie/token value that would
/// otherwise be persisted in a stored record or event payload (S6.12.2 rule 3,
/// S6.12.6 secret-scrubbing; mirrors `settings.rs` redaction style).
const REDACTED_PLACEHOLDER: &str = "[REDACTED]";

/// Top-level sourcing-spec fields permitted by the schema (S6.12.2). Any other
/// top-level key in `params_json` / spec body is rejected, never ignored
/// (S6.12.2 rule 2: no silent coercion). Kept here so the rejection set is
/// auditable from a single place.
const ALLOWED_SPEC_PARAM_KEYS: &[&str] = &[
    "sourcing_spec_id",
    "schema_version",
    "source",
    "handler_family",
    "handler_version_pin",
    "params",
    "required_capabilities",
    "idempotency_key",
    "spec_hash",
];

/// Substrings that mark a params key as carrying secret material; any value
/// under such a key is redacted before storage and before any event payload
/// (S6.12.2 rule 3 / S6.12.6). Matched case-insensitively against the key.
const SECRET_KEY_MARKERS: &[&str] = &[
    "secret", "token", "cookie", "password", "passwd", "apikey", "api_key", "auth", "credential",
    "bearer",
];

/// A version-pinned sourcing-spec record (`PRIM-SourcingSpecRecordV1`, S6.12.2).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourcingSpecRecord {
    /// Internal row id. Distinct from the caller-facing `sourcing_spec_id`.
    pub record_id: Uuid,
    /// Stable identity for the spec instance (caller-supplied).
    pub sourcing_spec_id: String,
    /// Semver of the sourcing-spec schema (`MAJOR.MINOR.PATCH`).
    pub schema_version: String,
    /// Source descriptor kind (e.g. `media_url`, `artifact_ref`, `feed`).
    pub source_kind: String,
    /// Portable source ref (no drive-letter / user-profile / machine-local path).
    pub source_ref: String,
    /// Target handler family (e.g. `media_downloader`, `asr`, `export`).
    pub handler_family: String,
    /// Semver constraint the binding MUST satisfy (exact / caret / range).
    pub handler_version_pin: String,
    /// Handler-scoped params, already secret-scrubbed (S6.12.2 rule 3).
    pub params_json: serde_json::Value,
    /// Required capability strings, evaluated deny-by-default (AS11.1).
    pub required_capabilities: Vec<String>,
    /// Caller-supplied dedupe key for ingestion (optional, S6.12.5).
    pub idempotency_key: Option<String>,
    /// SHA-256 over the canonicalized JSON form; registry/replay identity.
    pub spec_hash: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to register a sourcing-spec (the parsed/canonicalized form).
#[derive(Clone, Debug)]
pub struct NewSourcingSpec {
    pub sourcing_spec_id: String,
    pub schema_version: String,
    pub source_kind: String,
    pub source_ref: String,
    pub handler_family: String,
    pub handler_version_pin: String,
    /// Handler params object. Secret-bearing keys are redacted on the way in.
    pub params_json: serde_json::Value,
    pub required_capabilities: Vec<String>,
    pub idempotency_key: Option<String>,
}

/// A handler version matrix entry (`PRIM-HandlerVersionMatrixEntryV1`, S6.12.3).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandlerVersionMatrixEntry {
    pub entry_id: Uuid,
    pub handler_family: String,
    pub handler_version: String,
    /// Inclusive lower bound of accepted spec `schema_version` (semver).
    pub schema_version_min: String,
    /// Inclusive upper bound of accepted spec `schema_version` (semver).
    pub schema_version_max: String,
    pub side_effect: SideEffect,
    pub idempotency: IdempotencyClass,
    pub required_capabilities: Vec<String>,
    /// Determinism class D0-D3 (S6.3.0).
    pub determinism: String,
    pub status: HandlerStatus,
    /// The Workflow-Engine AI Job profile implementing this handler version.
    pub job_profile_ref: String,
    pub created_at_utc: DateTime<Utc>,
}

/// Input to publish a handler version matrix entry. Entries are immutable once
/// published (S6.12.3 rule 1); re-publishing the same `(family, version)` is a
/// no-op that returns the existing immutable entry.
#[derive(Clone, Debug)]
pub struct NewHandlerVersionEntry {
    pub handler_family: String,
    pub handler_version: String,
    pub schema_version_min: String,
    pub schema_version_max: String,
    pub side_effect: SideEffect,
    pub idempotency: IdempotencyClass,
    pub required_capabilities: Vec<String>,
    pub determinism: String,
    pub status: HandlerStatus,
    pub job_profile_ref: String,
}

/// A deterministic binding decision (`PRIM-SourcingBindingDecisionV1`, S6.12.4).
/// When `bound` is true the resolved handler version + matched entry are set;
/// when false a sibling [`VersionMismatchReceipt`] carries the reason.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BindingDecision {
    pub decision_id: Uuid,
    pub sourcing_spec_id: String,
    pub spec_hash: String,
    pub handler_family: String,
    pub handler_version_pin: String,
    /// Whether the spec bound to a concrete handler version.
    pub bound: bool,
    /// Resolved handler version (present when `bound`).
    pub resolved_handler_version: Option<String>,
    /// Matched matrix entry id (present when `bound`).
    pub matched_entry_id: Option<Uuid>,
    /// The matrix snapshot id resolved against, for replay (S6.12.4 rule 2).
    pub matrix_snapshot_id: Uuid,
    /// Whether the capability union was satisfied (S6.12.3 rule 3).
    pub capability_satisfied: bool,
    /// Human/machine resolution reason.
    pub resolution_reason: String,
    pub created_at_utc: DateTime<Utc>,
}

/// A version-mismatch rejection receipt (`PRIM-VersionMismatchReceiptV1`,
/// S6.12.4 rule 4). A first-class evidenced outcome, not a swallowed error.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VersionMismatchReceipt {
    pub receipt_id: Uuid,
    pub decision_id: Uuid,
    pub sourcing_spec_id: String,
    pub spec_hash: String,
    pub requested_pin: String,
    /// The candidate handler versions that were evaluated.
    pub evaluated_versions: Vec<String>,
    pub matrix_snapshot_id: Uuid,
    pub reason: MismatchReason,
    pub created_at_utc: DateTime<Utc>,
}

/// An idempotent ingestion receipt (`PRIM-SourcingIngestionReceiptV1`, S6.12.5).
/// Bytes live in ArtifactStore behind `artifact_manifest_refs`; this row records
/// only the governed receipt + dedupe outcome.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IngestionReceipt {
    pub receipt_id: Uuid,
    pub decision_id: Uuid,
    /// Effective ingestion identity component: `(handler_family,
    /// handler_version, spec_hash, idempotency_key?)` hashed/joined into a
    /// single key (S6.12.5 rule 1).
    pub ingestion_key: String,
    pub handler_family: String,
    pub handler_version: String,
    pub spec_hash: String,
    /// ArtifactStore manifest refs produced by the handler job (by reference
    /// only; never inlined bytes -- S6.12.5 rule 3 / S6.12.6).
    pub artifact_manifest_refs: Vec<String>,
    pub outcome: IngestionOutcome,
    /// Completed vs pending artifact counts for partial-failure recovery
    /// (S6.12.5 rule 4).
    pub completed_count: i64,
    pub pending_count: i64,
    pub created_at_utc: DateTime<Utc>,
}

fn json_string_array(value: serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(items) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// Redact secret-bearing values inside a params object before storage/events
/// (S6.12.2 rule 3, S6.12.6). Recurses into nested objects/arrays; a key whose
/// name matches a [`SECRET_KEY_MARKERS`] substring has its value replaced with
/// the placeholder regardless of value type.
fn redact_secrets(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, val) in map {
                let lowered = key.to_ascii_lowercase();
                let is_secret = SECRET_KEY_MARKERS.iter().any(|m| lowered.contains(m));
                if is_secret {
                    out.insert(key.clone(), serde_json::json!(REDACTED_PLACEHOLDER));
                } else {
                    out.insert(key.clone(), redact_secrets(val));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(redact_secrets).collect())
        }
        other => other.clone(),
    }
}

/// Canonicalize a serde_json value to a deterministic byte string with stable
/// key ordering (S6.12.2 rule 4). `serde_json::Value`'s map is a `BTreeMap`
/// (alphabetically ordered) under this crate's default features, so serializing
/// is already key-stable; this helper normalizes whitespace by using the compact
/// form. Used for `spec_hash` over the canonicalized JSON form.
fn canonical_json_bytes(value: &serde_json::Value) -> Vec<u8> {
    // Compact (no spaces) serialization; key order is stable via serde_json's
    // ordered map. Deterministic for identical semantic content.
    serde_json::to_vec(value).unwrap_or_default()
}

/// SHA-256 hex of bytes, via Postgres `digest`/`encode` to avoid pulling a new
/// crate. Computed by a tiny DB round-trip on the connection pool so the hash
/// matches whatever the rest of the system would compute server-side.
async fn sha256_hex(store: &AtelierStore, bytes: &[u8]) -> AtelierResult<String> {
    // pgcrypto `digest` is available in the atelier database (used elsewhere via
    // gen_random_uuid from pgcrypto). Fall back to sha256 builtin if present.
    let hex: String = sqlx::query_scalar("SELECT encode(sha256($1::bytea), 'hex')")
        .bind(bytes)
        .fetch_one(store.pool())
        .await?;
    Ok(hex)
}

/// Compare two `MAJOR.MINOR.PATCH` semver strings. Non-numeric / missing
/// components sort as 0. Returns Ordering of `a` vs `b`. Pre-release/build
/// metadata is ignored (only the numeric core is compared), which is sufficient
/// for the inclusive range + pin checks this module performs.
fn semver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn parts(v: &str) -> [u64; 3] {
        let core = v.trim().trim_start_matches(['v', 'V', '^', '~', '=', '>', '<']);
        let core = core.split(['-', '+']).next().unwrap_or(core);
        let mut out = [0u64; 3];
        for (i, seg) in core.split('.').take(3).enumerate() {
            out[i] = seg.trim().parse::<u64>().unwrap_or(0);
        }
        out
    }
    parts(a).cmp(&parts(b))
}

/// Whether `schema_version` falls within the inclusive `[min, max]` matrix
/// range (S6.12.3 rule 2 / supported_schema_versions).
fn schema_in_range(schema_version: &str, min: &str, max: &str) -> bool {
    semver_cmp(schema_version, min) != std::cmp::Ordering::Less
        && semver_cmp(schema_version, max) != std::cmp::Ordering::Greater
}

/// Whether `handler_version` satisfies the spec `pin` (S6.12.4 rule 1).
///
/// Supports the three pin shapes the spec names (S6.12.2): exact (`1.2.3` or
/// `=1.2.3`), caret (`^1.2.3` -> same major, >= the pinned version), and a
/// simple closed range (`>=1.2.0 <2.0.0`). Unknown shapes fall back to exact
/// equality, which is the safe/strict default (never an accidental widen).
fn pin_satisfied(handler_version: &str, pin: &str) -> bool {
    let pin = pin.trim();
    if pin.is_empty() {
        return false;
    }
    if let Some(rest) = pin.strip_prefix('^') {
        // Caret: same major, and handler_version >= pinned.
        let same_major = semver_major(handler_version) == semver_major(rest);
        let ge = semver_cmp(handler_version, rest) != std::cmp::Ordering::Less;
        return same_major && ge;
    }
    // Range form: space-separated >=lo and <hi comparators.
    if pin.contains(">=") || pin.contains('<') || pin.contains('>') {
        let mut lower_ok = true;
        let mut upper_ok = true;
        for token in pin.split_whitespace() {
            if let Some(lo) = token.strip_prefix(">=") {
                lower_ok &= semver_cmp(handler_version, lo) != std::cmp::Ordering::Less;
            } else if let Some(lo) = token.strip_prefix('>') {
                lower_ok &= semver_cmp(handler_version, lo) == std::cmp::Ordering::Greater;
            } else if let Some(hi) = token.strip_prefix("<=") {
                upper_ok &= semver_cmp(handler_version, hi) != std::cmp::Ordering::Greater;
            } else if let Some(hi) = token.strip_prefix('<') {
                upper_ok &= semver_cmp(handler_version, hi) == std::cmp::Ordering::Less;
            }
        }
        return lower_ok && upper_ok;
    }
    // Exact (with optional leading '=').
    let exact = pin.strip_prefix('=').unwrap_or(pin);
    semver_cmp(handler_version, exact) == std::cmp::Ordering::Equal
}

fn semver_major(v: &str) -> u64 {
    let core = v.trim().trim_start_matches(['v', 'V', '^', '~', '=', '>', '<']);
    let core = core.split(['-', '+']).next().unwrap_or(core);
    core.split('.')
        .next()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn spec_from_row(row: &sqlx::postgres::PgRow) -> SourcingSpecRecord {
    SourcingSpecRecord {
        record_id: row.get("record_id"),
        sourcing_spec_id: row.get("sourcing_spec_id"),
        schema_version: row.get("schema_version"),
        source_kind: row.get("source_kind"),
        source_ref: row.get("source_ref"),
        handler_family: row.get("handler_family"),
        handler_version_pin: row.get("handler_version_pin"),
        params_json: row.get("params_json"),
        required_capabilities: json_string_array(row.get("required_capabilities")),
        idempotency_key: row.get("idempotency_key"),
        spec_hash: row.get("spec_hash"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn matrix_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<HandlerVersionMatrixEntry> {
    let side_effect: String = row.get("side_effect");
    let idempotency: String = row.get("idempotency");
    let status: String = row.get("status");
    Ok(HandlerVersionMatrixEntry {
        entry_id: row.get("entry_id"),
        handler_family: row.get("handler_family"),
        handler_version: row.get("handler_version"),
        schema_version_min: row.get("schema_version_min"),
        schema_version_max: row.get("schema_version_max"),
        side_effect: SideEffect::from_token(&side_effect)?,
        idempotency: IdempotencyClass::from_token(&idempotency)?,
        required_capabilities: json_string_array(row.get("required_capabilities")),
        determinism: row.get("determinism"),
        status: HandlerStatus::from_token(&status)?,
        job_profile_ref: row.get("job_profile_ref"),
        created_at_utc: row.get("created_at_utc"),
    })
}

fn decision_from_row(row: &sqlx::postgres::PgRow) -> BindingDecision {
    BindingDecision {
        decision_id: row.get("decision_id"),
        sourcing_spec_id: row.get("sourcing_spec_id"),
        spec_hash: row.get("spec_hash"),
        handler_family: row.get("handler_family"),
        handler_version_pin: row.get("handler_version_pin"),
        bound: row.get("bound"),
        resolved_handler_version: row.get("resolved_handler_version"),
        matched_entry_id: row.get("matched_entry_id"),
        matrix_snapshot_id: row.get("matrix_snapshot_id"),
        capability_satisfied: row.get("capability_satisfied"),
        resolution_reason: row.get("resolution_reason"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn mismatch_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<VersionMismatchReceipt> {
    let reason: String = row.get("reason");
    Ok(VersionMismatchReceipt {
        receipt_id: row.get("receipt_id"),
        decision_id: row.get("decision_id"),
        sourcing_spec_id: row.get("sourcing_spec_id"),
        spec_hash: row.get("spec_hash"),
        requested_pin: row.get("requested_pin"),
        evaluated_versions: json_string_array(row.get("evaluated_versions")),
        matrix_snapshot_id: row.get("matrix_snapshot_id"),
        reason: MismatchReason::from_token(&reason)?,
        created_at_utc: row.get("created_at_utc"),
    })
}

fn ingestion_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<IngestionReceipt> {
    let outcome: String = row.get("outcome");
    Ok(IngestionReceipt {
        receipt_id: row.get("receipt_id"),
        decision_id: row.get("decision_id"),
        ingestion_key: row.get("ingestion_key"),
        handler_family: row.get("handler_family"),
        handler_version: row.get("handler_version"),
        spec_hash: row.get("spec_hash"),
        artifact_manifest_refs: json_string_array(row.get("artifact_manifest_refs")),
        outcome: IngestionOutcome::from_token(&outcome)?,
        completed_count: row.get("completed_count"),
        pending_count: row.get("pending_count"),
        created_at_utc: row.get("created_at_utc"),
    })
}

const SPEC_COLUMNS: &str = "record_id, sourcing_spec_id, schema_version, source_kind, source_ref, \
                            handler_family, handler_version_pin, params_json, \
                            required_capabilities, idempotency_key, spec_hash, created_at_utc";

const MATRIX_COLUMNS: &str = "entry_id, handler_family, handler_version, schema_version_min, \
                              schema_version_max, side_effect, idempotency, \
                              required_capabilities, determinism, status, job_profile_ref, \
                              created_at_utc";

const DECISION_COLUMNS: &str = "decision_id, sourcing_spec_id, spec_hash, handler_family, \
                                handler_version_pin, bound, resolved_handler_version, \
                                matched_entry_id, matrix_snapshot_id, capability_satisfied, \
                                resolution_reason, created_at_utc";

impl AtelierStore {
    /// Register (idempotently) a version-pinned sourcing-spec record (S6.12.2).
    ///
    /// Enforces the parse/validate contract this module owns:
    ///   * unknown top-level params keys are rejected, never ignored (rule 2:
    ///     no silent coercion);
    ///   * secret/cookie/token values are scrubbed from the stored params and
    ///     from the event payload (rule 3 / S6.12.6);
    ///   * `spec_hash` is computed over the canonicalized JSON form so
    ///     semantically identical specs share identity (rule 4).
    /// Idempotent on `spec_hash`: re-registering the same canonical spec returns
    /// the existing record rather than duplicating it. Emits
    /// `SOURCING_SPEC_REGISTERED`.
    pub async fn register_sourcing_spec(
        &self,
        new: &NewSourcingSpec,
    ) -> AtelierResult<SourcingSpecRecord> {
        if new.sourcing_spec_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "sourcing_spec_id must not be empty".into(),
            ));
        }
        if new.handler_family.trim().is_empty() {
            return Err(AtelierError::Validation(
                "handler_family must not be empty".into(),
            ));
        }
        if new.handler_version_pin.trim().is_empty() {
            return Err(AtelierError::Validation(
                "handler_version_pin must not be empty".into(),
            ));
        }
        // Portability: reject machine-local source refs (S6.12.2 source rule).
        let lowered_ref = new.source_ref.to_ascii_lowercase();
        if lowered_ref.contains(":\\") || new.source_ref.starts_with("/users/") {
            return Err(AtelierError::Validation(
                "source ref must be portable (no drive-letter/user-profile path)".into(),
            ));
        }
        // No silent coercion: any unknown top-level key in the params object is
        // a hard reject (rule 2). The handler-scoped values live under `params`;
        // the object presented here is the spec body's params map.
        if let serde_json::Value::Object(map) = &new.params_json {
            for key in map.keys() {
                if !ALLOWED_SPEC_PARAM_KEYS.contains(&key.as_str()) && key != "params" {
                    // Allow arbitrary keys ONLY under the nested `params` object;
                    // top-level unknowns are rejected.
                    return Err(AtelierError::Validation(format!(
                        "unknown top-level sourcing-spec field rejected (no silent coercion): {key}"
                    )));
                }
            }
        }

        // Secret hygiene: scrub before hashing + storage so no secret ever
        // reaches the DB or the canonical hash input.
        let scrubbed_params = redact_secrets(&new.params_json);

        // Canonical hashing over a stable canonical JSON form of the
        // identity-bearing fields (rule 4).
        let canonical = serde_json::json!({
            "sourcing_spec_id": new.sourcing_spec_id,
            "schema_version": new.schema_version,
            "source": { "kind": new.source_kind, "ref": new.source_ref },
            "handler_family": new.handler_family,
            "handler_version_pin": new.handler_version_pin,
            "params": scrubbed_params,
            "required_capabilities": new.required_capabilities,
            "idempotency_key": new.idempotency_key,
        });
        let spec_hash = sha256_hex(self, &canonical_json_bytes(&canonical)).await?;

        // Idempotent fast path on canonical spec_hash.
        if let Some(existing) = self.get_sourcing_spec_by_hash(&spec_hash).await? {
            return Ok(existing);
        }

        let caps = serde_json::json!(new.required_capabilities);
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_sourcing_spec
                 (sourcing_spec_id, schema_version, source_kind, source_ref, handler_family,
                  handler_version_pin, params_json, required_capabilities, idempotency_key, spec_hash)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (spec_hash) DO UPDATE SET spec_hash = EXCLUDED.spec_hash
               RETURNING {SPEC_COLUMNS}"#
        ))
        .bind(&new.sourcing_spec_id)
        .bind(&new.schema_version)
        .bind(&new.source_kind)
        .bind(&new.source_ref)
        .bind(&new.handler_family)
        .bind(&new.handler_version_pin)
        .bind(scrubbed_params)
        .bind(caps)
        .bind(&new.idempotency_key)
        .bind(&spec_hash)
        .fetch_one(self.pool())
        .await?;
        let record = spec_from_row(&row);

        self.record_event(
            SOURCING_SPEC_REGISTERED,
            "atelier_sourcing_spec",
            &record.spec_hash,
            serde_json::json!({
                "record_id": record.record_id,
                "sourcing_spec_id": record.sourcing_spec_id,
                "schema_version": record.schema_version,
                "handler_family": record.handler_family,
                "handler_version_pin": record.handler_version_pin,
                "spec_hash": record.spec_hash,
                // params omitted/redacted: never leak secrets into the ledger.
                "params": record.params_json,
            }),
        )
        .await?;
        Ok(record)
    }

    /// Fetch a sourcing-spec record by its canonical `spec_hash`.
    pub async fn get_sourcing_spec_by_hash(
        &self,
        spec_hash: &str,
    ) -> AtelierResult<Option<SourcingSpecRecord>> {
        let row = sqlx::query(&format!(
            "SELECT {SPEC_COLUMNS} FROM atelier_sourcing_spec WHERE spec_hash = $1"
        ))
        .bind(spec_hash)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(spec_from_row))
    }

    /// Publish a handler version matrix entry (S6.12.3). Entries are immutable
    /// once published (rule 1): re-publishing the same `(handler_family,
    /// handler_version)` returns the existing entry unchanged rather than
    /// mutating its side-effect/idempotency/capabilities. A behavior change MUST
    /// be a new `handler_version`. Emits `HANDLER_MATRIX_ENTRY_PUBLISHED` only on
    /// a genuinely new entry.
    pub async fn publish_handler_version(
        &self,
        new: &NewHandlerVersionEntry,
    ) -> AtelierResult<HandlerVersionMatrixEntry> {
        if new.handler_family.trim().is_empty() || new.handler_version.trim().is_empty() {
            return Err(AtelierError::Validation(
                "handler_family and handler_version must not be empty".into(),
            ));
        }
        if let Some(existing) = self
            .get_handler_version(&new.handler_family, &new.handler_version)
            .await?
        {
            // Immutable: return existing, do not mutate, do not re-emit.
            return Ok(existing);
        }

        let caps = serde_json::json!(new.required_capabilities);
        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_handler_version_matrix
                 (handler_family, handler_version, schema_version_min, schema_version_max,
                  side_effect, idempotency, required_capabilities, determinism, status,
                  job_profile_ref)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               ON CONFLICT (handler_family, handler_version) DO UPDATE
                 SET handler_family = EXCLUDED.handler_family
               RETURNING {MATRIX_COLUMNS}"#
        ))
        .bind(&new.handler_family)
        .bind(&new.handler_version)
        .bind(&new.schema_version_min)
        .bind(&new.schema_version_max)
        .bind(new.side_effect.as_token())
        .bind(new.idempotency.as_token())
        .bind(caps)
        .bind(&new.determinism)
        .bind(new.status.as_token())
        .bind(&new.job_profile_ref)
        .fetch_one(self.pool())
        .await?;
        let entry = matrix_from_row(&row)?;

        self.record_event(
            HANDLER_MATRIX_ENTRY_PUBLISHED,
            "atelier_handler_version_matrix",
            &entry.entry_id.to_string(),
            serde_json::json!({
                "handler_family": entry.handler_family,
                "handler_version": entry.handler_version,
                "side_effect": entry.side_effect.as_token(),
                "idempotency": entry.idempotency.as_token(),
                "status": entry.status.as_token(),
            }),
        )
        .await?;
        Ok(entry)
    }

    /// Fetch a single matrix entry by `(handler_family, handler_version)`.
    pub async fn get_handler_version(
        &self,
        handler_family: &str,
        handler_version: &str,
    ) -> AtelierResult<Option<HandlerVersionMatrixEntry>> {
        let row = sqlx::query(&format!(
            r#"SELECT {MATRIX_COLUMNS} FROM atelier_handler_version_matrix
               WHERE handler_family = $1 AND handler_version = $2"#
        ))
        .bind(handler_family)
        .bind(handler_version)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(matrix_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// List all matrix entries for a handler family (newest first).
    pub async fn list_handler_versions(
        &self,
        handler_family: &str,
    ) -> AtelierResult<Vec<HandlerVersionMatrixEntry>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {MATRIX_COLUMNS} FROM atelier_handler_version_matrix
               WHERE handler_family = $1
               ORDER BY created_at_utc DESC"#
        ))
        .bind(handler_family)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(matrix_from_row).collect()
    }

    /// Resolve a sourcing-spec to a concrete handler version and record a
    /// deterministic binding decision (S6.12.4). This is data-only resolution:
    /// it never invokes a handler; the actual run is a separate Workflow-Engine
    /// job that reads the resulting decision.
    ///
    /// Routing law:
    ///   1. Select the highest `ACTIVE` handler version whose `handler_version`
    ///      satisfies the spec pin AND whose `supported_schema_versions` range
    ///      includes the spec `schema_version` (rule 1).
    ///   2. Record the `matrix_snapshot_id` resolved against for replay (rule 2).
    ///   3. No fallback downgrade: if nothing satisfies the pin, reject (rule 3).
    ///   4. On failure produce a `VersionMismatchReceipt` with a machine-readable
    ///      reason; no handler job is enqueued (rule 4).
    ///
    /// `capabilities_granted` is the session-scoped capability intersection
    /// (AS11.1); the union of spec + handler required capabilities must be a
    /// subset or the binding is denied with `capability_denied` (S6.12.3 rule 3).
    /// `matrix_snapshot_id` pins the matrix state the caller resolved against.
    /// Emits `SOURCING_BINDING_DECIDED`, plus `VERSION_MISMATCH_REJECTED` on a
    /// rejection.
    pub async fn decide_binding(
        &self,
        spec_hash: &str,
        capabilities_granted: &[String],
        matrix_snapshot_id: Uuid,
    ) -> AtelierResult<BindingDecision> {
        let spec = self
            .get_sourcing_spec_by_hash(spec_hash)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("sourcing spec spec_hash={spec_hash}")))?;

        let candidates = self.list_handler_versions(&spec.handler_family).await?;
        let evaluated: Vec<String> = candidates
            .iter()
            .map(|c| c.handler_version.clone())
            .collect();

        // Pin-satisfying candidates (any status), used to disambiguate the
        // rejection reason between "no version matched the pin" and "matched but
        // schema unsupported / sunset".
        let pin_matches: Vec<&HandlerVersionMatrixEntry> = candidates
            .iter()
            .filter(|c| pin_satisfied(&c.handler_version, &spec.handler_version_pin))
            .collect();

        // Bindable: pin-satisfying, schema-in-range, not SUNSET, prefer ACTIVE.
        // Highest version wins (rule 1). DEPRECATED may proceed when the pin
        // targets it (rule 4) but ACTIVE is preferred.
        let mut bindable: Vec<&HandlerVersionMatrixEntry> = pin_matches
            .iter()
            .copied()
            .filter(|c| {
                c.status != HandlerStatus::Sunset
                    && schema_in_range(
                        &spec.schema_version,
                        &c.schema_version_min,
                        &c.schema_version_max,
                    )
            })
            .collect();
        bindable.sort_by(|a, b| {
            // ACTIVE before DEPRECATED, then highest version.
            let a_active = a.status == HandlerStatus::Active;
            let b_active = b.status == HandlerStatus::Active;
            b_active
                .cmp(&a_active)
                .then_with(|| semver_cmp(&b.handler_version, &a.handler_version))
        });

        // Determine outcome.
        let chosen = bindable.first().copied();

        if let Some(entry) = chosen {
            // Capability intersection: union(spec, handler) must be granted.
            let granted: std::collections::BTreeSet<&str> =
                capabilities_granted.iter().map(String::as_str).collect();
            let mut required: std::collections::BTreeSet<&str> =
                spec.required_capabilities.iter().map(String::as_str).collect();
            for c in &entry.required_capabilities {
                required.insert(c.as_str());
            }
            let cap_ok = required.iter().all(|c| granted.contains(c));

            if !cap_ok {
                return self
                    .record_rejection(
                        &spec,
                        matrix_snapshot_id,
                        &evaluated,
                        MismatchReason::CapabilityDenied,
                        false,
                        "capability intersection denied required capability union",
                    )
                    .await;
            }

            let reason = if entry.status == HandlerStatus::Deprecated {
                "bound to DEPRECATED handler version (pin targeted it); proceeding with warning"
            } else {
                "bound to highest ACTIVE handler version satisfying pin + schema range"
            };

            let row = sqlx::query(&format!(
                r#"INSERT INTO atelier_sourcing_binding_decision
                     (sourcing_spec_id, spec_hash, handler_family, handler_version_pin, bound,
                      resolved_handler_version, matched_entry_id, matrix_snapshot_id,
                      capability_satisfied, resolution_reason)
                   VALUES ($1, $2, $3, $4, TRUE, $5, $6, $7, TRUE, $8)
                   RETURNING {DECISION_COLUMNS}"#
            ))
            .bind(&spec.sourcing_spec_id)
            .bind(&spec.spec_hash)
            .bind(&spec.handler_family)
            .bind(&spec.handler_version_pin)
            .bind(&entry.handler_version)
            .bind(entry.entry_id)
            .bind(matrix_snapshot_id)
            .bind(reason)
            .fetch_one(self.pool())
            .await?;
            let decision = decision_from_row(&row);

            self.record_event(
                SOURCING_BINDING_DECIDED,
                "atelier_sourcing_binding_decision",
                &decision.decision_id.to_string(),
                serde_json::json!({
                    "sourcing_spec_id": decision.sourcing_spec_id,
                    "spec_hash": decision.spec_hash,
                    "resolved_handler_version": decision.resolved_handler_version,
                    "matched_entry_id": decision.matched_entry_id,
                    "matrix_snapshot_id": decision.matrix_snapshot_id,
                    "bound": true,
                }),
            )
            .await?;
            return Ok(decision);
        }

        // No bindable entry: classify the rejection reason deterministically.
        let reason = if pin_matches.is_empty() {
            MismatchReason::NoMatchingVersion
        } else if pin_matches.iter().all(|c| c.status == HandlerStatus::Sunset) {
            MismatchReason::Sunset
        } else {
            // Pin matched, not all sunset, but none in schema range.
            MismatchReason::SchemaUnsupported
        };
        let detail = match reason {
            MismatchReason::NoMatchingVersion => "no handler version satisfied the spec pin",
            MismatchReason::Sunset => "only pin-satisfying handler versions are SUNSET",
            MismatchReason::SchemaUnsupported => {
                "pin-satisfying handler version(s) do not support the spec schema_version"
            }
            MismatchReason::CapabilityDenied => "capability denied",
        };
        self.record_rejection(&spec, matrix_snapshot_id, &evaluated, reason, false, detail)
            .await
    }

    /// Internal: write a non-binding decision + its mismatch receipt in one
    /// transaction (S6.12.4 rule 4). Emits both `SOURCING_BINDING_DECIDED`
    /// (bound=false) and `VERSION_MISMATCH_REJECTED`.
    async fn record_rejection(
        &self,
        spec: &SourcingSpecRecord,
        matrix_snapshot_id: Uuid,
        evaluated_versions: &[String],
        reason: MismatchReason,
        capability_satisfied: bool,
        detail: &str,
    ) -> AtelierResult<BindingDecision> {
        let mut tx = self.pool().begin().await?;

        let decision_row = sqlx::query(&format!(
            r#"INSERT INTO atelier_sourcing_binding_decision
                 (sourcing_spec_id, spec_hash, handler_family, handler_version_pin, bound,
                  resolved_handler_version, matched_entry_id, matrix_snapshot_id,
                  capability_satisfied, resolution_reason)
               VALUES ($1, $2, $3, $4, FALSE, NULL, NULL, $5, $6, $7)
               RETURNING {DECISION_COLUMNS}"#
        ))
        .bind(&spec.sourcing_spec_id)
        .bind(&spec.spec_hash)
        .bind(&spec.handler_family)
        .bind(&spec.handler_version_pin)
        .bind(matrix_snapshot_id)
        .bind(capability_satisfied)
        .bind(detail)
        .fetch_one(&mut *tx)
        .await?;
        let decision = decision_from_row(&decision_row);

        let evaluated_json = serde_json::json!(evaluated_versions);
        sqlx::query(
            r#"INSERT INTO atelier_version_mismatch_receipt
                 (decision_id, sourcing_spec_id, spec_hash, requested_pin, evaluated_versions,
                  matrix_snapshot_id, reason)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(decision.decision_id)
        .bind(&spec.sourcing_spec_id)
        .bind(&spec.spec_hash)
        .bind(&spec.handler_version_pin)
        .bind(&evaluated_json)
        .bind(matrix_snapshot_id)
        .bind(reason.as_token())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.record_event(
            SOURCING_BINDING_DECIDED,
            "atelier_sourcing_binding_decision",
            &decision.decision_id.to_string(),
            serde_json::json!({
                "sourcing_spec_id": decision.sourcing_spec_id,
                "spec_hash": decision.spec_hash,
                "bound": false,
                "matrix_snapshot_id": decision.matrix_snapshot_id,
            }),
        )
        .await?;
        self.record_event(
            VERSION_MISMATCH_REJECTED,
            "atelier_version_mismatch_receipt",
            &decision.decision_id.to_string(),
            serde_json::json!({
                "sourcing_spec_id": spec.sourcing_spec_id,
                "spec_hash": spec.spec_hash,
                "requested_pin": spec.handler_version_pin,
                "reason": reason.as_token(),
                "evaluated_versions": evaluated_versions,
                "matrix_snapshot_id": matrix_snapshot_id,
            }),
        )
        .await?;
        Ok(decision)
    }

    /// Fetch a binding decision by id.
    pub async fn get_binding_decision(&self, decision_id: Uuid) -> AtelierResult<BindingDecision> {
        let row = sqlx::query(&format!(
            "SELECT {DECISION_COLUMNS} FROM atelier_sourcing_binding_decision WHERE decision_id = $1"
        ))
        .bind(decision_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("binding decision {decision_id}")))?;
        Ok(decision_from_row(&row))
    }

    /// Fetch the version-mismatch receipt for a (non-binding) decision, if any.
    pub async fn get_version_mismatch_receipt(
        &self,
        decision_id: Uuid,
    ) -> AtelierResult<Option<VersionMismatchReceipt>> {
        let row = sqlx::query(
            r#"SELECT receipt_id, decision_id, sourcing_spec_id, spec_hash, requested_pin,
                      evaluated_versions, matrix_snapshot_id, reason, created_at_utc
               FROM atelier_version_mismatch_receipt WHERE decision_id = $1"#,
        )
        .bind(decision_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(mismatch_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// Record an idempotent ingestion receipt for a successful binding decision
    /// (S6.12.5). The effective ingestion identity is `(handler_family,
    /// handler_version, spec_hash, idempotency_key?)` (rule 1), serialized into
    /// `ingestion_key`. A repeat ingestion under the same identity returns the
    /// prior receipt with `outcome = deduped` instead of re-recording side
    /// effects; a first ingestion records `outcome = fresh`. Artifact bytes are
    /// referenced by manifest ref only, never inlined (rule 3 / S6.12.6).
    /// `completed_count` / `pending_count` make partial-failure retries
    /// recoverable (rule 4). Recording against a non-binding decision is a
    /// validation error -- a rejected spec never ingests. Emits
    /// `SOURCING_INGESTION_RECEIPTED`.
    pub async fn record_ingestion_receipt(
        &self,
        decision_id: Uuid,
        idempotency_key: Option<&str>,
        artifact_manifest_refs: &[String],
        completed_count: i64,
        pending_count: i64,
    ) -> AtelierResult<IngestionReceipt> {
        let decision = self.get_binding_decision(decision_id).await?;
        if !decision.bound {
            return Err(AtelierError::Validation(format!(
                "cannot ingest against non-binding decision {decision_id}"
            )));
        }
        let handler_version = decision.resolved_handler_version.clone().ok_or_else(|| {
            AtelierError::Validation(format!(
                "bound decision {decision_id} missing resolved_handler_version"
            ))
        })?;

        // Effective ingestion identity (rule 1).
        let ingestion_key = format!(
            "{}|{}|{}|{}",
            decision.handler_family,
            handler_version,
            decision.spec_hash,
            idempotency_key.unwrap_or("")
        );

        // Idempotent fast path: a prior receipt under this identity wins and is
        // re-emitted as `deduped`.
        if let Some(prior) = self.get_ingestion_receipt_by_key(&ingestion_key).await? {
            self.record_event(
                SOURCING_INGESTION_RECEIPTED,
                "atelier_sourcing_ingestion_receipt",
                &prior.receipt_id.to_string(),
                serde_json::json!({
                    "decision_id": decision_id,
                    "ingestion_key": ingestion_key,
                    "outcome": IngestionOutcome::Deduped.as_token(),
                    "artifact_manifest_refs": prior.artifact_manifest_refs,
                }),
            )
            .await?;
            // Return a deduped view of the prior receipt.
            let mut deduped = prior;
            deduped.outcome = IngestionOutcome::Deduped;
            return Ok(deduped);
        }

        let refs_json = serde_json::json!(artifact_manifest_refs);
        let row = sqlx::query(
            r#"INSERT INTO atelier_sourcing_ingestion_receipt
                 (decision_id, ingestion_key, handler_family, handler_version, spec_hash,
                  artifact_manifest_refs, outcome, completed_count, pending_count)
               VALUES ($1, $2, $3, $4, $5, $6, 'fresh', $7, $8)
               ON CONFLICT (ingestion_key) DO UPDATE SET ingestion_key = EXCLUDED.ingestion_key
               RETURNING receipt_id, decision_id, ingestion_key, handler_family, handler_version,
                         spec_hash, artifact_manifest_refs, outcome, completed_count,
                         pending_count, created_at_utc"#,
        )
        .bind(decision_id)
        .bind(&ingestion_key)
        .bind(&decision.handler_family)
        .bind(&handler_version)
        .bind(&decision.spec_hash)
        .bind(&refs_json)
        .bind(completed_count)
        .bind(pending_count)
        .fetch_one(self.pool())
        .await?;
        let receipt = ingestion_from_row(&row)?;

        self.record_event(
            SOURCING_INGESTION_RECEIPTED,
            "atelier_sourcing_ingestion_receipt",
            &receipt.receipt_id.to_string(),
            serde_json::json!({
                "decision_id": receipt.decision_id,
                "ingestion_key": receipt.ingestion_key,
                "outcome": receipt.outcome.as_token(),
                "completed_count": receipt.completed_count,
                "pending_count": receipt.pending_count,
                "artifact_manifest_refs": receipt.artifact_manifest_refs,
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// Fetch an ingestion receipt by its effective ingestion identity key.
    pub async fn get_ingestion_receipt_by_key(
        &self,
        ingestion_key: &str,
    ) -> AtelierResult<Option<IngestionReceipt>> {
        let row = sqlx::query(
            r#"SELECT receipt_id, decision_id, ingestion_key, handler_family, handler_version,
                      spec_hash, artifact_manifest_refs, outcome, completed_count,
                      pending_count, created_at_utc
               FROM atelier_sourcing_ingestion_receipt WHERE ingestion_key = $1"#,
        )
        .bind(ingestion_key)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(ingestion_from_row(&r)?)),
            None => Ok(None),
        }
    }
}
