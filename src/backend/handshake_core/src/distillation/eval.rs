//! Evaluation and promotion gates for adapter checkpoints (Master Spec Section 9.1.4).
//!
//! Benchmark-gated promotion prevents silent regression by comparing candidate
//! checkpoints against previous and teacher baselines.

use serde::{Deserialize, Serialize};

/// Metrics produced by an eval run on a fixed suite of code tasks.
///
/// Stored in `eval_run.metrics_json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalMetrics {
    /// Fraction of tasks solved on first attempt.
    pub pass_at_1: f64,
    /// Fraction of tasks solved within k attempts.
    pub pass_at_k: f64,
    /// Fraction of generated outputs that compile successfully.
    pub compile_success_rate: f64,
    /// Fraction of generated outputs where all tests pass.
    pub test_pass_rate: f64,
    /// Repetition score (0 = no repetition, higher = more collapse).
    pub repetition_score: f64,
    /// Output entropy (lower values indicate potential collapse).
    pub entropy: f64,
    /// Fraction of outputs with syntax errors.
    pub syntax_error_rate: f64,
    /// Fraction of outputs with security flag triggers.
    pub security_flag_rate: f64,
}

/// Thresholds controlling the promotion gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalThresholds {
    /// Candidate core metrics must be >= previous - epsilon.
    pub core_metric_epsilon: f64,
    /// Candidate core metrics must be >= teacher - delta.
    pub teacher_delta: f64,
    /// Maximum allowed increase in collapse indicators vs previous.
    pub max_collapse_increase: f64,
    /// Absolute ceiling for security flag rate.
    pub max_security_flag_rate: f64,
}

/// Result of the promotion gate evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct PromotionDecision {
    /// Whether the candidate checkpoint is approved for promotion.
    pub approved: bool,
    /// Human-readable reasons for rejection (empty when approved).
    pub reasons: Vec<String>,
}

/// Evaluate a candidate checkpoint against previous and teacher baselines.
///
/// Implements Section 9.1.4:
/// 1. Core metrics (pass@1, pass@k, compile_success_rate, test_pass_rate) must
///    not regress beyond `epsilon` vs previous or `delta` vs teacher.
/// 2. Collapse indicators (repetition_score, syntax_error_rate) must not
///    increase beyond `max_collapse_increase` vs previous, and entropy must
///    not decrease beyond that threshold.
/// 3. Security flag rate must stay below `max_security_flag_rate`.
pub fn evaluate_and_maybe_promote(
    candidate: &EvalMetrics,
    previous: &EvalMetrics,
    teacher: &EvalMetrics,
    thresholds: &EvalThresholds,
) -> PromotionDecision {
    let mut reasons = Vec::new();

    // Core metrics: candidate >= previous - epsilon
    check_core_metric(
        "pass@1",
        candidate.pass_at_1,
        previous.pass_at_1,
        thresholds.core_metric_epsilon,
        &mut reasons,
    );
    check_core_metric(
        "pass@k",
        candidate.pass_at_k,
        previous.pass_at_k,
        thresholds.core_metric_epsilon,
        &mut reasons,
    );
    check_core_metric(
        "compile_success_rate",
        candidate.compile_success_rate,
        previous.compile_success_rate,
        thresholds.core_metric_epsilon,
        &mut reasons,
    );
    check_core_metric(
        "test_pass_rate",
        candidate.test_pass_rate,
        previous.test_pass_rate,
        thresholds.core_metric_epsilon,
        &mut reasons,
    );

    // Core metrics: candidate >= teacher - delta
    check_teacher_metric(
        "pass@1",
        candidate.pass_at_1,
        teacher.pass_at_1,
        thresholds.teacher_delta,
        &mut reasons,
    );
    check_teacher_metric(
        "pass@k",
        candidate.pass_at_k,
        teacher.pass_at_k,
        thresholds.teacher_delta,
        &mut reasons,
    );
    check_teacher_metric(
        "compile_success_rate",
        candidate.compile_success_rate,
        teacher.compile_success_rate,
        thresholds.teacher_delta,
        &mut reasons,
    );
    check_teacher_metric(
        "test_pass_rate",
        candidate.test_pass_rate,
        teacher.test_pass_rate,
        thresholds.teacher_delta,
        &mut reasons,
    );

    // Collapse indicators vs previous
    let max_inc = thresholds.max_collapse_increase;

    let rep_increase = candidate.repetition_score - previous.repetition_score;
    if rep_increase > max_inc {
        reasons.push(format!(
            "repetition_score increased by {rep_increase:.4} (max {max_inc:.4})"
        ));
    }

    let syntax_increase = candidate.syntax_error_rate - previous.syntax_error_rate;
    if syntax_increase > max_inc {
        reasons.push(format!(
            "syntax_error_rate increased by {syntax_increase:.4} (max {max_inc:.4})"
        ));
    }

    // Entropy decrease is a collapse signal (lower = more repetitive)
    let entropy_decrease = previous.entropy - candidate.entropy;
    if entropy_decrease > max_inc {
        reasons.push(format!(
            "entropy decreased by {entropy_decrease:.4} (max decrease {max_inc:.4})"
        ));
    }

    // Security flag rate absolute ceiling
    if candidate.security_flag_rate > thresholds.max_security_flag_rate {
        reasons.push(format!(
            "security_flag_rate {:.4} exceeds max {:.4}",
            candidate.security_flag_rate, thresholds.max_security_flag_rate
        ));
    }

    PromotionDecision {
        approved: reasons.is_empty(),
        reasons,
    }
}

fn check_core_metric(
    name: &str,
    candidate_val: f64,
    previous_val: f64,
    epsilon: f64,
    reasons: &mut Vec<String>,
) {
    if candidate_val < previous_val - epsilon {
        reasons.push(format!(
            "{name} regressed vs previous: {candidate_val:.4} < {:.4} (threshold {previous_val:.4} - {epsilon:.4})",
            previous_val - epsilon
        ));
    }
}

fn check_teacher_metric(
    name: &str,
    candidate_val: f64,
    teacher_val: f64,
    delta: f64,
    reasons: &mut Vec<String>,
) {
    if candidate_val < teacher_val - delta {
        reasons.push(format!(
            "{name} below teacher: {candidate_val:.4} < {:.4} (threshold {teacher_val:.4} - {delta:.4})",
            teacher_val - delta
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_metrics() -> EvalMetrics {
        EvalMetrics {
            pass_at_1: 0.85,
            pass_at_k: 0.95,
            compile_success_rate: 0.98,
            test_pass_rate: 0.92,
            repetition_score: 0.05,
            entropy: 4.5,
            syntax_error_rate: 0.02,
            security_flag_rate: 0.01,
        }
    }

    fn default_thresholds() -> EvalThresholds {
        EvalThresholds {
            core_metric_epsilon: 0.05,
            teacher_delta: 0.10,
            max_collapse_increase: 0.05,
            max_security_flag_rate: 0.05,
        }
    }

    #[test]
    fn eval_promotion_approves_when_candidate_beats_baselines() {
        let candidate = good_metrics();
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(decision.approved, "equal metrics should pass: {:?}", decision.reasons);
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn eval_promotion_approves_improvement_over_baselines() {
        let mut candidate = good_metrics();
        candidate.pass_at_1 = 0.90;
        candidate.pass_at_k = 0.97;
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(decision.approved);
    }

    #[test]
    fn eval_promotion_rejects_regression_vs_previous() {
        let mut candidate = good_metrics();
        candidate.pass_at_1 = 0.70; // 0.85 - 0.05 = 0.80, so 0.70 < 0.80
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(!decision.approved);
        assert!(decision.reasons.iter().any(|r| r.contains("pass@1") && r.contains("previous")));
    }

    #[test]
    fn eval_promotion_allows_within_epsilon_of_previous() {
        let mut candidate = good_metrics();
        candidate.pass_at_1 = 0.80; // exactly previous (0.85) - epsilon (0.05)
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(decision.approved, "at epsilon boundary should pass: {:?}", decision.reasons);
    }

    #[test]
    fn eval_promotion_rejects_below_teacher_delta() {
        let mut candidate = good_metrics();
        candidate.compile_success_rate = 0.75; // teacher 0.98 - delta 0.10 = 0.88
        let previous = good_metrics();
        let mut teacher = good_metrics();
        teacher.compile_success_rate = 0.98;
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(!decision.approved);
        assert!(decision.reasons.iter().any(|r| r.contains("compile_success_rate") && r.contains("teacher")));
    }

    #[test]
    fn eval_promotion_allows_within_delta_of_teacher() {
        let teacher = good_metrics();
        let thresholds = default_thresholds();
        // Compute the boundary the same way as production code to avoid float rounding
        let boundary = teacher.test_pass_rate - thresholds.teacher_delta;

        let mut candidate = good_metrics();
        candidate.test_pass_rate = boundary;
        let mut previous = good_metrics();
        previous.test_pass_rate = boundary; // also lower previous so epsilon check passes

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(decision.approved, "at delta boundary should pass: {:?}", decision.reasons);
    }

    #[test]
    fn eval_promotion_rejects_repetition_collapse() {
        let mut candidate = good_metrics();
        candidate.repetition_score = 0.15; // previous 0.05, increase 0.10 > max 0.05
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(!decision.approved);
        assert!(decision.reasons.iter().any(|r| r.contains("repetition_score")));
    }

    #[test]
    fn eval_promotion_rejects_entropy_collapse() {
        let mut candidate = good_metrics();
        candidate.entropy = 4.3; // previous 4.5, decrease 0.2 > max 0.05
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(!decision.approved);
        assert!(decision.reasons.iter().any(|r| r.contains("entropy")));
    }

    #[test]
    fn eval_promotion_rejects_syntax_error_increase() {
        let mut candidate = good_metrics();
        candidate.syntax_error_rate = 0.12; // previous 0.02, increase 0.10 > max 0.05
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(!decision.approved);
        assert!(decision.reasons.iter().any(|r| r.contains("syntax_error_rate")));
    }

    #[test]
    fn eval_promotion_rejects_security_flag_rate() {
        let mut candidate = good_metrics();
        candidate.security_flag_rate = 0.10; // max 0.05
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(!decision.approved);
        assert!(decision.reasons.iter().any(|r| r.contains("security_flag_rate")));
    }

    #[test]
    fn eval_promotion_collects_multiple_reasons() {
        let mut candidate = good_metrics();
        candidate.pass_at_1 = 0.50;          // fails vs previous and teacher
        candidate.repetition_score = 0.20;    // collapse
        candidate.security_flag_rate = 0.10;  // too high
        let previous = good_metrics();
        let teacher = good_metrics();
        let thresholds = default_thresholds();

        let decision = evaluate_and_maybe_promote(&candidate, &previous, &teacher, &thresholds);
        assert!(!decision.approved);
        assert!(
            decision.reasons.len() >= 3,
            "should report multiple failures, got: {:?}",
            decision.reasons
        );
    }

    #[test]
    fn eval_promotion_metrics_round_trip() {
        let metrics = good_metrics();
        let json = serde_json::to_string(&metrics).unwrap();
        let parsed: EvalMetrics = serde_json::from_str(&json).unwrap();
        assert!((parsed.pass_at_1 - metrics.pass_at_1).abs() < 1e-10);
        assert!((parsed.entropy - metrics.entropy).abs() < 1e-10);
    }

    #[test]
    fn eval_promotion_thresholds_round_trip() {
        let thresholds = default_thresholds();
        let json = serde_json::to_string(&thresholds).unwrap();
        let parsed: EvalThresholds = serde_json::from_str(&json).unwrap();
        assert!((parsed.core_metric_epsilon - thresholds.core_metric_epsilon).abs() < 1e-10);
        assert!((parsed.teacher_delta - thresholds.teacher_delta).abs() < 1e-10);
    }
}
