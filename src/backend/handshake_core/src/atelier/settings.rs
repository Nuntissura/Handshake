//! General settings / preferences domain (MT-200).
//!
//! Translates the legacy source "Global Settings" surface
//! (`src/ui/views/SettingsView.tsx`, `app/main.js` JSON config blob with
//! `libraryRoot` and runtime settings, saved via `saveConfig`) into a typed,
//! scoped preference store on PostgreSQL. legacy source stored an untyped JSON file on
//! the local filesystem; Handshake forbids that (no SQLite, no localhost
//! authority). Here every preference is a typed record with an explicit value
//! type, an explicit scope (global vs per-character, mirroring the legacy source split
//! "application-wide paths and runtime settings ... character-specific options
//! stay on the character"), scoped namespace validation, portable path
//! identifiers, and server-side type validation. This store is not secret
//! authority; secret-bearing keys and redaction-only values are rejected before
//! storage.
//!
//! This is the *general* preferences domain and is intentionally distinct from
//! LLM/provider configuration, which is governed separately.
//!
//! Source: legacy source `src/ui/views/SettingsView.tsx`, `app/main.js`
//! (saveConfig/getConfigInfo). Microtasks: MT-200 (settings/preferences),
//! MT-005 (event coverage).

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{AtelierError, AtelierResult, AtelierStore};

/// Event families emitted by the settings/preferences domain (MT-200, MT-005).
///
/// Defined here so the parent can fold these into `event_family::ALL` and the
/// MT-005 coverage check picks up settings mutations.
pub mod settings_event_family {
    pub const PREFERENCE_SET: &str = "atelier.preference.set";
    pub const PREFERENCE_RESET_TO_DEFAULT: &str = "atelier.preference.reset_to_default";
    pub const PREFERENCE_DELETED: &str = "atelier.preference.deleted";
    pub const RETENTION_PRUNE_CONFIRMED: &str = "atelier.preference.retention_prune_confirmed";

    /// All settings event families (for parity/coverage folding).
    pub const ALL: &[&str] = &[
        PREFERENCE_SET,
        PREFERENCE_RESET_TO_DEFAULT,
        PREFERENCE_DELETED,
        RETENTION_PRUNE_CONFIRMED,
    ];
}

const RETENTION_DEFAULT_POLICY_KEY: &str = "retention.default-policy";

/// Declared type of a preference value. Drives server-side validation so a
/// `Bool` preference cannot silently hold `"maybe"`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceType {
    String,
    Bool,
    Integer,
    Float,
    Json,
    Path,
}

impl PreferenceType {
    /// Stable string token persisted in the `value_type` column.
    pub fn as_str(self) -> &'static str {
        match self {
            PreferenceType::String => "string",
            PreferenceType::Bool => "bool",
            PreferenceType::Integer => "integer",
            PreferenceType::Float => "float",
            PreferenceType::Json => "json",
            PreferenceType::Path => "path",
        }
    }

    fn from_str(raw: &str) -> AtelierResult<PreferenceType> {
        match raw {
            "string" => Ok(PreferenceType::String),
            "bool" => Ok(PreferenceType::Bool),
            "integer" => Ok(PreferenceType::Integer),
            "float" => Ok(PreferenceType::Float),
            "json" => Ok(PreferenceType::Json),
            "path" => Ok(PreferenceType::Path),
            other => Err(AtelierError::Validation(format!(
                "unknown preference value_type: {other}"
            ))),
        }
    }

    /// Validate a raw string value against the declared type. Values are stored
    /// as text (JSON values as serialized JSON) and validated on the way in.
    fn validate(self, value: &str) -> AtelierResult<()> {
        match self {
            PreferenceType::String => Ok(()),
            PreferenceType::Path => validate_portable_path_identifier(value),
            PreferenceType::Bool => match value {
                "true" | "false" => Ok(()),
                other => Err(AtelierError::Validation(format!(
                    "bool preference must be 'true' or 'false', got {other:?}"
                ))),
            },
            PreferenceType::Integer => value.parse::<i64>().map(|_| ()).map_err(|_| {
                AtelierError::Validation(format!("integer preference invalid: {value:?}"))
            }),
            PreferenceType::Float => value.parse::<f64>().map(|_| ()).map_err(|_| {
                AtelierError::Validation(format!("float preference invalid: {value:?}"))
            }),
            PreferenceType::Json => serde_json::from_str::<serde_json::Value>(value)
                .map(|_| ())
                .map_err(|e| AtelierError::Validation(format!("json preference invalid: {e}"))),
        }
    }
}

/// Scope a preference applies to. `Global` mirrors legacy source application-wide
/// settings; `Character` mirrors "character-specific options stay on the
/// character" and links to `atelier_character(internal_id)`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceScope {
    Global,
    Character(Uuid),
}

impl PreferenceScope {
    fn kind(&self) -> &'static str {
        match self {
            PreferenceScope::Global => "global",
            PreferenceScope::Character(_) => "character",
        }
    }

    fn character_id(&self) -> Option<Uuid> {
        match self {
            PreferenceScope::Global => None,
            PreferenceScope::Character(id) => Some(*id),
        }
    }

    fn from_parts(kind: &str, character_id: Option<Uuid>) -> AtelierResult<PreferenceScope> {
        match kind {
            "global" => Ok(PreferenceScope::Global),
            "character" => {
                let id = character_id.ok_or_else(|| {
                    AtelierError::Validation(
                        "character-scoped preference missing character_internal_id".into(),
                    )
                })?;
                Ok(PreferenceScope::Character(id))
            }
            other => Err(AtelierError::Validation(format!(
                "unknown preference scope kind: {other}"
            ))),
        }
    }
}

/// Where the effective value came from. `Default` means a registry default is
/// active; it may be projected without creating an authority row.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceValueSource {
    Operator,
    Default,
}

impl PreferenceValueSource {
    pub fn as_str(self) -> &'static str {
        match self {
            PreferenceValueSource::Operator => "operator",
            PreferenceValueSource::Default => "default",
        }
    }

    fn from_str(raw: &str) -> AtelierResult<PreferenceValueSource> {
        match raw {
            "operator" => Ok(PreferenceValueSource::Operator),
            "default" => Ok(PreferenceValueSource::Default),
            other => Err(AtelierError::Validation(format!(
                "unknown preference source: {other}"
            ))),
        }
    }
}

/// A typed preference record as stored.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Preference {
    pub preference_id: Uuid,
    pub scope: PreferenceScope,
    pub key: String,
    pub namespace: String,
    pub name: String,
    pub value_type: PreferenceType,
    /// Raw stored value (text; JSON values are serialized JSON text).
    pub value: String,
    pub default_value: Option<String>,
    pub source: PreferenceValueSource,
    /// Whether the value is a secret. Redacted projections never expose it.
    pub redacted: bool,
    pub updated_by: Option<String>,
    pub revision: i64,
    pub redaction_class: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

const REDACTED_PLACEHOLDER: &str = "[REDACTED]";

impl Preference {
    /// Whether this preference holds a secret value.
    pub fn is_secret(&self) -> bool {
        self.redacted
    }

    /// A projection safe to show on operator surfaces and logs: secret values
    /// are replaced with a placeholder, non-secret values pass through.
    pub fn redacted_value(&self) -> &str {
        if self.redacted {
            REDACTED_PLACEHOLDER
        } else {
            &self.value
        }
    }

    /// A clone of this preference with any secret value replaced by the
    /// redaction placeholder. Use this for serialization to untrusted sinks.
    pub fn redacted_projection(&self) -> Preference {
        let mut projected = self.clone();
        if projected.redacted {
            projected.value = REDACTED_PLACEHOLDER.to_string();
        }
        projected
    }
}

/// Input for setting (create-or-update) a preference.
#[derive(Clone, Debug)]
pub struct SetPreference {
    pub scope: PreferenceScope,
    pub key: String,
    pub value_type: PreferenceType,
    pub value: String,
    pub redacted: bool,
}

/// Effective operator-safe projection for a setting. Defined defaults produce
/// this shape even when no `atelier_preference` row exists.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectivePreference {
    pub preference_id: Option<Uuid>,
    pub scope: PreferenceScope,
    pub key: String,
    pub namespace: String,
    pub name: String,
    pub value_type: PreferenceType,
    pub value: String,
    pub default_value: Option<String>,
    pub source: PreferenceValueSource,
    pub redacted: bool,
    pub revision: i64,
    pub redaction_class: String,
    pub updated_at_utc: Option<DateTime<Utc>>,
}

/// Recoverable receipt for setting mutations. It carries before/after revision
/// and value/source state so a later model can reason about reset/replay without
/// parsing event text.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreferenceChangeReceipt {
    pub receipt_id: Uuid,
    pub event_family: String,
    pub preference: Preference,
    pub revision_before: Option<i64>,
    pub revision_after: i64,
    pub value_before: Option<String>,
    pub value_after: String,
    pub source_before: Option<PreferenceValueSource>,
    pub source_after: PreferenceValueSource,
}

/// Governed vocabulary for atelier retention behavior. Settings may bind one
/// of these policies, but they never perform deletion by themselves.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RetentionDefaultPolicy {
    Retain,
    ReviewBeforePrune,
    PruneAfter30Days,
}

impl RetentionDefaultPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            RetentionDefaultPolicy::Retain => "retain",
            RetentionDefaultPolicy::ReviewBeforePrune => "review-before-prune",
            RetentionDefaultPolicy::PruneAfter30Days => "prune-after-30d",
        }
    }

    fn from_str(raw: &str) -> AtelierResult<Self> {
        match raw {
            "retain" => Ok(RetentionDefaultPolicy::Retain),
            "review-before-prune" => Ok(RetentionDefaultPolicy::ReviewBeforePrune),
            "prune-after-30d" => Ok(RetentionDefaultPolicy::PruneAfter30Days),
            other => Err(AtelierError::Validation(format!(
                "retention.default-policy value {other:?} is not allowed; use retain, review-before-prune, or prune-after-30d"
            ))),
        }
    }

    pub fn prune_after_days(self) -> Option<u32> {
        match self {
            RetentionDefaultPolicy::PruneAfter30Days => Some(30),
            RetentionDefaultPolicy::Retain | RetentionDefaultPolicy::ReviewBeforePrune => None,
        }
    }

    pub fn prune_confirmation_required(self) -> bool {
        match self {
            RetentionDefaultPolicy::Retain => false,
            RetentionDefaultPolicy::ReviewBeforePrune
            | RetentionDefaultPolicy::PruneAfter30Days => true,
        }
    }
}

/// Effective retention policy binding projected from settings. This shape is
/// explicit so later prune code can check policy and confirmation state without
/// parsing arbitrary strings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetentionPolicyBinding {
    pub scope: PreferenceScope,
    pub key: String,
    pub preference_id: Option<Uuid>,
    pub policy: RetentionDefaultPolicy,
    pub value_source: PreferenceValueSource,
    pub revision: i64,
    pub prune_after_days: Option<u32>,
    pub prune_confirmation_required: bool,
    pub automatic_prune_allowed: bool,
    pub updated_at_utc: Option<DateTime<Utc>>,
}

/// Audit receipt proving an operator/model lane explicitly confirmed that a
/// retention-bound prune may be considered. It is a prerequisite signal only;
/// it does not delete anything.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetentionPruneConfirmation {
    pub confirmation_id: Uuid,
    pub event_family: String,
    pub binding: RetentionPolicyBinding,
    pub confirmed_by: String,
    pub confirmed_at_utc: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug)]
struct PreferenceDefinition {
    key: &'static str,
    namespace: &'static str,
    name: &'static str,
    value_type: PreferenceType,
    default_value: &'static str,
}

const PREFERENCE_DEFINITIONS: &[PreferenceDefinition] = &[
    PreferenceDefinition {
        key: "data-roots.library-root",
        namespace: "data-roots",
        name: "library-root",
        value_type: PreferenceType::Path,
        default_value: "data-root:library",
    },
    PreferenceDefinition {
        key: "view-defaults.asset-grid-density",
        namespace: "view-defaults",
        name: "asset-grid-density",
        value_type: PreferenceType::String,
        default_value: "comfortable",
    },
    PreferenceDefinition {
        key: RETENTION_DEFAULT_POLICY_KEY,
        namespace: "retention",
        name: "default-policy",
        value_type: PreferenceType::String,
        default_value: "retain",
    },
    PreferenceDefinition {
        key: "feature-toggles.atelier-diagnostics",
        namespace: "feature-toggles",
        name: "atelier-diagnostics",
        value_type: PreferenceType::Bool,
        default_value: "true",
    },
];

const ALLOWED_PREFERENCE_NAMESPACES: &[&str] = &[
    "data-roots",
    "view-defaults",
    "retention",
    "feature-toggles",
];
const SECRET_KEY_MARKERS: &[&str] = &[
    "secret",
    "token",
    "password",
    "credential",
    "api-key",
    "api_key",
    "apikey",
];

fn preference_definition(key: &str) -> Option<&'static PreferenceDefinition> {
    PREFERENCE_DEFINITIONS
        .iter()
        .find(|definition| definition.key == key)
}

fn validate_preference_input(input: &SetPreference) -> AtelierResult<()> {
    validate_preference_key(&input.key)?;
    if input.redacted {
        return Err(AtelierError::Validation(
            "settings preferences must not store redacted or secret-bearing values".into(),
        ));
    }

    let key_lc = input.key.to_ascii_lowercase();
    if SECRET_KEY_MARKERS
        .iter()
        .any(|marker| key_lc.contains(marker))
    {
        return Err(AtelierError::Validation(format!(
            "settings preference key {:?} looks secret-bearing; use the configured secret authority",
            input.key
        )));
    }

    Ok(())
}

fn validate_defined_preference_value(key: &str, value: &str) -> AtelierResult<()> {
    if key == RETENTION_DEFAULT_POLICY_KEY {
        RetentionDefaultPolicy::from_str(value)?;
    }
    Ok(())
}

fn split_preference_key(key: &str) -> AtelierResult<(&str, &str)> {
    if key.trim().is_empty() {
        return Err(AtelierError::Validation(
            "preference key must not be empty".into(),
        ));
    }
    if key.trim() != key {
        return Err(AtelierError::Validation(
            "preference key must not contain leading or trailing whitespace".into(),
        ));
    }

    let (namespace, name) = key.split_once('.').ok_or_else(|| {
        AtelierError::Validation(
            "preference key must use a dotted namespace such as data-roots.library-root".into(),
        )
    })?;
    if name.is_empty() {
        return Err(AtelierError::Validation(
            "preference key must include a name after the namespace".into(),
        ));
    }
    if !ALLOWED_PREFERENCE_NAMESPACES.contains(&namespace) {
        return Err(AtelierError::Validation(format!(
            "preference namespace {namespace:?} is not allowed"
        )));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(AtelierError::Validation(format!(
            "preference name {name:?} contains unsupported characters"
        )));
    }
    Ok((namespace, name))
}

fn validate_preference_key(key: &str) -> AtelierResult<()> {
    split_preference_key(key)?;
    Ok(())
}

fn validate_portable_path_identifier(value: &str) -> AtelierResult<()> {
    if value.trim().is_empty() {
        return Err(AtelierError::Validation(
            "path preference must not be empty".into(),
        ));
    }
    if value.trim() != value {
        return Err(AtelierError::Validation(
            "path preference must not contain leading or trailing whitespace".into(),
        ));
    }

    let lower = value.to_ascii_lowercase();
    let looks_machine_local = value.contains('\\')
        || value.starts_with('/')
        || value.starts_with('~')
        || value.starts_with("//")
        || lower.starts_with("file:")
        || lower.starts_with("http://localhost")
        || lower.starts_with("https://localhost")
        || lower.starts_with("http://127.0.0.1")
        || lower.starts_with("https://127.0.0.1")
        || matches!(value.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic());

    if looks_machine_local {
        return Err(AtelierError::Validation(format!(
            "path preference {value:?} must be a portable logical identifier"
        )));
    }

    let (scheme, identifier) = value.split_once(':').ok_or_else(|| {
        AtelierError::Validation(
            "path preference must be a portable logical identifier such as data-root:library"
                .into(),
        )
    })?;
    if scheme.is_empty() || identifier.is_empty() {
        return Err(AtelierError::Validation(
            "path preference logical identifier must include a non-empty scheme and value".into(),
        ));
    }
    if !scheme
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(AtelierError::Validation(format!(
            "path preference scheme {scheme:?} contains unsupported characters"
        )));
    }
    if !identifier
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(AtelierError::Validation(format!(
            "path preference identifier {identifier:?} contains unsupported characters"
        )));
    }
    if identifier.contains("..") {
        return Err(AtelierError::Validation(
            "path preference identifier must not contain parent traversal".into(),
        ));
    }
    Ok(())
}

fn preference_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<Preference> {
    let scope_kind: String = row.get("scope_kind");
    let character_id: Option<Uuid> = row.get("character_internal_id");
    let value_type_raw: String = row.get("value_type");
    let source_raw: String = row.get("source");
    Ok(Preference {
        preference_id: row.get("preference_id"),
        scope: PreferenceScope::from_parts(&scope_kind, character_id)?,
        key: row.get("key"),
        namespace: row.get("namespace"),
        name: row.get("name"),
        value_type: PreferenceType::from_str(&value_type_raw)?,
        value: row.get("value"),
        default_value: row.get("default_value"),
        source: PreferenceValueSource::from_str(&source_raw)?,
        redacted: row.get("redacted"),
        updated_by: row.get("updated_by"),
        revision: row.get("revision"),
        redaction_class: row.get("redaction_class"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

const PREF_COLUMNS: &str = "preference_id, scope_kind, character_internal_id, key, value_type, \
                            value, redacted, namespace, name, default_value, source, updated_by, \
                            revision, redaction_class, created_at_utc, updated_at_utc";

fn effective_from_preference(preference: Preference, redact: bool) -> EffectivePreference {
    let value = if redact {
        preference.redacted_value().to_string()
    } else {
        preference.value.clone()
    };
    EffectivePreference {
        preference_id: Some(preference.preference_id),
        scope: preference.scope,
        key: preference.key,
        namespace: preference.namespace,
        name: preference.name,
        value_type: preference.value_type,
        value,
        default_value: preference.default_value,
        source: preference.source,
        redacted: preference.redacted,
        revision: preference.revision,
        redaction_class: preference.redaction_class,
        updated_at_utc: Some(preference.updated_at_utc),
    }
}

fn effective_from_definition(
    scope: PreferenceScope,
    definition: &PreferenceDefinition,
) -> EffectivePreference {
    EffectivePreference {
        preference_id: None,
        scope,
        key: definition.key.to_string(),
        namespace: definition.namespace.to_string(),
        name: definition.name.to_string(),
        value_type: definition.value_type,
        value: definition.default_value.to_string(),
        default_value: Some(definition.default_value.to_string()),
        source: PreferenceValueSource::Default,
        redacted: false,
        revision: 0,
        redaction_class: "public".to_string(),
        updated_at_utc: None,
    }
}

impl AtelierStore {
    /// Set (create or update) a typed preference. The value is validated
    /// against `value_type` server-side. Upsert key is (scope_kind,
    /// character_internal_id, key) so a given key is unique within its scope;
    /// re-setting the same key updates the value in place. Emits
    /// `PREFERENCE_SET`. Secret values are never echoed into the event payload.
    pub async fn set_preference(&self, input: &SetPreference) -> AtelierResult<Preference> {
        Ok(self.set_preference_with_receipt(input).await?.preference)
    }

    /// Set a preference and return a recoverable receipt with before/after
    /// revision metadata.
    pub async fn set_preference_with_receipt(
        &self,
        input: &SetPreference,
    ) -> AtelierResult<PreferenceChangeReceipt> {
        validate_preference_input(input)?;
        input.value_type.validate(&input.value)?;
        let (namespace, name) = split_preference_key(&input.key)?;
        let definition = preference_definition(&input.key);
        if let Some(definition) = definition {
            if definition.value_type != input.value_type {
                return Err(AtelierError::Validation(format!(
                    "preference {:?} must use value_type {:?}",
                    input.key, definition.value_type
                )));
            }
        }
        validate_defined_preference_value(&input.key, &input.value)?;
        let default_value = definition.map(|definition| definition.default_value.to_string());
        let before = self.get_preference(input.scope, &input.key).await?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_preference
                 (scope_kind, character_internal_id, key, namespace, name, value_type, value,
                  redacted, default_value, source, updated_by, revision, redaction_class)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, 1, 'public')
               ON CONFLICT (scope_kind, character_internal_id, key)
               DO UPDATE SET value      = EXCLUDED.value,
                             namespace  = EXCLUDED.namespace,
                             name       = EXCLUDED.name,
                             value_type = EXCLUDED.value_type,
                             redacted   = EXCLUDED.redacted,
                             default_value = EXCLUDED.default_value,
                             source     = EXCLUDED.source,
                             updated_by = EXCLUDED.updated_by,
                             revision   = atelier_preference.revision + 1,
                             redaction_class = EXCLUDED.redaction_class,
                             updated_at_utc = NOW()
               RETURNING {PREF_COLUMNS}"#
        ))
        .bind(input.scope.kind())
        .bind(input.scope.character_id())
        .bind(&input.key)
        .bind(namespace)
        .bind(name)
        .bind(input.value_type.as_str())
        .bind(&input.value)
        .bind(input.redacted)
        .bind(default_value.clone())
        .bind(PreferenceValueSource::Operator.as_str())
        .fetch_one(self.pool())
        .await?;

        let preference = preference_from_row(&row)?;
        let receipt = PreferenceChangeReceipt {
            receipt_id: Uuid::now_v7(),
            event_family: settings_event_family::PREFERENCE_SET.to_string(),
            revision_before: before.as_ref().map(|preference| preference.revision),
            revision_after: preference.revision,
            value_before: before.as_ref().map(|preference| preference.value.clone()),
            value_after: preference.value.clone(),
            source_before: before.as_ref().map(|preference| preference.source),
            source_after: preference.source,
            preference: preference.clone(),
        };
        self.record_event(
            settings_event_family::PREFERENCE_SET,
            "atelier_preference",
            &preference.preference_id.to_string(),
            serde_json::json!({
                "preference_id": preference.preference_id,
                "receipt_id": receipt.receipt_id,
                "scope_kind": preference.scope.kind(),
                "character_scoped": preference.scope.character_id().is_some(),
                "key": preference.key.clone(),
                "namespace": preference.namespace.clone(),
                "name": preference.name.clone(),
                "value_type": preference.value_type.as_str(),
                "redacted": preference.redacted,
                "source": preference.source.as_str(),
                "revision_before": receipt.revision_before,
                "revision_after": receipt.revision_after,
                "value_before": receipt.value_before.clone(),
                "value_after": receipt.value_after.clone(),
                "source_before": receipt.source_before.map(|source| source.as_str()),
                "source_after": receipt.source_after.as_str(),
                "default_value": preference.default_value.clone(),
                // Redacted projection: never leak secret values into the ledger.
                "value": preference.redacted_value(),
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// Fetch a single preference by scope + key.
    pub async fn get_preference(
        &self,
        scope: PreferenceScope,
        key: &str,
    ) -> AtelierResult<Option<Preference>> {
        let row = sqlx::query(&format!(
            r#"SELECT {PREF_COLUMNS}
               FROM atelier_preference
               WHERE scope_kind = $1
                 AND character_internal_id IS NOT DISTINCT FROM $2::uuid
                 AND key = $3"#
        ))
        .bind(scope.kind())
        .bind(scope.character_id())
        .bind(key)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(preference_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// Fetch an effective preference. If a defined preference is unset, returns
    /// its registry default without creating an authority row.
    pub async fn get_effective_preference(
        &self,
        scope: PreferenceScope,
        key: &str,
    ) -> AtelierResult<EffectivePreference> {
        validate_preference_key(key)?;
        if let Some(preference) = self.get_preference(scope, key).await? {
            return Ok(effective_from_preference(preference, false));
        }
        let definition = preference_definition(key).ok_or_else(|| {
            AtelierError::Validation(format!(
                "preference {key:?} is unset and has no registered default"
            ))
        })?;
        definition.value_type.validate(definition.default_value)?;
        Ok(effective_from_definition(scope, definition))
    }

    /// Resolve a preference value, falling back to a provided default when the
    /// key is unset. Mirrors legacy source's "(default)" projection for unset settings
    /// (e.g. an unset `libraryRoot` shows as default). The default is NOT
    /// validated or persisted; it is returned as-is.
    pub async fn get_preference_value_or_default(
        &self,
        scope: PreferenceScope,
        key: &str,
        default: &str,
    ) -> AtelierResult<String> {
        match self.get_preference(scope, key).await? {
            Some(pref) => Ok(pref.value),
            None => Ok(default.to_string()),
        }
    }

    /// List all preferences in a scope, ordered by key. Pass `redact = true`
    /// for an operator-safe projection where secret values are masked.
    pub async fn list_preferences(
        &self,
        scope: PreferenceScope,
        redact: bool,
    ) -> AtelierResult<Vec<Preference>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {PREF_COLUMNS}
               FROM atelier_preference
               WHERE scope_kind = $1
                 AND character_internal_id IS NOT DISTINCT FROM $2::uuid
               ORDER BY key ASC"#
        ))
        .bind(scope.kind())
        .bind(scope.character_id())
        .fetch_all(self.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let pref = preference_from_row(row)?;
            out.push(if redact {
                pref.redacted_projection()
            } else {
                pref
            });
        }
        Ok(out)
    }

    /// List the operator/model projection for a scope, including registry
    /// defaults for unset preferences. Pass `redact = true` for operator-safe
    /// values.
    pub async fn list_preference_projection(
        &self,
        scope: PreferenceScope,
        redact: bool,
    ) -> AtelierResult<Vec<EffectivePreference>> {
        let preferences = self.list_preferences(scope, false).await?;
        let mut seen_keys = HashSet::with_capacity(preferences.len());
        let mut out = Vec::with_capacity(preferences.len() + PREFERENCE_DEFINITIONS.len());

        for preference in preferences {
            seen_keys.insert(preference.key.clone());
            out.push(effective_from_preference(preference, redact));
        }

        for definition in PREFERENCE_DEFINITIONS {
            if !seen_keys.contains(definition.key) {
                definition.value_type.validate(definition.default_value)?;
                out.push(effective_from_definition(scope, definition));
            }
        }

        out.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(out)
    }

    /// Reset a registered preference to its default without deleting the row,
    /// preserving provenance through a revision bump and reset receipt.
    pub async fn reset_preference_to_default(
        &self,
        scope: PreferenceScope,
        key: &str,
    ) -> AtelierResult<PreferenceChangeReceipt> {
        validate_preference_key(key)?;
        let definition = preference_definition(key).ok_or_else(|| {
            AtelierError::Validation(format!(
                "preference {key:?} cannot be reset because it has no registered default"
            ))
        })?;
        definition.value_type.validate(definition.default_value)?;
        let before = self.get_preference(scope, key).await?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_preference
                 (scope_kind, character_internal_id, key, namespace, name, value_type, value,
                  redacted, default_value, source, updated_by, revision, redaction_class)
               VALUES ($1, $2, $3, $4, $5, $6, $7, FALSE, $8, $9, NULL, 1, 'public')
               ON CONFLICT (scope_kind, character_internal_id, key)
               DO UPDATE SET value      = EXCLUDED.value,
                             namespace  = EXCLUDED.namespace,
                             name       = EXCLUDED.name,
                             value_type = EXCLUDED.value_type,
                             redacted   = FALSE,
                             default_value = EXCLUDED.default_value,
                             source     = EXCLUDED.source,
                             updated_by = NULL,
                             revision   = atelier_preference.revision + 1,
                             redaction_class = EXCLUDED.redaction_class,
                             updated_at_utc = NOW()
               RETURNING {PREF_COLUMNS}"#
        ))
        .bind(scope.kind())
        .bind(scope.character_id())
        .bind(definition.key)
        .bind(definition.namespace)
        .bind(definition.name)
        .bind(definition.value_type.as_str())
        .bind(definition.default_value)
        .bind(definition.default_value)
        .bind(PreferenceValueSource::Default.as_str())
        .fetch_one(self.pool())
        .await?;

        let preference = preference_from_row(&row)?;
        let receipt = PreferenceChangeReceipt {
            receipt_id: Uuid::now_v7(),
            event_family: settings_event_family::PREFERENCE_RESET_TO_DEFAULT.to_string(),
            revision_before: before.as_ref().map(|preference| preference.revision),
            revision_after: preference.revision,
            value_before: before.as_ref().map(|preference| preference.value.clone()),
            value_after: preference.value.clone(),
            source_before: before.as_ref().map(|preference| preference.source),
            source_after: preference.source,
            preference: preference.clone(),
        };

        self.record_event(
            settings_event_family::PREFERENCE_RESET_TO_DEFAULT,
            "atelier_preference",
            &preference.preference_id.to_string(),
            serde_json::json!({
                "preference_id": preference.preference_id,
                "receipt_id": receipt.receipt_id,
                "scope_kind": preference.scope.kind(),
                "character_scoped": preference.scope.character_id().is_some(),
                "key": preference.key.clone(),
                "namespace": preference.namespace.clone(),
                "name": preference.name.clone(),
                "value_type": preference.value_type.as_str(),
                "revision_before": receipt.revision_before,
                "revision_after": receipt.revision_after,
                "value_before": receipt.value_before.clone(),
                "value_after": receipt.value_after.clone(),
                "source_before": receipt.source_before.map(|source| source.as_str()),
                "source_after": receipt.source_after.as_str(),
                "default_value": preference.default_value.clone(),
            }),
        )
        .await?;

        Ok(receipt)
    }

    /// Project the active retention policy binding for a scope. The default
    /// `retain` value is returned even when no preference row exists.
    pub async fn get_retention_policy_binding(
        &self,
        scope: PreferenceScope,
    ) -> AtelierResult<RetentionPolicyBinding> {
        let effective = self
            .get_effective_preference(scope, RETENTION_DEFAULT_POLICY_KEY)
            .await?;
        let policy = RetentionDefaultPolicy::from_str(&effective.value)?;
        Ok(RetentionPolicyBinding {
            scope: effective.scope,
            key: effective.key,
            preference_id: effective.preference_id,
            policy,
            value_source: effective.source,
            revision: effective.revision,
            prune_after_days: policy.prune_after_days(),
            prune_confirmation_required: policy.prune_confirmation_required(),
            // Settings bind policy and emit confirmations; deletion is handled
            // by a separate retention service with its own audit report.
            automatic_prune_allowed: false,
            updated_at_utc: effective.updated_at_utc,
        })
    }

    /// Emit an explicit retention prune confirmation event. This is a governed
    /// confirmation receipt only; it never performs deletion and it rejects the
    /// default `retain` policy.
    pub async fn confirm_retention_prune(
        &self,
        scope: PreferenceScope,
        confirmed_by: &str,
    ) -> AtelierResult<RetentionPruneConfirmation> {
        let confirmed_by = confirmed_by.trim();
        if confirmed_by.is_empty() {
            return Err(AtelierError::Validation(
                "retention prune confirmation requires confirmed_by".into(),
            ));
        }

        let binding = self.get_retention_policy_binding(scope).await?;
        if !binding.prune_confirmation_required {
            return Err(AtelierError::Validation(format!(
                "retention policy {:?} does not permit prune confirmation",
                binding.policy
            )));
        }

        let confirmation = RetentionPruneConfirmation {
            confirmation_id: Uuid::now_v7(),
            event_family: settings_event_family::RETENTION_PRUNE_CONFIRMED.to_string(),
            binding,
            confirmed_by: confirmed_by.to_string(),
            confirmed_at_utc: Utc::now(),
        };

        self.record_event(
            settings_event_family::RETENTION_PRUNE_CONFIRMED,
            "atelier_preference_retention_policy",
            &confirmation.confirmation_id.to_string(),
            serde_json::json!({
                "confirmation_id": confirmation.confirmation_id,
                "confirmed_by": confirmation.confirmed_by,
                "confirmed_at_utc": confirmation.confirmed_at_utc,
                "scope_kind": confirmation.binding.scope.kind(),
                "character_scoped": confirmation.binding.scope.character_id().is_some(),
                "key": confirmation.binding.key,
                "preference_id": confirmation.binding.preference_id,
                "policy": confirmation.binding.policy.as_str(),
                "value_source": confirmation.binding.value_source.as_str(),
                "revision": confirmation.binding.revision,
                "prune_after_days": confirmation.binding.prune_after_days,
                "prune_confirmation_required": confirmation.binding.prune_confirmation_required,
                "automatic_prune_allowed": confirmation.binding.automatic_prune_allowed,
            }),
        )
        .await?;

        Ok(confirmation)
    }

    /// Delete a preference by scope + key. Returns whether a row was removed.
    /// Emits `PREFERENCE_DELETED` when a row is actually deleted.
    pub async fn delete_preference(
        &self,
        scope: PreferenceScope,
        key: &str,
    ) -> AtelierResult<bool> {
        let deleted_id: Option<Uuid> = sqlx::query_scalar(
            r#"DELETE FROM atelier_preference
               WHERE scope_kind = $1
                 AND character_internal_id IS NOT DISTINCT FROM $2::uuid
                 AND key = $3
               RETURNING preference_id"#,
        )
        .bind(scope.kind())
        .bind(scope.character_id())
        .bind(key)
        .fetch_optional(self.pool())
        .await?;

        match deleted_id {
            Some(id) => {
                self.record_event(
                    settings_event_family::PREFERENCE_DELETED,
                    "atelier_preference",
                    &id.to_string(),
                    serde_json::json!({
                        "preference_id": id,
                        "scope_kind": scope.kind(),
                        "character_scoped": scope.character_id().is_some(),
                        "key": key,
                    }),
                )
                .await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
