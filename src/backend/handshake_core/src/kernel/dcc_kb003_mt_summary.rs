//! MT-063 Per-MT Summary Artifact.
//!
//! Acceptance (MT-063.json): "every completed/blocked MT attempt has summary
//! ref."
//!
//! `Kb003PerMtSummaryV1` is the durable per-microtask summary a no-context
//! reviewer reads before opening raw artifacts. It carries:
//! - mt_id + attempt index
//! - terminal status (completed / blocked / failed / cancelled)
//! - typed blocked reason if applicable
//! - drop-back evaluation if applicable
//! - retry budget snapshot if applicable
//! - bounded artifact ref list for opening from DCC.
//!
//! The summary is persisted as a JSON artifact under
//! `handshake-product/kb003/mt_summaries/`. The `summary_ref` field on every
//! per-MT result row points back to the summary handle so DCC can deep-link.
//!
//! Frontend renders via existing dcc-* IPC surface.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kernel::dcc_kb003_blocked_reasons::BlockedReason;
use crate::kernel::dcc_kb003_dropback::DccKb003DropBackEvaluationV1;
use crate::kernel::dcc_kb003_retry_budget::DccKb003RetryBudgetSnapshotV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MtTerminalStatus {
    Completed,
    Blocked,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kb003PerMtSummaryV1 {
    pub schema_version: String,
    pub summary_id: String,
    pub mt_id: String,
    pub attempt_index: u32,
    pub status: MtTerminalStatus,
    pub blocked_reason: Option<BlockedReason>,
    pub dropback: Option<DccKb003DropBackEvaluationV1>,
    pub retry_snapshot: Option<DccKb003RetryBudgetSnapshotV1>,
    pub artifact_refs: Vec<String>,
    pub recorded_at_utc: DateTime<Utc>,
}

impl Kb003PerMtSummaryV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.kb003_mt_summary@1";

    pub fn completed(mt_id: impl Into<String>, attempt_index: u32, artifact_refs: Vec<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            summary_id: format!("MTSUM-{}", Uuid::now_v7()),
            mt_id: mt_id.into(),
            attempt_index,
            status: MtTerminalStatus::Completed,
            blocked_reason: None,
            dropback: None,
            retry_snapshot: None,
            artifact_refs,
            recorded_at_utc: Utc::now(),
        }
    }

    pub fn blocked(
        mt_id: impl Into<String>,
        attempt_index: u32,
        reason: BlockedReason,
        dropback: Option<DccKb003DropBackEvaluationV1>,
        retry_snapshot: Option<DccKb003RetryBudgetSnapshotV1>,
        artifact_refs: Vec<String>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            summary_id: format!("MTSUM-{}", Uuid::now_v7()),
            mt_id: mt_id.into(),
            attempt_index,
            status: MtTerminalStatus::Blocked,
            blocked_reason: Some(reason),
            dropback,
            retry_snapshot,
            artifact_refs,
            recorded_at_utc: Utc::now(),
        }
    }

    pub fn failed(
        mt_id: impl Into<String>,
        attempt_index: u32,
        reason: BlockedReason,
        artifact_refs: Vec<String>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            summary_id: format!("MTSUM-{}", Uuid::now_v7()),
            mt_id: mt_id.into(),
            attempt_index,
            status: MtTerminalStatus::Failed,
            blocked_reason: Some(reason),
            dropback: None,
            retry_snapshot: None,
            artifact_refs,
            recorded_at_utc: Utc::now(),
        }
    }

    /// Canonical artifact handle for cross-referencing from per-attempt rows.
    pub fn artifact_handle(&self) -> String {
        format!("kb003://mt_summary/{}", self.summary_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::dcc_kb003_blocked_reasons::BlockedReason;

    #[test]
    fn completed_summary_has_summary_ref_and_no_reason() {
        let s = Kb003PerMtSummaryV1::completed("MT-001", 0, vec!["ART-1".into()]);
        assert!(s.summary_id.starts_with("MTSUM-"));
        assert_eq!(s.status, MtTerminalStatus::Completed);
        assert!(s.blocked_reason.is_none());
        assert!(s.artifact_handle().starts_with("kb003://mt_summary/"));
    }

    #[test]
    fn blocked_summary_carries_typed_reason_and_handle() {
        let reason = BlockedReason::AdapterUnavailable {
            adapter_kind: "microvm".into(),
            host_detail: "no kvm".into(),
        };
        let s = Kb003PerMtSummaryV1::blocked("MT-002", 1, reason.clone(), None, None, vec![]);
        assert_eq!(s.status, MtTerminalStatus::Blocked);
        assert_eq!(s.blocked_reason.as_ref().unwrap().tag(), reason.tag());
        assert!(s.artifact_handle().contains(&s.summary_id));
    }

    #[test]
    fn failed_summary_records_typed_reason() {
        let reason = BlockedReason::MissingApproval { missing_field: "operator_id".into() };
        let s = Kb003PerMtSummaryV1::failed("MT-003", 0, reason, vec![]);
        assert_eq!(s.status, MtTerminalStatus::Failed);
        assert!(s.blocked_reason.is_some());
    }
}
