//! MT-152 — MultiMetricPromotionFloor integration tests.
//!
//! Per the MT-152 contract proof_command:
//!   `cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target promotion_floor_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/self_improve/promotion_floor.rs
//! already covers happy-path approval, single-reason rejection per metric,
//! multi-reason rejection, and floor purity. This integration test file
//! satisfies the contract owned_files entry and adds the cross-cutting
//! adversarial / external-API scenarios required by the MT-152 red_team
//! minimum_controls + workflow brief:
//!
//!   - dev PASS up gate enforced (strictly improves; equal rejected).
//!   - latency-not-regressed gate enforced (per-tolerance threshold).
//!   - capsule-bytes-not-regressed gate enforced (per-tolerance threshold).
//!   - holdout-PASS-not-regressed gate enforced (per-tolerance pp; 0.0 default).
//!   - multi-metric AND logic: any one false -> reject; all four pass -> approve.
//!   - per-metric TYPED rejection (operator can see which gate failed
//!     by typed FloorReason discriminant; serde round-trip surfaces the
//!     same shape so the IPC consumer can render it).
//!   - tolerances are durable typed state (no magic numbers; defaults are
//!     stable across construction).
//!   - boundary semantics: exactly-at-tolerance vs over-tolerance, baseline
//!     zero (degenerate divide-by-zero handled), holdout pp tolerance honored.

use chrono::Utc;
use handshake_core::self_improve::evaluator::SplitMetrics;
use handshake_core::self_improve::{
    EvalResult, FloorReason, MetricDelta, MultiMetricPromotionFloor, PromotionDecision,
    PromotionTolerances,
};

/// Build a `SplitMetrics` with the four fields the floor inspects.
fn split(pass_rate: f64, latency_ms: u64, capsule_bytes: u64) -> SplitMetrics {
    SplitMetrics {
        pass_rate,
        pass_count: (pass_rate * 100.0).round() as u32,
        total_count: 100,
        latency_p95_ms: latency_ms,
        capsule_bytes_p95: capsule_bytes,
        per_item_results: Vec::new(),
    }
}

/// Build a full `EvalResult` (train ignored by floor; only dev + holdout used).
fn eval(dev: SplitMetrics, holdout: SplitMetrics) -> EvalResult {
    EvalResult {
        train: SplitMetrics::empty(),
        dev,
        holdout,
        evaluated_at_utc: Utc::now(),
        snapshot_hash: "0".repeat(64),
    }
}

/// Helper: assert the decision carries exactly one FloorReason of the
/// expected discriminant. Returns the typed reason so the test can poke
/// at its fields.
fn unwrap_single_reason(decision: PromotionDecision) -> FloorReason {
    match decision {
        PromotionDecision::Rejected { mut reasons, .. } => {
            assert_eq!(
                reasons.len(),
                1,
                "expected exactly one reason, got {reasons:?}"
            );
            reasons.remove(0)
        }
        PromotionDecision::Approved { .. } => panic!("expected Rejected, got Approved"),
    }
}

// ============================================================================
// dev PASS up gate
// ============================================================================

#[test]
fn dev_pass_strictly_up_required_equal_rate_rejects() {
    // Equal dev pass rate is not strictly improving; must reject.
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    match reason {
        FloorReason::DevPassRateNotImproved {
            baseline_pass_rate,
            trial_pass_rate,
        } => {
            assert!((baseline_pass_rate - 0.60).abs() < 1e-9);
            assert!((trial_pass_rate - 0.60).abs() < 1e-9);
        }
        other => panic!("expected DevPassRateNotImproved, got {other:?}"),
    }
}

#[test]
fn dev_pass_strictly_up_required_decreasing_rate_rejects() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.55, 100, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let decision = floor.evaluate(&baseline, &trial);
    match decision {
        PromotionDecision::Rejected { reasons, .. } => assert!(
            reasons
                .iter()
                .any(|r| matches!(r, FloorReason::DevPassRateNotImproved { .. })),
            "expected DevPassRateNotImproved among reasons: {reasons:?}"
        ),
        PromotionDecision::Approved { .. } => panic!("expected Rejected"),
    }
}

#[test]
fn dev_pass_tiny_improvement_passes_dev_gate() {
    // 1e-9 improvement still strictly > baseline; passes the dev gate
    // (other gates remain neutral so the decision is Approved).
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.60 + 1e-9, 100, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    assert!(
        floor.evaluate(&baseline, &trial).is_approved(),
        "tiny positive delta should pass dev gate"
    );
}

// ============================================================================
// latency-not-regressed gate
// ============================================================================

#[test]
fn latency_exactly_at_tolerance_does_not_reject() {
    // 5% regression == default tolerance; the gate uses strict `>` so
    // exactly-at-tolerance is accepted.
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.70, 105, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    assert!(
        floor.evaluate(&baseline, &trial).is_approved(),
        "exactly-at-tolerance must not reject"
    );
}

#[test]
fn latency_one_unit_over_tolerance_rejects_with_typed_reason() {
    // 6% regression > 5% tolerance.
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.70, 106, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    match reason {
        FloorReason::LatencyP95Regressed {
            baseline_ms,
            trial_ms,
            delta_pct,
        } => {
            assert_eq!(baseline_ms, 100);
            assert_eq!(trial_ms, 106);
            assert!(delta_pct > 5.0 && delta_pct < 7.0);
        }
        other => panic!("expected LatencyP95Regressed, got {other:?}"),
    }
}

#[test]
fn latency_baseline_zero_any_positive_trial_rejects_with_finite_delta() {
    // Baseline 0 -> any positive trial latency is a hard regression, but
    // operator evidence must remain JSON-safe and finite.
    let baseline = eval(split(0.50, 0, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.60, 10, 10_000), split(0.50, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let decision = floor.evaluate(&baseline, &trial);
    match decision {
        PromotionDecision::Rejected { reasons, .. } => {
            let lat = reasons
                .iter()
                .find_map(|r| match r {
                    FloorReason::LatencyP95Regressed {
                        baseline_ms,
                        trial_ms,
                        delta_pct,
                    } => Some((*baseline_ms, *trial_ms, *delta_pct)),
                    _ => None,
                })
                .expect("LatencyP95Regressed reason");
            assert_eq!(lat.0, 0);
            assert_eq!(lat.1, 10);
            assert!(lat.2.is_finite(), "delta_pct must be finite: {lat:?}");
            assert!(
                lat.2 > floor.tolerances.latency_p95_regression_tolerance_pct,
                "zero-baseline latency regression must still exceed tolerance"
            );
            let json = serde_json::to_value(&reasons).expect("serialize reasons");
            assert!(
                json.to_string().contains("\"delta_pct\":"),
                "operator evidence should include a numeric delta_pct"
            );
            assert!(
                !json.to_string().contains("\"delta_pct\":null"),
                "non-finite delta_pct serializes to null and is not allowed"
            );
        }
        PromotionDecision::Approved { .. } => panic!("expected Rejected"),
    }
}

#[test]
fn latency_baseline_and_trial_both_zero_does_not_reject_on_latency() {
    let baseline = eval(split(0.50, 0, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.60, 0, 10_000), split(0.50, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let decision = floor.evaluate(&baseline, &trial);
    if let PromotionDecision::Rejected { reasons, .. } = &decision {
        for r in reasons {
            if matches!(r, FloorReason::LatencyP95Regressed { .. }) {
                panic!("0->0 latency must not be a regression: {r:?}");
            }
        }
    }
    assert!(decision.is_approved(), "all gates should pass");
}

#[test]
fn latency_improvement_under_baseline_is_approved() {
    let baseline = eval(split(0.60, 200, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.70, 100, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let decision = floor.evaluate(&baseline, &trial);
    assert!(decision.is_approved());
    if let PromotionDecision::Approved { delta } = decision {
        assert_eq!(delta.latency_p95_delta_ms, -100);
    } else {
        panic!("expected Approved");
    }
}

// ============================================================================
// capsule-bytes-not-regressed gate
// ============================================================================

#[test]
fn capsule_bytes_exactly_at_tolerance_does_not_reject() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.70, 100, 10_500), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    assert!(floor.evaluate(&baseline, &trial).is_approved());
}

#[test]
fn capsule_bytes_one_unit_over_tolerance_rejects_with_typed_reason() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.70, 100, 10_501), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    match reason {
        FloorReason::CapsuleBytesRegressed {
            baseline_bytes,
            trial_bytes,
            delta_pct,
        } => {
            assert_eq!(baseline_bytes, 10_000);
            assert_eq!(trial_bytes, 10_501);
            assert!(delta_pct > 5.0 && delta_pct < 6.0);
        }
        other => panic!("expected CapsuleBytesRegressed, got {other:?}"),
    }
}

#[test]
fn capsule_bytes_baseline_zero_any_positive_trial_rejects_with_finite_delta() {
    let baseline = eval(split(0.50, 100, 0), split(0.50, 100, 10_000));
    let trial = eval(split(0.60, 100, 1_000), split(0.50, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    match reason {
        FloorReason::CapsuleBytesRegressed {
            baseline_bytes,
            trial_bytes,
            delta_pct,
        } => {
            assert_eq!(baseline_bytes, 0);
            assert_eq!(trial_bytes, 1_000);
            assert!(delta_pct.is_finite(), "delta_pct must be finite");
            assert!(
                delta_pct > floor.tolerances.capsule_bytes_p95_regression_tolerance_pct,
                "zero-baseline capsule regression must still exceed tolerance"
            );
            let json = serde_json::to_value(&FloorReason::CapsuleBytesRegressed {
                baseline_bytes,
                trial_bytes,
                delta_pct,
            })
            .expect("serialize reason");
            assert!(
                json.get("delta_pct").and_then(|v| v.as_f64()).is_some(),
                "operator evidence should include a numeric delta_pct"
            );
        }
        other => panic!("expected CapsuleBytesRegressed, got {other:?}"),
    }
}

// ============================================================================
// holdout-PASS-not-regressed gate
// ============================================================================

#[test]
fn holdout_default_tolerance_zero_pp_any_regression_rejects() {
    // Default holdout tolerance is 0.0 pp -> any negative trial pp rejects.
    let baseline = eval(split(0.70, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.80, 100, 10_000), split(0.59, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    match reason {
        FloorReason::HoldoutPassRegressed {
            baseline_pass_rate,
            trial_pass_rate,
            delta_pp,
        } => {
            assert!((baseline_pass_rate - 0.60).abs() < 1e-9);
            assert!((trial_pass_rate - 0.59).abs() < 1e-9);
            assert!((delta_pp - 0.01).abs() < 1e-9);
        }
        other => panic!("expected HoldoutPassRegressed, got {other:?}"),
    }
}

#[test]
fn holdout_custom_positive_tolerance_pp_accepts_small_regression() {
    // With a custom 2 pp tolerance, 1 pp regression is accepted.
    let tolerances = PromotionTolerances {
        latency_p95_regression_tolerance_pct: 5.0,
        capsule_bytes_p95_regression_tolerance_pct: 5.0,
        holdout_pass_regression_tolerance_pp: 0.02,
    };
    let floor = MultiMetricPromotionFloor::new(tolerances);
    let baseline = eval(split(0.70, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.80, 100, 10_000), split(0.59, 100, 10_000));
    assert!(
        floor.evaluate(&baseline, &trial).is_approved(),
        "1pp regression should be inside 2pp tolerance"
    );
}

#[test]
fn holdout_equal_pass_rate_does_not_reject() {
    let baseline = eval(split(0.70, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.80, 100, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    assert!(floor.evaluate(&baseline, &trial).is_approved());
}

#[test]
fn holdout_improvement_is_approved() {
    let baseline = eval(split(0.70, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.80, 100, 10_000), split(0.70, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let decision = floor.evaluate(&baseline, &trial);
    assert!(decision.is_approved());
    if let PromotionDecision::Approved { delta } = decision {
        assert!((delta.holdout_pass_delta_pp - 0.10).abs() < 1e-9);
    } else {
        panic!("expected Approved");
    }
}

// ============================================================================
// AND logic — any single false rejects; only all-four-true approves
// ============================================================================

#[test]
fn and_logic_all_four_gates_pass_approves() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    // Strict dev up, latency down, bytes down, holdout up — all four pass.
    let trial = eval(split(0.70, 90, 9_000), split(0.55, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let decision = floor.evaluate(&baseline, &trial);
    assert!(decision.is_approved());
    if let PromotionDecision::Approved { delta } = decision {
        // f64 floor arithmetic carries representation epsilon; compare with
        // tolerance rather than bitwise equality.
        assert!((delta.dev_pass_delta_pp - 0.10).abs() < 1e-9);
        assert_eq!(delta.latency_p95_delta_ms, -10);
        assert_eq!(delta.capsule_bytes_p95_delta_bytes, -1_000);
        assert!((delta.holdout_pass_delta_pp - 0.05).abs() < 1e-9);
        let _ = MetricDelta {
            dev_pass_delta_pp: 0.10,
            latency_p95_delta_ms: -10,
            capsule_bytes_p95_delta_bytes: -1_000,
            holdout_pass_delta_pp: 0.05,
        }; // satisfies use of MetricDelta import.
    }
}

#[test]
fn and_logic_only_dev_fails_rejects_with_dev_reason_only() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    assert!(matches!(reason, FloorReason::DevPassRateNotImproved { .. }));
}

#[test]
fn and_logic_only_latency_fails_rejects_with_latency_reason_only() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.70, 200, 10_000), split(0.50, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    assert!(matches!(reason, FloorReason::LatencyP95Regressed { .. }));
}

#[test]
fn and_logic_only_capsule_bytes_fails_rejects_with_capsule_reason_only() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.70, 100, 20_000), split(0.50, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    assert!(matches!(reason, FloorReason::CapsuleBytesRegressed { .. }));
}

#[test]
fn and_logic_only_holdout_fails_rejects_with_holdout_reason_only() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.70, 100, 10_000), split(0.40, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    assert!(matches!(reason, FloorReason::HoldoutPassRegressed { .. }));
}

#[test]
fn and_logic_all_four_fail_yields_all_four_typed_reasons() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.40, 200, 20_000), split(0.40, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    match floor.evaluate(&baseline, &trial) {
        PromotionDecision::Rejected { reasons, .. } => {
            assert_eq!(reasons.len(), 4, "should surface all four typed reasons");
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::DevPassRateNotImproved { .. })));
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::LatencyP95Regressed { .. })));
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::CapsuleBytesRegressed { .. })));
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::HoldoutPassRegressed { .. })));
        }
        PromotionDecision::Approved { .. } => panic!("expected Rejected"),
    }
}

// ============================================================================
// Per-metric typed rejection — operator-visible discriminant + serde
// ============================================================================

#[test]
fn typed_rejection_serde_round_trip_preserves_discriminant_and_fields() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.40, 200, 20_000), split(0.55, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let original = floor.evaluate(&baseline, &trial);
    let json = serde_json::to_string(&original).expect("serialize decision");
    let parsed: PromotionDecision =
        serde_json::from_str(&json).expect("deserialize decision");
    assert_eq!(original, parsed, "serde round-trip must preserve decision");
    // Spot-check the serialized JSON exposes the typed discriminants the
    // operator UI relies on to render which gate failed.
    assert!(json.contains("dev_pass_rate_not_improved"));
    assert!(json.contains("latency_p95_regressed"));
    assert!(json.contains("capsule_bytes_regressed"));
    assert!(json.contains("holdout_pass_regressed"));
    assert!(json.contains("\"decision\":\"rejected\""));
}

#[test]
fn approved_decision_serde_round_trip() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.70, 100, 10_000), split(0.55, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let decision = floor.evaluate(&baseline, &trial);
    let json = serde_json::to_string(&decision).expect("serialize");
    assert!(json.contains("\"decision\":\"approved\""));
    let parsed: PromotionDecision =
        serde_json::from_str(&json).expect("deserialize");
    // serde_json renders f64 with shortest-round-trip representation; the
    // parsed value should be bitwise-equal to the original.
    match (&decision, &parsed) {
        (
            PromotionDecision::Approved { delta: a },
            PromotionDecision::Approved { delta: b },
        ) => {
            assert!((a.dev_pass_delta_pp - b.dev_pass_delta_pp).abs() < 1e-12);
            assert_eq!(a.latency_p95_delta_ms, b.latency_p95_delta_ms);
            assert_eq!(
                a.capsule_bytes_p95_delta_bytes,
                b.capsule_bytes_p95_delta_bytes
            );
            assert!(
                (a.holdout_pass_delta_pp - b.holdout_pass_delta_pp).abs() < 1e-12
            );
        }
        _ => panic!("expected Approved on both sides"),
    }
}

#[test]
fn metric_delta_matches_arithmetic_on_rejection() {
    let baseline = eval(split(0.60, 100, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.40, 200, 20_000), split(0.55, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    match floor.evaluate(&baseline, &trial) {
        PromotionDecision::Rejected { delta, .. } => {
            assert!((delta.dev_pass_delta_pp - (-0.20)).abs() < 1e-9);
            assert_eq!(delta.latency_p95_delta_ms, 100);
            assert_eq!(delta.capsule_bytes_p95_delta_bytes, 10_000);
            assert!((delta.holdout_pass_delta_pp - (-0.05)).abs() < 1e-9);
        }
        PromotionDecision::Approved { .. } => panic!("expected Rejected"),
    }
}

// ============================================================================
// Tolerances are durable typed state (defaults stable across construction)
// ============================================================================

#[test]
fn default_tolerances_are_stable() {
    let a = PromotionTolerances::default();
    let b = PromotionTolerances::default();
    assert_eq!(a, b, "defaults must be stable across construction");
    assert_eq!(a.latency_p95_regression_tolerance_pct, 5.0);
    assert_eq!(a.capsule_bytes_p95_regression_tolerance_pct, 5.0);
    assert_eq!(a.holdout_pass_regression_tolerance_pp, 0.0);
}

#[test]
fn floor_is_pure_two_identical_inputs_produce_identical_outputs() {
    // External proof of MT-152 implementation_notes "floor is pure (no I/O)":
    // identical inputs must produce identical decisions across invocations.
    let baseline = eval(split(0.60, 100, 10_000), split(0.50, 100, 10_000));
    let trial = eval(split(0.70, 105, 10_000), split(0.55, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let a = floor.evaluate(&baseline, &trial);
    let b = floor.evaluate(&baseline, &trial);
    let c = floor.evaluate(&baseline, &trial);
    assert_eq!(a, b);
    assert_eq!(b, c);
}

#[test]
fn tolerances_round_trip_via_serde() {
    let tolerances = PromotionTolerances {
        latency_p95_regression_tolerance_pct: 7.5,
        capsule_bytes_p95_regression_tolerance_pct: 3.0,
        holdout_pass_regression_tolerance_pp: 0.015,
    };
    let json = serde_json::to_string(&tolerances).expect("serialize tolerances");
    let parsed: PromotionTolerances =
        serde_json::from_str(&json).expect("deserialize tolerances");
    assert_eq!(tolerances, parsed);
}

// ============================================================================
// Extreme regressions surface clean numeric movement (no panic, no overflow)
// ============================================================================

#[test]
fn extreme_capsule_bytes_regression_does_not_panic() {
    // 100x baseline; still a finite percentage.
    let baseline = eval(split(0.50, 100, 1_000), split(0.50, 100, 1_000));
    let trial = eval(split(0.60, 100, 100_000), split(0.50, 100, 1_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    let reason = unwrap_single_reason(floor.evaluate(&baseline, &trial));
    match reason {
        FloorReason::CapsuleBytesRegressed { delta_pct, .. } => {
            assert!(delta_pct.is_finite() && delta_pct > 5_000.0);
        }
        other => panic!("expected CapsuleBytesRegressed, got {other:?}"),
    }
}

#[test]
fn metric_delta_handles_negative_latency_movement() {
    // Trial latency strictly less than baseline -> negative delta_ms.
    let baseline = eval(split(0.60, 200, 10_000), split(0.60, 100, 10_000));
    let trial = eval(split(0.70, 100, 10_000), split(0.60, 100, 10_000));
    let floor = MultiMetricPromotionFloor::with_defaults();
    match floor.evaluate(&baseline, &trial) {
        PromotionDecision::Approved { delta } => {
            assert_eq!(delta.latency_p95_delta_ms, -100);
            assert_eq!(delta.capsule_bytes_p95_delta_bytes, 0);
        }
        PromotionDecision::Rejected { .. } => panic!("expected Approved"),
    }
}
