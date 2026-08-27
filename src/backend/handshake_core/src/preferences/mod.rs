//! Settings & Preferences domain (Master Spec v02.201 §10.17, PRIM-PreferenceRecord).
//!
//! WP-KERNEL-012 MT-072 remediation (FAIL_V2): editor settings previously persisted as an opaque
//! workspace-settings JSON document. Validator V2 required them re-authored as the canonical typed
//! [`PreferenceRecord`] authority in SurrealDB with a stable `preference_id`, declared `value_type`,
//! `scope`, registry `default_value`, `source`, monotonically increasing `revision`, typed validation,
//! reset-to-default semantics, change history, and recoverable EventLedger / Flight-Recorder receipts.
//!
//! This module owns the pure (storage-independent) half of that domain:
//! * the record/scope/value-type/source/redaction typed shapes (§10.17.3 SET-REC-001),
//! * the registry of defined editor preferences + their defaults (§10.17.3 SET-REC-003),
//! * typed validation of a candidate value against a registry entry (§10.17.3 SET-REC-002), and
//! * the redacted-projection row shape (§10.17.6 SET-PROJ-002).
//!
//! The SurrealDB persistence, EventLedger emission, and receipt durability live in
//! `crate::storage` (see `Database::preference_*`); the HTTP surface lives in
//! `crate::api::preferences`. SurrealDB is the sole canonical store for this domain
//! (§10.17.2 SET-STORE-002).

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Schema id stamped onto every preference record + receipt envelope so a no-context reader / tool can
/// identify the contract version (SET-REC-004 stable ids).
pub const PREFERENCE_RECORD_SCHEMA_ID: &str = "hsk.preference_record@1";
/// Schema id for the recoverable change receipt (SET-EVT-002).
pub const PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID: &str = "hsk.preference_change_receipt@1";

/// Declared value type of a preference (subset of PRIM-PreferenceValueType relevant to the editor
/// preferences migrated in MT-072). SET-REC-001.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreferenceValueType {
    /// A boolean toggle.
    Bool,
    /// A bounded integer.
    Int,
    /// A bounded float.
    Float,
    /// A free string (unused by the editor set today; reserved for the domain).
    String,
    /// A value drawn from a fixed enum domain.
    Enum,
    /// A structured JSON object with a domain-specific shape (e.g. the syntax color map).
    JsonObject,
}

impl PreferenceValueType {
    /// The stable persisted string form.
    pub fn as_str(self) -> &'static str {
        match self {
            PreferenceValueType::Bool => "bool",
            PreferenceValueType::Int => "int",
            PreferenceValueType::Float => "float",
            PreferenceValueType::String => "string",
            PreferenceValueType::Enum => "enum",
            PreferenceValueType::JsonObject => "json-object",
        }
    }

    /// Parse the stable persisted string form.
    pub fn from_str_opt(value: &str) -> Option<Self> {
        Some(match value {
            "bool" => PreferenceValueType::Bool,
            "int" => PreferenceValueType::Int,
            "float" => PreferenceValueType::Float,
            "string" => PreferenceValueType::String,
            "enum" => PreferenceValueType::Enum,
            "json-object" => PreferenceValueType::JsonObject,
            _ => return None,
        })
    }
}

/// The scope kind of a preference record. Resolution order is surface over workspace over global over
/// registry default (SET-REC-001). MT-072 editor preferences are `workspace`-scoped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreferenceScopeKind {
    /// Applies to every workspace unless overridden.
    Global,
    /// Applies to one workspace (carries the wsid in [`PreferenceScope::scope_ref`]).
    Workspace,
    /// Applies to one surface (carries the surface id in [`PreferenceScope::scope_ref`]).
    Surface,
}

impl PreferenceScopeKind {
    /// The stable persisted string form.
    pub fn as_str(self) -> &'static str {
        match self {
            PreferenceScopeKind::Global => "global",
            PreferenceScopeKind::Workspace => "workspace",
            PreferenceScopeKind::Surface => "surface",
        }
    }

    /// Parse the stable persisted string form.
    pub fn from_str_opt(value: &str) -> Option<Self> {
        Some(match value {
            "global" => PreferenceScopeKind::Global,
            "workspace" => PreferenceScopeKind::Workspace,
            "surface" => PreferenceScopeKind::Surface,
            _ => return None,
        })
    }
}

/// A concrete preference scope: a kind plus its reference (wsid for `workspace`, surface id for
/// `surface`, empty for `global`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceScope {
    /// Which scope layer this value lives in.
    pub kind: PreferenceScopeKind,
    /// The scope reference (wsid / surface id); empty string for global.
    pub scope_ref: String,
}

impl PreferenceScope {
    /// A workspace-scoped scope for `wsid`.
    pub fn workspace(wsid: impl Into<String>) -> Self {
        Self {
            kind: PreferenceScopeKind::Workspace,
            scope_ref: wsid.into(),
        }
    }
}

/// Provenance of the current value of a record (SET-REC-001 `source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreferenceSource {
    /// The value equals the registry default (never explicitly set, or reset back to default).
    Default,
    /// An operator explicitly set the value.
    Operator,
    /// The value arrived through an import.
    Import,
    /// The value was seeded by the one-time migration from the legacy opaque settings document.
    Migration,
}

impl PreferenceSource {
    /// The stable persisted string form.
    pub fn as_str(self) -> &'static str {
        match self {
            PreferenceSource::Default => "default",
            PreferenceSource::Operator => "operator",
            PreferenceSource::Import => "import",
            PreferenceSource::Migration => "migration",
        }
    }

    /// Parse the stable persisted string form.
    pub fn from_str_opt(value: &str) -> Option<Self> {
        Some(match value {
            "default" => PreferenceSource::Default,
            "operator" => PreferenceSource::Operator,
            "import" => PreferenceSource::Import,
            "migration" => PreferenceSource::Migration,
            _ => return None,
        })
    }
}

/// Redaction selector controlling whether the value appears in the redacted projection (SET-PROJ-001).
/// Every MT-072 editor preference is `Public` (no secret authority — SET-SCOPE-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RedactionClass {
    /// Safe to show verbatim in the projection and events.
    Public,
    /// Must be referenced by opaque id / hash in the projection and events.
    NonPublic,
}

impl RedactionClass {
    /// The stable persisted string form.
    pub fn as_str(self) -> &'static str {
        match self {
            RedactionClass::Public => "public",
            RedactionClass::NonPublic => "non-public",
        }
    }

    /// Parse the stable persisted string form.
    pub fn from_str_opt(value: &str) -> Option<Self> {
        Some(match value {
            "public" => RedactionClass::Public,
            "non-public" => RedactionClass::NonPublic,
            _ => return None,
        })
    }
}

/// A typed validation constraint attached to a registry schema entry (SET-REC-002).
#[derive(Debug, Clone)]
pub enum PreferenceConstraint {
    /// Accepts only JSON `true` / `false`.
    Bool,
    /// Accepts a JSON integer in the inclusive `[min, max]` range.
    IntRange { min: i64, max: i64 },
    /// Accepts a finite JSON number in the inclusive `[min, max]` range.
    FloatRange { min: f64, max: f64 },
    /// Accepts a JSON string equal to one of the enum members.
    Enum(&'static [&'static str]),
    /// Accepts a JSON object mapping a known syntax scope key to an sRGBA `[u8; 4]` array.
    SyntaxColorMap(&'static [&'static str]),
    /// Accepts a JSON object mapping a non-empty action id string to a non-empty chord string.
    ChordMap,
}

impl PreferenceConstraint {
    /// The declared value type implied by this constraint.
    pub fn value_type(&self) -> PreferenceValueType {
        match self {
            PreferenceConstraint::Bool => PreferenceValueType::Bool,
            PreferenceConstraint::IntRange { .. } => PreferenceValueType::Int,
            PreferenceConstraint::FloatRange { .. } => PreferenceValueType::Float,
            PreferenceConstraint::Enum(_) => PreferenceValueType::Enum,
            PreferenceConstraint::SyntaxColorMap(_) | PreferenceConstraint::ChordMap => {
                PreferenceValueType::JsonObject
            }
        }
    }
}

/// A structured, explicit validation failure (SET-REC-002 — never silently coerced or dropped). The
/// API surface renders this as an HTTP 400 with the machine-readable `code` + human `message`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferenceValidationError {
    /// The preference id that failed.
    pub preference_id: String,
    /// A stable machine code for the failure class.
    pub code: String,
    /// A human-readable explanation naming the constraint and the offending value.
    pub message: String,
}

impl std::fmt::Display for PreferenceValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}): {}", self.preference_id, self.code, self.message)
    }
}

impl std::error::Error for PreferenceValidationError {}

/// A registry-defined preference schema entry (PRIM-PreferenceSchemaEntry, SET-REC-001/002/003). Every
/// defined preference has exactly one entry supplying its namespace, value type, constraint, default,
/// and redaction class. The registry is the single source of truth for what a preference means.
#[derive(Debug, Clone)]
pub struct PreferenceSchemaEntry {
    /// Stable `<namespace>.<key>` id (SET-REC-001).
    pub preference_id: &'static str,
    /// One of the SET-SCOPE-001 namespaces.
    pub namespace: &'static str,
    /// The scope layer editor preferences live in (workspace for MT-072).
    pub scope_kind: PreferenceScopeKind,
    /// Human label for the projection / operator surface.
    pub label: &'static str,
    /// Typed validation constraint.
    pub constraint: PreferenceConstraint,
    /// Redaction class controlling projection visibility.
    pub redaction_class: RedactionClass,
    /// The registry default value (SET-REC-003 — every defined preference has one; reads of an unset
    /// preference resolve to this, never null).
    pub default_value: Value,
}

impl PreferenceSchemaEntry {
    /// The declared value type for this entry.
    pub fn value_type(&self) -> PreferenceValueType {
        self.constraint.value_type()
    }

    /// Validate `value` against this entry's constraint (SET-REC-002). Returns a structured error on
    /// any type mismatch, out-of-range number, unknown enum member, or malformed object shape.
    pub fn validate(&self, value: &Value) -> Result<(), PreferenceValidationError> {
        let err = |code: &str, message: String| PreferenceValidationError {
            preference_id: self.preference_id.to_owned(),
            code: code.to_owned(),
            message,
        };
        match &self.constraint {
            PreferenceConstraint::Bool => {
                if !value.is_boolean() {
                    return Err(err(
                        "not_bool",
                        format!("expected a boolean, got {value}"),
                    ));
                }
            }
            PreferenceConstraint::IntRange { min, max } => {
                let Some(n) = value.as_i64() else {
                    return Err(err(
                        "not_int",
                        format!("expected an integer, got {value}"),
                    ));
                };
                if n < *min || n > *max {
                    return Err(err(
                        "out_of_range",
                        format!("integer {n} is outside the allowed range [{min}, {max}]"),
                    ));
                }
            }
            PreferenceConstraint::FloatRange { min, max } => {
                let Some(n) = value.as_f64() else {
                    return Err(err(
                        "not_float",
                        format!("expected a number, got {value}"),
                    ));
                };
                if !n.is_finite() {
                    return Err(err("not_finite", format!("number {n} is not finite")));
                }
                if n < *min || n > *max {
                    return Err(err(
                        "out_of_range",
                        format!("number {n} is outside the allowed range [{min}, {max}]"),
                    ));
                }
            }
            PreferenceConstraint::Enum(members) => {
                let Some(s) = value.as_str() else {
                    return Err(err(
                        "not_string",
                        format!("expected one of {members:?}, got {value}"),
                    ));
                };
                if !members.contains(&s) {
                    return Err(err(
                        "unknown_enum_member",
                        format!("'{s}' is not one of {members:?}"),
                    ));
                }
            }
            PreferenceConstraint::SyntaxColorMap(keys) => {
                let Some(obj) = value.as_object() else {
                    return Err(err(
                        "not_object",
                        format!("expected a syntax color object, got {value}"),
                    ));
                };
                for (key, entry) in obj {
                    if !keys.contains(&key.as_str()) {
                        return Err(err(
                            "unknown_scope",
                            format!("'{key}' is not a known syntax scope {keys:?}"),
                        ));
                    }
                    let ok = entry
                        .as_array()
                        .filter(|arr| arr.len() == 4)
                        .map(|arr| {
                            arr.iter()
                                .all(|c| c.as_u64().is_some_and(|v| v <= 255))
                        })
                        .unwrap_or(false);
                    if !ok {
                        return Err(err(
                            "bad_color",
                            format!("scope '{key}' must be an sRGBA [r,g,b,a] array of 0..=255, got {entry}"),
                        ));
                    }
                }
            }
            PreferenceConstraint::ChordMap => {
                let Some(obj) = value.as_object() else {
                    return Err(err(
                        "not_object",
                        format!("expected a keybinding-override object, got {value}"),
                    ));
                };
                for (action_id, chord) in obj {
                    if action_id.trim().is_empty() {
                        return Err(err(
                            "empty_action",
                            "keybinding override has an empty action id".to_owned(),
                        ));
                    }
                    let ok = chord.as_str().is_some_and(|c| !c.trim().is_empty());
                    if !ok {
                        return Err(err(
                            "bad_chord",
                            format!("action '{action_id}' must map to a non-empty chord string, got {chord}"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// A fully-resolved preference record as returned by a read/set/reset (PRIM-PreferenceRecord,
/// SET-REC-001). Serializes to the API/JSON contract shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreferenceRecord {
    /// Schema id envelope.
    pub schema_id: String,
    /// Stable `<namespace>.<key>` id.
    pub preference_id: String,
    /// SET-SCOPE-001 namespace.
    pub namespace: String,
    /// Declared value type.
    pub value_type: String,
    /// The typed current value (the registry default when the record is unset).
    pub value: Value,
    /// Scope kind (`global` / `workspace` / `surface`).
    pub scope: String,
    /// Scope reference (wsid / surface id; empty for global).
    pub scope_ref: String,
    /// The registry default value.
    pub default_value: Value,
    /// Provenance of the current value.
    pub source: String,
    /// Monotonically increasing per (preference_id, scope). 0 means "never set — resolved to default".
    pub revision: i64,
    /// Redaction class.
    pub redaction_class: String,
    /// Last mutation actor (empty when resolved from default).
    pub updated_by: String,
    /// The EventLedger event id of the last mutation (empty when resolved from default).
    pub event_ledger_event_id: String,
}

impl PreferenceRecord {
    /// Build the "resolved to registry default, never set" record for `entry` in `scope`
    /// (SET-REC-003 — a read of an unset defined preference resolves deterministically to the default).
    pub fn resolved_default(entry: &PreferenceSchemaEntry, scope: &PreferenceScope) -> Self {
        Self {
            schema_id: PREFERENCE_RECORD_SCHEMA_ID.to_owned(),
            preference_id: entry.preference_id.to_owned(),
            namespace: entry.namespace.to_owned(),
            value_type: entry.value_type().as_str().to_owned(),
            value: entry.default_value.clone(),
            scope: scope.kind.as_str().to_owned(),
            scope_ref: scope.scope_ref.clone(),
            default_value: entry.default_value.clone(),
            source: PreferenceSource::Default.as_str().to_owned(),
            revision: 0,
            redaction_class: entry.redaction_class.as_str().to_owned(),
            updated_by: String::new(),
            event_ledger_event_id: String::new(),
        }
    }
}

/// A recoverable change receipt (PRIM-PreferenceChangeReceipt, SET-EVT-002) carrying before/after
/// revision and pointers to the EventLedger entry, sufficient to replay or revert the change. A reset
/// is modeled as a mutation with an explicit receipt (source=operator), never a provenance-losing delete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreferenceChangeReceipt {
    /// Schema id envelope.
    pub schema_id: String,
    /// Stable receipt id (uuid).
    pub receipt_id: String,
    /// The preference this receipt records.
    pub preference_id: String,
    /// Scope kind.
    pub scope: String,
    /// Scope reference.
    pub scope_ref: String,
    /// Revision before the change (null for the first set).
    pub before_revision: Option<i64>,
    /// Revision after the change.
    pub after_revision: i64,
    /// The value before the change (null for the first set).
    pub old_value: Option<Value>,
    /// The value after the change.
    pub new_value: Value,
    /// Provenance of the new value.
    pub source: String,
    /// The mutation actor.
    pub actor: String,
    /// The EventLedger event id of the change (also the Flight-Recorder-backed durable evidence).
    pub event_ledger_event_id: String,
    /// ISO8601 change timestamp.
    pub created_at: String,
}

/// A redacted projection row (PRIM-PreferenceProjection, SET-PROJ-002): a deterministic read-only view
/// over canonical PostgreSQL state. Non-public values are referenced by hash, never inlined.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreferenceProjectionRow {
    /// Stable id.
    pub preference_id: String,
    /// Namespace.
    pub namespace: String,
    /// Scope kind.
    pub scope: String,
    /// Effective value (redacted for non-public records).
    pub value: Value,
    /// Registry default.
    pub default_value: Value,
    /// Provenance.
    pub source: String,
    /// Current revision.
    pub revision: i64,
    /// True when [`Self::value`] was redacted rather than inlined.
    pub redacted: bool,
}

/// Build the EventLedger `PreferenceChangedEvent` payload (SET-EVT-001). Non-public values are
/// referenced by hash, never inlined; public editor values inline directly.
pub fn preference_changed_event_payload(
    receipt: &PreferenceChangeReceipt,
    redaction_class: RedactionClass,
    value_type: PreferenceValueType,
) -> Value {
    let (old_value_ref, new_value_ref) = match redaction_class {
        RedactionClass::Public => (
            receipt.old_value.clone().unwrap_or(Value::Null),
            receipt.new_value.clone(),
        ),
        RedactionClass::NonPublic => (
            receipt
                .old_value
                .as_ref()
                .map(value_hash_ref)
                .map(Value::String)
                .unwrap_or(Value::Null),
            Value::String(value_hash_ref(&receipt.new_value)),
        ),
    };
    json!({
        "type": "preference_record_changed",
        "preference_id": receipt.preference_id,
        "scope": receipt.scope,
        "scope_ref": receipt.scope_ref,
        "value_type": value_type.as_str(),
        "source": receipt.source,
        "actor": receipt.actor,
        "revision": receipt.after_revision,
        "before_revision": receipt.before_revision,
        "redaction_class": redaction_class.as_str(),
        "old_value_ref": old_value_ref,
        "new_value_ref": new_value_ref,
        "receipt_id": receipt.receipt_id,
    })
}

/// A stable opaque `sha256:<hex>` reference for a non-public value (SET-EVT-001 — never inline the raw
/// value in the event).
pub fn value_hash_ref(value: &Value) -> String {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::to_string(value).unwrap_or_default();
    let digest = Sha256::digest(canonical.as_bytes());
    format!("sha256:{digest:x}")
}

// ---------------------------------------------------------------------------
// Editor preference registry (MT-072). Every editor setting the WP ships is a
// defined preference with a stable id, value type, constraint, and default.
// ---------------------------------------------------------------------------

/// The `view-defaults` namespace (per-surface default view state — SET-SCOPE-001).
pub const VIEW_DEFAULTS_NAMESPACE: &str = "view-defaults";

/// The known syntax scope keys for the custom syntax color map (mirrors the frontend
/// `SYNTAX_SCOPE_KEYS` / MT-001 `HighlightScope` variants).
pub const SYNTAX_SCOPE_KEYS: &[&str] = &[
    "keyword", "string", "comment", "number", "function", "type", "operator", "other",
];

// Stable editor preference ids (SET-REC-004). Cited by tools, tests, the UserManual, and audits.
/// Editor text point size (distinct from chrome font — AC-002).
pub const PREF_EDITOR_FONT_SIZE: &str = "view-defaults.editor.font-size";
/// Columns per tab.
pub const PREF_EDITOR_TAB_SIZE: &str = "view-defaults.editor.tab-size";
/// Tabs-vs-spaces toggle.
pub const PREF_EDITOR_INSERT_SPACES: &str = "view-defaults.editor.insert-spaces";
/// Word-wrap mode.
pub const PREF_EDITOR_WORD_WRAP: &str = "view-defaults.editor.word-wrap";
/// Bounded word-wrap column (used when word-wrap = bounded).
pub const PREF_EDITOR_WORD_WRAP_COLUMN: &str = "view-defaults.editor.word-wrap-column";
/// Whitespace rendering mode.
pub const PREF_EDITOR_RENDER_WHITESPACE: &str = "view-defaults.editor.render-whitespace";
/// Minimap visibility.
pub const PREF_EDITOR_MINIMAP_ENABLED: &str = "view-defaults.editor.minimap-enabled";
/// Sticky-scroll band visibility.
pub const PREF_EDITOR_STICKY_SCROLL: &str = "view-defaults.editor.sticky-scroll";
/// Gutter line-number visibility.
pub const PREF_EDITOR_LINE_NUMBERS: &str = "view-defaults.editor.line-numbers";
/// Editor line-height multiplier.
pub const PREF_EDITOR_LINE_HEIGHT: &str = "view-defaults.editor.line-height";
/// Matching-bracket highlight.
pub const PREF_EDITOR_BRACKET_MATCHING: &str = "view-defaults.editor.bracket-matching";
/// Indent-guide lines.
pub const PREF_EDITOR_INDENT_GUIDES: &str = "view-defaults.editor.indent-guides";
/// Open rich documents in Reading view by default.
pub const PREF_EDITOR_READING_MODE_DEFAULT: &str = "view-defaults.editor.reading-mode-default";
/// Syntax palette mode (muted | standard | custom).
pub const PREF_EDITOR_SYNTAX_PALETTE_MODE: &str = "view-defaults.editor.syntax-palette-mode";
/// Custom per-scope syntax colors (json-object).
pub const PREF_EDITOR_SYNTAX_CUSTOM_COLORS: &str = "view-defaults.editor.syntax-custom-colors";
/// Editor keybinding overrides (json-object action->chord).
pub const PREF_EDITOR_KEYBINDING_OVERRIDES: &str = "view-defaults.editor.keybinding-overrides";

const WORD_WRAP_MEMBERS: &[&str] = &["off", "on", "bounded"];
const RENDER_WHITESPACE_MEMBERS: &[&str] = &["none", "boundary", "all"];
const SYNTAX_MODE_MEMBERS: &[&str] = &["muted", "standard", "custom"];

/// Build the full editor preference registry (SET-REC-001/002/003). This is the authoritative list of
/// preferences the settings dialog reads/writes through the PreferenceRecord authority.
pub fn editor_preference_registry() -> Vec<PreferenceSchemaEntry> {
    let entry = |preference_id: &'static str,
                 label: &'static str,
                 constraint: PreferenceConstraint,
                 default_value: Value| PreferenceSchemaEntry {
        preference_id,
        namespace: VIEW_DEFAULTS_NAMESPACE,
        scope_kind: PreferenceScopeKind::Workspace,
        label,
        constraint,
        redaction_class: RedactionClass::Public,
        default_value,
    };
    vec![
        entry(
            PREF_EDITOR_FONT_SIZE,
            "Editor font size",
            PreferenceConstraint::FloatRange { min: 6.0, max: 48.0 },
            json!(13.0),
        ),
        entry(
            PREF_EDITOR_TAB_SIZE,
            "Tab size",
            PreferenceConstraint::IntRange { min: 1, max: 16 },
            json!(4),
        ),
        entry(
            PREF_EDITOR_INSERT_SPACES,
            "Insert spaces",
            PreferenceConstraint::Bool,
            json!(true),
        ),
        entry(
            PREF_EDITOR_WORD_WRAP,
            "Word wrap",
            PreferenceConstraint::Enum(WORD_WRAP_MEMBERS),
            json!("off"),
        ),
        entry(
            PREF_EDITOR_WORD_WRAP_COLUMN,
            "Word wrap column",
            PreferenceConstraint::IntRange { min: 20, max: 400 },
            json!(80),
        ),
        entry(
            PREF_EDITOR_RENDER_WHITESPACE,
            "Render whitespace",
            PreferenceConstraint::Enum(RENDER_WHITESPACE_MEMBERS),
            json!("none"),
        ),
        entry(
            PREF_EDITOR_MINIMAP_ENABLED,
            "Minimap",
            PreferenceConstraint::Bool,
            json!(true),
        ),
        entry(
            PREF_EDITOR_STICKY_SCROLL,
            "Sticky scroll",
            PreferenceConstraint::Bool,
            json!(true),
        ),
        entry(
            PREF_EDITOR_LINE_NUMBERS,
            "Line numbers",
            PreferenceConstraint::Bool,
            json!(true),
        ),
        entry(
            PREF_EDITOR_LINE_HEIGHT,
            "Line height",
            PreferenceConstraint::FloatRange { min: 1.0, max: 2.0 },
            json!(1.0),
        ),
        entry(
            PREF_EDITOR_BRACKET_MATCHING,
            "Bracket matching",
            PreferenceConstraint::Bool,
            json!(true),
        ),
        entry(
            PREF_EDITOR_INDENT_GUIDES,
            "Indent guides",
            PreferenceConstraint::Bool,
            json!(true),
        ),
        entry(
            PREF_EDITOR_READING_MODE_DEFAULT,
            "Reading mode by default",
            PreferenceConstraint::Bool,
            json!(false),
        ),
        entry(
            PREF_EDITOR_SYNTAX_PALETTE_MODE,
            "Syntax palette mode",
            PreferenceConstraint::Enum(SYNTAX_MODE_MEMBERS),
            json!("standard"),
        ),
        entry(
            PREF_EDITOR_SYNTAX_CUSTOM_COLORS,
            "Custom syntax colors",
            PreferenceConstraint::SyntaxColorMap(SYNTAX_SCOPE_KEYS),
            json!({}),
        ),
        entry(
            PREF_EDITOR_KEYBINDING_OVERRIDES,
            "Editor keybinding overrides",
            PreferenceConstraint::ChordMap,
            json!({}),
        ),
    ]
}

/// Look up a registry schema entry by preference id, if defined.
pub fn lookup_editor_preference(preference_id: &str) -> Option<PreferenceSchemaEntry> {
    editor_preference_registry()
        .into_iter()
        .find(|entry| entry.preference_id == preference_id)
}

/// The set of editor preference ids in a stable object map (for the redacted projection default view).
pub fn editor_preference_defaults_map() -> Map<String, Value> {
    editor_preference_registry()
        .into_iter()
        .map(|entry| (entry.preference_id.to_owned(), entry.default_value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique_and_namespaced() {
        let registry = editor_preference_registry();
        let mut ids: Vec<&str> = registry.iter().map(|e| e.preference_id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate preference ids in registry");
        for entry in &registry {
            assert!(
                entry.preference_id.starts_with("view-defaults.editor."),
                "{} is not namespaced under view-defaults.editor",
                entry.preference_id
            );
        }
    }

    #[test]
    fn every_default_validates_against_its_own_constraint() {
        for entry in editor_preference_registry() {
            entry
                .validate(&entry.default_value)
                .unwrap_or_else(|e| panic!("registry default fails its own constraint: {e}"));
        }
    }

    #[test]
    fn int_range_rejects_out_of_range_and_wrong_type() {
        let entry = lookup_editor_preference(PREF_EDITOR_TAB_SIZE).unwrap();
        assert!(entry.validate(&json!(4)).is_ok());
        assert!(entry.validate(&json!(0)).is_err());
        assert!(entry.validate(&json!(17)).is_err());
        assert!(entry.validate(&json!("4")).is_err());
        assert!(entry.validate(&json!(true)).is_err());
    }

    #[test]
    fn float_range_rejects_out_of_range() {
        let entry = lookup_editor_preference(PREF_EDITOR_FONT_SIZE).unwrap();
        assert!(entry.validate(&json!(13.0)).is_ok());
        assert!(entry.validate(&json!(5.9)).is_err());
        assert!(entry.validate(&json!(48.1)).is_err());
    }

    #[test]
    fn enum_rejects_unknown_member() {
        let entry = lookup_editor_preference(PREF_EDITOR_WORD_WRAP).unwrap();
        assert!(entry.validate(&json!("off")).is_ok());
        assert!(entry.validate(&json!("bounded")).is_ok());
        assert!(entry.validate(&json!("wrap-everything")).is_err());
    }

    #[test]
    fn syntax_color_map_validates_shape() {
        let entry = lookup_editor_preference(PREF_EDITOR_SYNTAX_CUSTOM_COLORS).unwrap();
        assert!(entry.validate(&json!({"keyword": [255, 0, 0, 255]})).is_ok());
        assert!(entry.validate(&json!({"keyword": [255, 0, 0]})).is_err());
        assert!(entry.validate(&json!({"keyword": [256, 0, 0, 255]})).is_err());
        assert!(entry.validate(&json!({"nope": [1, 2, 3, 4]})).is_err());
    }

    #[test]
    fn chord_map_validates_shape() {
        let entry = lookup_editor_preference(PREF_EDITOR_KEYBINDING_OVERRIDES).unwrap();
        assert!(entry.validate(&json!({"open_find": "Ctrl+F"})).is_ok());
        assert!(entry.validate(&json!({"open_find": ""})).is_err());
        assert!(entry.validate(&json!({"": "Ctrl+F"})).is_err());
        assert!(entry.validate(&json!({"open_find": 5})).is_err());
    }

    #[test]
    fn nonpublic_event_payload_hashes_value() {
        let receipt = PreferenceChangeReceipt {
            schema_id: PREFERENCE_CHANGE_RECEIPT_SCHEMA_ID.to_owned(),
            receipt_id: "r1".to_owned(),
            preference_id: PREF_EDITOR_FONT_SIZE.to_owned(),
            scope: "workspace".to_owned(),
            scope_ref: "ws1".to_owned(),
            before_revision: None,
            after_revision: 1,
            old_value: None,
            new_value: json!("secret"),
            source: "operator".to_owned(),
            actor: "op".to_owned(),
            event_ledger_event_id: "e1".to_owned(),
            created_at: "2026-07-23T00:00:00Z".to_owned(),
        };
        let payload = preference_changed_event_payload(
            &receipt,
            RedactionClass::NonPublic,
            PreferenceValueType::String,
        );
        let new_ref = payload["new_value_ref"].as_str().unwrap();
        assert!(new_ref.starts_with("sha256:"));
        assert_ne!(new_ref, "secret");
    }
}
