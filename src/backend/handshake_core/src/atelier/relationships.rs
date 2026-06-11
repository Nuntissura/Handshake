//! Character relationships and relationship-map graph projection (MT-044).
//!
//! This module stores character-to-character relationship edges with explicit
//! endpoint validation and exposes a graph projection over the stored edges.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{event_ref_for_text, AtelierError, AtelierResult, AtelierStore};

pub mod relationships_event_family {
    pub const CHARACTER_RELATIONSHIP_CREATED: &str = "atelier.character_relationship.created";
    pub const CHARACTER_RELATIONSHIP_UPDATED: &str = "atelier.character_relationship.updated";
    pub const CHARACTER_RELATIONSHIP_DELETED: &str = "atelier.character_relationship.deleted";

    pub const ALL: &[&str] = &[
        CHARACTER_RELATIONSHIP_CREATED,
        CHARACTER_RELATIONSHIP_UPDATED,
        CHARACTER_RELATIONSHIP_DELETED,
    ];
}

#[derive(Clone, Debug)]
pub struct NewCharacterRelationship {
    pub source_character_id: Uuid,
    pub target_character_id: Uuid,
    pub relationship_kind: String,
    pub label: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UpdateCharacterRelationship {
    pub relationship_kind: String,
    pub label: Option<String>,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterRelationship {
    pub relationship_id: Uuid,
    pub source_character_id: Uuid,
    pub target_character_id: Uuid,
    pub relationship_kind: String,
    pub label: Option<String>,
    pub notes: String,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterRelationshipGraphNode {
    pub character_internal_id: Uuid,
    pub public_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterRelationshipGraphEdge {
    pub relationship_id: Uuid,
    pub source_character_id: Uuid,
    pub target_character_id: Uuid,
    pub relationship_kind: String,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterRelationshipGraph {
    pub anchor_character_id: Uuid,
    pub nodes: Vec<CharacterRelationshipGraphNode>,
    pub edges: Vec<CharacterRelationshipGraphEdge>,
}

fn clean_required_token(field: &str, value: &str) -> AtelierResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn clean_optional_text(field: &str, value: Option<&str>) -> AtelierResult<Option<String>> {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(|trimmed| {
            if trimmed.contains('\r') || trimmed.contains('\n') {
                return Err(AtelierError::Validation(format!(
                    "{field} must be a single-line value"
                )));
            }
            Ok(trimmed.to_string())
        })
        .transpose()
}

fn clean_notes(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

fn relationship_from_row(row: &sqlx::postgres::PgRow) -> CharacterRelationship {
    CharacterRelationship {
        relationship_id: row.get("relationship_id"),
        source_character_id: row.get("source_character_id"),
        target_character_id: row.get("target_character_id"),
        relationship_kind: row.get("relationship_kind"),
        label: row.get("label"),
        notes: row.get("notes"),
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    }
}

fn graph_node_from_row(row: &sqlx::postgres::PgRow) -> CharacterRelationshipGraphNode {
    CharacterRelationshipGraphNode {
        character_internal_id: row.get("internal_id"),
        public_id: row.get("public_id"),
        display_name: row.get("display_name"),
    }
}

fn graph_edge_from_row(row: &sqlx::postgres::PgRow) -> CharacterRelationshipGraphEdge {
    CharacterRelationshipGraphEdge {
        relationship_id: row.get("relationship_id"),
        source_character_id: row.get("source_character_id"),
        target_character_id: row.get("target_character_id"),
        relationship_kind: row.get("relationship_kind"),
        label: row.get("label"),
    }
}

impl AtelierStore {
    async fn require_character_endpoint(
        &self,
        field: &str,
        character_id: Uuid,
    ) -> AtelierResult<()> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM atelier_character WHERE internal_id = $1)",
        )
        .bind(character_id)
        .fetch_one(self.pool())
        .await?;
        if !exists {
            return Err(AtelierError::NotFound(format!(
                "{field} character endpoint {character_id}"
            )));
        }
        Ok(())
    }

    fn validate_relationship_endpoints(source: Uuid, target: Uuid) -> AtelierResult<()> {
        if source == target {
            return Err(AtelierError::Validation(
                "character relationship endpoints must be distinct".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn create_character_relationship(
        &self,
        new: &NewCharacterRelationship,
    ) -> AtelierResult<CharacterRelationship> {
        Self::validate_relationship_endpoints(new.source_character_id, new.target_character_id)?;
        self.require_character_endpoint("source", new.source_character_id)
            .await?;
        self.require_character_endpoint("target", new.target_character_id)
            .await?;
        let relationship_kind = clean_required_token("relationship_kind", &new.relationship_kind)?;
        let label = clean_optional_text("label", new.label.as_deref())?;
        let notes = clean_notes(new.notes.as_deref());
        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(
            r#"INSERT INTO atelier_character_relationship
                 (source_character_id, target_character_id, relationship_kind, label, notes)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING relationship_id, source_character_id, target_character_id,
                         relationship_kind, label, notes, created_at_utc, updated_at_utc"#,
        )
        .bind(new.source_character_id)
        .bind(new.target_character_id)
        .bind(&relationship_kind)
        .bind(label.as_deref())
        .bind(&notes)
        .fetch_one(&mut *tx)
        .await?;
        let relationship = relationship_from_row(&row);

        self.record_event_in_tx(
            &mut tx,
            relationships_event_family::CHARACTER_RELATIONSHIP_CREATED,
            "atelier_character_relationship",
            &relationship.relationship_id.to_string(),
            serde_json::json!({
                "relationship_id": relationship.relationship_id,
                "source_character_id_ref": event_ref_for_text(&relationship.source_character_id.to_string()),
                "target_character_id_ref": event_ref_for_text(&relationship.target_character_id.to_string()),
                "relationship_kind": relationship.relationship_kind,
                "has_label": relationship.label.is_some(),
                "has_notes": !relationship.notes.is_empty(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(relationship)
    }

    pub async fn get_character_relationship(
        &self,
        relationship_id: Uuid,
    ) -> AtelierResult<CharacterRelationship> {
        let row = sqlx::query(
            r#"SELECT relationship_id, source_character_id, target_character_id,
                      relationship_kind, label, notes, created_at_utc, updated_at_utc
               FROM atelier_character_relationship
               WHERE relationship_id = $1"#,
        )
        .bind(relationship_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("character relationship {relationship_id}"))
        })?;
        Ok(relationship_from_row(&row))
    }

    pub async fn update_character_relationship(
        &self,
        relationship_id: Uuid,
        update: &UpdateCharacterRelationship,
    ) -> AtelierResult<CharacterRelationship> {
        let relationship_kind =
            clean_required_token("relationship_kind", &update.relationship_kind)?;
        let label = clean_optional_text("label", update.label.as_deref())?;
        let notes = clean_notes(update.notes.as_deref());
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"UPDATE atelier_character_relationship
               SET relationship_kind = $2,
                   label = $3,
                   notes = $4,
                   updated_at_utc = NOW()
               WHERE relationship_id = $1
               RETURNING relationship_id, source_character_id, target_character_id,
                         relationship_kind, label, notes, created_at_utc, updated_at_utc"#,
        )
        .bind(relationship_id)
        .bind(&relationship_kind)
        .bind(label.as_deref())
        .bind(&notes)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("character relationship {relationship_id}"))
        })?;
        let relationship = relationship_from_row(&row);
        self.record_event_in_tx(
            &mut tx,
            relationships_event_family::CHARACTER_RELATIONSHIP_UPDATED,
            "atelier_character_relationship",
            &relationship.relationship_id.to_string(),
            serde_json::json!({
                "relationship_id": relationship.relationship_id,
                "relationship_kind": relationship.relationship_kind,
                "has_label": relationship.label.is_some(),
                "has_notes": !relationship.notes.is_empty(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(relationship)
    }

    pub async fn delete_character_relationship(
        &self,
        relationship_id: Uuid,
    ) -> AtelierResult<CharacterRelationship> {
        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"DELETE FROM atelier_character_relationship
               WHERE relationship_id = $1
               RETURNING relationship_id, source_character_id, target_character_id,
                         relationship_kind, label, notes, created_at_utc, updated_at_utc"#,
        )
        .bind(relationship_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("character relationship {relationship_id}"))
        })?;
        let relationship = relationship_from_row(&row);
        self.record_event_in_tx(
            &mut tx,
            relationships_event_family::CHARACTER_RELATIONSHIP_DELETED,
            "atelier_character_relationship",
            &relationship.relationship_id.to_string(),
            serde_json::json!({
                "relationship_id": relationship.relationship_id,
                "source_character_id_ref": event_ref_for_text(&relationship.source_character_id.to_string()),
                "target_character_id_ref": event_ref_for_text(&relationship.target_character_id.to_string()),
                "relationship_kind": relationship.relationship_kind,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(relationship)
    }

    pub async fn list_character_relationships(
        &self,
        character_id: Uuid,
    ) -> AtelierResult<Vec<CharacterRelationship>> {
        self.require_character_endpoint("relationship list", character_id)
            .await?;
        let rows = sqlx::query(
            r#"SELECT relationship_id, source_character_id, target_character_id,
                      relationship_kind, label, notes, created_at_utc, updated_at_utc
               FROM atelier_character_relationship
               WHERE source_character_id = $1 OR target_character_id = $1
               ORDER BY updated_at_utc DESC, relationship_id ASC"#,
        )
        .bind(character_id)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(relationship_from_row).collect())
    }

    pub async fn character_relationship_graph(
        &self,
        anchor_character_id: Uuid,
    ) -> AtelierResult<CharacterRelationshipGraph> {
        self.require_character_endpoint("graph anchor", anchor_character_id)
            .await?;
        let rows = sqlx::query(
            r#"WITH edge_rows AS (
                   SELECT edge_id AS relationship_id,
                          source_character_id,
                          target_character_id,
                          relationship_kind,
                          label,
                          updated_at_utc
                   FROM atelier_character_relationship_graph_projection
                   WHERE source_character_id = $1 OR target_character_id = $1
               ),
               node_rows AS (
                   SELECT DISTINCT c.internal_id, c.public_id, c.display_name
                   FROM atelier_character c
                   WHERE c.internal_id = $1
                      OR c.internal_id IN (
                           SELECT source_character_id FROM edge_rows
                           UNION
                           SELECT target_character_id FROM edge_rows
                      )
               )
               SELECT 0 AS sort_group,
                      updated_at_utc AS sort_updated_at,
                      NULL::text AS sort_text,
                      relationship_id AS sort_uuid,
                      'edge' AS row_kind,
                      relationship_id,
                      source_character_id,
                      target_character_id,
                      relationship_kind,
                      label,
                      NULL::uuid AS internal_id,
                      NULL::text AS public_id,
                      NULL::text AS display_name
               FROM edge_rows
               UNION ALL
               SELECT 1 AS sort_group,
                      NULL::timestamptz AS sort_updated_at,
                      public_id AS sort_text,
                      internal_id AS sort_uuid,
                      'node' AS row_kind,
                      NULL::uuid AS relationship_id,
                      NULL::uuid AS source_character_id,
                      NULL::uuid AS target_character_id,
                      NULL::text AS relationship_kind,
                      NULL::text AS label,
                      internal_id,
                      public_id,
                      display_name
               FROM node_rows
               ORDER BY sort_group ASC,
                        sort_updated_at DESC NULLS LAST,
                        sort_text ASC NULLS LAST,
                        sort_uuid ASC"#,
        )
        .bind(anchor_character_id)
        .fetch_all(self.pool())
        .await?;

        let mut edges = Vec::new();
        let mut nodes = Vec::new();
        for row in rows {
            let row_kind: String = row.get("row_kind");
            match row_kind.as_str() {
                "edge" => edges.push(graph_edge_from_row(&row)),
                "node" => nodes.push(graph_node_from_row(&row)),
                _ => {
                    return Err(AtelierError::Validation(format!(
                        "unexpected relationship graph row kind {row_kind}"
                    )));
                }
            }
        }

        Ok(CharacterRelationshipGraph {
            anchor_character_id,
            nodes,
            edges,
        })
    }
}
