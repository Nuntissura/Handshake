//! WP-KERNEL-009 MT-144 RetrievalFixtures (builders).
//!
//! Deterministic, in-memory fixture builders for the retrieval pipeline's
//! pure-logic stages, covering the contract scenarios: exact lookup, graph
//! traversal, hybrid retrieval, passage fallback, stale graph, bad citation, and
//! no-result recovery. These build the inputs the ranking / budget / fallback
//! stages consume so a no-context model can exercise each mode without a live
//! cluster; the END-TO-END real-PG fixtures (index -> query -> assert trace +
//! ranked evidence + bundle) live in `tests/knowledge_retrieval_*.rs` and seed
//! the committed knowledge substrate directly.
//!
//! Keeping the builders next to the product logic lets both the unit tests here
//! and the integration tests reuse one fixture vocabulary.

use crate::knowledge_retrieval::budget::{BudgetItem, PriorityTier};
use crate::knowledge_retrieval::plan::RetrievalStore;
use crate::knowledge_retrieval::ranking::CandidateFeatures;

/// A named retrieval scenario (the contract's fixture set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalScenario {
    ExactLookup,
    GraphTraversal,
    HybridRetrieval,
    PassageFallback,
    StaleGraph,
    BadCitation,
    NoResultRecovery,
}

impl RetrievalScenario {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactLookup => "exact_lookup",
            Self::GraphTraversal => "graph_traversal",
            Self::HybridRetrieval => "hybrid_retrieval",
            Self::PassageFallback => "passage_fallback",
            Self::StaleGraph => "stale_graph",
            Self::BadCitation => "bad_citation",
            Self::NoResultRecovery => "no_result_recovery",
        }
    }

    /// All scenarios (for exhaustive fixture sweeps).
    pub fn all() -> [RetrievalScenario; 7] {
        [
            Self::ExactLookup,
            Self::GraphTraversal,
            Self::HybridRetrieval,
            Self::PassageFallback,
            Self::StaleGraph,
            Self::BadCitation,
            Self::NoResultRecovery,
        ]
    }
}

/// A deterministic candidate-feature fixture for ranking tests.
pub fn candidate_feature(
    id: &str,
    store: RetrievalStore,
    quality: f64,
    proximity: f64,
) -> CandidateFeatures {
    CandidateFeatures {
        candidate_id: id.to_string(),
        kind: "passage_ref".to_string(),
        store,
        evidence_quality: quality,
        graph_proximity: proximity,
        relationship_type_weight: 0.6,
        source_authority: 0.7,
        recency: 0.8,
        via_hub: false,
        lexical: Some(quality),
        vector: Some(proximity),
    }
}

/// A small ranked candidate set (3 items, descending quality) for budget tests.
pub fn ranked_feature_set() -> Vec<CandidateFeatures> {
    vec![
        candidate_feature("cand-a", RetrievalStore::KnowledgeGraph, 0.9, 0.9),
        candidate_feature("cand-b", RetrievalStore::ShadowWsLexical, 0.6, 0.4),
        candidate_feature("cand-c", RetrievalStore::ShadowWsVector, 0.3, 0.1),
    ]
}

/// A budget-item fixture set with mixed tiers and sources for allocator tests.
pub fn budget_item_set() -> Vec<BudgetItem> {
    vec![
        BudgetItem {
            item_id: "auth-1".to_string(),
            tier: PriorityTier::Authoritative,
            token_count: 200,
            source_id: "src-auth".to_string(),
        },
        BudgetItem {
            item_id: "prim-1".to_string(),
            tier: PriorityTier::Primary,
            token_count: 200,
            source_id: "src-1".to_string(),
        },
        BudgetItem {
            item_id: "supp-1".to_string(),
            tier: PriorityTier::Supplementary,
            token_count: 200,
            source_id: "src-1".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_retrieval::budget::allocate;
    use crate::knowledge_retrieval::plan::RetrievalBudgets;
    use crate::knowledge_retrieval::ranking::{rank_candidates, RankingWeights};

    #[test]
    fn all_scenarios_are_named() {
        assert_eq!(RetrievalScenario::all().len(), 7);
        assert_eq!(RetrievalScenario::ExactLookup.as_str(), "exact_lookup");
        assert_eq!(
            RetrievalScenario::NoResultRecovery.as_str(),
            "no_result_recovery"
        );
    }

    #[test]
    fn ranked_fixture_set_ranks_highest_quality_first() {
        let ranked = rank_candidates(ranked_feature_set(), &RankingWeights::default());
        assert_eq!(ranked[0].candidate_id, "cand-a");
        assert_eq!(ranked.last().unwrap().candidate_id, "cand-c");
    }

    #[test]
    fn budget_fixture_admits_authoritative_first_under_pressure() {
        let mut budgets = RetrievalBudgets::default_bounded();
        budgets.max_total_evidence_tokens = 200; // room for exactly one item
        let alloc = allocate(&budget_item_set(), &budgets);
        assert_eq!(alloc.admitted, vec!["auth-1".to_string()]);
    }
}
