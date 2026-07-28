//! Frontend client for the canonical Settings & Preferences domain (Master Spec v02.201 §10.17).
//!
//! ## Why this module exists (MT-072 FAIL_V2 remediation)
//!
//! Validator V2 rejected the editor settings because they persisted as an opaque workspace-settings
//! JSON document (`PUT /workspaces/:id/settings`) instead of the canonical typed
//! [`PreferenceRecord`](crate::preferences) authority. The backend now exposes the PreferenceRecord
//! surface (`crate::api::preferences` — see `src/backend/handshake_core/src/api/preferences.rs`) with
//! stable `view-defaults.editor.*` ids, typed validation, monotonic revisions, reset-to-default, change
//! history, and EventLedger/Flight-Recorder receipts. This module is the FRONTEND half: a typed
//! transport seam the settings dialog reads resolved values from and writes editor preferences through,
//! plus the mapping between the render-layer editor types
//! ([`EditorPrefs`](crate::workspace_settings::EditorPrefs) /
//! [`SyntaxPalette`](crate::workspace_settings::SyntaxPalette) / the editor keybinding overrides) and
//! the stable per-id preference values the API stores.
//!
//! Editor preferences are now authoritative on the PreferenceRecord surface; they no longer ride the
//! opaque `/settings` document. Non-editor settings (theme, view mode, app keybindings, swarm-board
//! flag) stay on their existing `/settings` path — this module governs ONLY the editor namespace.
//!
//! ## Design
//!
//! Like [`crate::workspace_settings::SettingsTransport`], [`PreferenceTransport`] is a synchronous seam
//! so the load/set logic stays directly unit-testable with an in-memory stub (no live server). The
//! production [`PreferenceClient`] bridges it onto reqwest + the app's tokio runtime handle; the egui
//! thread never calls it directly — the shell drives it off-thread (HBR-QUIET).
//!
//! ## Wire contract (must match `crate::api::preferences`)
//!
//! * `GET    /workspaces/:id/preferences`                       → `{ preferences: [ProjectionRow] }`
//! * `GET    /workspaces/:id/preferences/:pref_id`              → `{ record: PreferenceRecord }`
//! * `PUT    /workspaces/:id/preferences/:pref_id`  `{value}`   → `{ record, receipt }` | 400 `{error,validation}`
//! * `POST   /workspaces/:id/preferences/:pref_id/reset`        → `{ record, receipt }`
//! * `GET    /workspaces/:id/preferences/:pref_id/history`      → `{ receipts: [ChangeReceipt] }`

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::workspace_settings::{
    EditorPrefs, RenderWhitespaceMode, SyntaxPalette, SyntaxPaletteMode, WordWrapMode,
    WorkspaceSettingsState, EDITOR_FONT_SIZE_RANGE, EDITOR_LINE_HEIGHT_RANGE, SYNTAX_SCOPE_KEYS,
    TAB_SIZE_RANGE,
};

/// The HTTP timeout for a single preference request (mirrors the settings transport budget).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Header carrying the mutation actor so the backend receipt records provenance (SET-EVT-001).
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";

// ── Stable editor preference ids (SET-REC-004) — MUST mirror the backend registry ─────────────────
// Source of truth: src/backend/handshake_core/src/preferences/mod.rs (PREF_EDITOR_* consts). Cited by
// the dialog, the mapping below, the AccessKit ids, the UserManual, tests, and audits.

/// Editor text point size (distinct from chrome font — AC-002). `float`, 6.0..=48.0.
pub const PREF_EDITOR_FONT_SIZE: &str = "view-defaults.editor.font-size";
/// Columns per tab. `int`, 1..=16.
pub const PREF_EDITOR_TAB_SIZE: &str = "view-defaults.editor.tab-size";
/// Tabs-vs-spaces toggle. `bool`.
pub const PREF_EDITOR_INSERT_SPACES: &str = "view-defaults.editor.insert-spaces";
/// Word-wrap mode. `enum` off | on | bounded.
pub const PREF_EDITOR_WORD_WRAP: &str = "view-defaults.editor.word-wrap";
/// Bounded word-wrap column (used when word-wrap = bounded). `int`, 20..=400.
pub const PREF_EDITOR_WORD_WRAP_COLUMN: &str = "view-defaults.editor.word-wrap-column";
/// Whitespace rendering mode. `enum` none | boundary | all.
pub const PREF_EDITOR_RENDER_WHITESPACE: &str = "view-defaults.editor.render-whitespace";
/// Minimap visibility. `bool`.
pub const PREF_EDITOR_MINIMAP_ENABLED: &str = "view-defaults.editor.minimap-enabled";
/// Sticky-scroll band visibility. `bool`.
pub const PREF_EDITOR_STICKY_SCROLL: &str = "view-defaults.editor.sticky-scroll";
/// Gutter line-number visibility. `bool`.
pub const PREF_EDITOR_LINE_NUMBERS: &str = "view-defaults.editor.line-numbers";
/// Editor line-height multiplier. `float`, 1.0..=2.0.
pub const PREF_EDITOR_LINE_HEIGHT: &str = "view-defaults.editor.line-height";
/// Matching-bracket highlight. `bool`.
pub const PREF_EDITOR_BRACKET_MATCHING: &str = "view-defaults.editor.bracket-matching";
/// Indent-guide lines. `bool`.
pub const PREF_EDITOR_INDENT_GUIDES: &str = "view-defaults.editor.indent-guides";
/// Open rich documents in Reading view by default. `bool`.
pub const PREF_EDITOR_READING_MODE_DEFAULT: &str = "view-defaults.editor.reading-mode-default";
/// Syntax palette mode. `enum` muted | standard | custom.
pub const PREF_EDITOR_SYNTAX_PALETTE_MODE: &str = "view-defaults.editor.syntax-palette-mode";
/// Custom per-scope syntax colors. `json-object` { scope: [r,g,b,a] }.
pub const PREF_EDITOR_SYNTAX_CUSTOM_COLORS: &str = "view-defaults.editor.syntax-custom-colors";
/// Editor keybinding overrides. `json-object` { action_id: chord }.
pub const PREF_EDITOR_KEYBINDING_OVERRIDES: &str = "view-defaults.editor.keybinding-overrides";

/// Every editor preference id, in registry order. Used by tests + the projection completeness check.
pub const EDITOR_PREFERENCE_IDS: &[&str] = &[
    PREF_EDITOR_FONT_SIZE,
    PREF_EDITOR_TAB_SIZE,
    PREF_EDITOR_INSERT_SPACES,
    PREF_EDITOR_WORD_WRAP,
    PREF_EDITOR_WORD_WRAP_COLUMN,
    PREF_EDITOR_RENDER_WHITESPACE,
    PREF_EDITOR_MINIMAP_ENABLED,
    PREF_EDITOR_STICKY_SCROLL,
    PREF_EDITOR_LINE_NUMBERS,
    PREF_EDITOR_LINE_HEIGHT,
    PREF_EDITOR_BRACKET_MATCHING,
    PREF_EDITOR_INDENT_GUIDES,
    PREF_EDITOR_READING_MODE_DEFAULT,
    PREF_EDITOR_SYNTAX_PALETTE_MODE,
    PREF_EDITOR_SYNTAX_CUSTOM_COLORS,
    PREF_EDITOR_KEYBINDING_OVERRIDES,
];

// ── Wire shapes (subset of the backend JSON the frontend consumes) ────────────────────────────────

/// A structured validation failure surfaced to the UI (SET-REC-002). Mirrors the backend
/// `PreferenceValidationError` JSON `{ preference_id, code, message }`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PreferenceValidationError {
    /// The preference id that failed validation.
    pub preference_id: String,
    /// A stable machine code for the failure class (e.g. `out_of_range`, `unknown_enum_member`).
    pub code: String,
    /// A human-readable explanation naming the constraint and the offending value.
    pub message: String,
}

/// A resolved preference record returned by GET/PUT/reset (the fields the frontend reads).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PreferenceRecord {
    /// Stable `<namespace>.<key>` id.
    pub preference_id: String,
    /// The typed current value.
    pub value: Value,
    /// The registry default value.
    #[serde(default)]
    pub default_value: Value,
    /// Provenance of the current value (`default` | `operator` | ...).
    #[serde(default)]
    pub source: String,
    /// Monotonically increasing per (preference_id, scope). 0 = resolved to default (never set).
    #[serde(default)]
    pub revision: i64,
}

/// A redacted projection row (SET-PROJ-002 — the read-only view the dialog hydrates from).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PreferenceProjectionRow {
    /// Stable id.
    pub preference_id: String,
    /// Effective value (redacted for non-public records; all editor prefs are public).
    pub value: Value,
    /// Registry default.
    #[serde(default)]
    pub default_value: Value,
    /// Provenance.
    #[serde(default)]
    pub source: String,
    /// Current revision.
    #[serde(default)]
    pub revision: i64,
}

/// A recoverable change receipt (SET-EVT-002 — the fields the History view surfaces).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PreferenceChangeReceipt {
    /// The preference this receipt records.
    #[serde(default)]
    pub preference_id: String,
    /// Revision after the change.
    #[serde(default)]
    pub after_revision: i64,
    /// The value after the change.
    #[serde(default)]
    pub new_value: Value,
    /// Provenance of the new value.
    #[serde(default)]
    pub source: String,
    /// The mutation actor.
    #[serde(default)]
    pub actor: String,
    /// ISO8601 change timestamp.
    #[serde(default)]
    pub created_at: String,
}

/// A preference transport failure, kept typed so the UI can distinguish a structured validation
/// rejection (operator's value was bad) from an unreachable backend (degrade visibly, no freeze).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreferenceTransportError {
    /// The backend rejected the value with a structured 400 (SET-REC-002). Surface `message` inline.
    Validation(PreferenceValidationError),
    /// The preference id is not a defined editor preference (404) — a programming error, not operator.
    UnknownPreference(String),
    /// The backend is unreachable, returned a non-success status, or the body failed to parse. The UI
    /// keeps the optimistic local value and shows a retryable error (graceful degradation).
    Unavailable(String),
}

impl std::fmt::Display for PreferenceTransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferenceTransportError::Validation(err) => {
                write!(f, "{}: {}", err.preference_id, err.message)
            }
            PreferenceTransportError::UnknownPreference(id) => {
                write!(f, "unknown preference '{id}'")
            }
            PreferenceTransportError::Unavailable(msg) => {
                write!(f, "settings backend unavailable: {msg}")
            }
        }
    }
}

impl std::error::Error for PreferenceTransportError {}

/// The synchronous preference transport seam. Synchronous so the mapping/set logic stays a pure,
/// directly-unit-testable seam with a stub (no live server). The production [`PreferenceClient`]
/// bridges it onto reqwest + the app's tokio runtime; the shell calls it ONLY from a short-lived
/// off-thread task (HBR-QUIET).
pub trait PreferenceTransport: Send + Sync {
    /// `GET /workspaces/:id/preferences` → the redacted projection over every editor preference.
    fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PreferenceProjectionRow>, PreferenceTransportError>;

    /// `PUT /workspaces/:id/preferences/:preference_id` `{value}` → the resolved record. A typed-invalid
    /// value returns [`PreferenceTransportError::Validation`]; an unreachable backend returns
    /// [`PreferenceTransportError::Unavailable`].
    fn set(
        &self,
        workspace_id: &str,
        preference_id: &str,
        value: Value,
    ) -> Result<PreferenceRecord, PreferenceTransportError>;

    /// `POST /workspaces/:id/preferences/:preference_id/reset` → the record reset to its default.
    fn reset(
        &self,
        workspace_id: &str,
        preference_id: &str,
    ) -> Result<PreferenceRecord, PreferenceTransportError>;

    /// `GET /workspaces/:id/preferences/:preference_id/history` → the change history, newest first.
    fn history(
        &self,
        workspace_id: &str,
        preference_id: &str,
    ) -> Result<Vec<PreferenceChangeReceipt>, PreferenceTransportError>;
}

/// Production transport: the backend's PostgreSQL-authoritative PreferenceRecord REST surface, bridged
/// onto the app's tokio runtime handle (the [`crate::workspace_settings::SettingsClient`] pattern).
/// reqwest is async; this holds a runtime [`Handle`] and bridges with `Handle::block_on` so the
/// transport stays a synchronous seam, and the app calls it ONLY from a short-lived off-thread task.
///
/// [`Handle`]: tokio::runtime::Handle
#[derive(Clone)]
pub struct PreferenceClient {
    client: reqwest::Client,
    base_url: String,
    actor_id: String,
    runtime: tokio::runtime::Handle,
}

impl PreferenceClient {
    /// Build a client against `base_url` bridging onto `runtime`, tagging mutations with `actor_id`.
    pub fn new(
        base_url: impl Into<String>,
        actor_id: impl Into<String>,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self {
            client: crate::backend_client::shared_http_client(),
            base_url: base_url.into(),
            actor_id: actor_id.into(),
            runtime,
        }
    }

    /// The production client: the hardcoded backend base URL, `operator` actor, on the app's runtime.
    pub fn production(runtime: tokio::runtime::Handle) -> Self {
        Self::new(crate::backend_client::BACKEND_BASE_URL, "operator", runtime)
    }

    fn base(&self, workspace_id: &str) -> String {
        format!(
            "{}/workspaces/{}/preferences",
            self.base_url,
            urlencode(workspace_id)
        )
    }
}

/// Minimal percent-encoding for a path segment (defensive: a stray space/slash cannot break the URL).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl PreferenceTransport for PreferenceClient {
    fn list(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<PreferenceProjectionRow>, PreferenceTransportError> {
        let url = self.base(workspace_id);
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(PreferenceTransportError::Unavailable(format!(
                    "GET preferences non-success status {}",
                    resp.status()
                )));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))?;
            let rows = body
                .get("preferences")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    PreferenceTransportError::Unavailable("projection missing 'preferences'".into())
                })?;
            rows.iter()
                .map(|row| {
                    serde_json::from_value::<PreferenceProjectionRow>(row.clone())
                        .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))
                })
                .collect()
        })
    }

    fn set(
        &self,
        workspace_id: &str,
        preference_id: &str,
        value: Value,
    ) -> Result<PreferenceRecord, PreferenceTransportError> {
        let url = format!("{}/{}", self.base(workspace_id), urlencode(preference_id));
        let client = self.client.clone();
        let actor = self.actor_id.clone();
        let body = json!({ "value": value });
        self.runtime.block_on(async move {
            let resp = client
                .put(&url)
                .header(HSK_HEADER_ACTOR_ID, actor)
                .timeout(REQUEST_TIMEOUT)
                .json(&body)
                .send()
                .await
                .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))?;
            parse_record_response(resp).await
        })
    }

    fn reset(
        &self,
        workspace_id: &str,
        preference_id: &str,
    ) -> Result<PreferenceRecord, PreferenceTransportError> {
        let url = format!(
            "{}/{}/reset",
            self.base(workspace_id),
            urlencode(preference_id)
        );
        let client = self.client.clone();
        let actor = self.actor_id.clone();
        self.runtime.block_on(async move {
            let resp = client
                .post(&url)
                .header(HSK_HEADER_ACTOR_ID, actor)
                .timeout(REQUEST_TIMEOUT)
                .json(&json!({}))
                .send()
                .await
                .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))?;
            parse_record_response(resp).await
        })
    }

    fn history(
        &self,
        workspace_id: &str,
        preference_id: &str,
    ) -> Result<Vec<PreferenceChangeReceipt>, PreferenceTransportError> {
        let url = format!(
            "{}/{}/history",
            self.base(workspace_id),
            urlencode(preference_id)
        );
        let client = self.client.clone();
        self.runtime.block_on(async move {
            let resp = client
                .get(&url)
                .timeout(REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))?;
            if !resp.status().is_success() {
                return Err(PreferenceTransportError::Unavailable(format!(
                    "GET history non-success status {}",
                    resp.status()
                )));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))?;
            let receipts = body
                .get("receipts")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            receipts
                .into_iter()
                .map(|r| {
                    serde_json::from_value::<PreferenceChangeReceipt>(r)
                        .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))
                })
                .collect()
        })
    }
}

/// Parse a PUT/reset response: 200 → the `record`; 400 → a structured [`PreferenceTransportError`];
/// 404 → [`PreferenceTransportError::UnknownPreference`]; anything else → `Unavailable`.
async fn parse_record_response(
    resp: reqwest::Response,
) -> Result<PreferenceRecord, PreferenceTransportError> {
    let status = resp.status();
    if status.is_success() {
        let body: Value = resp
            .json()
            .await
            .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()))?;
        let record = body.get("record").cloned().ok_or_else(|| {
            PreferenceTransportError::Unavailable("response missing 'record'".into())
        })?;
        return serde_json::from_value::<PreferenceRecord>(record)
            .map_err(|e| PreferenceTransportError::Unavailable(e.to_string()));
    }
    // Non-success: try to read the structured error body.
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if status.as_u16() == 400 {
        if let Some(validation) = body.get("validation") {
            if let Ok(err) = serde_json::from_value::<PreferenceValidationError>(validation.clone())
            {
                return Err(PreferenceTransportError::Validation(err));
            }
        }
        return Err(PreferenceTransportError::Unavailable(format!(
            "400 without a structured validation body: {body}"
        )));
    }
    if status.as_u16() == 404 {
        return Err(PreferenceTransportError::UnknownPreference(
            body.get("message")
                .and_then(Value::as_str)
                .unwrap_or("preference not found")
                .to_owned(),
        ));
    }
    Err(PreferenceTransportError::Unavailable(format!(
        "PUT/reset non-success status {status}: {body}"
    )))
}

// ── Shell write queue (drives the transport OFF the egui thread — HBR-QUIET) ───────────────────────

/// A queued editor-preference mutation the shell flushes off-thread. The write queue lets a control
/// edit return immediately (optimistic local update) while the PUT/reset/list runs on a worker.
#[derive(Debug, Clone, PartialEq)]
pub enum PreferenceWriteKind {
    /// `PUT` a typed value for `preference_id` (SET-REC-002).
    Set {
        /// The stable `view-defaults.editor.*` id.
        preference_id: String,
        /// The typed JSON value (already mapped to the backend encoding).
        value: Value,
    },
    /// `POST .../reset` `preference_id` back to its registry default (SET-UI-002).
    Reset {
        /// The stable id to reset.
        preference_id: String,
    },
    /// `GET` the projection and hydrate the render-layer settings from canonical state (SET-REC-003).
    Hydrate,
}

/// A queued write bound to the workspace it applies to (the binding may change between enqueue + flush).
#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceWrite {
    /// The workspace whose preferences this write targets.
    pub workspace_id: String,
    /// What to do.
    pub kind: PreferenceWriteKind,
}

/// The result of a flushed [`PreferenceWrite`], delivered back to the egui thread through the app's cell.
#[derive(Debug, Clone)]
pub enum PreferenceDeliveryOutcome {
    /// A `Set` completed (Ok = the resolved record; Err = validation / unavailable).
    Set {
        /// The id that was set.
        preference_id: String,
        /// The outcome.
        result: Result<PreferenceRecord, PreferenceTransportError>,
    },
    /// A `Reset` completed.
    Reset {
        /// The id that was reset.
        preference_id: String,
        /// The outcome.
        result: Result<PreferenceRecord, PreferenceTransportError>,
    },
    /// A `Hydrate` completed (Ok = the projection rows to apply; Err = unavailable).
    Hydrate(Result<Vec<PreferenceProjectionRow>, PreferenceTransportError>),
}

/// A flushed-write delivery the shell drains each frame (try_lock) to apply hydrate results / surface
/// errors, mirroring the settings-save delivery cell.
#[derive(Debug, Clone)]
pub struct PreferenceDelivery {
    /// The workspace the completed write targeted.
    pub workspace_id: String,
    /// What completed.
    pub outcome: PreferenceDeliveryOutcome,
}

/// Execute one queued write against `transport` and build its delivery (runs on a worker thread).
pub fn run_preference_write(
    transport: &dyn PreferenceTransport,
    write: &PreferenceWrite,
) -> PreferenceDelivery {
    let outcome = match &write.kind {
        PreferenceWriteKind::Set {
            preference_id,
            value,
        } => PreferenceDeliveryOutcome::Set {
            preference_id: preference_id.clone(),
            result: transport.set(&write.workspace_id, preference_id, value.clone()),
        },
        PreferenceWriteKind::Reset { preference_id } => PreferenceDeliveryOutcome::Reset {
            preference_id: preference_id.clone(),
            result: transport.reset(&write.workspace_id, preference_id),
        },
        PreferenceWriteKind::Hydrate => {
            PreferenceDeliveryOutcome::Hydrate(transport.list(&write.workspace_id))
        }
    };
    PreferenceDelivery {
        workspace_id: write.workspace_id.clone(),
        outcome,
    }
}

// ── Mapping: render-layer editor types <-> stable per-id preference values ─────────────────────────

/// The full canonical (preference_id → value) set for the scalar editor prefs. Value encodings MUST
/// satisfy the backend registry constraints (`crate::preferences::editor_preference_registry`):
/// * `word_wrap` splits into the `word-wrap` enum + a separate `word-wrap-column` int (the backend
///   models the column as its own preference; a `BoundedColumn(n)` therefore writes BOTH ids).
pub fn editor_prefs_value_map(prefs: &EditorPrefs) -> Vec<(&'static str, Value)> {
    let (wrap_mode, wrap_col): (&str, Option<u16>) = match prefs.word_wrap {
        WordWrapMode::Off => ("off", None),
        WordWrapMode::On => ("on", None),
        WordWrapMode::BoundedColumn(n) => ("bounded", Some(n)),
    };
    let mut out = vec![
        (PREF_EDITOR_FONT_SIZE, json!(prefs.editor_font_size)),
        (PREF_EDITOR_TAB_SIZE, json!(prefs.tab_size)),
        (PREF_EDITOR_INSERT_SPACES, json!(prefs.insert_spaces)),
        (PREF_EDITOR_WORD_WRAP, json!(wrap_mode)),
        (
            PREF_EDITOR_RENDER_WHITESPACE,
            json!(prefs.render_whitespace.as_str()),
        ),
        (PREF_EDITOR_MINIMAP_ENABLED, json!(prefs.minimap_enabled)),
        (PREF_EDITOR_STICKY_SCROLL, json!(prefs.sticky_scroll)),
        (PREF_EDITOR_LINE_NUMBERS, json!(prefs.line_numbers)),
        (PREF_EDITOR_LINE_HEIGHT, json!(prefs.line_height)),
        (PREF_EDITOR_BRACKET_MATCHING, json!(prefs.bracket_matching)),
        (PREF_EDITOR_INDENT_GUIDES, json!(prefs.indent_guides)),
        (
            PREF_EDITOR_READING_MODE_DEFAULT,
            json!(prefs.reading_mode_default),
        ),
    ];
    if let Some(n) = wrap_col {
        out.push((PREF_EDITOR_WORD_WRAP_COLUMN, json!(n)));
    }
    out
}

/// The (preference_id → value) set for the syntax palette: mode + the custom color map.
pub fn syntax_palette_value_map(palette: &SyntaxPalette) -> Vec<(&'static str, Value)> {
    let mut custom = serde_json::Map::new();
    for key in SYNTAX_SCOPE_KEYS {
        if let Some(rgba) = palette.custom_for(key) {
            custom.insert(
                (*key).to_owned(),
                Value::Array(rgba.iter().map(|c| Value::from(*c)).collect()),
            );
        }
    }
    vec![
        (
            PREF_EDITOR_SYNTAX_PALETTE_MODE,
            json!(palette.mode.as_str()),
        ),
        (PREF_EDITOR_SYNTAX_CUSTOM_COLORS, Value::Object(custom)),
    ]
}

/// The keybinding-overrides preference value: the `{ action_id: chord }` map the editor keybinding
/// table maintains (the SEPARATE editor override store, keyed by action id).
pub fn keybinding_overrides_value(settings: &WorkspaceSettingsState) -> Value {
    let mut map = serde_json::Map::new();
    for binding in &settings.editor_keybindings {
        map.insert(binding.action_id.clone(), json!(binding.chord));
    }
    Value::Object(map)
}

/// The writes that changed between `prev` and `next` scalar editor prefs — only the differing ids, so a
/// single control edit produces a single (or, for a bounded-wrap change, at most two) targeted PUT.
pub fn changed_editor_pref_writes(
    prev: &EditorPrefs,
    next: &EditorPrefs,
) -> Vec<(&'static str, Value)> {
    let before: HashMap<&'static str, Value> = editor_prefs_value_map(prev).into_iter().collect();
    editor_prefs_value_map(next)
        .into_iter()
        .filter(|(id, value)| before.get(id) != Some(value))
        .collect()
}

/// The writes that changed between `prev` and `next` syntax palettes (mode and/or custom map).
pub fn changed_syntax_palette_writes(
    prev: &SyntaxPalette,
    next: &SyntaxPalette,
) -> Vec<(&'static str, Value)> {
    let before: HashMap<&'static str, Value> = syntax_palette_value_map(prev).into_iter().collect();
    syntax_palette_value_map(next)
        .into_iter()
        .filter(|(id, value)| before.get(id) != Some(value))
        .collect()
}

/// Apply a projection (the resolved effective values) back onto the render-layer settings so the dialog
/// and live editors show the PostgreSQL-authoritative values (SET-REC-003: unset → registry default).
/// Unknown / malformed rows are skipped defensively (the loader also clamps numeric ranges so a
/// hand-edited out-of-range row cannot smuggle an invalid value into the live editor).
pub fn apply_projection(rows: &[PreferenceProjectionRow], settings: &mut WorkspaceSettingsState) {
    let by_id: HashMap<&str, &Value> = rows
        .iter()
        .map(|row| (row.preference_id.as_str(), &row.value))
        .collect();

    let prefs = &mut settings.editor_prefs;
    if let Some(v) = by_id.get(PREF_EDITOR_FONT_SIZE).and_then(|v| v.as_f64()) {
        prefs.editor_font_size = (v as f32).clamp(
            *EDITOR_FONT_SIZE_RANGE.start(),
            *EDITOR_FONT_SIZE_RANGE.end(),
        );
    }
    if let Some(v) = by_id.get(PREF_EDITOR_TAB_SIZE).and_then(|v| v.as_u64()) {
        prefs.tab_size = (v as u8).clamp(*TAB_SIZE_RANGE.start(), *TAB_SIZE_RANGE.end());
    }
    if let Some(v) = by_id
        .get(PREF_EDITOR_INSERT_SPACES)
        .and_then(|v| v.as_bool())
    {
        prefs.insert_spaces = v;
    }
    // word-wrap enum + optional bounded column (resolve the column first so `bounded` can use it).
    let wrap_col = by_id
        .get(PREF_EDITOR_WORD_WRAP_COLUMN)
        .and_then(|v| v.as_u64())
        .map(|n| n.min(u16::MAX as u64) as u16);
    if let Some(mode) = by_id.get(PREF_EDITOR_WORD_WRAP).and_then(|v| v.as_str()) {
        prefs.word_wrap = match mode {
            "on" => WordWrapMode::On,
            "bounded" => WordWrapMode::BoundedColumn(wrap_col.unwrap_or(80)),
            _ => WordWrapMode::Off,
        };
    }
    if let Some(mode) = by_id
        .get(PREF_EDITOR_RENDER_WHITESPACE)
        .and_then(|v| v.as_str())
        .and_then(RenderWhitespaceMode::from_str_opt)
    {
        prefs.render_whitespace = mode;
    }
    if let Some(v) = by_id
        .get(PREF_EDITOR_MINIMAP_ENABLED)
        .and_then(|v| v.as_bool())
    {
        prefs.minimap_enabled = v;
    }
    if let Some(v) = by_id
        .get(PREF_EDITOR_STICKY_SCROLL)
        .and_then(|v| v.as_bool())
    {
        prefs.sticky_scroll = v;
    }
    if let Some(v) = by_id
        .get(PREF_EDITOR_LINE_NUMBERS)
        .and_then(|v| v.as_bool())
    {
        prefs.line_numbers = v;
    }
    if let Some(v) = by_id.get(PREF_EDITOR_LINE_HEIGHT).and_then(|v| v.as_f64()) {
        prefs.line_height = (v as f32).clamp(
            *EDITOR_LINE_HEIGHT_RANGE.start(),
            *EDITOR_LINE_HEIGHT_RANGE.end(),
        );
    }
    if let Some(v) = by_id
        .get(PREF_EDITOR_BRACKET_MATCHING)
        .and_then(|v| v.as_bool())
    {
        prefs.bracket_matching = v;
    }
    if let Some(v) = by_id
        .get(PREF_EDITOR_INDENT_GUIDES)
        .and_then(|v| v.as_bool())
    {
        prefs.indent_guides = v;
    }
    if let Some(v) = by_id
        .get(PREF_EDITOR_READING_MODE_DEFAULT)
        .and_then(|v| v.as_bool())
    {
        prefs.reading_mode_default = v;
    }

    // Syntax palette (mode + custom map).
    let fallback = settings.syntax_palette.clone();
    let mode = by_id
        .get(PREF_EDITOR_SYNTAX_PALETTE_MODE)
        .and_then(|v| v.as_str())
        .and_then(SyntaxPaletteMode::from_str_opt)
        .unwrap_or(fallback.mode);
    let mut custom: HashMap<String, [u8; 4]> = HashMap::new();
    if let Some(obj) = by_id
        .get(PREF_EDITOR_SYNTAX_CUSTOM_COLORS)
        .and_then(|v| v.as_object())
    {
        for key in SYNTAX_SCOPE_KEYS {
            if let Some(arr) = obj.get(*key).and_then(Value::as_array) {
                if arr.len() == 4 {
                    let mut rgba = [0u8; 4];
                    let mut ok = true;
                    for (i, c) in arr.iter().enumerate() {
                        match c.as_u64() {
                            Some(n) if n <= 255 => rgba[i] = n as u8,
                            _ => ok = false,
                        }
                    }
                    if ok {
                        custom.insert((*key).to_owned(), rgba);
                    }
                }
            }
        }
    }
    settings.syntax_palette = SyntaxPalette { mode, custom };

    // Editor keybinding overrides ({ action_id: chord }).
    if let Some(obj) = by_id
        .get(PREF_EDITOR_KEYBINDING_OVERRIDES)
        .and_then(|v| v.as_object())
    {
        settings.clear_all_editor_chords();
        for (action_id, chord) in obj {
            if let Some(chord) = chord.as_str() {
                if !action_id.trim().is_empty() && !chord.trim().is_empty() {
                    settings.set_editor_chord(action_id, chord.to_owned());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_pref_ids_match_registry_shape() {
        // Every id is namespaced under view-defaults.editor. (SET-REC-004) and unique.
        let mut ids = EDITOR_PREFERENCE_IDS.to_vec();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate editor preference id");
        assert_eq!(count, 16, "the editor registry has 16 preferences");
        for id in EDITOR_PREFERENCE_IDS {
            assert!(
                id.starts_with("view-defaults.editor."),
                "{id} is not namespaced under view-defaults.editor"
            );
        }
    }

    #[test]
    fn value_map_covers_every_scalar_and_palette_id() {
        let prefs = EditorPrefs::default();
        let palette = SyntaxPalette::default();
        let mut ids: Vec<&str> = editor_prefs_value_map(&prefs)
            .into_iter()
            .map(|(id, _)| id)
            .chain(
                syntax_palette_value_map(&palette)
                    .into_iter()
                    .map(|(id, _)| id),
            )
            .collect();
        // The default word-wrap is Off, so the column id is omitted by design; add it for the cover set.
        ids.push(PREF_EDITOR_WORD_WRAP_COLUMN);
        // Keybinding overrides are written as a single map id (not part of the scalar/palette maps).
        ids.push(PREF_EDITOR_KEYBINDING_OVERRIDES);
        ids.sort_unstable();
        ids.dedup();
        let mut expected = EDITOR_PREFERENCE_IDS.to_vec();
        expected.sort_unstable();
        assert_eq!(
            ids, expected,
            "value maps must cover every editor preference id"
        );
    }

    #[test]
    fn changed_writes_only_include_the_edited_field() {
        let prev = EditorPrefs::default();
        let mut next = prev;
        next.tab_size = 8;
        let writes = changed_editor_pref_writes(&prev, &next);
        assert_eq!(writes.len(), 1, "only tab-size changed");
        assert_eq!(writes[0].0, PREF_EDITOR_TAB_SIZE);
        assert_eq!(writes[0].1, json!(8));
    }

    #[test]
    fn bounded_wrap_change_writes_both_mode_and_column() {
        let prev = EditorPrefs::default(); // Off
        let mut next = prev;
        next.word_wrap = WordWrapMode::BoundedColumn(100);
        let writes = changed_editor_pref_writes(&prev, &next);
        let ids: Vec<&str> = writes.iter().map(|(id, _)| *id).collect();
        assert!(
            ids.contains(&PREF_EDITOR_WORD_WRAP),
            "mode changes to bounded"
        );
        assert!(
            ids.contains(&PREF_EDITOR_WORD_WRAP_COLUMN),
            "the bounded column is written as its own preference"
        );
        let col = writes
            .iter()
            .find(|(id, _)| *id == PREF_EDITOR_WORD_WRAP_COLUMN)
            .unwrap();
        assert_eq!(col.1, json!(100));
    }

    #[test]
    fn font_size_value_satisfies_backend_float_encoding() {
        let prefs = EditorPrefs {
            editor_font_size: 20.0,
            ..Default::default()
        };
        let map: HashMap<_, _> = editor_prefs_value_map(&prefs).into_iter().collect();
        assert_eq!(map[PREF_EDITOR_FONT_SIZE].as_f64(), Some(20.0));
    }

    #[test]
    fn apply_projection_round_trips_scalar_prefs() {
        let mut settings = crate::workspace_settings::default_workspace_settings_state();
        let rows = vec![
            PreferenceProjectionRow {
                preference_id: PREF_EDITOR_FONT_SIZE.into(),
                value: json!(22.0),
                default_value: json!(13.0),
                source: "operator".into(),
                revision: 1,
            },
            PreferenceProjectionRow {
                preference_id: PREF_EDITOR_WORD_WRAP.into(),
                value: json!("bounded"),
                default_value: json!("off"),
                source: "operator".into(),
                revision: 1,
            },
            PreferenceProjectionRow {
                preference_id: PREF_EDITOR_WORD_WRAP_COLUMN.into(),
                value: json!(120),
                default_value: json!(80),
                source: "operator".into(),
                revision: 1,
            },
        ];
        apply_projection(&rows, &mut settings);
        assert_eq!(settings.editor_prefs.editor_font_size, 22.0);
        assert_eq!(
            settings.editor_prefs.word_wrap,
            WordWrapMode::BoundedColumn(120)
        );
    }

    #[test]
    fn apply_projection_clamps_out_of_range_font_size() {
        let mut settings = crate::workspace_settings::default_workspace_settings_state();
        let rows = vec![PreferenceProjectionRow {
            preference_id: PREF_EDITOR_FONT_SIZE.into(),
            value: json!(999.0),
            default_value: json!(13.0),
            source: "operator".into(),
            revision: 1,
        }];
        apply_projection(&rows, &mut settings);
        assert_eq!(
            settings.editor_prefs.editor_font_size,
            *EDITOR_FONT_SIZE_RANGE.end()
        );
    }

    #[test]
    fn validation_error_deserializes_from_backend_shape() {
        let body = json!({
            "preference_id": PREF_EDITOR_FONT_SIZE,
            "code": "out_of_range",
            "message": "number 100 is outside the allowed range [6, 48]",
        });
        let err: PreferenceValidationError = serde_json::from_value(body).unwrap();
        assert_eq!(err.code, "out_of_range");
        assert_eq!(err.preference_id, PREF_EDITOR_FONT_SIZE);
    }
}
