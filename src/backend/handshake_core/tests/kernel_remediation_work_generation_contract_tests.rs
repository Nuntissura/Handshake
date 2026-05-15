use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    remediation_work_generation_contract::{
        build_kernel002_remediation_work_generation, project_remediation_work_generation,
        validate_remediation_work_generation, RemediationDestinationKind,
        RemediationGenerationFailureState, RemediationWorkContractV1, RemediationWorkKind,
        StubContractV1, REMEDIATION_WORK_GENERATION_PROJECTION_SCHEMA_ID,
        REMEDIATION_WORK_GENERATION_SCHEMA_ID,
    },
};

#[test]
fn remediation_generation_preserves_parent_links_dependencies_and_recheck_contract() {
    let contract: RemediationWorkContractV1 = build_kernel002_remediation_work_generation();

    validate_remediation_work_generation(&contract)
        .expect("remediation work generation contract validates");

    assert_eq!(contract.schema_id, REMEDIATION_WORK_GENERATION_SCHEMA_ID);
    assert_eq!(contract.mt_id, "MT-059");
    assert_eq!(contract.remediation_microtask.parent_wp_id, contract.wp_id);
    assert!(contract
        .remediation_microtask
        .parent_mt_refs
        .contains(&"MT-057".to_string()));
    assert!(contract
        .remediation_microtask
        .parent_mt_refs
        .contains(&"MT-058".to_string()));
    assert!(contract
        .remediation_microtask
        .dependency_state
        .blocked_by_refs
        .contains(&"MT-057".to_string()));
    assert!(!contract
        .remediation_microtask
        .acceptance_criteria
        .is_empty());
    assert!(contract
        .remediation_microtask
        .allowed_action_ids
        .contains(&"kernel.validator_verdict_mediation.project".to_string()));
    assert!(contract
        .remediation_microtask
        .allowed_action_ids
        .contains(&"kernel.validator_finding_reports.project".to_string()));
    assert!(!contract.remediation_microtask.write_box_refs.is_empty());
    assert!(!contract.remediation_microtask.evidence_refs.is_empty());
    assert!(contract.remediation_microtask.retry_budget.max_attempts > 0);
    assert!(contract.remediation_microtask.validator_recheck.required);
}

#[test]
fn remediation_generation_preserves_packet_stub_route_for_scope_expansion() {
    let contract = build_kernel002_remediation_work_generation();
    let stub: &StubContractV1 = &contract.remediation_packet_stub;

    assert_eq!(
        stub.destination_kind,
        RemediationDestinationKind::NewPacketStub
    );
    assert_eq!(stub.parent_wp_id, contract.wp_id);
    assert!(contract
        .remediation_packet_stub
        .parent_report_refs
        .iter()
        .any(|report| report.contains("gap-report")));
    assert!(!contract
        .remediation_packet_stub
        .acceptance_criteria
        .is_empty());
    assert!(!contract.remediation_packet_stub.evidence_refs.is_empty());
    assert!(
        contract
            .remediation_packet_stub
            .generated_from_contracts_only
    );
}

#[test]
fn remediation_generation_projection_lists_generated_work_without_authority_mutation() {
    let contract = build_kernel002_remediation_work_generation();

    let projection =
        project_remediation_work_generation(&contract).expect("remediation projection derives");

    assert_eq!(
        projection.schema_id,
        REMEDIATION_WORK_GENERATION_PROJECTION_SCHEMA_ID
    );
    assert_eq!(projection.wp_id, contract.wp_id);
    assert_eq!(projection.mt_id, contract.mt_id);
    assert!(projection
        .generated_work_kinds
        .contains(&RemediationWorkKind::MicroTask));
    assert!(projection
        .generated_work_kinds
        .contains(&RemediationWorkKind::PacketStub));
    assert!(projection.validator_recheck_required);
    assert!(!projection.status_mutation_allowed);
    assert!(!projection.prose_source_allowed);
}

#[test]
fn remediation_generation_rejects_missing_parent_links_actions_or_recheck() {
    let mut contract = build_kernel002_remediation_work_generation();
    contract.remediation_microtask.parent_mt_refs.clear();
    let errors =
        validate_remediation_work_generation(&contract).expect_err("parent MT refs are required");
    assert!(errors
        .iter()
        .any(|error| error.field == "remediation_microtask.parent_mt_refs"));

    let mut contract = build_kernel002_remediation_work_generation();
    contract.remediation_microtask.allowed_action_ids.clear();
    let errors =
        validate_remediation_work_generation(&contract).expect_err("allowed actions are required");
    assert!(errors
        .iter()
        .any(|error| error.field == "remediation_microtask.allowed_action_ids"));

    let mut contract = build_kernel002_remediation_work_generation();
    contract.remediation_microtask.validator_recheck.required = false;
    let errors =
        validate_remediation_work_generation(&contract).expect_err("validator recheck is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "remediation_microtask.validator_recheck"));
}

#[test]
fn remediation_generation_records_failure_states_and_research_basis() {
    let contract = build_kernel002_remediation_work_generation();

    for failure_state in [
        RemediationGenerationFailureState::MissingParentLinks,
        RemediationGenerationFailureState::MissingDependencyState,
        RemediationGenerationFailureState::MissingAcceptanceCriteria,
        RemediationGenerationFailureState::MissingAllowedActions,
        RemediationGenerationFailureState::MissingWriteBoxes,
        RemediationGenerationFailureState::MissingEvidenceRefs,
        RemediationGenerationFailureState::MissingRetryBudget,
        RemediationGenerationFailureState::MissingValidatorRecheck,
        RemediationGenerationFailureState::ProseSourceUsed,
    ] {
        assert!(contract.failure_states.contains(&failure_state));
    }

    for source_fragment in [
        "docs.github.com/en/rest/issues",
        "docs.gitlab.com/user/work_items/linked_items",
        "kubernetes.io/docs/concepts/overview/working-with-objects/owners-dependents",
        "docs.temporal.io/encyclopedia/retry-policies",
    ] {
        assert!(contract
            .research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(source_fragment)));
    }
}

#[test]
fn kernel_action_catalog_exposes_remediation_generation_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.remediation_work_generation.project")
        .expect("remediation work generation projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "remediation_generation_parent_links"));
}
