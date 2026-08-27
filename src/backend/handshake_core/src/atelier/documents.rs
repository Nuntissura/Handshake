//! Character-scoped note/story/moodboard documents (MT-038).
//!
//! Documents have stable ids and typed metadata; raw text lives in append-only
//! versions so note/story/moodboard edits preserve prior text exactly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{
    atelier_event_sql, event_ref_for_text, uuid_from_record_link, AtelierError, AtelierResult,
    AtelierStore,
};

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

fn require_non_empty_trimmed(field: &str, value: &str) -> AtelierResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

/// One `atelier_character_document` row as the store returns it.
#[derive(SurrealValue)]
struct CharacterDocumentRow {
    document_id: SurrealUuid,
    character_internal_id: SurrealUuid,
    doc_type: String,
    title: String,
    tags_json: Vec<String>,
    current_version_id: Option<SurrealUuid>,
    current_version_seq: i64,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl TryFrom<CharacterDocumentRow> for CharacterDocument {
    type Error = AtelierError;

    fn try_from(row: CharacterDocumentRow) -> AtelierResult<Self> {
        let document_id: Uuid = row.document_id.into();
        Ok(CharacterDocument {
            document_id,
            character_internal_id: row.character_internal_id.into(),
            doc_type: CharacterDocumentType::from_token(&row.doc_type)?,
            title: row.title,
            tags: row.tags_json,
            current_version_id: row.current_version_id.map(Into::into).ok_or_else(|| {
                AtelierError::Validation(format!(
                    "character document {document_id} has no current version"
                ))
            })?,
            current_version_seq: row.current_version_seq,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

/// One `atelier_character_document_version` row as the store returns it.
#[derive(SurrealValue)]
struct CharacterDocumentVersionRow {
    version_id: SurrealUuid,
    document_id: SurrealUuid,
    version_seq: i64,
    title: String,
    body_raw_text: String,
    tags_json: Vec<String>,
    author: String,
    parent_version_id: Option<RecordId>,
    created_at_utc: Datetime,
}

impl TryFrom<CharacterDocumentVersionRow> for CharacterDocumentVersion {
    type Error = AtelierError;

    fn try_from(row: CharacterDocumentVersionRow) -> AtelierResult<Self> {
        let parent_version_id = row
            .parent_version_id
            .as_ref()
            .map(|link| uuid_from_record_link("parent_version_id", link))
            .transpose()?;
        Ok(CharacterDocumentVersion {
            version_id: row.version_id.into(),
            document_id: row.document_id.into(),
            version_seq: row.version_seq,
            title: row.title,
            body_raw_text: row.body_raw_text,
            tags: row.tags_json,
            author: row.author,
            parent_version_id,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

/// One `atelier_story_card` row as the store returns it.
#[derive(SurrealValue)]
struct StoryCardRow {
    card_id: SurrealUuid,
    story_document_id: SurrealUuid,
    seq: i64,
    title: String,
    body_raw_text: String,
    tags_json: Vec<String>,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl From<StoryCardRow> for StoryCard {
    fn from(row: StoryCardRow) -> Self {
        StoryCard {
            card_id: row.card_id.into(),
            story_document_id: row.story_document_id.into(),
            seq: row.seq,
            title: row.title,
            body_raw_text: row.body_raw_text,
            tags: row.tags_json,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

/// One `atelier_story_beat` row as the store returns it.
#[derive(SurrealValue)]
struct StoryBeatRow {
    beat_id: SurrealUuid,
    story_document_id: SurrealUuid,
    card_id: Option<RecordId>,
    seq: i64,
    beat_text: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl TryFrom<StoryBeatRow> for StoryBeat {
    type Error = AtelierError;

    fn try_from(row: StoryBeatRow) -> AtelierResult<Self> {
        let card_id = row
            .card_id
            .as_ref()
            .map(|link| uuid_from_record_link("card_id", link))
            .transpose()?;
        Ok(StoryBeat {
            beat_id: row.beat_id.into(),
            story_document_id: row.story_document_id.into(),
            card_id,
            seq: row.seq,
            beat_text: row.beat_text,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        })
    }
}

#[derive(Clone, SurrealValue)]
struct CreateDocumentBindings {
    doc_rid: RecordId,
    document_id: SurrealUuid,
    character_ref: RecordId,
    doc_type: String,
    title: String,
    tags_json: Vec<String>,
    version_rid: RecordId,
    version_id: SurrealUuid,
    body_raw_text: String,
    author: String,
}

#[derive(SurrealValue)]
struct DocumentIdBinding {
    document_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct ListDocumentsBindings {
    character_ref: RecordId,
    doc_type: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct AppendVersionBindings {
    document_id: SurrealUuid,
    version_rid: RecordId,
    version_id: SurrealUuid,
    title: String,
    body_raw_text: String,
    tags_json: Vec<String>,
    author: String,
}

#[derive(SurrealValue)]
struct DocumentRefBinding {
    doc_ref: RecordId,
}

#[derive(SurrealValue)]
struct CardIdBinding {
    card_id: SurrealUuid,
}

#[derive(Clone, SurrealValue)]
struct AddStoryCardBindings {
    card_rid: RecordId,
    card_id: SurrealUuid,
    story_ref: RecordId,
    seq: i64,
    title: String,
    body_raw_text: String,
    tags_json: Vec<String>,
}

#[derive(Clone, SurrealValue)]
struct AddStoryBeatBindings {
    beat_rid: RecordId,
    beat_id: SurrealUuid,
    story_ref: RecordId,
    card_ref: Option<RecordId>,
    seq: i64,
    beat_text: String,
}

const DOCUMENT_VERSION_SELECT: &str =
    "version_id, record::id(document_id) AS document_id, version_seq, title, \
     body_raw_text, tags_json, author, parent_version_id, created_at_utc";

/// Create the document, its first version, and the current-version pointer in
/// one atomic statement (the former insert-insert-update transaction).
const CREATE_DOCUMENT_STATEMENT: &str = concat!(
    "RETURN { \
       CREATE $doc_rid CONTENT { \
         document_id: $document_id, \
         character_internal_id: $character_ref, \
         doc_type: $doc_type, \
         title: $title, \
         tags_json: $tags_json \
       }; \
       CREATE $version_rid CONTENT { \
         version_id: $version_id, \
         document_id: $doc_rid, \
         version_seq: 1, \
         title: $title, \
         body_raw_text: $body_raw_text, \
         tags_json: $tags_json, \
         author: $author, \
         parent_version_id: NONE \
       }; \
       UPDATE $doc_rid SET \
         current_version_id = $version_id, \
         current_version_seq = 1, \
         updated_at_utc = time::now(); \
       RETURN (SELECT ",
    "version_id, record::id(document_id) AS document_id, version_seq, title, \
     body_raw_text, tags_json, author, parent_version_id, created_at_utc",
    " FROM $version_rid); };"
);

const GET_DOCUMENT_STATEMENT: &str =
    "SELECT document_id, record::id(character_internal_id) AS character_internal_id, \
            doc_type, title, tags_json, current_version_id, current_version_seq, \
            created_at_utc, updated_at_utc \
     FROM atelier_character_document WHERE document_id = $document_id LIMIT 1;";

const LIST_DOCUMENTS_STATEMENT: &str =
    "SELECT document_id, record::id(character_internal_id) AS character_internal_id, \
            doc_type, title, tags_json, current_version_id, current_version_seq, \
            created_at_utc, updated_at_utc \
     FROM atelier_character_document \
     WHERE character_internal_id = $character_ref \
       AND ($doc_type = NONE OR doc_type = $doc_type) \
     ORDER BY updated_at_utc DESC, document_id ASC;";

/// Append a version and advance the current-version pointer atomically. The
/// FOR loop is the missing-document guard: zero source rows append nothing,
/// and the final SELECT then returns an empty set, which the caller maps to
/// NotFound.
const APPEND_VERSION_STATEMENT: &str = concat!(
    "RETURN { \
       LET $doc_rid = type::record('atelier_character_document', $document_id); \
       LET $current = (SELECT current_version_id, current_version_seq FROM $doc_rid); \
       FOR $cur IN $current { \
         LET $next_seq = $cur.current_version_seq + 1; \
         LET $parent = IF $cur.current_version_id = NONE { NONE } ELSE { \
           type::record('atelier_character_document_version', $cur.current_version_id) \
         }; \
         CREATE $version_rid CONTENT { \
           version_id: $version_id, \
           document_id: $doc_rid, \
           version_seq: $next_seq, \
           title: $title, \
           body_raw_text: $body_raw_text, \
           tags_json: $tags_json, \
           author: $author, \
           parent_version_id: $parent \
         }; \
         UPDATE $doc_rid SET \
           title = $title, \
           tags_json = $tags_json, \
           current_version_id = $version_id, \
           current_version_seq = $next_seq, \
           updated_at_utc = time::now(); \
       }; \
       RETURN (SELECT ",
    "version_id, record::id(document_id) AS document_id, version_seq, title, \
     body_raw_text, tags_json, author, parent_version_id, created_at_utc",
    " FROM $version_rid); };"
);

const LATEST_VERSION_STATEMENT: &str = concat!(
    "SELECT ",
    "version_id, record::id(document_id) AS document_id, version_seq, title, \
     body_raw_text, tags_json, author, parent_version_id, created_at_utc",
    " FROM atelier_character_document_version WHERE document_id = $doc_ref \
     ORDER BY version_seq DESC LIMIT 1;"
);

const VERSION_HISTORY_STATEMENT: &str = concat!(
    "SELECT ",
    "version_id, record::id(document_id) AS document_id, version_seq, title, \
     body_raw_text, tags_json, author, parent_version_id, created_at_utc",
    " FROM atelier_character_document_version WHERE document_id = $doc_ref \
     ORDER BY version_seq ASC;"
);

const NEXT_STORY_CARD_SEQ_STATEMENT: &str =
    "RETURN (array::max((SELECT VALUE seq FROM atelier_story_card \
                         WHERE story_document_id = $doc_ref)) ?? 0) + 1;";

const ADD_STORY_CARD_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.card_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         card_id: $domain.card_id, \
         story_document_id: $domain.story_ref, \
         seq: $domain.seq, \
         title: $domain.title, \
         body_raw_text: $domain.body_raw_text, \
         tags_json: $domain.tags_json \
       }; \
       RETURN (SELECT card_id, record::id(story_document_id) AS story_document_id, seq, \
                      title, body_raw_text, tags_json, created_at_utc, updated_at_utc \
               FROM $rid); };"
);

const LIST_STORY_CARDS_STATEMENT: &str =
    "SELECT card_id, record::id(story_document_id) AS story_document_id, seq, title, \
            body_raw_text, tags_json, created_at_utc, updated_at_utc \
     FROM atelier_story_card WHERE story_document_id = $doc_ref \
     ORDER BY seq ASC, card_id ASC;";

const STORY_CARD_OWNER_STATEMENT: &str =
    "SELECT VALUE record::id(story_document_id) FROM atelier_story_card \
     WHERE card_id = $card_id LIMIT 1;";

const NEXT_STORY_BEAT_SEQ_STATEMENT: &str =
    "RETURN (array::max((SELECT VALUE seq FROM atelier_story_beat \
                         WHERE story_document_id = $doc_ref)) ?? 0) + 1;";

const ADD_STORY_BEAT_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.beat_rid; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         beat_id: $domain.beat_id, \
         story_document_id: $domain.story_ref, \
         card_id: $domain.card_ref, \
         seq: $domain.seq, \
         beat_text: $domain.beat_text \
       }; \
       RETURN (SELECT beat_id, record::id(story_document_id) AS story_document_id, card_id, \
                      seq, beat_text, created_at_utc, updated_at_utc \
               FROM $rid); };"
);

const LIST_STORY_BEATS_STATEMENT: &str =
    "SELECT beat_id, record::id(story_document_id) AS story_document_id, card_id, seq, \
            beat_text, created_at_utc, updated_at_utc \
     FROM atelier_story_beat WHERE story_document_id = $doc_ref \
     ORDER BY seq ASC, beat_id ASC;";

/// How often a seq-assigning insert retries when a concurrent writer takes the
/// same `(document, seq)` slot. The former PostgreSQL code serialized these
/// writers with an advisory lock; here the unique index rejects the loser and
/// the retry re-reads the next free sequence.
const SEQ_RACE_RETRIES: usize = 5;

fn is_unique_index_conflict(error: &AtelierError) -> bool {
    let text = error.to_string();
    text.contains("already contains") || text.contains("uq_atelier_")
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
        let document_id = Uuid::now_v7();
        let version_id = Uuid::now_v7();

        let bindings = CreateDocumentBindings {
            doc_rid: RecordId::new("atelier_character_document", SurrealUuid::from(document_id)),
            document_id: SurrealUuid::from(document_id),
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.character_internal_id),
            ),
            doc_type: new.doc_type.as_token().to_owned(),
            title: title.clone(),
            tags_json: tags.clone(),
            version_rid: RecordId::new(
                "atelier_character_document_version",
                SurrealUuid::from(version_id),
            ),
            version_id: SurrealUuid::from(version_id),
            body_raw_text: new.body_raw_text.clone(),
            author: author.clone(),
        };
        let row: Option<CharacterDocumentVersionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(CREATE_DOCUMENT_STATEMENT, bindings).await })
            })
            .await?;
        let version: CharacterDocumentVersion = row
            .ok_or_else(|| {
                AtelierError::Internal("creating a character document returned no row".to_owned())
            })?
            .try_into()?;

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
        let bindings = DocumentIdBinding {
            document_id: SurrealUuid::from(document_id),
        };
        let row: Option<CharacterDocumentRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_DOCUMENT_STATEMENT, bindings).await })
            })
            .await?;
        row.ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?
            .try_into()
    }

    pub async fn list_character_documents(
        &self,
        character_internal_id: Uuid,
        doc_type: Option<CharacterDocumentType>,
    ) -> AtelierResult<Vec<CharacterDocument>> {
        let bindings = ListDocumentsBindings {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(character_internal_id),
            ),
            doc_type: doc_type.map(|value| value.as_token().to_owned()),
        };
        let rows: Vec<CharacterDocumentRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_DOCUMENTS_STATEMENT, bindings).await })
            })
            .await?;
        rows.into_iter().map(CharacterDocument::try_from).collect()
    }

    pub async fn append_character_document_version(
        &self,
        document_id: Uuid,
        update: &AppendCharacterDocumentVersion,
    ) -> AtelierResult<CharacterDocumentVersion> {
        let title = require_non_empty_trimmed("title", &update.title)?;
        let author = require_non_empty_trimmed("author", &update.author)?;
        let tags = clean_document_tags(&update.tags);
        let version_id = Uuid::now_v7();

        let bindings = AppendVersionBindings {
            document_id: SurrealUuid::from(document_id),
            version_rid: RecordId::new(
                "atelier_character_document_version",
                SurrealUuid::from(version_id),
            ),
            version_id: SurrealUuid::from(version_id),
            title: title.clone(),
            body_raw_text: update.body_raw_text.clone(),
            tags_json: tags.clone(),
            author: author.clone(),
        };
        let row: Option<CharacterDocumentVersionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(APPEND_VERSION_STATEMENT, bindings).await })
            })
            .await?;
        let version: CharacterDocumentVersion = row
            .ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?
            .try_into()?;

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
        let bindings = DocumentRefBinding {
            doc_ref: RecordId::new("atelier_character_document", SurrealUuid::from(document_id)),
        };
        let row: Option<CharacterDocumentVersionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(LATEST_VERSION_STATEMENT, bindings).await })
            })
            .await?;
        row.map(CharacterDocumentVersion::try_from).transpose()
    }

    pub async fn character_document_history(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<Vec<CharacterDocumentVersion>> {
        let bindings = DocumentRefBinding {
            doc_ref: RecordId::new("atelier_character_document", SurrealUuid::from(document_id)),
        };
        let rows: Vec<CharacterDocumentVersionRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(VERSION_HISTORY_STATEMENT, bindings).await })
            })
            .await?;
        rows.into_iter()
            .map(CharacterDocumentVersion::try_from)
            .collect()
    }

    async fn next_story_seq(
        &self,
        statement: &'static str,
        story_document_id: Uuid,
    ) -> AtelierResult<i64> {
        let bindings = DocumentRefBinding {
            doc_ref: RecordId::new(
                "atelier_character_document",
                SurrealUuid::from(story_document_id),
            ),
        };
        let seq: Option<i64> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(statement, bindings).await })
            })
            .await?;
        seq.ok_or_else(|| {
            AtelierError::Internal("computing the next story sequence returned no value".to_owned())
        })
    }

    pub async fn add_story_card(&self, new: &NewStoryCard) -> AtelierResult<StoryCard> {
        self.require_story_document(new.story_document_id).await?;
        let title = require_non_empty_trimmed("title", &new.title)?;
        let tags = clean_document_tags(&new.tags);

        let mut last_error: Option<AtelierError> = None;
        for _ in 0..SEQ_RACE_RETRIES {
            let seq = self
                .next_story_seq(NEXT_STORY_CARD_SEQ_STATEMENT, new.story_document_id)
                .await?;
            let card_id = Uuid::now_v7();
            let bindings = AddStoryCardBindings {
                card_rid: RecordId::new("atelier_story_card", SurrealUuid::from(card_id)),
                card_id: SurrealUuid::from(card_id),
                story_ref: RecordId::new(
                    "atelier_character_document",
                    SurrealUuid::from(new.story_document_id),
                ),
                seq,
                title: title.clone(),
                body_raw_text: new.body_raw_text.clone(),
                tags_json: tags.clone(),
            };
            let written: AtelierResult<Option<StoryCardRow>> = self
                .write_with_event(
                    ADD_STORY_CARD_STATEMENT,
                    bindings,
                    documents_event_family::STORY_CARD_ADDED,
                    "atelier_character_document",
                    &new.story_document_id.to_string(),
                    serde_json::json!({
                        "card_id": card_id,
                        "story_document_id": new.story_document_id,
                        "seq": seq,
                        "title": title,
                        "tag_count": tags.len(),
                        "body_raw_text_ref": event_ref_for_text(&new.body_raw_text),
                    }),
                )
                .await;
            match written {
                Ok(Some(row)) => return Ok(row.into()),
                Ok(None) => {
                    return Err(AtelierError::Internal(
                        "adding a story card returned no row".to_owned(),
                    ));
                }
                Err(error) if is_unique_index_conflict(&error) => {
                    last_error = Some(error);
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            AtelierError::Internal("story card sequence retries exhausted".to_owned())
        }))
    }

    pub async fn list_story_cards(&self, story_document_id: Uuid) -> AtelierResult<Vec<StoryCard>> {
        self.require_story_document(story_document_id).await?;
        let bindings = DocumentRefBinding {
            doc_ref: RecordId::new(
                "atelier_character_document",
                SurrealUuid::from(story_document_id),
            ),
        };
        let rows: Vec<StoryCardRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(
                    async move { ctx.query_values(LIST_STORY_CARDS_STATEMENT, bindings).await },
                )
            })
            .await?;
        Ok(rows.into_iter().map(StoryCard::from).collect())
    }

    pub async fn add_story_beat(&self, new: &NewStoryBeat) -> AtelierResult<StoryBeat> {
        self.require_story_document(new.story_document_id).await?;
        require_non_empty_trimmed("beat_text", &new.beat_text)?;

        if let Some(card_id) = new.card_id {
            let bindings = CardIdBinding {
                card_id: SurrealUuid::from(card_id),
            };
            let card_story_document_id: Option<SurrealUuid> = self
                .store()
                .with_data_operation(move |ctx| {
                    Box::pin(
                        async move { ctx.query_first(STORY_CARD_OWNER_STATEMENT, bindings).await },
                    )
                })
                .await?;
            let card_story_document_id: Uuid = card_story_document_id
                .map(Into::into)
                .ok_or_else(|| AtelierError::NotFound(format!("story card {card_id}")))?;
            if card_story_document_id != new.story_document_id {
                return Err(AtelierError::Validation(format!(
                    "story card {card_id} does not belong to story document {}",
                    new.story_document_id
                )));
            }
        }

        let mut last_error: Option<AtelierError> = None;
        for _ in 0..SEQ_RACE_RETRIES {
            let seq = self
                .next_story_seq(NEXT_STORY_BEAT_SEQ_STATEMENT, new.story_document_id)
                .await?;
            let beat_id = Uuid::now_v7();
            let bindings = AddStoryBeatBindings {
                beat_rid: RecordId::new("atelier_story_beat", SurrealUuid::from(beat_id)),
                beat_id: SurrealUuid::from(beat_id),
                story_ref: RecordId::new(
                    "atelier_character_document",
                    SurrealUuid::from(new.story_document_id),
                ),
                card_ref: new
                    .card_id
                    .map(|card_id| RecordId::new("atelier_story_card", SurrealUuid::from(card_id))),
                seq,
                beat_text: new.beat_text.clone(),
            };
            let written: AtelierResult<Option<StoryBeatRow>> = self
                .write_with_event(
                    ADD_STORY_BEAT_STATEMENT,
                    bindings,
                    documents_event_family::STORY_BEAT_ADDED,
                    "atelier_character_document",
                    &new.story_document_id.to_string(),
                    serde_json::json!({
                        "beat_id": beat_id,
                        "story_document_id": new.story_document_id,
                        "card_id": new.card_id,
                        "seq": seq,
                        "beat_text_ref": event_ref_for_text(&new.beat_text),
                    }),
                )
                .await;
            match written {
                Ok(Some(row)) => return row.try_into(),
                Ok(None) => {
                    return Err(AtelierError::Internal(
                        "adding a story beat returned no row".to_owned(),
                    ));
                }
                Err(error) if is_unique_index_conflict(&error) => {
                    last_error = Some(error);
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            AtelierError::Internal("story beat sequence retries exhausted".to_owned())
        }))
    }

    pub async fn list_story_beats(&self, story_document_id: Uuid) -> AtelierResult<Vec<StoryBeat>> {
        self.require_story_document(story_document_id).await?;
        let bindings = DocumentRefBinding {
            doc_ref: RecordId::new(
                "atelier_character_document",
                SurrealUuid::from(story_document_id),
            ),
        };
        let rows: Vec<StoryBeatRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(
                    async move { ctx.query_values(LIST_STORY_BEATS_STATEMENT, bindings).await },
                )
            })
            .await?;
        rows.into_iter().map(StoryBeat::try_from).collect()
    }
}
