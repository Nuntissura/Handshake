//! WP-KERNEL-009 MT-131 SchemaFirstFiltering.
//!
//! Spec 2.3.14.1.4 ("rich metadata enables filtering") + the cheapest-
//! authoritative rule: before a graph traversal or hybrid sweep widens the
//! candidate set, use the MemoryOntology / schema matches to NARROW candidate
//! facts and reduce off-topic graph expansion. A query that names known
//! ontology terms (or their aliases) should only expand the graph from entities
//! whose facts touch those terms.
//!
//! This is a backend, PostgreSQL-backed filter over the committed MemoryGraph:
//! it resolves query tokens to ontology terms (and aliases) via
//! `storage/knowledge_memory.rs`, then keeps only the memory facts whose
//! predicate term or entity object schema matches an in-scope term. The output
//! is the bounded candidate seed the GraphTraversalPlanner (MT-132) expands
//! from.
//!
//! No SQLite, no prompt-only helper text: the ontology terms and aliases are
//! authority rows; this filter is a deterministic read over them.

use std::collections::BTreeSet;

use sqlx::PgPool;

use crate::storage::knowledge_memory::{
    list_memory_facts, resolve_memory_ontology_alias, MemoryFact,
};
use crate::storage::StorageResult;

/// The result of schema-first filtering: the in-scope ontology term ids the
/// query resolved to, and the facts that survived the schema match. Empty
/// `matched_term_ids` means the query named no known schema, so the caller MUST
/// widen (graph/hybrid) rather than treat an empty filter as "no candidates".
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaFilterResult {
    pub matched_term_ids: BTreeSet<String>,
    pub candidate_facts: Vec<MemoryFact>,
    /// Facts dropped because their schema did not match any in-scope term.
    pub off_topic_dropped: usize,
}

impl SchemaFilterResult {
    /// Whether the query resolved to at least one known schema term. When
    /// false, schema-first filtering cannot narrow and the planner SHOULD fall
    /// through to a broader mode.
    pub fn has_schema_scope(&self) -> bool {
        !self.matched_term_ids.is_empty()
    }

    /// The subject entity ids of the surviving facts — the bounded seed set the
    /// graph-traversal planner expands from.
    pub fn seed_entity_ids(&self) -> BTreeSet<String> {
        let mut seeds = BTreeSet::new();
        for fact in &self.candidate_facts {
            seeds.insert(fact.subject_entity_id.clone());
            if let Some(obj) = &fact.object_entity_id {
                seeds.insert(obj.clone());
            }
        }
        seeds
    }
}

/// Tokenize a query into candidate alias keys. Deterministic: lowercase, split
/// on non-alphanumeric, drop empties and 1-char noise, keep 2- and 3-token
/// windows so phrase aliases ("kernel task run") resolve. The keys are matched
/// against `alias_norm_key`, so the normalization here MUST mirror however an
/// alias was normalized at write time (lowercase, space-joined).
pub fn query_term_candidates(query_text: &str) -> Vec<String> {
    let tokens: Vec<String> = query_text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(|t| t.to_lowercase())
        .collect();

    let mut candidates: BTreeSet<String> = tokens.iter().cloned().collect();
    for window in tokens.windows(2) {
        candidates.insert(window.join(" "));
    }
    for window in tokens.windows(3) {
        candidates.insert(window.join(" "));
    }
    candidates.into_iter().collect()
}

/// Resolve a query's tokens to in-scope ontology term ids by matching aliases.
/// Each candidate token/phrase is looked up through
/// `resolve_memory_ontology_alias`; the resolved term's `term_id` becomes an
/// in-scope id.
pub async fn resolve_query_schema(
    pool: &PgPool,
    workspace_id: &str,
    query_text: &str,
) -> StorageResult<BTreeSet<String>> {
    let mut term_ids = BTreeSet::new();
    for candidate in query_term_candidates(query_text) {
        if let Some(term) = resolve_memory_ontology_alias(pool, workspace_id, &candidate).await? {
            term_ids.insert(term.term_id);
        }
    }
    Ok(term_ids)
}

/// Keep only the facts whose predicate term or entity object is in schema scope.
/// A fact is in scope when its `predicate_term_id` is an in-scope term OR its
/// `object_entity_id` is an entity the in-scope set references. This is the
/// schema match that prevents off-topic graph expansion. With an empty scope it
/// keeps everything (the caller decides to widen) rather than silently dropping
/// all candidates.
pub fn filter_facts_by_schema(
    facts: Vec<MemoryFact>,
    in_scope_terms: &BTreeSet<String>,
) -> (Vec<MemoryFact>, usize) {
    if in_scope_terms.is_empty() {
        return (facts, 0);
    }
    let mut kept = Vec::with_capacity(facts.len());
    let mut dropped = 0usize;
    for fact in facts {
        let predicate_in_scope = fact
            .predicate_term_id
            .as_ref()
            .is_some_and(|t| in_scope_terms.contains(t));
        let object_in_scope = fact
            .object_entity_id
            .as_ref()
            .is_some_and(|e| in_scope_terms.contains(e));
        if predicate_in_scope || object_in_scope {
            kept.push(fact);
        } else {
            dropped += 1;
        }
    }
    (kept, dropped)
}

/// End-to-end schema-first filter: resolve the query's schema scope, load the
/// workspace facts, and narrow them. Bounded by `fact_limit`.
pub async fn schema_first_filter(
    pool: &PgPool,
    workspace_id: &str,
    query_text: &str,
    fact_limit: i64,
) -> StorageResult<SchemaFilterResult> {
    let matched_term_ids = resolve_query_schema(pool, workspace_id, query_text).await?;
    let facts = list_memory_facts(pool, workspace_id, fact_limit).await?;
    let (candidate_facts, off_topic_dropped) = filter_facts_by_schema(facts, &matched_term_ids);
    Ok(SchemaFilterResult {
        matched_term_ids,
        candidate_facts,
        off_topic_dropped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::knowledge_memory::MemoryClaimAuthorityLabel;
    use serde_json::json;

    #[test]
    fn query_tokens_include_unigrams_and_phrases() {
        let cands = query_term_candidates("Kernel Task Run id");
        assert!(cands.contains(&"kernel".to_string()));
        assert!(cands.contains(&"task".to_string()));
        assert!(cands.contains(&"kernel task".to_string()));
        assert!(cands.contains(&"kernel task run".to_string()));
        assert!(!cands.iter().any(|c| c.len() == 1));
    }

    #[test]
    fn empty_schema_scope_keeps_all_facts() {
        let facts = vec![sample_fact("f1", Some("pred-x"), None)];
        let scope = BTreeSet::new();
        let (kept, dropped) = filter_facts_by_schema(facts, &scope);
        assert_eq!(kept.len(), 1);
        assert_eq!(dropped, 0);
    }

    #[test]
    fn in_scope_predicate_is_kept_off_topic_dropped() {
        let mut scope = BTreeSet::new();
        scope.insert("pred-in".to_string());
        let facts = vec![
            sample_fact("f1", Some("pred-in"), None),
            sample_fact("f2", Some("pred-out"), None),
        ];
        let (kept, dropped) = filter_facts_by_schema(facts, &scope);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].fact_id, "f1");
        assert_eq!(dropped, 1);
    }

    #[test]
    fn in_scope_object_entity_is_kept() {
        let mut scope = BTreeSet::new();
        scope.insert("ent-in".to_string());
        let facts = vec![sample_fact("f1", Some("pred-out"), Some("ent-in"))];
        let (kept, dropped) = filter_facts_by_schema(facts, &scope);
        assert_eq!(kept.len(), 1);
        assert_eq!(dropped, 0);
    }

    #[test]
    fn seed_entity_ids_collects_subjects_and_objects() {
        let result = SchemaFilterResult {
            matched_term_ids: BTreeSet::from(["t".to_string()]),
            candidate_facts: vec![sample_fact("f1", Some("t"), Some("obj-1"))],
            off_topic_dropped: 0,
        };
        let seeds = result.seed_entity_ids();
        assert!(seeds.contains("subj"));
        assert!(seeds.contains("obj-1"));
    }

    fn sample_fact(
        id: &str,
        predicate_term_id: Option<&str>,
        object_entity_id: Option<&str>,
    ) -> MemoryFact {
        MemoryFact {
            fact_id: id.to_string(),
            workspace_id: "ws".to_string(),
            claim_id: "claim".to_string(),
            subject_entity_id: "subj".to_string(),
            predicate_key: "pk".to_string(),
            predicate_term_id: predicate_term_id.map(ToString::to_string),
            object_entity_id: object_entity_id.map(ToString::to_string),
            object_literal: None,
            qualifiers: json!({}),
            authority_label: MemoryClaimAuthorityLabel::Derived,
            extractor_version: "v1".to_string(),
            created_in_run: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}
