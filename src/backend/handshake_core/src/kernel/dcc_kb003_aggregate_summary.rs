//! MT-064 Aggregate Run Summary.
//!
//! Acceptance (MT-064.json): "no-context reviewer can inspect aggregate
//! before raw artifacts."
//!
//! Combines a sequence of [`Kb003PerMtSummaryV1`] records into a single
//! aggregate that summarizes the whole run: counts per status, ordered
//! blocked-reason taxonomy, and the artifact handles for follow-up.
//!
//! Frontend renders via existing dcc-* IPC surface.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::dcc_kb003_blocked_reasons::{BlockedDisposition, BlockedLane};
use crate::kernel::dcc_kb003_mt_summary::{Kb003PerMtSummaryV1, MtTerminalStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003BlockedTagCountV1 {
    pub tag: String,
    pub lane: BlockedLane,
    pub disposition: BlockedDisposition,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003AggregateRunSummaryV1 {
    pub schema_version: &'static str,
    pub aggregate_id: String,
    pub wp_id: String,
    pub completed_count: u32,
    pub blocked_count: u32,
    pub failed_count: u32,
    pub cancelled_count: u32,
    pub total_attempts: u32,
    pub blocked_tag_counts: Vec<Kb003BlockedTagCountV1>,
    pub summary_handles: Vec<String>,
    pub recorded_at_utc: DateTime<Utc>,
}

impl Kb003AggregateRunSummaryV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_aggregate_summary@1";

    pub fn from_per_mt(wp_id: impl Into<String>, summaries: &[Kb003PerMtSummaryV1]) -> Self {
        let mut completed = 0u32;
        let mut blocked = 0u32;
        let mut failed = 0u32;
        let mut cancelled = 0u32;
        let mut tag_buckets: Vec<Kb003BlockedTagCountV1> = Vec::new();
        let mut handles: Vec<String> = Vec::with_capacity(summaries.len());

        for s in summaries {
            handles.push(s.artifact_handle());
            match s.status {
                MtTerminalStatus::Completed => completed += 1,
                MtTerminalStatus::Blocked => blocked += 1,
                MtTerminalStatus::Failed => failed += 1,
                MtTerminalStatus::Cancelled => cancelled += 1,
            }
            if let Some(r) = &s.blocked_reason {
                let tag = r.tag().to_string();
                if let Some(entry) = tag_buckets.iter_mut().find(|b| b.tag == tag) {
                    entry.count += 1;
                } else {
                    tag_buckets.push(Kb003BlockedTagCountV1 {
                        tag,
                        lane: r.lane(),
                        disposition: r.disposition(),
                        count: 1,
                    });
                }
            }
        }

        // Stable order: by tag string ascending.
        tag_buckets.sort_by(|a, b| a.tag.cmp(&b.tag));

        Self {
            schema_version: Self::SCHEMA_VERSION,
            aggregate_id: format!("AGGSUM-{}", Uuid::new_v4()),
            wp_id: wp_id.into(),
            completed_count: completed,
            blocked_count: blocked,
            failed_count: failed,
            cancelled_count: cancelled,
            total_attempts: summaries.len() as u32,
            blocked_tag_counts: tag_buckets,
            summary_handles: handles,
            recorded_at_utc: Utc::now(),
        }
    }

    /// Acceptance helper: the aggregate is self-describing if a no-context
    /// reviewer can decide whether the WP is healthy from this struct alone.
    pub fn is_self_describing(&self) -> bool {
        if self.wp_id.is_empty() {
            return false;
        }
        if self.total_attempts == 0 {
            return true; // empty run is trivially complete to inspect.
        }
        // Counts add up.
        self.completed_count + self.blocked_count + self.failed_count + self.cancelled_count
            == self.total_attempts
            // Handles enumerated for follow-up inspection.
            && self.summary_handles.len() as u32 == self.total_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::dcc_kb003_blocked_reasons::BlockedReason;

    #[test]
    fn aggregate_counts_per_status_and_blocked_tags() {
        let s1 = Kb003PerMtSummaryV1::completed("MT-001", 0, vec![]);
        let s2 = Kb003PerMtSummaryV1::blocked(
            "MT-002",
            0,
            BlockedReason::MissingApproval { missing_field: "operator_id".into() },
            None,
            None,
            vec![],
        );
        let s3 = Kb003PerMtSummaryV1::blocked(
            "MT-003",
            0,
            BlockedReason::MissingApproval { missing_field: "operator_id".into() },
            None,
            None,
            vec![],
        );
        let s4 = Kb003PerMtSummaryV1::failed(
            "MT-004",
            0,
            BlockedReason::AdapterUnavailable { adapter_kind: "microvm".into(), host_detail: "no kvm".into() },
            vec![],
        );
        let agg = Kb003AggregateRunSummaryV1::from_per_mt("WP-KERNEL-003", &[s1, s2, s3, s4]);
        assert_eq!(agg.completed_count, 1);
        assert_eq!(agg.blocked_count, 2);
        assert_eq!(agg.failed_count, 1);
        assert_eq!(agg.total_attempts, 4);
        assert_eq!(agg.blocked_tag_counts.len(), 2);
        // missing_approval appears twice.
        let ma = agg
            .blocked_tag_counts
            .iter()
            .find(|t| t.tag == "BLOCKED_MISSING_APPROVAL")
            .unwrap();
        assert_eq!(ma.count, 2);
        assert!(agg.is_self_describing());
    }

    #[test]
    fn empty_aggregate_is_trivially_self_describing() {
        let agg = Kb003AggregateRunSummaryV1::from_per_mt("WP-X", &[]);
        assert_eq!(agg.total_attempts, 0);
        assert!(agg.is_self_describing());
    }
}
