use serde::{Deserialize, Serialize};

use super::crdt::persistence::sha256_hex;
pub const REMEDIATION_WORK_GENERATION_SCHEMA_ID: &str = "hsk.kernel.remediation_work_generation@1";
pub const REMEDIATION_WORK_GENERATION_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.remediation_work_generation_projection@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemediationWorkKind {
    MicroTask,
    PacketStub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemediationDestinationKind {
    SamePacketMicrotask,
    NewPacketStub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemediationGenerationSourceKind {
    ValidatorVerdictContract,
    ValidatorFindingReportContract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RemediationGenerationFailureState {
    MissingParentLinks,
    MissingDependencyState,
    MissingAcceptanceCriteria,
    MissingAllowedActions,
    MissingWriteBoxes,
    MissingEvidenceRefs,
    MissingRetryBudget,
    MissingValidatorRecheck,
    ProseSourceUsed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationSourceRefV1 {
    pub source_ref: String,
    pub source_kind: RemediationGenerationSourceKind,
    pub source_hash: String,
    pub prose_source: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationDependencyStateV1 {
    pub blocked_by_refs: Vec<String>,
    pub unlocks_refs: Vec<String>,
    pub blocks_dependents: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationRetryBudgetV1 {
    pub max_attempts: u32,
    pub attempts_consumed: u32,
    pub terminal_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorRecheckRequirementV1 {
    pub required: bool,
    pub validator_role: String,
    pub recheck_action_id: String,
    pub evidence_required: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationMicroTaskContractV1 {
    pub contract_id: String,
    pub parent_wp_id: String,
    pub parent_mt_refs: Vec<String>,
    pub dependency_state: RemediationDependencyStateV1,
    pub acceptance_criteria: Vec<String>,
    pub allowed_action_ids: Vec<String>,
    pub write_box_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub retry_budget: RemediationRetryBudgetV1,
    pub validator_recheck: ValidatorRecheckRequirementV1,
    pub generated_from_contracts_only: bool,
    pub status_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationPacketStubContractV1 {
    pub stub_id: String,
    pub parent_wp_id: String,
    pub parent_report_refs: Vec<String>,
    pub proposed_packet_id: String,
    pub destination_kind: RemediationDestinationKind,
    pub acceptance_criteria: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub generated_from_contracts_only: bool,
}

pub type StubContractV1 = RemediationPacketStubContractV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationResearchBasisV1 {
    pub source_ref: String,
    pub pattern_found: String,
    pub selected_reuse: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationWorkGenerationContractV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub source_refs: Vec<RemediationSourceRefV1>,
    pub remediation_microtask: RemediationMicroTaskContractV1,
    pub remediation_packet_stub: RemediationPacketStubContractV1,
    pub failure_states: Vec<RemediationGenerationFailureState>,
    pub research_basis_refs: Vec<RemediationResearchBasisV1>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

pub type RemediationWorkContractV1 = RemediationWorkGenerationContractV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationWorkGenerationProjectionV1 {
    pub schema_id: String,
    pub source_contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub generated_work_kinds: Vec<RemediationWorkKind>,
    pub parent_mt_refs: Vec<String>,
    pub parent_report_refs: Vec<String>,
    pub dependency_blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub validator_recheck_required: bool,
    pub status_mutation_allowed: bool,
    pub prose_source_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationWorkGenerationError {
    pub field: String,
    pub message: String,
}

pub fn build_kernel002_remediation_work_generation() -> RemediationWorkGenerationContractV1 {
    let wp_id = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";
    let mt_id = "MT-059";

    RemediationWorkGenerationContractV1 {
        schema_id: REMEDIATION_WORK_GENERATION_SCHEMA_ID.to_string(),
        contract_id: format!("{wp_id}-{mt_id}-remediation-work-generation"),
        wp_id: wp_id.to_string(),
        mt_id: mt_id.to_string(),
        source_refs: vec![
            RemediationSourceRefV1 {
                source_ref: format!("validator-verdict-contract://{wp_id}/MT-057"),
                source_kind: RemediationGenerationSourceKind::ValidatorVerdictContract,
                source_hash: source_hash(
                    "kernel002-mt059",
                    &[
                        "validator-verdict-contract",
                        &format!("validator-verdict-contract://{wp_id}/MT-057"),
                    ],
                ),
                prose_source: false,
            },
            RemediationSourceRefV1 {
                source_ref: format!("validator-finding-report-contract://{wp_id}/MT-058"),
                source_kind: RemediationGenerationSourceKind::ValidatorFindingReportContract,
                source_hash: source_hash(
                    "kernel002-mt059",
                    &[
                        "validator-finding-report-contract",
                        &format!("validator-finding-report-contract://{wp_id}/MT-058"),
                    ],
                ),
                prose_source: false,
            },
        ],
        remediation_microtask: RemediationMicroTaskContractV1 {
            contract_id: format!("{wp_id}-{mt_id}-follow-up-microtask"),
            parent_wp_id: wp_id.to_string(),
            parent_mt_refs: vec!["MT-057".to_string(), "MT-058".to_string()],
            dependency_state: RemediationDependencyStateV1 {
                blocked_by_refs: vec!["MT-057".to_string(), "MT-058".to_string()],
                unlocks_refs: vec!["MT-060".to_string(), "MT-061".to_string()],
                blocks_dependents: true,
            },
            acceptance_criteria: vec![
                "Generated remediation microtask preserves parent WP and MT links."
                    .to_string(),
                "Generated remediation microtask includes dependency state and retry budget."
                    .to_string(),
                "Generated remediation microtask requires validator recheck evidence before closeout."
                    .to_string(),
            ],
            allowed_action_ids: vec![
                "kernel.validator_verdict_mediation.project".to_string(),
                "kernel.validator_finding_reports.project".to_string(),
                "kernel.remediation_work_generation.project".to_string(),
            ],
            write_box_refs: vec![
                "hsk.write_box.remediation_microtask_contract@1".to_string(),
                "hsk.write_box.readonly_projection@1".to_string(),
            ],
            evidence_refs: vec![
                "validator-verdict-contract://kernel002/MT-057/evidence".to_string(),
                "validator-finding-report-contract://kernel002/MT-058/evidence".to_string(),
            ],
            retry_budget: RemediationRetryBudgetV1 {
                max_attempts: 2,
                attempts_consumed: 0,
                terminal_action: "escalate-to-mediation-packet".to_string(),
            },
            validator_recheck: ValidatorRecheckRequirementV1 {
                required: true,
                validator_role: "INTEGRATION_VALIDATOR".to_string(),
                recheck_action_id: "kernel.validator_verdict_mediation.project".to_string(),
                evidence_required: vec![
                    "focused-test-output".to_string(),
                    "catalog-validation-output".to_string(),
                    "receipt-ref".to_string(),
                ],
            },
            generated_from_contracts_only: true,
            status_mutation_allowed: false,
        },
        remediation_packet_stub: RemediationPacketStubContractV1 {
            stub_id: format!("{wp_id}-{mt_id}-scope-expansion-stub"),
            parent_wp_id: wp_id.to_string(),
            parent_report_refs: vec![
                "validator-finding-report://kernel002-mt058-gap-report".to_string(),
                "validator-finding-report://kernel002-mt058-out-of-scope-report".to_string(),
            ],
            proposed_packet_id: "WP-KERNEL-002-REMEDIATION-FOLLOW-UP-STUB-v1".to_string(),
            destination_kind: RemediationDestinationKind::NewPacketStub,
            acceptance_criteria: vec![
                "Generated packet stub preserves parent report references.".to_string(),
                "Generated packet stub remains pending until operator or protocol promotion."
                    .to_string(),
            ],
            evidence_refs: vec![
                "validator-finding-report-contract://kernel002/MT-058/gap-report".to_string(),
                "validator-finding-report-contract://kernel002/MT-058/out-of-scope-report"
                    .to_string(),
            ],
            generated_from_contracts_only: true,
        },
        failure_states: vec![
            RemediationGenerationFailureState::MissingParentLinks,
            RemediationGenerationFailureState::MissingDependencyState,
            RemediationGenerationFailureState::MissingAcceptanceCriteria,
            RemediationGenerationFailureState::MissingAllowedActions,
            RemediationGenerationFailureState::MissingWriteBoxes,
            RemediationGenerationFailureState::MissingEvidenceRefs,
            RemediationGenerationFailureState::MissingRetryBudget,
            RemediationGenerationFailureState::MissingValidatorRecheck,
            RemediationGenerationFailureState::ProseSourceUsed,
        ],
        research_basis_refs: vec![
            RemediationResearchBasisV1 {
                source_ref: "https://docs.github.com/en/rest/issues/issues".to_string(),
                pattern_found: "Issue APIs preserve structured parent, assignee, label, and state references for follow-up work.".to_string(),
                selected_reuse: "Use stable source refs and projected destinations instead of prose-only remediation notes.".to_string(),
            },
            RemediationResearchBasisV1 {
                source_ref: "https://docs.gitlab.com/user/work_items/linked_items/".to_string(),
                pattern_found: "Linked work items model explicit dependency and relationship state between follow-up units.".to_string(),
                selected_reuse: "Carry blocked-by and unlocks refs into generated remediation work.".to_string(),
            },
            RemediationResearchBasisV1 {
                source_ref: "https://kubernetes.io/docs/concepts/overview/working-with-objects/owners-dependents/".to_string(),
                pattern_found: "Owner and dependent references make generated resources attributable and garbage-collectable.".to_string(),
                selected_reuse: "Preserve parent WP, MT, and validator report links on every generated work artifact.".to_string(),
            },
            RemediationResearchBasisV1 {
                source_ref: "https://docs.temporal.io/encyclopedia/retry-policies".to_string(),
                pattern_found: "Retry policies bound repeated work and define terminal handling when attempts are exhausted.".to_string(),
                selected_reuse: "Require retry budget and validator recheck before remediation closeout.".to_string(),
            },
        ],
        product_authority_refs: vec![
            "kernel.validator_verdict_mediation_contract".to_string(),
            "kernel.validator_finding_report_contract".to_string(),
            "kernel.action_catalog".to_string(),
        ],
        folded_source_refs: vec![
            "MT-057 validator verdict mediation contract".to_string(),
            "MT-058 validator finding report contract".to_string(),
        ],
    }
}

fn source_hash(domain: &str, parts: &[&str]) -> String {
    format!(
        "sha256:{}",
        sha256_hex(format!("{domain}|{}", parts.join("|")).as_bytes())
    )
}

fn is_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

pub fn project_remediation_work_generation(
    contract: &RemediationWorkGenerationContractV1,
) -> Result<RemediationWorkGenerationProjectionV1, Vec<RemediationWorkGenerationError>> {
    validate_remediation_work_generation(contract)?;

    Ok(RemediationWorkGenerationProjectionV1 {
        schema_id: REMEDIATION_WORK_GENERATION_PROJECTION_SCHEMA_ID.to_string(),
        source_contract_id: contract.contract_id.clone(),
        wp_id: contract.wp_id.clone(),
        mt_id: contract.mt_id.clone(),
        generated_work_kinds: vec![
            RemediationWorkKind::MicroTask,
            RemediationWorkKind::PacketStub,
        ],
        parent_mt_refs: contract.remediation_microtask.parent_mt_refs.clone(),
        parent_report_refs: contract.remediation_packet_stub.parent_report_refs.clone(),
        dependency_blockers: contract
            .remediation_microtask
            .dependency_state
            .blocked_by_refs
            .clone(),
        evidence_refs: contract.remediation_microtask.evidence_refs.clone(),
        validator_recheck_required: contract.remediation_microtask.validator_recheck.required,
        status_mutation_allowed: contract.remediation_microtask.status_mutation_allowed,
        prose_source_allowed: false,
    })
}

pub fn validate_remediation_work_generation(
    contract: &RemediationWorkGenerationContractV1,
) -> Result<(), Vec<RemediationWorkGenerationError>> {
    let mut errors = Vec::new();
    require_value(
        &mut errors,
        "schema_id",
        contract.schema_id == REMEDIATION_WORK_GENERATION_SCHEMA_ID,
        "schema id must match remediation work generation contract",
    );
    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    require_non_empty_vec(&mut errors, "source_refs", &contract.source_refs);

    for source_ref in &contract.source_refs {
        if source_ref.prose_source {
            push_error(
                &mut errors,
                "source_refs.prose_source",
                "prose sources cannot generate remediation work",
            );
        }
        require_non_empty(
            &mut errors,
            "source_refs.source_ref",
            &source_ref.source_ref,
        );
        require_non_empty(
            &mut errors,
            "source_refs.source_hash",
            &source_ref.source_hash,
        );
        if !is_sha256_digest(&source_ref.source_hash) {
            push_error(
                &mut errors,
                "source_refs.source_hash",
                "remediation source hashes must be sha256 digests",
            );
        }
    }

    validate_microtask_contract(&mut errors, &contract.remediation_microtask);
    validate_packet_stub_contract(&mut errors, &contract.remediation_packet_stub);
    require_non_empty_vec(&mut errors, "failure_states", &contract.failure_states);
    require_non_empty_vec(
        &mut errors,
        "research_basis_refs",
        &contract.research_basis_refs,
    );
    require_non_empty_vec(
        &mut errors,
        "product_authority_refs",
        &contract.product_authority_refs,
    );
    require_non_empty_vec(
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

fn validate_microtask_contract(
    errors: &mut Vec<RemediationWorkGenerationError>,
    microtask: &RemediationMicroTaskContractV1,
) {
    require_non_empty(
        errors,
        "remediation_microtask.contract_id",
        &microtask.contract_id,
    );
    require_non_empty(
        errors,
        "remediation_microtask.parent_wp_id",
        &microtask.parent_wp_id,
    );
    require_non_empty_vec(
        errors,
        "remediation_microtask.parent_mt_refs",
        &microtask.parent_mt_refs,
    );
    require_non_empty_vec(
        errors,
        "remediation_microtask.dependency_state.blocked_by_refs",
        &microtask.dependency_state.blocked_by_refs,
    );
    require_non_empty_vec(
        errors,
        "remediation_microtask.acceptance_criteria",
        &microtask.acceptance_criteria,
    );
    require_non_empty_vec(
        errors,
        "remediation_microtask.allowed_action_ids",
        &microtask.allowed_action_ids,
    );
    require_non_empty_vec(
        errors,
        "remediation_microtask.write_box_refs",
        &microtask.write_box_refs,
    );
    require_non_empty_vec(
        errors,
        "remediation_microtask.evidence_refs",
        &microtask.evidence_refs,
    );

    if microtask.retry_budget.max_attempts == 0
        || microtask.retry_budget.attempts_consumed > microtask.retry_budget.max_attempts
        || microtask.retry_budget.terminal_action.trim().is_empty()
    {
        push_error(
            errors,
            "remediation_microtask.retry_budget",
            "retry budget must bound attempts and define terminal action",
        );
    }

    if !microtask.validator_recheck.required
        || microtask.validator_recheck.validator_role.trim().is_empty()
        || microtask
            .validator_recheck
            .recheck_action_id
            .trim()
            .is_empty()
        || microtask.validator_recheck.evidence_required.is_empty()
    {
        push_error(
            errors,
            "remediation_microtask.validator_recheck",
            "validator recheck must be required with role, action, and evidence",
        );
    }

    if !microtask.generated_from_contracts_only {
        push_error(
            errors,
            "remediation_microtask.generated_from_contracts_only",
            "remediation microtasks must be generated from contracts only",
        );
    }

    if microtask.status_mutation_allowed {
        push_error(
            errors,
            "remediation_microtask.status_mutation_allowed",
            "projection cannot mutate task status",
        );
    }
}

fn validate_packet_stub_contract(
    errors: &mut Vec<RemediationWorkGenerationError>,
    stub: &RemediationPacketStubContractV1,
) {
    require_non_empty(errors, "remediation_packet_stub.stub_id", &stub.stub_id);
    require_non_empty(
        errors,
        "remediation_packet_stub.parent_wp_id",
        &stub.parent_wp_id,
    );
    require_non_empty_vec(
        errors,
        "remediation_packet_stub.parent_report_refs",
        &stub.parent_report_refs,
    );
    require_non_empty(
        errors,
        "remediation_packet_stub.proposed_packet_id",
        &stub.proposed_packet_id,
    );
    require_non_empty_vec(
        errors,
        "remediation_packet_stub.acceptance_criteria",
        &stub.acceptance_criteria,
    );
    require_non_empty_vec(
        errors,
        "remediation_packet_stub.evidence_refs",
        &stub.evidence_refs,
    );

    if stub.destination_kind != RemediationDestinationKind::NewPacketStub {
        push_error(
            errors,
            "remediation_packet_stub.destination_kind",
            "out-of-scope findings must generate a packet stub",
        );
    }

    if !stub.generated_from_contracts_only {
        push_error(
            errors,
            "remediation_packet_stub.generated_from_contracts_only",
            "packet stubs must be generated from contracts only",
        );
    }
}

fn require_non_empty(errors: &mut Vec<RemediationWorkGenerationError>, field: &str, value: &str) {
    if value.trim().is_empty() {
        push_error(errors, field, "field must not be empty");
    }
}

fn require_non_empty_vec<T>(
    errors: &mut Vec<RemediationWorkGenerationError>,
    field: &str,
    value: &[T],
) {
    if value.is_empty() {
        push_error(errors, field, "field must not be empty");
    }
}

fn require_value(
    errors: &mut Vec<RemediationWorkGenerationError>,
    field: &str,
    valid: bool,
    message: &str,
) {
    if !valid {
        push_error(errors, field, message);
    }
}

fn push_error(errors: &mut Vec<RemediationWorkGenerationError>, field: &str, message: &str) {
    errors.push(RemediationWorkGenerationError {
        field: field.to_string(),
        message: message.to_string(),
    });
}
