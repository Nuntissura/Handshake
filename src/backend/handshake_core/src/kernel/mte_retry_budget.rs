//! MTE Retry Budget (companion to MT-052 / MT-054 / MT-059).
//!
//! Captures the per-MT retry budget the MTE scheduler consults when an
//! attempt fails or is blocked with `MteBlockedHandling::Retry`. The budget
//! is *not* a `kb003_promotion` concept; it is the scheduler-level policy
//! that decides whether to backoff-and-retry, escalate, or hold.
//!
//! Why this lives next to `mte_blocked_taxonomy.rs`: handled-by-retry
//! reasons (`CapacityExceeded`, transient `PostgresFailure`) flow through
//! this budget; handled-by-gate or handled-by-escalation reasons do not.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackoffStrategy {
    Constant,
    Linear,
    Exponential,
}

impl BackoffStrategy {
    /// Compute backoff seconds for `attempt` (0-indexed) with `base`.
    pub fn backoff_seconds(&self, base: u32, attempt: u32) -> u32 {
        match self {
            Self::Constant => base,
            Self::Linear => base.saturating_mul(attempt.saturating_add(1)),
            Self::Exponential => {
                // base * 2^attempt, clamped to u32 range without overflow.
                let shift = attempt.min(31);
                base.saturating_mul(1u32.checked_shl(shift).unwrap_or(u32::MAX))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MteRetryBudgetV1 {
    pub schema_version: String,
    pub max_attempts: u32,
    pub base_backoff_seconds: u32,
    pub backoff_strategy: BackoffStrategy,
}

impl MteRetryBudgetV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.mte.retry_budget@1";

    pub fn new(
        max_attempts: u32,
        base_backoff_seconds: u32,
        backoff_strategy: BackoffStrategy,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            max_attempts,
            base_backoff_seconds,
            backoff_strategy,
        }
    }

    pub fn default_lane() -> Self {
        Self::new(3, 5, BackoffStrategy::Exponential)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteRetryVerdict {
    Allow {
        next_attempt: u32,
        backoff_seconds: u32,
    },
    Declined {
        reason: &'static str,
    },
}

impl MteRetryVerdict {
    pub fn allows_retry(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}

pub struct MteRetryBudgetCheck;

impl MteRetryBudgetCheck {
    /// Decide whether `attempts_so_far` permits another retry under `budget`.
    pub fn evaluate(budget: &MteRetryBudgetV1, attempts_so_far: u32) -> MteRetryVerdict {
        if attempts_so_far >= budget.max_attempts {
            return MteRetryVerdict::Declined {
                reason: "retry budget exhausted",
            };
        }
        MteRetryVerdict::Allow {
            next_attempt: attempts_so_far.saturating_add(1),
            backoff_seconds: budget
                .backoff_strategy
                .backoff_seconds(budget.base_backoff_seconds, attempts_so_far),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_allows_retries_under_cap() {
        let b = MteRetryBudgetV1::new(3, 2, BackoffStrategy::Linear);
        let v = MteRetryBudgetCheck::evaluate(&b, 0);
        match v {
            MteRetryVerdict::Allow {
                next_attempt,
                backoff_seconds,
            } => {
                assert_eq!(next_attempt, 1);
                assert_eq!(backoff_seconds, 2);
            }
            other => panic!("expected Allow, got {other:?}"),
        }
    }

    #[test]
    fn exhausted_budget_declines() {
        let b = MteRetryBudgetV1::new(2, 1, BackoffStrategy::Constant);
        let v = MteRetryBudgetCheck::evaluate(&b, 2);
        assert!(!v.allows_retry());
    }

    #[test]
    fn linear_backoff_scales_with_attempts() {
        let s = BackoffStrategy::Linear;
        assert_eq!(s.backoff_seconds(5, 0), 5);
        assert_eq!(s.backoff_seconds(5, 1), 10);
        assert_eq!(s.backoff_seconds(5, 3), 20);
    }

    #[test]
    fn exponential_backoff_doubles() {
        let s = BackoffStrategy::Exponential;
        assert_eq!(s.backoff_seconds(1, 0), 1);
        assert_eq!(s.backoff_seconds(1, 1), 2);
        assert_eq!(s.backoff_seconds(1, 3), 8);
    }

    #[test]
    fn constant_backoff_never_changes() {
        let s = BackoffStrategy::Constant;
        for i in 0..5 {
            assert_eq!(s.backoff_seconds(7, i), 7);
        }
    }

    #[test]
    fn default_lane_is_three_attempts_exponential() {
        let b = MteRetryBudgetV1::default_lane();
        assert_eq!(b.max_attempts, 3);
        assert!(matches!(b.backoff_strategy, BackoffStrategy::Exponential));
    }
}
