//! MT-153 — GoodhartSentinel integration tests.
//!
//! Per the MT-153 contract proof_command:
//!   `cargo test -p handshake_core --target-dir ../Handshake_Artifacts/handshake-cargo-target goodhart_sentinel_tests`
//!
//! Inline `#[cfg(test)] mod tests` inside src/self_improve/goodhart_sentinel.rs
//! already covers continue_for_one/two_widening, pause_on_third_consecutive,
//! equal_gap_resets, narrowing_resets, history_bounded_at_max, and
//! receipt serde round-trip. This integration test file satisfies the
//! contract owned_files entry and adds the external-API + adversarial
//! scenarios listed in the MT-153 red_team minimum_controls and the
//! workflow brief:
//!
//!   - monotonic widening over exactly 3 iterations triggers auto-pause
//!     with FR-EVT-GOODHART-PAUSE.
//!   - non-monotonic widening (oscillation) -> no pause.
//!   - pause is sticky via LoopIpcState (AlreadyPaused on second pause
//!     attempt; scheduler must consult IPC state, not the sentinel alone).
//!   - explicit operator unpause clears the IPC pause state; subsequent
//!     pause-with-reason succeeds again (state machine works end-to-end).
//!   - edge: gap exactly stable across 4+ iterations -> no pause.
//!   - edge: gap shrinking then widening (within history bound) ->
//!     sentinel only inspects the latest window so an earlier widening
//!     burst does not leak across a narrowing reset.
//!   - edge: 3rd widening with all-zero gap differences then a tiny positive
//!     epsilon at the end (strict > zero) — strictly widening means strict.
//!   - bounded history truncates at SENTINEL_HISTORY_MAX_ENTRIES.
//!   - receipt history_snapshot carries the pre-latest entries so an
//!     auditor can replay the chain.

use chrono::Utc;
use handshake_core::self_improve::evaluator::SplitMetrics;
use handshake_core::self_improve::goodhart_sentinel::{
    SENTINEL_HISTORY_MAX_ENTRIES, SENTINEL_WIDEN_TRIGGER,
};
use handshake_core::self_improve::ipc::{LoopIpcError, LoopIpcState};
use handshake_core::self_improve::{
    EvalResult, GoodhartSentinel, PauseReason, SentinelDecision, SentinelEntry, SentinelHistory,
    FR_EVT_GOODHART_PAUSE,
};

fn eval(dev_pass: f64, holdout_pass: f64) -> EvalResult {
    EvalResult {
        train: SplitMetrics::empty(),
        dev: SplitMetrics {
            pass_rate: dev_pass,
            pass_count: 0,
            total_count: 0,
            latency_p95_ms: 0,
            capsule_bytes_p95: 0,
            per_item_results: Vec::new(),
        },
        holdout: SplitMetrics {
            pass_rate: holdout_pass,
            pass_count: 0,
            total_count: 0,
            latency_p95_ms: 0,
            capsule_bytes_p95: 0,
            per_item_results: Vec::new(),
        },
        evaluated_at_utc: Utc::now(),
        snapshot_hash: "0".repeat(64),
    }
}

fn entry(iteration_number: u32, gap: f64) -> SentinelEntry {
    SentinelEntry {
        iteration_number,
        dev_pass_rate: gap,
        holdout_pass_rate: 0.0,
        gap,
        accepted_at_utc: Utc::now(),
    }
}

/// Helper: feed N gap values into a fresh history, return the history.
fn history_from_gaps(gaps: &[f64]) -> SentinelHistory {
    let mut h = SentinelHistory::default();
    for (i, g) in gaps.iter().enumerate() {
        h.push(entry((i + 1) as u32, *g));
    }
    h
}

// ============================================================================
// Monotonic widening detection over 3 iterations -> auto-pause
// ============================================================================

#[test]
fn monotonic_widening_three_iterations_triggers_auto_pause() {
    // history gaps [0.05, 0.10, 0.15] + latest gap 0.20 -> 3 consecutive
    // widening transitions -> Pause.
    let history = history_from_gaps(&[0.05, 0.10, 0.15]);
    let latest = eval(0.70, 0.50); // gap 0.20
    let decision = GoodhartSentinel::evaluate(&history, &latest);
    match decision {
        SentinelDecision::Pause { reason, receipt } => {
            assert_eq!(receipt.fr_event_kind, FR_EVT_GOODHART_PAUSE);
            match reason {
                PauseReason::MonotonicGapWidening {
                    gaps,
                    iteration_numbers,
                } => {
                    assert_eq!(
                        gaps.len(),
                        SENTINEL_WIDEN_TRIGGER + 1,
                        "window must include 4 datapoints for 3 transitions"
                    );
                    assert_eq!(iteration_numbers.len(), SENTINEL_WIDEN_TRIGGER + 1);
                    // Strict monotonic widening: each gap > the previous.
                    for w in gaps.windows(2) {
                        assert!(w[1] > w[0], "gap window must be strictly widening");
                    }
                }
                other => panic!("expected MonotonicGapWidening, got {other:?}"),
            }
        }
        SentinelDecision::Continue => panic!("expected Pause after 3 widening iterations"),
    }
}

#[test]
fn pause_receipt_carries_history_snapshot_for_audit_replay() {
    let history = history_from_gaps(&[0.05, 0.10, 0.15]);
    let latest = eval(0.70, 0.50);
    match GoodhartSentinel::evaluate(&history, &latest) {
        SentinelDecision::Pause { receipt, .. } => {
            assert_eq!(
                receipt.history_snapshot.entries.len(),
                3,
                "snapshot should mirror pre-latest history exactly"
            );
            assert_eq!(receipt.fr_event_kind, FR_EVT_GOODHART_PAUSE);
            // receipt_id must be a non-nil v7 UUID.
            assert!(!receipt.receipt_id.is_nil());
            assert_eq!(receipt.receipt_id.get_version_num(), 7);
        }
        SentinelDecision::Continue => panic!("expected Pause"),
    }
}

// ============================================================================
// Non-monotonic gap -> no pause
// ============================================================================

#[test]
fn oscillating_gap_widening_narrowing_widening_does_not_pause() {
    // history gaps [0.05, 0.10, 0.07] + latest 0.12 -> not strictly widening
    // across last 3 transitions (0.10 -> 0.07 is narrowing).
    let history = history_from_gaps(&[0.05, 0.10, 0.07]);
    let latest = eval(0.70, 0.58);
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Continue
    ));
}

#[test]
fn five_widening_then_narrow_then_two_widening_does_not_pause() {
    // Tail must be widening; an earlier widening burst is irrelevant once
    // a narrow resets the chain.
    let history = history_from_gaps(&[0.01, 0.05, 0.10, 0.15, 0.20, 0.10, 0.12]);
    let latest = eval(0.65, 0.50); // gap 0.15
    // Tail window: [0.15, 0.20, 0.10, 0.12, 0.15] -> last 4 are
    // [0.20, 0.10, 0.12, 0.15] -> 0.20 -> 0.10 narrowing, not strictly widening.
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Continue
    ));
}

// ============================================================================
// Edge: gap exactly stable across many iterations -> no pause
// ============================================================================

#[test]
fn stable_gap_across_many_iterations_does_not_pause() {
    // Equal gaps are NOT strictly widening -> no pause regardless of length.
    let history = history_from_gaps(&[0.10, 0.10, 0.10, 0.10, 0.10]);
    let latest = eval(0.60, 0.50);
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Continue
    ));
}

// ============================================================================
// Edge: gap shrinking then widening — earlier widening doesn't leak
// ============================================================================

#[test]
fn earlier_widening_burst_followed_by_narrowing_then_partial_widening_does_not_pause()
{
    // history: [0.05, 0.10, 0.15, 0.20] (3 widening transitions), but then
    // [0.10] narrows, then latest 0.15 widens (1 transition since reset).
    // Sentinel only inspects the last 4 gaps -> [0.20, 0.10, 0.15] + latest
    // 0.18 = [0.20, 0.10, 0.15, 0.18] -> first transition 0.20 -> 0.10 is
    // narrowing -> not strictly widening.
    let history = history_from_gaps(&[0.05, 0.10, 0.15, 0.20, 0.10, 0.15]);
    let latest = eval(0.68, 0.50); // gap 0.18
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Continue
    ));
}

#[test]
fn three_widening_after_a_long_stable_prefix_triggers_pause() {
    // Stable then strictly widening tail must still trigger.
    let history = history_from_gaps(&[0.10, 0.10, 0.10, 0.10, 0.11, 0.12, 0.13]);
    let latest = eval(0.64, 0.50); // gap 0.14
    match GoodhartSentinel::evaluate(&history, &latest) {
        SentinelDecision::Pause { reason, .. } => match reason {
            PauseReason::MonotonicGapWidening { gaps, .. } => {
                assert_eq!(gaps, vec![0.11, 0.12, 0.13, 0.14]);
            }
            other => panic!("expected MonotonicGapWidening, got {other:?}"),
        },
        SentinelDecision::Continue => panic!("expected Pause"),
    }
}

// ============================================================================
// Edge: strict > is strict (epsilon widening counts)
// ============================================================================

#[test]
fn strictly_widening_by_epsilon_still_triggers_pause() {
    let history = history_from_gaps(&[0.10, 0.10001, 0.10002]);
    let latest = eval(0.60003, 0.50); // gap ~0.10003
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Pause { .. }
    ));
}

#[test]
fn zero_delta_widening_does_not_trigger_pause() {
    // All gaps equal to 0.10 -> strict > fails -> Continue.
    let history = history_from_gaps(&[0.10, 0.10, 0.10]);
    let latest = eval(0.60, 0.50); // gap 0.10
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Continue
    ));
}

// ============================================================================
// Bounded history — caller pushes more than max; ring buffer truncates
// ============================================================================

#[test]
fn history_truncates_at_max_entries() {
    let mut h = SentinelHistory::default();
    for i in 0..(SENTINEL_HISTORY_MAX_ENTRIES + 20) as u32 {
        h.push(entry(i, 0.01 * f64::from(i)));
    }
    assert_eq!(h.entries.len(), SENTINEL_HISTORY_MAX_ENTRIES);
    // The OLDEST entries are dropped; the newest remain.
    let first = h.entries.front().expect("non-empty");
    let last = h.entries.back().expect("non-empty");
    assert!(last.iteration_number > first.iteration_number);
    assert_eq!(
        last.iteration_number,
        (SENTINEL_HISTORY_MAX_ENTRIES + 20 - 1) as u32
    );
}

#[test]
fn monotonic_widening_detected_even_when_older_entries_get_truncated() {
    // Push enough entries to force ring-buffer truncation, with the most
    // recent 4 strictly widening. Pause should still trigger because the
    // sentinel inspects the live tail, not the dropped prefix.
    let mut h = SentinelHistory::default();
    // 10 stable entries:
    for i in 0..(SENTINEL_HISTORY_MAX_ENTRIES as u32) {
        h.push(entry(i, 0.05));
    }
    assert_eq!(h.entries.len(), SENTINEL_HISTORY_MAX_ENTRIES);
    // Now feed 3 strictly widening entries; ring drops oldest.
    h.push(entry(100, 0.10));
    h.push(entry(101, 0.15));
    h.push(entry(102, 0.20));
    let latest = eval(0.75, 0.50); // gap 0.25
    assert!(matches!(
        GoodhartSentinel::evaluate(&h, &latest),
        SentinelDecision::Pause { .. }
    ));
}

// ============================================================================
// Pause is sticky via LoopIpcState consumer
// ============================================================================

#[test]
fn sentinel_pause_routes_into_ipc_state_and_is_sticky() {
    // Compose: sentinel emits Pause -> caller forwards reason into
    // LoopIpcState.pause_with_reason. Second pause attempt errors with
    // AlreadyPaused — proving the IPC state machine enforces stickiness.
    let history = history_from_gaps(&[0.05, 0.10, 0.15]);
    let latest = eval(0.70, 0.50);
    let pause = match GoodhartSentinel::evaluate(&history, &latest) {
        SentinelDecision::Pause { reason, .. } => reason,
        SentinelDecision::Continue => panic!("expected Pause"),
    };
    let ipc = LoopIpcState::new(25);
    assert!(!ipc.status().paused);
    let receipt = ipc.pause_with_reason(pause.clone()).expect("first pause");
    assert!(ipc.status().paused);
    assert_eq!(receipt.reason, pause);

    // Sticky: second pause must error with AlreadyPaused.
    let err = ipc.pause_with_reason(pause.clone()).unwrap_err();
    assert!(matches!(err, LoopIpcError::AlreadyPaused));
    assert!(ipc.status().paused, "still paused after sticky error");
}

#[test]
fn operator_unpause_clears_state_and_allows_re_pause() {
    let history = history_from_gaps(&[0.05, 0.10, 0.15]);
    let latest = eval(0.70, 0.50);
    let pause_reason = match GoodhartSentinel::evaluate(&history, &latest) {
        SentinelDecision::Pause { reason, .. } => reason,
        SentinelDecision::Continue => panic!("expected Pause"),
    };
    let ipc = LoopIpcState::new(25);
    ipc.pause_with_reason(pause_reason.clone()).unwrap();
    assert!(ipc.status().paused);

    // Operator unpause clears state.
    let unpause = ipc
        .unpause("reviewed Goodhart drift, dev/holdout normalised".to_string())
        .expect("unpause");
    assert_eq!(unpause.fr_event_kind, "FR-EVT-LOOP-UNPAUSE");
    assert!(!ipc.status().paused);
    assert!(ipc.status().pause_reason.is_none());

    // Re-pause now works (state machine is unstuck after explicit unpause).
    let second = ipc.pause_with_reason(pause_reason).unwrap();
    assert_eq!(second.fr_event_kind, "FR-EVT-LOOP-PAUSE");
    assert!(ipc.status().paused);
}

#[test]
fn unpause_without_rationale_rejected_keeping_pause_sticky() {
    let history = history_from_gaps(&[0.05, 0.10, 0.15]);
    let latest = eval(0.70, 0.50);
    let pause_reason = match GoodhartSentinel::evaluate(&history, &latest) {
        SentinelDecision::Pause { reason, .. } => reason,
        SentinelDecision::Continue => panic!("expected Pause"),
    };
    let ipc = LoopIpcState::new(25);
    ipc.pause_with_reason(pause_reason).unwrap();

    // Empty rationale rejected -> pause stays sticky.
    let err = ipc.unpause("  ".to_string()).unwrap_err();
    assert!(matches!(err, LoopIpcError::EmptyRationale));
    assert!(
        ipc.status().paused,
        "rejected unpause must leave state sticky"
    );

    // Real rationale eventually clears.
    ipc.unpause("genuine rationale".to_string()).unwrap();
    assert!(!ipc.status().paused);
}

#[test]
fn unpause_on_unpaused_loop_is_typed_error() {
    let ipc = LoopIpcState::new(25);
    assert!(!ipc.status().paused);
    let err = ipc.unpause("doesn't matter".to_string()).unwrap_err();
    assert!(matches!(err, LoopIpcError::NotPaused));
}

// ============================================================================
// Helper: build SentinelEntry from EvalResult (entry_from_eval)
// ============================================================================

#[test]
fn entry_from_eval_computes_gap_from_dev_minus_holdout() {
    let result = eval(0.80, 0.55);
    let e = GoodhartSentinel::entry_from_eval(7, &result);
    assert_eq!(e.iteration_number, 7);
    assert!((e.dev_pass_rate - 0.80).abs() < 1e-9);
    assert!((e.holdout_pass_rate - 0.55).abs() < 1e-9);
    assert!((e.gap - 0.25).abs() < 1e-9);
}

#[test]
fn entry_from_eval_handles_negative_gap_holdout_above_dev() {
    // Holdout > dev can happen on small or noisy splits; the sentinel
    // tracks the *signed* gap so caller can detect inverted regimes.
    let result = eval(0.40, 0.55);
    let e = GoodhartSentinel::entry_from_eval(9, &result);
    assert!((e.gap - (-0.15)).abs() < 1e-9);
}

// ============================================================================
// Determinism + history immutability under .evaluate()
// ============================================================================

#[test]
fn sentinel_evaluate_does_not_mutate_history() {
    // The sentinel is a read-only inspector; pushing the entry is the
    // caller's responsibility. Verify two consecutive evaluations against
    // the same history yield the same decision shape.
    let history = history_from_gaps(&[0.05, 0.10, 0.15]);
    let latest = eval(0.70, 0.50);
    let pre_len = history.entries.len();
    let a = GoodhartSentinel::evaluate(&history, &latest);
    let b = GoodhartSentinel::evaluate(&history, &latest);
    assert_eq!(history.entries.len(), pre_len, "history not mutated");
    // Both must be Pause (the inputs are identical) — receipt ids differ
    // by design (timestamps + UUIDv7) but the discriminant matches.
    assert!(matches!(a, SentinelDecision::Pause { .. }));
    assert!(matches!(b, SentinelDecision::Pause { .. }));
}

#[test]
fn sentinel_is_no_io_pure_inspection() {
    // Two distinct sentinel calls with the same inputs return the same
    // decision discriminant. This is the closest external-API proof of
    // "Sentinel is pure (no I/O)" (validator_focus on MT-153 contract).
    let history = SentinelHistory::default();
    let latest = eval(0.60, 0.55); // gap 0.05
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Continue
    ));
    assert!(matches!(
        GoodhartSentinel::evaluate(&history, &latest),
        SentinelDecision::Continue
    ));
}

// ============================================================================
// Decision serde round-trip — operator UI can render the typed reason
// ============================================================================

#[test]
fn pause_decision_serde_round_trip_preserves_reason_and_receipt() {
    let history = history_from_gaps(&[0.05, 0.10, 0.15]);
    let latest = eval(0.70, 0.50);
    let decision = GoodhartSentinel::evaluate(&history, &latest);
    let json = serde_json::to_string(&decision).expect("serialize");
    let parsed: SentinelDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decision, parsed);
    assert!(json.contains("monotonic_gap_widening"));
    assert!(json.contains("FR-EVT-GOODHART-PAUSE"));
    assert!(json.contains("\"decision\":\"pause\""));
}

#[test]
fn continue_decision_serde_round_trip() {
    let history = SentinelHistory::default();
    let latest = eval(0.60, 0.55);
    let decision = GoodhartSentinel::evaluate(&history, &latest);
    let json = serde_json::to_string(&decision).expect("serialize");
    assert!(json.contains("\"decision\":\"continue\""));
    let parsed: SentinelDecision = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decision, parsed);
}
