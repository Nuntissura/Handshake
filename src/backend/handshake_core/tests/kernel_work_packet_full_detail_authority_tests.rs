use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    work_packet_full_detail_authority::{
        build_kernel002_work_packet_full_detail_authority,
        validate_work_packet_full_detail_authority, GeneratedArtifactKind,
        MicrotaskPlanSourceField, WorkPacketAuthorityFailureState, WorkPacketFullDetailSection,
    },
};

#[test]
fn work_packet_full_detail_authority_makes_packet_the_execution_source() {
    let authority = build_kernel002_work_packet_full_detail_authority();

    validate_work_packet_full_detail_authority(&authority)
        .expect("work packet full-detail authority validates");

    assert_eq!(
        authority.schema_id,
        "hsk.kernel.work_packet_full_detail_authority@1"
    );
    assert_eq!(
        authority.parent_contract_schema_id,
        "hsk.work_packet_contract@1"
    );
    assert!(
        authority
            .full_detail_authority
            .can_execute_without_microtask_files
    );
    assert!(!authority.hidden_chat_context_required);
    assert!(!authority.manual_sidecar_authority_allowed);
    assert!(authority
        .full_detail_authority
        .required_sections
        .contains(&WorkPacketFullDetailSection::ImplementationRealityNotes));
    assert!(authority
        .full_detail_authority
        .required_sections
        .contains(&WorkPacketFullDetailSection::AcceptanceCriteria));
    assert!(authority
        .full_detail_authority
        .required_sections
        .contains(&WorkPacketFullDetailSection::VerificationPlan));
}

#[test]
fn microtask_source_plan_can_regenerate_every_contract_and_projection() {
    let authority = build_kernel002_work_packet_full_detail_authority();

    validate_work_packet_full_detail_authority(&authority)
        .expect("work packet full-detail authority validates");

    assert_eq!(authority.microtask_source_plan.declared_microtask_count, 61);
    assert_eq!(
        authority
            .microtask_source_plan
            .declared_ids
            .first()
            .unwrap(),
        "MT-001"
    );
    assert_eq!(
        authority.microtask_source_plan.declared_ids.last().unwrap(),
        "MT-061"
    );
    assert!(authority
        .microtask_source_plan
        .declared_ids
        .contains(&"MT-052".to_string()));
    assert!(authority
        .microtask_source_plan
        .source_fields
        .contains(&MicrotaskPlanSourceField::AcceptanceCriteria));
    assert!(authority
        .microtask_source_plan
        .source_fields
        .contains(&MicrotaskPlanSourceField::ProofTargets));
    assert!(authority
        .microtask_source_plan
        .generated_artifacts
        .contains(&GeneratedArtifactKind::MicrotaskJsonContract));
    assert!(authority
        .microtask_source_plan
        .generated_artifacts
        .contains(&GeneratedArtifactKind::MarkdownProjection));
    assert!(authority
        .microtask_source_plan
        .generated_artifacts
        .contains(&GeneratedArtifactKind::TaskBoardRow));
    assert!(authority
        .regeneration_contract
        .required_provenance_fields
        .contains(&"source_hash".to_string()));
    assert!(authority
        .regeneration_contract
        .required_provenance_fields
        .contains(&"projection_hash".to_string()));
    assert_eq!(
        authority.regeneration_contract.generator,
        "wp-contract-import.mjs"
    );
}

#[test]
fn work_packet_full_detail_authority_rejects_sidecar_and_round_trip_loss() {
    let mut authority = build_kernel002_work_packet_full_detail_authority();
    authority.hidden_chat_context_required = true;
    let errors = validate_work_packet_full_detail_authority(&authority)
        .expect_err("hidden chat context must fail validation");
    assert!(errors
        .iter()
        .any(|error| error.field == "hidden_chat_context_required"));

    let mut authority = build_kernel002_work_packet_full_detail_authority();
    authority.microtask_source_plan.declared_ids.pop();
    let errors = validate_work_packet_full_detail_authority(&authority)
        .expect_err("declared MT count must be one-to-one");
    assert!(errors
        .iter()
        .any(|error| error.field == "microtask_source_plan.declared_ids"));

    let mut authority = build_kernel002_work_packet_full_detail_authority();
    authority
        .microtask_source_plan
        .source_fields
        .retain(|field| *field != MicrotaskPlanSourceField::AcceptanceCriteria);
    let errors = validate_work_packet_full_detail_authority(&authority)
        .expect_err("acceptance criteria must stay in the source plan");
    assert!(errors
        .iter()
        .any(|error| error.field == "microtask_source_plan.source_fields"));

    assert!(authority
        .failure_states
        .contains(&WorkPacketAuthorityFailureState::RoundTripLoss));
}

#[test]
fn kernel_action_catalog_exposes_work_packet_full_detail_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.work_packet_full_detail.project")
        .expect("work packet full-detail projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "work_packet_no_context_execution"));
}
