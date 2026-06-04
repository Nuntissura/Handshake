use std::collections::HashSet;

use handshake_core::kernel::{
    action_catalog::{
        kernel002_action_catalog, validate_kernel_action_catalog, KernelActionCatalogError,
    },
    action_envelope::{ApprovalPosture, AuthorityEffect},
};

#[test]
fn kernel_action_catalog_has_required_model_facing_actions() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    for action_id in [
        "kernel.action_catalog.view",
        "kernel.crdt_workspace.propose_patch",
        "kernel.write_box.promote",
        "kernel.mirror_advisory.capture",
        "kernel.direct_edit.deny",
    ] {
        assert!(
            catalog.action(action_id).is_some(),
            "missing model-facing action: {action_id}"
        );
    }
}

#[test]
fn editable_surface_actions_are_registered_with_expected_write_boxes() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let model_manual = catalog
        .action("kernel.model_manual.update_section")
        .expect("model manual update action must be registered");
    assert_eq!(
        model_manual.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        model_manual.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert!(
        model_manual.expected_write_boxes.iter().any(|write_box| {
            write_box.write_box_schema_id == "hsk.write_box.model_manual_section@1"
                && write_box.target_id == "manual_section"
        }),
        "model manual action must declare the ModelManual write-box schema"
    );

    let policy = catalog
        .action("kernel.memory_capsule.policy_table_update")
        .expect("memory capsule policy table update action must be registered");
    assert_eq!(
        policy.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        policy.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert!(
        policy.expected_write_boxes.iter().any(|write_box| {
            write_box.write_box_schema_id == "hsk.write_box.memory_capsule_policy_update@1"
                && write_box.target_id == "memory_capsule_policy"
        }),
        "retrieval policy action must declare the MemoryCapsule policy write-box schema"
    );
}

#[test]
fn every_catalog_action_has_stable_contract_and_dcc_metadata() {
    let catalog = kernel002_action_catalog();
    let mut seen = HashSet::new();

    for action in catalog.actions {
        assert!(seen.insert(action.action_id), "duplicate action id");
        assert!(action.action_id.starts_with("kernel."));
        assert!(action.input_schema_id.starts_with("hsk."));
        assert!(action.result_schema_id.starts_with("hsk."));
        assert!(!action.role_eligibility.is_empty());
        assert!(!action.capability_requirements.is_empty());
        assert!(!action.expected_write_boxes.is_empty());
        assert!(!action.validation_hooks.is_empty());
        assert!(!action.dcc_preview.panel_id.is_empty());
        assert!(!action.dcc_preview.summary.is_empty());
        assert!(!action.dcc_preview.primary_state_fields.is_empty());
    }
}

#[test]
fn promotion_and_denial_actions_encode_authority_boundaries() {
    let catalog = kernel002_action_catalog();

    let promote = catalog
        .action("kernel.write_box.promote")
        .expect("promotion action");
    assert_eq!(
        promote.authority_effect,
        AuthorityEffect::EventLedgerAuthorityWrite
    );
    assert_eq!(
        promote.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert!(promote.promotion_path.event_kind.contains("Promotion"));
    assert!(promote
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "promotion_gate"));

    let denial = catalog
        .action("kernel.direct_edit.deny")
        .expect("direct edit denial action");
    assert_eq!(denial.authority_effect, AuthorityEffect::None);
    assert_eq!(denial.approval_posture, ApprovalPosture::Denied);
    assert!(denial
        .promotion_path
        .lawful_replacement_action_ids
        .contains(&"kernel.mirror_advisory.capture"));
    assert!(denial
        .promotion_path
        .lawful_replacement_action_ids
        .contains(&"kernel.crdt_workspace.propose_patch"));
    assert!(
        denial
            .expected_write_boxes
            .iter()
            .any(|write_box| write_box.write_box_schema_id == "hsk.write_box_direct_edit_denied@1"),
        "direct-edit denial action must declare the concrete denial evidence schema"
    );
}

#[test]
fn action_catalog_does_not_expose_sqlite_authority_tokens() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    for action in catalog.actions {
        let mut exposed_values = vec![
            action.action_id.to_string(),
            action.title,
            action.input_schema_id,
            action.result_schema_id,
            action.promotion_path.path_id,
            action.dcc_preview.panel_id,
            action.dcc_preview.summary,
        ];

        exposed_values.extend(action.validation_hooks.into_iter().map(|hook| hook.hook_id));

        for value in exposed_values {
            let normalized = value.to_ascii_lowercase();
            assert!(
                !normalized.contains("sqlite"),
                "catalog action {} exposes SQLite token in value: {value}",
                action.action_id
            );
        }
    }
}

#[test]
fn catalog_validation_rejects_duplicate_or_incomplete_actions() {
    let catalog = kernel002_action_catalog();
    assert!(validate_kernel_action_catalog(&catalog).is_ok());

    let mut duplicate = catalog.clone();
    duplicate.actions = vec![catalog.actions[0].clone(), catalog.actions[0].clone()];
    let errors = validate_kernel_action_catalog(&duplicate)
        .expect_err("duplicate action ids must fail catalog validation");
    assert!(errors
        .iter()
        .any(|error| matches!(error, KernelActionCatalogError::DuplicateActionId { .. })));
}
