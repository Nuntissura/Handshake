//! General settings / preferences domain (MT-200).
//!
//! Translates the CastKit Codex "Global Settings" surface
//! (`src/ui/views/SettingsView.tsx`, `app/main.js` JSON config blob with
//! `libraryRoot` and runtime settings, saved via `saveConfig`) into a typed,
//! scoped preference store on PostgreSQL. CKC stored an untyped JSON file on
//! the local filesystem; Handshake forbids that (no SQLite, no localhost
//! authority). Here every preference is a typed record with an explicit value
//! type, an explicit scope (global vs per-character, mirroring the CKC split
//! "application-wide paths and runtime settings ... character-specific options
//! stay on the character"), an optional redaction flag for secrets, and
//! server-side type validation. Reads can project a redacted view so secret
//! values never leak into operator surfaces or logs.
//!
//! This is the *general* preferences domain and is intentionally distinct from
//! LLM/provider configuration, which is governed separately.
//!
//! Source: CKC `src/ui/views/SettingsView.tsx`, `app/main.js`
//! (saveConfig/getConfigInfo). Microtasks: MT-200 (settings/preferences),
//! MT-005 (event coverage).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

/// Event families emitted by the settings/preferences domain (MT-200, MT-005).
///
/// Defined here so the parent can fold these into `event_family::ALL` and the
/// MT-005 coverage check picks up settings mutations.
pub mod settings_event_family {
    pub const PREFERENCE_SET: &str = "atelier.preference.set";
    pub const PREFERENCE_DELETED: &str = "atelier.preference.deleted";

    /// All settings event families (for parity/coverage folding).
    pub const ALL: &[&str] = &[PREFERENCE_SET, PREFERENCE_DELETED];
}

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
            PreferenceType::String | PreferenceType::Path => Ok(()),
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
                .map_err(|e| {
                    AtelierError::Validation(format!("json preference invalid: {e}"))
                }),
        }
    }
}

/// Scope a preference applies to. `Global` mirrors CKC application-wide
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

/// A typed preference record as stored.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Preference {
    pub preference_id: Uuid,
    pub scope: PreferenceScope,
    pub key: String,
    pub value_type: PreferenceType,
    /// Raw stored value (text; JSON values are serialized JSON text).
    pub value: String,
    /// Whether the value is a secret. Redacted projections never expose it.
    pub redacted: bool,
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

fn preference_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<Preference> {
    let scope_kind: String = row.get("scope_kind");
    let character_id: Option<Uuid> = row.get("character_internal_id");
    let value_type_raw: String = row.get("value_type");
    Ok(Preference {
        preference_id: row.get("preference_id"),
        scope: PreferenceScope::from_parts(&scope_kind, character_id)?,
        key: row.get("key"),
        value_type: PreferenceType::from_str(&value_type_raw)?,
        value: row.get("value"),
        redacted: row.get("redacted"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

const PREF_COLUMNS: &str = "preference_id, scope_kind, character_internal_id, key, value_type, \
                            value, redacted, created_at_utc, updated_at_utc";

impl AtelierStore {
    /// Set (create or update) a typed preference. The value is validated
    /// against `value_type` server-side. Upsert key is (scope_kind,
    /// character_internal_id, key) so a given key is unique within its scope;
    /// re-setting the same key updates the value in place. Emits
    /// `PREFERENCE_SET`. Secret values are never echoed into the event payload.
    pub async fn set_preference(&self, input: &SetPreference) -> AtelierResult<Preference> {
        if input.key.trim().is_empty() {
            return Err(AtelierError::Validation("preference key must not be empty".into()));
        }
        input.value_type.validate(&input.value)?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_preference
                 (scope_kind, character_internal_id, key, value_type, value, redacted)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (scope_kind, character_internal_id, key)
               DO UPDATE SET value      = EXCLUDED.value,
                             value_type = EXCLUDED.value_type,
                             redacted   = EXCLUDED.redacted,
                             updated_at_utc = NOW()
               RETURNING {PREF_COLUMNS}"#
        ))
        .bind(input.scope.kind())
        .bind(input.scope.character_id())
        .bind(&input.key)
        .bind(input.value_type.as_str())
        .bind(&input.value)
        .bind(input.redacted)
        .fetch_one(self.pool())
        .await?;

        let preference = preference_from_row(&row)?;
        self.record_event(
            settings_event_family::PREFERENCE_SET,
            "atelier_preference",
            &preference.preference_id.to_string(),
            serde_json::json!({
                "preference_id": preference.preference_id,
                "scope_kind": preference.scope.kind(),
                "character_internal_id": preference.scope.character_id(),
                "key": preference.key,
                "value_type": preference.value_type.as_str(),
                "redacted": preference.redacted,
                // Redacted projection: never leak secret values into the ledger.
                "value": preference.redacted_value(),
            }),
        )
        .await?;
        Ok(preference)
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

    /// Resolve a preference value, falling back to a provided default when the
    /// key is unset. Mirrors CKC's "(default)" projection for unset settings
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
            out.push(if redact { pref.redacted_projection() } else { pref });
        }
        Ok(out)
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
                        "character_internal_id": scope.character_id(),
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
