//! MT-068 DCC Run Detail.
//!
//! Acceptance (MT-068.json): "detail view has no hidden dependency on
//! terminal scrollback."
//!
//! Combines the authoritative sandbox projection (MT-010), the blocked
//! reason overlay (MT-060), the lane wake timeline (MT-065), and the
//! per-MT summary handles (MT-063) into a single detail view that a
//! no-context model or operator can read without opening any terminal,
//! chat, or log scrollback.
//!
//! Frontend renders via existing dcc-* IPC surface; no app/** edits.

use serde::{Deserialize, Serialize};

use crate::kernel::dcc_kb003_blocked_reasons::DccKb003BlockedReasonOverlayV1;
use crate::kernel::dcc_kb003_lane_wake::DccKb003LaneWakeRowV1;
use crate::kernel::sandbox::dcc_projection::DccSandboxProjectionV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003RunDetailV1 {
    pub projection_family_id: String,
    pub projection: DccSandboxProjectionV1,
    pub blocked_overlay: DccKb003BlockedReasonOverlayV1,
    pub lane_wake_timeline: Vec<DccKb003LaneWakeRowV1>,
    pub mt_summary_handles: Vec<String>,
}

impl DccKb003RunDetailV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.run_detail@1";

    pub fn new(
        projection: DccSandboxProjectionV1,
        blocked_overlay: DccKb003BlockedReasonOverlayV1,
        lane_wake_timeline: Vec<DccKb003LaneWakeRowV1>,
        mt_summary_handles: Vec<String>,
    ) -> Self {
        Self {
            projection_family_id: Self::FAMILY_ID.to_string(),
            projection,
            blocked_overlay,
            lane_wake_timeline,
            mt_summary_handles,
        }
    }

    /// Acceptance helper: the detail view stands on its own when:
    /// 1. The embedded projection is self-describing (MT-010 acceptance).
    /// 2. Every blocked overlay row that exists has a short rationale that
    ///    a reader can act on.
    /// 3. The timeline has well-formed rows.
    pub fn is_self_describing(&self) -> bool {
        if !self.projection.is_self_describing() {
            return false;
        }
        for row in &self.blocked_overlay.rows {
            if row.rationale_short.is_empty() {
                return false;
            }
        }
        for row in &self.lane_wake_timeline {
            if row.rationale_short.is_empty() {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::dcc_kb003_blocked_reasons::{
        BlockedReason, DccKb003BlockedReasonOverlayV1, DccKb003BlockedReasonRowV1,
    };
    use crate::kernel::dcc_kb003_lane_wake::{DccKb003LaneWakeRowV1, LaneWakeReceiptV1};
    use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
    use crate::kernel::sandbox::dcc_projection::{
        DccDenialSummaryV1, DccSandboxOutcome, DccSandboxProjectionV1,
        DCC_SANDBOX_PROJECTION_FAMILY_ID,
    };
    use crate::kernel::sandbox::policy::CapabilityDecision;
    use crate::kernel::sandbox::run::SandboxRunStatus;

    fn sample_projection_denied() -> DccSandboxProjectionV1 {
        DccSandboxProjectionV1 {
            projection_family_id: DCC_SANDBOX_PROJECTION_FAMILY_ID.to_string(),
            run_id: "SBX-1".into(),
            kernel_task_run_id: "KTR-1".into(),
            session_run_id: "SES-1".into(),
            adapter_kind: "process_tier".into(),
            policy_version_id: "POL-1@1".into(),
            policy_default_decision: CapabilityDecision::Deny,
            capability_rows: Vec::new(),
            workspace_id: "WSP-1".into(),
            workspace_root_relative: "x/y/z".into(),
            run_status: SandboxRunStatus::Rejected,
            outcome: DccSandboxOutcome::DeniedByPolicy,
            requested_at_utc: "2026-05-17T00:00:00Z".into(),
            started_at_utc: None,
            finished_at_utc: None,
            denial: Some(DccDenialSummaryV1 {
                denial_id: "DEN-1".into(),
                kind: "POLICY_DENIED".into(),
                capability: Some("NETWORK".into()),
                action_description: "fetch".into(),
                reason: "deny default".into(),
                policy_version_id: "POL-1@1".into(),
            }),
            validation: None,
            promotion: None,
            artifact_refs: Vec::new(),
            artifact_classes_in_view: vec![Kb003ArtifactClass::SandboxLog],
            source_schema_ids: Vec::new(),
        }
    }

    #[test]
    fn detail_is_self_describing_with_full_evidence() {
        let projection = sample_projection_denied();
        let reason = BlockedReason::PolicyDenied {
            capability: "NETWORK".into(),
            policy_version_id: "POL-1@1".into(),
            denial_id: "DEN-1".into(),
        };
        let overlay =
            DccKb003BlockedReasonOverlayV1::new(vec![DccKb003BlockedReasonRowV1::from_reason(
                reason.clone(),
            )]);
        let wake = LaneWakeReceiptV1::wake(
            crate::kernel::dcc_kb003_blocked_reasons::BlockedLane::Sandbox,
            vec!["kb003://denial/DEN-1".into()],
            reason,
            "operator granted network",
        );
        let timeline = vec![DccKb003LaneWakeRowV1::from_receipt(&wake)];
        let detail = DccKb003RunDetailV1::new(
            projection,
            overlay,
            timeline,
            vec!["kb003://mt_summary/MTSUM-1".into()],
        );
        assert!(detail.is_self_describing());
    }

    #[test]
    fn family_id_is_versioned() {
        assert!(DccKb003RunDetailV1::FAMILY_ID.starts_with("hsk.dcc.kb003."));
        assert!(DccKb003RunDetailV1::FAMILY_ID.contains('@'));
    }
}
