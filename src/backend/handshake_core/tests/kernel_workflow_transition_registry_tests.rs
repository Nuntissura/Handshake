use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    workflow_transition_registry::{
        kernel002_workflow_transition_registry, preview_workflow_transition,
        validate_workflow_transition_registry, ApprovalBoundary, DccTransitionPreviewPosture,
        QueueAutomationMode, QueueAutomationRuleV1, QueueAutomationSourceKind,
        QueueAutomationTriggerKind, WorkflowMutationKind,
        WorkflowTransitionRegistryValidationError,
    },
};

#[test]
fn workflow_transition_registry_folds_legacy_stub_into_explicit_transition_law() {
    let registry = kernel002_workflow_transition_registry();
    validate_workflow_transition_registry(&registry).expect("registry must validate");

    let transition = registry
        .transition_rule("kernel.mt.claim")
        .expect("claim transition rule");
    assert_eq!(transition.from_state_id, "MT_PENDING");
    assert_eq!(transition.to_state_id, "MT_CLAIMED");
    assert_eq!(transition.governed_action_id, "kernel.microtask.claim");
    assert!(transition
        .eligible_actor_kinds
        .contains(&"KERNEL_BUILDER".to_string()));
    assert!(transition
        .folded_source_refs
        .iter()
        .any(|source| source.contains("WP-1-Workflow-Transition-Automation-Registry-v1")));

    assert!(registry
        .queue_automation_rules
        .iter()
        .any(|rule| rule.trigger_kind == QueueAutomationTriggerKind::DependencyCleared));
    assert!(registry
        .executor_policies
        .iter()
        .any(|policy| policy.policy_id == "kernel.executor.local_small_model"));

    for mutation_kind in [
        WorkflowMutationKind::WorkPacket,
        WorkflowMutationKind::MicroTask,
        WorkflowMutationKind::TaskBoardProjection,
        WorkflowMutationKind::RoleMailboxQueue,
        WorkflowMutationKind::DevCommandCenterAction,
    ] {
        assert!(registry
            .transition_rules
            .iter()
            .any(|rule| rule.mutation_kind == mutation_kind));
    }
}

#[test]
fn queue_automation_validation_rejects_silent_approval_boundary_crossing_and_prose_sources() {
    let mut registry = kernel002_workflow_transition_registry();
    registry.queue_automation_rules.push(QueueAutomationRuleV1 {
        rule_id: "bad-auto-approval-crossing".to_string(),
        trigger_kind: QueueAutomationTriggerKind::ApprovalDecision,
        trigger_source_kind: QueueAutomationSourceKind::MailboxChronology,
        transition_rule_id: "kernel.mt.validator_verdict".to_string(),
        mode: QueueAutomationMode::Automatic,
        stable_source_ids: vec!["mailbox-thread-order".to_string()],
        dcc_preview_id: "dcc.bad-auto-approval-crossing".to_string(),
    });

    let errors = validate_workflow_transition_registry(&registry)
        .expect_err("bad automation must fail validation");

    assert!(errors.iter().any(|error| matches!(
        error,
        WorkflowTransitionRegistryValidationError::AutomationCrossesApprovalBoundary { .. }
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        WorkflowTransitionRegistryValidationError::NonAuthoritativeAutomationSource { .. }
    )));
}

#[test]
fn dcc_preview_reports_actor_ineligible_approval_required_and_lawful_postures() {
    let registry = kernel002_workflow_transition_registry();

    let ineligible = preview_workflow_transition(&registry, "kernel.mt.claim", "OBSERVER")
        .expect("preview for ineligible actor");
    assert_eq!(
        ineligible.posture,
        DccTransitionPreviewPosture::ActorIneligible
    );
    assert_eq!(ineligible.authority_effect, AuthorityEffect::ProjectionOnly);

    let approval = preview_workflow_transition(
        &registry,
        "kernel.mt.validator_verdict",
        "INTEGRATION_VALIDATOR",
    )
    .expect("preview for approval-bound transition");
    assert_eq!(
        approval.posture,
        DccTransitionPreviewPosture::ApprovalRequired
    );
    assert_eq!(
        approval.approval_boundary,
        ApprovalBoundary::ValidatorApproval
    );

    let lawful = preview_workflow_transition(&registry, "kernel.mt.claim", "KERNEL_BUILDER")
        .expect("preview for lawful transition");
    assert_eq!(lawful.posture, DccTransitionPreviewPosture::Lawful);
}

#[test]
fn kernel_action_catalog_exposes_workflow_transition_preview_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.workflow_transition.preview")
        .expect("workflow transition preview action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "transition_rule_registered"));
    assert!(action
        .dcc_preview
        .primary_state_fields
        .contains(&"approval_boundary".to_string()));
}

#[test]
fn registry_requires_every_mutation_to_have_rule_actor_action_boundary_and_preview() {
    let mut registry = kernel002_workflow_transition_registry();
    registry.transition_rules[0].eligible_actor_kinds.clear();
    registry.transition_rules[0].governed_action_id.clear();
    registry.transition_rules[0].dcc_preview.panel_id.clear();

    let errors = validate_workflow_transition_registry(&registry)
        .expect_err("incomplete transition rule must fail");

    assert!(errors.iter().any(|error| matches!(
        error,
        WorkflowTransitionRegistryValidationError::MissingTransitionField { field, .. }
            if *field == "eligible_actor_kinds"
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        WorkflowTransitionRegistryValidationError::MissingTransitionField { field, .. }
            if *field == "governed_action_id"
    )));
    assert!(errors.iter().any(|error| matches!(
        error,
        WorkflowTransitionRegistryValidationError::MissingTransitionField { field, .. }
            if *field == "dcc_preview.panel_id"
    )));

    let mut registry = kernel002_workflow_transition_registry();
    registry
        .transition_rules
        .retain(|rule| rule.mutation_kind != WorkflowMutationKind::DevCommandCenterAction);
    let errors = validate_workflow_transition_registry(&registry)
        .expect_err("every declared mutation kind must have a registered rule");
    assert!(errors.iter().any(|error| matches!(
        error,
        WorkflowTransitionRegistryValidationError::MissingMutationKindRule {
            mutation_kind: WorkflowMutationKind::DevCommandCenterAction
        }
    )));
}
