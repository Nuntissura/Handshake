//! MT-016 Replay projection contract.
//!
//! Acceptance (MT-016.json): "replay does not read provider chat, terminal
//! scrollback, or transient logs." This module declares the *input contract*
//! for a sandbox-run replay: the only inputs a reconstructor is allowed to
//! consume are durable rows and EventLedger events keyed by `run_id`.
//!
//! The actual SQL query lives in `crate::storage::kb003_storage`; this module
//! owns the contract a query implementation must satisfy and a deterministic
//! reconstructor that turns the durable inputs into a projection.

use serde::{Deserialize, Serialize};

use super::dcc_projection::{
    DccCapabilityRowV1, DccDenialSummaryV1, DccPromotionSummaryV1, DccSandboxOutcome,
    DccSandboxProjectionV1, DccValidationSummaryV1, DCC_SANDBOX_PROJECTION_FAMILY_ID,
};
use super::denial::SandboxDenialRecordV1;
use super::policy::{CapabilityDecision, SandboxCapability, SandboxPolicyV1};
use super::run::SandboxRunV1;
use super::workspace::SandboxWorkspaceV1;

/// Inputs accepted by `reconstruct_projection`. Every field is a durable
/// record sourced from Postgres/EventLedger storage. The lack of any
/// `terminal_log` or `chat_transcript` field is intentional and load-bearing
/// for the MT-016 acceptance.
#[derive(Debug, Clone)]
pub struct ReplayInputsV1<'a> {
    pub run: &'a SandboxRunV1,
    pub policy: &'a SandboxPolicyV1,
    pub workspace: &'a SandboxWorkspaceV1,
    pub denial: Option<&'a SandboxDenialRecordV1>,
    pub validation: Option<&'a ReplayValidationFactsV1>,
    pub promotion: Option<&'a ReplayPromotionFactsV1>,
    pub artifact_refs: &'a [String],
}

/// Minimum facts needed to reconstruct the validation summary at replay time.
/// Populated by the validation-storage query in `kb003_storage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayValidationFactsV1 {
    pub validation_run_id: String,
    pub verdict: String,
    pub check_count: u32,
    pub failed_check_count: u32,
    pub report_artifact_ref: Option<String>,
}

/// Minimum facts needed to reconstruct the promotion summary at replay time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayPromotionFactsV1 {
    pub decision_id: String,
    pub decision: String,
    pub receipt_id: Option<String>,
    pub receipt_artifact_ref: Option<String>,
    pub rationale_short: String,
}

/// Deterministic projection reconstructor. Given only durable rows, returns a
/// `DccSandboxProjectionV1` that matches what the live runner would emit.
pub fn reconstruct_projection(inputs: ReplayInputsV1<'_>) -> DccSandboxProjectionV1 {
    let has_denial = inputs.denial.is_some();
    let has_promotion_accepted = matches!(
        inputs.promotion.as_ref().map(|p| p.decision.as_str()),
        Some("PROMOTED")
    );
    let outcome = DccSandboxOutcome::derive(inputs.run.status, has_denial, has_promotion_accepted);

    let capability_rows: Vec<DccCapabilityRowV1> = SandboxCapability::ALL
        .iter()
        .map(|cap| DccCapabilityRowV1 {
            capability: cap.as_str().to_string(),
            decision: inputs.policy.decide(*cap),
        })
        .collect();

    DccSandboxProjectionV1 {
        projection_family_id: DCC_SANDBOX_PROJECTION_FAMILY_ID.to_string(),
        run_id: inputs.run.run_id.0.clone(),
        kernel_task_run_id: inputs.run.kernel_task_run_id.clone(),
        session_run_id: inputs.run.session_run_id.clone(),
        adapter_kind: inputs.run.adapter_kind.clone(),
        policy_version_id: inputs.policy.version_id(),
        policy_default_decision: inputs.policy.default_decision,
        capability_rows,
        workspace_id: inputs.workspace.workspace_id.clone(),
        workspace_root_relative: inputs.workspace.root_relative_path.clone(),
        run_status: inputs.run.status,
        outcome,
        requested_at_utc: inputs.run.requested_at_utc.to_rfc3339(),
        started_at_utc: inputs.run.started_at_utc.map(|t| t.to_rfc3339()),
        finished_at_utc: inputs.run.finished_at_utc.map(|t| t.to_rfc3339()),
        denial: inputs.denial.map(|d| DccDenialSummaryV1 {
            denial_id: d.denial_id.clone(),
            kind: format!("{:?}", d.kind).to_ascii_uppercase(),
            capability: d.capability.map(|c| c.as_str().to_string()),
            action_description: d.action_description.clone(),
            reason: d.reason.clone(),
            policy_version_id: d.policy_version_id.clone(),
        }),
        validation: inputs.validation.map(|v| DccValidationSummaryV1 {
            validation_run_id: v.validation_run_id.clone(),
            verdict: v.verdict.clone(),
            check_count: v.check_count,
            failed_check_count: v.failed_check_count,
            report_artifact_ref: v.report_artifact_ref.clone(),
        }),
        promotion: inputs.promotion.map(|p| DccPromotionSummaryV1 {
            decision_id: p.decision_id.clone(),
            decision: p.decision.clone(),
            receipt_id: p.receipt_id.clone(),
            receipt_artifact_ref: p.receipt_artifact_ref.clone(),
            rationale_short: p.rationale_short.clone(),
        }),
        artifact_refs: inputs.artifact_refs.to_vec(),
        artifact_classes_in_view: Vec::new(),
        source_schema_ids: DccSandboxProjectionV1::canonical_source_schema_ids()
            .into_iter()
            .map(String::from)
            .collect(),
    }
}

/// Declarative contract: which kinds of inputs are FORBIDDEN at replay time.
/// Held as a constant so audits and the MT-016 unit test can assert by name.
pub const FORBIDDEN_REPLAY_INPUT_KINDS: &[&str] = &[
    "provider_chat_transcript",
    "terminal_scrollback",
    "transient_runtime_log",
    "in_memory_session_state",
];

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::kernel::sandbox::denial::DenialKind;
    use crate::kernel::sandbox::run::{SandboxRunId, SandboxRunStatus};

    #[test]
    fn reconstructor_uses_only_durable_inputs() {
        let run = SandboxRunV1 {
            run_id: SandboxRunId("SBX-r1".into()),
            kernel_task_run_id: "KTR-1".into(),
            session_run_id: "SES-1".into(),
            adapter_kind: "policy_scoped_local".into(),
            policy_version_id: "POL-1@1".into(),
            workspace_id: "WSP-1".into(),
            status: SandboxRunStatus::Rejected,
            requested_at_utc: Utc::now(),
            started_at_utc: None,
            finished_at_utc: None,
            denial_id: Some("DEN-1".into()),
            artifact_refs: vec!["ART-log-1".into()],
        };
        let policy = SandboxPolicyV1::default_deny("baseline");
        let workspace = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/work/r1");
        let denial = SandboxDenialRecordV1::new(
            "SBX-r1",
            policy.version_id(),
            DenialKind::PolicyDenied,
            Some(SandboxCapability::Network),
            "fetch x",
            "default_deny NETWORK",
        );
        let arts = vec!["ART-log-1".to_string()];
        let inputs = ReplayInputsV1 {
            run: &run,
            policy: &policy,
            workspace: &workspace,
            denial: Some(&denial),
            validation: None,
            promotion: None,
            artifact_refs: &arts,
        };
        let projection = reconstruct_projection(inputs);
        assert_eq!(projection.outcome, DccSandboxOutcome::DeniedByPolicy);
        assert!(projection.denial.is_some());
        assert_eq!(projection.capability_rows.len(), SandboxCapability::ALL.len());
        assert!(
            projection.is_self_describing(),
            "replay must yield a self-describing projection (MT-010 contract)"
        );
    }

    #[test]
    fn forbidden_input_kinds_documented() {
        assert!(FORBIDDEN_REPLAY_INPUT_KINDS.contains(&"terminal_scrollback"));
        assert!(FORBIDDEN_REPLAY_INPUT_KINDS.contains(&"provider_chat_transcript"));
    }

    #[test]
    fn capability_rows_default_to_deny() {
        let run = sample_run(SandboxRunStatus::Started);
        let policy = SandboxPolicyV1::default_deny("baseline");
        let workspace = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/w");
        let arts: Vec<String> = vec![];
        let inputs = ReplayInputsV1 {
            run: &run,
            policy: &policy,
            workspace: &workspace,
            denial: None,
            validation: None,
            promotion: None,
            artifact_refs: &arts,
        };
        let projection = reconstruct_projection(inputs);
        for row in &projection.capability_rows {
            assert_eq!(row.decision, CapabilityDecision::Deny);
        }
    }

    fn sample_run(status: SandboxRunStatus) -> SandboxRunV1 {
        SandboxRunV1 {
            run_id: SandboxRunId("SBX-x".into()),
            kernel_task_run_id: "KTR-x".into(),
            session_run_id: "SES-x".into(),
            adapter_kind: "policy_scoped_local".into(),
            policy_version_id: "POL-x@1".into(),
            workspace_id: "WSP-x".into(),
            status,
            requested_at_utc: Utc::now(),
            started_at_utc: None,
            finished_at_utc: None,
            denial_id: None,
            artifact_refs: vec![],
        }
    }
}
