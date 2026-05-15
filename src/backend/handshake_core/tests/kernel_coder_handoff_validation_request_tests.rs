use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    coder_handoff_validation_request::{
        build_kernel002_coder_handoff_validation_request, project_coder_handoff_validation_request,
        validate_coder_handoff_validation_request, CoderHandoffArtifactKind,
        CoderHandoffContractV1, CoderHandoffFailureState, CoderHandoffTestOutcome,
        CODER_HANDOFF_VALIDATION_REQUEST_PROJECTION_SCHEMA_ID,
        CODER_HANDOFF_VALIDATION_REQUEST_SCHEMA_ID,
    },
};

#[test]
fn coder_handoff_contract_records_identity_scope_artifacts_receipts_and_review_request() {
    let contract: CoderHandoffContractV1 = build_kernel002_coder_handoff_validation_request();

    validate_coder_handoff_validation_request(&contract).expect("coder handoff contract validates");

    assert_eq!(
        contract.schema_id,
        CODER_HANDOFF_VALIDATION_REQUEST_SCHEMA_ID
    );
    assert_eq!(contract.wp_id, contract.claimed_scope.wp_id);
    assert_eq!(contract.mt_id, contract.claimed_scope.mt_id);
    assert_eq!(contract.actor.actor_role, "CODER");
    assert!(!contract.actor.actor_session.is_empty());
    assert!(contract
        .claimed_scope
        .allowed_paths
        .contains(&"src/backend/handshake_core/src/kernel/**".to_string()));
    assert!(contract
        .touched_artifacts
        .iter()
        .any(|artifact| artifact.kind == CoderHandoffArtifactKind::SourceFile));
    assert!(contract
        .touched_actions
        .iter()
        .any(|action| action.action_id == "kernel.coder_handoff_validation_request.project"));
    assert!(contract
        .receipt_refs
        .iter()
        .any(|receipt| receipt.receipt_kind == "STATUS"));
    assert!(contract
        .tests
        .iter()
        .any(|test| test.outcome == CoderHandoffTestOutcome::Passed));
    assert!(!contract.evidence_refs.is_empty());
    assert!(!contract.known_blockers.is_empty());
    assert_eq!(contract.requested_review.target_role, "WP_VALIDATOR");
    assert_eq!(contract.requested_review.receipt_kind, "REVIEW_REQUEST");
    assert_eq!(contract.requested_review.named_verb, "MT_HANDOFF");
    assert_eq!(contract.requested_review.review_mode, "OVERLAP");
    assert!(!contract.requested_review.status_edit_allowed);
}

#[test]
fn coder_handoff_projection_generates_validator_request_without_status_editing() {
    let contract = build_kernel002_coder_handoff_validation_request();

    let projection = project_coder_handoff_validation_request(&contract)
        .expect("coder handoff projection derives");

    assert_eq!(
        projection.schema_id,
        CODER_HANDOFF_VALIDATION_REQUEST_PROJECTION_SCHEMA_ID
    );
    assert_eq!(projection.wp_id, contract.wp_id);
    assert_eq!(projection.mt_id, contract.mt_id);
    assert_eq!(projection.source_handoff_id, contract.handoff_id);
    assert_eq!(projection.review_request_receipt_kind, "REVIEW_REQUEST");
    assert_eq!(projection.target_role, "WP_VALIDATOR");
    assert!(projection.review_request_ready);
    assert!(!projection.status_mutation_allowed);
    assert!(projection
        .microtask_json_refs
        .contains(&contract.claimed_scope.microtask_contract_ref));
    assert_eq!(
        projection.file_targets.len(),
        contract.touched_artifacts.len()
    );
    assert!(projection
        .evidence_refs
        .iter()
        .any(|evidence| evidence.contains("kernel_coder_handoff_validation_request")));
}

#[test]
fn coder_handoff_rejects_missing_identity_out_of_scope_files_and_manual_status_mutation() {
    let mut contract = build_kernel002_coder_handoff_validation_request();
    contract.actor.actor_session.clear();
    let errors = validate_coder_handoff_validation_request(&contract)
        .expect_err("actor session is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "actor.actor_session"));

    let mut contract = build_kernel002_coder_handoff_validation_request();
    contract.touched_artifacts[0].path = "src/backend/handshake_core/src/unrelated.rs".to_string();
    let errors = validate_coder_handoff_validation_request(&contract)
        .expect_err("touched file outside MT allowed paths must fail");
    assert!(errors
        .iter()
        .any(|error| error.field == "touched_artifacts.path"));

    let mut contract = build_kernel002_coder_handoff_validation_request();
    contract.requested_review.status_edit_allowed = true;
    let errors = validate_coder_handoff_validation_request(&contract)
        .expect_err("review request must not mutate status");
    assert!(errors
        .iter()
        .any(|error| error.field == "requested_review.status_edit_allowed"));
}

#[test]
fn coder_handoff_records_failure_states_and_research_basis() {
    let contract = build_kernel002_coder_handoff_validation_request();

    validate_coder_handoff_validation_request(&contract).expect("coder handoff contract validates");

    for failure_state in [
        CoderHandoffFailureState::MissingMicrotaskIdentity,
        CoderHandoffFailureState::MissingActorSession,
        CoderHandoffFailureState::ScopeOutsideAllowedPaths,
        CoderHandoffFailureState::MissingReceipts,
        CoderHandoffFailureState::MissingTests,
        CoderHandoffFailureState::MissingEvidence,
        CoderHandoffFailureState::MissingRequestedReview,
        CoderHandoffFailureState::ManualStatusEditAttempt,
        CoderHandoffFailureState::ReviewRequestWouldMutateStatus,
    ] {
        assert!(contract.failure_states.contains(&failure_state));
    }

    for source_fragment in [
        "docs.github.com/en/rest/pulls/review-requests",
        "docs.gitlab.com/api/merge_request_approvals",
        "gerrit-review.googlesource.com/Documentation/config-submit-requirements",
        "kubernetes.io/docs/concepts/workloads/pods/pod-condition",
    ] {
        assert!(contract
            .research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(source_fragment)));
    }
}

#[test]
fn kernel_action_catalog_exposes_coder_handoff_validation_request_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.coder_handoff_validation_request.project")
        .expect("coder handoff validation request projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "coder_handoff_identity_scope"));
}
