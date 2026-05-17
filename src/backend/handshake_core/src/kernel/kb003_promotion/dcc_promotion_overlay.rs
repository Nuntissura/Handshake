//! MT-049: DCC promotion overlay — populates the `promotion`,
//! `artifact_refs`, and `outcome` fields of `DccSandboxProjectionV1` from a
//! `PromotionGateOutput`.
//!
//! Acceptance composite:
//! - MT-049 "Validation Replay: re-run descriptor set against same candidate.
//!   Replay records new run ID linked to original." — the overlay reads
//!   *only* from durable receipt + decision rows, so a replay that issues a
//!   new validation_run_id but reuses the same sandbox_run_id projects
//!   correctly when the same overlay is reapplied.
//! - Operator surface acceptance: the projection's `outcome` becomes
//!   `Promoted` exactly when the gate accepted; otherwise the projection
//!   shows the rejection rationale via `DccPromotionSummaryV1.rationale_short`.

use crate::kernel::kb003_promotion::artifact_bundle::Kb003ArtifactBundleV1;
use crate::kernel::kb003_promotion::decision::{PromotionDecisionV1, PromotionOutcome};
use crate::kernel::kb003_promotion::receipt::PromotionReceiptV1;
use crate::kernel::sandbox::dcc_projection::{
    DccPromotionSummaryV1, DccSandboxOutcome, DccSandboxProjectionV1,
};

pub struct DccPromotionOverlay;

impl DccPromotionOverlay {
    /// Mutate `projection` in place so it reflects `decision` + `receipt`.
    /// Both inputs are durable rows the caller already persisted.
    pub fn apply(
        projection: &mut DccSandboxProjectionV1,
        decision: &PromotionDecisionV1,
        receipt: &PromotionReceiptV1,
        bundle: Option<&Kb003ArtifactBundleV1>,
    ) {
        let summary = DccPromotionSummaryV1 {
            decision_id: decision.decision_id.clone(),
            decision: decision.outcome.tag().to_string(),
            receipt_id: Some(receipt.receipt_id.clone()),
            receipt_artifact_ref: Some(format!(
                "kb003://promotion_receipt/{}",
                receipt.receipt_id
            )),
            rationale_short: match &decision.outcome {
                PromotionOutcome::Accepted => "accepted".to_string(),
                PromotionOutcome::Rejected { reason } => reason.rationale_short(),
            },
        };
        projection.promotion = Some(summary);

        // Re-derive outcome with the new promotion evidence.
        let has_denial = projection.denial.is_some();
        let has_accepted = matches!(decision.outcome, PromotionOutcome::Accepted);
        projection.outcome = DccSandboxOutcome::derive(projection.run_status, has_denial, has_accepted);

        // Surface receipt + bundle handles in the projection's artifact list so
        // operators can open them from DCC without a second join.
        if let Some(b) = bundle {
            for h in &b.handles {
                if !projection.artifact_refs.contains(&h.handle) {
                    projection.artifact_refs.push(h.handle.clone());
                }
                if !projection.artifact_classes_in_view.contains(&h.class) {
                    projection.artifact_classes_in_view.push(h.class);
                }
            }
        }
        let receipt_handle = format!("kb003://promotion_receipt/{}", receipt.receipt_id);
        if !projection.artifact_refs.contains(&receipt_handle) {
            projection.artifact_refs.push(receipt_handle);
        }
        if !projection
            .artifact_classes_in_view
            .contains(&crate::kernel::kb003_artifact_classes::Kb003ArtifactClass::PromotionReceipt)
        {
            projection.artifact_classes_in_view.push(
                crate::kernel::kb003_artifact_classes::Kb003ArtifactClass::PromotionReceipt,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
    use crate::kernel::kb003_promotion::artifact_bundle::{
        Kb003ArtifactHandleV1, KbArtifactBundleAssembler,
    };
    use crate::kernel::kb003_promotion::decision::PromotionRejectionReason;
    use crate::kernel::sandbox::dcc_projection::{
        DccSandboxOutcome, DccSandboxProjectionV1, DCC_SANDBOX_PROJECTION_FAMILY_ID,
    };
    use crate::kernel::sandbox::policy::CapabilityDecision;
    use crate::kernel::sandbox::run::{SandboxRunStatus, SandboxRunV1};

    fn projection_in_progress() -> DccSandboxProjectionV1 {
        DccSandboxProjectionV1 {
            projection_family_id: DCC_SANDBOX_PROJECTION_FAMILY_ID.to_string(),
            run_id: "SBX-test".into(),
            kernel_task_run_id: "KTR-1".into(),
            session_run_id: "SES-1".into(),
            adapter_kind: "process_tier".into(),
            policy_version_id: "POL-1@1".into(),
            policy_default_decision: CapabilityDecision::Deny,
            capability_rows: Vec::new(),
            workspace_id: "WSP-1".into(),
            workspace_root_relative: "handshake-product/kb003/work/x".into(),
            run_status: SandboxRunStatus::Completed,
            outcome: DccSandboxOutcome::AwaitingPromotion,
            requested_at_utc: "2026-05-17T00:00:00Z".into(),
            started_at_utc: None,
            finished_at_utc: None,
            denial: None,
            validation: None,
            promotion: None,
            artifact_refs: Vec::new(),
            artifact_classes_in_view: Vec::new(),
            source_schema_ids: Vec::new(),
        }
    }

    fn sample_bundle() -> Kb003ArtifactBundleV1 {
        let run = SandboxRunV1::new_requested("KTR-1", "SES-1", "process_tier", "POL-1@1", "WSP-1");
        let handles = vec![
            Kb003ArtifactHandleV1::new(Kb003ArtifactClass::SandboxLog, "h1aaaaaaaaaaaaaa").unwrap(),
        ];
        KbArtifactBundleAssembler::assemble(&run, handles).unwrap()
    }

    #[test]
    fn accepted_decision_flips_outcome_to_promoted() {
        let mut p = projection_in_progress();
        let d = PromotionDecisionV1::accepted("SBX-1", "VR-1");
        let r = PromotionReceiptV1::new(d.clone(), "IK-1", None, None, vec![]);
        let b = sample_bundle();
        DccPromotionOverlay::apply(&mut p, &d, &r, Some(&b));
        assert_eq!(p.outcome, DccSandboxOutcome::Promoted);
        let prom = p.promotion.as_ref().unwrap();
        assert_eq!(prom.decision, "ACCEPTED");
        assert_eq!(prom.receipt_id.as_deref(), Some(r.receipt_id.as_str()));
        assert!(p.artifact_refs.iter().any(|s| s.starts_with("kb003://promotion_receipt/")));
        assert!(p
            .artifact_classes_in_view
            .contains(&Kb003ArtifactClass::PromotionReceipt));
        assert!(p
            .artifact_classes_in_view
            .contains(&Kb003ArtifactClass::SandboxLog));
    }

    #[test]
    fn rejected_decision_records_rationale_without_promotion_outcome() {
        let mut p = projection_in_progress();
        let d = PromotionDecisionV1::rejected(
            "SBX-1",
            "VR-1",
            PromotionRejectionReason::MissingApproval {
                missing_field: "operator_id".into(),
            },
        );
        let r = PromotionReceiptV1::new(d.clone(), "IK-1", None, None, vec![]);
        DccPromotionOverlay::apply(&mut p, &d, &r, None);
        assert_eq!(p.outcome, DccSandboxOutcome::AwaitingPromotion);
        let prom = p.promotion.as_ref().unwrap();
        assert_eq!(prom.decision, "REJECTED_MISSING_APPROVAL");
        assert!(prom.rationale_short.contains("operator_id"));
    }
}
