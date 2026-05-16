use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    validator_verdict_mediation_contract::{
        build_kernel002_validator_verdict_mediation_contract, project_validator_verdict_mediation,
        validate_validator_verdict_mediation_contract, DependencyImpactKind,
        MediationInstructionKind, ReproducibilityKind, ValidatorVerdictFailureState,
        ValidatorVerdictKind, VerdictRoutingOutcome, VerdictSeverity,
        VALIDATOR_VERDICT_MEDIATION_PROJECTION_SCHEMA_ID, VALIDATOR_VERDICT_MEDIATION_SCHEMA_ID,
    },
};

#[test]
fn validator_verdict_records_failed_acceptance_evidence_severity_and_routing() {
    let contract = build_kernel002_validator_verdict_mediation_contract();

    validate_validator_verdict_mediation_contract(&contract)
        .expect("validator verdict mediation contract validates");

    assert_eq!(contract.schema_id, VALIDATOR_VERDICT_MEDIATION_SCHEMA_ID);
    assert_eq!(
        contract.wp_id,
        "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
    );
    assert_eq!(contract.mt_id, "MT-057");
    assert_eq!(
        contract.validator_verdict.verdict,
        ValidatorVerdictKind::Fail
    );
    assert_eq!(contract.validator_verdict.severity, VerdictSeverity::High);
    assert_eq!(
        contract.validator_verdict.reproducibility,
        ReproducibilityKind::Deterministic
    );
    assert!(!contract
        .validator_verdict
        .failed_acceptance_criteria
        .is_empty());
    assert!(!contract.validator_verdict.evidence_refs.is_empty());
    assert_eq!(
        contract.validator_verdict.dependency_impact.kind,
        DependencyImpactKind::BlocksDependents
    );
    assert_eq!(
        contract.validator_verdict.routing_outcome,
        VerdictRoutingOutcome::MustLoopBack
    );
    assert_eq!(
        contract.mediation_instruction.instruction_kind,
        MediationInstructionKind::Repair
    );
    assert!(!contract
        .mediation_instruction
        .exact_remediation_steps
        .is_empty());
}

#[test]
fn validator_verdict_projection_is_review_response_without_status_mutation() {
    let contract = build_kernel002_validator_verdict_mediation_contract();

    let projection = project_validator_verdict_mediation(&contract)
        .expect("validator verdict projection derives");

    assert_eq!(
        projection.schema_id,
        VALIDATOR_VERDICT_MEDIATION_PROJECTION_SCHEMA_ID
    );
    assert_eq!(projection.wp_id, contract.wp_id);
    assert_eq!(projection.mt_id, contract.mt_id);
    assert_eq!(projection.receipt_kind, "VALIDATOR_REVIEW");
    assert_eq!(projection.named_verb, "MT_VERDICT");
    assert!(!projection.status_mutation_allowed);
    assert!(!projection.mt_may_advance);
    assert!(projection.mt_must_loop_back);
    assert!(!projection.mt_must_escalate);
    assert_eq!(
        projection.failed_acceptance_criteria,
        contract.validator_verdict.failed_acceptance_criteria
    );
    assert_eq!(
        projection.remediation_steps,
        contract.mediation_instruction.exact_remediation_steps
    );
}

#[test]
fn validator_verdict_rejects_impossible_or_unactionable_outcomes() {
    let mut contract = build_kernel002_validator_verdict_mediation_contract();
    contract.validator_verdict.verdict = ValidatorVerdictKind::Pass;
    let errors = validate_validator_verdict_mediation_contract(&contract)
        .expect_err("pass verdict cannot carry failed acceptance criteria");
    assert!(errors
        .iter()
        .any(|error| error.field == "validator_verdict.failed_acceptance_criteria"));

    let mut contract = build_kernel002_validator_verdict_mediation_contract();
    contract
        .mediation_instruction
        .exact_remediation_steps
        .clear();
    let errors = validate_validator_verdict_mediation_contract(&contract)
        .expect_err("failed verdict must include exact remediation steps");
    assert!(errors
        .iter()
        .any(|error| error.field == "mediation_instruction.exact_remediation_steps"));

    let mut contract = build_kernel002_validator_verdict_mediation_contract();
    contract.validator_verdict.severity = VerdictSeverity::Critical;
    contract.validator_verdict.routing_outcome = VerdictRoutingOutcome::MayAdvance;
    let errors = validate_validator_verdict_mediation_contract(&contract)
        .expect_err("critical failed verdict cannot advance");
    assert!(errors
        .iter()
        .any(|error| error.field == "validator_verdict.routing_outcome"));
}

#[test]
fn validator_verdict_records_failure_states_and_current_research_basis() {
    let contract = build_kernel002_validator_verdict_mediation_contract();

    for failure_state in [
        ValidatorVerdictFailureState::MissingVerdict,
        ValidatorVerdictFailureState::MissingFailedAcceptanceCriteria,
        ValidatorVerdictFailureState::MissingEvidence,
        ValidatorVerdictFailureState::MissingSeverity,
        ValidatorVerdictFailureState::MissingReproducibility,
        ValidatorVerdictFailureState::MissingRemediationInstructions,
        ValidatorVerdictFailureState::MissingDependencyImpact,
        ValidatorVerdictFailureState::MissingRoutingOutcome,
        ValidatorVerdictFailureState::PassWithFailedCriteria,
        ValidatorVerdictFailureState::FailWithoutMediation,
        ValidatorVerdictFailureState::AdvanceWithCriticalFailure,
    ] {
        assert!(contract.failure_states.contains(&failure_state));
    }

    for source_fragment in [
        "docs.github.com/en/rest/checks/runs",
        "docs.gitlab.com/ci/testing/unit_test_reports",
        "gerrit-review.googlesource.com/Documentation/config-submit-requirements",
        "kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle",
    ] {
        assert!(contract
            .research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(source_fragment)));
    }
}

#[test]
fn kernel_action_catalog_exposes_validator_verdict_mediation_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.validator_verdict_mediation.project")
        .expect("validator verdict mediation projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "validator_verdict_routing"));
}
