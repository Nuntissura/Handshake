//! Stealth Reference Window state + read-only projection (MT-205).
//!
//! Implements the governed DATA + EVIDENCE model for the Stealth Reference
//! Window defined in spec module 10-product-surfaces.md Section 10.18 (Stealth
//! Reference Window, Normative, [ADD v02.189]):
//!   * 10.18.2 window-registry state model (`StealthReferenceWindow`),
//!   * 10.18.3 content-reference contract (`ContentRef`),
//!   * 10.18.4 visibility + quiet flags (`VisibilityFlag` / `QuietFlags`),
//!   * 10.18.5 read-only projection (the list/get/resolve read paths plus the
//!     governed mutations upsert/add/remove/reorder),
//!   * 10.18.6 EventLedger evidence obligations (every mutation emits an event),
//!   * 10.18.7 storage / portability / quiet guardrails.
//!
//! This module is STATE + PROJECTION ONLY. It NEVER controls an OS window,
//! never opens a `tauri` window, never calls a forbidden focus/foreground API,
//! never injects synthetic input, never spawns a process or opens a socket. The
//! actual off-screen window + CDP capture is an out-of-module, capability-gated
//! Workflow-Engine job (Section 10.18.5/6); this module only stores the governed
//! registry records + content references + receipts and emits events. Per
//! HBR-QUIET (Section 6.6) and [GLOBAL-BUILD-046]..[GLOBAL-BUILD-054] the
//! registry is non-intrusive by construction: quiet flags default ON and the
//! store refuses to persist a non-quiet window outside an audited foreground
//! exception.
//!
//! CKC source (intent only, never copied): `app/backend/automationStealth.js`
//! (background-stealth contract -- no visible window, no focus steal, no taskbar,
//! quiet by default, skipped attention surfaces logged). CKC stored this as an
//! Electron/localhost runtime flag with no durable governed state; Handshake
//! forbids that and instead persists a typed registry on PostgreSQL
//! (TECH-POSTGRESQL). SQLite is forbidden in every path (MT-004).
//!
//! Storage authority: PostgreSQL via [`super::AtelierStore::pool`] (sqlx 0.8).
//! Microtasks: MT-205 (stealth reference window state + projection), MT-005
//! (event coverage).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

/// Stealth Reference Window event families (MT-205, MT-005).
///
/// Defined here so the parent can fold these into [`super::event_family::ALL`]
/// and the MT-005 coverage check picks up stealth-window mutations. Mirrors the
/// EventLedger event names mandated by Section 10.18.6.
pub mod stealth_ref_event_family {
    /// A registry entry (window) was created.
    pub const STEALTH_REF_WINDOW_CREATED: &str = "atelier.stealth_ref.window_created";
    /// A `ContentRef` was added to a window.
    pub const STEALTH_REF_ADDED: &str = "atelier.stealth_ref.added";
    /// A `ContentRef` was removed from a window.
    pub const STEALTH_REF_REMOVED: &str = "atelier.stealth_ref.removed";
    /// A window's content references were reordered / layout repinned.
    pub const STEALTH_REF_REORDERED: &str = "atelier.stealth_ref.reordered";
    /// An off-screen capture receipt was recorded for a window.
    pub const STEALTH_REF_CAPTURED: &str = "atelier.stealth_ref.captured";
    /// A window registry entry was closed.
    pub const STEALTH_REF_WINDOW_CLOSED: &str = "atelier.stealth_ref.window_closed";

    /// All stealth-ref event families (parity/coverage helper).
    pub const ALL: &[&str] = &[
        STEALTH_REF_WINDOW_CREATED,
        STEALTH_REF_ADDED,
        STEALTH_REF_REMOVED,
        STEALTH_REF_REORDERED,
        STEALTH_REF_CAPTURED,
        STEALTH_REF_WINDOW_CLOSED,
    ];
}

/// Re-export at module root so callers can write `stealth_window::STEALTH_REF_ADDED`.
pub use stealth_ref_event_family::{
    STEALTH_REF_ADDED, STEALTH_REF_CAPTURED, STEALTH_REF_REMOVED, STEALTH_REF_REORDERED,
    STEALTH_REF_WINDOW_CLOSED, STEALTH_REF_WINDOW_CREATED,
};

/// Visibility mode of a stealth reference window (Section 10.18.4).
///
/// `OffScreenOnly` is the default and the only non-intrusive mode that needs no
/// host surface: the window is never composited to a visible surface and is
/// consumed solely via IPC reads + off-screen CDP capture. `DiagnosticEmbed`
/// renders inside an already-visible diagnostics/operator surface the operator
/// opened deliberately. `ForegroundExceptionBound` is reachable ONLY under the
/// Section 6.6.7 per-packet, audited foreground exception.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityFlag {
    OffScreenOnly,
    DiagnosticEmbed,
    ForegroundExceptionBound,
}

impl VisibilityFlag {
    /// Stable lowercase DB token persisted in the `visibility` column.
    pub fn as_token(self) -> &'static str {
        match self {
            VisibilityFlag::OffScreenOnly => "off_screen_only",
            VisibilityFlag::DiagnosticEmbed => "diagnostic_embed",
            VisibilityFlag::ForegroundExceptionBound => "foreground_exception_bound",
        }
    }

    /// Parse a stored token. Unknown tokens are a validation error rather than a
    /// silent default, so a corrupt row never masquerades as off-screen.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "off_screen_only" => Ok(VisibilityFlag::OffScreenOnly),
            "diagnostic_embed" => Ok(VisibilityFlag::DiagnosticEmbed),
            "foreground_exception_bound" => Ok(VisibilityFlag::ForegroundExceptionBound),
            other => Err(AtelierError::Validation(format!(
                "unknown stealth visibility token: {other}"
            ))),
        }
    }

    /// Whether this visibility requires the audited per-packet foreground
    /// exception (Section 6.6.7). Only `ForegroundExceptionBound` does.
    fn requires_foreground_exception(self) -> bool {
        matches!(self, VisibilityFlag::ForegroundExceptionBound)
    }
}

/// Quiet invariants for a stealth window (Section 10.18.4). All default `true`;
/// the store refuses to persist a window whose quiet flags are not all `true`
/// except under an audited `ForegroundExceptionBound` packet. Inverting any flag
/// outside that exception is a HBR-QUIET violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuietFlags {
    pub no_focus_steal: bool,
    pub no_foreground: bool,
    pub no_taskbar: bool,
    pub no_global_shortcut: bool,
    pub no_synthetic_input: bool,
}

impl Default for QuietFlags {
    /// All quiet invariants ON, the non-intrusive default (Section 10.18.4).
    fn default() -> Self {
        Self {
            no_focus_steal: true,
            no_foreground: true,
            no_taskbar: true,
            no_global_shortcut: true,
            no_synthetic_input: true,
        }
    }
}

impl QuietFlags {
    /// Whether every quiet invariant is held ON (fully non-intrusive).
    pub fn all_quiet(&self) -> bool {
        self.no_focus_steal
            && self.no_foreground
            && self.no_taskbar
            && self.no_global_shortcut
            && self.no_synthetic_input
    }

    fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "no_focus_steal": self.no_focus_steal,
            "no_foreground": self.no_foreground,
            "no_taskbar": self.no_taskbar,
            "no_global_shortcut": self.no_global_shortcut,
            "no_synthetic_input": self.no_synthetic_input,
        })
    }

    fn from_json(value: &serde_json::Value) -> AtelierResult<Self> {
        let read = |key: &str| -> AtelierResult<bool> {
            value
                .get(key)
                .and_then(serde_json::Value::as_bool)
                .ok_or_else(|| {
                    AtelierError::Validation(format!("quiet flags missing boolean field: {key}"))
                })
        };
        Ok(QuietFlags {
            no_focus_steal: read("no_focus_steal")?,
            no_foreground: read("no_foreground")?,
            no_taskbar: read("no_taskbar")?,
            no_global_shortcut: read("no_global_shortcut")?,
            no_synthetic_input: read("no_synthetic_input")?,
        })
    }
}

/// Lifecycle status of a registry entry (Section 10.18.2/10.18.6). `Open` is the
/// live registry entry; `Closed` is a soft-closed entry retained for audit (no
/// silent deletes of governed state).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StealthRefStatus {
    Open,
    Closed,
}

impl StealthRefStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            StealthRefStatus::Open => "open",
            StealthRefStatus::Closed => "closed",
        }
    }

    fn from_token(token: &str) -> StealthRefStatus {
        match token {
            "closed" => StealthRefStatus::Closed,
            _ => StealthRefStatus::Open,
        }
    }
}

/// Kind of governed source a `ContentRef` points at (Section 10.18.3).
///
/// A content reference is a typed pointer to canonical state already
/// materialized elsewhere; it MUST resolve through a governed source and MUST
/// NOT embed raw media, transcripts, or secrets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentRefKind {
    /// An ArtifactStore manifest entry (PRIM-ArtifactManifest).
    Artifact,
    /// A spec-router section id.
    SpecAnchor,
    /// An ASR transcript artifact (FEAT-ASR).
    Transcript,
    /// A Workflow-Engine / AI-Job receipt.
    JobReceipt,
    /// An EventLedger event id.
    LedgerEvent,
    /// An off-screen capture artifact (Section 6.6.2).
    Screenshot,
    /// A PRIM-DiagnosticSurface row.
    Diagnostic,
}

impl ContentRefKind {
    pub fn as_token(self) -> &'static str {
        match self {
            ContentRefKind::Artifact => "artifact",
            ContentRefKind::SpecAnchor => "spec_anchor",
            ContentRefKind::Transcript => "transcript",
            ContentRefKind::JobReceipt => "job_receipt",
            ContentRefKind::LedgerEvent => "ledger_event",
            ContentRefKind::Screenshot => "screenshot",
            ContentRefKind::Diagnostic => "diagnostic",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "artifact" => Ok(ContentRefKind::Artifact),
            "spec_anchor" => Ok(ContentRefKind::SpecAnchor),
            "transcript" => Ok(ContentRefKind::Transcript),
            "job_receipt" => Ok(ContentRefKind::JobReceipt),
            "ledger_event" => Ok(ContentRefKind::LedgerEvent),
            "screenshot" => Ok(ContentRefKind::Screenshot),
            "diagnostic" => Ok(ContentRefKind::Diagnostic),
            other => Err(AtelierError::Validation(format!(
                "unknown content ref kind token: {other}"
            ))),
        }
    }
}

/// A typed pointer to canonical, governed state (Section 10.18.3).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentRef {
    pub ref_id: Uuid,
    pub window_ref_id: Uuid,
    /// Position in the window's ordered content list (0-based; reorder rewrites).
    pub seq: i64,
    pub ref_kind: ContentRefKind,
    /// Canonical governed locator: an ArtifactStore manifest id, a spec section
    /// id, an EventLedger id, or a DiagnosticSurface id. NEVER a drive-letter /
    /// user-profile / machine-local filesystem path and NEVER a localhost or
    /// process-local source (Section 10.18.3/10.18.7).
    pub resolver: String,
    /// SHA256 of the resolved payload at pin time; lets diagnostics detect drift
    /// between the pinned reference and current canonical state.
    pub content_sha256: String,
    /// Asserts the resolved view is already scrubbed of secrets/cookies/tokens
    /// (Section 10.18.3/10.18.7). The store refuses to persist a ref whose view
    /// is not asserted redacted, so no raw secret can ever enter the registry.
    pub redaction_state: bool,
    pub pinned_at_utc: DateTime<Utc>,
}

/// A stealth-reference-window registry entry (Section 10.18.2).
///
/// Holds REFERENCES only -- never inline product content, media, or secrets --
/// so the registry stays small, portable, and payload-free. `layout` is a
/// LOGICAL descriptor (panel order, scroll offsets), never physical screen
/// coordinates, keeping the registry disk- and display-agnostic
/// ([GLOBAL-PORTABILITY-004]).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StealthReferenceWindow {
    pub window_ref_id: Uuid,
    /// The operator or governed-agent actor that created the entry.
    pub owner_actor: String,
    /// ASCII display label; used by diagnostics/automation only, never rendered
    /// to foreground.
    pub title: String,
    pub visibility: VisibilityFlag,
    pub quiet: QuietFlags,
    /// Logical layout descriptor (panel order, scroll offsets); NOT physical
    /// screen coordinates.
    pub layout: serde_json::Value,
    pub status: StealthRefStatus,
    /// Monotonically increasing revision (Section 10.18.2); bumped on every
    /// governed mutation.
    pub revision: i64,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// Input to create / idempotently re-open a registry entry.
#[derive(Clone, Debug)]
pub struct NewStealthWindow {
    pub owner_actor: String,
    pub title: String,
    pub visibility: VisibilityFlag,
    pub quiet: QuietFlags,
    /// Optional logical layout descriptor; defaults to an empty object.
    pub layout: Option<serde_json::Value>,
}

/// Input to add a content reference to a window.
#[derive(Clone, Debug)]
pub struct NewContentRef {
    pub ref_kind: ContentRefKind,
    pub resolver: String,
    pub content_sha256: String,
    pub redaction_state: bool,
}

const WINDOW_COLUMNS: &str = "window_ref_id, owner_actor, title, visibility, quiet_json, \
                              layout_json, status, revision, created_at_utc, updated_at_utc";

const REF_COLUMNS: &str = "ref_id, window_ref_id, seq, ref_kind, resolver, content_sha256, \
                           redaction_state, pinned_at_utc";

fn window_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<StealthReferenceWindow> {
    let visibility_token: String = row.get("visibility");
    let status_token: String = row.get("status");
    let quiet_json: serde_json::Value = row.get("quiet_json");
    Ok(StealthReferenceWindow {
        window_ref_id: row.get("window_ref_id"),
        owner_actor: row.get("owner_actor"),
        title: row.get("title"),
        visibility: VisibilityFlag::from_token(&visibility_token)?,
        quiet: QuietFlags::from_json(&quiet_json)?,
        layout: row.get("layout_json"),
        status: StealthRefStatus::from_token(&status_token),
        revision: row.get("revision"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn ref_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<ContentRef> {
    let kind_token: String = row.get("ref_kind");
    Ok(ContentRef {
        ref_id: row.get("ref_id"),
        window_ref_id: row.get("window_ref_id"),
        seq: row.get("seq"),
        ref_kind: ContentRefKind::from_token(&kind_token)?,
        resolver: row.get("resolver"),
        content_sha256: row.get("content_sha256"),
        redaction_state: row.get("redaction_state"),
        pinned_at_utc: row.get("pinned_at_utc"),
    })
}

/// Validate the non-intrusive quiet invariant (Section 10.18.4): quiet flags
/// must all be ON unless the window is an audited `ForegroundExceptionBound`
/// entry. This is the data-side guard that keeps the registry non-intrusive by
/// construction; the actual OS quiet-mode window config and focus-audit live in
/// the out-of-module Tauri/Workflow-Engine path.
fn validate_quiet(visibility: VisibilityFlag, quiet: &QuietFlags) -> AtelierResult<()> {
    if quiet.all_quiet() {
        return Ok(());
    }
    if visibility.requires_foreground_exception() {
        // Only an explicitly foreground-exception-bound window may relax quiet
        // flags, and only under the audited per-packet exception upstream.
        return Ok(());
    }
    Err(AtelierError::Validation(
        "stealth reference window must keep all quiet flags ON \
         (HBR-QUIET) outside an audited ForegroundExceptionBound window"
            .into(),
    ))
}

/// Validate a content-reference resolver (Section 10.18.3/10.18.7 LAW): a
/// resolver MUST be a governed id (ArtifactStore manifest id, spec section id,
/// EventLedger id, DiagnosticSurface id). It MUST NOT be a localhost / process-
/// local source and MUST NOT be a drive-letter / user-profile / machine-local
/// filesystem path. There is no "localhost authority" for reference content.
fn validate_resolver(resolver: &str) -> AtelierResult<()> {
    let trimmed = resolver.trim();
    if trimmed.is_empty() {
        return Err(AtelierError::Validation(
            "content ref resolver must not be empty".into(),
        ));
    }
    let lowered = trimmed.to_ascii_lowercase();

    // Reject localhost / process-local authority.
    let forbidden_authority = [
        "localhost",
        "127.0.0.1",
        "0.0.0.0",
        "::1",
        "http://",
        "https://",
        "ws://",
        "file://",
    ];
    for needle in forbidden_authority {
        if lowered.contains(needle) {
            return Err(AtelierError::Validation(format!(
                "content ref resolver must be a governed id, not a localhost / \
                 network / file authority: {resolver:?}"
            )));
        }
    }

    // Reject machine-local filesystem paths (drive letters, UNC, posix roots,
    // user-profile prefixes) so the registry stays disk-agnostic.
    let looks_like_drive = trimmed.len() >= 2
        && trimmed.as_bytes()[0].is_ascii_alphabetic()
        && trimmed.as_bytes()[1] == b':';
    let looks_like_path = looks_like_drive
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || lowered.starts_with("~/")
        || lowered.contains(":\\");
    if looks_like_path {
        return Err(AtelierError::Validation(format!(
            "content ref resolver must be a portable governed id, not a \
             machine-local filesystem path: {resolver:?}"
        )));
    }
    Ok(())
}

impl AtelierStore {
    /// Create a stealth reference window registry entry, or idempotently return
    /// the existing entry for the same `(owner_actor, title)` (Section 10.18.2).
    ///
    /// Re-creating the same titled window for the same actor is therefore safe
    /// and never duplicates a registry row; the existing entry (with its
    /// content refs and revision) wins. Quiet flags are validated up front: a
    /// non-quiet window is rejected outside an audited `ForegroundExceptionBound`
    /// entry (Section 10.18.4, HBR-QUIET). Emits `STEALTH_REF_WINDOW_CREATED`.
    /// This stores registry STATE only -- it never opens or shows an OS window.
    pub async fn create_stealth_window(
        &self,
        new: &NewStealthWindow,
    ) -> AtelierResult<StealthReferenceWindow> {
        if new.owner_actor.trim().is_empty() {
            return Err(AtelierError::Validation(
                "stealth window owner_actor must not be empty".into(),
            ));
        }
        if new.title.trim().is_empty() {
            return Err(AtelierError::Validation(
                "stealth window title must not be empty".into(),
            ));
        }
        if !new.title.is_ascii() {
            return Err(AtelierError::Validation(
                "stealth window title must be ASCII (diagnostics/automation label)".into(),
            ));
        }
        validate_quiet(new.visibility, &new.quiet)?;

        // Idempotent fast path: an existing entry for this actor+title wins.
        if let Some(existing) = self
            .get_stealth_window_by_title(&new.owner_actor, &new.title)
            .await?
        {
            return Ok(existing);
        }

        let layout = new.layout.clone().unwrap_or_else(|| serde_json::json!({}));

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_stealth_window
                 (owner_actor, title, visibility, quiet_json, layout_json, status, revision)
               VALUES ($1, $2, $3, $4, $5, 'open', 1)
               ON CONFLICT (owner_actor, title) DO UPDATE
                 SET owner_actor = EXCLUDED.owner_actor
               RETURNING {WINDOW_COLUMNS}"#
        ))
        .bind(&new.owner_actor)
        .bind(&new.title)
        .bind(new.visibility.as_token())
        .bind(new.quiet.to_json())
        .bind(&layout)
        .fetch_one(self.pool())
        .await?;
        let window = window_from_row(&row)?;

        self.record_event(
            STEALTH_REF_WINDOW_CREATED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
            serde_json::json!({
                "window_ref_id": window.window_ref_id,
                "owner_actor": window.owner_actor,
                "title": window.title,
                "visibility": window.visibility.as_token(),
                "quiet": window.quiet.to_json(),
                "revision": window.revision,
            }),
        )
        .await?;
        Ok(window)
    }

    /// Fetch a window by `(owner_actor, title)`.
    pub async fn get_stealth_window_by_title(
        &self,
        owner_actor: &str,
        title: &str,
    ) -> AtelierResult<Option<StealthReferenceWindow>> {
        let row = sqlx::query(&format!(
            r#"SELECT {WINDOW_COLUMNS}
               FROM atelier_stealth_window
               WHERE owner_actor = $1 AND title = $2"#
        ))
        .bind(owner_actor)
        .bind(title)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(window_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// Fetch a single window registry entry by id (Section 10.18.5
    /// `get_window`).
    pub async fn get_stealth_window(
        &self,
        window_ref_id: Uuid,
    ) -> AtelierResult<StealthReferenceWindow> {
        let row = sqlx::query(&format!(
            r#"SELECT {WINDOW_COLUMNS}
               FROM atelier_stealth_window
               WHERE window_ref_id = $1"#
        ))
        .bind(window_ref_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("stealth window {window_ref_id}")))?;
        window_from_row(&row)
    }

    /// List the registry entries visible to an actor (Section 10.18.5
    /// `list_windows`), newest first, optionally filtered to a status. The
    /// projection is READ-ONLY; it never mutates registry state.
    pub async fn list_stealth_windows(
        &self,
        owner_actor: &str,
        status: Option<StealthRefStatus>,
        limit: i64,
    ) -> AtelierResult<Vec<StealthReferenceWindow>> {
        let capped = limit.clamp(1, 1000);
        let rows = sqlx::query(&format!(
            r#"SELECT {WINDOW_COLUMNS}
               FROM atelier_stealth_window
               WHERE owner_actor = $1
                 AND ($2::TEXT IS NULL OR status = $2)
               ORDER BY updated_at_utc DESC
               LIMIT $3"#
        ))
        .bind(owner_actor)
        .bind(status.map(|s| s.as_token()))
        .bind(capped)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(window_from_row).collect()
    }

    /// Add a content reference to a window (Section 10.18.5 `add_ref`).
    ///
    /// Validates the governed-source LAW (Section 10.18.3): the resolver must be
    /// a portable governed id, never a localhost / network / file authority and
    /// never a machine-local filesystem path. The resolved view must be asserted
    /// redacted (`redaction_state = true`) so no raw secret can enter the
    /// registry (Section 10.18.7 secret hygiene). The new ref is appended at the
    /// next sequence inside a transaction so concurrent adds cannot collide on
    /// `(window_ref_id, seq)`. Bumps the window `revision` and emits
    /// `STEALTH_REF_ADDED`.
    pub async fn add_stealth_ref(
        &self,
        window_ref_id: Uuid,
        new: &NewContentRef,
    ) -> AtelierResult<ContentRef> {
        validate_resolver(&new.resolver)?;
        if new.content_sha256.trim().is_empty() {
            return Err(AtelierError::Validation(
                "content ref content_sha256 must not be empty".into(),
            ));
        }
        if !new.redaction_state {
            return Err(AtelierError::Validation(
                "content ref must assert redaction_state = true; \
                 secrets/cookies/tokens MUST be scrubbed before pinning"
                    .into(),
            ));
        }

        let mut tx = self.pool().begin().await?;

        // Guard the FK + closed-state explicitly so a bad window id or a closed
        // window is a clean validation/not-found error, not a raw constraint hit.
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM atelier_stealth_window WHERE window_ref_id = $1 FOR UPDATE",
        )
        .bind(window_ref_id)
        .fetch_optional(&mut *tx)
        .await?;
        match status.as_deref() {
            None => {
                return Err(AtelierError::NotFound(format!(
                    "stealth window {window_ref_id}"
                )))
            }
            Some("closed") => {
                return Err(AtelierError::Validation(format!(
                    "cannot add a content ref to closed stealth window {window_ref_id}"
                )));
            }
            Some(_) => {}
        }

        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), -1) + 1 FROM atelier_stealth_ref WHERE window_ref_id = $1",
        )
        .bind(window_ref_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_stealth_ref
                 (window_ref_id, seq, ref_kind, resolver, content_sha256, redaction_state)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING {REF_COLUMNS}"#
        ))
        .bind(window_ref_id)
        .bind(next_seq)
        .bind(new.ref_kind.as_token())
        .bind(&new.resolver)
        .bind(&new.content_sha256)
        .bind(new.redaction_state)
        .fetch_one(&mut *tx)
        .await?;

        let new_revision: i64 = sqlx::query_scalar(
            r#"UPDATE atelier_stealth_window
               SET revision = revision + 1, updated_at_utc = NOW()
               WHERE window_ref_id = $1
               RETURNING revision"#,
        )
        .bind(window_ref_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let content_ref = ref_from_row(&row)?;
        self.record_event(
            STEALTH_REF_ADDED,
            "atelier_stealth_window",
            &window_ref_id.to_string(),
            serde_json::json!({
                "window_ref_id": window_ref_id,
                "ref_id": content_ref.ref_id,
                "ref_kind": content_ref.ref_kind.as_token(),
                "seq": content_ref.seq,
                // Resolver is a governed id (no secret); content hash only.
                "resolver": content_ref.resolver,
                "content_sha256": content_ref.content_sha256,
                "revision": new_revision,
            }),
        )
        .await?;
        Ok(content_ref)
    }

    /// The ordered content references for a window (ascending sequence). This is
    /// part of the read-only projection (Section 10.18.5); it returns reference
    /// METADATA only, never resolved raw payloads.
    pub async fn list_stealth_refs(&self, window_ref_id: Uuid) -> AtelierResult<Vec<ContentRef>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {REF_COLUMNS}
               FROM atelier_stealth_ref
               WHERE window_ref_id = $1
               ORDER BY seq ASC"#
        ))
        .bind(window_ref_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(ref_from_row).collect()
    }

    /// Remove a content reference from a window (Section 10.18.5 `remove_ref`).
    ///
    /// Returns whether a ref was removed. Bumps the window `revision` and emits
    /// `STEALTH_REF_REMOVED` when a row is actually removed. Remaining refs keep
    /// their existing sequence values (gaps are tolerated; ordering is by `seq`
    /// ascending). Use [`Self::reorder_stealth_refs`] to compact / repin order.
    pub async fn remove_stealth_ref(
        &self,
        window_ref_id: Uuid,
        ref_id: Uuid,
    ) -> AtelierResult<bool> {
        let mut tx = self.pool().begin().await?;

        let removed: Option<Uuid> = sqlx::query_scalar(
            r#"DELETE FROM atelier_stealth_ref
               WHERE window_ref_id = $1 AND ref_id = $2
               RETURNING ref_id"#,
        )
        .bind(window_ref_id)
        .bind(ref_id)
        .fetch_optional(&mut *tx)
        .await?;

        if removed.is_none() {
            tx.rollback().await?;
            return Ok(false);
        }

        let new_revision: i64 = sqlx::query_scalar(
            r#"UPDATE atelier_stealth_window
               SET revision = revision + 1, updated_at_utc = NOW()
               WHERE window_ref_id = $1
               RETURNING revision"#,
        )
        .bind(window_ref_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        self.record_event(
            STEALTH_REF_REMOVED,
            "atelier_stealth_window",
            &window_ref_id.to_string(),
            serde_json::json!({
                "window_ref_id": window_ref_id,
                "ref_id": ref_id,
                "revision": new_revision,
            }),
        )
        .await?;
        Ok(true)
    }

    /// Reorder a window's content references and/or repin its layout (Section
    /// 10.18.5 `reorder`).
    ///
    /// `ordered_ref_ids` MUST be exactly the current set of `ref_id`s for the
    /// window (a permutation, no missing/extra/duplicate ids) so an operator can
    /// never silently drop a reference by reordering. New `seq` values are
    /// assigned 0..N in the supplied order inside a transaction. An optional new
    /// logical `layout` is repinned in the same mutation. Bumps the window
    /// `revision` and emits `STEALTH_REF_REORDERED`.
    pub async fn reorder_stealth_refs(
        &self,
        window_ref_id: Uuid,
        ordered_ref_ids: &[Uuid],
        layout: Option<&serde_json::Value>,
    ) -> AtelierResult<StealthReferenceWindow> {
        let mut tx = self.pool().begin().await?;

        // Lock the window row and confirm it exists / is open.
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM atelier_stealth_window WHERE window_ref_id = $1 FOR UPDATE",
        )
        .bind(window_ref_id)
        .fetch_optional(&mut *tx)
        .await?;
        match status.as_deref() {
            None => {
                return Err(AtelierError::NotFound(format!(
                    "stealth window {window_ref_id}"
                )))
            }
            Some("closed") => {
                return Err(AtelierError::Validation(format!(
                    "cannot reorder refs on closed stealth window {window_ref_id}"
                )));
            }
            Some(_) => {}
        }

        // The supplied order must be exactly the current ref set (a permutation).
        let current: Vec<Uuid> =
            sqlx::query_scalar("SELECT ref_id FROM atelier_stealth_ref WHERE window_ref_id = $1")
                .bind(window_ref_id)
                .fetch_all(&mut *tx)
                .await?;

        let mut want = ordered_ref_ids.to_vec();
        want.sort();
        want.dedup();
        if want.len() != ordered_ref_ids.len() {
            return Err(AtelierError::Validation(
                "reorder list must not contain duplicate ref_ids".into(),
            ));
        }
        let mut have = current.clone();
        have.sort();
        if want != have {
            return Err(AtelierError::Validation(format!(
                "reorder list must be exactly the current {} ref(s) of stealth window {window_ref_id} \
                 (no missing/extra ids)",
                current.len()
            )));
        }

        // Two-phase reassignment to dodge the (window_ref_id, seq) unique
        // constraint: first push every seq into a disjoint high range, then set
        // the final 0..N values in the requested order.
        sqlx::query(
            "UPDATE atelier_stealth_ref SET seq = seq + 1000000000 WHERE window_ref_id = $1",
        )
        .bind(window_ref_id)
        .execute(&mut *tx)
        .await?;

        for (idx, ref_id) in ordered_ref_ids.iter().enumerate() {
            sqlx::query(
                "UPDATE atelier_stealth_ref SET seq = $3 WHERE window_ref_id = $1 AND ref_id = $2",
            )
            .bind(window_ref_id)
            .bind(ref_id)
            .bind(idx as i64)
            .execute(&mut *tx)
            .await?;
        }

        let row = match layout {
            Some(new_layout) => {
                sqlx::query(&format!(
                    r#"UPDATE atelier_stealth_window
                       SET layout_json = $2, revision = revision + 1, updated_at_utc = NOW()
                       WHERE window_ref_id = $1
                       RETURNING {WINDOW_COLUMNS}"#
                ))
                .bind(window_ref_id)
                .bind(new_layout)
                .fetch_one(&mut *tx)
                .await?
            }
            None => {
                sqlx::query(&format!(
                    r#"UPDATE atelier_stealth_window
                       SET revision = revision + 1, updated_at_utc = NOW()
                       WHERE window_ref_id = $1
                       RETURNING {WINDOW_COLUMNS}"#
                ))
                .bind(window_ref_id)
                .fetch_one(&mut *tx)
                .await?
            }
        };

        tx.commit().await?;

        let window = window_from_row(&row)?;
        self.record_event(
            STEALTH_REF_REORDERED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
            serde_json::json!({
                "window_ref_id": window.window_ref_id,
                "ordered_ref_ids": ordered_ref_ids,
                "relayout": layout.is_some(),
                "revision": window.revision,
            }),
        )
        .await?;
        Ok(window)
    }

    /// Record an off-screen capture receipt for a window (Section 10.18.5/6).
    ///
    /// The actual off-screen CDP capture (`Page.captureScreenshot`) runs as an
    /// out-of-module, capability-gated Workflow-Engine job; THIS method only
    /// records the governed receipt: the ArtifactStore manifest id of the
    /// produced screenshot plus its content hash. Idempotent on
    /// `(window_ref_id, artifact_manifest_id)` so re-recording the same capture
    /// returns the existing receipt rather than duplicating it. Bumps the window
    /// `revision` and emits `STEALTH_REF_CAPTURED`. The manifest id is a governed
    /// id (validated like a resolver); no raw pixels or paths are stored here.
    pub async fn record_stealth_capture(
        &self,
        window_ref_id: Uuid,
        artifact_manifest_id: &str,
        content_sha256: &str,
    ) -> AtelierResult<StealthCaptureReceipt> {
        validate_resolver(artifact_manifest_id)?;
        if content_sha256.trim().is_empty() {
            return Err(AtelierError::Validation(
                "capture content_sha256 must not be empty".into(),
            ));
        }
        // Guard: the window must exist (capture receipt can never dangle).
        let _ = self.get_stealth_window(window_ref_id).await?;

        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_stealth_capture
                 (window_ref_id, artifact_manifest_id, content_sha256)
               VALUES ($1, $2, $3)
               ON CONFLICT (window_ref_id, artifact_manifest_id)
                 DO UPDATE SET content_sha256 = EXCLUDED.content_sha256
               RETURNING capture_id, window_ref_id, artifact_manifest_id,
                         content_sha256, captured_at_utc"#,
        )
        .bind(window_ref_id)
        .bind(artifact_manifest_id)
        .bind(content_sha256)
        .fetch_one(&mut *tx)
        .await?;

        let new_revision: i64 = sqlx::query_scalar(
            r#"UPDATE atelier_stealth_window
               SET revision = revision + 1, updated_at_utc = NOW()
               WHERE window_ref_id = $1
               RETURNING revision"#,
        )
        .bind(window_ref_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let receipt = StealthCaptureReceipt {
            capture_id: row.get("capture_id"),
            window_ref_id: row.get("window_ref_id"),
            artifact_manifest_id: row.get("artifact_manifest_id"),
            content_sha256: row.get("content_sha256"),
            captured_at_utc: row.get("captured_at_utc"),
        };

        self.record_event(
            STEALTH_REF_CAPTURED,
            "atelier_stealth_window",
            &window_ref_id.to_string(),
            serde_json::json!({
                "window_ref_id": window_ref_id,
                "capture_id": receipt.capture_id,
                "artifact_manifest_id": receipt.artifact_manifest_id,
                "content_sha256": receipt.content_sha256,
                "revision": new_revision,
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// The capture receipts recorded for a window, newest first (read-only
    /// projection; Section 10.18.5).
    pub async fn list_stealth_captures(
        &self,
        window_ref_id: Uuid,
    ) -> AtelierResult<Vec<StealthCaptureReceipt>> {
        let rows = sqlx::query(
            r#"SELECT capture_id, window_ref_id, artifact_manifest_id, content_sha256,
                      captured_at_utc
               FROM atelier_stealth_capture
               WHERE window_ref_id = $1
               ORDER BY captured_at_utc DESC"#,
        )
        .bind(window_ref_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows
            .iter()
            .map(|row| StealthCaptureReceipt {
                capture_id: row.get("capture_id"),
                window_ref_id: row.get("window_ref_id"),
                artifact_manifest_id: row.get("artifact_manifest_id"),
                content_sha256: row.get("content_sha256"),
                captured_at_utc: row.get("captured_at_utc"),
            })
            .collect())
    }

    /// Soft-close a window registry entry (Section 10.18.2/6). The row and its
    /// content refs are retained for audit (no silent deletes of governed
    /// state); only the status flips. Bumps `revision` and emits
    /// `STEALTH_REF_WINDOW_CLOSED`. Closing an already-closed window is
    /// idempotent and returns the entry unchanged in status.
    pub async fn close_stealth_window(
        &self,
        window_ref_id: Uuid,
    ) -> AtelierResult<StealthReferenceWindow> {
        let row = sqlx::query(&format!(
            r#"UPDATE atelier_stealth_window
               SET status = 'closed',
                   revision = revision + 1,
                   updated_at_utc = NOW()
               WHERE window_ref_id = $1
               RETURNING {WINDOW_COLUMNS}"#
        ))
        .bind(window_ref_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("stealth window {window_ref_id}")))?;
        let window = window_from_row(&row)?;

        self.record_event(
            STEALTH_REF_WINDOW_CLOSED,
            "atelier_stealth_window",
            &window.window_ref_id.to_string(),
            serde_json::json!({
                "window_ref_id": window.window_ref_id,
                "owner_actor": window.owner_actor,
                "revision": window.revision,
            }),
        )
        .await?;
        Ok(window)
    }
}

/// A governed off-screen-capture receipt (Section 10.18.6 ArtifactStore
/// receipt). Holds the ArtifactStore manifest id + content hash only; never raw
/// pixels or machine-local paths.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StealthCaptureReceipt {
    pub capture_id: Uuid,
    pub window_ref_id: Uuid,
    pub artifact_manifest_id: String,
    pub content_sha256: String,
    pub captured_at_utc: DateTime<Utc>,
}

#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn quiet_default_is_all_on() {
        assert!(QuietFlags::default().all_quiet());
    }

    #[test]
    fn quiet_rejected_for_off_screen_window() {
        let mut q = QuietFlags::default();
        q.no_foreground = false;
        assert!(validate_quiet(VisibilityFlag::OffScreenOnly, &q).is_err());
        // Allowed only under the audited foreground exception.
        assert!(validate_quiet(VisibilityFlag::ForegroundExceptionBound, &q).is_ok());
    }

    #[test]
    fn resolver_rejects_localhost_and_paths() {
        assert!(validate_resolver("http://localhost:8080/x").is_err());
        assert!(validate_resolver("file:///tmp/x.png").is_err());
        assert!(validate_resolver("C:\\Users\\op\\x.png").is_err());
        assert!(validate_resolver("/var/lib/x").is_err());
        assert!(validate_resolver("~/x").is_err());
        assert!(validate_resolver("").is_err());
    }

    #[test]
    fn resolver_accepts_governed_ids() {
        assert!(validate_resolver("artifact-manifest-01J0000000000000000000").is_ok());
        assert!(validate_resolver("spec:10.18.3").is_ok());
        assert!(validate_resolver("ledger-event-abc123").is_ok());
    }

    #[test]
    fn quiet_flags_json_roundtrip() {
        let q = QuietFlags::default();
        let parsed = QuietFlags::from_json(&q.to_json()).expect("roundtrip");
        assert_eq!(q, parsed);
    }
}
