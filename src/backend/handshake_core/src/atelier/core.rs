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

fn normalize_public_id(public_id: &str) -> String {
    public_id
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

impl AtelierStore {
    /// Create a character. The `public_id` is the stable operator-facing label;
    /// the `internal_id` is the storage key and is never the public identity.
    pub async fn create_character(&self, new: &NewCharacter) -> AtelierResult<Character> {
        let public_id = normalize_public_id(&new.public_id);
        if public_id.is_empty() {
            return Err(AtelierError::Validation(
                "public_id must not be empty".into(),
            ));
        }
        if public_id.contains(['\r', '\n', '\t']) {
            return Err(AtelierError::Validation(
                "public_id must normalize to a single line".into(),
            ));
        }
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_character (public_id, display_name)
               VALUES ($1, $2)
               RETURNING internal_id, public_id, display_name, created_at_utc, updated_at_utc"#,
        )
        .bind(&public_id)
        .bind(&new.display_name)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505") => {
                AtelierError::Conflict(format!("character public_id={} already exists", public_id))
            }
            other => AtelierError::Database(other),
        })?;
        let character = character_from_row(&row);
        if let Err(err) = self
            .record_event_in_tx(
                &mut tx,
                event_family::CHARACTER_CREATED,
                "atelier_character",
                &character.public_id,
                serde_json::json!({
                    "public_id": character.public_id,
                }),
            )
            .await
        {
            tx.rollback().await?;
            return Err(err);
        }
        tx.commit().await?;
        Ok(character)
    }

    /// Fetch a character by its stable public id.
    pub async fn get_character_by_public_id(&self, public_id: &str) -> AtelierResult<Character> {
        let public_id = normalize_public_id(public_id);
        let row = sqlx::query(
            r#"SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc
               FROM atelier_character WHERE public_id = $1"#,
        )
        .bind(&public_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character public_id={public_id}")))?;
        Ok(character_from_row(&row))
    }

    /// Fetch a character by its internal storage id.
    pub async fn get_character_by_internal_id(
        &self,
        internal_id: Uuid,
    ) -> AtelierResult<Character> {
        let row = sqlx::query(
            r#"SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc
               FROM atelier_character WHERE internal_id = $1"#,
        )
        .bind(internal_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character internal_id={internal_id}")))?;
        Ok(character_from_row(&row))
    }

    /// List characters in stable display order, capped by the caller.
    pub async fn list_characters(&self, limit: i64) -> AtelierResult<Vec<Character>> {
        let limit = limit.clamp(1, 500);
        let rows = sqlx::query(
            r#"SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc
               FROM atelier_character
               ORDER BY display_name ASC, public_id ASC
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(character_from_row).collect())
    }
}
