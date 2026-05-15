use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    validator_finding_report_contract::{
        build_kernel002_validator_finding_reports, project_validator_finding_reports,
        validate_validator_finding_reports, FindingRoutingOutcome, ProposedDestinationKind,
        ReportAuthorityMode, ValidatorFindingReportContractV1, ValidatorFindingReportFailureState,
        ValidatorFindingReportKind, VALIDATOR_FINDING_REPORTS_PROJECTION_SCHEMA_ID,
        VALIDATOR_FINDING_REPORTS_SCHEMA_ID,
    },
};

#[test]
fn validator_finding_reports_preserve_issue_bug_gap_and_out_of_scope_details() {
    let contract: ValidatorFindingReportContractV1 = build_kernel002_validator_finding_reports();

    validate_validator_finding_reports(&contract).expect("finding report contract validates");

    assert_eq!(contract.schema_id, VALIDATOR_FINDING_REPORTS_SCHEMA_ID);
    assert_eq!(contract.mt_id, "MT-058");
    assert_eq!(
        contract.issue_report.core.report_kind,
        ValidatorFindingReportKind::Issue
    );
    assert_eq!(
        contract.bug_report.core.report_kind,
        ValidatorFindingReportKind::Bug
    );
    assert_eq!(
        contract.gap_report.core.report_kind,
        ValidatorFindingReportKind::Gap
    );
    assert_eq!(
        contract.out_of_scope_report.core.report_kind,
        ValidatorFindingReportKind::OutOfScope
    );

    for report in contract.all_report_cores() {
        assert_eq!(
            report.authority_mode,
            ReportAuthorityMode::MachineContractOnly
        );
        assert!(!report.validator_reasoning.is_empty());
        assert!(!report.source_refs.is_empty());
        assert!(!report.affected_surfaces.is_empty());
        assert!(!report.reproduction_or_proof.is_empty());
        assert!(!report.proposed_destination.destination_ref.is_empty());
    }
}

#[test]
fn validator_finding_projection_routes_reports_without_prose_authority() {
    let contract = build_kernel002_validator_finding_reports();

    let projection =
        project_validator_finding_reports(&contract).expect("finding projection derives");

    assert_eq!(
        projection.schema_id,
        VALIDATOR_FINDING_REPORTS_PROJECTION_SCHEMA_ID
    );
    assert_eq!(projection.wp_id, contract.wp_id);
    assert_eq!(projection.mt_id, contract.mt_id);
    assert_eq!(projection.report_ids.len(), 4);
    assert!(projection
        .report_kinds
        .contains(&ValidatorFindingReportKind::Issue));
    assert!(projection
        .destination_kinds
        .contains(&ProposedDestinationKind::ProductBugBacklog));
    assert!(projection
        .destination_kinds
        .contains(&ProposedDestinationKind::SpecGapQueue));
    assert!(projection
        .destination_kinds
        .contains(&ProposedDestinationKind::OutOfScopeParkingLot));
    assert!(projection
        .routing_outcomes
        .contains(&FindingRoutingOutcome::CreateRemediationMicrotask));
    assert!(!projection.status_mutation_allowed);
    assert!(!projection.prose_only_report_allowed);
}

#[test]
fn validator_finding_reports_reject_missing_proof_and_prose_only_authority() {
    let mut contract = build_kernel002_validator_finding_reports();
    contract.issue_report.core.validator_reasoning.clear();
    let errors =
        validate_validator_finding_reports(&contract).expect_err("validator reasoning is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "issue_report.validator_reasoning"));

    let mut contract = build_kernel002_validator_finding_reports();
    contract.bug_report.core.reproduction_or_proof.clear();
    let errors = validate_validator_finding_reports(&contract)
        .expect_err("bug report requires reproduction or proof");
    assert!(errors
        .iter()
        .any(|error| error.field == "bug_report.reproduction_or_proof"));

    let mut contract = build_kernel002_validator_finding_reports();
    contract.gap_report.core.authority_mode = ReportAuthorityMode::ProseOnlyReport;
    let errors = validate_validator_finding_reports(&contract)
        .expect_err("prose-only report authority must be rejected");
    assert!(errors
        .iter()
        .any(|error| error.field == "gap_report.authority_mode"));
}

#[test]
fn validator_finding_reports_record_failure_states_and_research_basis() {
    let contract = build_kernel002_validator_finding_reports();

    for failure_state in [
        ValidatorFindingReportFailureState::MissingValidatorReasoning,
        ValidatorFindingReportFailureState::MissingSourceRefs,
        ValidatorFindingReportFailureState::MissingAffectedSurfaces,
        ValidatorFindingReportFailureState::MissingReproductionOrProof,
        ValidatorFindingReportFailureState::MissingProposedDestination,
        ValidatorFindingReportFailureState::MissingRoutingOutcome,
        ValidatorFindingReportFailureState::ProseOnlyReportAuthority,
        ValidatorFindingReportFailureState::ReportKindMismatch,
        ValidatorFindingReportFailureState::OutOfScopeWithoutParkingDestination,
    ] {
        assert!(contract.failure_states.contains(&failure_state));
    }

    for source_fragment in [
        "docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms",
        "docs.github.com/en/code-security/reference/code-scanning/sarif-files/sarif-support-for-code-scanning",
        "docs.gitlab.com/user/work_items",
        "kubernetes.io/docs/reference/using-api/api-concepts",
    ] {
        assert!(contract
            .research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(source_fragment)));
    }
}

#[test]
fn kernel_action_catalog_exposes_validator_finding_report_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.validator_finding_reports.project")
        .expect("validator finding reports projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "validator_finding_report_destinations"));
}
