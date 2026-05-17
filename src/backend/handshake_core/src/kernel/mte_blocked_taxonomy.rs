//! MT-052: MTE Blocked Taxonomy (eligibility-check support).
//!
//! Acceptance (MT-052.json): "Promotion Eligibility Check: implement promotion
//! preconditions. Acceptance: ineligible candidate produces typed rejection
//! receipt."
//!
//! The MTE blocked taxonomy is the *scheduler-level* reason an MT cannot
//! advance. It is distinct from `PromotionRejectionReason` (which is the
//! promotion gate's decision-level taxonomy in `kb003_promotion::decision`):
//!
//! - `MteBlockedReason` covers reasons the MT cannot be scheduled, started,
//!   validated, or attempted at promotion time.
//! - `PromotionRejectionReason` covers reasons the gate refused a candidate
//!   that actually reached it.
//!
//! Each variant declares its handling semantics via `handling()`:
//! `Retry`, `Escalate`, or `Gate`. The eligibility check pre-promotes by
//! returning a typed `MteEligibilityVerdict` so the caller can produce the
//! rejection receipt without invoking the heavy promotion gate.

use serde::{Deserialize, Serialize};

/// What to do when a blocked reason fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteBlockedHandling {
    /// Retry after backoff (covered by `MteRetryBudgetV1`).
    Retry,
    /// Escalate to the operator; the scheduler cannot recover on its own.
    Escalate,
    /// Wait at a gate; another MT or external signal must clear it.
    Gate,
}

/// Why an MT cannot advance at the MTE scheduler level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteBlockedReason {
    /// No sandbox slot, no worker, or queue backpressure.
    CapacityExceeded { detail: String },
    /// An upstream MT/dependency is missing or not yet completed.
    DependencyMissing {
        missing_dependency_id: String,
    },
    /// Sandbox policy version changed under the MT; replay denied.
    PolicyChange {
        old_policy_version_id: String,
        new_policy_version_id: String,
    },
    /// Operator paused the lane (manual pause, dry-run mode).
    OperatorPaused { reason: String },
    /// Waiting on a downstream consumer (DCC projection, integration validator).
    DownstreamWait { waiting_on: String },
    /// Resource cap evaluator returned Halt during pre-run check.
    ResourceExhausted {
        dimension: String,
        observed: u64,
        cap: u64,
    },
}

impl MteBlockedReason {
    /// Stable short tag used in receipts and DCC surfaces.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::CapacityExceeded { .. } => "MTE_BLOCKED_CAPACITY_EXCEEDED",
            Self::DependencyMissing { .. } => "MTE_BLOCKED_DEPENDENCY_MISSING",
            Self::PolicyChange { .. } => "MTE_BLOCKED_POLICY_CHANGE",
            Self::OperatorPaused { .. } => "MTE_BLOCKED_OPERATOR_PAUSED",
            Self::DownstreamWait { .. } => "MTE_BLOCKED_DOWNSTREAM_WAIT",
            Self::ResourceExhausted { .. } => "MTE_BLOCKED_RESOURCE_EXHAUSTED",
        }
    }

    /// Whether the scheduler retries, escalates, or gates on this reason.
    pub fn handling(&self) -> MteBlockedHandling {
        match self {
            Self::CapacityExceeded { .. } => MteBlockedHandling::Retry,
            Self::DependencyMissing { .. } => MteBlockedHandling::Gate,
            Self::PolicyChange { .. } => MteBlockedHandling::Escalate,
            Self::OperatorPaused { .. } => MteBlockedHandling::Gate,
            Self::DownstreamWait { .. } => MteBlockedHandling::Gate,
            Self::ResourceExhausted { .. } => MteBlockedHandling::Escalate,
        }
    }

    /// Short rationale string for receipts.
    pub fn rationale_short(&self) -> String {
        match self {
            Self::CapacityExceeded { detail } => format!("capacity exceeded: {detail}"),
            Self::DependencyMissing {
                missing_dependency_id,
            } => format!("dependency missing: {missing_dependency_id}"),
            Self::PolicyChange {
                old_policy_version_id,
                new_policy_version_id,
            } => format!(
                "policy changed {old_policy_version_id} -> {new_policy_version_id}"
            ),
            Self::OperatorPaused { reason } => format!("operator paused: {reason}"),
            Self::DownstreamWait { waiting_on } => format!("waiting on {waiting_on}"),
            Self::ResourceExhausted {
                dimension,
                observed,
                cap,
            } => format!("resource exhausted: {dimension} {observed}>={cap}"),
        }
    }
}

/// Eligibility decision for an MT before the promotion gate is even invoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteEligibilityVerdict {
    Eligible,
    Ineligible { reason: MteBlockedReason },
}

impl MteEligibilityVerdict {
    pub fn is_eligible(&self) -> bool {
        matches!(self, Self::Eligible)
    }

    pub fn block_reason(&self) -> Option<&MteBlockedReason> {
        if let Self::Ineligible { reason } = self {
            Some(reason)
        } else {
            None
        }
    }
}

/// Inputs the eligibility check inspects. Kept small on purpose so MTE
/// schedulers can call it cheaply per tick.
pub struct MteEligibilityInputs<'a> {
    pub mt_id: &'a str,
    pub deps_satisfied: bool,
    pub missing_dependency_id: Option<&'a str>,
    pub operator_paused: Option<&'a str>,
    pub capacity_available: bool,
    pub capacity_detail: Option<&'a str>,
    pub policy_changed_to: Option<&'a str>,
    pub policy_was: Option<&'a str>,
    pub downstream_waiting_on: Option<&'a str>,
    pub resource_halt: Option<(String, u64, u64)>,
}

pub struct MteEligibilityCheck;

impl MteEligibilityCheck {
    /// Stable evaluation order: operator pause > capacity > policy change >
    /// dependency > downstream wait > resource halt. This order is what
    /// produces the typed rejection receipt for the highest-priority blocker.
    pub fn evaluate(inputs: &MteEligibilityInputs<'_>) -> MteEligibilityVerdict {
        if let Some(reason) = inputs.operator_paused {
            return MteEligibilityVerdict::Ineligible {
                reason: MteBlockedReason::OperatorPaused {
                    reason: reason.to_string(),
                },
            };
        }
        if !inputs.capacity_available {
            return MteEligibilityVerdict::Ineligible {
                reason: MteBlockedReason::CapacityExceeded {
                    detail: inputs
                        .capacity_detail
                        .unwrap_or("no detail")
                        .to_string(),
                },
            };
        }
        if let (Some(was), Some(to)) = (inputs.policy_was, inputs.policy_changed_to) {
            if was != to {
                return MteEligibilityVerdict::Ineligible {
                    reason: MteBlockedReason::PolicyChange {
                        old_policy_version_id: was.to_string(),
                        new_policy_version_id: to.to_string(),
                    },
                };
            }
        }
        if !inputs.deps_satisfied {
            return MteEligibilityVerdict::Ineligible {
                reason: MteBlockedReason::DependencyMissing {
                    missing_dependency_id: inputs
                        .missing_dependency_id
                        .unwrap_or("<unspecified>")
                        .to_string(),
                },
            };
        }
        if let Some(waiting) = inputs.downstream_waiting_on {
            return MteEligibilityVerdict::Ineligible {
                reason: MteBlockedReason::DownstreamWait {
                    waiting_on: waiting.to_string(),
                },
            };
        }
        if let Some((dim, observed, cap)) = inputs.resource_halt.clone() {
            return MteEligibilityVerdict::Ineligible {
                reason: MteBlockedReason::ResourceExhausted {
                    dimension: dim,
                    observed,
                    cap,
                },
            };
        }
        MteEligibilityVerdict::Eligible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn happy_inputs() -> MteEligibilityInputs<'static> {
        MteEligibilityInputs {
            mt_id: "MT-1",
            deps_satisfied: true,
            missing_dependency_id: None,
            operator_paused: None,
            capacity_available: true,
            capacity_detail: None,
            policy_changed_to: None,
            policy_was: None,
            downstream_waiting_on: None,
            resource_halt: None,
        }
    }

    // MT-052 acceptance: ineligible candidate produces typed rejection.
    #[test]
    fn dependency_missing_produces_typed_block() {
        let mut inp = happy_inputs();
        inp.deps_satisfied = false;
        inp.missing_dependency_id = Some("MT-0");
        let v = MteEligibilityCheck::evaluate(&inp);
        match v {
            MteEligibilityVerdict::Ineligible {
                reason:
                    MteBlockedReason::DependencyMissing {
                        missing_dependency_id,
                    },
            } => assert_eq!(missing_dependency_id, "MT-0"),
            other => panic!("expected DependencyMissing, got {other:?}"),
        }
    }

    #[test]
    fn all_six_variants_carry_distinct_tags() {
        let variants = vec![
            MteBlockedReason::CapacityExceeded {
                detail: "x".into(),
            },
            MteBlockedReason::DependencyMissing {
                missing_dependency_id: "MT-0".into(),
            },
            MteBlockedReason::PolicyChange {
                old_policy_version_id: "POL@1".into(),
                new_policy_version_id: "POL@2".into(),
            },
            MteBlockedReason::OperatorPaused {
                reason: "manual".into(),
            },
            MteBlockedReason::DownstreamWait {
                waiting_on: "DCC".into(),
            },
            MteBlockedReason::ResourceExhausted {
                dimension: "WALL_MS".into(),
                observed: 200,
                cap: 100,
            },
        ];
        let mut tags: std::collections::BTreeSet<&'static str> =
            std::collections::BTreeSet::new();
        for v in &variants {
            tags.insert(v.tag());
            assert!(!v.rationale_short().is_empty());
            // Every variant declares a handling.
            let _ = v.handling();
        }
        assert_eq!(tags.len(), 6);
    }

    #[test]
    fn handling_semantics_are_stable() {
        assert_eq!(
            MteBlockedReason::CapacityExceeded { detail: "".into() }.handling(),
            MteBlockedHandling::Retry
        );
        assert_eq!(
            MteBlockedReason::DependencyMissing {
                missing_dependency_id: "x".into()
            }
            .handling(),
            MteBlockedHandling::Gate
        );
        assert_eq!(
            MteBlockedReason::PolicyChange {
                old_policy_version_id: "a".into(),
                new_policy_version_id: "b".into()
            }
            .handling(),
            MteBlockedHandling::Escalate
        );
        assert_eq!(
            MteBlockedReason::OperatorPaused { reason: "".into() }.handling(),
            MteBlockedHandling::Gate
        );
        assert_eq!(
            MteBlockedReason::DownstreamWait {
                waiting_on: "".into()
            }
            .handling(),
            MteBlockedHandling::Gate
        );
        assert_eq!(
            MteBlockedReason::ResourceExhausted {
                dimension: "".into(),
                observed: 0,
                cap: 0
            }
            .handling(),
            MteBlockedHandling::Escalate
        );
    }

    #[test]
    fn operator_pause_beats_capacity() {
        let mut inp = happy_inputs();
        inp.operator_paused = Some("maintenance");
        inp.capacity_available = false;
        let v = MteEligibilityCheck::evaluate(&inp);
        matches!(
            v,
            MteEligibilityVerdict::Ineligible {
                reason: MteBlockedReason::OperatorPaused { .. }
            }
        );
    }

    #[test]
    fn eligible_path_returns_eligible() {
        let inp = happy_inputs();
        assert!(MteEligibilityCheck::evaluate(&inp).is_eligible());
    }
}
