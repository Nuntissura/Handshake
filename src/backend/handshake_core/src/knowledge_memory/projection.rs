//! MT-116 OntologyGraphProjection, MT-117 FactGraphProjection,
//! MT-118 PassageEvidenceGraphProjection.
//!
//! These are READ-ONLY graph projections built from authority rows. A
//! projection is NEVER authority (spec 2.3.13.11): deleting or recomputing one
//! does not mutate any authority record, and a projection only carries stable
//! refs back into authority (term ids, entity ids, claim ids, fact ids, span
//! ids, source ids, passage ids). Every node and edge in these projections
//! cites the authority id it was derived from, so a no-context model can always
//! jump back to the evidence.
//!
//! Each projection is a pure function of the current authority state in one
//! workspace, bounded by a `limit`. They reuse the committed substrate
//! (entities/edges/claims/spans/passages) and the MemoryGraph tables
//! (ontology terms/aliases, facts) — they do not re-extract or re-parse.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::storage::knowledge::{KnowledgeClaimState, KnowledgeMemoryPassage, KnowledgeStore};
use crate::storage::knowledge_memory::{
    list_memory_facts, list_memory_ontology_aliases, list_memory_ontology_terms, MemoryFact,
    MemoryOntologyAlias, MemoryOntologyLifecycle, MemoryOntologyTerm,
};
use crate::storage::postgres::PostgresDatabase;
use crate::storage::StorageResult;

use super::passage::load_passages_for_workspace;

// ---------------------------------------------------------------------------
// MT-116 OntologyGraphProjection
// ---------------------------------------------------------------------------

/// A node in the ontology graph: one ontology term, with stable ref back to
/// the authority `term_id`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OntologyGraphNode {
    pub term_id: String,
    pub term_kind: String,
    pub term_key: String,
    pub normalized_label: String,
    pub lifecycle_state: String,
    pub observation_count: i32,
    pub aliases: Vec<String>,
}

/// An edge in the ontology graph: a `supersedes` relation between two terms
/// (from a retired term's `superseded_by_term_id`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OntologyGraphEdge {
    pub from_term_id: String,
    pub to_term_id: String,
    pub relation: String,
}

/// The ontology graph projection for a workspace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OntologyGraphProjection {
    pub workspace_id: String,
    pub nodes: Vec<OntologyGraphNode>,
    pub edges: Vec<OntologyGraphEdge>,
    /// Marks this payload as a projection, never authority (spec 2.3.13.11).
    pub authority_class: &'static str,
}

/// Build the ontology graph projection. `stable_only` restricts to terms that
/// have been promoted to stable retrieval ontology.
pub async fn build_ontology_graph(
    pool: &PgPool,
    workspace_id: &str,
    stable_only: bool,
    limit: i64,
) -> StorageResult<OntologyGraphProjection> {
    let lifecycle_filter = stable_only.then_some(MemoryOntologyLifecycle::Stable);
    let terms =
        list_memory_ontology_terms(pool, workspace_id, None, lifecycle_filter, limit).await?;

    let mut nodes = Vec::with_capacity(terms.len());
    let mut edges = Vec::new();
    for term in &terms {
        let aliases: Vec<String> = list_memory_ontology_aliases(pool, &term.term_id)
            .await?
            .into_iter()
            .map(|alias: MemoryOntologyAlias| alias.alias_surface)
            .collect();
        nodes.push(ontology_node(term, aliases));
        if let Some(superseded_by) = &term.superseded_by_term_id {
            edges.push(OntologyGraphEdge {
                from_term_id: superseded_by.clone(),
                to_term_id: term.term_id.clone(),
                relation: "supersedes".to_string(),
            });
        }
    }

    Ok(OntologyGraphProjection {
        workspace_id: workspace_id.to_string(),
        nodes,
        edges,
        authority_class: "projection",
    })
}

fn ontology_node(term: &MemoryOntologyTerm, aliases: Vec<String>) -> OntologyGraphNode {
    OntologyGraphNode {
        term_id: term.term_id.clone(),
        term_kind: term.term_kind.as_str().to_string(),
        term_key: term.term_key.clone(),
        normalized_label: term.normalized_label.clone(),
        lifecycle_state: term.lifecycle_state.as_str().to_string(),
        observation_count: term.observation_count,
        aliases,
    }
}

// ---------------------------------------------------------------------------
// MT-117 FactGraphProjection
// ---------------------------------------------------------------------------

/// A node in the fact graph: a knowledge entity that participates as a fact
/// subject or object. Stable ref back to `entity_id`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactGraphNode {
    pub entity_id: String,
    pub display_name: String,
    pub entity_kind: String,
}

/// An edge in the fact graph: one memory fact rendered as
/// subject --predicate--> object, carrying the backing claim id, the fact's
/// authority label, and (for literal-object facts) the literal value as the
/// target label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactGraphEdge {
    pub fact_id: String,
    pub claim_id: String,
    pub subject_entity_id: String,
    pub predicate_key: String,
    /// Present for relationship facts (entity object); None for attribute facts.
    pub object_entity_id: Option<String>,
    /// Present for attribute facts (literal object); None for relationship facts.
    pub object_literal: Option<String>,
    pub authority_label: String,
}

/// The fact graph projection for a workspace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactGraphProjection {
    pub workspace_id: String,
    pub nodes: Vec<FactGraphNode>,
    pub edges: Vec<FactGraphEdge>,
    pub authority_class: &'static str,
}

/// Build the fact graph projection from stable facts and the entities they
/// connect. When `trusted_only` is set, a fact must have both a
/// retrieval-trusted authority label and an accepted backing claim. Deprecated,
/// superseded, unsupported, bare model-suggested, proposed, and conflicted
/// facts are excluded from the stable fact graph.
pub async fn build_fact_graph(
    db: &PostgresDatabase,
    pool: &PgPool,
    workspace_id: &str,
    trusted_only: bool,
    limit: i64,
) -> StorageResult<FactGraphProjection> {
    let facts = list_memory_facts(pool, workspace_id, limit).await?;

    let mut nodes: Vec<FactGraphNode> = Vec::new();
    let mut seen_entities = std::collections::HashSet::new();
    let mut edges = Vec::new();

    for fact in &facts {
        if trusted_only {
            if !super::claim_labels::is_retrieval_trusted(fact.authority_label) {
                continue;
            }
            let Some(claim) = db.get_knowledge_claim(&fact.claim_id).await? else {
                continue;
            };
            if claim.lifecycle_state != KnowledgeClaimState::Accepted {
                continue;
            }
            if claim_has_unresolved_conflict(pool, &fact.claim_id).await? {
                continue;
            }
        }
        push_entity_node(db, fact, &mut nodes, &mut seen_entities).await?;
        edges.push(fact_edge(fact));
    }

    Ok(FactGraphProjection {
        workspace_id: workspace_id.to_string(),
        nodes,
        edges,
        authority_class: "projection",
    })
}

async fn push_entity_node(
    db: &PostgresDatabase,
    fact: &MemoryFact,
    nodes: &mut Vec<FactGraphNode>,
    seen: &mut std::collections::HashSet<String>,
) -> StorageResult<()> {
    let mut entity_ids = vec![fact.subject_entity_id.clone()];
    if let Some(object_entity_id) = &fact.object_entity_id {
        entity_ids.push(object_entity_id.clone());
    }
    for entity_id in entity_ids {
        if !seen.insert(entity_id.clone()) {
            continue;
        }
        if let Some(entity) = db.get_knowledge_entity(&entity_id).await? {
            nodes.push(FactGraphNode {
                entity_id: entity.entity_id,
                display_name: entity.display_name,
                entity_kind: entity.entity_kind.as_str().to_string(),
            });
        }
    }
    Ok(())
}

fn fact_edge(fact: &MemoryFact) -> FactGraphEdge {
    FactGraphEdge {
        fact_id: fact.fact_id.clone(),
        claim_id: fact.claim_id.clone(),
        subject_entity_id: fact.subject_entity_id.clone(),
        predicate_key: fact.predicate_key.clone(),
        object_entity_id: fact.object_entity_id.clone(),
        object_literal: fact.object_literal.clone(),
        authority_label: fact.authority_label.as_str().to_string(),
    }
}

async fn claim_has_unresolved_conflict(pool: &PgPool, claim_id: &str) -> StorageResult<bool> {
    let unresolved: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM knowledge_claim_conflicts
            WHERE resolved_at IS NULL
              AND (claim_id = $1 OR conflicting_claim_id = $1)
        )
        "#,
    )
    .bind(claim_id)
    .fetch_one(pool)
    .await?;
    Ok(unresolved)
}

// ---------------------------------------------------------------------------
// MT-118 PassageEvidenceGraphProjection
// ---------------------------------------------------------------------------

/// A passage node in the evidence graph, with stable ref back to `passage_id`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PassageEvidenceNode {
    pub passage_id: String,
    pub retrieval_mode: String,
    pub compaction_policy: String,
    pub token_count: Option<i32>,
}

/// An evidence edge: passage --derived_from--> {source|claim|span}. The typed
/// `ref_kind` plus the target authority id let a consumer walk from a passage
/// to every source/claim/span it was derived from (and onward to facts, since
/// facts back claims).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PassageEvidenceEdge {
    pub passage_id: String,
    pub ref_kind: String,
    pub target_id: String,
}

/// The passage/evidence graph projection for a workspace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PassageEvidenceGraphProjection {
    pub workspace_id: String,
    pub nodes: Vec<PassageEvidenceNode>,
    pub edges: Vec<PassageEvidenceEdge>,
    pub authority_class: &'static str,
}

/// Build the passage/evidence graph projection: every passage in the workspace
/// and its derivation lineage (source/claim/span refs). Reuses the committed
/// passage evidence store.
pub async fn build_passage_evidence_graph(
    db: &PostgresDatabase,
    pool: &PgPool,
    workspace_id: &str,
    limit: i64,
) -> StorageResult<PassageEvidenceGraphProjection> {
    let passages = load_passages_for_workspace(pool, workspace_id, limit).await?;

    let mut nodes = Vec::with_capacity(passages.len());
    let mut edges = Vec::new();
    for passage in &passages {
        nodes.push(passage_node(passage));
        let evidence = db
            .list_knowledge_passage_evidence(&passage.passage_id)
            .await?;
        for reference in evidence {
            let (ref_kind, target_id) = evidence_ref_parts(&reference);
            edges.push(PassageEvidenceEdge {
                passage_id: passage.passage_id.clone(),
                ref_kind: ref_kind.to_string(),
                target_id,
            });
        }
    }

    Ok(PassageEvidenceGraphProjection {
        workspace_id: workspace_id.to_string(),
        nodes,
        edges,
        authority_class: "projection",
    })
}

fn passage_node(passage: &KnowledgeMemoryPassage) -> PassageEvidenceNode {
    PassageEvidenceNode {
        passage_id: passage.passage_id.clone(),
        retrieval_mode: passage.retrieval_mode.as_str().to_string(),
        compaction_policy: passage.compaction_policy.as_str().to_string(),
        token_count: passage.token_count,
    }
}

fn evidence_ref_parts(
    reference: &crate::storage::knowledge::KnowledgePassageEvidenceRef,
) -> (&'static str, String) {
    use crate::storage::knowledge::KnowledgePassageEvidenceRef as Ref;
    match reference {
        Ref::Source { source_id } => ("source", source_id.clone()),
        Ref::Claim { claim_id } => ("claim", claim_id.clone()),
        Ref::Span { span_id } => ("span", span_id.clone()),
    }
}
