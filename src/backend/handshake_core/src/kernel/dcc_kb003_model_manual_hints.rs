//! Model-manual-style DCC hints.
//!
//! Acceptance theme repeated across MT-060..MT-074 (worker prompt + packet
//! acceptance text): "no-context model can read the projection without
//! terminal logs / scrollback / raw log access."
//!
//! This module emits short, breadcrumb-style hints a no-context model can
//! read from the DCC projection alone to know what the next useful action
//! is. Hints are derived from a [`DccSandboxProjectionV1`] and the
//! [`DccKb003PromotionControlStateV1`]; they are intentionally short,
//! actionable, and stable.
//!
//! Frontend renders via existing dcc-* IPC surface.

use serde::{Deserialize, Serialize};

use crate::kernel::dcc_kb003_promotion_control_state::{
    DccKb003PromotionControlStateV1, PromotionEligibility, PromotionIneligibilityReason,
};
use crate::kernel::sandbox::dcc_projection::{DccSandboxOutcome, DccSandboxProjectionV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HintCategory {
    Inspect,
    Retry,
    Approve,
    Escalate,
    Promote,
    NoOp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003ManualHintV1 {
    pub category: HintCategory,
    pub message: String,
}

impl DccKb003ManualHintV1 {
    pub fn new(category: HintCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003ManualHintsV1 {
    pub projection_family_id: String,
    pub sandbox_run_id: String,
    pub hints: Vec<DccKb003ManualHintV1>,
}

impl DccKb003ManualHintsV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.manual_hints@1";

    pub fn derive(
        projection: &DccSandboxProjectionV1,
        control_state: &DccKb003PromotionControlStateV1,
    ) -> Self {
        let mut hints = Vec::new();
        match projection.outcome {
            DccSandboxOutcome::Pending => {
                hints.push(DccKb003ManualHintV1::new(
                    HintCategory::NoOp,
                    "sandbox queued; wait for adapter to start the run",
                ));
            }
            DccSandboxOutcome::Running => {
                hints.push(DccKb003ManualHintV1::new(
                    HintCategory::Inspect,
                    "sandbox running; open the run detail view to follow progress",
                ));
            }
            DccSandboxOutcome::DeniedByPolicy => {
                if let Some(d) = &projection.denial {
                    hints.push(DccKb003ManualHintV1::new(
                        HintCategory::Escalate,
                        format!(
                            "policy {} denied capability {}; grant or narrow the policy",
                            d.policy_version_id,
                            d.capability.clone().unwrap_or_else(|| "<unspecified>".into())
                        ),
                    ));
                }
            }
            DccSandboxOutcome::FailedValidation => {
                hints.push(DccKb003ManualHintV1::new(
                    HintCategory::Inspect,
                    "validation failed; open validation report to read the failing descriptor",
                ));
            }
            DccSandboxOutcome::AwaitingPromotion => {
                if control_state.can_promote() {
                    hints.push(DccKb003ManualHintV1::new(
                        HintCategory::Promote,
                        "validation passed; promotion is eligible",
                    ));
                } else {
                    Self::derive_ineligibility_hints(&control_state.eligibility, &mut hints);
                }
            }
            DccSandboxOutcome::Promoted => {
                hints.push(DccKb003ManualHintV1::new(
                    HintCategory::NoOp,
                    "run promoted; no further action required",
                ));
            }
            DccSandboxOutcome::Rejected => {
                hints.push(DccKb003ManualHintV1::new(
                    HintCategory::Retry,
                    "run rejected before policy check; retry from the run list",
                ));
            }
        }
        Self {
            projection_family_id: Self::FAMILY_ID.to_string(),
            sandbox_run_id: projection.run_id.clone(),
            hints,
        }
    }

    fn derive_ineligibility_hints(
        eligibility: &PromotionEligibility,
        hints: &mut Vec<DccKb003ManualHintV1>,
    ) {
        if let PromotionEligibility::Ineligible { reasons } = eligibility {
            for r in reasons {
                match r {
                    PromotionIneligibilityReason::OperatorApprovalRequired { missing_field } => {
                        hints.push(DccKb003ManualHintV1::new(
                            HintCategory::Approve,
                            format!("operator approval missing '{missing_field}'; record approval"),
                        ));
                    }
                    PromotionIneligibilityReason::ValidationMissing => {
                        hints.push(DccKb003ManualHintV1::new(
                            HintCategory::Inspect,
                            "validation has not run yet; trigger validator",
                        ));
                    }
                    PromotionIneligibilityReason::ValidationBlocked { verdict } => {
                        hints.push(DccKb003ManualHintV1::new(
                            HintCategory::Escalate,
                            format!("validation verdict={verdict} blocks promotion; remediate then re-run"),
                        ));
                    }
                    PromotionIneligibilityReason::AlreadyPromoted { decision_id } => {
                        hints.push(DccKb003ManualHintV1::new(
                            HintCategory::NoOp,
                            format!("already promoted (decision {decision_id}); no further action"),
                        ));
                    }
                    PromotionIneligibilityReason::SandboxNotCompleted { .. } => {
                        hints.push(DccKb003ManualHintV1::new(
                            HintCategory::Inspect,
                            "sandbox not in completed state; wait or inspect run detail",
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
    use crate::kernel::sandbox::dcc_projection::{
        DccDenialSummaryV1, DccValidationSummaryV1, DCC_SANDBOX_PROJECTION_FAMILY_ID,
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
    fn hints_for_awaiting_promotion_eligible_say_promote() {
        let mut p = base(DccSandboxOutcome::AwaitingPromotion, SandboxRunStatus::Completed);
        p.validation = Some(DccValidationSummaryV1 {
            validation_run_id: "VR-1".into(),
            verdict: "PASS".into(),
            check_count: 1,
            failed_check_count: 0,
            report_artifact_ref: None,
        });
        let cs = DccKb003PromotionControlStateV1::derive(&p, true, None);
        let h = DccKb003ManualHintsV1::derive(&p, &cs);
        assert!(h.hints.iter().any(|x| x.category == HintCategory::Promote));
    }

    #[test]
    fn hints_for_denied_by_policy_recommend_escalation() {
        let mut p = base(DccSandboxOutcome::DeniedByPolicy, SandboxRunStatus::Rejected);
        p.denial = Some(DccDenialSummaryV1 {
            denial_id: "DEN-1".into(),
            kind: "POLICY_DENIED".into(),
            capability: Some("NETWORK".into()),
            action_description: "fetch".into(),
            reason: "default deny".into(),
            policy_version_id: "POL-1@1".into(),
        });
        let cs = DccKb003PromotionControlStateV1::derive(&p, false, None);
        let h = DccKb003ManualHintsV1::derive(&p, &cs);
        assert!(h.hints.iter().any(|x| x.category == HintCategory::Escalate));
    }

    #[test]
    fn hints_for_missing_validation_recommend_inspect() {
        let p = base(DccSandboxOutcome::AwaitingPromotion, SandboxRunStatus::Completed);
        let cs = DccKb003PromotionControlStateV1::derive(&p, true, None);
        let h = DccKb003ManualHintsV1::derive(&p, &cs);
        assert!(h.hints.iter().any(|x| x.category == HintCategory::Inspect));
    }
}
