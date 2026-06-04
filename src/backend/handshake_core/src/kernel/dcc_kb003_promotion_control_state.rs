//! MT-069 DCC Promotion Control State.
//!
//! Acceptance (MT-069.json): "UI/API cannot promote when eligibility is
//! false."
//!
//! Surfaces typed eligibility + approval state for the promotion control on
//! the DCC sandbox lane. Any UI or API that triggers promotion must consult
//! this projection first; the [`PromotionEligibility::can_promote`] helper
//! is the single source-of-truth for the operator-side action.
//!
//! The eligibility computation is intentionally conservative: any missing
//! input (validation, sandbox completion, approval gate) yields
//! [`PromotionEligibility::Ineligible`] with a typed reason. The frontend
//! renders this projection through the existing dcc-* IPC surface; no
//! app/** edits required.

use serde::{Deserialize, Serialize};

use crate::kernel::sandbox::dcc_projection::{DccSandboxOutcome, DccSandboxProjectionV1};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionIneligibilityReason {
    SandboxNotCompleted { current_outcome: String },
    ValidationMissing,
    ValidationBlocked { verdict: String },
    AlreadyPromoted { decision_id: String },
    OperatorApprovalRequired { missing_field: String },
}

impl PromotionIneligibilityReason {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::SandboxNotCompleted { .. } => "INELIGIBLE_SANDBOX_NOT_COMPLETED",
            Self::ValidationMissing => "INELIGIBLE_VALIDATION_MISSING",
            Self::ValidationBlocked { .. } => "INELIGIBLE_VALIDATION_BLOCKED",
            Self::AlreadyPromoted { .. } => "INELIGIBLE_ALREADY_PROMOTED",
            Self::OperatorApprovalRequired { .. } => "INELIGIBLE_OPERATOR_APPROVAL_REQUIRED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionEligibility {
    Eligible,
    Ineligible {
        reasons: Vec<PromotionIneligibilityReason>,
    },
}

impl PromotionEligibility {
    /// Authoritative gate: returns true iff the operator's UI/API path may
    /// trigger promotion. Callers must consult this before exposing the
    /// promote button.
    pub fn can_promote(&self) -> bool {
        matches!(self, Self::Eligible)
    }

    pub fn add(&mut self, reason: PromotionIneligibilityReason) {
        match self {
            Self::Eligible => {
                *self = Self::Ineligible {
                    reasons: vec![reason],
                }
            }
            Self::Ineligible { reasons } => reasons.push(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003PromotionControlStateV1 {
    pub projection_family_id: String,
    pub sandbox_run_id: String,
    pub eligibility: PromotionEligibility,
    pub approval_received: bool,
    pub operator_approval_field_required: Option<String>,
}

impl DccKb003PromotionControlStateV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.promotion_control_state@1";

    /// Derive control state from the sandbox projection + a boolean
    /// indicating whether the operator's approval evidence is present.
    pub fn derive(
        projection: &DccSandboxProjectionV1,
        approval_received: bool,
        approval_field_required: Option<&str>,
    ) -> Self {
        let mut eligibility = PromotionEligibility::Eligible;

        // Sandbox must be in a completed-style state.
        match projection.outcome {
            DccSandboxOutcome::Promoted => {
                if let Some(p) = &projection.promotion {
                    eligibility.add(PromotionIneligibilityReason::AlreadyPromoted {
                        decision_id: p.decision_id.clone(),
                    });
                }
            }
            DccSandboxOutcome::AwaitingPromotion => {}
            other => {
                eligibility.add(PromotionIneligibilityReason::SandboxNotCompleted {
                    current_outcome: format!("{other:?}"),
                });
            }
        }

        // Validation must be present and non-blocking.
        match &projection.validation {
            None => {
                eligibility.add(PromotionIneligibilityReason::ValidationMissing);
            }
            Some(v) if v.failed_check_count > 0 || v.verdict.eq_ignore_ascii_case("FAIL") => {
                eligibility.add(PromotionIneligibilityReason::ValidationBlocked {
                    verdict: v.verdict.clone(),
                });
            }
            Some(_) => {}
        }

        // Operator approval gate.
        if !approval_received {
            if let Some(field) = approval_field_required {
                eligibility.add(PromotionIneligibilityReason::OperatorApprovalRequired {
                    missing_field: field.to_string(),
                });
            }
        }

        Self {
            projection_family_id: Self::FAMILY_ID.to_string(),
            sandbox_run_id: projection.run_id.clone(),
            eligibility,
            approval_received,
            operator_approval_field_required: approval_field_required.map(str::to_string),
        }
    }

    pub fn can_promote(&self) -> bool {
        self.eligibility.can_promote()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
    use crate::kernel::sandbox::dcc_projection::{
        DccPromotionSummaryV1, DccValidationSummaryV1, DCC_SANDBOX_PROJECTION_FAMILY_ID,
    };
    use crate::kernel::sandbox::policy::CapabilityDecision;
    use crate::kernel::sandbox::run::SandboxRunStatus;

    fn base(outcome: DccSandboxOutcome, status: SandboxRunStatus) -> DccSandboxProjectionV1 {
        DccSandboxProjectionV1 {
            projection_family_id: DCC_SANDBOX_PROJECTION_FAMILY_ID.to_string(),
            run_id: "SBX-1".into(),
            kernel_task_run_id: "KTR-1".into(),
            session_run_id: "SES-1".into(),
            adapter_kind: "process_tier".into(),
            policy_version_id: "POL-1@1".into(),
            policy_default_decision: CapabilityDecision::Deny,
            capability_rows: vec![],
            workspace_id: "WSP-1".into(),
            workspace_root_relative: "x".into(),
            run_status: status,
            outcome,
            requested_at_utc: "2026-05-17T00:00:00Z".into(),
            started_at_utc: None,
            finished_at_utc: None,
            denial: None,
            validation: None,
            promotion: None,
            artifact_refs: vec![],
            artifact_classes_in_view: vec![Kb003ArtifactClass::SandboxLog],
            source_schema_ids: vec![],
        }
    }

    #[test]
    fn ineligible_when_sandbox_not_completed() {
        let p = base(DccSandboxOutcome::Running, SandboxRunStatus::Started);
        let s = DccKb003PromotionControlStateV1::derive(&p, true, None);
        assert!(!s.can_promote());
        let reasons = match s.eligibility {
            PromotionEligibility::Ineligible { reasons } => reasons,
            _ => panic!(),
        };
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PromotionIneligibilityReason::SandboxNotCompleted { .. })));
        // Must also flag missing validation.
        assert!(reasons
            .iter()
            .any(|r| matches!(r, PromotionIneligibilityReason::ValidationMissing)));
    }

    #[test]
    fn ineligible_when_validation_blocks() {
        let mut p = base(
            DccSandboxOutcome::AwaitingPromotion,
            SandboxRunStatus::Completed,
        );
        p.validation = Some(DccValidationSummaryV1 {
            validation_run_id: "VR-1".into(),
            verdict: "FAIL".into(),
            check_count: 3,
            failed_check_count: 1,
            report_artifact_ref: None,
        });
        let s = DccKb003PromotionControlStateV1::derive(&p, true, None);
        assert!(!s.can_promote());
    }

    #[test]
    fn ineligible_when_already_promoted() {
        let mut p = base(DccSandboxOutcome::Promoted, SandboxRunStatus::Completed);
        p.promotion = Some(DccPromotionSummaryV1 {
            decision_id: "PD-1".into(),
            decision: "ACCEPTED".into(),
            receipt_id: Some("PR-1".into()),
            receipt_artifact_ref: None,
            rationale_short: "ok".into(),
        });
        p.validation = Some(DccValidationSummaryV1 {
            validation_run_id: "VR-1".into(),
            verdict: "PASS".into(),
            check_count: 1,
            failed_check_count: 0,
            report_artifact_ref: None,
        });
        let s = DccKb003PromotionControlStateV1::derive(&p, true, None);
        assert!(!s.can_promote());
    }

    #[test]
    fn ineligible_when_approval_required_and_missing() {
        let mut p = base(
            DccSandboxOutcome::AwaitingPromotion,
            SandboxRunStatus::Completed,
        );
        p.validation = Some(DccValidationSummaryV1 {
            validation_run_id: "VR-1".into(),
            verdict: "PASS".into(),
            check_count: 1,
            failed_check_count: 0,
            report_artifact_ref: None,
        });
        let s = DccKb003PromotionControlStateV1::derive(&p, false, Some("operator_id"));
        assert!(!s.can_promote());
    }

    #[test]
    fn eligible_when_all_gates_pass() {
        let mut p = base(
            DccSandboxOutcome::AwaitingPromotion,
            SandboxRunStatus::Completed,
        );
        p.validation = Some(DccValidationSummaryV1 {
            validation_run_id: "VR-1".into(),
            verdict: "PASS".into(),
            check_count: 1,
            failed_check_count: 0,
            report_artifact_ref: None,
        });
        let s = DccKb003PromotionControlStateV1::derive(&p, true, Some("operator_id"));
        assert!(s.can_promote());
    }
}
