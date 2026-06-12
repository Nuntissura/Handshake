//! WP-KERNEL-009 MT-134 StructureAwareRanking.
//!
//! Spec 2.6.6.7.14.6 ("score + select deterministically (stable tie-breakers)")
//! and 2.3.13.11 (MemoryPassage records ranking features). Rank retrieval
//! candidates using evidence quality, graph proximity, relationship type,
//! source authority, recency, and hub suppression — DETERMINISTICALLY, with a
//! stable tie-break so the same inputs always yield the same order (a
//! requirement for replayable RetrievalTraces).
//!
//! This is a pure, side-effect-free scoring function over the structured
//! features the upstream planners already gathered. It does not read the DB; it
//! consumes candidate features and produces a sorted `Vec<RetrievalCandidate>`
//! (spec object) the snippet assembler (MT-135) and bundle compiler (MT-136)
//! consume.

use crate::knowledge_retrieval::plan::{CandidateScores, RetrievalCandidate, RetrievalStore};

/// Tunable weights for the structure-aware score. Defaults are normalized so a
/// perfect candidate on every axis scores ~1.0. Held in one struct so the
/// ranking is explainable and a future policy can re-weight without code churn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankingWeights {
    pub evidence_quality: f64,
    pub graph_proximity: f64,
    pub relationship_type: f64,
    pub source_authority: f64,
    pub recency: f64,
    /// Penalty subtracted when the candidate came through a suppressed hub.
    pub hub_penalty: f64,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            evidence_quality: 0.30,
            graph_proximity: 0.20,
            relationship_type: 0.15,
            source_authority: 0.20,
            recency: 0.15,
            hub_penalty: 0.25,
        }
    }
}

/// The structured features of one candidate, all normalized to `[0, 1]` (except
/// `hub` which is a boolean). These are gathered by the upstream planners /
/// passage records; this struct is the ranking input.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateFeatures {
    pub candidate_id: String,
    pub kind: String,
    pub store: RetrievalStore,
    /// Evidence quality (e.g. extraction confidence of the backing passage).
    pub evidence_quality: f64,
    /// Graph proximity: 1.0 at the seed, decaying with hop distance.
    pub graph_proximity: f64,
    /// Relationship-type weight (e.g. `defines`/`implements` > `mentions`).
    pub relationship_type_weight: f64,
    /// Source authority (e.g. authoritative spec/packet vs advisory note).
    pub source_authority: f64,
    /// Recency in `[0, 1]`, 1.0 = freshest.
    pub recency: f64,
    /// Whether this candidate was reached through a suppressed hub.
    pub via_hub: bool,
    /// Optional component scores to surface in the trace candidate.
    pub lexical: Option<f64>,
    pub vector: Option<f64>,
}

impl CandidateFeatures {
    /// Map a relationship type string to a structural weight. Definitional /
    /// implementational edges carry more authority than loose mentions.
    pub fn relationship_weight_for(edge_type: &str) -> f64 {
        match edge_type {
            "defines" | "implements" | "validates" => 1.0,
            "documents" | "depends_on" | "contains" => 0.8,
            "references" | "derived_from" | "supersedes" => 0.6,
            "links_to" | "relates_to" => 0.4,
            "mentions" => 0.25,
            _ => 0.3,
        }
    }
}

/// Compute a candidate's deterministic base score from its features and the
/// weights. Clamped to `[0, 1]`. A via-hub candidate is penalized (hub
/// suppression) but never removed — it is recorded with a lower score.
pub fn score_candidate(features: &CandidateFeatures, weights: &RankingWeights) -> f64 {
    let mut score = weights.evidence_quality * features.evidence_quality
        + weights.graph_proximity * features.graph_proximity
        + weights.relationship_type * features.relationship_type_weight
        + weights.source_authority * features.source_authority
        + weights.recency * features.recency;
    if features.via_hub {
        score -= weights.hub_penalty;
    }
    score.clamp(0.0, 1.0)
}

/// Rank candidates highest-score-first with a STABLE tie-break: equal scores
/// order by `candidate_id` ascending. Returns spec `RetrievalCandidate` objects
/// with `base_score` and a `tiebreak` key set, ready for the trace.
pub fn rank_candidates(
    mut features: Vec<CandidateFeatures>,
    weights: &RankingWeights,
) -> Vec<RetrievalCandidate> {
    // Deterministic input order first (defends against unstable upstream order).
    features.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));

    let mut scored: Vec<(f64, RetrievalCandidate)> = features
        .into_iter()
        .map(|f| {
            let base_score = score_candidate(&f, weights);
            let graph = if f.graph_proximity > 0.0 {
                Some(f.graph_proximity)
            } else {
                None
            };
            let candidate = RetrievalCandidate {
                candidate_id: f.candidate_id.clone(),
                kind: f.kind,
                store: f.store,
                scores: CandidateScores {
                    lexical: f.lexical,
                    vector: f.vector,
                    graph,
                    pack: None,
                    trust_adjust: if f.via_hub { Some(-1.0) } else { None },
                },
                base_score,
                tiebreak: f.candidate_id,
            };
            (base_score, candidate)
        })
        .collect();

    // Highest score first; tie-break by candidate_id ascending (stable).
    scored.sort_by(|(sa, ca), (sb, cb)| {
        sb.partial_cmp(sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(ca.tiebreak.cmp(&cb.tiebreak))
    });

    scored.into_iter().map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feat(id: &str, quality: f64, proximity: f64, via_hub: bool) -> CandidateFeatures {
        CandidateFeatures {
            candidate_id: id.to_string(),
            kind: "passage_ref".to_string(),
            store: RetrievalStore::KnowledgeGraph,
            evidence_quality: quality,
            graph_proximity: proximity,
            relationship_type_weight: 0.6,
            source_authority: 0.7,
            recency: 0.8,
            via_hub,
            lexical: None,
            vector: None,
        }
    }

    #[test]
    fn relationship_weights_order_definitional_above_mentions() {
        assert!(
            CandidateFeatures::relationship_weight_for("defines")
                > CandidateFeatures::relationship_weight_for("mentions")
        );
        assert!(
            CandidateFeatures::relationship_weight_for("implements")
                > CandidateFeatures::relationship_weight_for("references")
        );
    }

    #[test]
    fn higher_quality_scores_higher() {
        let w = RankingWeights::default();
        let high = score_candidate(&feat("a", 0.9, 0.9, false), &w);
        let low = score_candidate(&feat("b", 0.1, 0.1, false), &w);
        assert!(high > low);
    }

    #[test]
    fn hub_candidate_is_penalized_but_present() {
        let w = RankingWeights::default();
        let normal = score_candidate(&feat("a", 0.8, 0.8, false), &w);
        let hub = score_candidate(&feat("a", 0.8, 0.8, true), &w);
        assert!(hub < normal);
        // Still present in the ranking (not removed).
        let ranked = rank_candidates(vec![feat("a", 0.8, 0.8, true)], &w);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].scores.trust_adjust, Some(-1.0));
    }

    #[test]
    fn ranking_is_deterministic_with_stable_tiebreak() {
        let w = RankingWeights::default();
        // Two identical-score candidates: order must be by candidate_id.
        let ranked = rank_candidates(
            vec![feat("z", 0.5, 0.5, false), feat("a", 0.5, 0.5, false)],
            &w,
        );
        assert_eq!(ranked[0].candidate_id, "a");
        assert_eq!(ranked[1].candidate_id, "z");
        // Re-run yields identical order.
        let again = rank_candidates(
            vec![feat("a", 0.5, 0.5, false), feat("z", 0.5, 0.5, false)],
            &w,
        );
        assert_eq!(
            ranked.iter().map(|c| &c.candidate_id).collect::<Vec<_>>(),
            again.iter().map(|c| &c.candidate_id).collect::<Vec<_>>()
        );
    }
}
