//! Character identity (MT-006): a stable, operator-facing `public_id` separate
//! from the internal storage `internal_id`, so renames/imports/exports never
//! leak storage keys and identity survives across the data graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_family, AtelierError, AtelierResult, AtelierStore};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Character {
    pub internal_id: Uuid,
    pub public_id: String,
    pub display_name: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewCharacter {
    pub public_id: String,
    pub display_name: String,
}

fn character_from_row(row: &sqlx::postgres::PgRow) -> Character {
    Character {
        internal_id: row.get("internal_id"),
        public_id: row.get("public_id"),
        display_name: row.get("display_name"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

impl AtelierStore {
    /// Create a character. The `public_id` is the stable operator-facing label;
    /// the `internal_id` is the storage key and is never the public identity.
    pub async fn create_character(&self, new: &NewCharacter) -> AtelierResult<Character> {
        if new.public_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "public_id must not be empty".into(),
            ));
        }
        let row = sqlx::query(
            r#"INSERT INTO atelier_character (public_id, display_name)
               VALUES ($1, $2)
               RETURNING internal_id, public_id, display_name, created_at_utc, updated_at_utc"#,
        )
        .bind(&new.public_id)
        .bind(&new.display_name)
        .fetch_one(self.pool())
        .await?;
        let character = character_from_row(&row);
        self.record_event(
            event_family::CHARACTER_CREATED,
            "atelier_character",
            &character.public_id,
            serde_json::json!({
                "public_id": character.public_id,
            }),
        )
        .await?;
        Ok(character)
    }

    /// Fetch a character by its stable public id.
    pub async fn get_character_by_public_id(&self, public_id: &str) -> AtelierResult<Character> {
        let row = sqlx::query(
            r#"SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc
               FROM atelier_character WHERE public_id = $1"#,
        )
        .bind(public_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character public_id={public_id}")))?;
        Ok(character_from_row(&row))
    }
}
