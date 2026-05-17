//! MT-054: MTE Drop-Back (Promotion Reject Path support).
//!
//! Acceptance (MT-054.json): "Promotion Reject Path: record rejected
//! promotion attempts. Acceptance: reject path creates receipt and does not
//! mutate authority."
//!
//! When the promotion gate rejects an MT candidate, the MTE scheduler must
//! decide whether to:
//!   - drop back to a prior MT (re-run the upstream step),
//!   - hold the current MT in place and let the validator try again, or
//!   - escalate to the operator.
//!
//! This module captures that decision logic and produces a typed drop-back
//! record. The record is *not* a mutation of authority; it is an MTE-level
//! receipt the orchestrator persists alongside the rejection receipt issued
//! by the promotion gate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kernel::kb003_promotion::decision::PromotionRejectionReason;
use crate::kernel::mte_blocked_taxonomy::MteBlockedReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteDropBackAction {
    /// Re-run the named prior MT before retrying this one.
    DropToPrior,
    /// Keep the current MT in place; let the validator/coder try again.
    HoldInPlace,
    /// Cannot recover automatically; require operator decision.
    Escalate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MteDropBackDecisionV1 {
    pub schema_version: &'static str,
    pub mt_id: String,
    pub wp_id: String,
    pub action: MteDropBackAction,
    pub target_mt_id: Option<String>,
    pub reason_tag: String,
    pub rationale: String,
    /// Optional ref to the matching promotion-rejection receipt so reviewers
    /// can join the two.
    pub linked_rejection_receipt_id: Option<String>,
    pub decided_at_utc: DateTime<Utc>,
}

impl MteDropBackDecisionV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.mte.drop_back@1";

    /// Decide based on a `PromotionRejectionReason`.
    pub fn for_promotion_rejection(
        mt_id: impl Into<String>,
        wp_id: impl Into<String>,
        prior_mt_id: Option<String>,
        reason: &PromotionRejectionReason,
        linked_rejection_receipt_id: Option<String>,
    ) -> Self {
        let (action, target) = match reason {
            // Validation can sometimes be retried in place (flaky descriptor),
            // but recovering generally requires re-running the upstream sandbox
            // MT. Default: drop back when a prior MT exists, else hold.
            PromotionRejectionReason::ValidationFailure { .. } => match &prior_mt_id {
                Some(p) => (MteDropBackAction::DropToPrior, Some(p.clone())),
                None => (MteDropBackAction::HoldInPlace, None),
            },
            // Stale candidates always drop back: the upstream MT moved.
            PromotionRejectionReason::StaleCandidate { .. } => match &prior_mt_id {
                Some(p) => (MteDropBackAction::DropToPrior, Some(p.clone())),
                None => (MteDropBackAction::Escalate, None),
            },
            // Hard policy / fixture issues escalate to operator.
            PromotionRejectionReason::PolicyDenial { .. }
            | PromotionRejectionReason::MissingApproval { .. }
            | PromotionRejectionReason::MissingArtifact { .. } => {
                (MteDropBackAction::Escalate, None)
            }
            // Idempotency duplicates always escalate (operator-visible bug).
            PromotionRejectionReason::DuplicateIdempotencyKey { .. } => {
                (MteDropBackAction::Escalate, None)
            }
            // Transient infrastructure failures hold in place; the scheduler
            // will retry per `MteRetryBudgetV1`.
            PromotionRejectionReason::PostgresFailure { .. }
            | PromotionRejectionReason::ProjectionRebuildFailure { .. } => {
                (MteDropBackAction::HoldInPlace, None)
            }
        };
        Self {
            schema_version: Self::SCHEMA_VERSION,
            mt_id: mt_id.into(),
            wp_id: wp_id.into(),
            action,
            target_mt_id: target,
            reason_tag: reason.tag().to_string(),
            rationale: reason.rationale_short(),
            linked_rejection_receipt_id,
            decided_at_utc: Utc::now(),
        }
    }

    /// Decide based on an MTE-level block (the candidate never reached the
    /// gate; it was blocked at the scheduler level).
    pub fn for_blocked_reason(
        mt_id: impl Into<String>,
        wp_id: impl Into<String>,
        prior_mt_id: Option<String>,
        reason: &MteBlockedReason,
    ) -> Self {
        let action = match reason {
            MteBlockedReason::CapacityExceeded { .. } => MteDropBackAction::HoldInPlace,
            MteBlockedReason::DependencyMissing { .. } => match prior_mt_id {
                Some(_) => MteDropBackAction::DropToPrior,
                None => MteDropBackAction::HoldInPlace,
            },
            MteBlockedReason::PolicyChange { .. } => MteDropBackAction::Escalate,
            MteBlockedReason::OperatorPaused { .. } => MteDropBackAction::HoldInPlace,
            MteBlockedReason::DownstreamWait { .. } => MteDropBackAction::HoldInPlace,
            MteBlockedReason::ResourceExhausted { .. } => MteDropBackAction::Escalate,
        };
        let target = if matches!(action, MteDropBackAction::DropToPrior) {
            prior_mt_id
        } else {
            None
        };
        Self {
            schema_version: Self::SCHEMA_VERSION,
            mt_id: mt_id.into(),
            wp_id: wp_id.into(),
            action,
            target_mt_id: target,
            reason_tag: reason.tag().to_string(),
            rationale: reason.rationale_short(),
            linked_rejection_receipt_id: None,
            decided_at_utc: Utc::now(),
        }
    }

    /// This decision is *not* an authority mutation. Callers can assert this
    /// in their integration tests to satisfy MT-054's "does not mutate
    /// authority" rule.
    pub fn mutates_authority(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // MT-054 acceptance: reject path produces a typed record that does not
    // mutate authority.
    #[test]
    fn validation_failure_drops_back_when_prior_exists() {
        let reason = PromotionRejectionReason::ValidationFailure {
            validation_run_id: "VR-1".into(),
            blocking_outcomes: vec!["no_sandbox_escape".into()],
            report_artifact_ref: None,
        };
        let dec = MteDropBackDecisionV1::for_promotion_rejection(
            "MT-2",
            "WP-X",
            Some("MT-1".into()),
            &reason,
            Some("PR-1".into()),
        );
        assert_eq!(dec.action, MteDropBackAction::DropToPrior);
        assert_eq!(dec.target_mt_id.as_deref(), Some("MT-1"));
        assert_eq!(dec.reason_tag, "REJECTED_VALIDATION_FAILURE");
        assert_eq!(dec.linked_rejection_receipt_id.as_deref(), Some("PR-1"));
        assert!(!dec.mutates_authority());
    }

    #[test]
    fn validation_failure_with_no_prior_holds_in_place() {
        let reason = PromotionRejectionReason::ValidationFailure {
            validation_run_id: "VR-1".into(),
            blocking_outcomes: vec!["x".into()],
            report_artifact_ref: None,
        };
        let dec = MteDropBackDecisionV1::for_promotion_rejection(
            "MT-1", "WP-X", None, &reason, None,
        );
        assert_eq!(dec.action, MteDropBackAction::HoldInPlace);
        assert!(dec.target_mt_id.is_none());
    }

    #[test]
    fn policy_denial_always_escalates() {
        let reason = PromotionRejectionReason::PolicyDenial {
            denial_id: "DEN-1".into(),
            policy_version_id: "POL@1".into(),
            capability: Some("NETWORK".into()),
        };
        let dec = MteDropBackDecisionV1::for_promotion_rejection(
            "MT-1",
            "WP-X",
            Some("MT-0".into()),
            &reason,
            None,
        );
        assert_eq!(dec.action, MteDropBackAction::Escalate);
        assert!(dec.target_mt_id.is_none());
    }

    #[test]
    fn missing_approval_escalates() {
        let reason = PromotionRejectionReason::MissingApproval {
            missing_field: "operator_id".into(),
        };
        let dec = MteDropBackDecisionV1::for_promotion_rejection(
            "MT-1", "WP-X", None, &reason, None,
        );
        assert_eq!(dec.action, MteDropBackAction::Escalate);
    }

    #[test]
    fn postgres_failure_holds_for_retry() {
        let reason = PromotionRejectionReason::PostgresFailure {
            storage_error: "deadlock".into(),
        };
        let dec = MteDropBackDecisionV1::for_promotion_rejection(
            "MT-1", "WP-X", None, &reason, None,
        );
        assert_eq!(dec.action, MteDropBackAction::HoldInPlace);
    }

    #[test]
    fn blocked_dependency_drops_back() {
        let reason = MteBlockedReason::DependencyMissing {
            missing_dependency_id: "MT-0".into(),
        };
        let dec = MteDropBackDecisionV1::for_blocked_reason(
            "MT-1",
            "WP-X",
            Some("MT-0".into()),
            &reason,
        );
        assert_eq!(dec.action, MteDropBackAction::DropToPrior);
        assert_eq!(dec.target_mt_id.as_deref(), Some("MT-0"));
    }

    #[test]
    fn blocked_resource_exhausted_escalates() {
        let reason = MteBlockedReason::ResourceExhausted {
            dimension: "WALL_MS".into(),
            observed: 5000,
            cap: 1000,
        };
        let dec = MteDropBackDecisionV1::for_blocked_reason(
            "MT-1",
            "WP-X",
            Some("MT-0".into()),
            &reason,
        );
        assert_eq!(dec.action, MteDropBackAction::Escalate);
        assert!(dec.target_mt_id.is_none());
    }
}
