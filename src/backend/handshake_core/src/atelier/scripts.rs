//! Character image-sourcing scripts (MT-040).
//!
//! Scripts are persisted as per-character data with provenance and usage refs.
//! They are not executable authority: no runner, command, or hidden execution
//! flag is exposed, and the table constrains authority mode to data-only.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{
    event_ref_for_text, reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore,
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

fn refs_from_json(field: &str, value: serde_json::Value) -> AtelierResult<Vec<String>> {
    let values = value.as_array().ok_or_else(|| {
        AtelierError::Validation(format!("{field} must be stored as a JSON array"))
    })?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty() && value.trim() == *value)
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    AtelierError::Validation(format!(
                        "{field} must contain only non-empty trimmed string refs"
                    ))
                })
        })
        .collect()
}

fn script_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<CharacterScript> {
    let provenance_refs_json: serde_json::Value = row.get("provenance_refs_json");
    let usage_refs_json: serde_json::Value = row.get("usage_refs_json");
    let authority_mode_token: String = row.get("authority_mode");
    Ok(CharacterScript {
        script_id: row.get("script_id"),
        character_internal_id: row.get("character_internal_id"),
        name: row.get("script_name"),
        script_body_raw_text: row.get("script_body_raw_text"),
        provenance_refs: refs_from_json("provenance_refs_json", provenance_refs_json)?,
        usage_refs: refs_from_json("usage_refs_json", usage_refs_json)?,
        authority_mode: CharacterScriptAuthorityMode::from_token(&authority_mode_token)?,
        hidden_executable_authority: row.get("hidden_executable_authority"),
        created_by: row.get("created_by"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

impl AtelierStore {
    async fn require_character_internal_id(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM atelier_character WHERE internal_id = $1)",
        )
        .bind(character_internal_id)
        .fetch_one(self.pool())
        .await?;
        if !exists {
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
        let provenance_refs_json = serde_json::Value::from(provenance_refs.clone());
        let usage_refs_json = serde_json::Value::from(usage_refs.clone());
        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_character_script
                 (character_internal_id, script_name, script_body_raw_text,
                  provenance_refs_json, usage_refs_json, authority_mode,
                  hidden_executable_authority, created_by)
               VALUES ($1, $2, $3, $4, $5, 'data_only', FALSE, $6)
               RETURNING script_id, character_internal_id, script_name, script_body_raw_text,
                         provenance_refs_json, usage_refs_json, authority_mode,
                         hidden_executable_authority, created_by, created_at_utc,
                         updated_at_utc"#,
        )
        .bind(new.character_internal_id)
        .bind(&name)
        .bind(&new.script_body_raw_text)
        .bind(&provenance_refs_json)
        .bind(&usage_refs_json)
        .bind(&created_by)
        .fetch_one(&mut *tx)
        .await?;
        let script = script_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            scripts_event_family::CHARACTER_SCRIPT_CREATED,
            "atelier_character_script",
            &script.script_id.to_string(),
            serde_json::json!({
                "script_id": script.script_id,
                "character_internal_id": script.character_internal_id,
                "script_name_ref": event_ref_for_text(&script.name),
                "script_body_ref": event_ref_for_text(&script.script_body_raw_text),
                "provenance_ref_count": script.provenance_refs.len(),
                "usage_ref_count": script.usage_refs.len(),
                "authority_mode": script.authority_mode.as_token(),
                "hidden_executable_authority": script.hidden_executable_authority,
                "created_by_ref": event_ref_for_text(&script.created_by),
            }),
        )
        .await?;

        tx.commit().await?;
        Ok(script)
    }

    pub async fn get_character_script(&self, script_id: Uuid) -> AtelierResult<CharacterScript> {
        let row = sqlx::query(
            r#"SELECT script_id, character_internal_id, script_name, script_body_raw_text,
                      provenance_refs_json, usage_refs_json, authority_mode,
                      hidden_executable_authority, created_by, created_at_utc,
                      updated_at_utc
               FROM atelier_character_script
               WHERE script_id = $1"#,
        )
        .bind(script_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character script {script_id}")))?;
        script_from_row(&row)
    }

    pub async fn list_character_scripts(
        &self,
        character_internal_id: Uuid,
    ) -> AtelierResult<Vec<CharacterScript>> {
        let rows = sqlx::query(
            r#"SELECT script_id, character_internal_id, script_name, script_body_raw_text,
                      provenance_refs_json, usage_refs_json, authority_mode,
                      hidden_executable_authority, created_by, created_at_utc,
                      updated_at_utc
               FROM atelier_character_script
               WHERE character_internal_id = $1
               ORDER BY updated_at_utc DESC, script_id ASC"#,
        )
        .bind(character_internal_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(script_from_row).collect()
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
        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(
            r#"SELECT script_id, character_internal_id, script_name, script_body_raw_text,
                      provenance_refs_json, usage_refs_json, authority_mode,
                      hidden_executable_authority, created_by, created_at_utc,
                      updated_at_utc
               FROM atelier_character_script
               WHERE script_id = $1
               FOR UPDATE"#,
        )
        .bind(script_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character script {script_id}")))?;
        let mut script = script_from_row(&row)?;
        if script
            .usage_refs
            .iter()
            .any(|existing| existing == &usage_ref)
        {
            tx.commit().await?;
            return Ok(script);
        }

        script.usage_refs.push(usage_ref.clone());
        let usage_refs_json = serde_json::Value::from(script.usage_refs.clone());
        let row = sqlx::query(
            r#"UPDATE atelier_character_script
               SET usage_refs_json = $2,
                   updated_at_utc = NOW()
               WHERE script_id = $1
               RETURNING script_id, character_internal_id, script_name, script_body_raw_text,
                         provenance_refs_json, usage_refs_json, authority_mode,
                         hidden_executable_authority, created_by, created_at_utc,
                         updated_at_utc"#,
        )
        .bind(script_id)
        .bind(&usage_refs_json)
        .fetch_one(&mut *tx)
        .await?;
        let updated = script_from_row(&row)?;

        self.record_event_in_tx(
            &mut tx,
            scripts_event_family::CHARACTER_SCRIPT_USAGE_RECORDED,
            "atelier_character_script",
            &updated.script_id.to_string(),
            serde_json::json!({
                "script_id": updated.script_id,
                "character_internal_id": updated.character_internal_id,
                "usage_ref_ref": event_ref_for_text(&usage_ref),
                "usage_ref_count": updated.usage_refs.len(),
                "recorded_by_ref": event_ref_for_text(&recorded_by),
                "authority_mode": updated.authority_mode.as_token(),
            }),
        )
        .await?;

        tx.commit().await?;
        Ok(updated)
    }
}
