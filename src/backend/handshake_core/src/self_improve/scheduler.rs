//! MT-156: LoopScheduler with HBR-SWARM-002 cap (25 iterations / 24h
//! rolling window) and pending-review gating.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

use super::goodhart_sentinel::SentinelReceipt;
use super::promotion_gate_adapter::PromotionTicket;

pub const FR_EVT_DISTILL_LOOP_CAP: &str = "FR-EVT-DISTILL-LOOP-CAP";

/// IterationBudget controls the rolling window cap. Defaults to
/// HBR-SWARM-002 (25 / 24h).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IterationBudget {
    pub max_iterations_per_24h: u32,
    pub rolling_window_seconds: u64,
}

impl Default for IterationBudget {
    fn default() -> Self {
        Self {
            max_iterations_per_24h: 25,
            rolling_window_seconds: 86400,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerHistoryEntry {
    pub iteration_id: Uuid,
    pub scheduled_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerHistory {
    pub entries: VecDeque<SchedulerHistoryEntry>,
}

impl SchedulerHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the count of entries within the window ending at `now`.
    pub fn count_in_window(&self, now: DateTime<Utc>, window_seconds: u64) -> usize {
        let cutoff = now - Duration::seconds(window_seconds as i64);
        self.entries
            .iter()
            .filter(|e| e.scheduled_at_utc > cutoff)
            .count()
    }

    pub fn push(&mut self, entry: SchedulerHistoryEntry) {
        self.entries.push_back(entry);
        // Bound at 128 entries to keep memory bounded across long runs.
        while self.entries.len() > 128 {
            self.entries.pop_front();
        }
    }

    pub fn window_started_at(
        &self,
        now: DateTime<Utc>,
        window_seconds: u64,
    ) -> Option<DateTime<Utc>> {
        let cutoff = now - Duration::seconds(window_seconds as i64);
        self.entries
            .iter()
            .find(|e| e.scheduled_at_utc > cutoff)
            .map(|e| e.scheduled_at_utc)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum ScheduleDecision {
    Schedule,
    Skip { reason: SkipReason },
}

impl ScheduleDecision {
    pub fn is_schedule(&self) -> bool {
        matches!(self, Self::Schedule)
    }

    pub fn skip_reason(&self) -> Option<&SkipReason> {
        match self {
            Self::Schedule => None,
            Self::Skip { reason } => Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum SkipReason {
    BudgetExhausted {
        count_in_window: u32,
        cap: u32,
        window_started_at_utc: DateTime<Utc>,
        next_eligible_at_utc: DateTime<Utc>,
    },
    GoodhartPause {
        receipt: SentinelReceipt,
    },
    OperatorPause {
        rationale: String,
    },
    PendingPromotionReview {
        iteration_id: Uuid,
        ticket: PromotionTicket,
    },
}

/// Flight recorder event that the scheduler emits on BudgetExhausted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopCapEvent {
    pub kind: String,
    pub count_in_window: u32,
    pub cap: u32,
    pub window_started_at_utc: DateTime<Utc>,
    pub next_eligible_at_utc: DateTime<Utc>,
}

/// LoopScheduler core type — pure given a fixed clock; tests inject a
/// fixed clock to drive deterministic scenarios.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopScheduler {
    pub budget: IterationBudget,
}

impl LoopScheduler {
    pub fn new(budget: IterationBudget) -> Self {
        Self { budget }
    }

    pub fn with_defaults() -> Self {
        Self {
            budget: IterationBudget::default(),
        }
    }

    /// try_schedule_next: returns Schedule or Skip with a typed reason.
    /// The caller is the only entry point that may invoke
    /// `LoopCore::run_one_iteration`.
    pub fn try_schedule_next(
        &self,
        history: &SchedulerHistory,
        now: DateTime<Utc>,
        operator_paused: Option<&str>,
        goodhart_pause: Option<&SentinelReceipt>,
        pending_review: Option<(Uuid, PromotionTicket)>,
    ) -> ScheduleDecision {
        // 1) Operator pause — highest priority.
        if let Some(rationale) = operator_paused {
            return ScheduleDecision::Skip {
                reason: SkipReason::OperatorPause {
                    rationale: rationale.to_string(),
                },
            };
        }

        // 2) Goodhart pause.
        if let Some(receipt) = goodhart_pause {
            return ScheduleDecision::Skip {
                reason: SkipReason::GoodhartPause {
                    receipt: receipt.clone(),
                },
            };
        }

        // 3) Pending review for a prior iteration.
        if let Some((iteration_id, ticket)) = pending_review {
            return ScheduleDecision::Skip {
                reason: SkipReason::PendingPromotionReview {
                    iteration_id,
                    ticket,
                },
            };
        }

        // 4) Budget rolling-window cap.
        let count = history.count_in_window(now, self.budget.rolling_window_seconds);
        if count as u32 >= self.budget.max_iterations_per_24h {
            let window_started_at_utc = history
                .window_started_at(now, self.budget.rolling_window_seconds)
                .unwrap_or(now);
            let next_eligible_at_utc = window_started_at_utc
                + Duration::seconds(self.budget.rolling_window_seconds as i64);
            return ScheduleDecision::Skip {
                reason: SkipReason::BudgetExhausted {
                    count_in_window: count as u32,
                    cap: self.budget.max_iterations_per_24h,
                    window_started_at_utc,
                    next_eligible_at_utc,
                },
            };
        }

        ScheduleDecision::Schedule
    }

    /// Build a LoopCapEvent from a SkipReason::BudgetExhausted. Returns
    /// None for other skip reasons.
    pub fn cap_event(reason: &SkipReason) -> Option<LoopCapEvent> {
        if let SkipReason::BudgetExhausted {
            count_in_window,
            cap,
            window_started_at_utc,
            next_eligible_at_utc,
        } = reason
        {
            Some(LoopCapEvent {
                kind: FR_EVT_DISTILL_LOOP_CAP.to_string(),
                count_in_window: *count_in_window,
                cap: *cap,
                window_started_at_utc: *window_started_at_utc,
                next_eligible_at_utc: *next_eligible_at_utc,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::self_improve::goodhart_sentinel::{SentinelHistory, FR_EVT_GOODHART_PAUSE};

    fn ts(seconds_ago: i64) -> DateTime<Utc> {
        Utc::now() - Duration::seconds(seconds_ago)
    }

    #[test]
    fn schedule_when_history_empty() {
        let scheduler = LoopScheduler::with_defaults();
        let history = SchedulerHistory::new();
        let d = scheduler.try_schedule_next(&history, Utc::now(), None, None, None);
        assert!(d.is_schedule());
    }

    #[test]
    fn skip_at_cap() {
        let scheduler = LoopScheduler::with_defaults();
        let now = Utc::now();
        let mut history = SchedulerHistory::new();
        for _ in 0..25 {
            history.push(SchedulerHistoryEntry {
                iteration_id: Uuid::now_v7(),
                scheduled_at_utc: now - Duration::seconds(30),
            });
        }
        let d = scheduler.try_schedule_next(&history, now, None, None, None);
        match d {
            ScheduleDecision::Skip { reason } => match reason {
                SkipReason::BudgetExhausted {
                    count_in_window,
                    cap,
                    ..
                } => {
                    assert_eq!(count_in_window, 25);
                    assert_eq!(cap, 25);
                }
                _ => panic!("expected BudgetExhausted"),
            },
            _ => panic!("expected Skip"),
        }
    }

    #[test]
    fn rolling_window_drops_old_entries() {
        let scheduler = LoopScheduler::with_defaults();
        let now = Utc::now();
        let mut history = SchedulerHistory::new();
        // 25 entries older than 24h
        for _ in 0..25 {
            history.push(SchedulerHistoryEntry {
                iteration_id: Uuid::now_v7(),
                scheduled_at_utc: now - Duration::seconds(86_500), // > 24h
            });
        }
        // Now within-window count is 0, so we can schedule.
        let d = scheduler.try_schedule_next(&history, now, None, None, None);
        assert!(d.is_schedule());
    }

    #[test]
    fn operator_pause_blocks_schedule() {
        let scheduler = LoopScheduler::with_defaults();
        let history = SchedulerHistory::new();
        let d = scheduler.try_schedule_next(&history, Utc::now(), Some("manual"), None, None);
        match d.skip_reason() {
            Some(SkipReason::OperatorPause { rationale }) => assert_eq!(rationale, "manual"),
            _ => panic!("expected OperatorPause"),
        }
    }

    #[test]
    fn goodhart_pause_blocks_schedule() {
        let scheduler = LoopScheduler::with_defaults();
        let history = SchedulerHistory::new();
        let receipt = SentinelReceipt {
            receipt_id: Uuid::now_v7(),
            paused_at_utc: Utc::now(),
            fr_event_kind: FR_EVT_GOODHART_PAUSE.to_string(),
            history_snapshot: SentinelHistory::default(),
        };
        let d = scheduler.try_schedule_next(&history, Utc::now(), None, Some(&receipt), None);
        assert!(matches!(
            d.skip_reason(),
            Some(SkipReason::GoodhartPause { .. })
        ));
    }

    #[test]
    fn pending_review_blocks_schedule() {
        let scheduler = LoopScheduler::with_defaults();
        let history = SchedulerHistory::new();
        let ticket = PromotionTicket {
            ticket_id: Uuid::now_v7(),
            iteration_id: Uuid::now_v7(),
            submitted_at_utc: Utc::now(),
        };
        let d = scheduler.try_schedule_next(
            &history,
            Utc::now(),
            None,
            None,
            Some((ticket.iteration_id, ticket.clone())),
        );
        assert!(matches!(
            d.skip_reason(),
            Some(SkipReason::PendingPromotionReview { .. })
        ));
    }

    #[test]
    fn cap_event_built_from_budget_exhausted() {
        let reason = SkipReason::BudgetExhausted {
            count_in_window: 25,
            cap: 25,
            window_started_at_utc: ts(86_000),
            next_eligible_at_utc: Utc::now(),
        };
        let event = LoopScheduler::cap_event(&reason).unwrap();
        assert_eq!(event.kind, FR_EVT_DISTILL_LOOP_CAP);
        assert_eq!(event.count_in_window, 25);
        assert_eq!(event.cap, 25);
    }

    #[test]
    fn cap_event_returns_none_for_other_skips() {
        let reason = SkipReason::OperatorPause {
            rationale: "x".to_string(),
        };
        assert!(LoopScheduler::cap_event(&reason).is_none());
    }
}
