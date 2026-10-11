//! Character identity (MT-006): a stable, operator-facing `public_id` separate
//! from the internal storage `internal_id`, so renames/imports/exports never
//! leak storage keys and identity survives across the data graph.
//!
//! WP-CKC-posekit-overhaul (SurrealDB port): `public_id` is whitespace-
//! normalised before storage and lookup, the character row and its
//! `CHARACTER_CREATED` event commit in one statement, and the CKC surfaces gain
//! an internal-id lookup plus a bounded, stably ordered list.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{atelier_event_sql, event_family, AtelierError, AtelierResult, AtelierStore};

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

#[derive(Clone, SurrealValue)]
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

#[derive(SurrealValue)]
struct InternalIdBinding {
    internal_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct ListLimitBinding {
    limit: i64,
}

/// The column list every character read spells out, so a field added to the
/// table later cannot silently change what this module reads.
const CHARACTER_COLUMNS: &str =
    "internal_id, public_id, display_name, created_at_utc, updated_at_utc";

/// Read one character by its operator-facing id.
const SELECT_CHARACTER_BY_PUBLIC_ID: &str = concat!(
    "SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc",
    " FROM atelier_character WHERE public_id = $public_id LIMIT 1;"
);

/// Read one character by its storage key.
const SELECT_CHARACTER_BY_INTERNAL_ID: &str = concat!(
    "SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc",
    " FROM atelier_character WHERE internal_id = $internal_id LIMIT 1;"
);

/// Stable display order for the CKC character picker.
const LIST_CHARACTERS: &str = concat!(
    "SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc",
    " FROM atelier_character ORDER BY display_name ASC, public_id ASC LIMIT $limit;"
);

/// Create the character row and its `CHARACTER_CREATED` event in one
/// statement: the former `pool.begin()` transaction of the reference branch.
const CREATE_CHARACTER_STATEMENT: &str = concat!(
    "RETURN { CREATE $domain.character_id CONTENT { \
       internal_id: $domain.internal_id, \
       public_id: $domain.public_id, \
       display_name: $domain.display_name \
     } RETURN NONE; ",
    atelier_event_sql!(),
    " RETURN (SELECT internal_id, public_id, display_name, created_at_utc, updated_at_utc \
       FROM $domain.character_id)[0]; };"
);

/// Collapse every whitespace run to one space and trim: the CKC public id is
/// a single-line operator label, and `" char\n"` and `"char"` are the same
/// character.
pub(crate) fn normalize_public_id(public_id: &str) -> String {
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
    ///
    /// `public_id` is unique. PostgreSQL surfaced a duplicate as a constraint
    /// violation from the INSERT itself; here the uniqueness index rejects the
    /// CREATE the same way, and the resulting store error is mapped to a typed
    /// [`AtelierError::Conflict`] rather than a bare database error, so callers
    /// can tell "this name is taken" apart from "the store is broken".
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
        let internal_id = Uuid::now_v7();
        let bindings = CreateCharacterBindings {
            character_id: RecordId::new("atelier_character", SurrealUuid::from(internal_id)),
            internal_id: SurrealUuid::from(internal_id),
            public_id: public_id.clone(),
            display_name: new.display_name.clone(),
        };
        let created: Option<CharacterRow> = self
            .write_with_event(
                CREATE_CHARACTER_STATEMENT,
                bindings,
                event_family::CHARACTER_CREATED,
                "atelier_character",
                &public_id,
                serde_json::json!({
                    "public_id": public_id,
                }),
            )
            .await
            .map_err(|error| {
                let text = error.to_string();
                if text.contains("uq_atelier_character_1") || text.contains("already contains") {
                    AtelierError::Conflict(format!(
                        "character public_id={public_id} already exists"
                    ))
                } else {
                    error
                }
            })?;
        let character: Character = created
            .ok_or_else(|| {
                AtelierError::Internal("creating an atelier character returned no row".to_owned())
            })?
            .into();
        Ok(character)
    }

    /// Fetch a character by its stable public id.
    pub async fn get_character_by_public_id(&self, public_id: &str) -> AtelierResult<Character> {
        let public_id = normalize_public_id(public_id);
        let bindings = PublicIdBinding {
            public_id: public_id.clone(),
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

    /// Fetch a character by its internal storage id.
    pub async fn get_character_by_internal_id(
        &self,
        internal_id: Uuid,
    ) -> AtelierResult<Character> {
        let bindings = InternalIdBinding {
            internal_id: SurrealUuid::from(internal_id),
        };
        let row: Option<CharacterRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(SELECT_CHARACTER_BY_INTERNAL_ID, bindings)
                        .await
                })
            })
            .await?;
        row.map(Character::from)
            .ok_or_else(|| AtelierError::NotFound(format!("character internal_id={internal_id}")))
    }

    /// List characters in stable display order, capped by the caller.
    pub async fn list_characters(&self, limit: i64) -> AtelierResult<Vec<Character>> {
        let bindings = ListLimitBinding {
            limit: limit.clamp(1, 500),
        };
        let rows: Vec<CharacterRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_CHARACTERS, bindings).await })
            })
            .await?;
        Ok(rows.into_iter().map(Character::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A field present on [`CharacterRow`] but absent from the select list
    /// deserializes as missing at runtime, in a query that otherwise looks
    /// correct. Catch that at compile-and-test time instead.
    #[test]
    fn select_lists_cover_every_row_field() {
        for field in [
            "internal_id",
            "public_id",
            "display_name",
            "created_at_utc",
            "updated_at_utc",
        ] {
            assert!(
                CHARACTER_COLUMNS.contains(field),
                "CharacterRow reads `{field}` but the column list does not name it"
            );
            for statement in [
                SELECT_CHARACTER_BY_PUBLIC_ID,
                SELECT_CHARACTER_BY_INTERNAL_ID,
                LIST_CHARACTERS,
                CREATE_CHARACTER_STATEMENT,
            ] {
                assert!(
                    statement.contains(field),
                    "CharacterRow reads `{field}` but a select list does not ask for it"
                );
            }
        }
    }

    #[test]
    fn public_id_normalizes_whitespace_runs_to_single_spaces() {
        assert_eq!(normalize_public_id("  a\n"), "a");
        assert_eq!(normalize_public_id("a \t b\r\nc"), "a b c");
        assert_eq!(normalize_public_id("   "), "");
    }
}
