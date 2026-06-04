//! MT-152: MultiMetricPromotionFloor.
//!
//! Pure function (no I/O). Tolerances are durable typed state and changing
//! them requires its own PromotionGate operator review.

use serde::{Deserialize, Serialize};

use super::evaluator::EvalResult;

/// Finite sentinel for a positive trial value over a zero baseline.
///
/// Mathematically this regression is unbounded, but non-finite floats become
/// `null` or serialization errors in JSON evidence consumers. Use the largest
/// finite f64 so the floor still fails closed without emitting invalid
/// operator-facing evidence.
const UNBOUNDED_REGRESSION_PCT_SENTINEL: f64 = f64::MAX;

/// Tolerances exposed as a typed struct so callers cannot pass magic
/// numbers. Defaults per MT-152 implementation notes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PromotionTolerances {
    /// Maximum acceptable latency p95 regression as a percentage of baseline.
    pub latency_p95_regression_tolerance_pct: f64,
    /// Maximum acceptable capsule-bytes p95 regression as a percentage of baseline.
    pub capsule_bytes_p95_regression_tolerance_pct: f64,
    /// Maximum acceptable holdout PASS rate regression in absolute
    /// percentage points (0.0 means no regression allowed).
    pub holdout_pass_regression_tolerance_pp: f64,
}

impl Default for PromotionTolerances {
    fn default() -> Self {
        Self {
            latency_p95_regression_tolerance_pct: 5.0,
            capsule_bytes_p95_regression_tolerance_pct: 5.0,
            holdout_pass_regression_tolerance_pp: 0.0,
        }
    }
}

/// Reasons the floor may reject. Typed; every reason carries the inputs
/// the reviewer needs to reason about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum FloorReason {
    DevPassRateNotImproved {
        baseline_pass_rate: f64,
        trial_pass_rate: f64,
    },
    LatencyP95Regressed {
        baseline_ms: u64,
        trial_ms: u64,
        delta_pct: f64,
    },
    CapsuleBytesRegressed {
        baseline_bytes: u64,
        trial_bytes: u64,
        delta_pct: f64,
    },
    HoldoutPassRegressed {
        baseline_pass_rate: f64,
        trial_pass_rate: f64,
        delta_pp: f64,
    },
}

/// Delta surface returned alongside an Approved or Rejected decision so the
/// caller can render the actual numeric movement to the operator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MetricDelta {
    pub dev_pass_delta_pp: f64,
    pub latency_p95_delta_ms: i64,
    pub capsule_bytes_p95_delta_bytes: i64,
    pub holdout_pass_delta_pp: f64,
}

/// Decision returned by the floor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum PromotionDecision {
    Approved {
        delta: MetricDelta,
    },
    Rejected {
        reasons: Vec<FloorReason>,
        delta: MetricDelta,
    },
}

impl PromotionDecision {
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }
}

/// Floor itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultiMetricPromotionFloor {
    pub tolerances: PromotionTolerances,
}

impl MultiMetricPromotionFloor {
    pub fn new(tolerances: PromotionTolerances) -> Self {
        Self { tolerances }
    }

    pub fn with_defaults() -> Self {
        Self {
            tolerances: PromotionTolerances::default(),
        }
    }

    /// Evaluate baseline vs trial. Pure: no I/O.
    pub fn evaluate(&self, baseline: &EvalResult, trial: &EvalResult) -> PromotionDecision {
        let delta = MetricDelta {
            dev_pass_delta_pp: trial.dev.pass_rate - baseline.dev.pass_rate,
            latency_p95_delta_ms: trial.dev.latency_p95_ms as i64
                - baseline.dev.latency_p95_ms as i64,
            capsule_bytes_p95_delta_bytes: trial.dev.capsule_bytes_p95 as i64
                - baseline.dev.capsule_bytes_p95 as i64,
            holdout_pass_delta_pp: trial.holdout.pass_rate - baseline.holdout.pass_rate,
        };

        let mut reasons = Vec::new();

        // 1) dev PASS strictly up
        if trial.dev.pass_rate <= baseline.dev.pass_rate {
            reasons.push(FloorReason::DevPassRateNotImproved {
                baseline_pass_rate: baseline.dev.pass_rate,
                trial_pass_rate: trial.dev.pass_rate,
            });
        }

        // 2) latency p95 not regressed beyond tolerance
        if baseline.dev.latency_p95_ms > 0 {
            let baseline_ms = baseline.dev.latency_p95_ms as f64;
            let trial_ms = trial.dev.latency_p95_ms as f64;
            let pct = ((trial_ms - baseline_ms) / baseline_ms) * 100.0;
            if pct > self.tolerances.latency_p95_regression_tolerance_pct {
                reasons.push(FloorReason::LatencyP95Regressed {
                    baseline_ms: baseline.dev.latency_p95_ms,
                    trial_ms: trial.dev.latency_p95_ms,
                    delta_pct: pct,
                });
            }
        } else if trial.dev.latency_p95_ms > 0 {
            // baseline 0 -> any positive trial is an unbounded regression.
            // Surface a finite sentinel so JSON evidence remains usable.
            reasons.push(FloorReason::LatencyP95Regressed {
                baseline_ms: baseline.dev.latency_p95_ms,
                trial_ms: trial.dev.latency_p95_ms,
                delta_pct: UNBOUNDED_REGRESSION_PCT_SENTINEL,
            });
        }

        // 3) capsule bytes p95 not regressed beyond tolerance
        if baseline.dev.capsule_bytes_p95 > 0 {
            let baseline_bytes = baseline.dev.capsule_bytes_p95 as f64;
            let trial_bytes = trial.dev.capsule_bytes_p95 as f64;
            let pct = ((trial_bytes - baseline_bytes) / baseline_bytes) * 100.0;
            if pct > self.tolerances.capsule_bytes_p95_regression_tolerance_pct {
                reasons.push(FloorReason::CapsuleBytesRegressed {
                    baseline_bytes: baseline.dev.capsule_bytes_p95,
                    trial_bytes: trial.dev.capsule_bytes_p95,
                    delta_pct: pct,
                });
            }
        } else if trial.dev.capsule_bytes_p95 > 0 {
            reasons.push(FloorReason::CapsuleBytesRegressed {
                baseline_bytes: baseline.dev.capsule_bytes_p95,
                trial_bytes: trial.dev.capsule_bytes_p95,
                delta_pct: UNBOUNDED_REGRESSION_PCT_SENTINEL,
            });
        }

        // 4) holdout PASS not regressed beyond tolerance (in percentage
        // points)
        let holdout_delta_pp = baseline.holdout.pass_rate - trial.holdout.pass_rate;
        if holdout_delta_pp > self.tolerances.holdout_pass_regression_tolerance_pp {
            reasons.push(FloorReason::HoldoutPassRegressed {
                baseline_pass_rate: baseline.holdout.pass_rate,
                trial_pass_rate: trial.holdout.pass_rate,
                delta_pp: holdout_delta_pp,
            });
        }

        if reasons.is_empty() {
            PromotionDecision::Approved { delta }
        } else {
            PromotionDecision::Rejected { reasons, delta }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::evaluator::SplitMetrics;
    use super::*;
    use chrono::Utc;

    fn metric(pass_rate: f64, latency: u64, bytes: u64, total: u32) -> SplitMetrics {
        SplitMetrics {
            pass_rate,
            pass_count: (pass_rate * total as f64).round() as u32,
            total_count: total,
            latency_p95_ms: latency,
            capsule_bytes_p95: bytes,
            per_item_results: Vec::new(),
        }
    }

    fn eval_result(dev: SplitMetrics, holdout: SplitMetrics) -> EvalResult {
        EvalResult {
            train: SplitMetrics::empty(),
            dev,
            holdout,
            evaluated_at_utc: Utc::now(),
            snapshot_hash: "0".repeat(64),
        }
    }

    #[test]
    fn approved_when_all_metrics_pass() {
        let baseline = eval_result(metric(0.50, 100, 10_000, 18), metric(0.50, 100, 10_000, 6));
        let trial = eval_result(metric(0.60, 100, 10_000, 18), metric(0.55, 100, 10_000, 6));
        let floor = MultiMetricPromotionFloor::with_defaults();
        let decision = floor.evaluate(&baseline, &trial);
        assert!(decision.is_approved());
        if let PromotionDecision::Approved { delta } = decision {
            assert!((delta.dev_pass_delta_pp - 0.10).abs() < 1e-9);
        }
    }

    #[test]
    fn rejected_when_dev_not_improved() {
        let baseline = eval_result(metric(0.60, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let trial = eval_result(metric(0.60, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let floor = MultiMetricPromotionFloor::with_defaults();
        let decision = floor.evaluate(&baseline, &trial);
        if let PromotionDecision::Rejected { reasons, .. } = decision {
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::DevPassRateNotImproved { .. })));
        } else {
            panic!("expected Rejected");
        }
    }

    #[test]
    fn rejected_when_latency_regresses_beyond_tolerance() {
        let baseline = eval_result(metric(0.60, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let trial = eval_result(metric(0.70, 110, 10_000, 18), metric(0.60, 100, 10_000, 6));
        // 10% regression > 5% tolerance default
        let floor = MultiMetricPromotionFloor::with_defaults();
        let decision = floor.evaluate(&baseline, &trial);
        if let PromotionDecision::Rejected { reasons, .. } = decision {
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::LatencyP95Regressed { .. })));
        } else {
            panic!("expected Rejected");
        }
    }

    #[test]
    fn rejected_when_capsule_bytes_regresses_beyond_tolerance() {
        let baseline = eval_result(metric(0.60, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let trial = eval_result(metric(0.70, 100, 10_700, 18), metric(0.60, 100, 10_000, 6));
        let floor = MultiMetricPromotionFloor::with_defaults();
        let decision = floor.evaluate(&baseline, &trial);
        if let PromotionDecision::Rejected { reasons, .. } = decision {
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::CapsuleBytesRegressed { .. })));
        } else {
            panic!("expected Rejected");
        }
    }

    #[test]
    fn rejected_when_holdout_regresses_at_all() {
        let baseline = eval_result(metric(0.60, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let trial = eval_result(metric(0.70, 100, 10_000, 18), metric(0.59, 100, 10_000, 6));
        let floor = MultiMetricPromotionFloor::with_defaults();
        let decision = floor.evaluate(&baseline, &trial);
        if let PromotionDecision::Rejected { reasons, .. } = decision {
            assert!(reasons
                .iter()
                .any(|r| matches!(r, FloorReason::HoldoutPassRegressed { .. })));
        } else {
            panic!("expected Rejected");
        }
    }

    #[test]
    fn rejected_with_multiple_reasons() {
        let baseline = eval_result(metric(0.60, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let trial = eval_result(metric(0.40, 200, 20_000, 18), metric(0.50, 100, 10_000, 6));
        let floor = MultiMetricPromotionFloor::with_defaults();
        let decision = floor.evaluate(&baseline, &trial);
        if let PromotionDecision::Rejected { reasons, .. } = decision {
            assert!(reasons.len() >= 3);
        } else {
            panic!("expected Rejected");
        }
    }

    #[test]
    fn floor_is_pure_no_io() {
        // Two identical inputs always produce the same output.
        let baseline = eval_result(metric(0.60, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let trial = eval_result(metric(0.70, 100, 10_000, 18), metric(0.60, 100, 10_000, 6));
        let floor = MultiMetricPromotionFloor::with_defaults();
        let a = floor.evaluate(&baseline, &trial);
        let b = floor.evaluate(&baseline, &trial);
        assert_eq!(a, b);
    }
}
