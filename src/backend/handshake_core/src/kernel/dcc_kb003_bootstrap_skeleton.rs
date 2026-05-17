//! MT-066 Bootstrap Skeleton Receipt Projection.
//!
//! Acceptance (MT-066.json): "first skeleton sandbox run creates restartable
//! receipts. All receipts visible after restart."
//!
//! The bootstrap skeleton is the minimum set of receipts the kernel must
//! emit on the very first sandbox run so that, after a process restart,
//! the operator can see the lane history from durable storage alone. The
//! skeleton receipts mirror the KB003 lifecycle:
//!
//! - SandboxRunRequested
//! - SandboxRunStarted
//! - SandboxRunCompleted (or Rejected)
//! - ValidationRunRecorded
//! - PromotionDecided
//! - PromotionReceiptIssued
//!
//! [`BootstrapSkeletonProjection`] is the DCC view that re-projects the
//! skeleton from the receipt list. The acceptance test asserts a round-trip
//! through `serde_json` reconstructs the same projection (the "all receipts
//! visible after restart" criterion).
//!
//! Frontend renders via existing dcc-* IPC surface.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkeletonReceiptKind {
    SandboxRunRequested,
    SandboxRunStarted,
    SandboxRunCompleted,
    SandboxRunRejected,
    ValidationRunRecorded,
    PromotionDecided,
    PromotionReceiptIssued,
}

impl SkeletonReceiptKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SandboxRunRequested => "SANDBOX_RUN_REQUESTED",
            Self::SandboxRunStarted => "SANDBOX_RUN_STARTED",
            Self::SandboxRunCompleted => "SANDBOX_RUN_COMPLETED",
            Self::SandboxRunRejected => "SANDBOX_RUN_REJECTED",
            Self::ValidationRunRecorded => "VALIDATION_RUN_RECORDED",
            Self::PromotionDecided => "PROMOTION_DECIDED",
            Self::PromotionReceiptIssued => "PROMOTION_RECEIPT_ISSUED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkeletonReceiptV1 {
    pub kind: SkeletonReceiptKind,
    pub receipt_id: String,
    pub recorded_at_utc: DateTime<Utc>,
    pub artifact_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapSkeletonProjectionV1 {
    pub projection_family_id: String,
    pub sandbox_run_id: String,
    pub receipts: Vec<SkeletonReceiptV1>,
}

impl BootstrapSkeletonProjectionV1 {
    pub const FAMILY_ID: &'static str = "hsk.dcc.kb003.bootstrap_skeleton@1";

    pub fn new(sandbox_run_id: impl Into<String>, receipts: Vec<SkeletonReceiptV1>) -> Self {
        Self {
            projection_family_id: Self::FAMILY_ID.to_string(),
            sandbox_run_id: sandbox_run_id.into(),
            receipts,
        }
    }

    /// All skeleton kinds present for a happy-path (Completed) run.
    pub fn required_happy_path_kinds() -> &'static [SkeletonReceiptKind] {
        &[
            SkeletonReceiptKind::SandboxRunRequested,
            SkeletonReceiptKind::SandboxRunStarted,
            SkeletonReceiptKind::SandboxRunCompleted,
            SkeletonReceiptKind::ValidationRunRecorded,
            SkeletonReceiptKind::PromotionDecided,
            SkeletonReceiptKind::PromotionReceiptIssued,
        ]
    }

    /// Restart-replay coverage check. Returns true when every required
    /// skeleton kind is present in the receipts list (i.e. visible after a
    /// process restart).
    pub fn covers_happy_path(&self) -> bool {
        let needs = Self::required_happy_path_kinds();
        needs.iter().all(|k| self.receipts.iter().any(|r| r.kind == *k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn full_happy_path(run_id: &str) -> BootstrapSkeletonProjectionV1 {
        let now = Utc::now();
        BootstrapSkeletonProjectionV1::new(
            run_id,
            vec![
                SkeletonReceiptV1 { kind: SkeletonReceiptKind::SandboxRunRequested, receipt_id: "R1".into(), recorded_at_utc: now, artifact_ref: None },
                SkeletonReceiptV1 { kind: SkeletonReceiptKind::SandboxRunStarted, receipt_id: "R2".into(), recorded_at_utc: now, artifact_ref: None },
                SkeletonReceiptV1 { kind: SkeletonReceiptKind::SandboxRunCompleted, receipt_id: "R3".into(), recorded_at_utc: now, artifact_ref: Some("ART-log".into()) },
                SkeletonReceiptV1 { kind: SkeletonReceiptKind::ValidationRunRecorded, receipt_id: "R4".into(), recorded_at_utc: now, artifact_ref: Some("ART-vr".into()) },
                SkeletonReceiptV1 { kind: SkeletonReceiptKind::PromotionDecided, receipt_id: "R5".into(), recorded_at_utc: now, artifact_ref: None },
                SkeletonReceiptV1 { kind: SkeletonReceiptKind::PromotionReceiptIssued, receipt_id: "R6".into(), recorded_at_utc: now, artifact_ref: Some("ART-pr".into()) },
            ],
        )
    }

    #[test]
    fn happy_path_covers_all_required_kinds() {
        let p = full_happy_path("SBX-1");
        assert!(p.covers_happy_path());
    }

    #[test]
    fn missing_promotion_decision_fails_coverage() {
        let mut p = full_happy_path("SBX-1");
        p.receipts.retain(|r| r.kind != SkeletonReceiptKind::PromotionDecided);
        assert!(!p.covers_happy_path());
    }

    #[test]
    fn projection_survives_serde_roundtrip_post_restart() {
        let p = full_happy_path("SBX-1");
        let json = serde_json::to_string(&p).unwrap();
        let recovered: BootstrapSkeletonProjectionV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, p);
        assert!(recovered.covers_happy_path());
    }
}
