//! MT-056: MTE Lane Settlement (Approval Ref Binding support).
//!
//! Acceptance (MT-056.json): "Approval Ref Binding: bind approval evidence to
//! promotion decisions. Acceptance: promotion cannot accept without required
//! approval posture."
//!
//! The MTE lane settlement record is the WP-level "this lane is closed" event:
//! it binds the aggregate summary (MT-053) to the operator approval evidence
//! required to settle the lane as `Pass`. A lane cannot reach `Pass` without
//! an attached `LaneApprovalEvidenceV1`; attempting to build a `Pass`
//! settlement without it returns a typed `LaneSettlementBuildError`.
//!
//! Settlement verdicts are PASS / FAIL / BLOCKED / CANCELLED. The verdict
//! determines whether downstream integration validation may run.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kernel::mte_aggregate_summary::MteAggregateSummaryV1;

/// Approval evidence the lane settlement needs to attach for a PASS verdict.
/// Mirrors `kb003_promotion::gate::OperatorApprovalEvidence` shape so the same
/// evidence can be reused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneApprovalEvidenceV1 {
    pub operator_id: String,
    pub review_receipt_id: String,
    pub approval_source: String,
    pub reason: String,
}

impl LaneApprovalEvidenceV1 {
    pub fn missing_field(&self) -> Option<&'static str> {
        if self.operator_id.trim().is_empty() {
            return Some("operator_id");
        }
        if self.review_receipt_id.trim().is_empty() {
            return Some("review_receipt_id");
        }
        if self.approval_source.trim().is_empty() {
            return Some("approval_source");
        }
        if self.reason.trim().is_empty() {
            return Some("reason");
        }
        None
    }

    pub fn looks_fixture(&self) -> bool {
        // H-C3 fix: only scan identifying fields, not the free-form `reason`.
        // A legitimate operator describing what they validated (e.g.
        // `reason: "kernel-proof passing"`) was being rejected as fake.
        // Identifiers (operator_id, review_receipt_id, approval_source) are
        // the load-bearing fields for "is this real evidence"; reason is
        // narrative.
        let needle = |s: &str| {
            let lower = s.to_ascii_lowercase();
            lower.contains("fixture") || lower.contains("kernel-proof")
        };
        needle(&self.operator_id)
            || needle(&self.review_receipt_id)
            || needle(&self.approval_source)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteLaneVerdict {
    Pass,
    Fail,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneSettlementBuildError {
    /// Pass requires attached approval evidence.
    PassWithoutApproval,
    /// Pass requires every MT to be Completed + Accepted.
    PassWithUnclosedMts { unclosed_count: u32 },
    /// Approval is malformed (missing field).
    ApprovalMissingField { field: &'static str },
    /// Approval looks fixture-like (mirrors gate-level guard).
    ApprovalLooksFixture,
    EmptyWpId,
}

impl std::fmt::Display for LaneSettlementBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PassWithoutApproval => write!(
                f,
                "MTE lane settlement refuses Pass without LaneApprovalEvidenceV1"
            ),
            Self::PassWithUnclosedMts { unclosed_count } => write!(
                f,
                "MTE lane settlement refuses Pass with {unclosed_count} unclosed MTs"
            ),
            Self::ApprovalMissingField { field } => {
                write!(f, "approval missing field {field}")
            }
            Self::ApprovalLooksFixture => write!(f, "approval looks fixture-like"),
            Self::EmptyWpId => write!(f, "wp_id must not be empty"),
        }
    }
}

impl std::error::Error for LaneSettlementBuildError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MteLaneSettlementV1 {
    pub schema_version: &'static str,
    pub wp_id: String,
    pub verdict: MteLaneVerdict,
    pub aggregate_summary: MteAggregateSummaryV1,
    pub approval: Option<LaneApprovalEvidenceV1>,
    pub receipt_refs: Vec<String>,
    pub rationale: String,
    pub settled_at_utc: DateTime<Utc>,
}

impl MteLaneSettlementV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.mte.lane_settlement@1";

    pub fn settle_pass(
        wp_id: impl Into<String>,
        aggregate: MteAggregateSummaryV1,
        approval: LaneApprovalEvidenceV1,
        receipt_refs: Vec<String>,
        rationale: impl Into<String>,
    ) -> Result<Self, LaneSettlementBuildError> {
        let wp_id = wp_id.into();
        if wp_id.trim().is_empty() {
            return Err(LaneSettlementBuildError::EmptyWpId);
        }
        if let Some(field) = approval.missing_field() {
            return Err(LaneSettlementBuildError::ApprovalMissingField { field });
        }
        if approval.looks_fixture() {
            return Err(LaneSettlementBuildError::ApprovalLooksFixture);
        }
        // Pass requires every MT to be Completed and Accepted (or the lane has
        // no MTs at all — an empty lane is trivially passed).
        let total = aggregate.total_mts;
        let accepted = aggregate.promotion_counts.accepted;
        let completed = aggregate.status_counts.completed;
        if total != accepted || total != completed {
            return Err(LaneSettlementBuildError::PassWithUnclosedMts {
                unclosed_count: total.saturating_sub(accepted.min(completed)),
            });
        }
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            wp_id,
            verdict: MteLaneVerdict::Pass,
            aggregate_summary: aggregate,
            approval: Some(approval),
            receipt_refs,
            rationale: rationale.into(),
            settled_at_utc: Utc::now(),
        })
    }

    /// Non-Pass settlement variants do not require approval evidence; they
    /// record why the lane closed without merging.
    pub fn settle_non_pass(
        wp_id: impl Into<String>,
        verdict: MteLaneVerdict,
        aggregate: MteAggregateSummaryV1,
        receipt_refs: Vec<String>,
        rationale: impl Into<String>,
    ) -> Result<Self, LaneSettlementBuildError> {
        let wp_id = wp_id.into();
        if wp_id.trim().is_empty() {
            return Err(LaneSettlementBuildError::EmptyWpId);
        }
        if matches!(verdict, MteLaneVerdict::Pass) {
            return Err(LaneSettlementBuildError::PassWithoutApproval);
        }
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            wp_id,
            verdict,
            aggregate_summary: aggregate,
            approval: None,
            receipt_refs,
            rationale: rationale.into(),
            settled_at_utc: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::mte_aggregate_summary::{
        MtePromotionCounts, MteStatusCounts,
    };

    fn aggregate_all_accepted(n: u32) -> MteAggregateSummaryV1 {
        MteAggregateSummaryV1 {
            schema_version: MteAggregateSummaryV1::SCHEMA_VERSION,
            wp_id: "WP-X".into(),
            total_mts: n,
            status_counts: MteStatusCounts {
                completed: n,
                ..Default::default()
            },
            promotion_counts: MtePromotionCounts {
                accepted: n,
                ..Default::default()
            },
            blocker_counts_by_tag: Default::default(),
            total_wall_time_ms: 0,
            oldest_pending_mt_id: None,
            oldest_pending_started_at_utc: None,
            accepted_promotion_receipt_ids: (0..n).map(|i| format!("PR-{i}")).collect(),
        }
    }

    fn good_approval() -> LaneApprovalEvidenceV1 {
        LaneApprovalEvidenceV1 {
            operator_id: "op-ilja".into(),
            review_receipt_id: "OPR-deadbeef".into(),
            approval_source: "operator_review_receipt".into(),
            reason: "all MTs validated, merging WP".into(),
        }
    }

    // MT-056 acceptance: promotion cannot accept without required approval
    // posture. (`settle_pass` is the lane-level equivalent.)
    #[test]
    fn pass_without_approval_is_typed_error() {
        let agg = aggregate_all_accepted(2);
        let mut bad = good_approval();
        bad.operator_id = "".into();
        let err = MteLaneSettlementV1::settle_pass(
            "WP-X",
            agg,
            bad,
            vec![],
            "ok",
        )
        .unwrap_err();
        match err {
            LaneSettlementBuildError::ApprovalMissingField { field } => {
                assert_eq!(field, "operator_id");
            }
            other => panic!("expected ApprovalMissingField, got {other:?}"),
        }
    }

    #[test]
    fn pass_with_fixture_like_approval_refused() {
        let agg = aggregate_all_accepted(1);
        let mut bad = good_approval();
        bad.operator_id = "kernel-proof-fixture".into();
        let err = MteLaneSettlementV1::settle_pass(
            "WP-X",
            agg,
            bad,
            vec![],
            "ok",
        )
        .unwrap_err();
        assert_eq!(err, LaneSettlementBuildError::ApprovalLooksFixture);
    }

    #[test]
    fn pass_with_unclosed_mts_refused() {
        let mut agg = aggregate_all_accepted(3);
        agg.promotion_counts.accepted = 2; // one MT not yet accepted
        let err = MteLaneSettlementV1::settle_pass(
            "WP-X",
            agg,
            good_approval(),
            vec![],
            "ok",
        )
        .unwrap_err();
        match err {
            LaneSettlementBuildError::PassWithUnclosedMts { unclosed_count } => {
                assert!(unclosed_count >= 1);
            }
            other => panic!("expected PassWithUnclosedMts, got {other:?}"),
        }
    }

    #[test]
    fn happy_pass_succeeds() {
        let agg = aggregate_all_accepted(2);
        let s = MteLaneSettlementV1::settle_pass(
            "WP-X",
            agg,
            good_approval(),
            vec!["PR-0".into(), "PR-1".into()],
            "merging WP",
        )
        .unwrap();
        assert_eq!(s.verdict, MteLaneVerdict::Pass);
        assert!(s.approval.is_some());
    }

    #[test]
    fn fail_settlement_does_not_need_approval() {
        let agg = aggregate_all_accepted(0);
        let s = MteLaneSettlementV1::settle_non_pass(
            "WP-X",
            MteLaneVerdict::Fail,
            agg,
            vec![],
            "validation blocked",
        )
        .unwrap();
        assert_eq!(s.verdict, MteLaneVerdict::Fail);
        assert!(s.approval.is_none());
    }

    #[test]
    fn non_pass_helper_refuses_pass_verdict() {
        let agg = aggregate_all_accepted(0);
        let err = MteLaneSettlementV1::settle_non_pass(
            "WP-X",
            MteLaneVerdict::Pass,
            agg,
            vec![],
            "x",
        )
        .unwrap_err();
        assert_eq!(err, LaneSettlementBuildError::PassWithoutApproval);
    }

    #[test]
    fn empty_wp_id_refused() {
        let agg = aggregate_all_accepted(0);
        let err = MteLaneSettlementV1::settle_pass(
            "",
            agg,
            good_approval(),
            vec![],
            "x",
        )
        .unwrap_err();
        assert_eq!(err, LaneSettlementBuildError::EmptyWpId);
    }
}
