//! MT-161 (part 1) — InjectionScoringFormula integration tests.
//!
//! Inline tests in src/memory/scoring.rs cover deterministic + similarity
//! dominance + version pinning + clamp. This file adds adversarial /
//! cross-cutting integration tests for the public API.

use handshake_core::memory::scoring::{
    FormulaWeights, InjectionScoringFormula, ScoreInputs, INJECTION_SCORING_FORMULA_VERSION,
};

fn neutral() -> ScoreInputs {
    ScoreInputs {
        importance: 0.5,
        recency_age_secs: 86_400, // 1 day
        trust: 0.5,
        outcome_weight: 0.5,
        embedding_similarity: 0.5,
    }
}

#[test]
fn mt161_formula_version_constant_is_pinned() {
    assert_eq!(
        INJECTION_SCORING_FORMULA_VERSION,
        "injection_scoring_formula_v1",
        "version constant is consumed by RetrievalPolicy.scoring_formula_version; \
         silently bumping it would invalidate every persisted capsule's traceability"
    );
    assert_eq!(InjectionScoringFormula::version(), INJECTION_SCORING_FORMULA_VERSION);
}

#[test]
fn mt161_score_is_deterministic_across_invocations() {
    // Run 64 invocations against the same input and assert all produce
    // bit-identical scores. Catches accidental introduction of any
    // non-deterministic source (clock, RNG, system load proxy).
    let weights = FormulaWeights::default();
    let inputs = neutral();
    let baseline = InjectionScoringFormula::score(&inputs, &weights);
    for _ in 0..64 {
        assert_eq!(InjectionScoringFormula::score(&inputs, &weights), baseline);
    }
}

#[test]
fn mt161_score_is_bounded_zero_to_one() {
    let weights = FormulaWeights::default();
    // Sweep the input space with a small deterministic grid; every
    // produced score must be in [0.0, 1.0].
    for &imp in &[0.0, 0.25, 0.5, 0.75, 1.0] {
        for &age_secs in &[0u64, 3600, 86_400, 30 * 86_400, 365 * 86_400, u64::MAX / 4] {
            for &trust in &[0.0, 0.5, 1.0] {
                for &outcome in &[0.0, 0.5, 1.0] {
                    for &sim in &[0.0, 0.5, 1.0] {
                        let inputs = ScoreInputs {
                            importance: imp,
                            recency_age_secs: age_secs,
                            trust,
                            outcome_weight: outcome,
                            embedding_similarity: sim,
                        };
                        let s = InjectionScoringFormula::score(&inputs, &weights);
                        assert!(
                            (0.0..=1.0).contains(&s),
                            "score must be in [0,1]; got {s} for inputs={inputs:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn mt161_recent_item_outscores_old_item_when_other_axes_equal() {
    let weights = FormulaWeights::default();
    let recent = ScoreInputs {
        recency_age_secs: 60, // 1 minute
        ..neutral()
    };
    let ancient = ScoreInputs {
        recency_age_secs: 365 * 86_400, // 1 year
        ..neutral()
    };
    assert!(
        InjectionScoringFormula::score(&recent, &weights)
            > InjectionScoringFormula::score(&ancient, &weights),
        "recency tie-breaker must favor recent items when other axes are equal"
    );
}

#[test]
fn mt161_higher_trust_outscores_lower_trust_when_other_axes_equal() {
    let weights = FormulaWeights::default();
    let high = ScoreInputs {
        trust: 1.0,
        ..neutral()
    };
    let low = ScoreInputs {
        trust: 0.0,
        ..neutral()
    };
    assert!(
        InjectionScoringFormula::score(&high, &weights)
            > InjectionScoringFormula::score(&low, &weights)
    );
}

#[test]
fn mt161_higher_outcome_weight_outscores_lower_when_other_axes_equal() {
    let weights = FormulaWeights::default();
    let high = ScoreInputs {
        outcome_weight: 1.0,
        ..neutral()
    };
    let low = ScoreInputs {
        outcome_weight: 0.0,
        ..neutral()
    };
    assert!(
        InjectionScoringFormula::score(&high, &weights)
            > InjectionScoringFormula::score(&low, &weights)
    );
}

#[test]
fn mt161_higher_importance_outscores_lower_when_other_axes_equal() {
    let weights = FormulaWeights::default();
    let high = ScoreInputs {
        importance: 1.0,
        ..neutral()
    };
    let low = ScoreInputs {
        importance: 0.0,
        ..neutral()
    };
    assert!(
        InjectionScoringFormula::score(&high, &weights)
            > InjectionScoringFormula::score(&low, &weights)
    );
}

#[test]
fn mt161_negative_similarity_is_clamped_to_zero_not_panic() {
    // ScoreInputs.validated clamps embedding_similarity to [0,1]; a
    // negative value (which can arise from cosine similarity in [-1,1])
    // becomes 0.0, NOT a panic or NaN.
    let weights = FormulaWeights::default();
    let neg = ScoreInputs {
        embedding_similarity: -0.5,
        ..neutral()
    };
    let zero = ScoreInputs {
        embedding_similarity: 0.0,
        ..neutral()
    };
    let s_neg = InjectionScoringFormula::score(&neg, &weights);
    let s_zero = InjectionScoringFormula::score(&zero, &weights);
    assert!(s_neg.is_finite(), "score must be finite for negative similarity");
    assert_eq!(s_neg, s_zero, "negative similarity must clamp to 0.0");
}

#[test]
fn mt161_custom_weights_can_re_balance_the_formula() {
    // Demonstrate operator-tunable weights without changing version.
    let custom = FormulaWeights {
        similarity: 0.10,
        importance: 0.10,
        outcome: 0.10,
        recency: 0.60,
        trust: 0.10,
    };
    let recent = ScoreInputs {
        recency_age_secs: 60,
        ..neutral()
    };
    let ancient = ScoreInputs {
        recency_age_secs: 365 * 86_400,
        ..neutral()
    };
    let r_score = InjectionScoringFormula::score(&recent, &custom);
    let a_score = InjectionScoringFormula::score(&ancient, &custom);
    // Recency-weighted formula should widen the gap vs default weights.
    let default_r = InjectionScoringFormula::score(&recent, &FormulaWeights::default());
    let default_a = InjectionScoringFormula::score(&ancient, &FormulaWeights::default());
    let gap_custom = r_score - a_score;
    let gap_default = default_r - default_a;
    assert!(
        gap_custom > gap_default,
        "recency-weighted custom weights should widen the recent-vs-ancient gap; \
         got gap_custom={gap_custom}, gap_default={gap_default}"
    );
}

#[test]
fn mt161_score_inputs_serde_round_trip() {
    let inputs = neutral();
    let json = serde_json::to_string(&inputs).expect("ScoreInputs serializes");
    let back: ScoreInputs = serde_json::from_str(&json).expect("ScoreInputs deserializes");
    assert_eq!(inputs, back);
}

#[test]
fn mt161_formula_weights_serde_round_trip() {
    let w = FormulaWeights::default();
    let json = serde_json::to_string(&w).expect("FormulaWeights serializes");
    let back: FormulaWeights = serde_json::from_str(&json).expect("FormulaWeights deserializes");
    assert_eq!(w, back);
}
