use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    mt_loop_scheduler_contract::{
        build_kernel002_mt_loop_scheduler, evaluate_mt_loop_scheduler, validate_mt_loop_scheduler,
        DispatchDecisionKind, LoopPrerequisiteKind, LoopSchedulerFailureState,
        MT_LOOP_SCHEDULER_PROJECTION_SCHEMA_ID, MT_LOOP_SCHEDULER_SCHEMA_ID,
    },
    validator_verdict_mediation_contract::{ValidatorVerdictKind, VerdictRoutingOutcome},
};

#[test]
fn mt_loop_scheduler_dispatches_next_coder_only_when_all_gates_allow_it() {
    let contract = build_kernel002_mt_loop_scheduler();

    validate_mt_loop_scheduler(&contract).expect("scheduler contract validates");
    let projection = evaluate_mt_loop_scheduler(&contract).expect("scheduler projection derives");

    assert_eq!(contract.schema_id, MT_LOOP_SCHEDULER_SCHEMA_ID);
    assert_eq!(contract.mt_id, "MT-060");
    assert_eq!(contract.next_mt_id, "MT-061");
    assert!(contract.current_coder.completed);
    assert!(contract.claim_lease.active);
    assert!(!contract.claim_lease.expired);
    assert!(contract.dependency_state.blocked_by_refs.is_empty());
    assert!(contract
        .dependency_state
        .dependent_mt_refs
        .contains(&"MT-061".to_string()));
    assert!(contract.retry_budget.remaining_attempts > 0);
    assert_eq!(contract.verdict_state.verdict, ValidatorVerdictKind::Pass);
    assert_eq!(
        contract.verdict_state.routing_outcome,
        VerdictRoutingOutcome::MayAdvance
    );
    for prerequisite in [
        LoopPrerequisiteKind::ClaimLease,
        LoopPrerequisiteKind::CurrentCoderCompletion,
        LoopPrerequisiteKind::DependencyState,
        LoopPrerequisiteKind::RetryBudget,
        LoopPrerequisiteKind::VerdictState,
    ] {
        assert!(contract.required_prerequisites.contains(&prerequisite));
    }

    assert_eq!(projection.schema_id, MT_LOOP_SCHEDULER_PROJECTION_SCHEMA_ID);
    assert_eq!(projection.decision, DispatchDecisionKind::DispatchNextCoder);
    assert!(projection.next_coder_dispatch_required);
    assert_eq!(projection.next_mt_id, "MT-061");
    assert!(projection.dependents_may_advance);
    assert!(!projection.remediation_required);
    assert!(!projection.status_mutation_allowed);
}

#[test]
fn mt_loop_scheduler_routes_failed_verdicts_to_remediation_before_dependents_advance() {
    let mut contract = build_kernel002_mt_loop_scheduler();
    contract.verdict_state.verdict = ValidatorVerdictKind::Fail;
    contract.verdict_state.routing_outcome = VerdictRoutingOutcome::MustLoopBack;

    let projection = evaluate_mt_loop_scheduler(&contract)
        .expect("failed verdict still projects a governed remediation route");

    assert_eq!(
        projection.decision,
        DispatchDecisionKind::RouteToRemediation
    );
    assert!(!projection.next_coder_dispatch_required);
    assert!(!projection.dependents_may_advance);
    assert!(projection.remediation_required);
    assert!(projection
        .blocked_prerequisites
        .contains(&LoopPrerequisiteKind::VerdictState));
    assert!(projection
        .remediation_action_ids
        .contains(&"kernel.remediation_work_generation.project".to_string()));
}

#[test]
fn mt_loop_scheduler_holds_or_remediates_failed_prerequisites_without_status_mutation() {
    let mut contract = build_kernel002_mt_loop_scheduler();
    contract.current_coder.completed = false;

    let projection = evaluate_mt_loop_scheduler(&contract)
        .expect("incomplete coder projects hold instead of dispatch");
    assert_eq!(
        projection.decision,
        DispatchDecisionKind::HoldForCoderCompletion
    );
    assert!(!projection.next_coder_dispatch_required);
    assert!(!projection.status_mutation_allowed);

    let mut contract = build_kernel002_mt_loop_scheduler();
    contract
        .dependency_state
        .blocked_by_refs
        .push("MT-059".to_string());
    let projection = evaluate_mt_loop_scheduler(&contract)
        .expect("blocked dependency projects remediation before advancement");
    assert_eq!(
        projection.decision,
        DispatchDecisionKind::RouteToRemediation
    );
    assert!(!projection.dependents_may_advance);
    assert!(projection.remediation_required);
    assert!(projection
        .blocked_prerequisites
        .contains(&LoopPrerequisiteKind::DependencyState));
}

#[test]
fn mt_loop_scheduler_rejects_missing_lease_dependency_retry_or_verdict_evidence() {
    let mut contract = build_kernel002_mt_loop_scheduler();
    contract.claim_lease.lease_id.clear();
    let errors = validate_mt_loop_scheduler(&contract).expect_err("lease id is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "claim_lease.lease_id"));

    let mut contract = build_kernel002_mt_loop_scheduler();
    contract.dependency_state.dependent_mt_refs.clear();
    let errors = validate_mt_loop_scheduler(&contract).expect_err("dependent refs are required");
    assert!(errors
        .iter()
        .any(|error| error.field == "dependency_state.dependent_mt_refs"));

    let mut contract = build_kernel002_mt_loop_scheduler();
    contract.retry_budget.max_attempts = 0;
    let errors = validate_mt_loop_scheduler(&contract).expect_err("retry max is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "retry_budget.max_attempts"));

    let mut contract = build_kernel002_mt_loop_scheduler();
    contract.verdict_state.evidence_refs.clear();
    let errors = validate_mt_loop_scheduler(&contract).expect_err("verdict evidence is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "verdict_state.evidence_refs"));
}

#[test]
fn mt_loop_scheduler_records_failure_states_and_current_research_basis() {
    let contract = build_kernel002_mt_loop_scheduler();

    for failure_state in [
        LoopSchedulerFailureState::MissingClaimLease,
        LoopSchedulerFailureState::MissingCurrentCoderCompletion,
        LoopSchedulerFailureState::MissingDependencyState,
        LoopSchedulerFailureState::MissingRetryBudget,
        LoopSchedulerFailureState::MissingVerdictState,
        LoopSchedulerFailureState::DispatchWithFailedPrerequisite,
        LoopSchedulerFailureState::DependentAdvancedBeforeRemediation,
    ] {
        assert!(contract.failure_states.contains(&failure_state));
    }

    for source_fragment in [
        "kubernetes.io/docs/concepts/architecture/controller",
        "kubernetes.io/docs/concepts/architecture/leases",
        "docs.temporal.io/encyclopedia/retry-policies",
        "docs.github.com/en/actions/concepts/workflows-and-actions/concurrency",
    ] {
        assert!(contract
            .research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(source_fragment)));
    }
}

#[test]
fn kernel_action_catalog_exposes_mt_loop_scheduler_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.mt_loop_scheduler.project")
        .expect("MT loop scheduler projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "mt_loop_scheduler_lease_state"));
}
