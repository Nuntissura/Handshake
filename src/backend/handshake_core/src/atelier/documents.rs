//! Character-scoped note/story/moodboard documents (MT-038).
//!
//! Documents have stable ids and typed metadata; raw text lives in append-only
//! versions so note/story/moodboard edits preserve prior text exactly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_ref_for_text, AtelierError, AtelierResult, AtelierStore};

pub mod documents_event_family {
    pub const CHARACTER_DOCUMENT_CREATED: &str = "atelier.character_document.created";
    pub const CHARACTER_DOCUMENT_VERSION_APPENDED: &str =
        "atelier.character_document.version_appended";
    pub const STORY_CARD_ADDED: &str = "atelier.story.card_added";
    pub const STORY_BEAT_ADDED: &str = "atelier.story.beat_added";

    pub const ALL: &[&str] = &[
        CHARACTER_DOCUMENT_CREATED,
        CHARACTER_DOCUMENT_VERSION_APPENDED,
        STORY_CARD_ADDED,
        STORY_BEAT_ADDED,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterDocumentType {
    Note,
    Story,
    Moodboard,
}

impl CharacterDocumentType {
    pub fn as_token(self) -> &'static str {
        match self {
            CharacterDocumentType::Note => "note",
            CharacterDocumentType::Story => "story",
            CharacterDocumentType::Moodboard => "moodboard",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "note" => Ok(CharacterDocumentType::Note),
            "story" => Ok(CharacterDocumentType::Story),
            "moodboard" => Ok(CharacterDocumentType::Moodboard),
            other => Err(AtelierError::Validation(format!(
                "unknown character document type token: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NewCharacterDocument {
    pub character_internal_id: Uuid,
    pub doc_type: CharacterDocumentType,
    pub title: String,
    pub body_raw_text: String,
    pub tags: Vec<String>,
    pub author: String,
}

#[derive(Clone, Debug)]
pub struct AppendCharacterDocumentVersion {
    pub title: String,
    pub body_raw_text: String,
    pub tags: Vec<String>,
    pub author: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterDocument {
    pub document_id: Uuid,
    pub character_internal_id: Uuid,
    pub doc_type: CharacterDocumentType,
    pub title: String,
    pub tags: Vec<String>,
    pub current_version_id: Uuid,
    pub current_version_seq: i64,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterDocumentVersion {
    pub version_id: Uuid,
    pub document_id: Uuid,
    pub version_seq: i64,
    pub title: String,
    pub body_raw_text: String,
    pub tags: Vec<String>,
    pub author: String,
    pub parent_version_id: Option<Uuid>,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewStoryCard {
    pub story_document_id: Uuid,
    pub title: String,
    pub body_raw_text: String,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryCard {
    pub card_id: Uuid,
    pub story_document_id: Uuid,
    pub seq: i64,
    pub title: String,
    pub body_raw_text: String,
    pub tags: Vec<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewStoryBeat {
    pub story_document_id: Uuid,
    pub card_id: Option<Uuid>,
    pub beat_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoryBeat {
    pub beat_id: Uuid,
    pub story_document_id: Uuid,
    pub card_id: Option<Uuid>,
    pub seq: i64,
    pub beat_text: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

fn clean_document_tags(tags: &[String]) -> Vec<String> {
    let mut seen = Vec::new();
    for tag in tags {
        let trimmed = tag.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if !seen.iter().any(|existing| existing == &trimmed) {
            seen.push(trimmed);
        }
    }
    seen
}

fn tags_from_json(value: serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
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

fn document_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<CharacterDocument> {
    let doc_type_token: String = row.get("doc_type");
    let tags_json: serde_json::Value = row.get("tags_json");
    let current_version_id: Option<Uuid> = row.get("current_version_id");
    Ok(CharacterDocument {
        document_id: row.get("document_id"),
        character_internal_id: row.get("character_internal_id"),
        doc_type: CharacterDocumentType::from_token(&doc_type_token)?,
        title: row.get("title"),
        tags: tags_from_json(tags_json),
        current_version_id: current_version_id.ok_or_else(|| {
            AtelierError::Validation(format!(
                "character document {} has no current version",
                row.get::<Uuid, _>("document_id")
            ))
        })?,
        current_version_seq: row.get("current_version_seq"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn version_from_row(row: &sqlx::postgres::PgRow) -> CharacterDocumentVersion {
    let tags_json: serde_json::Value = row.get("tags_json");
    CharacterDocumentVersion {
        version_id: row.get("version_id"),
        document_id: row.get("document_id"),
        version_seq: row.get("version_seq"),
        title: row.get("title"),
        body_raw_text: row.get("body_raw_text"),
        tags: tags_from_json(tags_json),
        author: row.get("author"),
        parent_version_id: row.get("parent_version_id"),
        created_at_utc: row.get("created_at_utc"),
    }
}

fn story_card_from_row(row: &sqlx::postgres::PgRow) -> StoryCard {
    let tags_json: serde_json::Value = row.get("tags_json");
    StoryCard {
        card_id: row.get("card_id"),
        story_document_id: row.get("story_document_id"),
        seq: row.get("seq"),
        title: row.get("title"),
        body_raw_text: row.get("body_raw_text"),
        tags: tags_from_json(tags_json),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn story_beat_from_row(row: &sqlx::postgres::PgRow) -> StoryBeat {
    StoryBeat {
        beat_id: row.get("beat_id"),
        story_document_id: row.get("story_document_id"),
        card_id: row.get("card_id"),
        seq: row.get("seq"),
        beat_text: row.get("beat_text"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

impl AtelierStore {
    async fn require_story_document(&self, story_document_id: Uuid) -> AtelierResult<()> {
        let document = self.get_character_document(story_document_id).await?;
        if document.doc_type != CharacterDocumentType::Story {
            return Err(AtelierError::Validation(format!(
                "document {story_document_id} must be a story document"
            )));
        }
        Ok(())
    }

    pub async fn create_character_document(
        &self,
        new: &NewCharacterDocument,
    ) -> AtelierResult<CharacterDocumentVersion> {
        let title = require_non_empty_trimmed("title", &new.title)?;
        let author = require_non_empty_trimmed("author", &new.author)?;
        let tags = clean_document_tags(&new.tags);
        let tags_json = serde_json::Value::from(tags.clone());
        let mut tx = self.pool().begin().await?;

        let doc_row = sqlx::query(
            r#"INSERT INTO atelier_character_document
                 (character_internal_id, doc_type, title, tags_json)
               VALUES ($1, $2, $3, $4)
               RETURNING document_id"#,
        )
        .bind(new.character_internal_id)
        .bind(new.doc_type.as_token())
        .bind(&title)
        .bind(&tags_json)
        .fetch_one(&mut *tx)
        .await?;
        let document_id: Uuid = doc_row.get("document_id");

        let version_row = sqlx::query(
            r#"INSERT INTO atelier_character_document_version
                 (document_id, version_seq, title, body_raw_text, tags_json, author,
                  parent_version_id)
               VALUES ($1, 1, $2, $3, $4, $5, NULL)
               RETURNING version_id, document_id, version_seq, title, body_raw_text,
                         tags_json, author, parent_version_id, created_at_utc"#,
        )
        .bind(document_id)
        .bind(&title)
        .bind(&new.body_raw_text)
        .bind(&tags_json)
        .bind(&author)
        .fetch_one(&mut *tx)
        .await?;
        let version = version_from_row(&version_row);

        sqlx::query(
            r#"UPDATE atelier_character_document
               SET current_version_id = $2,
                   current_version_seq = 1,
                   updated_at_utc = NOW()
               WHERE document_id = $1"#,
        )
        .bind(document_id)
        .bind(version.version_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.record_event(
            documents_event_family::CHARACTER_DOCUMENT_CREATED,
            "atelier_character_document",
            &document_id.to_string(),
            serde_json::json!({
                "character_internal_id": new.character_internal_id,
                "doc_type": new.doc_type.as_token(),
                "version_id": version.version_id,
                "version_seq": version.version_seq,
                "title": title,
                "tag_count": tags.len(),
                "body_raw_text_ref": event_ref_for_text(&version.body_raw_text),
            }),
        )
        .await?;
        Ok(version)
    }

    pub async fn get_character_document(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<CharacterDocument> {
        let row = sqlx::query(
            r#"SELECT document_id, character_internal_id, doc_type, title, tags_json,
                      current_version_id, current_version_seq, created_at_utc,
                      updated_at_utc
               FROM atelier_character_document
               WHERE document_id = $1"#,
        )
        .bind(document_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?;
        document_from_row(&row)
    }

    pub async fn list_character_documents(
        &self,
        character_internal_id: Uuid,
        doc_type: Option<CharacterDocumentType>,
    ) -> AtelierResult<Vec<CharacterDocument>> {
        let rows = sqlx::query(
            r#"SELECT document_id, character_internal_id, doc_type, title, tags_json,
                      current_version_id, current_version_seq, created_at_utc,
                      updated_at_utc
               FROM atelier_character_document
               WHERE character_internal_id = $1
                 AND ($2::text IS NULL OR doc_type = $2)
               ORDER BY updated_at_utc DESC, document_id ASC"#,
        )
        .bind(character_internal_id)
        .bind(doc_type.map(CharacterDocumentType::as_token))
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(document_from_row).collect()
    }

    pub async fn append_character_document_version(
        &self,
        document_id: Uuid,
        update: &AppendCharacterDocumentVersion,
    ) -> AtelierResult<CharacterDocumentVersion> {
        let title = require_non_empty_trimmed("title", &update.title)?;
        let author = require_non_empty_trimmed("author", &update.author)?;
        let tags = clean_document_tags(&update.tags);
        let tags_json = serde_json::Value::from(tags.clone());
        let mut tx = self.pool().begin().await?;

        let doc_row = sqlx::query(
            r#"SELECT current_version_id, current_version_seq
               FROM atelier_character_document
               WHERE document_id = $1
               FOR UPDATE"#,
        )
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?;
        let parent_version_id: Option<Uuid> = doc_row.get("current_version_id");
        let current_version_seq: i64 = doc_row.get("current_version_seq");
        let next_version_seq = current_version_seq + 1;

        let version_row = sqlx::query(
            r#"INSERT INTO atelier_character_document_version
                 (document_id, version_seq, title, body_raw_text, tags_json, author,
                  parent_version_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING version_id, document_id, version_seq, title, body_raw_text,
                         tags_json, author, parent_version_id, created_at_utc"#,
        )
        .bind(document_id)
        .bind(next_version_seq)
        .bind(&title)
        .bind(&update.body_raw_text)
        .bind(&tags_json)
        .bind(&author)
        .bind(parent_version_id)
        .fetch_one(&mut *tx)
        .await?;
        let version = version_from_row(&version_row);

        sqlx::query(
            r#"UPDATE atelier_character_document
               SET title = $2,
                   tags_json = $3,
                   current_version_id = $4,
                   current_version_seq = $5,
                   updated_at_utc = NOW()
               WHERE document_id = $1"#,
        )
        .bind(document_id)
        .bind(&title)
        .bind(&tags_json)
        .bind(version.version_id)
        .bind(version.version_seq)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.record_event(
            documents_event_family::CHARACTER_DOCUMENT_VERSION_APPENDED,
            "atelier_character_document",
            &document_id.to_string(),
            serde_json::json!({
                "version_id": version.version_id,
                "version_seq": version.version_seq,
                "parent_version_id": version.parent_version_id,
                "title": title,
                "tag_count": tags.len(),
                "body_raw_text_ref": event_ref_for_text(&version.body_raw_text),
            }),
        )
        .await?;
        Ok(version)
    }

    pub async fn latest_character_document_version(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<Option<CharacterDocumentVersion>> {
        let row = sqlx::query(
            r#"SELECT version_id, document_id, version_seq, title, body_raw_text,
                      tags_json, author, parent_version_id, created_at_utc
               FROM atelier_character_document_version
               WHERE document_id = $1
               ORDER BY version_seq DESC
               LIMIT 1"#,
        )
        .bind(document_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(version_from_row))
    }

    pub async fn character_document_history(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<Vec<CharacterDocumentVersion>> {
        let rows = sqlx::query(
            r#"SELECT version_id, document_id, version_seq, title, body_raw_text,
                      tags_json, author, parent_version_id, created_at_utc
               FROM atelier_character_document_version
               WHERE document_id = $1
               ORDER BY version_seq ASC"#,
        )
        .bind(document_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(version_from_row).collect())
    }

    pub async fn add_story_card(&self, new: &NewStoryCard) -> AtelierResult<StoryCard> {
        self.require_story_document(new.story_document_id).await?;
        let title = require_non_empty_trimmed("title", &new.title)?;
        let tags = clean_document_tags(&new.tags);
        let tags_json = serde_json::Value::from(tags.clone());
        let mut tx = self.pool().begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock(39039, hashtext($1))")
            .bind(new.story_document_id.to_string())
            .execute(&mut *tx)
            .await?;

        let seq: i64 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(seq), 0) + 1
               FROM atelier_story_card
               WHERE story_document_id = $1"#,
        )
        .bind(new.story_document_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_story_card
                 (story_document_id, seq, title, body_raw_text, tags_json)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING card_id, story_document_id, seq, title, body_raw_text,
                         tags_json, created_at_utc, updated_at_utc"#,
        )
        .bind(new.story_document_id)
        .bind(seq)
        .bind(&title)
        .bind(&new.body_raw_text)
        .bind(&tags_json)
        .fetch_one(&mut *tx)
        .await?;
        let card = story_card_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            documents_event_family::STORY_CARD_ADDED,
            "atelier_character_document",
            &new.story_document_id.to_string(),
            serde_json::json!({
                "card_id": card.card_id,
                "story_document_id": card.story_document_id,
                "seq": card.seq,
                "title": title,
                "tag_count": tags.len(),
                "body_raw_text_ref": event_ref_for_text(&card.body_raw_text),
            }),
        )
        .await?;

        tx.commit().await?;
        Ok(card)
    }

    pub async fn list_story_cards(&self, story_document_id: Uuid) -> AtelierResult<Vec<StoryCard>> {
        self.require_story_document(story_document_id).await?;
        let rows = sqlx::query(
            r#"SELECT card_id, story_document_id, seq, title, body_raw_text,
                      tags_json, created_at_utc, updated_at_utc
               FROM atelier_story_card
               WHERE story_document_id = $1
               ORDER BY seq ASC, card_id ASC"#,
        )
        .bind(story_document_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(story_card_from_row).collect())
    }

    pub async fn add_story_beat(&self, new: &NewStoryBeat) -> AtelierResult<StoryBeat> {
        self.require_story_document(new.story_document_id).await?;
        require_non_empty_trimmed("beat_text", &new.beat_text)?;

        let mut tx = self.pool().begin().await?;

        sqlx::query("SELECT pg_advisory_xact_lock(39040, hashtext($1))")
            .bind(new.story_document_id.to_string())
            .execute(&mut *tx)
            .await?;

        if let Some(card_id) = new.card_id {
            let card_story_document_id: Uuid = sqlx::query_scalar(
                r#"SELECT story_document_id
                   FROM atelier_story_card
                   WHERE card_id = $1"#,
            )
            .bind(card_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AtelierError::NotFound(format!("story card {card_id}")))?;
            if card_story_document_id != new.story_document_id {
                return Err(AtelierError::Validation(format!(
                    "story card {card_id} does not belong to story document {}",
                    new.story_document_id
                )));
            }
        }

        let seq: i64 = sqlx::query_scalar(
            r#"SELECT COALESCE(MAX(seq), 0) + 1
               FROM atelier_story_beat
               WHERE story_document_id = $1"#,
        )
        .bind(new.story_document_id)
        .fetch_one(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_story_beat
                 (story_document_id, card_id, seq, beat_text)
               VALUES ($1, $2, $3, $4)
               RETURNING beat_id, story_document_id, card_id, seq, beat_text,
                         created_at_utc, updated_at_utc"#,
        )
        .bind(new.story_document_id)
        .bind(new.card_id)
        .bind(seq)
        .bind(&new.beat_text)
        .fetch_one(&mut *tx)
        .await?;
        let beat = story_beat_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            documents_event_family::STORY_BEAT_ADDED,
            "atelier_character_document",
            &new.story_document_id.to_string(),
            serde_json::json!({
                "beat_id": beat.beat_id,
                "story_document_id": beat.story_document_id,
                "card_id": beat.card_id,
                "seq": beat.seq,
                "beat_text_ref": event_ref_for_text(&beat.beat_text),
            }),
        )
        .await?;

        tx.commit().await?;
        Ok(beat)
    }

    pub async fn list_story_beats(&self, story_document_id: Uuid) -> AtelierResult<Vec<StoryBeat>> {
        self.require_story_document(story_document_id).await?;
        let rows = sqlx::query(
            r#"SELECT beat_id, story_document_id, card_id, seq, beat_text,
                      created_at_utc, updated_at_utc
               FROM atelier_story_beat
               WHERE story_document_id = $1
               ORDER BY seq ASC, beat_id ASC"#,
        )
        .bind(story_document_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(story_beat_from_row).collect())
    }
}
