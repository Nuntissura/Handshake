//! WP-KERNEL-009 MT-133 PassageFallbackPlanner.
//!
//! Spec 2.3.13.11 (MemoryPassage as the bounded model-context unit) +
//! 2.6.6.7.14: when graph candidates are missing, stale, contradicted, or
//! low-confidence, retrieval MUST fall back to passage retrieval rather than
//! returning nothing or silently widening to an unbounded hybrid sweep. The
//! fallback is itself recorded (mode `passage_fallback`, persisted as
//! `hybrid_rag` with a passage-fallback reason) so an operator can see why the
//! cheaper graph path was abandoned.
//!
//! Inputs: the graph-traversal result (MT-132) and the workspace's passages
//! (committed `knowledge_memory_passages`, loaded via
//! `knowledge_memory::passage::load_passages_for_workspace`). The planner
//! decides, deterministically, whether to fall back and which passages seed the
//! fallback.

use crate::knowledge_retrieval::graph_planner::GraphTraversalResult;
use crate::storage::knowledge::KnowledgeMemoryPassage;

/// Why a passage fallback was triggered. Each maps to an actor-visible reason
/// recorded in the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PassageFallbackReason {
    /// The graph traversal produced no candidate edges.
    GraphCandidatesMissing,
    /// All graph candidates were below the confidence floor.
    GraphCandidatesLowConfidence,
    /// Graph candidates existed but their backing claims are contradicted.
    GraphCandidatesContradicted,
    /// The graph candidates are stale (index freshness could not be confirmed).
    GraphCandidatesStale,
}

impl PassageFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GraphCandidatesMissing => "graph_candidates_missing",
            Self::GraphCandidatesLowConfidence => "graph_candidates_low_confidence",
            Self::GraphCandidatesContradicted => "graph_candidates_contradicted",
            Self::GraphCandidatesStale => "graph_candidates_stale",
        }
    }
}

/// The decision: whether to fall back, why, and the passages that seed it.
#[derive(Debug, Clone, PartialEq)]
pub struct PassageFallbackDecision {
    pub fallback: bool,
    pub reason: Option<PassageFallbackReason>,
    pub fallback_passages: Vec<KnowledgeMemoryPassage>,
    /// Actor-visible explanation surfaced in the trace.
    pub rationale: String,
}

impl PassageFallbackDecision {
    fn no_fallback() -> Self {
        Self {
            fallback: false,
            reason: None,
            fallback_passages: Vec::new(),
            rationale: "graph candidates were sufficient; no passage fallback needed".to_string(),
        }
    }
}

/// Signals about the graph candidates the caller has already gathered, so the
/// planner can decide WHY a fallback is needed (not just that edges are empty).
#[derive(Debug, Clone, Copy, Default)]
pub struct GraphCandidateSignals {
    /// True if any backing claim of a graph candidate is contradicted.
    pub any_contradicted: bool,
    /// True if the index freshness for the candidates could not be confirmed.
    pub freshness_uncertain: bool,
    /// The max confidence across graph candidates (0.0 if none).
    pub max_confidence: f64,
}

/// The minimum confidence a graph candidate must reach to avoid a low-confidence
/// fallback.
pub const GRAPH_CONFIDENCE_FLOOR: f64 = 0.35;

/// Decide whether to fall back to passages. Precedence (cheapest-to-honor
/// first): contradiction > staleness > missing > low-confidence. A contradiction
/// is the strongest reason to abandon the graph path because the graph answer is
/// actively wrong, not merely absent.
pub fn decide_passage_fallback(
    graph: &GraphTraversalResult,
    signals: GraphCandidateSignals,
    available_passages: Vec<KnowledgeMemoryPassage>,
) -> PassageFallbackDecision {
    let reason = if signals.any_contradicted {
        Some(PassageFallbackReason::GraphCandidatesContradicted)
    } else if signals.freshness_uncertain && graph.has_candidates() {
        Some(PassageFallbackReason::GraphCandidatesStale)
    } else if !graph.has_candidates() {
        Some(PassageFallbackReason::GraphCandidatesMissing)
    } else if signals.max_confidence < GRAPH_CONFIDENCE_FLOOR {
        Some(PassageFallbackReason::GraphCandidatesLowConfidence)
    } else {
        None
    };

    let Some(reason) = reason else {
        return PassageFallbackDecision::no_fallback();
    };

    let rationale = format!(
        "falling back to passage retrieval: {} (graph edges={}, max_confidence={:.2})",
        reason.as_str(),
        graph.edges.len(),
        signals.max_confidence
    );
    PassageFallbackDecision {
        fallback: true,
        reason: Some(reason),
        fallback_passages: available_passages,
        rationale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge_retrieval::graph_planner::{GraphTraversalResult, TraversedEdge};
    use crate::storage::knowledge::KnowledgeEdgeType;

    fn graph_with_edges(n: usize) -> GraphTraversalResult {
        GraphTraversalResult {
            visited: vec![],
            edges: (0..n)
                .map(|i| TraversedEdge {
                    edge_id: format!("e{i}"),
                    relationship_id: format!("REL-{i}"),
                    edge_type: KnowledgeEdgeType::References,
                    source_entity_id: "a".to_string(),
                    target_entity_id: "b".to_string(),
                    depth: 1,
                    confidence: 0.9,
                })
                .collect(),
            suppressed_hubs: vec![],
            stop_reason: "x".to_string(),
            seeds: vec![],
        }
    }

    #[test]
    fn missing_graph_candidates_triggers_fallback() {
        let decision = decide_passage_fallback(
            &graph_with_edges(0),
            GraphCandidateSignals::default(),
            vec![],
        );
        assert!(decision.fallback);
        assert_eq!(
            decision.reason,
            Some(PassageFallbackReason::GraphCandidatesMissing)
        );
    }

    #[test]
    fn sufficient_high_confidence_graph_does_not_fall_back() {
        let decision = decide_passage_fallback(
            &graph_with_edges(3),
            GraphCandidateSignals {
                any_contradicted: false,
                freshness_uncertain: false,
                max_confidence: 0.9,
            },
            vec![],
        );
        assert!(!decision.fallback);
        assert!(decision.reason.is_none());
    }

    #[test]
    fn contradiction_takes_precedence_over_presence() {
        let decision = decide_passage_fallback(
            &graph_with_edges(5),
            GraphCandidateSignals {
                any_contradicted: true,
                freshness_uncertain: false,
                max_confidence: 0.99,
            },
            vec![],
        );
        assert!(decision.fallback);
        assert_eq!(
            decision.reason,
            Some(PassageFallbackReason::GraphCandidatesContradicted)
        );
    }

    #[test]
    fn low_confidence_present_graph_falls_back() {
        let decision = decide_passage_fallback(
            &graph_with_edges(2),
            GraphCandidateSignals {
                any_contradicted: false,
                freshness_uncertain: false,
                max_confidence: 0.1,
            },
            vec![],
        );
        assert!(decision.fallback);
        assert_eq!(
            decision.reason,
            Some(PassageFallbackReason::GraphCandidatesLowConfidence)
        );
    }
}
