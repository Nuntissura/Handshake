//! MT-058: MTE Promotion Closeout Bundle.
//!
//! Acceptance (MT-058.json): "Promotion Closeout Bundle: implement canonical
//! closeout bundle. Acceptance: Integration Validator can review one bundle
//! for promotion."
//!
//! The closeout bundle is the *single artifact* the Integration Validator
//! reviews when deciding whether to merge a WP. It folds together:
//!
//! - The MTE aggregate summary (MT-053) — high-level WP status.
//! - The lane settlement record (MT-056) — verdict + approval evidence.
//! - The list of per-MT summaries (MT-051) — per-MT detail.
//! - References to all promotion receipts and key artifact bundles.
//!
//! Validation: a closeout bundle for a passing lane refuses to build if the
//! aggregate disagrees with the lane settlement (e.g. settlement says PASS
//! but aggregate has rejected promotions). This is the "one bundle" guarantee
//! the validator depends on.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kernel::mte_aggregate_summary::MteAggregateSummaryV1;
use crate::kernel::mte_lane_settlement::{MteLaneSettlementV1, MteLaneVerdict};
use crate::kernel::mte_per_mt_summary::MtePerMtSummaryV1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseoutBundleBuildError {
    EmptyWpId,
    AggregateLaneSettlementWpMismatch,
    PassSettlementButAggregateHasRejections,
    PassSettlementWithoutApproval,
    PerMtCountDisagreesWithAggregate {
        per_mt_count: u32,
        aggregate_count: u32,
    },
}

impl std::fmt::Display for CloseoutBundleBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyWpId => write!(f, "wp_id must not be empty"),
            Self::AggregateLaneSettlementWpMismatch => {
                write!(f, "aggregate and lane settlement disagree on wp_id")
            }
            Self::PassSettlementButAggregateHasRejections => write!(
                f,
                "PASS lane settlement requires no rejected promotions in aggregate"
            ),
            Self::PassSettlementWithoutApproval => {
                write!(f, "PASS lane settlement must carry approval evidence")
            }
            Self::PerMtCountDisagreesWithAggregate {
                per_mt_count,
                aggregate_count,
            } => write!(
                f,
                "per-MT summary count {per_mt_count} disagrees with aggregate.total_mts {aggregate_count}"
            ),
        }
    }
}

impl std::error::Error for CloseoutBundleBuildError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MteCloseoutBundleV1 {
    pub schema_version: &'static str,
    pub wp_id: String,
    pub lane_settlement: MteLaneSettlementV1,
    pub aggregate: MteAggregateSummaryV1,
    pub per_mt_summaries: Vec<MtePerMtSummaryV1>,
    pub promotion_receipt_refs: Vec<String>,
    pub artifact_bundle_refs: Vec<String>,
    pub built_at_utc: DateTime<Utc>,
}

impl MteCloseoutBundleV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.mte.closeout_bundle@1";

    #[allow(clippy::too_many_arguments)]
    pub fn build(
        wp_id: impl Into<String>,
        lane_settlement: MteLaneSettlementV1,
        aggregate: MteAggregateSummaryV1,
        per_mt_summaries: Vec<MtePerMtSummaryV1>,
        promotion_receipt_refs: Vec<String>,
        artifact_bundle_refs: Vec<String>,
    ) -> Result<Self, CloseoutBundleBuildError> {
        let wp_id = wp_id.into();
        if wp_id.trim().is_empty() {
            return Err(CloseoutBundleBuildError::EmptyWpId);
        }
        if lane_settlement.wp_id != wp_id || aggregate.wp_id != wp_id {
            return Err(CloseoutBundleBuildError::AggregateLaneSettlementWpMismatch);
        }
        if matches!(lane_settlement.verdict, MteLaneVerdict::Pass) {
            if aggregate.promotion_counts.rejected > 0 {
                return Err(
                    CloseoutBundleBuildError::PassSettlementButAggregateHasRejections,
                );
            }
            if lane_settlement.approval.is_none() {
                return Err(CloseoutBundleBuildError::PassSettlementWithoutApproval);
            }
        }
        let per_mt_count = per_mt_summaries.len() as u32;
        if per_mt_count != aggregate.total_mts {
            return Err(
                CloseoutBundleBuildError::PerMtCountDisagreesWithAggregate {
                    per_mt_count,
                    aggregate_count: aggregate.total_mts,
                },
            );
        }
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            wp_id,
            lane_settlement,
            aggregate,
            per_mt_summaries,
            promotion_receipt_refs,
            artifact_bundle_refs,
            built_at_utc: Utc::now(),
        })
    }

    /// Helper for the Integration Validator: returns true when the bundle is
    /// safe to merge.
    ///
    /// H-C1 fix: also asserts zero rejected promotions and zero failed MTs.
    /// `build()` already enforces this for `Pass` settlement, but a bundle
    /// whose `aggregate` field is patched in place could otherwise bypass the
    /// invariant at read time. Defence in depth keeps the merge gate tight.
    pub fn ready_to_merge(&self) -> bool {
        matches!(self.lane_settlement.verdict, MteLaneVerdict::Pass)
            && self.aggregate.all_terminal()
            && self.aggregate.promotion_counts.rejected == 0
            && self.aggregate.status_counts.failed == 0
            && self.lane_settlement.approval.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::mte_aggregate_summary::{MtePromotionCounts, MteStatusCounts};
    use crate::kernel::mte_lane_settlement::LaneApprovalEvidenceV1;
    use crate::kernel::mte_per_mt_summary::{
        MteMtStatus, MtePromotionOutcomeView,
    };
    use crate::kernel::mte_validation_report_projection::MteValidationReportProjectionV1;
    use crate::kernel::validation::report::{DescriptorOutcome, ValidationReport};
    use crate::kernel::validation::status::ValidationStatus;
    use uuid::Uuid;

    fn pass_proj() -> MteValidationReportProjectionV1 {
        let mut r = ValidationReport::new(Uuid::now_v7());
        r.push(DescriptorOutcome::new("d", ValidationStatus::pass()));
        MteValidationReportProjectionV1::from_report(&r, None)
    }

    fn agg_pass(n: u32) -> MteAggregateSummaryV1 {
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

    fn settlement_pass(agg: MteAggregateSummaryV1) -> MteLaneSettlementV1 {
        let approval = LaneApprovalEvidenceV1 {
            operator_id: "op-ilja".into(),
            review_receipt_id: "OPR-1".into(),
            approval_source: "operator_review_receipt".into(),
            reason: "merge".into(),
        };
        MteLaneSettlementV1::settle_pass("WP-X", agg, approval, vec![], "ok").unwrap()
    }

    fn per_mt(n: u32) -> Vec<MtePerMtSummaryV1> {
        (0..n)
            .map(|i| {
                MtePerMtSummaryV1::build(
                    format!("MT-{i}"),
                    "WP-X",
                    Some(format!("SBX-{i}")),
                    MteMtStatus::Completed,
                    Some(pass_proj()),
                    MtePromotionOutcomeView::Accepted {
                        receipt_id: format!("PR-{i}"),
                    },
                    vec![],
                    0,
                    None,
                    None,
                    vec![],
                )
                .unwrap()
            })
            .collect()
    }

    // MT-058 acceptance: Integration Validator can review one bundle.
    #[test]
    fn happy_bundle_is_ready_to_merge() {
        let agg = agg_pass(2);
        let s = settlement_pass(agg.clone());
        let mts = per_mt(2);
        let b = MteCloseoutBundleV1::build(
            "WP-X",
            s,
            agg,
            mts,
            vec!["PR-0".into(), "PR-1".into()],
            vec!["ART-0".into()],
        )
        .unwrap();
        assert!(b.ready_to_merge());
    }

    #[test]
    fn wp_id_mismatch_rejected() {
        let agg = agg_pass(1);
        let s = settlement_pass(agg.clone());
        let mts = per_mt(1);
        let err = MteCloseoutBundleV1::build(
            "WP-OTHER",
            s,
            agg,
            mts,
            vec!["PR-0".into()],
            vec![],
        )
        .unwrap_err();
        assert_eq!(
            err,
            CloseoutBundleBuildError::AggregateLaneSettlementWpMismatch
        );
    }

    #[test]
    fn per_mt_count_must_match_aggregate() {
        let agg = agg_pass(2);
        let s = settlement_pass(agg.clone());
        let mts = per_mt(1); // mismatch
        let err = MteCloseoutBundleV1::build("WP-X", s, agg, mts, vec![], vec![])
            .unwrap_err();
        match err {
            CloseoutBundleBuildError::PerMtCountDisagreesWithAggregate {
                per_mt_count,
                aggregate_count,
            } => {
                assert_eq!(per_mt_count, 1);
                assert_eq!(aggregate_count, 2);
            }
            other => panic!("expected count disagreement, got {other:?}"),
        }
    }

    #[test]
    fn pass_settlement_refuses_rejected_in_aggregate() {
        let mut agg = agg_pass(1);
        agg.promotion_counts.rejected = 1;
        // Settlement was built before the corruption; bundle catches it.
        let s = settlement_pass(agg_pass(1));
        let mts = per_mt(1);
        let err = MteCloseoutBundleV1::build("WP-X", s, agg, mts, vec![], vec![])
            .unwrap_err();
        assert_eq!(
            err,
            CloseoutBundleBuildError::PassSettlementButAggregateHasRejections
        );
    }

    #[test]
    fn empty_wp_id_rejected() {
        let agg = agg_pass(0);
        let s = settlement_pass(agg.clone());
        let err = MteCloseoutBundleV1::build("", s, agg, vec![], vec![], vec![])
            .unwrap_err();
        assert_eq!(err, CloseoutBundleBuildError::EmptyWpId);
    }
}
