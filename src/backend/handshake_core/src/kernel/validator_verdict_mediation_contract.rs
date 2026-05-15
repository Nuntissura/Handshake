pub const VALIDATOR_VERDICT_MEDIATION_SCHEMA_ID: &str = "hsk.kernel.validator_verdict_mediation@1";
pub const VALIDATOR_VERDICT_MEDIATION_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.validator_verdict_mediation_projection@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidatorVerdictKind {
    Pass,
    Fail,
    MediationRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerdictRoutingOutcome {
    MayAdvance,
    MustLoopBack,
    MustEscalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerdictSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReproducibilityKind {
    Deterministic,
    Intermittent,
    NotReproduced,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DependencyImpactKind {
    None,
    BlocksDependents,
    RequiresRemediationBeforeDependents,
    EscalatesPacket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediationInstructionKind {
    Repair,
    ClarifyScope,
    AddEvidence,
    Escalate,
    Recheck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidatorVerdictFailureState {
    MissingVerdict,
    MissingFailedAcceptanceCriteria,
    MissingEvidence,
    MissingSeverity,
    MissingReproducibility,
    MissingRemediationInstructions,
    MissingDependencyImpact,
    MissingRoutingOutcome,
    PassWithFailedCriteria,
    FailWithoutMediation,
    AdvanceWithCriticalFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorVerdictActorV1 {
    pub actor_role: String,
    pub actor_session: String,
    pub review_batch_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorDependencyImpactV1 {
    pub kind: DependencyImpactKind,
    pub impacted_refs: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorVerdictContractV1 {
    pub verdict_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub reviewed_handoff_ref: String,
    pub validator_actor: ValidatorVerdictActorV1,
    pub verdict: ValidatorVerdictKind,
    pub failed_acceptance_criteria: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub severity: VerdictSeverity,
    pub reproducibility: ReproducibilityKind,
    pub dependency_impact: ValidatorDependencyImpactV1,
    pub routing_outcome: VerdictRoutingOutcome,
    pub receipt_kind: String,
    pub named_verb: String,
    pub status_mutation_allowed: bool,
    pub generated_without_model_status_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediationInstructionContractV1 {
    pub instruction_id: String,
    pub instruction_kind: MediationInstructionKind,
    pub target_role: String,
    pub target_session_ref: String,
    pub exact_remediation_steps: Vec<String>,
    pub recheck_required: bool,
    pub recheck_action_id: String,
    pub escalation_ref: String,
    pub creates_remediation_microtask: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorVerdictResearchBasisV1 {
    pub source_ref: String,
    pub pattern_found: String,
    pub selected_reuse: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorVerdictMediationContractV1 {
    pub schema_id: &'static str,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub supported_verdicts: Vec<ValidatorVerdictKind>,
    pub supported_routing_outcomes: Vec<VerdictRoutingOutcome>,
    pub validator_verdict: ValidatorVerdictContractV1,
    pub mediation_instruction: MediationInstructionContractV1,
    pub failure_states: Vec<ValidatorVerdictFailureState>,
    pub research_basis_refs: Vec<ValidatorVerdictResearchBasisV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorVerdictMediationProjectionV1 {
    pub schema_id: String,
    pub source_contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub receipt_kind: String,
    pub named_verb: String,
    pub status_mutation_allowed: bool,
    pub mt_may_advance: bool,
    pub mt_must_loop_back: bool,
    pub mt_must_escalate: bool,
    pub failed_acceptance_criteria: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub remediation_steps: Vec<String>,
    pub severity: VerdictSeverity,
    pub reproducibility: ReproducibilityKind,
    pub dependency_impact: DependencyImpactKind,
    pub routing_outcome: VerdictRoutingOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorVerdictMediationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_validator_verdict_mediation_contract() -> ValidatorVerdictMediationContractV1
{
    let wp_id = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";
    let mt_id = "MT-057";

    ValidatorVerdictMediationContractV1 {
        schema_id: VALIDATOR_VERDICT_MEDIATION_SCHEMA_ID,
        contract_id: "kernel002-validator-verdict-mediation-mt057".to_string(),
        wp_id: wp_id.to_string(),
        mt_id: mt_id.to_string(),
        supported_verdicts: vec![
            ValidatorVerdictKind::Pass,
            ValidatorVerdictKind::Fail,
            ValidatorVerdictKind::MediationRequired,
        ],
        supported_routing_outcomes: vec![
            VerdictRoutingOutcome::MayAdvance,
            VerdictRoutingOutcome::MustLoopBack,
            VerdictRoutingOutcome::MustEscalate,
        ],
        validator_verdict: ValidatorVerdictContractV1 {
            verdict_id: "kernel002-mt057-validator-verdict".to_string(),
            wp_id: wp_id.to_string(),
            mt_id: mt_id.to_string(),
            reviewed_handoff_ref: "review-request://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-056".to_string(),
            validator_actor: ValidatorVerdictActorV1 {
                actor_role: "INTEGRATION_VALIDATOR".to_string(),
                actor_session: "role-session://INTEGRATION_VALIDATOR/current".to_string(),
                review_batch_ref: "validation-batch://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
            },
            verdict: ValidatorVerdictKind::Fail,
            failed_acceptance_criteria: vec![
                "MT-057-AC-001: verdict must include failed acceptance criteria and evidence refs".to_string(),
                "MT-057-AC-002: verdict must include severity, reproducibility, dependency impact, and routing outcome".to_string(),
            ],
            evidence_refs: vec![
                "test-output://kernel_validator_verdict_mediation_contract_tests".to_string(),
                "test-output://validator_verdict_mediation_contract_harness".to_string(),
                "receipt://MT-056/review-request/projection".to_string(),
            ],
            severity: VerdictSeverity::High,
            reproducibility: ReproducibilityKind::Deterministic,
            dependency_impact: ValidatorDependencyImpactV1 {
                kind: DependencyImpactKind::BlocksDependents,
                impacted_refs: vec!["MT-058".to_string(), "MT-059".to_string()],
                summary: "Dependent MTs must wait until the failed acceptance criteria have exact remediation and recheck evidence.".to_string(),
            },
            routing_outcome: VerdictRoutingOutcome::MustLoopBack,
            receipt_kind: "VALIDATOR_REVIEW".to_string(),
            named_verb: "MT_VERDICT".to_string(),
            status_mutation_allowed: false,
            generated_without_model_status_edit: true,
        },
        mediation_instruction: MediationInstructionContractV1 {
            instruction_id: "kernel002-mt057-mediation-instruction".to_string(),
            instruction_kind: MediationInstructionKind::Repair,
            target_role: "CODER".to_string(),
            target_session_ref: "role-session://CODER/current".to_string(),
            exact_remediation_steps: vec![
                "Add the missing validator verdict fields to the microtask contract or implementation.".to_string(),
                "Attach focused product and proof-harness evidence refs for the repaired behavior.".to_string(),
                "Re-run validator verdict mediation focused tests before requesting another review.".to_string(),
            ],
            recheck_required: true,
            recheck_action_id: "kernel.validator_verdict_mediation.project".to_string(),
            escalation_ref: "escalation://kernel-builder/validator-verdict-mediation".to_string(),
            creates_remediation_microtask: false,
        },
        failure_states: required_failure_states(),
        research_basis_refs: vec![
            research(
                "https://docs.github.com/en/rest/checks/runs?apiVersion=2022-11-28",
                "Check runs separate status, conclusion, details URL, output summary, and annotations.",
                "Validator verdicts should keep conclusion, evidence, and details refs as typed fields.",
            ),
            research(
                "https://docs.gitlab.com/ci/testing/unit_test_reports/",
                "Unit test report UIs expose failed test names so failures can be rerun locally.",
                "Failed acceptance criteria should be machine-addressable enough to drive exact rechecks.",
            ),
            research(
                "https://gerrit-review.googlesource.com/Documentation/config-submit-requirements.html",
                "Submit requirements encode review gates and whether a change is submittable.",
                "Handshake verdict routing should distinguish advance, loopback, and escalation gates.",
            ),
            research(
                "https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-conditions",
                "Conditions carry typed status, reason, and transition metadata.",
                "Validator mediation should carry typed reason-like failure states and reproducibility.",
            ),
        ],
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.task_contract_lifecycle".to_string(),
            "kernel.coder_handoff_validation_request".to_string(),
            "kernel.generated_documentation_status_projection".to_string(),
        ],
        folded_source_refs: vec![
            "MT-057 Validator Verdict and Mediation Contract".to_string(),
            "workflow-transition-registry".to_string(),
            "task-contract-lifecycle".to_string(),
            "coder-handoff-validation-request".to_string(),
        ],
    }
}

pub fn validate_validator_verdict_mediation_contract(
    contract: &ValidatorVerdictMediationContractV1,
) -> Result<(), Vec<ValidatorVerdictMediationError>> {
    let mut errors = Vec::new();

    if contract.schema_id != VALIDATOR_VERDICT_MEDIATION_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "validator verdict mediation schema id is required",
        ));
    }
    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    validate_supported_verdicts(&mut errors, contract);
    validate_supported_routing(&mut errors, contract);
    validate_verdict(&mut errors, contract);
    validate_mediation(&mut errors, contract);
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

pub fn project_validator_verdict_mediation(
    contract: &ValidatorVerdictMediationContractV1,
) -> Result<ValidatorVerdictMediationProjectionV1, Vec<ValidatorVerdictMediationError>> {
    validate_validator_verdict_mediation_contract(contract)?;

    let routing = contract.validator_verdict.routing_outcome;

    Ok(ValidatorVerdictMediationProjectionV1 {
        schema_id: VALIDATOR_VERDICT_MEDIATION_PROJECTION_SCHEMA_ID.to_string(),
        source_contract_id: contract.contract_id.clone(),
        wp_id: contract.wp_id.clone(),
        mt_id: contract.mt_id.clone(),
        receipt_kind: contract.validator_verdict.receipt_kind.clone(),
        named_verb: contract.validator_verdict.named_verb.clone(),
        status_mutation_allowed: contract.validator_verdict.status_mutation_allowed,
        mt_may_advance: routing == VerdictRoutingOutcome::MayAdvance,
        mt_must_loop_back: routing == VerdictRoutingOutcome::MustLoopBack,
        mt_must_escalate: routing == VerdictRoutingOutcome::MustEscalate,
        failed_acceptance_criteria: contract
            .validator_verdict
            .failed_acceptance_criteria
            .clone(),
        evidence_refs: contract.validator_verdict.evidence_refs.clone(),
        remediation_steps: contract
            .mediation_instruction
            .exact_remediation_steps
            .clone(),
        severity: contract.validator_verdict.severity,
        reproducibility: contract.validator_verdict.reproducibility,
        dependency_impact: contract.validator_verdict.dependency_impact.kind,
        routing_outcome: routing,
    })
}

fn validate_supported_verdicts(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    contract: &ValidatorVerdictMediationContractV1,
) {
    for verdict in [
        ValidatorVerdictKind::Pass,
        ValidatorVerdictKind::Fail,
        ValidatorVerdictKind::MediationRequired,
    ] {
        if !contract.supported_verdicts.contains(&verdict) {
            errors.push(error(
                "supported_verdicts",
                "pass, fail, and mediation-required verdicts must be encoded",
            ));
        }
    }
}

fn validate_supported_routing(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    contract: &ValidatorVerdictMediationContractV1,
) {
    for routing in [
        VerdictRoutingOutcome::MayAdvance,
        VerdictRoutingOutcome::MustLoopBack,
        VerdictRoutingOutcome::MustEscalate,
    ] {
        if !contract.supported_routing_outcomes.contains(&routing) {
            errors.push(error(
                "supported_routing_outcomes",
                "advance, loopback, and escalation outcomes must be encoded",
            ));
        }
    }
}

fn validate_verdict(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    contract: &ValidatorVerdictMediationContractV1,
) {
    let verdict = &contract.validator_verdict;

    require_non_empty(errors, "validator_verdict.verdict_id", &verdict.verdict_id);
    if verdict.wp_id != contract.wp_id {
        errors.push(error(
            "validator_verdict.wp_id",
            "verdict wp_id must match contract wp_id",
        ));
    }
    if verdict.mt_id != contract.mt_id {
        errors.push(error(
            "validator_verdict.mt_id",
            "verdict mt_id must match contract mt_id",
        ));
    }
    require_non_empty(
        errors,
        "validator_verdict.reviewed_handoff_ref",
        &verdict.reviewed_handoff_ref,
    );
    validate_actor(errors, &verdict.validator_actor);
    validate_verdict_payload(errors, verdict);
    validate_dependency_impact(errors, verdict);
    validate_routing(errors, verdict);
    validate_status_boundary(errors, verdict);
}

fn validate_actor(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    actor: &ValidatorVerdictActorV1,
) {
    if !["WP_VALIDATOR", "INTEGRATION_VALIDATOR", "KERNEL_BUILDER"]
        .contains(&actor.actor_role.as_str())
    {
        errors.push(error(
            "validator_actor.actor_role",
            "validator verdict actor must be an allowed validator role",
        ));
    }
    require_non_empty(
        errors,
        "validator_actor.actor_session",
        &actor.actor_session,
    );
    require_non_empty(
        errors,
        "validator_actor.review_batch_ref",
        &actor.review_batch_ref,
    );
}

fn validate_verdict_payload(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    verdict: &ValidatorVerdictContractV1,
) {
    if verdict.verdict == ValidatorVerdictKind::Pass
        && !verdict.failed_acceptance_criteria.is_empty()
    {
        errors.push(error(
            "validator_verdict.failed_acceptance_criteria",
            "pass verdicts cannot carry failed acceptance criteria",
        ));
    }
    if verdict_requires_mediation(verdict.verdict) {
        require_vec(
            errors,
            "validator_verdict.failed_acceptance_criteria",
            &verdict.failed_acceptance_criteria,
        );
    }
    require_vec(
        errors,
        "validator_verdict.evidence_refs",
        &verdict.evidence_refs,
    );
    if verdict_requires_mediation(verdict.verdict) && verdict.severity == VerdictSeverity::Info {
        errors.push(error(
            "validator_verdict.severity",
            "failed verdicts require non-info severity",
        ));
    }
    if verdict_requires_mediation(verdict.verdict)
        && verdict.reproducibility == ReproducibilityKind::NotApplicable
    {
        errors.push(error(
            "validator_verdict.reproducibility",
            "failed verdicts require reproducibility posture",
        ));
    }
    for evidence_ref in &verdict.evidence_refs {
        require_non_empty(errors, "validator_verdict.evidence_refs", evidence_ref);
    }
}

fn validate_dependency_impact(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    verdict: &ValidatorVerdictContractV1,
) {
    require_non_empty(
        errors,
        "validator_verdict.dependency_impact.summary",
        &verdict.dependency_impact.summary,
    );
    if verdict_requires_mediation(verdict.verdict)
        && verdict.dependency_impact.kind == DependencyImpactKind::None
    {
        errors.push(error(
            "validator_verdict.dependency_impact",
            "failed verdicts require an explicit dependency impact",
        ));
    }
    if verdict.dependency_impact.kind != DependencyImpactKind::None {
        require_vec(
            errors,
            "validator_verdict.dependency_impact.impacted_refs",
            &verdict.dependency_impact.impacted_refs,
        );
    }
}

fn validate_routing(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    verdict: &ValidatorVerdictContractV1,
) {
    if verdict.verdict == ValidatorVerdictKind::Pass
        && verdict.routing_outcome != VerdictRoutingOutcome::MayAdvance
    {
        errors.push(error(
            "validator_verdict.routing_outcome",
            "pass verdicts should allow the MT to advance",
        ));
    }
    if verdict_requires_mediation(verdict.verdict)
        && verdict.routing_outcome == VerdictRoutingOutcome::MayAdvance
    {
        errors.push(error(
            "validator_verdict.routing_outcome",
            "failed or mediated verdicts cannot advance directly",
        ));
    }
    if verdict.severity == VerdictSeverity::Critical
        && verdict.routing_outcome == VerdictRoutingOutcome::MayAdvance
    {
        errors.push(error(
            "validator_verdict.routing_outcome",
            "critical verdicts cannot advance",
        ));
    }
    if verdict.dependency_impact.kind == DependencyImpactKind::EscalatesPacket
        && verdict.routing_outcome != VerdictRoutingOutcome::MustEscalate
    {
        errors.push(error(
            "validator_verdict.routing_outcome",
            "packet-escalating dependency impact must route to escalation",
        ));
    }
    if verdict.receipt_kind != "VALIDATOR_REVIEW" && verdict.receipt_kind != "REVIEW_RESPONSE" {
        errors.push(error(
            "validator_verdict.receipt_kind",
            "validator verdict receipt must be VALIDATOR_REVIEW or REVIEW_RESPONSE",
        ));
    }
    if verdict.named_verb != "MT_VERDICT" {
        errors.push(error(
            "validator_verdict.named_verb",
            "validator verdicts must use MT_VERDICT",
        ));
    }
}

fn validate_status_boundary(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    verdict: &ValidatorVerdictContractV1,
) {
    if verdict.status_mutation_allowed {
        errors.push(error(
            "validator_verdict.status_mutation_allowed",
            "verdict projection must not directly mutate status fields",
        ));
    }
    if !verdict.generated_without_model_status_edit {
        errors.push(error(
            "validator_verdict.generated_without_model_status_edit",
            "verdict projection must be generated without model status edits",
        ));
    }
}

fn validate_mediation(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    contract: &ValidatorVerdictMediationContractV1,
) {
    let mediation = &contract.mediation_instruction;
    require_non_empty(
        errors,
        "mediation_instruction.instruction_id",
        &mediation.instruction_id,
    );
    require_non_empty(
        errors,
        "mediation_instruction.target_role",
        &mediation.target_role,
    );
    require_non_empty(
        errors,
        "mediation_instruction.target_session_ref",
        &mediation.target_session_ref,
    );
    require_non_empty(
        errors,
        "mediation_instruction.recheck_action_id",
        &mediation.recheck_action_id,
    );
    if verdict_requires_mediation(contract.validator_verdict.verdict) {
        require_vec(
            errors,
            "mediation_instruction.exact_remediation_steps",
            &mediation.exact_remediation_steps,
        );
        if !mediation.recheck_required {
            errors.push(error(
                "mediation_instruction.recheck_required",
                "failed verdict mediation must require recheck",
            ));
        }
    }
    if contract.validator_verdict.routing_outcome == VerdictRoutingOutcome::MustEscalate
        && mediation.instruction_kind != MediationInstructionKind::Escalate
    {
        errors.push(error(
            "mediation_instruction.instruction_kind",
            "escalation routing must use escalation mediation instruction",
        ));
    }
    if contract.validator_verdict.routing_outcome == VerdictRoutingOutcome::MustLoopBack
        && mediation.target_role != "CODER"
    {
        errors.push(error(
            "mediation_instruction.target_role",
            "loopback verdicts route exact repair instructions to CODER",
        ));
    }
    for step in &mediation.exact_remediation_steps {
        require_non_empty(
            errors,
            "mediation_instruction.exact_remediation_steps",
            step,
        );
    }
}

fn validate_failure_states(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    failure_states: &[ValidatorVerdictFailureState],
) {
    for required in required_failure_states() {
        if !failure_states.contains(&required) {
            errors.push(error(
                "failure_states",
                "validator verdict failure states must cover verdict, evidence, routing, mediation, and critical-advance hazards",
            ));
        }
    }
}

fn validate_research_basis(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    research_basis_refs: &[ValidatorVerdictResearchBasisV1],
) {
    require_vec(errors, "research_basis_refs", research_basis_refs);
    for required in [
        "docs.github.com/en/rest/checks/runs",
        "docs.gitlab.com/ci/testing/unit_test_reports",
        "gerrit-review.googlesource.com/Documentation/config-submit-requirements",
        "kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle",
    ] {
        if !research_basis_refs
            .iter()
            .any(|basis| basis.source_ref.contains(required))
        {
            errors.push(error(
                "research_basis_refs",
                "current verdict, test-report, submit-requirement, and typed-condition patterns must be recorded",
            ));
        }
    }
}

fn verdict_requires_mediation(verdict: ValidatorVerdictKind) -> bool {
    matches!(
        verdict,
        ValidatorVerdictKind::Fail | ValidatorVerdictKind::MediationRequired
    )
}

fn required_failure_states() -> Vec<ValidatorVerdictFailureState> {
    vec![
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
    ]
}

fn research(
    source_ref: &str,
    pattern_found: &str,
    selected_reuse: &str,
) -> ValidatorVerdictResearchBasisV1 {
    ValidatorVerdictResearchBasisV1 {
        source_ref: source_ref.to_string(),
        pattern_found: pattern_found.to_string(),
        selected_reuse: selected_reuse.to_string(),
    }
}

fn error(field: &'static str, message: &'static str) -> ValidatorVerdictMediationError {
    ValidatorVerdictMediationError { field, message }
}

fn require_non_empty(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<ValidatorVerdictMediationError>,
    field: &'static str,
    values: &[T],
) {
    if values.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}
