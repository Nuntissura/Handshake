//! Character relationships and relationship-map graph projection (MT-044).
//!
//! This module stores character-to-character relationship edges with explicit
//! endpoint validation and exposes a graph projection over the stored edges.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::{Datetime, RecordId, SurrealValue, Uuid as SurrealUuid};
use uuid::Uuid;

use super::{atelier_event_sql, event_ref_for_text, AtelierError, AtelierResult, AtelierStore};

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

/// One `atelier_character_relationship` row as the store returns it, with the
/// endpoint links projected back to their uuid keys.
#[derive(SurrealValue)]
struct CharacterRelationshipRow {
    relationship_id: SurrealUuid,
    source_character_id: SurrealUuid,
    target_character_id: SurrealUuid,
    relationship_kind: String,
    label: Option<String>,
    notes: String,
    created_at_utc: Datetime,
    updated_at_utc: Datetime,
}

impl From<CharacterRelationshipRow> for CharacterRelationship {
    fn from(row: CharacterRelationshipRow) -> Self {
        CharacterRelationship {
            relationship_id: row.relationship_id.into(),
            source_character_id: row.source_character_id.into(),
            target_character_id: row.target_character_id.into(),
            relationship_kind: row.relationship_kind,
            label: row.label,
            notes: row.notes,
            created_at_utc: row.created_at_utc.into(),
            updated_at_utc: row.updated_at_utc.into(),
        }
    }
}

/// One graph-projection edge row.
#[derive(SurrealValue)]
struct GraphEdgeRow {
    relationship_id: SurrealUuid,
    source_character_id: SurrealUuid,
    target_character_id: SurrealUuid,
    relationship_kind: String,
    label: Option<String>,
}

/// One graph node row from `atelier_character`.
#[derive(SurrealValue)]
struct GraphNodeRow {
    internal_id: SurrealUuid,
    public_id: String,
    display_name: String,
}

#[derive(SurrealValue)]
struct CharacterExistsBinding {
    internal_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct RelationshipIdBinding {
    relationship_id: SurrealUuid,
}

#[derive(SurrealValue)]
struct CharacterRefBinding {
    character_ref: RecordId,
}

#[derive(SurrealValue)]
struct GraphNodesBindings {
    anchor_id: SurrealUuid,
    endpoint_ids: Vec<SurrealUuid>,
}

#[derive(Clone, SurrealValue)]
struct CreateRelationshipBindings {
    record_id: RecordId,
    relationship_id: SurrealUuid,
    source_ref: RecordId,
    target_ref: RecordId,
    relationship_kind: String,
    label: Option<String>,
    notes: String,
}

#[derive(Clone, SurrealValue)]
struct UpdateRelationshipBindings {
    relationship_id: SurrealUuid,
    relationship_kind: String,
    label: Option<String>,
    notes: String,
}

const RELATIONSHIP_SELECT_LIST: &str =
    "relationship_id, record::id(source_character_id) AS source_character_id, \
     record::id(target_character_id) AS target_character_id, relationship_kind, \
     label, notes, created_at_utc, updated_at_utc";

const CREATE_RELATIONSHIP_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = $domain.record_id; ",
    atelier_event_sql!(),
    " CREATE $rid CONTENT { \
         relationship_id: $domain.relationship_id, \
         source_character_id: $domain.source_ref, \
         target_character_id: $domain.target_ref, \
         relationship_kind: $domain.relationship_kind, \
         label: $domain.label, \
         notes: $domain.notes \
       }; \
       RETURN (SELECT ",
    "relationship_id, record::id(source_character_id) AS source_character_id, \
     record::id(target_character_id) AS target_character_id, relationship_kind, \
     label, notes, created_at_utc, updated_at_utc",
    " FROM ONLY $rid); };"
);

const GET_RELATIONSHIP_STATEMENT: &str = concat!(
    "SELECT ",
    "relationship_id, record::id(source_character_id) AS source_character_id, \
     record::id(target_character_id) AS target_character_id, relationship_kind, \
     label, notes, created_at_utc, updated_at_utc",
    " FROM atelier_character_relationship WHERE relationship_id = $relationship_id LIMIT 1;"
);

const UPDATE_RELATIONSHIP_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_character_relationship', $domain.relationship_id); ",
    atelier_event_sql!(),
    " UPDATE $rid SET \
         relationship_kind = $domain.relationship_kind, \
         label = $domain.label, \
         notes = $domain.notes, \
         updated_at_utc = time::now(); \
       RETURN (SELECT ",
    "relationship_id, record::id(source_character_id) AS source_character_id, \
     record::id(target_character_id) AS target_character_id, relationship_kind, \
     label, notes, created_at_utc, updated_at_utc",
    " FROM $rid); };"
);

const DELETE_RELATIONSHIP_STATEMENT: &str = concat!(
    "RETURN { \
       LET $rid = type::record('atelier_character_relationship', $domain.relationship_id); \
       LET $before = (SELECT ",
    "relationship_id, record::id(source_character_id) AS source_character_id, \
     record::id(target_character_id) AS target_character_id, relationship_kind, \
     label, notes, created_at_utc, updated_at_utc",
    " FROM $rid); ",
    atelier_event_sql!(),
    " DELETE $rid; \
       RETURN $before; };"
);

const LIST_RELATIONSHIPS_STATEMENT: &str = concat!(
    "SELECT ",
    "relationship_id, record::id(source_character_id) AS source_character_id, \
     record::id(target_character_id) AS target_character_id, relationship_kind, \
     label, notes, created_at_utc, updated_at_utc",
    " FROM atelier_character_relationship \
     WHERE source_character_id = $character_ref OR target_character_id = $character_ref \
     ORDER BY updated_at_utc DESC, relationship_id ASC;"
);

/// Edges around one character, from the stored-edge projection table, newest
/// first (the former edge sort: updated DESC, id ASC).
const GRAPH_EDGES_STATEMENT: &str = "SELECT record::id(edge_id) AS relationship_id, \
            record::id(source_character_id) AS source_character_id, \
            record::id(target_character_id) AS target_character_id, \
            relationship_kind, label \
     FROM atelier_character_relationship_graph_projection \
     WHERE source_character_id = $character_ref OR target_character_id = $character_ref \
     ORDER BY updated_at_utc DESC, relationship_id ASC;";

/// Nodes for the anchor plus every edge endpoint (public_id ASC, id ASC — the
/// former node sort).
const GRAPH_NODES_STATEMENT: &str =
    "SELECT internal_id, public_id, display_name FROM atelier_character \
     WHERE internal_id = $anchor_id OR internal_id IN $endpoint_ids \
     ORDER BY public_id ASC, internal_id ASC;";

impl AtelierStore {
    async fn require_character_endpoint(
        &self,
        field: &str,
        character_id: Uuid,
    ) -> AtelierResult<()> {
        let bindings = CharacterExistsBinding {
            internal_id: SurrealUuid::from(character_id),
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
        if !exists.unwrap_or(false) {
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
        let relationship_id = Uuid::now_v7();

        let bindings = CreateRelationshipBindings {
            record_id: RecordId::new(
                "atelier_character_relationship",
                SurrealUuid::from(relationship_id),
            ),
            relationship_id: SurrealUuid::from(relationship_id),
            source_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.source_character_id),
            ),
            target_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(new.target_character_id),
            ),
            relationship_kind: relationship_kind.clone(),
            label: label.clone(),
            notes: notes.clone(),
        };
        let row: Option<CharacterRelationshipRow> = self
            .write_with_event(
                CREATE_RELATIONSHIP_STATEMENT,
                bindings,
                relationships_event_family::CHARACTER_RELATIONSHIP_CREATED,
                "atelier_character_relationship",
                &relationship_id.to_string(),
                serde_json::json!({
                    "relationship_id": relationship_id,
                    "source_character_id_ref":
                        event_ref_for_text(&new.source_character_id.to_string()),
                    "target_character_id_ref":
                        event_ref_for_text(&new.target_character_id.to_string()),
                    "relationship_kind": relationship_kind,
                    "has_label": label.is_some(),
                    "has_notes": !notes.is_empty(),
                }),
            )
            .await?;
        Ok(row
            .ok_or_else(|| {
                AtelierError::Internal(
                    "creating a character relationship returned no row".to_owned(),
                )
            })?
            .into())
    }

    pub async fn get_character_relationship(
        &self,
        relationship_id: Uuid,
    ) -> AtelierResult<CharacterRelationship> {
        let bindings = RelationshipIdBinding {
            relationship_id: SurrealUuid::from(relationship_id),
        };
        let row: Option<CharacterRelationshipRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move { ctx.query_first(GET_RELATIONSHIP_STATEMENT, bindings).await })
            })
            .await?;
        row.map(CharacterRelationship::from).ok_or_else(|| {
            AtelierError::NotFound(format!("character relationship {relationship_id}"))
        })
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

        // Existence check first so a missing edge maps to NotFound before any
        // event is appended (the former transaction ordered its writes the
        // same way).
        self.get_character_relationship(relationship_id).await?;

        let bindings = UpdateRelationshipBindings {
            relationship_id: SurrealUuid::from(relationship_id),
            relationship_kind: relationship_kind.clone(),
            label: label.clone(),
            notes: notes.clone(),
        };
        let row: Option<CharacterRelationshipRow> = self
            .write_with_event(
                UPDATE_RELATIONSHIP_STATEMENT,
                bindings,
                relationships_event_family::CHARACTER_RELATIONSHIP_UPDATED,
                "atelier_character_relationship",
                &relationship_id.to_string(),
                serde_json::json!({
                    "relationship_id": relationship_id,
                    "relationship_kind": relationship_kind,
                    "has_label": label.is_some(),
                    "has_notes": !notes.is_empty(),
                }),
            )
            .await?;
        row.map(CharacterRelationship::from).ok_or_else(|| {
            AtelierError::NotFound(format!("character relationship {relationship_id}"))
        })
    }

    pub async fn delete_character_relationship(
        &self,
        relationship_id: Uuid,
    ) -> AtelierResult<CharacterRelationship> {
        // Pre-read for the event payload and the NotFound path.
        let existing = self.get_character_relationship(relationship_id).await?;

        let bindings = RelationshipIdBinding {
            relationship_id: SurrealUuid::from(relationship_id),
        };
        let row: Option<CharacterRelationshipRow> = self
            .write_with_event(
                DELETE_RELATIONSHIP_STATEMENT,
                bindings,
                relationships_event_family::CHARACTER_RELATIONSHIP_DELETED,
                "atelier_character_relationship",
                &relationship_id.to_string(),
                serde_json::json!({
                    "relationship_id": relationship_id,
                    "source_character_id_ref":
                        event_ref_for_text(&existing.source_character_id.to_string()),
                    "target_character_id_ref":
                        event_ref_for_text(&existing.target_character_id.to_string()),
                    "relationship_kind": existing.relationship_kind,
                }),
            )
            .await?;
        row.map(CharacterRelationship::from).ok_or_else(|| {
            AtelierError::NotFound(format!("character relationship {relationship_id}"))
        })
    }

    pub async fn list_character_relationships(
        &self,
        character_id: Uuid,
    ) -> AtelierResult<Vec<CharacterRelationship>> {
        self.require_character_endpoint("relationship list", character_id)
            .await?;
        let bindings = CharacterRefBinding {
            character_ref: RecordId::new("atelier_character", SurrealUuid::from(character_id)),
        };
        let rows: Vec<CharacterRelationshipRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(async move {
                    ctx.query_values(LIST_RELATIONSHIPS_STATEMENT, bindings)
                        .await
                })
            })
            .await?;
        Ok(rows.into_iter().map(CharacterRelationship::from).collect())
    }

    pub async fn character_relationship_graph(
        &self,
        anchor_character_id: Uuid,
    ) -> AtelierResult<CharacterRelationshipGraph> {
        self.require_character_endpoint("graph anchor", anchor_character_id)
            .await?;
        let edge_bindings = CharacterRefBinding {
            character_ref: RecordId::new(
                "atelier_character",
                SurrealUuid::from(anchor_character_id),
            ),
        };
        let edge_rows: Vec<GraphEdgeRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(
                    async move { ctx.query_values(GRAPH_EDGES_STATEMENT, edge_bindings).await },
                )
            })
            .await?;
        let edges: Vec<CharacterRelationshipGraphEdge> = edge_rows
            .into_iter()
            .map(|row| CharacterRelationshipGraphEdge {
                relationship_id: row.relationship_id.into(),
                source_character_id: row.source_character_id.into(),
                target_character_id: row.target_character_id.into(),
                relationship_kind: row.relationship_kind,
                label: row.label,
            })
            .collect();

        let mut endpoint_ids: Vec<SurrealUuid> = Vec::new();
        for edge in &edges {
            for endpoint in [edge.source_character_id, edge.target_character_id] {
                let endpoint = SurrealUuid::from(endpoint);
                if !endpoint_ids.contains(&endpoint) {
                    endpoint_ids.push(endpoint);
                }
            }
        }
        let node_bindings = GraphNodesBindings {
            anchor_id: SurrealUuid::from(anchor_character_id),
            endpoint_ids,
        };
        let node_rows: Vec<GraphNodeRow> = self
            .store()
            .with_data_operation(move |ctx| {
                Box::pin(
                    async move { ctx.query_values(GRAPH_NODES_STATEMENT, node_bindings).await },
                )
            })
            .await?;
        let nodes = node_rows
            .into_iter()
            .map(|row| CharacterRelationshipGraphNode {
                character_internal_id: row.internal_id.into(),
                public_id: row.public_id,
                display_name: row.display_name,
            })
            .collect();

        Ok(CharacterRelationshipGraph {
            anchor_character_id,
            nodes,
            edges,
        })
    }
}
