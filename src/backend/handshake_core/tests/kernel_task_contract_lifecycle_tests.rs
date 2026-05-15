use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    task_contract_lifecycle::{
        build_kernel002_task_contract_lifecycle, validate_task_contract_lifecycle,
        ContractAuthorityRule, ContractFailureState, ContractLifecycleState, ProvenanceHashV1,
    },
};

#[test]
fn task_contract_lifecycle_defines_stub_wp_and_microtask_contract_schemas() {
    let lifecycle = build_kernel002_task_contract_lifecycle();

    validate_task_contract_lifecycle(&lifecycle).expect("task contract lifecycle validates");

    assert_eq!(lifecycle.schema_id, "hsk.kernel.task_contract_lifecycle@1");
    assert_eq!(
        lifecycle.stub_contract.schema_id,
        "hsk.work_packet_stub_contract@1"
    );
    assert_eq!(
        lifecycle.work_packet_contract.schema_id,
        "hsk.work_packet_contract@1"
    );
    assert_eq!(
        lifecycle.micro_task_contracts[0].schema_id,
        "hsk.microtask_contract@1"
    );
    assert!(lifecycle
        .stub_contract
        .authority_rules
        .contains(&ContractAuthorityRule::MachineContractAuthority));
    assert!(lifecycle
        .stub_contract
        .required_fields
        .contains(&"activation_contract".to_string()));
    assert!(lifecycle
        .work_packet_contract
        .required_fields
        .contains(&"microtasks".to_string()));
    assert!(lifecycle.micro_task_contracts[0]
        .required_fields
        .contains(&"handoff".to_string()));
}

#[test]
fn task_contract_lifecycle_preserves_transitions_provenance_hooks_and_failure_states() {
    let lifecycle = build_kernel002_task_contract_lifecycle();

    validate_task_contract_lifecycle(&lifecycle).expect("task contract lifecycle validates");

    assert!(lifecycle.transitions.iter().any(|transition| {
        transition.from == ContractLifecycleState::StubReadyForPromotion
            && transition.to == ContractLifecycleState::WorkPacketActive
            && transition.action_id == "kernel.work_packet_contract.activate"
            && transition.validation_hook_id == "source_hash_match"
    }));
    assert!(lifecycle.transitions.iter().any(|transition| {
        transition.from == ContractLifecycleState::WorkPacketActive
            && transition.to == ContractLifecycleState::MicroTaskGenerated
            && transition.action_id == "kernel.microtask_contract.extract"
    }));
    assert!(lifecycle
        .receipt_events
        .contains(&"REVIEW_REQUEST".to_string()));
    assert!(lifecycle
        .receipt_events
        .contains(&"CODER_HANDOFF".to_string()));
    assert!(lifecycle
        .projection_hooks
        .contains(&"task_board_projection".to_string()));
    assert!(lifecycle
        .validation_hooks
        .contains(&"transition_allowed".to_string()));
    assert!(lifecycle
        .source_imports
        .iter()
        .all(|source| is_sha256_digest(&source.provenance_hash.hash_value)));
    assert!(lifecycle
        .stub_contract
        .provenance_hashes
        .iter()
        .all(|hash| is_sha256_digest(&hash.hash_value)));
    assert!(lifecycle
        .failure_states
        .contains(&ContractFailureState::SourceHashMismatch));
    assert!(lifecycle
        .failure_states
        .contains(&ContractFailureState::ProjectionDrift));
}

#[test]
fn task_contract_lifecycle_rejects_shadow_authority_and_incomplete_failure_paths() {
    let mut lifecycle = build_kernel002_task_contract_lifecycle();
    lifecycle
        .stub_contract
        .authority_rules
        .retain(|rule| *rule != ContractAuthorityRule::MachineContractAuthority);
    let errors = validate_task_contract_lifecycle(&lifecycle)
        .expect_err("stub contract must retain machine authority rule");
    assert!(errors
        .iter()
        .any(|error| error.field == "stub_contract.authority_rules"));

    let mut lifecycle = build_kernel002_task_contract_lifecycle();
    lifecycle.micro_task_contracts[0].failure_states.clear();
    let errors = validate_task_contract_lifecycle(&lifecycle)
        .expect_err("microtask contracts must define failure states");
    assert!(errors
        .iter()
        .any(|error| error.field == "micro_task_contracts.failure_states"));

    let mut lifecycle = build_kernel002_task_contract_lifecycle();
    lifecycle.transitions[0].receipt_event.clear();
    let errors = validate_task_contract_lifecycle(&lifecycle)
        .expect_err("lifecycle transitions must emit receipt events");
    assert!(errors
        .iter()
        .any(|error| error.field == "transitions.receipt_event"));
}

#[test]
fn task_contract_lifecycle_rejects_fake_provenance_digests() {
    let mut lifecycle = build_kernel002_task_contract_lifecycle();
    lifecycle.source_imports[0].provenance_hash = ProvenanceHashV1 {
        hash_kind: "sha256".to_string(),
        hash_value: "zzzzzzzzzzzzzzzz".to_string(),
    };
    let errors = validate_task_contract_lifecycle(&lifecycle)
        .expect_err("source imports must reject fake hashes");
    assert!(errors
        .iter()
        .any(|error| error.field == "source_imports.provenance_hash"));

    let mut lifecycle = build_kernel002_task_contract_lifecycle();
    lifecycle.stub_contract.provenance_hashes[0] = ProvenanceHashV1 {
        hash_kind: "sha256".to_string(),
        hash_value: "sha256:not-a-real-digest".to_string(),
    };
    let errors = validate_task_contract_lifecycle(&lifecycle)
        .expect_err("contract schema provenance must reject fake hashes");
    assert!(errors
        .iter()
        .any(|error| error.field == "stub_contract.provenance_hashes"));
}

#[test]
fn task_contract_lifecycle_json_round_trips() {
    let lifecycle = build_kernel002_task_contract_lifecycle();

    let json = serde_json::to_string(&lifecycle).expect("lifecycle serializes");
    let decoded: handshake_core::kernel::task_contract_lifecycle::TaskContractLifecycleV1 =
        serde_json::from_str(&json).expect("lifecycle deserializes");

    assert_eq!(decoded, lifecycle);
}

#[test]
fn kernel_action_catalog_exposes_task_contract_lifecycle_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.task_contract_lifecycle.project")
        .expect("task contract lifecycle projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "task_contract_lifecycle_states"));
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}
