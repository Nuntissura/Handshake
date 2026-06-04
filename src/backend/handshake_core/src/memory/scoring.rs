//! MT-161 (part 1): InjectionScoringFormula — deterministic scoring formula
//! for MemoryPack item ranking.
//!
//! Formula: linear combination of (importance, recency, trust,
//! outcome-tuned, embedding-similarity-to-query). Deterministic given
//! fixed inputs.

use serde::{Deserialize, Serialize};

/// Stable id pinned in the RetrievalPolicy.scoring_formula_version field
/// so the policy can declare which formula version a capsule was built
/// under. Bump on any change to the math below.
pub const INJECTION_SCORING_FORMULA_VERSION: &str = "injection_scoring_formula_v1";

/// Inputs to the scoring formula. All in [0.0, 1.0] except recency_age_secs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreInputs {
    /// Operator-set importance weight in [0, 1].
    pub importance: f64,
    /// Item recorded-age in seconds.
    pub recency_age_secs: u64,
    /// Per-source provenance trust score in [0, 1].
    pub trust: f64,
    /// Outcome-tuned weight from MT-158, in [0, 1].
    pub outcome_weight: f64,
    /// Embedding cosine similarity to query, in [-1, 1] but clamped here.
    pub embedding_similarity: f64,
}

impl ScoreInputs {
    pub fn validated(self) -> Self {
        Self {
            importance: clamp(self.importance, 0.0, 1.0),
            recency_age_secs: self.recency_age_secs,
            trust: clamp(self.trust, 0.0, 1.0),
            outcome_weight: clamp(self.outcome_weight, 0.0, 1.0),
            embedding_similarity: clamp(self.embedding_similarity, 0.0, 1.0),
        }
    }
}

fn clamp(value: f64, lo: f64, hi: f64) -> f64 {
    if value < lo {
        lo
    } else if value > hi {
        hi
    } else {
        value
    }
}

/// Weights per term. Default values were chosen so similarity dominates
/// (semantic match), importance + outcome are secondary, and recency +
/// trust are tie-breakers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FormulaWeights {
    pub similarity: f64,
    pub importance: f64,
    pub outcome: f64,
    pub recency: f64,
    pub trust: f64,
}

impl Default for FormulaWeights {
    fn default() -> Self {
        Self {
            similarity: 0.40,
            importance: 0.20,
            outcome: 0.20,
            recency: 0.10,
            trust: 0.10,
        }
    }
}

/// Recency decay constant: e^(-age / TAU). Default 30 days.
const RECENCY_TAU_SECS: f64 = 30.0 * 86_400.0;

pub struct InjectionScoringFormula;

impl InjectionScoringFormula {
    pub fn version() -> &'static str {
        INJECTION_SCORING_FORMULA_VERSION
    }

    /// Compute final score in [0, 1] for an item.
    pub fn score(inputs: &ScoreInputs, weights: &FormulaWeights) -> f64 {
        let inputs = inputs.validated();
        let recency = (-(inputs.recency_age_secs as f64) / RECENCY_TAU_SECS).exp();
        let weighted = weights.similarity * inputs.embedding_similarity
            + weights.importance * inputs.importance
            + weights.outcome * inputs.outcome_weight
            + weights.recency * recency
            + weights.trust * inputs.trust;
        // Final clamp; the weights nominally sum to 1.0 but we don't
        // require it.
        clamp(weighted, 0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_is_deterministic() {
        let inputs = ScoreInputs {
            importance: 0.5,
            recency_age_secs: 3600,
            trust: 0.8,
            outcome_weight: 0.6,
            embedding_similarity: 0.7,
        };
        let weights = FormulaWeights::default();
        let a = InjectionScoringFormula::score(&inputs, &weights);
        let b = InjectionScoringFormula::score(&inputs, &weights);
        assert_eq!(a, b);
        assert!(a >= 0.0 && a <= 1.0);
    }

    #[test]
    fn high_similarity_dominates() {
        let weights = FormulaWeights::default();
        let high = ScoreInputs {
            importance: 0.0,
            recency_age_secs: u64::MAX / 2,
            trust: 0.0,
            outcome_weight: 0.0,
            embedding_similarity: 1.0,
        };
        let low = ScoreInputs {
            importance: 0.0,
            recency_age_secs: u64::MAX / 2,
            trust: 0.0,
            outcome_weight: 0.0,
            embedding_similarity: 0.0,
        };
        assert!(
            InjectionScoringFormula::score(&high, &weights)
                > InjectionScoringFormula::score(&low, &weights)
        );
    }

    #[test]
    fn formula_version_constant_stable() {
        assert_eq!(
            InjectionScoringFormula::version(),
            "injection_scoring_formula_v1"
        );
    }

    #[test]
    fn out_of_range_inputs_clamp() {
        let weights = FormulaWeights::default();
        let bad = ScoreInputs {
            importance: 5.0,
            recency_age_secs: 0,
            trust: -1.0,
            outcome_weight: 1.5,
            embedding_similarity: 2.0,
        };
        let score = InjectionScoringFormula::score(&bad, &weights);
        assert!(score >= 0.0 && score <= 1.0);
    }
}
