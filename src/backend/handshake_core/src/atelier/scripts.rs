//! Character image-sourcing scripts (MT-040).
//!
//! Scripts are persisted as per-character data with provenance and usage refs.
//! They are not executable authority: no runner, command, or hidden execution
//! flag is exposed, and the schema constrains authority mode to data-only.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{
    atelier_event_sql, event_ref_for_text, reject_legacy_runtime_ref, AtelierError, AtelierResult,
    AtelierStore,
};

pub mod scripts_event_family {
    pub const CHARACTER_SCRIPT_CREATED: &str = "atelier.character_script.created";
    pub const CHARACTER_SCRIPT_USAGE_RECORDED: &str = "atelier.character_script.usage_recorded";

    pub const ALL: &[&str] = &[CHARACTER_SCRIPT_CREATED, CHARACTER_SCRIPT_USAGE_RECORDED];
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterScriptAuthorityMode {
    DataOnly,
}

impl CharacterScriptAuthorityMode {
    pub fn as_token(self) -> &'static str {
        match self {
            CharacterScriptAuthorityMode::DataOnly => "data_only",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "data_only" => Ok(CharacterScriptAuthorityMode::DataOnly),
            other => Err(AtelierError::Validation(format!(
                "unknown character script authority mode: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewCharacterScript {
    pub character_internal_id: Uuid,
    pub name: String,
    pub script_body_raw_text: String,
    pub provenance_refs: Vec<String>,
    pub usage_refs: Vec<String>,
    pub created_by: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterScript {
    pub script_id: Uuid,
    pub character_internal_id: Uuid,
    pub name: String,
    pub script_body_raw_text: String,
    pub provenance_refs: Vec<String>,
    pub usage_refs: Vec<String>,
    pub authority_mode: CharacterScriptAuthorityMode,
    pub hidden_executable_authority: bool,
    pub created_by: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

fn require_non_empty_trimmed(field: &str, value: &str) -> AtelierResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn clean_refs(
    field: &str,
    values: &[String],
    require_non_empty: bool,
) -> AtelierResult<Vec<String>> {
    if require_non_empty && values.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must include at least one ref"
        )));
    }
    let mut cleaned = Vec::new();
    for value in values {
        let value = require_non_empty_trimmed(field, value)?;
        reject_legacy_runtime_ref(field, &value)?;
        if !cleaned.iter().any(|existing| existing == &value) {
            cleaned.push(value);
        }
    }
    if require_non_empty && cleaned.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must include at least one ref"
        )));
    }
    Ok(cleaned)
}

/// The character-script select list. The character link is projected back to
/// its uuid key so the row shape matches the public struct.
const CHARACTER_SCRIPT_SELECT: &str =
    "SELECT script_id, record::id(character_internal_id) AS character_internal_id, \
            script_name, script_body_raw_text, provenance_refs_json, usage_refs_json, \
            authority_mode, hidden_executable_authority, created_by, created_at_utc, \
            updated_at_utc";

/// One `atelier_character_script` row as the store returns it.
#[derive(SurrealValue)]
struct CharacterScriptRow {
    script_id: SurrealUuid,
    character_internal_id: SurrealUuid,
    script_name: String,
    script_body_raw_text: String,
    provenance_refs_json: Vec<String>,
    usage_refs_json: Vec<String>,
    authority_mode: String,
    hidden_executable_authority: bool,
    created_by: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl TryFrom<CharacterScriptRow> for CharacterScript {
    type Error = AtelierError;

    fn try_from(row: CharacterScriptRow) -> AtelierResult<Self> {
        Ok(CharacterScript {
            script_id: row.script_id.into(),
            character_internal_id: row.character_internal_id.into(),
            name: row.script_name,
            script_body_raw_text: row.script_body_raw_text,
            provenance_refs: row.provenance_refs_json,
            usage_refs: row.usage_refs_json,
            authority_mode: CharacterScriptAuthorityMode::from_token(&row.authority_mode)?,
            hidden_executable_authority: row.hidden_executable_authority,
            created_by: row.created_by,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

#[derive(Clone, SurrealValue)]
struct CreateCharacterScriptBindings {
    record_id: RecordId,
    script_id: SurrealUuid,
    character_ref: RecordId,
    script_name: String,
    script_body_raw_text: String,
    provenance_refs_json: Vec<String>,
    usage_refs_json: Vec<String>,
    created_by: String,
}

#[derive(SurrealValue)]
struct ScriptIdBinding {
    script_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct CharacterRefBinding {
    character_ref: RecordId,
}

#[derive(SurrealValue)]
struct CharacterExistsBinding {
    internal_id: SurrealUuid,
}

#[derive(Clone, SurrealValue)]
struct RecordScriptUsageBindings {
    script_id: SurrealUuid,
    usage_ref: String,
}

const CREATE_CHARACTER_SCRIPT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         script_id: $domain.script_id, \
         character_internal_id: $domain.character_ref, \
         script_name: $domain.script_name, \
         script_body_raw_text: $domain.script_body_raw_text, \
         provenance_refs_json: $domain.provenance_refs_json, \
         usage_refs_json: $domain.usage_refs_json, \
         authority_mode: 'data_only', \
         hidden_executable_authority: false, \
         created_by: $domain.created_by \
       }; ",
    "RETURN (",
    "SELECT script_id, record::id(character_internal_id) AS character_internal_id, \
            script_name, script_body_raw_text, provenance_refs_json, usage_refs_json, \
            authority_mode, hidden_executable_authority, created_by, created_at_utc, \
            updated_at_utc",
    " FROM ONLY $rid); };"
);

const GET_CHARACTER_SCRIPT_STATEMENT: &str = concat!(
    "SELECT script_id, record::id(character_internal_id) AS character_internal_id, \
            script_name, script_body_raw_text, provenance_refs_json, usage_refs_json, \
            authority_mode, hidden_executable_authority, created_by, created_at_utc, \
            updated_at_utc",
    " FROM atelier_character_script WHERE script_id = $script_id LIMIT 1;"
);

const LIST_CHARACTER_SCRIPTS_STATEMENT: &str = concat!(
    "SELECT script_id, record::id(character_internal_id) AS character_internal_id, \
            script_name, script_body_raw_text, provenance_refs_json, usage_refs_json, \
            authority_mode, hidden_executable_authority, created_by, created_at_utc, \
            updated_at_utc",
    " FROM atelier_character_script WHERE character_internal_id = $character_ref \
     ORDER BY updated_at_utc DESC, script_id ASC;"
);

/// Append one usage ref (set-union, so a duplicate add is a no-op) and record
/// the usage event in the same atomic statement.
const RECORD_SCRIPT_USAGE_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_character_script', $domain.script_id); ",
    atelier_event_sql!(),
    " UPDATE $rid SET \
         usage_refs_json = array::union(usage_refs_json, [$domain.usage_ref]), \
         updated_at_utc = time::now(); ",
    "RETURN (",
    "SELECT script_id, record::id(character_internal_id) AS character_internal_id, \
            script_name, script_body_raw_text, provenance_refs_json, usage_refs_json, \
            authority_mode, hidden_executable_authority, created_by, created_at_utc, \
            updated_at_utc",
    " FROM $rid); };"
);

impl AtelierStore {
    async fn require_character_internal_id(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<()> {
        let bindings = CharacterExistsBinding {
            internal_id: SurrealUuid::from(character_internal_id),
        };
        let exists: Option<bool> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "RETURN record::exists(type::record('atelier_character', $internal_id));",
                        bindings,
                    )
                    .await
                })
            })
            .await?;
        if !exists.unwrap_or(false) {
            return Err(AtelierError::NotFound(format!(
                "character internal_id={character_internal_id}"
            )));
        }
        Ok(())
    }

    pub async fn create_character_script(
        &self,
        new: &NewCharacterScript,
    ) -> AtelierResult<CharacterScript> {
        self.require_character_internal_id(new.character_internal_id)
            .await?;
        let name = require_non_empty_trimmed("name", &new.name)?;
        let created_by = require_non_empty_trimmed("created_by", &new.created_by)?;
        require_non_empty_trimmed("script_body_raw_text", &new.script_body_raw_text)?;
        let provenance_refs = clean_refs("provenance_refs", &new.provenance_refs, true)?;
        let usage_refs = clean_refs("usage_refs", &new.usage_refs, false)?;
        let script_id = Uuid::now_v7();

        let bindings = CreateCharacterScriptBindings {
            record_id: RecordId::new("atelier_character_script", SurrealUuid::from(script_id)),
            script_id: SurrealUuid::from(script_id),
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.character_internal_id),
            ),
            script_name: name.clone(),
            script_body_raw_text: new.script_body_raw_text.clone(),
            provenance_refs_json: provenance_refs.clone(),
            usage_refs_json: usage_refs.clone(),
            created_by: created_by.clone(),
        };
        let row: Option<CharacterScriptRow> = self
            .write_with_event(
                CREATE_CHARACTER_SCRIPT_STATEMENT,
                bindings,
                scripts_event_family::CHARACTER_SCRIPT_CREATED,
                "atelier_character_script",
                &script_id.to_string(),
                serde_json::json!({
                    "script_id": script_id,
                    "character_internal_id": new.character_internal_id,
                    "script_name_ref": event_ref_for_text(&name),
                    "script_body_ref": event_ref_for_text(&new.script_body_raw_text),
                    "provenance_ref_count": provenance_refs.len(),
                    "usage_ref_count": usage_refs.len(),
                    "authority_mode": CharacterScriptAuthorityMode::DataOnly.as_token(),
                    "hidden_executable_authority": false,
                    "created_by_ref": event_ref_for_text(&created_by),
                }),
            )
            .await?;
        row.ok_or_else(|| {
            AtelierError::Internal("creating a character script returned no row".to_owned())
        })?
        .try_into()
    }

    pub async fn get_character_script(&self, script_id: Uuid) -> AtelierResult<CharacterScript> {
        let bindings = ScriptIdBinding {
            script_id: SurrealUuid::from(script_id),
        };
        let row: Option<CharacterScriptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(GET_CHARACTER_SCRIPT_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        row.ok_or_else(|| AtelierError::NotFound(format!("character script {script_id}")))?
            .try_into()
    }

    pub async fn list_character_scripts(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<CharacterScript>> {
        let bindings = CharacterRefBinding {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
        };
        let rows: Vec<CharacterScriptRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_CHARACTER_SCRIPTS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter().map(CharacterScript::try_from).collect()
    }

    pub async fn record_character_script_usage(
        &self,
        script_id: Uuid,
        usage_ref: &str,
        recorded_by: &str,
    ) -> AtelierResult<CharacterScript> {
        let usage_ref = clean_refs("usage_ref", &[usage_ref.to_string()], true)?
            .pop()
            .ok_or_else(|| AtelierError::Validation("usage_ref must not be empty".into()))?;
        let recorded_by = require_non_empty_trimmed("recorded_by", recorded_by)?;

        // Pre-read outside the write so an already-recorded usage returns
        // without appending a duplicate event, matching the former
        // read-then-decide contract.
        let current = self.get_character_script(script_id).await?;
        if current
            .usage_refs
            .iter()
            .any(|existing| existing == &usage_ref)
        {
            return Ok(current);
        }

        let bindings = RecordScriptUsageBindings {
            script_id: SurrealUuid::from(script_id),
            usage_ref: usage_ref.clone(),
        };
        let row: Option<CharacterScriptRow> = self
            .write_with_event(
                RECORD_SCRIPT_USAGE_STATEMENT,
                bindings,
                scripts_event_family::CHARACTER_SCRIPT_USAGE_RECORDED,
                "atelier_character_script",
                &script_id.to_string(),
                serde_json::json!({
                    "script_id": script_id,
                    "character_internal_id": current.character_internal_id,
                    "usage_ref_ref": event_ref_for_text(&usage_ref),
                    "usage_ref_count": current.usage_refs.len() + 1,
                    "recorded_by_ref": event_ref_for_text(&recorded_by),
                    "authority_mode": current.authority_mode.as_token(),
                }),
            )
            .await?;
        row.ok_or_else(|| AtelierError::NotFound(format!("character script {script_id}")))?
            .try_into()
    }
}
