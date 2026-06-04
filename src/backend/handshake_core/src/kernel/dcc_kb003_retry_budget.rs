//! MT-061 Retry Budget.
//!
//! Acceptance (MT-061.json): "retry exhaustion becomes typed BLOCKED/FAILED."
//!
//! Bounds retry behavior for any KB003 lane action (sandbox adapter
//! invocation, validation descriptor evaluation, promotion gate evaluation).
//! Callers register attempts via [`RetryBudget::record_attempt`]; when the
//! budget is exhausted the next decision returns a typed
//! [`RetryBudgetOutcome::ExhaustedBlocked`] or
//! [`RetryBudgetOutcome::ExhaustedFailed`] carrying the bound reason. The
//! caller wires the result into [`crate::kernel::dcc_kb003_blocked_reasons`]
//! when the exhaustion is recoverable, or into a typed FAILED status when it
//! is not.
//!
//! Frontend renders via existing dcc-* IPC surface (RetryBudgetSnapshot is
//! the DCC contract). No app/** edits required.

use serde::{Deserialize, Serialize};

use crate::kernel::dcc_kb003_blocked_reasons::BlockedReason;

/// Static budget configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryBudgetConfigV1 {
    /// Max total attempts (initial + retries).
    pub max_attempts: u32,
    /// Whether exhaustion is recoverable (BLOCKED) or terminal (FAILED).
    pub exhaustion_is_terminal: bool,
}

impl RetryBudgetConfigV1 {
    pub const DEFAULT_SANDBOX: Self = Self {
        max_attempts: 3,
        exhaustion_is_terminal: false,
    };
    pub const DEFAULT_VALIDATION: Self = Self {
        max_attempts: 2,
        exhaustion_is_terminal: false,
    };
    pub const DEFAULT_PROMOTION: Self = Self {
        max_attempts: 1,
        exhaustion_is_terminal: true,
    };

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_attempts == 0 {
            return Err("max_attempts must be >= 1");
        }
        Ok(())
    }
}

/// Per-attempt record persisted alongside the budget so a no-context model
/// can read the history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryAttemptV1 {
    pub attempt_index: u32,
    pub outcome_tag: String,
    pub failure_reason_short: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryBudget {
    pub config: RetryBudgetConfigV1,
    pub attempts: Vec<RetryAttemptV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetryBudgetOutcome {
    /// More attempts allowed.
    AttemptsRemaining { used: u32, remaining: u32 },
    /// Budget exhausted; exhaustion is recoverable — caller should emit a
    /// typed BlockedReason.
    ExhaustedBlocked {
        used: u32,
        last_reason: BlockedReason,
    },
    /// Budget exhausted; exhaustion is terminal — caller should emit a typed
    /// FAILED status (no retry).
    ExhaustedFailed {
        used: u32,
        last_reason: BlockedReason,
    },
}

impl RetryBudget {
    pub fn new(config: RetryBudgetConfigV1) -> Result<Self, &'static str> {
        config.validate()?;
        Ok(Self {
            config,
            attempts: Vec::new(),
        })
    }

    pub fn used(&self) -> u32 {
        self.attempts.len() as u32
    }

    pub fn remaining(&self) -> u32 {
        self.config.max_attempts.saturating_sub(self.used())
    }

    /// Record an attempt outcome. The caller is the runner; this method does
    /// not itself perform the action.
    pub fn record_attempt(
        &mut self,
        outcome_tag: impl Into<String>,
        failure_reason_short: Option<String>,
    ) {
        let idx = self.used();
        self.attempts.push(RetryAttemptV1 {
            attempt_index: idx,
            outcome_tag: outcome_tag.into(),
            failure_reason_short,
        });
    }

    /// Decide what comes next given the last failure reason. The reason is
    /// folded into the typed exhausted outcome.
    pub fn decide_next(&self, last_reason: BlockedReason) -> RetryBudgetOutcome {
        if self.remaining() > 0 {
            RetryBudgetOutcome::AttemptsRemaining {
                used: self.used(),
                remaining: self.remaining(),
            }
        } else if self.config.exhaustion_is_terminal {
            RetryBudgetOutcome::ExhaustedFailed {
                used: self.used(),
                last_reason,
            }
        } else {
            RetryBudgetOutcome::ExhaustedBlocked {
                used: self.used(),
                last_reason,
            }
        }
    }
}

/// DCC projection-friendly snapshot of a retry budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003RetryBudgetSnapshotV1 {
    pub used: u32,
    pub remaining: u32,
    pub max_attempts: u32,
    pub exhaustion_is_terminal: bool,
    pub last_attempt: Option<RetryAttemptV1>,
}

impl DccKb003RetryBudgetSnapshotV1 {
    pub fn from_budget(b: &RetryBudget) -> Self {
        Self {
            used: b.used(),
            remaining: b.remaining(),
            max_attempts: b.config.max_attempts,
            exhaustion_is_terminal: b.config.exhaustion_is_terminal,
            last_attempt: b.attempts.last().cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::dcc_kb003_blocked_reasons::BlockedReason;

    fn reason() -> BlockedReason {
        BlockedReason::ResourceCapExceeded {
            cap_kind: "wall_time_s".into(),
            observed: "60".into(),
            limit: "30".into(),
        }
    }

    #[test]
    fn config_rejects_zero_attempts() {
        let cfg = RetryBudgetConfigV1 {
            max_attempts: 0,
            exhaustion_is_terminal: false,
        };
        assert!(RetryBudget::new(cfg).is_err());
    }

    #[test]
    fn within_budget_yields_attempts_remaining() {
        let mut b = RetryBudget::new(RetryBudgetConfigV1::DEFAULT_SANDBOX).unwrap();
        b.record_attempt("fail", Some("transient".into()));
        let out = b.decide_next(reason());
        assert!(matches!(
            out,
            RetryBudgetOutcome::AttemptsRemaining {
                used: 1,
                remaining: 2
            }
        ));
    }

    #[test]
    fn recoverable_exhaustion_becomes_typed_blocked() {
        let mut b = RetryBudget::new(RetryBudgetConfigV1::DEFAULT_SANDBOX).unwrap();
        for _ in 0..3 {
            b.record_attempt("fail", Some("transient".into()));
        }
        let out = b.decide_next(reason());
        match out {
            RetryBudgetOutcome::ExhaustedBlocked { used, last_reason } => {
                assert_eq!(used, 3);
                assert_eq!(last_reason.tag(), "BLOCKED_RESOURCE_CAP_EXCEEDED");
            }
            other => panic!("expected ExhaustedBlocked, got {other:?}"),
        }
    }

    #[test]
    fn terminal_exhaustion_becomes_typed_failed() {
        let mut b = RetryBudget::new(RetryBudgetConfigV1::DEFAULT_PROMOTION).unwrap();
        b.record_attempt("fail", None);
        let out = b.decide_next(reason());
        assert!(matches!(out, RetryBudgetOutcome::ExhaustedFailed { .. }));
    }

    #[test]
    fn snapshot_mirrors_budget_state() {
        let mut b = RetryBudget::new(RetryBudgetConfigV1::DEFAULT_VALIDATION).unwrap();
        b.record_attempt("ok", None);
        let snap = DccKb003RetryBudgetSnapshotV1::from_budget(&b);
        assert_eq!(snap.used, 1);
        assert_eq!(snap.remaining, 1);
        assert_eq!(snap.max_attempts, 2);
        assert!(snap.last_attempt.is_some());
    }
}
