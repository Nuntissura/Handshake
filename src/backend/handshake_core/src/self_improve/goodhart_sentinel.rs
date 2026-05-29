//! MT-153: GoodhartSentinel.
//!
//! Observes dev/holdout gap across accepted iterations. Pauses when the gap
//! widens monotonically for 3 consecutive iterations. Strictness matters:
//! equal-gap iterations reset the counter so stable runs do not false-positive.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::evaluator::EvalResult;

pub const FR_EVT_GOODHART_PAUSE: &str = "FR-EVT-GOODHART-PAUSE";

/// Bound on history retained. Bounded to prevent unbounded memory growth
/// across long-lived loop instances per MT-153 red-team controls.
pub const SENTINEL_HISTORY_MAX_ENTRIES: usize = 10;

/// Number of consecutive widening iterations that trigger Pause.
pub const SENTINEL_WIDEN_TRIGGER: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentinelEntry {
    pub iteration_number: u32,
    pub dev_pass_rate: f64,
    pub holdout_pass_rate: f64,
    pub gap: f64,
    pub accepted_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SentinelHistory {
    pub entries: VecDeque<SentinelEntry>,
}

impl SentinelHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a new entry; bounded ring buffer drops oldest at the cap.
    pub fn push(&mut self, entry: SentinelEntry) {
        self.entries.push_back(entry);
        while self.entries.len() > SENTINEL_HISTORY_MAX_ENTRIES {
            self.entries.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl PartialEq for SentinelHistory {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum SentinelDecision {
    Continue,
    Pause {
        reason: PauseReason,
        receipt: SentinelReceipt,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PauseReason {
    MonotonicGapWidening {
        gaps: Vec<f64>,
        iteration_numbers: Vec<u32>,
    },
    Operator {
        rationale: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SentinelReceipt {
    pub receipt_id: Uuid,
    pub paused_at_utc: DateTime<Utc>,
    pub fr_event_kind: String,
    pub history_snapshot: SentinelHistory,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GoodhartSentinel;

impl GoodhartSentinel {
    /// Evaluate the sentinel against the latest eval result. Caller must
    /// only invoke this when the iteration was ACCEPTED (per MT-153).
    pub fn evaluate(history: &SentinelHistory, latest: &EvalResult) -> SentinelDecision {
        // Compute the latest gap = dev_pass - holdout_pass.
        let latest_gap = latest.dev.pass_rate - latest.holdout.pass_rate;

        // Build a candidate history including the latest entry to inspect
        // monotonic widening over the last SENTINEL_WIDEN_TRIGGER entries.
        let mut tail_gaps: Vec<f64> = history.entries.iter().map(|e| e.gap).collect();
        tail_gaps.push(latest_gap);

        // Need at least SENTINEL_WIDEN_TRIGGER+1 datapoints (n widening pairs => n+1 gaps).
        // For SENTINEL_WIDEN_TRIGGER=3 we look at the last 4 gaps and check
        // that gaps[k] > gaps[k-1] for the last 3 transitions (strictly
        // widening).
        let required = SENTINEL_WIDEN_TRIGGER + 1;
        if tail_gaps.len() >= required {
            let window = &tail_gaps[tail_gaps.len() - required..];
            let strictly_widening = window.windows(2).all(|pair| pair[1] > pair[0]);

            if strictly_widening {
                // Collect iteration numbers for the receipt. The latest is
                // not yet in `history`; we synthesize it from the eval
                // result's snapshot_hash absence and use 0 as a sentinel
                // for the caller to override. In practice the caller
                // appends the entry after observing the decision.
                let mut iteration_numbers: Vec<u32> =
                    history.entries.iter().map(|e| e.iteration_number).collect();
                iteration_numbers.push(iteration_numbers.last().copied().unwrap_or(0) + 1);
                let receipt = SentinelReceipt {
                    receipt_id: Uuid::now_v7(),
                    paused_at_utc: Utc::now(),
                    fr_event_kind: FR_EVT_GOODHART_PAUSE.to_string(),
                    history_snapshot: history.clone(),
                };
                let len = iteration_numbers.len();
                return SentinelDecision::Pause {
                    reason: PauseReason::MonotonicGapWidening {
                        gaps: window.to_vec(),
                        iteration_numbers: iteration_numbers[len - required..].to_vec(),
                    },
                    receipt,
                };
            }
        }

        SentinelDecision::Continue
    }

    /// Helper: build a SentinelEntry from an EvalResult.
    pub fn entry_from_eval(iteration_number: u32, eval: &EvalResult) -> SentinelEntry {
        let gap = eval.dev.pass_rate - eval.holdout.pass_rate;
        SentinelEntry {
            iteration_number,
            dev_pass_rate: eval.dev.pass_rate,
            holdout_pass_rate: eval.holdout.pass_rate,
            gap,
            accepted_at_utc: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::evaluator::SplitMetrics;
    use super::*;

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

    fn entry(iter_num: u32, gap: f64) -> SentinelEntry {
        SentinelEntry {
            iteration_number: iter_num,
            dev_pass_rate: gap,
            holdout_pass_rate: 0.0,
            gap,
            accepted_at_utc: Utc::now(),
        }
    }

    #[test]
    fn continue_for_one_widening() {
        let history = SentinelHistory::default();
        let decision = GoodhartSentinel::evaluate(&history, &eval(0.6, 0.55));
        assert!(matches!(decision, SentinelDecision::Continue));
    }

    #[test]
    fn continue_for_two_widening() {
        let mut history = SentinelHistory::default();
        history.push(entry(1, 0.05));
        let decision = GoodhartSentinel::evaluate(&history, &eval(0.6, 0.5));
        // gap series: [0.05, 0.10] — 1 widening transition; need 3.
        assert!(matches!(decision, SentinelDecision::Continue));
    }

    #[test]
    fn pause_on_third_consecutive_widening() {
        // history: [0.05, 0.10, 0.15] then latest 0.20
        let mut history = SentinelHistory::default();
        history.push(entry(1, 0.05));
        history.push(entry(2, 0.10));
        history.push(entry(3, 0.15));
        let decision = GoodhartSentinel::evaluate(&history, &eval(0.7, 0.5));
        match decision {
            SentinelDecision::Pause { reason, receipt } => {
                assert_eq!(receipt.fr_event_kind, FR_EVT_GOODHART_PAUSE);
                match reason {
                    PauseReason::MonotonicGapWidening { gaps, .. } => {
                        assert_eq!(gaps.len(), 4);
                    }
                    _ => panic!("expected MonotonicGapWidening"),
                }
            }
            _ => panic!("expected Pause"),
        }
    }

    #[test]
    fn equal_gap_resets_counter() {
        // history: [0.05, 0.10, 0.10] — middle equal so widening chain
        // breaks; latest 0.15 widens once.
        let mut history = SentinelHistory::default();
        history.push(entry(1, 0.05));
        history.push(entry(2, 0.10));
        history.push(entry(3, 0.10));
        let decision = GoodhartSentinel::evaluate(&history, &eval(0.7, 0.55));
        assert!(matches!(decision, SentinelDecision::Continue));
    }

    #[test]
    fn narrowing_resets_counter() {
        let mut history = SentinelHistory::default();
        history.push(entry(1, 0.05));
        history.push(entry(2, 0.10));
        history.push(entry(3, 0.08));
        let decision = GoodhartSentinel::evaluate(&history, &eval(0.6, 0.5));
        assert!(matches!(decision, SentinelDecision::Continue));
    }

    #[test]
    fn history_bounded_at_max() {
        let mut history = SentinelHistory::default();
        for i in 0..(SENTINEL_HISTORY_MAX_ENTRIES + 5) as u32 {
            history.push(entry(i, 0.01 * i as f64));
        }
        assert_eq!(history.entries.len(), SENTINEL_HISTORY_MAX_ENTRIES);
    }

    #[test]
    fn receipt_round_trips_via_serde() {
        let history = SentinelHistory::default();
        let decision = match GoodhartSentinel::evaluate(&history, &eval(0.6, 0.55)) {
            SentinelDecision::Continue => SentinelDecision::Pause {
                reason: PauseReason::Operator {
                    rationale: "manual".to_string(),
                },
                receipt: SentinelReceipt {
                    receipt_id: Uuid::now_v7(),
                    paused_at_utc: Utc::now(),
                    fr_event_kind: FR_EVT_GOODHART_PAUSE.to_string(),
                    history_snapshot: SentinelHistory::default(),
                },
            },
            other => other,
        };
        let json = serde_json::to_string(&decision).unwrap();
        let _back: SentinelDecision = serde_json::from_str(&json).unwrap();
    }
}
