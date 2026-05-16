use serde::{Deserialize, Serialize};

use super::action_catalog::KernelActionCatalogV1;
use super::action_envelope::{
    EventLedgerMapping, KernelActionDenialV1, KernelActorRef, KernelReceiptMapping,
};

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
    pub role_id: String,
    pub target_path: String,
    pub target_class: DirectEditTargetClass,
    pub operation: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectEditDecisionV1 {
    pub status: DirectEditDecisionStatus,
    pub denial: Option<KernelActionDenialV1>,
    pub write_box_direct_edit_denied: Option<WriteBoxDirectEditDeniedV1>,
    pub lawful_action_id: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxDirectEditTargetV1 {
    pub target_ref: String,
    pub target_class: DirectEditTargetClass,
    pub authority_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteBoxDirectEditDeniedV1 {
    pub schema_id: String,
    pub evidence_id: String,
    pub actor: KernelActorRef,
    pub target: WriteBoxDirectEditTargetV1,
    pub attempted_action: String,
    pub denial_reason: String,
    pub recovery_instruction: String,
    pub ui_response_ref: String,
    pub api_response_ref: String,
    pub receipt_refs: Vec<String>,
    pub event_ledger_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteBoxDirectEditDeniedValidationError {
    pub field: &'static str,
    pub message: &'static str,
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
            write_box_direct_edit_denied: None,
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
    let (denial, write_box_direct_edit_denied) = denial_evidence(
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
        write_box_direct_edit_denied: Some(write_box_direct_edit_denied),
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
        let (denial, write_box_direct_edit_denied) = denial_evidence(
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
            write_box_direct_edit_denied: Some(write_box_direct_edit_denied),
            lawful_action_id: Some("kernel.direct_edit.deny".to_string()),
            evidence_refs,
        };
    };

    DirectEditDecisionV1 {
        status: DirectEditDecisionStatus::Wrapped,
        denial: None,
        write_box_direct_edit_denied: None,
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
        write_box_direct_edit_denied: None,
        lawful_action_id: Some(action_id.to_string()),
        evidence_refs: vec![evidence_ref(attempt, "dcc_quick_action_wrapped")],
    }
}

fn deny_dcc_quick_action(
    attempt: &DirectEditAttemptV1,
    catalog: &KernelActionCatalogV1,
) -> DirectEditDecisionV1 {
    let evidence_refs = vec![evidence_ref(attempt, "dcc_quick_action_unregistered")];
    let (denial, write_box_direct_edit_denied) = denial_evidence(
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
        write_box_direct_edit_denied: Some(write_box_direct_edit_denied),
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
) -> (KernelActionDenialV1, WriteBoxDirectEditDeniedV1) {
    let denial_id = format!("denial-{}", attempt.attempt_id);
    let receipt_refs = vec![format!(
        "receipt://direct-edit-denial/{}",
        attempt.attempt_id
    )];
    let event_ledger_refs = vec![format!(
        "eventledger://direct-edit-denied/{}",
        attempt.trace_id
    )];
    let recovery_instruction =
        "Use kernel.crdt_workspace.propose_patch or kernel.mirror_advisory.capture through the Kernel Action Catalog"
            .to_string();
    let denial = KernelActionDenialV1 {
        schema_id: "hsk.kernel_action_denial@1".to_string(),
        denial_id: denial_id.clone(),
        request_trace_id: attempt.trace_id.clone(),
        denial_code: denial_code.to_string(),
        reason: reason.clone(),
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
    };
    let write_box_evidence = WriteBoxDirectEditDeniedV1 {
        schema_id: "hsk.write_box_direct_edit_denied@1".to_string(),
        evidence_id: denial_id,
        actor: KernelActorRef {
            actor_id: attempt.actor_id.clone(),
            actor_kind: attempt.actor_kind.clone(),
            role_id: attempt.role_id.clone(),
        },
        target: WriteBoxDirectEditTargetV1 {
            target_ref: attempt.target_path.clone(),
            target_class: attempt.target_class,
            authority_class: direct_edit_authority_class(attempt.target_class).to_string(),
        },
        attempted_action: attempt.operation.clone(),
        denial_reason: reason,
        recovery_instruction,
        ui_response_ref: format!("dcc://direct-edit-denials/{}", attempt.attempt_id),
        api_response_ref: format!("api://kernel/direct-edit-denials/{}", attempt.attempt_id),
        receipt_refs,
        event_ledger_refs,
    };
    (denial, write_box_evidence)
}

fn evidence_ref(attempt: &DirectEditAttemptV1, evidence_kind: &str) -> String {
    format!(
        "hsk.direct_edit_evidence:{}:{}:{}",
        evidence_kind, attempt.attempt_id, attempt.trace_id
    )
}

fn direct_edit_authority_class(target_class: DirectEditTargetClass) -> &'static str {
    match target_class {
        DirectEditTargetClass::AuthorityArtifact => "authority_artifact",
        DirectEditTargetClass::GeneratedFile => "generated_file",
        DirectEditTargetClass::GeneratedMirror => "generated_mirror",
        DirectEditTargetClass::CrdtWorkspace => "crdt_workspace",
        DirectEditTargetClass::RoleMailboxReply => "role_mailbox_reply",
        DirectEditTargetClass::DccQuickAction => "dcc_quick_action",
        DirectEditTargetClass::GitAction => "git_action",
        DirectEditTargetClass::ProductCode => "product_code",
    }
}

pub fn validate_write_box_direct_edit_denied(
    evidence: &WriteBoxDirectEditDeniedV1,
) -> Result<(), Vec<WriteBoxDirectEditDeniedValidationError>> {
    let mut errors = Vec::new();
    require_non_empty(&mut errors, "schema_id", &evidence.schema_id);
    require_non_empty(&mut errors, "evidence_id", &evidence.evidence_id);
    require_non_empty(&mut errors, "actor.actor_id", &evidence.actor.actor_id);
    require_non_empty(&mut errors, "actor.actor_kind", &evidence.actor.actor_kind);
    require_non_empty(&mut errors, "actor.role_id", &evidence.actor.role_id);
    require_non_empty(
        &mut errors,
        "target.target_ref",
        &evidence.target.target_ref,
    );
    require_non_empty(
        &mut errors,
        "target.authority_class",
        &evidence.target.authority_class,
    );
    require_non_empty(&mut errors, "attempted_action", &evidence.attempted_action);
    require_non_empty(&mut errors, "denial_reason", &evidence.denial_reason);
    require_non_empty(
        &mut errors,
        "recovery_instruction",
        &evidence.recovery_instruction,
    );
    require_non_empty(&mut errors, "ui_response_ref", &evidence.ui_response_ref);
    require_non_empty(&mut errors, "api_response_ref", &evidence.api_response_ref);
    require_vec(&mut errors, "receipt_refs", &evidence.receipt_refs);
    require_vec(
        &mut errors,
        "event_ledger_refs",
        &evidence.event_ledger_refs,
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn require_non_empty(
    errors: &mut Vec<WriteBoxDirectEditDeniedValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(WriteBoxDirectEditDeniedValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<WriteBoxDirectEditDeniedValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(WriteBoxDirectEditDeniedValidationError {
            field,
            message: "at least one value is required",
        });
    }
}
