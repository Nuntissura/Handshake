use handshake_core::kernel::{
    action_catalog::kernel002_action_catalog,
    action_envelope::KernelActionDenialV1,
    direct_edit_guard::{
        guard_direct_edit_attempt, validate_write_box_direct_edit_denied, DirectEditAttemptV1,
        DirectEditDecisionStatus, DirectEditTargetClass,
    },
};

fn authority_attempt() -> DirectEditAttemptV1 {
    DirectEditAttemptV1 {
        attempt_id: "attempt-001".to_string(),
        actor_id: "actor-model-1".to_string(),
        actor_kind: "model".to_string(),
        role_id: "CODER".to_string(),
        target_path: ".GOV/task_packets/WP-KERNEL-002/packet.json".to_string(),
        target_class: DirectEditTargetClass::AuthorityArtifact,
        operation: "raw_patch".to_string(),
        trace_id: "trace-direct-edit".to_string(),
    }
}

#[test]
fn raw_authority_edit_attempt_is_denied_with_actionable_replacements() {
    let catalog = kernel002_action_catalog();
    let decision = guard_direct_edit_attempt(&authority_attempt(), &catalog);

    assert_eq!(decision.status, DirectEditDecisionStatus::Denied);
    let denial: &KernelActionDenialV1 = decision.denial.as_ref().expect("denial evidence");
    assert_eq!(denial.denial_code, "direct_authority_edit_denied");
    assert_eq!(denial.request_trace_id, "trace-direct-edit");
    assert!(denial.reason.contains("registered write-box action"));
    assert!(denial
        .lawful_replacement_action_ids
        .contains(&"kernel.mirror_advisory.capture".to_string()));
    assert!(denial
        .lawful_replacement_action_ids
        .contains(&"kernel.crdt_workspace.propose_patch".to_string()));
    assert!(!denial.evidence_refs.is_empty());
    assert_eq!(denial.receipt_mappings[0].receipt_kind, "DENIAL");
    assert_eq!(
        denial.event_mappings[0].event_kind,
        "KernelDirectEditDeniedV1"
    );

    let write_box_evidence = decision
        .write_box_direct_edit_denied
        .as_ref()
        .expect("durable WriteBoxDirectEditDeniedV1 evidence");
    validate_write_box_direct_edit_denied(write_box_evidence)
        .expect("direct-edit denial evidence must validate");
    assert_eq!(write_box_evidence.actor.actor_id, "actor-model-1");
    assert_eq!(write_box_evidence.actor.role_id, "CODER");
    assert_eq!(
        write_box_evidence.target.target_ref,
        ".GOV/task_packets/WP-KERNEL-002/packet.json"
    );
    assert_eq!(write_box_evidence.attempted_action, "raw_patch");
    assert_eq!(
        write_box_evidence.recovery_instruction,
        "Use kernel.crdt_workspace.propose_patch or kernel.mirror_advisory.capture through the Kernel Action Catalog"
    );
    assert_eq!(
        write_box_evidence.ui_response_ref,
        "dcc://direct-edit-denials/attempt-001"
    );
    assert_eq!(
        write_box_evidence.api_response_ref,
        "api://kernel/direct-edit-denials/attempt-001"
    );
}

#[test]
fn direct_edit_denial_validation_requires_role_attribution() {
    let catalog = kernel002_action_catalog();
    let decision = guard_direct_edit_attempt(&authority_attempt(), &catalog);
    let mut evidence = decision
        .write_box_direct_edit_denied
        .expect("durable WriteBoxDirectEditDeniedV1 evidence");

    evidence.actor.role_id.clear();
    let errors = validate_write_box_direct_edit_denied(&evidence)
        .expect_err("direct-edit denial evidence must retain role attribution");

    assert!(errors.iter().any(|error| error.field == "actor.role_id"));
}

#[test]
fn generated_mirror_and_crdt_attempts_are_wrapped_into_lawful_actions() {
    let catalog = kernel002_action_catalog();

    let mirror_attempt = DirectEditAttemptV1 {
        target_class: DirectEditTargetClass::GeneratedMirror,
        target_path: "docs/generated/task-board.md".to_string(),
        ..authority_attempt()
    };
    let mirror_decision = guard_direct_edit_attempt(&mirror_attempt, &catalog);
    assert_eq!(mirror_decision.status, DirectEditDecisionStatus::Wrapped);
    assert_eq!(
        mirror_decision.lawful_action_id.as_deref(),
        Some("kernel.mirror_advisory.capture")
    );
    assert!(mirror_decision.denial.is_none());

    let workspace_attempt = DirectEditAttemptV1 {
        target_class: DirectEditTargetClass::CrdtWorkspace,
        target_path: "workspace/doc-1".to_string(),
        ..authority_attempt()
    };
    let workspace_decision = guard_direct_edit_attempt(&workspace_attempt, &catalog);
    assert_eq!(workspace_decision.status, DirectEditDecisionStatus::Wrapped);
    assert_eq!(
        workspace_decision.lawful_action_id.as_deref(),
        Some("kernel.crdt_workspace.propose_patch")
    );
}

#[test]
fn replacement_actions_must_exist_in_catalog() {
    let catalog = kernel002_action_catalog();
    let decision = guard_direct_edit_attempt(&authority_attempt(), &catalog);
    let denial = decision.denial.expect("denial evidence");

    for action_id in denial.lawful_replacement_action_ids {
        assert!(
            catalog.action(&action_id).is_some(),
            "replacement action must exist in catalog: {action_id}"
        );
    }
}
