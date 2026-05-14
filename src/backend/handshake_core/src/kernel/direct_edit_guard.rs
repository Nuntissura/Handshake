use serde::{Deserialize, Serialize};

use super::action_catalog::KernelActionCatalogV1;
use super::action_envelope::{EventLedgerMapping, KernelActionDenialV1, KernelReceiptMapping};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectEditTargetClass {
    AuthorityArtifact,
    GeneratedFile,
    GeneratedMirror,
    CrdtWorkspace,
    RoleMailboxReply,
    DccQuickAction,
    GitAction,
    ProductCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectEditDecisionStatus {
    Denied,
    Wrapped,
    Allowed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEditAttemptV1 {
    pub attempt_id: String,
    pub actor_id: String,
    pub actor_kind: String,
    pub target_path: String,
    pub target_class: DirectEditTargetClass,
    pub operation: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEditDecisionV1 {
    pub status: DirectEditDecisionStatus,
    pub denial: Option<KernelActionDenialV1>,
    pub lawful_action_id: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEditRegressionCaseV1 {
    pub case_id: String,
    pub attempt: DirectEditAttemptV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEditRegressionCaseResultV1 {
    pub case_id: String,
    pub status: DirectEditDecisionStatus,
    pub lawful_action_id: Option<String>,
    pub denial_code: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEditRegressionHarnessResultV1 {
    pub schema_id: &'static str,
    pub harness_id: String,
    pub case_results: Vec<DirectEditRegressionCaseResultV1>,
    pub all_paths_guarded: bool,
    pub unguarded_case_ids: Vec<String>,
}

pub fn guard_direct_edit_attempt(
    attempt: &DirectEditAttemptV1,
    catalog: &KernelActionCatalogV1,
) -> DirectEditDecisionV1 {
    match attempt.target_class {
        DirectEditTargetClass::AuthorityArtifact => deny_authority_edit(attempt, catalog),
        DirectEditTargetClass::GeneratedFile => wrap_with_action(
            attempt,
            catalog,
            "kernel.mirror_advisory.capture",
            "generated_file_write_wrapped",
        ),
        DirectEditTargetClass::GeneratedMirror => wrap_with_action(
            attempt,
            catalog,
            "kernel.mirror_advisory.capture",
            "generated_mirror_edit_wrapped",
        ),
        DirectEditTargetClass::CrdtWorkspace => wrap_with_action(
            attempt,
            catalog,
            "kernel.crdt_workspace.propose_patch",
            "crdt_workspace_edit_wrapped",
        ),
        DirectEditTargetClass::RoleMailboxReply => wrap_with_action(
            attempt,
            catalog,
            "kernel.role_mailbox_loop_control.project",
            "role_mailbox_reply_wrapped",
        ),
        DirectEditTargetClass::DccQuickAction => guard_dcc_quick_action(attempt, catalog),
        DirectEditTargetClass::GitAction => wrap_with_action(
            attempt,
            catalog,
            "kernel.git_engine_decision_gate.project",
            "git_action_wrapped",
        ),
        DirectEditTargetClass::ProductCode => DirectEditDecisionV1 {
            status: DirectEditDecisionStatus::Allowed,
            denial: None,
            lawful_action_id: None,
            evidence_refs: vec![evidence_ref(attempt, "product_code_edit_allowed")],
        },
    }
}

pub fn run_direct_edit_regression_harness(
    harness_id: &str,
    cases: &[DirectEditRegressionCaseV1],
    catalog: &KernelActionCatalogV1,
) -> DirectEditRegressionHarnessResultV1 {
    let case_results: Vec<DirectEditRegressionCaseResultV1> = cases
        .iter()
        .map(|case| {
            let decision = guard_direct_edit_attempt(&case.attempt, catalog);
            DirectEditRegressionCaseResultV1 {
                case_id: case.case_id.clone(),
                status: decision.status,
                lawful_action_id: decision.lawful_action_id,
                denial_code: decision
                    .denial
                    .as_ref()
                    .map(|denial| denial.denial_code.clone()),
                evidence_refs: decision.evidence_refs,
            }
        })
        .collect();

    let unguarded_case_ids: Vec<String> = case_results
        .iter()
        .filter(|case| {
            case.evidence_refs.is_empty()
                || matches!(case.status, DirectEditDecisionStatus::Allowed)
                || (matches!(case.status, DirectEditDecisionStatus::Wrapped)
                    && case.lawful_action_id.is_none())
        })
        .map(|case| case.case_id.clone())
        .collect();

    DirectEditRegressionHarnessResultV1 {
        schema_id: "hsk.kernel.direct_edit_regression_harness_result@1",
        harness_id: harness_id.to_string(),
        case_results,
        all_paths_guarded: unguarded_case_ids.is_empty(),
        unguarded_case_ids,
    }
}

fn deny_authority_edit(
    attempt: &DirectEditAttemptV1,
    catalog: &KernelActionCatalogV1,
) -> DirectEditDecisionV1 {
    let replacements = replacement_actions(catalog);
    let evidence_refs = vec![evidence_ref(attempt, "raw_authority_edit_attempt")];
    let denial = denial_evidence(
        attempt,
        "direct_authority_edit_denied",
        format!(
            "Direct mutation of authority artifact '{}' is denied; use a registered write-box action.",
            attempt.target_path
        ),
        replacements,
        &evidence_refs,
    );

    DirectEditDecisionV1 {
        status: DirectEditDecisionStatus::Denied,
        denial: Some(denial),
        lawful_action_id: Some("kernel.direct_edit.deny".to_string()),
        evidence_refs,
    }
}

fn wrap_with_action(
    attempt: &DirectEditAttemptV1,
    catalog: &KernelActionCatalogV1,
    action_id: &str,
    evidence_kind: &str,
) -> DirectEditDecisionV1 {
    let evidence_refs = vec![evidence_ref(attempt, evidence_kind)];
    let Some(action) = catalog.action(action_id) else {
        let denial = denial_evidence(
            attempt,
            "registered_action_missing",
            format!(
                "Direct edit route '{}' is denied because registered action '{}' is unavailable.",
                attempt.operation, action_id
            ),
            replacement_actions(catalog),
            &evidence_refs,
        );
        return DirectEditDecisionV1 {
            status: DirectEditDecisionStatus::Denied,
            denial: Some(denial),
            lawful_action_id: Some("kernel.direct_edit.deny".to_string()),
            evidence_refs,
        };
    };

    DirectEditDecisionV1 {
        status: DirectEditDecisionStatus::Wrapped,
        denial: None,
        lawful_action_id: Some(action.action_id.to_string()),
        evidence_refs,
    }
}

fn guard_dcc_quick_action(
    attempt: &DirectEditAttemptV1,
    catalog: &KernelActionCatalogV1,
) -> DirectEditDecisionV1 {
    let Some(action_id) = attempt.operation.strip_prefix("registered_action:") else {
        return deny_dcc_quick_action(attempt, catalog);
    };
    if catalog.action(action_id).is_none() {
        return deny_dcc_quick_action(attempt, catalog);
    }

    DirectEditDecisionV1 {
        status: DirectEditDecisionStatus::Wrapped,
        denial: None,
        lawful_action_id: Some(action_id.to_string()),
        evidence_refs: vec![evidence_ref(attempt, "dcc_quick_action_wrapped")],
    }
}

fn deny_dcc_quick_action(
    attempt: &DirectEditAttemptV1,
    catalog: &KernelActionCatalogV1,
) -> DirectEditDecisionV1 {
    let evidence_refs = vec![evidence_ref(attempt, "dcc_quick_action_unregistered")];
    let denial = denial_evidence(
        attempt,
        "dcc_quick_action_unregistered",
        format!(
            "DCC quick action '{}' is denied because it does not resolve to a registered kernel action.",
            attempt.operation
        ),
        replacement_actions(catalog),
        &evidence_refs,
    );

    DirectEditDecisionV1 {
        status: DirectEditDecisionStatus::Denied,
        denial: Some(denial),
        lawful_action_id: Some("kernel.direct_edit.deny".to_string()),
        evidence_refs,
    }
}

fn replacement_actions(catalog: &KernelActionCatalogV1) -> Vec<String> {
    [
        "kernel.mirror_advisory.capture",
        "kernel.crdt_workspace.propose_patch",
        "kernel.role_mailbox_loop_control.project",
        "kernel.git_engine_decision_gate.project",
    ]
    .into_iter()
    .filter(|action_id| catalog.action(action_id).is_some())
    .map(str::to_string)
    .collect()
}

fn denial_evidence(
    attempt: &DirectEditAttemptV1,
    denial_code: &str,
    reason: String,
    lawful_replacement_action_ids: Vec<String>,
    evidence_refs: &[String],
) -> KernelActionDenialV1 {
    KernelActionDenialV1 {
        schema_id: "hsk.kernel_action_denial@1".to_string(),
        denial_id: format!("denial-{}", attempt.attempt_id),
        request_trace_id: attempt.trace_id.clone(),
        denial_code: denial_code.to_string(),
        reason,
        lawful_replacement_action_ids,
        evidence_refs: evidence_refs.to_vec(),
        receipt_mappings: vec![KernelReceiptMapping {
            receipt_kind: "DENIAL".to_string(),
            receipt_schema_id: "hsk.wp_receipt@1".to_string(),
            correlation_id: attempt.trace_id.clone(),
        }],
        event_mappings: vec![EventLedgerMapping {
            event_kind: "KernelDirectEditDeniedV1".to_string(),
            event_schema_id: "hsk.event.kernel_direct_edit_denied@1".to_string(),
            idempotency_key: attempt.trace_id.clone(),
        }],
    }
}

fn evidence_ref(attempt: &DirectEditAttemptV1, evidence_kind: &str) -> String {
    format!(
        "hsk.direct_edit_evidence:{}:{}:{}",
        evidence_kind, attempt.attempt_id, attempt.trace_id
    )
}
