//! MT-010 DCC Projection Contract for sandbox and promotion state.
//!
//! Acceptance (MT-010.json): "no-context model can inspect state without
//! terminal logs." This module declares the minimum operator-visible
//! projection: enough structured state for a model or operator to answer
//! "what's the current sandbox run doing, why was it denied/rejected, what
//! validations and promotion decision followed?" without reading provider
//! chat, terminal scrollback, or transient logs.
//!
//! The projection is *derived*: it never carries authority. Authority lives
//! in `SandboxRunV1`, `SandboxPolicyV1`, the validation/promotion records, and
//! the EventLedger. This file is the contract the DCC view layer ingests when
//! it renders the sandbox/promotion lane.

use serde::{Deserialize, Serialize};

use super::policy::CapabilityDecision;
use super::run::SandboxRunStatus;
use crate::kernel::kb003_artifact_classes::Kb003ArtifactClass;
use crate::kernel::kb003_schemas::{
    SCHEMA_KERNEL_PROMOTION_DECISION_V1, SCHEMA_KERNEL_PROMOTION_RECEIPT_V1,
    SCHEMA_KERNEL_SANDBOX_POLICY_V1, SCHEMA_KERNEL_SANDBOX_RUN_V1, SCHEMA_KERNEL_VALIDATION_RUN_V1,
};

/// Projection family id; the DCC layout registry pins this preset by id so
/// the operator sees the same sandbox lane every session.
pub const DCC_SANDBOX_PROJECTION_FAMILY_ID: &str = "hsk.dcc.kb003.sandbox_promotion_lane@1";

/// High-level outcome states the operator can see at a glance, independent of
/// the underlying lifecycle granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DccSandboxOutcome {
    Pending,
    Running,
    DeniedByPolicy,
    FailedValidation,
    AwaitingPromotion,
    Promoted,
    Rejected,
}

impl DccSandboxOutcome {
    pub fn derive(
        status: SandboxRunStatus,
        has_denial: bool,
        has_promotion_accepted: bool,
    ) -> Self {
        match status {
            SandboxRunStatus::Requested => Self::Pending,
            SandboxRunStatus::Started => Self::Running,
            SandboxRunStatus::Rejected if has_denial => Self::DeniedByPolicy,
            SandboxRunStatus::Rejected => Self::Rejected,
            SandboxRunStatus::Completed if has_promotion_accepted => Self::Promoted,
            SandboxRunStatus::Completed => Self::AwaitingPromotion,
        }
    }
}

/// Minimum operator-readable row describing a capability decision recorded by
/// the active policy. Source: `SandboxPolicyV1::decide(...)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccCapabilityRowV1 {
    pub capability: String,
    pub decision: CapabilityDecision,
}

/// Compact denial summary; full record lives under
/// `SandboxDenialRecordV1`. The projection carries enough that a no-context
/// model knows what was blocked and which policy version blocked it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccDenialSummaryV1 {
    pub denial_id: String,
    pub kind: String,
    pub capability: Option<String>,
    pub action_description: String,
    pub reason: String,
    pub policy_version_id: String,
}

/// Compact validation summary visible to the operator without reading the
/// full validation report artifact. Mirrors fields the validator runner will
/// write (kept stable for MT-013 storage and MT-030+ validator MTs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccValidationSummaryV1 {
    pub validation_run_id: String,
    pub verdict: String,
    pub check_count: u32,
    pub failed_check_count: u32,
    pub report_artifact_ref: Option<String>,
}

/// Compact promotion view visible to the operator. `decision` is the typed
/// PROMOTED/REJECTED/HELD verdict; `receipt_ref` points to the durable receipt
/// artifact (`PromotionReceipt` class).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccPromotionSummaryV1 {
    pub decision_id: String,
    pub decision: String,
    pub receipt_id: Option<String>,
    pub receipt_artifact_ref: Option<String>,
    pub rationale_short: String,
}

/// Full operator projection row.
///
/// Every field is sourced from durable state (`SandboxRunV1`, the policy
/// record, the validation/promotion records, the artifact registry). A
/// no-context model reading this projection knows the run identity, current
/// outcome, denied capabilities (if any), validation outcome (if any), and
/// promotion outcome (if any) without touching terminal logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccSandboxProjectionV1 {
    pub projection_family_id: String,

    // Identity (sourced from SandboxRunV1).
    pub run_id: String,
    pub kernel_task_run_id: String,
    pub session_run_id: String,
    pub adapter_kind: String,

    // Policy snapshot (sourced from SandboxPolicyV1).
    pub policy_version_id: String,
    pub policy_default_decision: CapabilityDecision,
    pub capability_rows: Vec<DccCapabilityRowV1>,

    // Workspace snapshot (sourced from SandboxWorkspaceV1).
    pub workspace_id: String,
    pub workspace_root_relative: String,

    // Lifecycle.
    pub run_status: SandboxRunStatus,
    pub outcome: DccSandboxOutcome,
    pub requested_at_utc: String,
    pub started_at_utc: Option<String>,
    pub finished_at_utc: Option<String>,

    // Evidence summaries (denials, validation, promotion).
    pub denial: Option<DccDenialSummaryV1>,
    pub validation: Option<DccValidationSummaryV1>,
    pub promotion: Option<DccPromotionSummaryV1>,

    // Artifact references the operator may open from DCC.
    pub artifact_refs: Vec<String>,
    pub artifact_classes_in_view: Vec<Kb003ArtifactClass>,

    // Authoritative source ids the projection was derived from. Used by tests
    // and by the DCC layer to render "freshness" badges.
    pub source_schema_ids: Vec<String>,
}

impl DccSandboxProjectionV1 {
    /// Returns the canonical set of source schema ids a complete sandbox-lane
    /// projection draws from. The DCC layer uses this to render freshness
    /// indicators and to refuse a render when an upstream schema is missing.
    pub fn canonical_source_schema_ids() -> Vec<&'static str> {
        vec![
            SCHEMA_KERNEL_SANDBOX_RUN_V1,
            SCHEMA_KERNEL_SANDBOX_POLICY_V1,
            SCHEMA_KERNEL_VALIDATION_RUN_V1,
            SCHEMA_KERNEL_PROMOTION_DECISION_V1,
            SCHEMA_KERNEL_PROMOTION_RECEIPT_V1,
        ]
    }

    /// Acceptance helper: does this projection let a no-context reader answer
    /// "what is going on?" without terminal logs? The check is structural:
    /// identity present, lifecycle present, and the evidence summary that
    /// matches the current outcome is present.
    pub fn is_self_describing(&self) -> bool {
        if self.run_id.is_empty() || self.policy_version_id.is_empty() {
            return false;
        }
        match self.outcome {
            DccSandboxOutcome::DeniedByPolicy => self.denial.is_some(),
            DccSandboxOutcome::Promoted => self.promotion.is_some() && self.validation.is_some(),
            DccSandboxOutcome::FailedValidation => self.validation.is_some(),
            DccSandboxOutcome::AwaitingPromotion => self.validation.is_some(),
            DccSandboxOutcome::Pending
            | DccSandboxOutcome::Running
            | DccSandboxOutcome::Rejected => {
                // Pending/Running need no extra evidence; raw Rejected may exist before a
                // denial record is persisted (e.g. adapter unavailable before policy check).
                true
            }
        }
    }

    /// MT-067/068 helper consumed by `dcc_kb003_sandbox_run_list` and
    /// `dcc_kb003_run_detail`: short stable summary line a no-context reader
    /// can read at a glance.
    pub fn summary_line(&self) -> String {
        format!(
            "run {} [{:?}] outcome={:?} policy={}",
            self.run_id, self.run_status, self.outcome, self.policy_version_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_derives_from_status_and_evidence() {
        // Fully-qualify both `Rejected` variants since `DccSandboxOutcome` AND
        // `SandboxRunStatus` both have a `Rejected` variant in scope. The
        // glob imports created an ambiguity the compiler refuses to resolve.
        use DccSandboxOutcome as Out;
        use SandboxRunStatus as St;
        assert_eq!(Out::derive(St::Requested, false, false), Out::Pending);
        assert_eq!(Out::derive(St::Started, false, false), Out::Running);
        assert_eq!(Out::derive(St::Rejected, true, false), Out::DeniedByPolicy);
        assert_eq!(Out::derive(St::Rejected, false, false), Out::Rejected);
        assert_eq!(
            Out::derive(St::Completed, false, false),
            Out::AwaitingPromotion
        );
        assert_eq!(Out::derive(St::Completed, false, true), Out::Promoted);
    }

    #[test]
    fn projection_lists_all_kb003_source_schemas() {
        let sources = DccSandboxProjectionV1::canonical_source_schema_ids();
        assert!(sources.contains(&SCHEMA_KERNEL_SANDBOX_RUN_V1));
        assert!(sources.contains(&SCHEMA_KERNEL_SANDBOX_POLICY_V1));
        assert!(sources.contains(&SCHEMA_KERNEL_VALIDATION_RUN_V1));
        assert!(sources.contains(&SCHEMA_KERNEL_PROMOTION_DECISION_V1));
        assert!(sources.contains(&SCHEMA_KERNEL_PROMOTION_RECEIPT_V1));
    }

    #[test]
    fn projection_family_id_is_versioned_and_namespaced() {
        assert!(DCC_SANDBOX_PROJECTION_FAMILY_ID.starts_with("hsk.dcc.kb003."));
        assert!(DCC_SANDBOX_PROJECTION_FAMILY_ID.contains('@'));
    }

    #[test]
    fn summary_line_includes_identity_and_outcome() {
        let p = sample_projection(
            SandboxRunStatus::Completed,
            DccSandboxOutcome::AwaitingPromotion,
        );
        let line = p.summary_line();
        assert!(line.contains("SBX-test"));
        assert!(line.contains("AwaitingPromotion"));
    }

    #[test]
    fn self_describing_requires_evidence_for_denied_outcome() {
        let mut projection = sample_projection(
            SandboxRunStatus::Rejected,
            DccSandboxOutcome::DeniedByPolicy,
        );
        // No denial attached => not self-describing.
        projection.denial = None;
        assert!(!projection.is_self_describing());

        projection.denial = Some(DccDenialSummaryV1 {
            denial_id: "DEN-1".into(),
            kind: "POLICY_DENIED".into(),
            capability: Some("NETWORK".into()),
            action_description: "fetch https://x".into(),
            reason: "default_deny NETWORK".into(),
            policy_version_id: "POL-1@1".into(),
        });
        assert!(
            projection.is_self_describing(),
            "denial summary makes the projection self-describing"
        );
    }

    #[test]
    fn self_describing_requires_promotion_and_validation_for_promoted_outcome() {
        let mut projection =
            sample_projection(SandboxRunStatus::Completed, DccSandboxOutcome::Promoted);
        projection.validation = None;
        projection.promotion = None;
        assert!(!projection.is_self_describing());

        projection.validation = Some(DccValidationSummaryV1 {
            validation_run_id: "VR-1".into(),
            verdict: "PASS".into(),
            check_count: 3,
            failed_check_count: 0,
            report_artifact_ref: Some("ART-vr-1".into()),
        });
        projection.promotion = Some(DccPromotionSummaryV1 {
            decision_id: "PD-1".into(),
            decision: "PROMOTED".into(),
            receipt_id: Some("PR-1".into()),
            receipt_artifact_ref: Some("ART-pr-1".into()),
            rationale_short: "all checks passed".into(),
        });
        assert!(projection.is_self_describing());
    }

    fn sample_projection(
        status: SandboxRunStatus,
        outcome: DccSandboxOutcome,
    ) -> DccSandboxProjectionV1 {
        DccSandboxProjectionV1 {
            projection_family_id: DCC_SANDBOX_PROJECTION_FAMILY_ID.to_string(),
            run_id: "SBX-test".into(),
            kernel_task_run_id: "KTR-1".into(),
            session_run_id: "SES-1".into(),
            adapter_kind: "policy_scoped_local".into(),
            policy_version_id: "POL-1@1".into(),
            policy_default_decision: CapabilityDecision::Deny,
            capability_rows: Vec::new(),
            workspace_id: "WSP-1".into(),
            workspace_root_relative: "handshake-product/kb003/work/x".into(),
            run_status: status,
            outcome,
            requested_at_utc: "2026-05-17T00:00:00Z".into(),
            started_at_utc: None,
            finished_at_utc: None,
            denial: None,
            validation: None,
            promotion: None,
            artifact_refs: Vec::new(),
            artifact_classes_in_view: vec![Kb003ArtifactClass::SandboxLog],
            source_schema_ids: DccSandboxProjectionV1::canonical_source_schema_ids()
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}
