//! KB003 top-level DCC rollup.
//!
//! Combines every KB003 DCC surface into a single operator view that a
//! no-context model can read to understand the full sandbox + validation +
//! promotion state for one sandbox run. Used by the restart-replay tests
//! (Batch H) and by MT-080 as the canonical top-level type whose round-trip
//! the test matrix asserts.
//!
//! Composition:
//! - `projection`         (MT-010)  authoritative sandbox projection
//! - `blocked_overlay`    (MT-060)  active blocked reasons with disposition
//! - `lane_wake_timeline` (MT-065)  wake/settlement receipts as rows
//! - `promotion_control`  (MT-069)  typed promotion eligibility
//! - `manual_hints`       (theme)   model-manual breadcrumbs
//! - `aggregate_summary`  (MT-064)  optional WP-scope aggregate (may be None
//!   for a single-run rollup)
//!
//! Frontend renders via existing dcc-* IPC surface; no app/** edits.

use serde::{Deserialize, Serialize};

use crate::kernel::dcc_kb003_aggregate_summary::Kb003AggregateRunSummaryV1;
use crate::kernel::dcc_kb003_blocked_reasons::DccKb003BlockedReasonOverlayV1;
use crate::kernel::dcc_kb003_lane_wake::DccKb003LaneWakeRowV1;
use crate::kernel::dcc_kb003_model_manual_hints::DccKb003ManualHintsV1;
use crate::kernel::dcc_kb003_promotion_control_state::DccKb003PromotionControlStateV1;
use crate::kernel::sandbox::dcc_projection::DccSandboxProjectionV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003RollupV1 {
    pub projection_family_id: String,
    pub sandbox_run_id: String,
    pub projection: DccSandboxProjectionV1,
    pub blocked_overlay: DccKb003BlockedReasonOverlayV1,
    pub lane_wake_timeline: Vec<DccKb003LaneWakeRowV1>,
    pub promotion_control: DccKb003PromotionControlStateV1,
    pub manual_hints: DccKb003ManualHintsV1,
    pub aggregate_summary: Option<Kb003AggregateRunSummaryV1>,
}

impl DccKb003RollupV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.rollup@1";

    pub fn new(
        projection: DccSandboxProjectionV1,
        blocked_overlay: DccKb003BlockedReasonOverlayV1,
        lane_wake_timeline: Vec<DccKb003LaneWakeRowV1>,
        promotion_control: DccKb003PromotionControlStateV1,
        manual_hints: DccKb003ManualHintsV1,
        aggregate_summary: Option<Kb003AggregateRunSummaryV1>,
    ) -> Self {
        Self {
            projection_family_id: Self::FAMILY_ID.to_string(),
            sandbox_run_id: projection.run_id.clone(),
            projection,
            blocked_overlay,
            lane_wake_timeline,
            promotion_control,
            manual_hints,
            aggregate_summary,
        }
    }

    /// Acceptance helper: rollup is self-describing iff every component is
    /// internally consistent. Used by the restart-replay test matrix as the
    /// single check after deserializing the round-trip.
    pub fn is_self_describing(&self) -> bool {
        self.projection.is_self_describing()
    }

    /// Round-trip portable JSON form. The restart replay tests in Batch H
    /// will assert `from_portable_json(self.portable_json()) == self`.
    pub fn portable_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::dcc_kb003_blocked_reasons::DccKb003BlockedReasonOverlayV1;
    use crate::kernel::dcc_kb003_model_manual_hints::DccKb003ManualHintsV1;
    use crate::kernel::dcc_kb003_promotion_control_state::DccKb003PromotionControlStateV1;
    use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
    use crate::kernel::sandbox::dcc_projection::{
        DccDenialSummaryV1, DccSandboxOutcome, DccSandboxProjectionV1,
        DCC_SANDBOX_PROJECTION_FAMILY_ID,
    };
    use crate::kernel::sandbox::policy::CapabilityDecision;
    use crate::kernel::sandbox::run::SandboxRunStatus;

    fn denied_projection() -> DccSandboxProjectionV1 {
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
                reason: "default deny".into(),
                policy_version_id: "POL-1@1".into(),
            }),
            validation: None,
            promotion: None,
            artifact_refs: vec![],
            artifact_classes_in_view: vec![Kb003ArtifactClass::SandboxLog],
            source_schema_ids: vec![],
        }
    }

    #[test]
    fn rollup_is_self_describing_for_denied_run() {
        let p = denied_projection();
        let cs = DccKb003PromotionControlStateV1::derive(&p, false, None);
        let hints = DccKb003ManualHintsV1::derive(&p, &cs);
        let rollup = DccKb003RollupV1::new(
            p,
            DccKb003BlockedReasonOverlayV1::new(vec![]),
            vec![],
            cs,
            hints,
            None,
        );
        assert!(rollup.is_self_describing());
        assert_eq!(rollup.projection_family_id, DccKb003RollupV1::FAMILY_ID);
    }

    #[test]
    fn rollup_round_trips_via_serde() {
        let p = denied_projection();
        let cs = DccKb003PromotionControlStateV1::derive(&p, false, None);
        let hints = DccKb003ManualHintsV1::derive(&p, &cs);
        let rollup = DccKb003RollupV1::new(
            p,
            DccKb003BlockedReasonOverlayV1::new(vec![]),
            vec![],
            cs,
            hints,
            None,
        );
        let json = rollup.portable_json().unwrap();
        let recovered: DccKb003RollupV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, rollup);
    }
}
