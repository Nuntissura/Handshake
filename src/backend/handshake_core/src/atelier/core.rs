//! Character identity (MT-006): a stable, operator-facing `public_id` separate
//! from the internal storage `internal_id`, so renames/imports/exports never
//! leak storage keys and identity survives across the data graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
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

/// One `atelier_character` row as the store returns it.
#[derive(SurrealValue)]
struct CharacterRow {
    internal_id: SurrealUuid,
    public_id: String,
    display_name: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl From<CharacterRow> for Character {
    fn from(row: CharacterRow) -> Self {
        Character {
            internal_id: row.internal_id.into(),
            public_id: row.public_id,
            display_name: row.display_name,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

#[derive(SurrealValue)]
struct CreateCharacterBindings {
    character_id: RecordId,
    internal_id: SurrealUuid,
    public_id: String,
    display_name: String,
}

#[derive(SurrealValue)]
struct PublicIdBinding {
    public_id: String,
}

/// Read one character by its operator-facing id.
///
/// The select list is spelled out rather than `SELECT *` so that a field added
/// to the table later cannot silently change what this module reads.
const SELECT_CHARACTER_BY_PUBLIC_ID: &str =
    "SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc \
     FROM atelier_character WHERE public_id = $public_id LIMIT 1;";

impl AtelierStore {
    /// Create a character. The `public_id` is the stable operator-facing label;
    /// the `internal_id` is the storage key and is never the public identity.
    ///
    /// `public_id` is unique. PostgreSQL surfaced a duplicate as a constraint
    /// violation from the INSERT itself; here the uniqueness index rejects the
    /// CREATE the same way, and the resulting store error is mapped to a typed
    /// [`AtelierError::Conflict`] rather than a bare database error, so callers
    /// can tell "this name is taken" apart from "the store is broken".
    pub async fn create_character(&self, new: &NewCharacter) -> AtelierResult<Character> {
        if new.public_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "public_id must not be empty".into(),
            ));
        }
        let internal_id = Uuid::now_v7();
        let bindings = CreateCharacterBindings {
            character_id: RecordId::new("atelier_character", SurrealUuid::from(internal_id)),
            internal_id: SurrealUuid::from(internal_id),
            public_id: new.public_id.clone(),
            display_name: new.display_name.clone(),
        };
        let created: Option<CharacterRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "CREATE $character_id CONTENT { \
                           internal_id: $internal_id, \
                           public_id: $public_id, \
                           display_name: $display_name \
                         };",
                        bindings,
                    )
                    .await
                })
            })
            .await
            .map_err(|error| {
                let text = error.to_string();
                if text.contains("uq_atelier_character_1") || text.contains("already contains") {
                    AtelierError::Conflict(format!(
                        "character public_id={} already exists",
                        new.public_id
                    ))
                } else {
                    AtelierError::Database(error)
                }
            })?;
        let character: Character = created
            .ok_or_else(|| {
                AtelierError::Internal("creating an atelier character returned no row".to_owned())
            })?
            .into();
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
        let bindings = PublicIdBinding {
            public_id: public_id.to_owned(),
        };
        let row: Option<CharacterRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(SELECT_CHARACTER_BY_PUBLIC_ID, bindings)
                        .await
                })
            })
            .await?;
        row.map(Character::from)
            .ok_or_else(|| AtelierError::NotFound(format!("character public_id={public_id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A field present on [`CharacterRow`] but absent from the select list
    /// deserializes as missing at runtime, in a query that otherwise looks
    /// correct. Catch that at compile-and-test time instead.
    #[test]
    fn select_list_covers_every_row_field() {
        for field in [
            "internal_id",
            "public_id",
            "display_name",
            "created_at_utc",
            "updated_at_utc",
        ] {
            assert!(
                SELECT_CHARACTER_BY_PUBLIC_ID.contains(field),
                "CharacterRow reads `{field}` but the select list does not ask for it"
            );
        }
    }
}
