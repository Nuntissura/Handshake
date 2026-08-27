//! Bracket-link and backlink projections (MT-041).
//!
//! The source document text remains unchanged authority. This module rebuilds
//! ordered projection rows from typed `[[kind:id|label]]` markers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::documents::CharacterDocumentType;
use super::{atelier_event_sql, event_ref_for_text, AtelierError, AtelierResult, AtelierStore};

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

/// One `atelier_bracket_link_projection` row as the store returns it, with the
/// source links projected back to their uuid keys.
#[derive(SurrealValue)]
struct BracketLinkRow {
    link_id: SurrealUuid,
    source_document_id: SurrealUuid,
    source_version_id: SurrealUuid,
    source_doc_type: String,
    seq: i64,
    raw_marker: String,
    target_kind: String,
    target_id: String,
    target_label: Option<String>,
    created_at_utc: Datetime,
}

impl TryFrom<BracketLinkRow> for BracketLinkProjection {
    type Error = AtelierError;

    fn try_from(row: BracketLinkRow) -> AtelierResult<Self> {
        Ok(BracketLinkProjection {
            link_id: row.link_id.into(),
            source_document_id: row.source_document_id.into(),
            source_version_id: row.source_version_id.into(),
            source_doc_type: CharacterDocumentType::from_token(&row.source_doc_type)?,
            seq: row.seq,
            raw_marker: row.raw_marker,
            target_kind: BracketLinkTargetKind::from_token(&row.target_kind)?,
            target_id: row.target_id,
            target_label: row.target_label,
            created_at_utc: row.created_at_utc.into(),
        })
    }
}

#[derive(SurrealValue)]
struct InternalIdBinding {
    internal_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct PublicIdBinding {
    public_id: String,
}

#[derive(SurrealValue)]
struct DocumentIdBinding {
    document_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct VersionIdBinding {
    version_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct AssetIdBinding {
    asset_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct DocumentRefBinding {
    doc_ref: RecordId,
}

#[derive(SurrealValue)]
struct BacklinkBindings {
    target_kind: String,
    target_id: String,
}

/// One rebuilt link travelling to [`REBUILD_BRACKET_LINKS_STATEMENT`].
#[derive(Clone, SurrealValue)]
struct BracketLinkInsert {
    link_id: SurrealUuid,
    seq: i64,
    raw_marker: String,
    target_kind: String,
    target_id: String,
    target_label: Option<String>,
}

#[derive(Clone, SurrealValue)]
struct RebuildBracketLinksBindings {
    doc_ref: RecordId,
    version_ref: RecordId,
    source_doc_type: String,
    links: Vec<BracketLinkInsert>,
}

#[derive(SurrealValue)]
struct DocumentSourceRow {
    document_id: SurrealUuid,
    doc_type: String,
    current_version_id: Option<SurrealUuid>,
}

const SELECT_DOCUMENT_SOURCE_STATEMENT: &str =
    "SELECT document_id, doc_type, current_version_id FROM atelier_character_document \
     WHERE document_id = $document_id LIMIT 1;";

const SELECT_VERSION_BODY_STATEMENT: &str =
    "SELECT VALUE body_raw_text FROM atelier_character_document_version \
     WHERE version_id = $version_id LIMIT 1;";

/// Replace the projection set for one document and append the rebuild event in
/// the same atomic statement, so a reader can never observe a half-rebuilt
/// projection. This is the replacement for the former advisory-lock +
/// row-lock transaction: the whole delete + insert set commits together.
const REBUILD_BRACKET_LINKS_STATEMENT: &str = concat!(
    "RETURN { \
       DELETE atelier_bracket_link_projection WHERE source_document_id = $domain.doc_ref; \
       FOR $link IN $domain.links { \
         CREATE type::record('atelier_bracket_link_projection', $link.link_id) CONTENT { \
           link_id: $link.link_id, \
           source_document_id: $domain.doc_ref, \
           source_version_id: $domain.version_ref, \
           source_doc_type: $domain.source_doc_type, \
           seq: $link.seq, \
           raw_marker: $link.raw_marker, \
           target_kind: $link.target_kind, \
           target_id: $link.target_id, \
           target_label: $link.target_label \
         }; \
       }; ",
    atelier_event_sql!(),
    " RETURN array::len($domain.links); };"
);

const LIST_BRACKET_LINKS_STATEMENT: &str = concat!(
    "SELECT ",
    "link_id, record::id(source_document_id) AS source_document_id, \
     record::id(source_version_id) AS source_version_id, source_doc_type, seq, \
     raw_marker, target_kind, target_id, target_label, created_at_utc",
    " FROM atelier_bracket_link_projection WHERE source_document_id = $doc_ref \
     ORDER BY seq ASC, link_id ASC;"
);

const LIST_BACKLINKS_STATEMENT: &str = concat!(
    "SELECT ",
    "link_id, record::id(source_document_id) AS source_document_id, \
     record::id(source_version_id) AS source_version_id, source_doc_type, seq, \
     raw_marker, target_kind, target_id, target_label, created_at_utc",
    " FROM atelier_bracket_link_projection \
     WHERE target_kind = $target_kind AND target_id = $target_id \
     ORDER BY source_document_id ASC, seq ASC, link_id ASC;"
);

impl AtelierStore {
    async fn canonical_character_id_from_link_ref(&self, value: &str) -> AtelierResult<Uuid> {
        if let Ok(parsed_id) = Uuid::parse_str(value) {
            let bindings = InternalIdBinding {
                internal_id: SurrealUuid::from(parsed_id),
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
            if exists.unwrap_or(false) {
                return Ok(parsed_id);
            }
        }

        let bindings = PublicIdBinding {
            public_id: value.to_owned(),
        };
        let internal_id: Option<SurrealUuid> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(
                        "SELECT VALUE internal_id FROM atelier_character \
                         WHERE public_id = $public_id LIMIT 1;",
                        bindings,
                    )
                    .await
                })
            })
            .await?;
        internal_id
            .map(Into::into)
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
                let bindings = DocumentIdBinding {
                    document_id: SurrealUuid::from(document_id),
                };
                let doc_type: Option<String> = self
                    .store()
                    .with_data_operation(move |ctx| {
                        Box::pin(async move {
                            ctx.query_first(
                                "SELECT VALUE doc_type FROM atelier_character_document \
                                 WHERE document_id = $document_id LIMIT 1;",
                                bindings,
                            )
                            .await
                        })
                    })
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
                let bindings = AssetIdBinding {
                    asset_id: SurrealUuid::from(asset_id),
                };
                let exists: Option<bool> = self
                    .store()
                    .with_data_operation(move |ctx| {
                        Box::pin(async move {
                            ctx.query_first(
                                "RETURN record::exists(type::record('atelier_media_asset', $asset_id));",
                                bindings,
                            )
                            .await
                        })
                    })
                    .await?;
                if !exists.unwrap_or(false) {
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
        let source_bindings = DocumentIdBinding {
            document_id: SurrealUuid::from(document_id),
        };
        let source: Option<DocumentSourceRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(SELECT_DOCUMENT_SOURCE_STATEMENT, source_bindings)
                        .await
                })
            })
            .await?;
        let source = source
            .ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?;
        let source_doc_type = CharacterDocumentType::from_token(&source.doc_type)?;
        let source_version_id: Uuid = source
            .current_version_id
            .map(Into::into)
            .ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?;
        let source_document_id: Uuid = source.document_id.into();

        let body_bindings = VersionIdBinding {
            version_id: SurrealUuid::from(source_version_id),
        };
        let source_body_raw_text: Option<String> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_first(SELECT_VERSION_BODY_STATEMENT, body_bindings)
                        .await
                })
            })
            .await?;
        let source_body_raw_text = source_body_raw_text
            .ok_or_else(|| AtelierError::NotFound(format!("character document {document_id}")))?;

        let parsed_links = parse_bracket_links(&source_body_raw_text)?;
        let mut validated_links = Vec::with_capacity(parsed_links.len());
        for mut link in parsed_links {
            link.target_id = self.canonical_bracket_link_target_id(&link).await?;
            validated_links.push(link);
        }

        let target_kinds: Vec<&str> = validated_links
            .iter()
            .map(|link| link.target_kind.as_token())
            .collect();
        let inserts: Vec<BracketLinkInsert> = validated_links
            .iter()
            .map(|link| BracketLinkInsert {
                link_id: SurrealUuid::from(Uuid::now_v7()),
                seq: link.seq,
                raw_marker: link.raw_marker.clone(),
                target_kind: link.target_kind.as_token().to_owned(),
                target_id: link.target_id.clone(),
                target_label: link.target_label.clone(),
            })
            .collect();
        let bindings = RebuildBracketLinksBindings {
            doc_ref: RecordId::new(
                "atelier_character_document",
                SurrealUuid::from(source_document_id),
            ),
            version_ref: RecordId::new(
                "atelier_character_document_version",
                SurrealUuid::from(source_version_id),
            ),
            source_doc_type: source_doc_type.as_token().to_owned(),
            links: inserts,
        };

        let written: Option<i64> = self
            .write_with_event(
                REBUILD_BRACKET_LINKS_STATEMENT,
                bindings,
                links_event_family::BRACKET_LINKS_REBUILT,
                "atelier_character_document",
                &source_document_id.to_string(),
                serde_json::json!({
                    "source_document_id_ref": event_ref_for_text(&source_document_id.to_string()),
                    "source_version_id_ref": event_ref_for_text(&source_version_id.to_string()),
                    "source_doc_type": source_doc_type.as_token(),
                    "link_count": validated_links.len(),
                    "target_kinds": target_kinds,
                }),
            )
            .await?;
        if written.is_none() {
            return Err(AtelierError::Internal(
                "rebuilding bracket links returned no result".to_owned(),
            ));
        }

        self.list_bracket_links_from_document(document_id).await
    }

    pub async fn list_bracket_links_from_document(
        &self,
        document_id: Uuid,
    ) -> AtelierResult<Vec<BracketLinkProjection>> {
        let bindings = DocumentRefBinding {
            doc_ref: RecordId::new("atelier_character_document", SurrealUuid::from(document_id)),
        };
        let rows: Vec<BracketLinkRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_BRACKET_LINKS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        rows.into_iter()
            .map(BracketLinkProjection::try_from)
            .collect()
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
        let bindings = BacklinkBindings {
            target_kind: target_kind.as_token().to_owned(),
            target_id: query_target_id,
        };
        let rows: Vec<BracketLinkRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_values(LIST_BACKLINKS_STATEMENT, bindings).await })
            })
            .await?;
        rows.into_iter()
            .map(BracketLinkProjection::try_from)
            .collect()
    }
}
