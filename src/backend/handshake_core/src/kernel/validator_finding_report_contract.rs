pub const VALIDATOR_FINDING_REPORTS_SCHEMA_ID: &str = "hsk.kernel.validator_finding_reports@1";
pub const VALIDATOR_FINDING_REPORTS_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.validator_finding_reports_projection@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidatorFindingReportKind {
    Issue,
    Bug,
    Gap,
    OutOfScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingRoutingOutcome {
    AppendToValidatorVerdict,
    CreateRemediationMicrotask,
    CreateProductBug,
    CreateSpecGap,
    ParkOutOfScope,
    EscalateToOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProposedDestinationKind {
    CurrentWorkPacket,
    RemediationMicrotask,
    ProductBugBacklog,
    SpecGapQueue,
    OutOfScopeParkingLot,
    OperatorDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingSourceKind {
    ValidatorVerdict,
    TestOutput,
    SourceFile,
    WorkPacketContract,
    MicrotaskContract,
    ExternalReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AffectedSurfaceKind {
    ProductCode,
    ProductTest,
    ProofHarness,
    WorkPacketContract,
    MicrotaskContract,
    OperatorWorkflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FindingProofKind {
    ReproductionSteps,
    ProofCommand,
    SourceReference,
    ArtifactReference,
    NegativeCounterexample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportAuthorityMode {
    MachineContractOnly,
    GeneratedMarkdownProjection,
    ProseOnlyReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidatorFindingReportFailureState {
    MissingValidatorReasoning,
    MissingSourceRefs,
    MissingAffectedSurfaces,
    MissingReproductionOrProof,
    MissingProposedDestination,
    MissingRoutingOutcome,
    ProseOnlyReportAuthority,
    ReportKindMismatch,
    OutOfScopeWithoutParkingDestination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingSourceRefV1 {
    pub source_ref: String,
    pub source_kind: FindingSourceKind,
    pub source_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffectedSurfaceRefV1 {
    pub surface_ref: String,
    pub surface_kind: AffectedSurfaceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingProofRefV1 {
    pub proof_kind: FindingProofKind,
    pub proof_ref: String,
    pub result_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedFindingDestinationV1 {
    pub destination_kind: ProposedDestinationKind,
    pub destination_ref: String,
    pub owner_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorFindingReportCoreV1 {
    pub report_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub report_kind: ValidatorFindingReportKind,
    pub validator_role: String,
    pub validator_session: String,
    pub validator_reasoning: String,
    pub source_refs: Vec<FindingSourceRefV1>,
    pub affected_surfaces: Vec<AffectedSurfaceRefV1>,
    pub reproduction_or_proof: Vec<FindingProofRefV1>,
    pub proposed_destination: ProposedFindingDestinationV1,
    pub routing_outcome: FindingRoutingOutcome,
    pub authority_mode: ReportAuthorityMode,
    pub status_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueReportContractV1 {
    pub core: ValidatorFindingReportCoreV1,
    pub issue_summary: String,
    pub classification_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BugReportContractV1 {
    pub core: ValidatorFindingReportCoreV1,
    pub observed_behavior: String,
    pub expected_behavior: String,
    pub reproduction_steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapReportContractV1 {
    pub core: ValidatorFindingReportCoreV1,
    pub gap_statement: String,
    pub missing_contract_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutOfScopeReportContractV1 {
    pub core: ValidatorFindingReportCoreV1,
    pub rejected_scope_statement: String,
    pub parking_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorFindingReportResearchBasisV1 {
    pub source_ref: String,
    pub pattern_found: String,
    pub selected_reuse: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorFindingReportsContractV1 {
    pub schema_id: &'static str,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub issue_report: IssueReportContractV1,
    pub bug_report: BugReportContractV1,
    pub gap_report: GapReportContractV1,
    pub out_of_scope_report: OutOfScopeReportContractV1,
    pub failure_states: Vec<ValidatorFindingReportFailureState>,
    pub research_basis_refs: Vec<ValidatorFindingReportResearchBasisV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

pub type ValidatorFindingReportContractV1 = ValidatorFindingReportsContractV1;

impl ValidatorFindingReportsContractV1 {
    pub fn all_report_cores(&self) -> Vec<&ValidatorFindingReportCoreV1> {
        vec![
            &self.issue_report.core,
            &self.bug_report.core,
            &self.gap_report.core,
            &self.out_of_scope_report.core,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorFindingReportsProjectionV1 {
    pub schema_id: String,
    pub source_contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub report_ids: Vec<String>,
    pub report_kinds: Vec<ValidatorFindingReportKind>,
    pub destination_kinds: Vec<ProposedDestinationKind>,
    pub routing_outcomes: Vec<FindingRoutingOutcome>,
    pub source_refs: Vec<String>,
    pub affected_surfaces: Vec<String>,
    pub status_mutation_allowed: bool,
    pub prose_only_report_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorFindingReportError {
    pub field: String,
    pub message: String,
}

pub fn build_kernel002_validator_finding_reports() -> ValidatorFindingReportsContractV1 {
    let wp_id = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";
    let mt_id = "MT-058";

    ValidatorFindingReportsContractV1 {
        schema_id: VALIDATOR_FINDING_REPORTS_SCHEMA_ID,
        contract_id: "kernel002-validator-finding-reports-mt058".to_string(),
        wp_id: wp_id.to_string(),
        mt_id: mt_id.to_string(),
        issue_report: IssueReportContractV1 {
            core: core(
                wp_id,
                mt_id,
                "kernel002-mt058-issue-report",
                ValidatorFindingReportKind::Issue,
                "Validator found an actionable issue that should stay attached to the current MT review but needs a remediation edge before dependent MTs advance.",
                ProposedDestinationKind::RemediationMicrotask,
                "remediation-mt://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-059",
                "CODER",
                FindingRoutingOutcome::CreateRemediationMicrotask,
            ),
            issue_summary: "Validator issue needs a bounded remediation microtask.".to_string(),
            classification_tags: vec![
                "validator-finding".to_string(),
                "remediation-required".to_string(),
            ],
        },
        bug_report: BugReportContractV1 {
            core: core(
                wp_id,
                mt_id,
                "kernel002-mt058-bug-report",
                ValidatorFindingReportKind::Bug,
                "Validator can preserve a product bug separately from verdict pass/fail so bug routing does not depend on prose report interpretation.",
                ProposedDestinationKind::ProductBugBacklog,
                "bug-backlog://kernel002/validator-bug",
                "PRODUCT_TRIAGE",
                FindingRoutingOutcome::CreateProductBug,
            ),
            observed_behavior: "Finding report only exists as free-form validator prose.".to_string(),
            expected_behavior: "Finding report is typed, reproducible, and routable.".to_string(),
            reproduction_steps: vec![
                "Run validator review with a non-pass/fail finding.".to_string(),
                "Observe that the finding requires a typed bug report destination.".to_string(),
            ],
        },
        gap_report: GapReportContractV1 {
            core: core(
                wp_id,
                mt_id,
                "kernel002-mt058-gap-report",
                ValidatorFindingReportKind::Gap,
                "Validator needs a durable gap report when the packet contract omits required acceptance details or validation inputs.",
                ProposedDestinationKind::SpecGapQueue,
                "spec-gap://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-058",
                "ORCHESTRATOR",
                FindingRoutingOutcome::CreateSpecGap,
            ),
            gap_statement: "Work packet does not yet encode every non-pass/fail validator finding class.".to_string(),
            missing_contract_fields: vec![
                "validator_reasoning".to_string(),
                "proposed_destination".to_string(),
                "routing_outcome".to_string(),
            ],
        },
        out_of_scope_report: OutOfScopeReportContractV1 {
            core: core(
                wp_id,
                mt_id,
                "kernel002-mt058-out-of-scope-report",
                ValidatorFindingReportKind::OutOfScope,
                "Validator can preserve an out-of-scope observation without letting it mutate the active MT status or silently expand the WP.",
                ProposedDestinationKind::OutOfScopeParkingLot,
                "out-of-scope://kernel002/validator-observations",
                "ORCHESTRATOR",
                FindingRoutingOutcome::ParkOutOfScope,
            ),
            rejected_scope_statement:
                "Finding belongs to a future governance UX polish pass, not this Kernel002 MT.".to_string(),
            parking_reason:
                "Preserve the observation for planning without blocking current implementation."
                    .to_string(),
        },
        failure_states: required_failure_states(),
        research_basis_refs: vec![
            research(
                "https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms",
                "Issue forms use structured YAML fields, validation, labels, projects, assignees, and issue type.",
                "Validator finding reports should use typed fields rather than mutable report prose.",
            ),
            research(
                "https://docs.github.com/en/code-security/reference/code-scanning/sarif-files/sarif-support-for-code-scanning",
                "SARIF-style results carry severity, rule, message, location, and precision as machine-readable fields.",
                "Finding reports should preserve source refs, affected surfaces, and proof refs as structured data.",
            ),
            research(
                "https://docs.gitlab.com/user/work_items/",
                "Work items separate issues, tasks, test cases, objectives, state, type, labels, and blocking metadata.",
                "Validator findings should route to explicit destination kinds instead of expanding the current WP implicitly.",
            ),
            research(
                "https://kubernetes.io/docs/reference/using-api/api-concepts/#resource-versions",
                "API resources use resource versions and watch/list semantics so clients can reason about freshness and ordering.",
                "Finding reports should carry stable source refs and hashes for replayable validation state.",
            ),
        ],
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.validator_verdict_mediation_contract".to_string(),
            "kernel.generated_documentation_status_projection".to_string(),
            "kernel.task_contract_lifecycle".to_string(),
        ],
        folded_source_refs: vec![
            "MT-058 Mechanical Issue, Bug, Gap, and Out-of-Scope Reports".to_string(),
            "kernel.validator_verdict_mediation_contract".to_string(),
            "kernel.generated_documentation_status_projection".to_string(),
        ],
    }
}

pub fn validate_validator_finding_reports(
    contract: &ValidatorFindingReportsContractV1,
) -> Result<(), Vec<ValidatorFindingReportError>> {
    let mut errors = Vec::new();

    if contract.schema_id != VALIDATOR_FINDING_REPORTS_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "validator finding reports schema id is required",
        ));
    }
    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    validate_issue_report(&mut errors, contract);
    validate_bug_report(&mut errors, contract);
    validate_gap_report(&mut errors, contract);
    validate_out_of_scope_report(&mut errors, contract);
    validate_failure_states(&mut errors, &contract.failure_states);
    validate_research_basis(&mut errors, &contract.research_basis_refs);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &contract.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &contract.folded_source_refs,
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_validator_finding_reports(
    contract: &ValidatorFindingReportsContractV1,
) -> Result<ValidatorFindingReportsProjectionV1, Vec<ValidatorFindingReportError>> {
    validate_validator_finding_reports(contract)?;

    let reports = contract.all_report_cores();

    Ok(ValidatorFindingReportsProjectionV1 {
        schema_id: VALIDATOR_FINDING_REPORTS_PROJECTION_SCHEMA_ID.to_string(),
        source_contract_id: contract.contract_id.clone(),
        wp_id: contract.wp_id.clone(),
        mt_id: contract.mt_id.clone(),
        report_ids: reports
            .iter()
            .map(|report| report.report_id.clone())
            .collect(),
        report_kinds: reports.iter().map(|report| report.report_kind).collect(),
        destination_kinds: reports
            .iter()
            .map(|report| report.proposed_destination.destination_kind)
            .collect(),
        routing_outcomes: reports
            .iter()
            .map(|report| report.routing_outcome)
            .collect(),
        source_refs: reports
            .iter()
            .flat_map(|report| {
                report
                    .source_refs
                    .iter()
                    .map(|source| source.source_ref.clone())
            })
            .collect(),
        affected_surfaces: reports
            .iter()
            .flat_map(|report| {
                report
                    .affected_surfaces
                    .iter()
                    .map(|surface| surface.surface_ref.clone())
            })
            .collect(),
        status_mutation_allowed: reports.iter().any(|report| report.status_mutation_allowed),
        prose_only_report_allowed: reports
            .iter()
            .any(|report| report.authority_mode == ReportAuthorityMode::ProseOnlyReport),
    })
}

fn validate_issue_report(
    errors: &mut Vec<ValidatorFindingReportError>,
    contract: &ValidatorFindingReportsContractV1,
) {
    validate_core(
        errors,
        contract,
        "issue_report",
        &contract.issue_report.core,
        ValidatorFindingReportKind::Issue,
    );
    require_non_empty(
        errors,
        "issue_report.issue_summary",
        &contract.issue_report.issue_summary,
    );
    require_vec(
        errors,
        "issue_report.classification_tags",
        &contract.issue_report.classification_tags,
    );
}

fn validate_bug_report(
    errors: &mut Vec<ValidatorFindingReportError>,
    contract: &ValidatorFindingReportsContractV1,
) {
    validate_core(
        errors,
        contract,
        "bug_report",
        &contract.bug_report.core,
        ValidatorFindingReportKind::Bug,
    );
    require_non_empty(
        errors,
        "bug_report.observed_behavior",
        &contract.bug_report.observed_behavior,
    );
    require_non_empty(
        errors,
        "bug_report.expected_behavior",
        &contract.bug_report.expected_behavior,
    );
    require_vec(
        errors,
        "bug_report.reproduction_steps",
        &contract.bug_report.reproduction_steps,
    );
    if contract
        .bug_report
        .core
        .proposed_destination
        .destination_kind
        != ProposedDestinationKind::ProductBugBacklog
    {
        errors.push(error(
            "bug_report.proposed_destination",
            "bug reports must route to the product bug backlog",
        ));
    }
}

fn validate_gap_report(
    errors: &mut Vec<ValidatorFindingReportError>,
    contract: &ValidatorFindingReportsContractV1,
) {
    validate_core(
        errors,
        contract,
        "gap_report",
        &contract.gap_report.core,
        ValidatorFindingReportKind::Gap,
    );
    require_non_empty(
        errors,
        "gap_report.gap_statement",
        &contract.gap_report.gap_statement,
    );
    require_vec(
        errors,
        "gap_report.missing_contract_fields",
        &contract.gap_report.missing_contract_fields,
    );
    if contract
        .gap_report
        .core
        .proposed_destination
        .destination_kind
        != ProposedDestinationKind::SpecGapQueue
    {
        errors.push(error(
            "gap_report.proposed_destination",
            "gap reports must route to the spec gap queue",
        ));
    }
}

fn validate_out_of_scope_report(
    errors: &mut Vec<ValidatorFindingReportError>,
    contract: &ValidatorFindingReportsContractV1,
) {
    validate_core(
        errors,
        contract,
        "out_of_scope_report",
        &contract.out_of_scope_report.core,
        ValidatorFindingReportKind::OutOfScope,
    );
    require_non_empty(
        errors,
        "out_of_scope_report.rejected_scope_statement",
        &contract.out_of_scope_report.rejected_scope_statement,
    );
    require_non_empty(
        errors,
        "out_of_scope_report.parking_reason",
        &contract.out_of_scope_report.parking_reason,
    );
    if ![
        ProposedDestinationKind::OutOfScopeParkingLot,
        ProposedDestinationKind::OperatorDecision,
    ]
    .contains(
        &contract
            .out_of_scope_report
            .core
            .proposed_destination
            .destination_kind,
    ) {
        errors.push(error(
            "out_of_scope_report.proposed_destination",
            "out-of-scope reports must route to parking or operator decision",
        ));
    }
    if ![
        FindingRoutingOutcome::ParkOutOfScope,
        FindingRoutingOutcome::EscalateToOperator,
    ]
    .contains(&contract.out_of_scope_report.core.routing_outcome)
    {
        errors.push(error(
            "out_of_scope_report.routing_outcome",
            "out-of-scope reports must park or escalate",
        ));
    }
}

fn validate_core(
    errors: &mut Vec<ValidatorFindingReportError>,
    contract: &ValidatorFindingReportsContractV1,
    prefix: &str,
    core: &ValidatorFindingReportCoreV1,
    expected_kind: ValidatorFindingReportKind,
) {
    require_non_empty(errors, &format!("{prefix}.report_id"), &core.report_id);
    if core.wp_id != contract.wp_id {
        errors.push(error(
            &format!("{prefix}.wp_id"),
            "finding report wp_id must match contract wp_id",
        ));
    }
    if core.mt_id != contract.mt_id {
        errors.push(error(
            &format!("{prefix}.mt_id"),
            "finding report mt_id must match contract mt_id",
        ));
    }
    if core.report_kind != expected_kind {
        errors.push(error(
            &format!("{prefix}.report_kind"),
            "finding report kind must match its contract wrapper",
        ));
    }
    require_non_empty(
        errors,
        &format!("{prefix}.validator_role"),
        &core.validator_role,
    );
    require_non_empty(
        errors,
        &format!("{prefix}.validator_session"),
        &core.validator_session,
    );
    require_non_empty(
        errors,
        &format!("{prefix}.validator_reasoning"),
        &core.validator_reasoning,
    );
    validate_source_refs(errors, prefix, &core.source_refs);
    validate_affected_surfaces(errors, prefix, &core.affected_surfaces);
    validate_proof_refs(errors, prefix, &core.reproduction_or_proof);
    validate_destination(errors, prefix, &core.proposed_destination);
    if core.authority_mode != ReportAuthorityMode::MachineContractOnly {
        errors.push(error(
            &format!("{prefix}.authority_mode"),
            "validator finding reports must be machine contracts, not prose authority",
        ));
    }
    if core.status_mutation_allowed {
        errors.push(error(
            &format!("{prefix}.status_mutation_allowed"),
            "finding reports must not directly mutate status fields",
        ));
    }
}

fn validate_source_refs(
    errors: &mut Vec<ValidatorFindingReportError>,
    prefix: &str,
    source_refs: &[FindingSourceRefV1],
) {
    require_vec(errors, &format!("{prefix}.source_refs"), source_refs);
    for source in source_refs {
        require_non_empty(errors, &format!("{prefix}.source_refs"), &source.source_ref);
        require_non_empty(
            errors,
            &format!("{prefix}.source_hash"),
            &source.source_hash,
        );
    }
}

fn validate_affected_surfaces(
    errors: &mut Vec<ValidatorFindingReportError>,
    prefix: &str,
    surfaces: &[AffectedSurfaceRefV1],
) {
    require_vec(errors, &format!("{prefix}.affected_surfaces"), surfaces);
    for surface in surfaces {
        require_non_empty(
            errors,
            &format!("{prefix}.affected_surfaces"),
            &surface.surface_ref,
        );
    }
}

fn validate_proof_refs(
    errors: &mut Vec<ValidatorFindingReportError>,
    prefix: &str,
    proof_refs: &[FindingProofRefV1],
) {
    require_vec(
        errors,
        &format!("{prefix}.reproduction_or_proof"),
        proof_refs,
    );
    for proof in proof_refs {
        require_non_empty(
            errors,
            &format!("{prefix}.reproduction_or_proof"),
            &proof.proof_ref,
        );
        require_non_empty(
            errors,
            &format!("{prefix}.proof_result_ref"),
            &proof.result_ref,
        );
    }
}

fn validate_destination(
    errors: &mut Vec<ValidatorFindingReportError>,
    prefix: &str,
    destination: &ProposedFindingDestinationV1,
) {
    require_non_empty(
        errors,
        &format!("{prefix}.proposed_destination"),
        &destination.destination_ref,
    );
    require_non_empty(
        errors,
        &format!("{prefix}.proposed_destination.owner_role"),
        &destination.owner_role,
    );
}

fn validate_failure_states(
    errors: &mut Vec<ValidatorFindingReportError>,
    failure_states: &[ValidatorFindingReportFailureState],
) {
    for required in required_failure_states() {
        if !failure_states.contains(&required) {
            errors.push(error(
                "failure_states",
                "validator finding report failure states must cover reasoning, refs, proof, destination, routing, prose authority, kind, and scope hazards",
            ));
        }
    }
}

fn validate_research_basis(
    errors: &mut Vec<ValidatorFindingReportError>,
    research_basis_refs: &[ValidatorFindingReportResearchBasisV1],
) {
    require_vec(errors, "research_basis_refs", research_basis_refs);
    for required in [
        "docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms",
        "docs.github.com/en/code-security/reference/code-scanning/sarif-files/sarif-support-for-code-scanning",
        "docs.gitlab.com/user/work_items",
        "kubernetes.io/docs/reference/using-api/api-concepts",
    ] {
        if !research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(required))
        {
            errors.push(error(
                "research_basis_refs",
                "current issue-form, SARIF, work-item, and resource-version patterns must be recorded",
            ));
        }
    }
}

fn core(
    wp_id: &str,
    mt_id: &str,
    report_id: &str,
    report_kind: ValidatorFindingReportKind,
    validator_reasoning: &str,
    destination_kind: ProposedDestinationKind,
    destination_ref: &str,
    owner_role: &str,
    routing_outcome: FindingRoutingOutcome,
) -> ValidatorFindingReportCoreV1 {
    ValidatorFindingReportCoreV1 {
        report_id: report_id.to_string(),
        wp_id: wp_id.to_string(),
        mt_id: mt_id.to_string(),
        report_kind,
        validator_role: "INTEGRATION_VALIDATOR".to_string(),
        validator_session: "role-session://INTEGRATION_VALIDATOR/current".to_string(),
        validator_reasoning: validator_reasoning.to_string(),
        source_refs: vec![
            source(
                "validator-verdict://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-057",
                FindingSourceKind::ValidatorVerdict,
            ),
            source(
                "microtask-contract://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-058",
                FindingSourceKind::MicrotaskContract,
            ),
        ],
        affected_surfaces: vec![
            surface(
                "src/backend/handshake_core/src/kernel/**",
                AffectedSurfaceKind::ProductCode,
            ),
            surface(
                "src/backend/handshake_core/tests/**",
                AffectedSurfaceKind::ProductTest,
            ),
        ],
        reproduction_or_proof: vec![
            proof(
                FindingProofKind::ProofCommand,
                "cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test kernel_validator_finding_report_contract_tests",
            ),
            proof(
                FindingProofKind::ArtifactReference,
                "receipt://MT-058/validator-finding-report",
            ),
        ],
        proposed_destination: ProposedFindingDestinationV1 {
            destination_kind,
            destination_ref: destination_ref.to_string(),
            owner_role: owner_role.to_string(),
        },
        routing_outcome,
        authority_mode: ReportAuthorityMode::MachineContractOnly,
        status_mutation_allowed: false,
    }
}

fn source(source_ref: &str, source_kind: FindingSourceKind) -> FindingSourceRefV1 {
    FindingSourceRefV1 {
        source_ref: source_ref.to_string(),
        source_kind,
        source_hash: format!("sha256:{source_ref}:mt058"),
    }
}

fn surface(surface_ref: &str, surface_kind: AffectedSurfaceKind) -> AffectedSurfaceRefV1 {
    AffectedSurfaceRefV1 {
        surface_ref: surface_ref.to_string(),
        surface_kind,
    }
}

fn proof(proof_kind: FindingProofKind, proof_ref: &str) -> FindingProofRefV1 {
    FindingProofRefV1 {
        proof_kind,
        proof_ref: proof_ref.to_string(),
        result_ref: format!("finding-proof-result://{proof_ref}"),
    }
}

fn required_failure_states() -> Vec<ValidatorFindingReportFailureState> {
    vec![
        ValidatorFindingReportFailureState::MissingValidatorReasoning,
        ValidatorFindingReportFailureState::MissingSourceRefs,
        ValidatorFindingReportFailureState::MissingAffectedSurfaces,
        ValidatorFindingReportFailureState::MissingReproductionOrProof,
        ValidatorFindingReportFailureState::MissingProposedDestination,
        ValidatorFindingReportFailureState::MissingRoutingOutcome,
        ValidatorFindingReportFailureState::ProseOnlyReportAuthority,
        ValidatorFindingReportFailureState::ReportKindMismatch,
        ValidatorFindingReportFailureState::OutOfScopeWithoutParkingDestination,
    ]
}

fn research(
    source_ref: &str,
    pattern_found: &str,
    selected_reuse: &str,
) -> ValidatorFindingReportResearchBasisV1 {
    ValidatorFindingReportResearchBasisV1 {
        source_ref: source_ref.to_string(),
        pattern_found: pattern_found.to_string(),
        selected_reuse: selected_reuse.to_string(),
    }
}

fn error(field: &str, message: &str) -> ValidatorFindingReportError {
    ValidatorFindingReportError {
        field: field.to_string(),
        message: message.to_string(),
    }
}

fn require_non_empty(errors: &mut Vec<ValidatorFindingReportError>, field: &str, value: &str) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(errors: &mut Vec<ValidatorFindingReportError>, field: &str, values: &[T]) {
    if values.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}
