//! Bracket-link and backlink projections (MT-041).
//!
//! The source document text remains unchanged authority. This module rebuilds
//! ordered projection rows from typed `[[kind:id|label]]` markers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::documents::CharacterDocumentType;
use super::{event_ref_for_text, AtelierError, AtelierResult, AtelierStore};

pub mod links_event_family {
    pub const BRACKET_LINKS_REBUILT: &str = "atelier.bracket_links.rebuilt";

    pub const ALL: &[&str] = &[BRACKET_LINKS_REBUILT];
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BracketLinkTargetKind {
    Character,
    Document,
    Story,
    Moodboard,
    Image,
}

impl BracketLinkTargetKind {
    pub fn as_token(self) -> &'static str {
        match self {
            BracketLinkTargetKind::Character => "character",
            BracketLinkTargetKind::Document => "document",
            BracketLinkTargetKind::Story => "story",
            BracketLinkTargetKind::Moodboard => "moodboard",
            BracketLinkTargetKind::Image => "image",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "character" => Ok(BracketLinkTargetKind::Character),
            "document" => Ok(BracketLinkTargetKind::Document),
            "story" => Ok(BracketLinkTargetKind::Story),
            "moodboard" => Ok(BracketLinkTargetKind::Moodboard),
            "image" => Ok(BracketLinkTargetKind::Image),
            other => Err(AtelierError::Validation(format!(
                "unknown bracket link target kind: {other}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BracketLinkProjection {
    pub link_id: Uuid,
    pub source_document_id: Uuid,
    pub source_version_id: Uuid,
    pub source_doc_type: CharacterDocumentType,
    pub seq: i64,
    pub raw_marker: String,
    pub target_kind: BracketLinkTargetKind,
    pub target_id: String,
    pub target_label: Option<String>,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedBracketLink {
    seq: i64,
    raw_marker: String,
    target_kind: BracketLinkTargetKind,
    target_id: String,
    target_label: Option<String>,
}

fn parse_target_kind(token: &str) -> AtelierResult<BracketLinkTargetKind> {
    match token {
        "character" | "char" => Ok(BracketLinkTargetKind::Character),
        "document" | "doc" => Ok(BracketLinkTargetKind::Document),
        "story" => Ok(BracketLinkTargetKind::Story),
        "moodboard" => Ok(BracketLinkTargetKind::Moodboard),
        "image" | "img" | "media" => Ok(BracketLinkTargetKind::Image),
        other => Err(AtelierError::Validation(format!(
            "unsupported bracket link kind: {other}"
        ))),
    }
}

fn parse_bracket_links(text: &str) -> AtelierResult<Vec<ParsedBracketLink>> {
    let mut links = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative_start) = text[cursor..].find("[[") {
        let start = cursor + relative_start;
        let inner_start = start + 2;
        let rest = &text[inner_start..];
        let Some(relative_end) = rest.find("]]") else {
            return Err(AtelierError::Validation(format!(
                "unterminated bracket link marker near byte {start}"
            )));
        };
        let inner = &rest[..relative_end];
        if inner.is_empty() || inner.contains("[[") || inner.contains('\n') || inner.contains('\r')
        {
            return Err(AtelierError::Validation(format!(
                "malformed bracket link marker near byte {start}"
            )));
        }
        let end = inner_start + relative_end + 2;
        let raw_marker_text = &text[start..end];
        let (target_part, label) = inner
            .split_once('|')
            .map(|(target, label)| (target, Some(label)))
            .unwrap_or((inner, None));
        let (kind_token, target_id) = target_part.split_once(':').ok_or_else(|| {
            AtelierError::Validation(format!(
                "bracket link marker {raw_marker_text:?} must use kind:id"
            ))
        })?;
        let kind_token = kind_token.trim().to_ascii_lowercase();
        let target_id = target_id.trim();
        if target_id.is_empty() {
            return Err(AtelierError::Validation(format!(
                "bracket link marker {raw_marker_text:?} has empty target id"
            )));
        }
        let target_label = label
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        links.push(ParsedBracketLink {
            seq: links.len() as i64 + 1,
            raw_marker: raw_marker_text.to_string(),
            target_kind: parse_target_kind(&kind_token)?,
            target_id: target_id.to_string(),
            target_label,
        });
        cursor = end;
    }
    Ok(links)
}

fn projection_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<BracketLinkProjection> {
    let source_doc_type_token: String = row.get("source_doc_type");
    let target_kind_token: String = row.get("target_kind");
    Ok(BracketLinkProjection {
        link_id: row.get("link_id"),
        source_document_id: row.get("source_document_id"),
        source_version_id: row.get("source_version_id"),
        source_doc_type: CharacterDocumentType::from_token(&source_doc_type_token)?,
        seq: row.get("seq"),
        raw_marker: row.get("raw_marker"),
        target_kind: BracketLinkTargetKind::from_token(&target_kind_token)?,
        target_id: row.get("target_id"),
        target_label: row.get("target_label"),
        created_at_utc: row.get("created_at_utc"),
    })
}

impl AtelierStore {
    async fn canonical_character_id_from_link_ref(&self, value: &str) -> AtelierResult<Uuid> {
        let internal_match = if let Ok(parsed_id) = Uuid::parse_str(value) {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT internal_id FROM atelier_character WHERE internal_id = $1",
            )
            .bind(parsed_id)
            .fetch_optional(self.pool())
            .await?
        } else {
            None
        };
        if let Some(internal_id) = internal_match {
            return Ok(internal_id);
        }

        sqlx::query_scalar::<_, Uuid>(
            "SELECT internal_id FROM atelier_character WHERE public_id = $1",
        )
        .bind(value)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("bracket link character target {value}")))
    }

    async fn canonical_bracket_link_target_id(
        &self,
        link: &ParsedBracketLink,
    ) -> AtelierResult<String> {
        match link.target_kind {
            BracketLinkTargetKind::Character => {
                let internal_id = self
                    .canonical_character_id_from_link_ref(&link.target_id)
                    .await?;
                Ok(internal_id.to_string())
            }
            BracketLinkTargetKind::Document
            | BracketLinkTargetKind::Story
            | BracketLinkTargetKind::Moodboard => {
                let document_id = Uuid::parse_str(&link.target_id).map_err(|_| {
                    AtelierError::Validation(format!(
                        "bracket link document target must be a UUID: {}",
                        link.target_id
                    ))
                })?;
                let doc_type: Option<String> = sqlx::query_scalar(
                    "SELECT doc_type FROM atelier_character_document WHERE document_id = $1",
                )
                .bind(document_id)
                .fetch_optional(self.pool())
                .await?;
                let doc_type = doc_type.ok_or_else(|| {
                    AtelierError::NotFound(format!("bracket link document target {document_id}"))
                })?;
                if link.target_kind == BracketLinkTargetKind::Story && doc_type != "story" {
                    return Err(AtelierError::Validation(format!(
                        "bracket link target {document_id} is not a story"
                    )));
                }
                if link.target_kind == BracketLinkTargetKind::Moodboard && doc_type != "moodboard" {
                    return Err(AtelierError::Validation(format!(
                        "bracket link target {document_id} is not a moodboard"
                    )));
                }
                Ok(document_id.to_string())
            }
            BracketLinkTargetKind::Image => {
                let asset_id = Uuid::parse_str(&link.target_id).map_err(|_| {
                    AtelierError::Validation(format!(
                        "bracket link image target must be a UUID: {}",
                        link.target_id
                    ))
                })?;
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM atelier_media_asset WHERE asset_id = $1)",
                )
                .bind(asset_id)
                .fetch_one(self.pool())
                .await?;
                if !exists {
                    return Err(AtelierError::NotFound(format!(
                        "bracket link image target {asset_id}"
                    )));
                }
                Ok(asset_id.to_string())
            }
        }
    }

    pub async fn rebuild_bracket_links_for_character_document(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<Vec<BracketLinkProjection>> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(39041, hashtext($1))")
            .bind(document_id.to_string())
            .execute(&mut *tx)
            .await?;

        let source_row = sqlx::query(
            r#"SELECT d.document_id, d.doc_type, v.version_id, v.body_raw_text
               FROM atelier_character_document d
               JOIN atelier_character_document_version v
                 ON v.version_id = d.current_version_id
               WHERE d.document_id = $1
               FOR UPDATE OF d"#,
        )
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?;
        let source_document_id: Uuid = source_row.get("document_id");
        let source_doc_type_token: String = source_row.get("doc_type");
        let source_doc_type = CharacterDocumentType::from_token(&source_doc_type_token)?;
        let source_version_id: Uuid = source_row.get("version_id");
        let source_body_raw_text: String = source_row.get("body_raw_text");

        let parsed_links = parse_bracket_links(&source_body_raw_text)?;
        let mut validated_links = Vec::with_capacity(parsed_links.len());
        for mut link in parsed_links {
            link.target_id = self.canonical_bracket_link_target_id(&link).await?;
            validated_links.push(link);
        }

        sqlx::query("DELETE FROM atelier_bracket_link_projection WHERE source_document_id = $1")
            .bind(document_id)
            .execute(&mut *tx)
            .await?;
        let mut rebuilt = Vec::new();
        for link in &validated_links {
            let row = sqlx::query(
                r#"INSERT INTO atelier_bracket_link_projection
                     (source_document_id, source_version_id, source_doc_type, seq,
                      raw_marker, target_kind, target_id, target_label)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                   RETURNING link_id, source_document_id, source_version_id,
                             source_doc_type, seq, raw_marker, target_kind,
                             target_id, target_label, created_at_utc"#,
            )
            .bind(source_document_id)
            .bind(source_version_id)
            .bind(source_doc_type.as_token())
            .bind(link.seq)
            .bind(&link.raw_marker)
            .bind(link.target_kind.as_token())
            .bind(&link.target_id)
            .bind(link.target_label.as_deref())
            .fetch_one(&mut *tx)
            .await?;
            rebuilt.push(projection_from_row(&row)?);
        }

        let target_kinds: Vec<&str> = rebuilt
            .iter()
            .map(|link| link.target_kind.as_token())
            .collect();
        self.record_event_in_tx(
            &mut tx,
            links_event_family::BRACKET_LINKS_REBUILT,
            "atelier_character_document",
            &source_document_id.to_string(),
            serde_json::json!({
                "source_document_id_ref": event_ref_for_text(&source_document_id.to_string()),
                "source_version_id_ref": event_ref_for_text(&source_version_id.to_string()),
                "source_doc_type": source_doc_type.as_token(),
                "link_count": rebuilt.len(),
                "target_kinds": target_kinds,
            }),
        )
        .await?;

        tx.commit().await?;
        Ok(rebuilt)
    }

    pub async fn list_bracket_links_from_document(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<Vec<BracketLinkProjection>> {
        let rows = sqlx::query(
            r#"SELECT link_id, source_document_id, source_version_id,
                      source_doc_type, seq, raw_marker, target_kind,
                      target_id, target_label, created_at_utc
               FROM atelier_bracket_link_projection
               WHERE source_document_id = $1
               ORDER BY seq ASC, link_id ASC"#,
        )
        .bind(document_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(projection_from_row).collect()
    }

    pub async fn list_backlinks_to(
        &self,
        target_kind: BracketLinkTargetKind,
        target_id: &str,
    ) -> AtelierResult<Vec<BracketLinkProjection>> {
        let target_id = target_id.trim();
        if target_id.is_empty() {
            return Err(AtelierError::Validation(
                "target_id must not be empty".into(),
            ));
        }
        let query_target_id = if target_kind == BracketLinkTargetKind::Character {
            self.canonical_character_id_from_link_ref(target_id)
                .await
                .map(|internal_id| internal_id.to_string())
                .unwrap_or_else(|_| target_id.to_string())
        } else {
            match target_kind {
                BracketLinkTargetKind::Document
                | BracketLinkTargetKind::Story
                | BracketLinkTargetKind::Moodboard
                | BracketLinkTargetKind::Image => Uuid::parse_str(target_id)
                    .map(|id| id.to_string())
                    .map_err(|_| {
                        AtelierError::Validation(format!(
                            "{} target_id must be a UUID: {target_id}",
                            target_kind.as_token()
                        ))
                    })?,
                BracketLinkTargetKind::Character => unreachable!(),
            }
        };
        let rows = sqlx::query(
            r#"SELECT link_id, source_document_id, source_version_id,
                      source_doc_type, seq, raw_marker, target_kind,
                      target_id, target_label, created_at_utc
               FROM atelier_bracket_link_projection
               WHERE target_kind = $1
                 AND target_id = $2
               ORDER BY source_document_id ASC, seq ASC, link_id ASC"#,
        )
        .bind(target_kind.as_token())
        .bind(&query_target_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(projection_from_row).collect()
    }
}
