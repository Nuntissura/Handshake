//! MT-051: Per-MT Summary (MTE layer).
//!
//! Acceptance (MT-051.json): "PromotionCandidate Contract: define promotion
//! candidate shape from patch proposal or write box. Acceptance: missing
//! validation refs block promotion."
//!
//! The per-MT summary is the MTE-layer "promotion candidate shape" for a
//! single microtask. It binds together everything the orchestrator and
//! validator need to look at *that MT*: the sandbox run id, the validation
//! projection (MT-050), the promotion outcome (if any), blockers, retry
//! count, and the evidence refs.
//!
//! Construction enforces the "missing validation refs block promotion"
//! acceptance: building a summary with `promotion_outcome=Some(Accepted)` but
//! without a validation projection is a typed `SummaryBuildError`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::kernel::mte_validation_report_projection::MteValidationReportProjectionV1;

/// Compact MT-level status as the MTE sees it. Distinct from
/// `SandboxRunStatus`: this is the *MTE scheduler* view, not the sandbox
/// adapter view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteMtStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Blocked,
    Cancelled,
}

impl MteMtStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Blocked => "BLOCKED",
            Self::Cancelled => "CANCELLED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Coarse validation verdict at the per-MT level. Mirrors the projection's
/// `blocks_promotion` plus a small enum for surfaces that do not want to
/// inspect the full projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MteValidationVerdict {
    NotRun,
    Pass,
    Fail,
    Blocked,
    Skipped,
}

impl MteValidationVerdict {
    pub fn from_projection(p: &MteValidationReportProjectionV1) -> Self {
        if p.blocks_promotion() {
            Self::Fail
        } else if p.total_outcomes == 0 {
            Self::Skipped
        } else {
            Self::Pass
        }
    }

    pub fn allows_promotion(&self) -> bool {
        matches!(self, Self::Pass)
    }
}

/// Promotion outcome view at the MTE per-MT level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MtePromotionOutcomeView {
    NotAttempted,
    Accepted { receipt_id: String },
    Rejected { reason_tag: String, rationale: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryBuildError {
    AcceptedWithoutValidationRefs,
    AcceptedWithoutPassingVerdict,
    EmptyMtId,
    InconsistentTimestamps,
}

impl std::fmt::Display for SummaryBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcceptedWithoutValidationRefs => write!(
                f,
                "MTE per-MT summary refuses Accepted promotion without a validation projection"
            ),
            Self::AcceptedWithoutPassingVerdict => write!(
                f,
                "MTE per-MT summary refuses Accepted promotion when validation verdict is not Pass"
            ),
            Self::EmptyMtId => write!(f, "mt_id must not be empty"),
            Self::InconsistentTimestamps => {
                write!(f, "finished_at_utc cannot precede started_at_utc")
            }
        }
    }
}

impl std::error::Error for SummaryBuildError {}

/// Per-MT MTE summary. Schema id `hsk.kernel.mte.per_mt_summary@1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtePerMtSummaryV1 {
    pub schema_version: &'static str,
    pub mt_id: String,
    pub wp_id: String,
    pub run_id: Option<String>,
    pub status: MteMtStatus,
    pub validation_verdict: MteValidationVerdict,
    pub validation_projection: Option<MteValidationReportProjectionV1>,
    pub promotion_outcome: MtePromotionOutcomeView,
    pub blockers: Vec<String>,
    pub retry_count: u32,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub finished_at_utc: Option<DateTime<Utc>>,
    pub evidence_refs: Vec<String>,
}

impl MtePerMtSummaryV1 {
    pub const SCHEMA_VERSION: &'static str = "hsk.kernel.mte.per_mt_summary@1";

    /// Builder-style constructor; validates the "missing validation refs
    /// block promotion" acceptance and basic identity invariants.
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        mt_id: impl Into<String>,
        wp_id: impl Into<String>,
        run_id: Option<String>,
        status: MteMtStatus,
        validation_projection: Option<MteValidationReportProjectionV1>,
        promotion_outcome: MtePromotionOutcomeView,
        blockers: Vec<String>,
        retry_count: u32,
        started_at_utc: Option<DateTime<Utc>>,
        finished_at_utc: Option<DateTime<Utc>>,
        evidence_refs: Vec<String>,
    ) -> Result<Self, SummaryBuildError> {
        let mt_id = mt_id.into();
        if mt_id.trim().is_empty() {
            return Err(SummaryBuildError::EmptyMtId);
        }
        if let (Some(s), Some(f)) = (started_at_utc, finished_at_utc) {
            if f < s {
                return Err(SummaryBuildError::InconsistentTimestamps);
            }
        }
        let validation_verdict = match &validation_projection {
            Some(p) => MteValidationVerdict::from_projection(p),
            None => MteValidationVerdict::NotRun,
        };
        // Acceptance: missing validation refs block promotion.
        if matches!(promotion_outcome, MtePromotionOutcomeView::Accepted { .. }) {
            if validation_projection.is_none() {
                return Err(SummaryBuildError::AcceptedWithoutValidationRefs);
            }
            if !validation_verdict.allows_promotion() {
                return Err(SummaryBuildError::AcceptedWithoutPassingVerdict);
            }
        }
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            mt_id,
            wp_id: wp_id.into(),
            run_id,
            status,
            validation_verdict,
            validation_projection,
            promotion_outcome,
            blockers,
            retry_count,
            started_at_utc,
            finished_at_utc,
            evidence_refs,
        })
    }

    /// Quick "ready to merge" view used by aggregate summaries.
    pub fn is_passed_and_accepted(&self) -> bool {
        self.status == MteMtStatus::Completed
            && matches!(self.validation_verdict, MteValidationVerdict::Pass)
            && matches!(self.promotion_outcome, MtePromotionOutcomeView::Accepted { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::validation::report::{DescriptorOutcome, ValidationReport};
    use crate::kernel::validation::status::ValidationStatus;
    use uuid::Uuid;

    fn pass_projection() -> MteValidationReportProjectionV1 {
        let mut r = ValidationReport::new(Uuid::now_v7());
        r.push(DescriptorOutcome::new("d", ValidationStatus::pass()));
        MteValidationReportProjectionV1::from_report(&r, Some("kb003://v/h".into()))
    }

    fn fail_projection() -> MteValidationReportProjectionV1 {
        let mut r = ValidationReport::new(Uuid::now_v7());
        r.push(DescriptorOutcome::new(
            "d",
            ValidationStatus::fail("nope").unwrap(),
        ));
        MteValidationReportProjectionV1::from_report(&r, None)
    }

    // MT-051 acceptance: missing validation refs block promotion.
    #[test]
    fn accepted_without_validation_projection_rejected() {
        let err = MtePerMtSummaryV1::build(
            "MT-001",
            "WP-X",
            Some("SBX-1".into()),
            MteMtStatus::Completed,
            None,
            MtePromotionOutcomeView::Accepted {
                receipt_id: "PR-1".into(),
            },
            vec![],
            0,
            None,
            None,
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, SummaryBuildError::AcceptedWithoutValidationRefs);
    }

    #[test]
    fn accepted_with_failing_validation_rejected() {
        let err = MtePerMtSummaryV1::build(
            "MT-001",
            "WP-X",
            Some("SBX-1".into()),
            MteMtStatus::Completed,
            Some(fail_projection()),
            MtePromotionOutcomeView::Accepted {
                receipt_id: "PR-1".into(),
            },
            vec![],
            0,
            None,
            None,
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, SummaryBuildError::AcceptedWithoutPassingVerdict);
    }

    #[test]
    fn accepted_with_passing_validation_builds() {
        let s = MtePerMtSummaryV1::build(
            "MT-001",
            "WP-X",
            Some("SBX-1".into()),
            MteMtStatus::Completed,
            Some(pass_projection()),
            MtePromotionOutcomeView::Accepted {
                receipt_id: "PR-1".into(),
            },
            vec![],
            0,
            None,
            None,
            vec!["kb003://artifact/h".into()],
        )
        .unwrap();
        assert!(s.is_passed_and_accepted());
        assert_eq!(s.validation_verdict, MteValidationVerdict::Pass);
    }

    #[test]
    fn empty_mt_id_rejected() {
        let err = MtePerMtSummaryV1::build(
            "   ",
            "WP-X",
            None,
            MteMtStatus::Pending,
            None,
            MtePromotionOutcomeView::NotAttempted,
            vec![],
            0,
            None,
            None,
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, SummaryBuildError::EmptyMtId);
    }

    #[test]
    fn inconsistent_timestamps_rejected() {
        let later = Utc::now();
        let earlier = later - chrono::Duration::seconds(60);
        let err = MtePerMtSummaryV1::build(
            "MT-1",
            "WP-X",
            None,
            MteMtStatus::Completed,
            None,
            MtePromotionOutcomeView::NotAttempted,
            vec![],
            0,
            Some(later),
            Some(earlier),
            vec![],
        )
        .unwrap_err();
        assert_eq!(err, SummaryBuildError::InconsistentTimestamps);
    }

    #[test]
    fn not_attempted_with_no_projection_is_fine() {
        let s = MtePerMtSummaryV1::build(
            "MT-2",
            "WP-X",
            None,
            MteMtStatus::Pending,
            None,
            MtePromotionOutcomeView::NotAttempted,
            vec![],
            0,
            None,
            None,
            vec![],
        )
        .unwrap();
        assert_eq!(s.validation_verdict, MteValidationVerdict::NotRun);
    }
}
