//! MT-062 Smart DropBack semantics.
//!
//! Acceptance (MT-062.json): "smart/always/never modes have test coverage."
//!
//! DropBack governs whether a failed/blocked KB003 lane attempt should
//! revert (drop back) to a prior safe state — for sandbox, that means
//! cleaning the workspace; for validation, that means discarding partial
//! evidence; for promotion, that means rolling back any provisional
//! receipt.
//!
//! Three modes:
//! - `Never`  — never drop back; preserve partial state for forensic review.
//! - `Always` — always drop back on failure/blocked.
//! - `Smart`  — drop back only when the failure is recoverable (escalate or
//!   retry disposition), preserve state when the failure is a hard gate so
//!   the operator has full context. Smart mode reads the
//!   [`BlockedDisposition`] from the typed [`BlockedReason`].
//!
//! Frontend renders via existing dcc-* IPC surface.

use serde::{Deserialize, Serialize};

use crate::kernel::dcc_kb003_blocked_reasons::{BlockedDisposition, BlockedReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DropBackMode {
    Never,
    Always,
    Smart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DropBackDecision {
    /// Roll back to prior safe state.
    Rollback,
    /// Keep partial state intact.
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropBackPolicyV1 {
    pub mode: DropBackMode,
}

impl DropBackPolicyV1 {
    pub const fn never() -> Self {
        Self {
            mode: DropBackMode::Never,
        }
    }
    pub const fn always() -> Self {
        Self {
            mode: DropBackMode::Always,
        }
    }
    pub const fn smart() -> Self {
        Self {
            mode: DropBackMode::Smart,
        }
    }

    /// Decide whether to rollback given the failure reason. Pure function;
    /// callers persist the decision into the per-MT summary (MT-063).
    pub fn decide(&self, reason: &BlockedReason) -> DropBackDecision {
        match self.mode {
            DropBackMode::Never => DropBackDecision::Preserve,
            DropBackMode::Always => DropBackDecision::Rollback,
            DropBackMode::Smart => match reason.disposition() {
                // Recoverable disposition → rollback; user can retry cleanly.
                BlockedDisposition::Retry | BlockedDisposition::Escalate => {
                    DropBackDecision::Rollback
                }
                // Hard gate → preserve so operator can inspect partial state.
                BlockedDisposition::Gate => DropBackDecision::Preserve,
            },
        }
    }
}

/// DCC-facing summary of a drop-back evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003DropBackEvaluationV1 {
    pub mode: DropBackMode,
    pub decision: DropBackDecision,
    pub reason_tag: String,
    pub reason_rationale_short: String,
}

impl DccKb003DropBackEvaluationV1 {
    pub fn new(mode: DropBackMode, reason: &BlockedReason) -> Self {
        let decision = DropBackPolicyV1 { mode }.decide(reason);
        Self {
            mode,
            decision,
            reason_tag: reason.tag().to_string(),
            reason_rationale_short: reason.rationale_short(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retry_reason() -> BlockedReason {
        BlockedReason::ResourceCapExceeded {
            cap_kind: "memory".into(),
            observed: "2GB".into(),
            limit: "1GB".into(),
        }
    }
    fn gate_reason() -> BlockedReason {
        BlockedReason::AdapterUnavailable {
            adapter_kind: "microvm".into(),
            host_detail: "no kvm".into(),
        }
    }
    fn escalate_reason() -> BlockedReason {
        BlockedReason::MissingApproval {
            missing_field: "operator_id".into(),
        }
    }

    #[test]
    fn never_mode_always_preserves() {
        let p = DropBackPolicyV1::never();
        assert_eq!(p.decide(&retry_reason()), DropBackDecision::Preserve);
        assert_eq!(p.decide(&gate_reason()), DropBackDecision::Preserve);
        assert_eq!(p.decide(&escalate_reason()), DropBackDecision::Preserve);
    }

    #[test]
    fn always_mode_always_rolls_back() {
        let p = DropBackPolicyV1::always();
        assert_eq!(p.decide(&retry_reason()), DropBackDecision::Rollback);
        assert_eq!(p.decide(&gate_reason()), DropBackDecision::Rollback);
        assert_eq!(p.decide(&escalate_reason()), DropBackDecision::Rollback);
    }

    #[test]
    fn smart_mode_rolls_back_retry_and_escalate_preserves_gate() {
        let p = DropBackPolicyV1::smart();
        assert_eq!(p.decide(&retry_reason()), DropBackDecision::Rollback);
        assert_eq!(p.decide(&escalate_reason()), DropBackDecision::Rollback);
        assert_eq!(p.decide(&gate_reason()), DropBackDecision::Preserve);
    }

    #[test]
    fn dcc_evaluation_carries_typed_reason() {
        let ev = DccKb003DropBackEvaluationV1::new(DropBackMode::Smart, &gate_reason());
        assert_eq!(ev.decision, DropBackDecision::Preserve);
        assert_eq!(ev.reason_tag, "BLOCKED_ADAPTER_UNAVAILABLE");
        assert!(ev.reason_rationale_short.contains("microvm"));
    }
}
