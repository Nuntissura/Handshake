//! MT-067 DCC Sandbox Run List.
//!
//! Acceptance (MT-067.json): "operator can find current and past sandbox
//! runs."
//!
//! A typed projection of every sandbox run the kernel has ever recorded —
//! suitable for rendering the operator's sandbox lane index page.
//! Frontend renders via existing dcc-* IPC surface; no app/** edits.
//!
//! The list row is derived from [`DccSandboxProjectionV1`] (MT-010) so it
//! never drifts from the authoritative projection.

use serde::{Deserialize, Serialize};

use crate::kernel::sandbox::dcc_projection::{DccSandboxOutcome, DccSandboxProjectionV1};
use crate::kernel::sandbox::run::SandboxRunStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003SandboxRunListRowV1 {
    pub run_id: String,
    pub adapter_kind: String,
    pub run_status: SandboxRunStatus,
    pub outcome: DccSandboxOutcome,
    pub requested_at_utc: String,
    pub finished_at_utc: Option<String>,
    pub policy_version_id: String,
    pub has_denial: bool,
    pub has_validation: bool,
    pub has_promotion: bool,
}

impl DccKb003SandboxRunListRowV1 {
    pub fn from_projection(p: &DccSandboxProjectionV1) -> Self {
        Self {
            run_id: p.run_id.clone(),
            adapter_kind: p.adapter_kind.clone(),
            run_status: p.run_status,
            outcome: p.outcome,
            requested_at_utc: p.requested_at_utc.clone(),
            finished_at_utc: p.finished_at_utc.clone(),
            policy_version_id: p.policy_version_id.clone(),
            has_denial: p.denial.is_some(),
            has_validation: p.validation.is_some(),
            has_promotion: p.promotion.is_some(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccKb003SandboxRunListV1 {
    pub projection_family_id: String,
    pub rows: Vec<DccKb003SandboxRunListRowV1>,
}

impl DccKb003SandboxRunListV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.sandbox_run_list@1";

    pub fn from_projections(projections: &[DccSandboxProjectionV1]) -> Self {
        let rows: Vec<_> = projections
            .iter()
            .map(DccKb003SandboxRunListRowV1::from_projection)
            .collect();
        Self {
            projection_family_id: Self::FAMILY_ID.to_string(),
            rows,
        }
    }

    /// Total number of rows; `len() == 0` means the operator sees an empty
    /// list, which is a valid state (no sandbox runs yet).
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Operator-search helper: find a run by id.
    pub fn find(&self, run_id: &str) -> Option<&DccKb003SandboxRunListRowV1> {
        self.rows.iter().find(|r| r.run_id == run_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
    use crate::kernel::sandbox::dcc_projection::{
        DCC_SANDBOX_PROJECTION_FAMILY_ID, DccPromotionSummaryV1,
    };
    use crate::kernel::sandbox::policy::CapabilityDecision;

    fn sample(run_id: &str, status: SandboxRunStatus, has_prom: bool) -> DccSandboxProjectionV1 {
        DccSandboxProjectionV1 {
            projection_family_id: DCC_SANDBOX_PROJECTION_FAMILY_ID.to_string(),
            run_id: run_id.into(),
            kernel_task_run_id: "KTR-1".into(),
            session_run_id: "SES-1".into(),
            adapter_kind: "process_tier".into(),
            policy_version_id: "POL-1@1".into(),
            policy_default_decision: CapabilityDecision::Deny,
            capability_rows: Vec::new(),
            workspace_id: "WSP-1".into(),
            workspace_root_relative: "x/y/z".into(),
            run_status: status,
            outcome: DccSandboxOutcome::derive(status, false, has_prom),
            requested_at_utc: "2026-05-17T00:00:00Z".into(),
            started_at_utc: None,
            finished_at_utc: None,
            denial: None,
            validation: None,
            promotion: if has_prom {
                Some(DccPromotionSummaryV1 {
                    decision_id: "PD-1".into(),
                    decision: "ACCEPTED".into(),
                    receipt_id: Some("PR-1".into()),
                    receipt_artifact_ref: Some("kb003://promotion_receipt/PR-1".into()),
                    rationale_short: "ok".into(),
                })
            } else {
                None
            },
            artifact_refs: Vec::new(),
            artifact_classes_in_view: vec![Kb003ArtifactClass::SandboxLog],
            source_schema_ids: Vec::new(),
        }
    }

    #[test]
    fn list_projects_all_runs_and_finds_by_id() {
        let projections = vec![
            sample("SBX-1", SandboxRunStatus::Completed, true),
            sample("SBX-2", SandboxRunStatus::Started, false),
        ];
        let list = DccKb003SandboxRunListV1::from_projections(&projections);
        assert_eq!(list.len(), 2);
        let row = list.find("SBX-1").expect("found");
        assert!(row.has_promotion);
        assert_eq!(row.outcome, DccSandboxOutcome::Promoted);
        let row2 = list.find("SBX-2").expect("found");
        assert!(!row2.has_promotion);
        assert!(list.find("SBX-MISSING").is_none());
    }

    #[test]
    fn empty_list_is_valid_state() {
        let list = DccKb003SandboxRunListV1::from_projections(&[]);
        assert!(list.is_empty());
        assert_eq!(
            list.projection_family_id,
            DccKb003SandboxRunListV1::FAMILY_ID
        );
    }
}
