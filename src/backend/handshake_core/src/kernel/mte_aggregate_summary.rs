//! MT-053: MTE Aggregate Summary (Promotion Accept Path support).
//!
//! Acceptance (MT-053.json): "Promotion Accept Path: append accepted
//! promotion events to EventLedger. Acceptance: accepted promotion is
//! replayable from durable events."
//!
//! The MTE aggregate summary is the WP-level rollup over the per-MT summaries
//! defined in `mte_per_mt_summary.rs`. It folds in *which* MTs reached an
//! accepted promotion, *which* are still in flight, and *which* are blocked
//! (with their `MteBlockedReason`). Surfaces use this summary to render WP
//! status and to decide when the lane is ready to settle.
//!
//! Replayability note: the aggregate is a pure projection over per-MT
//! summaries (which themselves project the durable sandbox/validation/
//! promotion records). Replaying the durable events rebuilds the per-MT
//! summaries, and `MteAggregateSummaryV1::from_per_mt` rebuilds this
//! aggregate deterministically.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::kernel::mte_blocked_taxonomy::MteBlockedReason;
use crate::kernel::mte_per_mt_summary::{
    MteMtStatus, MtePerMtSummaryV1, MtePromotionOutcomeView, MteValidationVerdict,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MteStatusCounts {
    pub pending: u32,
    pub running: u32,
    pub completed: u32,
    pub failed: u32,
    pub blocked: u32,
    pub cancelled: u32,
}

impl MteStatusCounts {
    pub fn total(&self) -> u32 {
        self.pending
            + self.running
            + self.completed
            + self.failed
            + self.blocked
            + self.cancelled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MtePromotionCounts {
    pub accepted: u32,
    pub rejected: u32,
    pub not_attempted: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MteAggregateSummaryV1 {
    pub schema_version: &'static str,
    pub wp_id: String,
    pub total_mts: u32,
    pub status_counts: MteStatusCounts,
    pub promotion_counts: MtePromotionCounts,
    /// Count of MTs currently blocked, keyed by `MteBlockedReason::tag()`.
    pub blocker_counts_by_tag: BTreeMap<String, u32>,
    /// Sum of `finished_at - started_at` across MTs that report both.
    pub total_wall_time_ms: u64,
    /// Oldest MT that has not reached a terminal status (`Completed`,
    /// `Failed`, `Cancelled`). `None` when all MTs are terminal.
    pub oldest_pending_mt_id: Option<String>,
    pub oldest_pending_started_at_utc: Option<DateTime<Utc>>,
    /// Receipt-id refs for accepted promotions; replayable from durable
    /// events.
    pub accepted_promotion_receipt_ids: Vec<String>,
}

impl MteAggregateSummaryV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.mte.aggregate_summary@1";

    pub fn from_per_mt(wp_id: impl Into<String>, summaries: &[MtePerMtSummaryV1]) -> Self {
        let mut status_counts = MteStatusCounts::default();
        let mut promotion_counts = MtePromotionCounts::default();
        let mut blocker_counts: BTreeMap<String, u32> = BTreeMap::new();
        let mut wall_time_ms: u64 = 0;
        let mut oldest: Option<(String, DateTime<Utc>)> = None;
        let mut accepted_ids: Vec<String> = Vec::new();

        for s in summaries {
            match s.status {
                MteMtStatus::Pending => status_counts.pending += 1,
                MteMtStatus::Running => status_counts.running += 1,
                MteMtStatus::Completed => status_counts.completed += 1,
                MteMtStatus::Failed => status_counts.failed += 1,
                MteMtStatus::Blocked => status_counts.blocked += 1,
                MteMtStatus::Cancelled => status_counts.cancelled += 1,
            }
            match &s.promotion_outcome {
                MtePromotionOutcomeView::Accepted { receipt_id } => {
                    promotion_counts.accepted += 1;
                    accepted_ids.push(receipt_id.clone());
                }
                MtePromotionOutcomeView::Rejected { .. } => promotion_counts.rejected += 1,
                MtePromotionOutcomeView::NotAttempted => promotion_counts.not_attempted += 1,
            }
            if s.status == MteMtStatus::Blocked {
                for b in &s.blockers {
                    *blocker_counts.entry(b.clone()).or_insert(0) += 1;
                }
            }
            if let (Some(start), Some(end)) = (s.started_at_utc, s.finished_at_utc) {
                let dur = end.signed_duration_since(start);
                if dur.num_milliseconds() > 0 {
                    wall_time_ms = wall_time_ms
                        .saturating_add(dur.num_milliseconds() as u64);
                }
            }
            if !s.status.is_terminal() {
                if let Some(start) = s.started_at_utc {
                    let take = match &oldest {
                        Some((_, t)) => start < *t,
                        None => true,
                    };
                    if take {
                        oldest = Some((s.mt_id.clone(), start));
                    }
                }
            }
        }

        // Deterministic order on the accepted receipt list.
        accepted_ids.sort();

        Self {
            schema_version: Self::SCHEMA_VERSION,
            wp_id: wp_id.into(),
            total_mts: summaries.len() as u32,
            status_counts,
            promotion_counts,
            blocker_counts_by_tag: blocker_counts,
            total_wall_time_ms: wall_time_ms,
            oldest_pending_mt_id: oldest.as_ref().map(|(id, _)| id.clone()),
            oldest_pending_started_at_utc: oldest.map(|(_, t)| t),
            accepted_promotion_receipt_ids: accepted_ids,
        }
    }

    /// Convenience helper: bump a blocker count from a typed reason. Used by
    /// callers that compute blockers separately and want to fold them in.
    pub fn record_blocker(&mut self, reason: &MteBlockedReason) {
        *self
            .blocker_counts_by_tag
            .entry(reason.tag().to_string())
            .or_insert(0) += 1;
    }

    pub fn all_terminal(&self) -> bool {
        self.status_counts.pending == 0
            && self.status_counts.running == 0
            && self.status_counts.blocked == 0
    }
}

/// View that indicates whether `MteValidationVerdict::Pass` should be required
/// for every accepted MT in the WP. Used by the lane settlement layer.
pub fn count_passing_mts(summaries: &[MtePerMtSummaryV1]) -> u32 {
    summaries
        .iter()
        .filter(|s| matches!(s.validation_verdict, MteValidationVerdict::Pass))
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::mte_per_mt_summary::{
        MteMtStatus, MtePerMtSummaryV1, MtePromotionOutcomeView,
    };
    use crate::kernel::mte_validation_report_projection::MteValidationReportProjectionV1;
    use crate::kernel::validation::report::{DescriptorOutcome, ValidationReport};
    use crate::kernel::validation::status::ValidationStatus;
    use uuid::Uuid;

    fn pass_projection() -> MteValidationReportProjectionV1 {
        let mut r = ValidationReport::new(Uuid::new_v4());
        r.push(DescriptorOutcome::new("d", ValidationStatus::pass()));
        MteValidationReportProjectionV1::from_report(&r, None)
    }

    fn summary(
        id: &str,
        status: MteMtStatus,
        promo: MtePromotionOutcomeView,
        blockers: Vec<String>,
        started: Option<DateTime<Utc>>,
        finished: Option<DateTime<Utc>>,
    ) -> MtePerMtSummaryV1 {
        let proj = if matches!(promo, MtePromotionOutcomeView::Accepted { .. }) {
            Some(pass_projection())
        } else {
            None
        };
        MtePerMtSummaryV1::build(
            id, "WP-X", None, status, proj, promo, blockers, 0, started, finished,
            vec![],
        )
        .unwrap()
    }

    // MT-053 acceptance: accepted promotions surface in the aggregate (the
    // event ledger is the canonical record; this aggregate is the projection
    // that replay rebuilds).
    #[test]
    fn aggregate_lists_accepted_receipt_ids() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(10);
        let a = summary(
            "MT-1",
            MteMtStatus::Completed,
            MtePromotionOutcomeView::Accepted {
                receipt_id: "PR-a".into(),
            },
            vec![],
            Some(earlier),
            Some(now),
        );
        let b = summary(
            "MT-2",
            MteMtStatus::Completed,
            MtePromotionOutcomeView::Accepted {
                receipt_id: "PR-b".into(),
            },
            vec![],
            Some(earlier),
            Some(now),
        );
        let c = summary(
            "MT-3",
            MteMtStatus::Failed,
            MtePromotionOutcomeView::Rejected {
                reason_tag: "REJECTED_VALIDATION_FAILURE".into(),
                rationale: "blocked".into(),
            },
            vec![],
            Some(earlier),
            Some(now),
        );

        let agg = MteAggregateSummaryV1::from_per_mt("WP-X", &[a, b, c]);
        assert_eq!(agg.total_mts, 3);
        assert_eq!(agg.promotion_counts.accepted, 2);
        assert_eq!(agg.promotion_counts.rejected, 1);
        assert_eq!(
            agg.accepted_promotion_receipt_ids,
            vec!["PR-a".to_string(), "PR-b".to_string()]
        );
        assert!(agg.all_terminal());
        assert!(agg.total_wall_time_ms >= 10_000); // 3 MTs * ~10s
    }

    #[test]
    fn aggregate_tracks_oldest_pending() {
        let now = Utc::now();
        let old = now - chrono::Duration::seconds(120);
        let a = summary(
            "MT-1",
            MteMtStatus::Running,
            MtePromotionOutcomeView::NotAttempted,
            vec![],
            Some(old),
            None,
        );
        let b = summary(
            "MT-2",
            MteMtStatus::Running,
            MtePromotionOutcomeView::NotAttempted,
            vec![],
            Some(now),
            None,
        );
        let agg = MteAggregateSummaryV1::from_per_mt("WP-X", &[a, b]);
        assert_eq!(agg.oldest_pending_mt_id.as_deref(), Some("MT-1"));
        assert!(!agg.all_terminal());
    }

    #[test]
    fn aggregate_folds_blocker_tags() {
        let s = summary(
            "MT-1",
            MteMtStatus::Blocked,
            MtePromotionOutcomeView::NotAttempted,
            vec!["MTE_BLOCKED_DEPENDENCY_MISSING".into()],
            None,
            None,
        );
        let agg = MteAggregateSummaryV1::from_per_mt("WP-X", &[s]);
        assert_eq!(
            agg.blocker_counts_by_tag
                .get("MTE_BLOCKED_DEPENDENCY_MISSING"),
            Some(&1)
        );
        assert_eq!(agg.status_counts.blocked, 1);
    }

    #[test]
    fn record_blocker_increments_typed_counter() {
        let mut agg = MteAggregateSummaryV1::from_per_mt("WP-X", &[]);
        agg.record_blocker(&MteBlockedReason::OperatorPaused {
            reason: "manual".into(),
        });
        agg.record_blocker(&MteBlockedReason::OperatorPaused {
            reason: "manual".into(),
        });
        assert_eq!(
            agg.blocker_counts_by_tag
                .get("MTE_BLOCKED_OPERATOR_PAUSED"),
            Some(&2)
        );
    }
}
