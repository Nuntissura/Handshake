use serde::{Deserialize, Serialize};

use super::validator_verdict_mediation_contract::{ValidatorVerdictKind, VerdictRoutingOutcome};

pub const MT_LOOP_SCHEDULER_SCHEMA_ID: &str = "hsk.kernel.mt_loop_scheduler@1";
pub const MT_LOOP_SCHEDULER_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.mt_loop_scheduler_projection@1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DispatchDecisionKind {
    DispatchNextCoder,
    HoldForLease,
    HoldForCoderCompletion,
    RouteToRemediation,
    Escalate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopPrerequisiteKind {
    ClaimLease,
    CurrentCoderCompletion,
    DependencyState,
    RetryBudget,
    VerdictState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopSchedulerFailureState {
    MissingClaimLease,
    MissingCurrentCoderCompletion,
    MissingDependencyState,
    MissingRetryBudget,
    MissingVerdictState,
    DispatchWithFailedPrerequisite,
    DependentAdvancedBeforeRemediation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSchedulerClaimLeaseV1 {
    pub lease_id: String,
    pub claimant_session_id: String,
    pub active: bool,
    pub expired: bool,
    pub exclusive: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSchedulerCurrentCoderV1 {
    pub coder_session_id: String,
    pub claimed_mt_id: String,
    pub completed: bool,
    pub completion_receipt_ref: String,
    pub handoff_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSchedulerDependencyStateV1 {
    pub parent_mt_refs: Vec<String>,
    pub blocked_by_refs: Vec<String>,
    pub dependent_mt_refs: Vec<String>,
    pub remediation_before_dependents: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSchedulerRetryBudgetV1 {
    pub max_attempts: u32,
    pub remaining_attempts: u32,
    pub attempts_consumed: u32,
    pub terminal_action_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSchedulerVerdictStateV1 {
    pub verdict_id: String,
    pub verdict: ValidatorVerdictKind,
    pub routing_outcome: VerdictRoutingOutcome,
    pub evidence_refs: Vec<String>,
    pub remediation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopSchedulerResearchBasisV1 {
    pub source_ref: String,
    pub pattern_found: String,
    pub selected_reuse: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtLoopSchedulerContractV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub next_mt_id: String,
    pub current_coder: LoopSchedulerCurrentCoderV1,
    pub claim_lease: LoopSchedulerClaimLeaseV1,
    pub dependency_state: LoopSchedulerDependencyStateV1,
    pub retry_budget: LoopSchedulerRetryBudgetV1,
    pub verdict_state: LoopSchedulerVerdictStateV1,
    pub required_prerequisites: Vec<LoopPrerequisiteKind>,
    pub allowed_action_ids: Vec<String>,
    pub remediation_action_ids: Vec<String>,
    pub failure_states: Vec<LoopSchedulerFailureState>,
    pub research_basis_refs: Vec<LoopSchedulerResearchBasisV1>,
    pub status_mutation_allowed: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtLoopSchedulerProjectionV1 {
    pub schema_id: String,
    pub source_contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub next_mt_id: String,
    pub decision: DispatchDecisionKind,
    pub next_coder_dispatch_required: bool,
    pub dependents_may_advance: bool,
    pub remediation_required: bool,
    pub blocked_prerequisites: Vec<LoopPrerequisiteKind>,
    pub remediation_action_ids: Vec<String>,
    pub allowed_action_ids: Vec<String>,
    pub status_mutation_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MtLoopSchedulerError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_mt_loop_scheduler() -> MtLoopSchedulerContractV1 {
    let wp_id = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";

    MtLoopSchedulerContractV1 {
        schema_id: MT_LOOP_SCHEDULER_SCHEMA_ID.to_string(),
        contract_id: "kernel002-mt060-loop-scheduler".to_string(),
        wp_id: wp_id.to_string(),
        mt_id: "MT-060".to_string(),
        next_mt_id: "MT-061".to_string(),
        current_coder: LoopSchedulerCurrentCoderV1 {
            coder_session_id: "KERNEL_BUILDER-20260514-130219".to_string(),
            claimed_mt_id: "MT-060".to_string(),
            completed: true,
            completion_receipt_ref:
                "receipt://WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-060"
                    .to_string(),
            handoff_ref: "kernel.coder_handoff_validation_request.project/MT-056".to_string(),
        },
        claim_lease: LoopSchedulerClaimLeaseV1 {
            lease_id: "claim-lease-kernel002-mt060-coder".to_string(),
            claimant_session_id: "KERNEL_BUILDER-20260514-130219".to_string(),
            active: true,
            expired: false,
            exclusive: true,
            evidence_refs: vec![
                "role-mailbox-claim-lease://kernel002/MT-060/coder".to_string(),
                "mt-board://kernel002/MT-060/claim".to_string(),
            ],
        },
        dependency_state: LoopSchedulerDependencyStateV1 {
            parent_mt_refs: vec!["MT-059".to_string()],
            blocked_by_refs: Vec::new(),
            dependent_mt_refs: vec!["MT-061".to_string()],
            remediation_before_dependents: true,
        },
        retry_budget: LoopSchedulerRetryBudgetV1 {
            max_attempts: 2,
            remaining_attempts: 1,
            attempts_consumed: 1,
            terminal_action_id: "kernel.remediation_work_generation.project".to_string(),
        },
        verdict_state: LoopSchedulerVerdictStateV1 {
            verdict_id: "validator-verdict-kernel002-mt060-pass".to_string(),
            verdict: ValidatorVerdictKind::Pass,
            routing_outcome: VerdictRoutingOutcome::MayAdvance,
            evidence_refs: vec![
                "validator-verdict-contract://kernel002/MT-060/pass".to_string(),
                "proof-harness://kernel002/mt-loop-scheduler".to_string(),
            ],
            remediation_ref: "kernel.remediation_work_generation.project".to_string(),
        },
        required_prerequisites: vec![
            LoopPrerequisiteKind::ClaimLease,
            LoopPrerequisiteKind::CurrentCoderCompletion,
            LoopPrerequisiteKind::DependencyState,
            LoopPrerequisiteKind::RetryBudget,
            LoopPrerequisiteKind::VerdictState,
        ],
        allowed_action_ids: vec![
            "kernel.role_mailbox_claim_lease.project".to_string(),
            "kernel.local_model_microtask_loop.project".to_string(),
            "kernel.validator_verdict_mediation.project".to_string(),
            "kernel.remediation_work_generation.project".to_string(),
            "kernel.mt_loop_scheduler.project".to_string(),
        ],
        remediation_action_ids: vec![
            "kernel.remediation_work_generation.project".to_string(),
            "kernel.validator_verdict_mediation.project".to_string(),
        ],
        failure_states: vec![
            LoopSchedulerFailureState::MissingClaimLease,
            LoopSchedulerFailureState::MissingCurrentCoderCompletion,
            LoopSchedulerFailureState::MissingDependencyState,
            LoopSchedulerFailureState::MissingRetryBudget,
            LoopSchedulerFailureState::MissingVerdictState,
            LoopSchedulerFailureState::DispatchWithFailedPrerequisite,
            LoopSchedulerFailureState::DependentAdvancedBeforeRemediation,
        ],
        research_basis_refs: vec![
            LoopSchedulerResearchBasisV1 {
                source_ref: "https://kubernetes.io/docs/concepts/architecture/controller/"
                    .to_string(),
                pattern_found: "Controllers reconcile current state toward desired state through a bounded control loop.".to_string(),
                selected_reuse: "Model next-coder dispatch as a projection over current state instead of direct status mutation.".to_string(),
            },
            LoopSchedulerResearchBasisV1 {
                source_ref: "https://kubernetes.io/docs/concepts/architecture/leases/"
                    .to_string(),
                pattern_found: "Lease objects coordinate active ownership and leader election.".to_string(),
                selected_reuse: "Require an active, unexpired exclusive claim lease before dispatching another coder.".to_string(),
            },
            LoopSchedulerResearchBasisV1 {
                source_ref: "https://docs.temporal.io/encyclopedia/retry-policies"
                    .to_string(),
                pattern_found: "Retry policies bound attempts and define terminal behavior after exhaustion.".to_string(),
                selected_reuse: "Route exhausted retry budgets to remediation rather than dependent advancement.".to_string(),
            },
            LoopSchedulerResearchBasisV1 {
                source_ref: "https://docs.github.com/en/actions/concepts/workflows-and-actions/concurrency".to_string(),
                pattern_found: "Concurrency controls prevent overlapping workflow executions where only one active actor is safe.".to_string(),
                selected_reuse: "Treat next-coder dispatch as exclusive per MT until the current coder and verdict gates clear.".to_string(),
            },
        ],
        status_mutation_allowed: false,
        product_authority_refs: vec![
            "kernel.role_mailbox_claim_lease".to_string(),
            "kernel.local_model_microtask_loop".to_string(),
            "kernel.validator_verdict_mediation_contract".to_string(),
            "kernel.remediation_work_generation_contract".to_string(),
        ],
        folded_source_refs: vec![
            "MT-056 coder handoff validation request contract".to_string(),
            "MT-057 validator verdict mediation contract".to_string(),
            "MT-059 remediation work generation contract".to_string(),
        ],
    }
}

pub fn evaluate_mt_loop_scheduler(
    contract: &MtLoopSchedulerContractV1,
) -> Result<MtLoopSchedulerProjectionV1, Vec<MtLoopSchedulerError>> {
    validate_mt_loop_scheduler(contract)?;
    let blocked_prerequisites = blocked_prerequisites(contract);
    let decision = dispatch_decision(contract, &blocked_prerequisites);

    Ok(MtLoopSchedulerProjectionV1 {
        schema_id: MT_LOOP_SCHEDULER_PROJECTION_SCHEMA_ID.to_string(),
        source_contract_id: contract.contract_id.clone(),
        wp_id: contract.wp_id.clone(),
        mt_id: contract.mt_id.clone(),
        next_mt_id: contract.next_mt_id.clone(),
        next_coder_dispatch_required: decision == DispatchDecisionKind::DispatchNextCoder,
        dependents_may_advance: decision == DispatchDecisionKind::DispatchNextCoder,
        remediation_required: decision == DispatchDecisionKind::RouteToRemediation,
        decision,
        blocked_prerequisites,
        remediation_action_ids: contract.remediation_action_ids.clone(),
        allowed_action_ids: contract.allowed_action_ids.clone(),
        status_mutation_allowed: contract.status_mutation_allowed,
    })
}

pub fn validate_mt_loop_scheduler(
    contract: &MtLoopSchedulerContractV1,
) -> Result<(), Vec<MtLoopSchedulerError>> {
    let mut errors = Vec::new();

    if contract.schema_id != MT_LOOP_SCHEDULER_SCHEMA_ID {
        errors.push(error("schema_id", "schema id must match MT loop scheduler"));
    }
    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    require_non_empty(&mut errors, "next_mt_id", &contract.next_mt_id);
    validate_current_coder(&mut errors, contract);
    validate_claim_lease(&mut errors, contract);
    validate_dependency_state(&mut errors, contract);
    validate_retry_budget(&mut errors, contract);
    validate_verdict_state(&mut errors, contract);
    require_vec(
        &mut errors,
        "required_prerequisites",
        &contract.required_prerequisites,
    );
    require_vec(
        &mut errors,
        "allowed_action_ids",
        &contract.allowed_action_ids,
    );
    require_vec(
        &mut errors,
        "remediation_action_ids",
        &contract.remediation_action_ids,
    );
    require_vec(&mut errors, "failure_states", &contract.failure_states);
    require_vec(
        &mut errors,
        "research_basis_refs",
        &contract.research_basis_refs,
    );
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

    for prerequisite in [
        LoopPrerequisiteKind::ClaimLease,
        LoopPrerequisiteKind::CurrentCoderCompletion,
        LoopPrerequisiteKind::DependencyState,
        LoopPrerequisiteKind::RetryBudget,
        LoopPrerequisiteKind::VerdictState,
    ] {
        if !contract.required_prerequisites.contains(&prerequisite) {
            errors.push(error(
                "required_prerequisites",
                "scheduler must list every dispatch prerequisite",
            ));
        }
    }

    if !contract
        .remediation_action_ids
        .contains(&"kernel.remediation_work_generation.project".to_string())
    {
        errors.push(error(
            "remediation_action_ids",
            "scheduler must be able to route failed prerequisites to remediation generation",
        ));
    }

    if contract.status_mutation_allowed {
        errors.push(error(
            "status_mutation_allowed",
            "scheduler projection cannot mutate MT, WP, task board, or lease authority",
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_current_coder(
    errors: &mut Vec<MtLoopSchedulerError>,
    contract: &MtLoopSchedulerContractV1,
) {
    let coder = &contract.current_coder;
    require_non_empty(
        errors,
        "current_coder.coder_session_id",
        &coder.coder_session_id,
    );
    require_non_empty(errors, "current_coder.claimed_mt_id", &coder.claimed_mt_id);
    require_non_empty(
        errors,
        "current_coder.completion_receipt_ref",
        &coder.completion_receipt_ref,
    );
    require_non_empty(errors, "current_coder.handoff_ref", &coder.handoff_ref);

    if coder.claimed_mt_id != contract.mt_id {
        errors.push(error(
            "current_coder.claimed_mt_id",
            "current coder must be bound to the scheduler MT",
        ));
    }
}

fn validate_claim_lease(
    errors: &mut Vec<MtLoopSchedulerError>,
    contract: &MtLoopSchedulerContractV1,
) {
    let lease = &contract.claim_lease;
    require_non_empty(errors, "claim_lease.lease_id", &lease.lease_id);
    require_non_empty(
        errors,
        "claim_lease.claimant_session_id",
        &lease.claimant_session_id,
    );
    require_vec(errors, "claim_lease.evidence_refs", &lease.evidence_refs);

    if lease.claimant_session_id != contract.current_coder.coder_session_id {
        errors.push(error(
            "claim_lease.claimant_session_id",
            "claim lease claimant must match the current coder session",
        ));
    }
}

fn validate_dependency_state(
    errors: &mut Vec<MtLoopSchedulerError>,
    contract: &MtLoopSchedulerContractV1,
) {
    let dependency_state = &contract.dependency_state;
    require_vec(
        errors,
        "dependency_state.parent_mt_refs",
        &dependency_state.parent_mt_refs,
    );
    require_vec(
        errors,
        "dependency_state.dependent_mt_refs",
        &dependency_state.dependent_mt_refs,
    );

    if !dependency_state
        .dependent_mt_refs
        .contains(&contract.next_mt_id)
    {
        errors.push(error(
            "dependency_state.dependent_mt_refs",
            "dependency state must cite the next MT before dispatch",
        ));
    }

    if !dependency_state.remediation_before_dependents {
        errors.push(error(
            "dependency_state.remediation_before_dependents",
            "failed prerequisites must route to remediation before dependents advance",
        ));
    }
}

fn validate_retry_budget(
    errors: &mut Vec<MtLoopSchedulerError>,
    contract: &MtLoopSchedulerContractV1,
) {
    let retry = &contract.retry_budget;
    if retry.max_attempts == 0 {
        errors.push(error(
            "retry_budget.max_attempts",
            "retry budget must allow at least one attempt",
        ));
    }
    if retry.remaining_attempts > retry.max_attempts {
        errors.push(error(
            "retry_budget.remaining_attempts",
            "remaining attempts cannot exceed max attempts",
        ));
    }
    if retry.attempts_consumed > retry.max_attempts {
        errors.push(error(
            "retry_budget.attempts_consumed",
            "consumed attempts cannot exceed max attempts",
        ));
    }
    require_non_empty(
        errors,
        "retry_budget.terminal_action_id",
        &retry.terminal_action_id,
    );
}

fn validate_verdict_state(
    errors: &mut Vec<MtLoopSchedulerError>,
    contract: &MtLoopSchedulerContractV1,
) {
    let verdict = &contract.verdict_state;
    require_non_empty(errors, "verdict_state.verdict_id", &verdict.verdict_id);
    require_vec(
        errors,
        "verdict_state.evidence_refs",
        &verdict.evidence_refs,
    );
    require_non_empty(
        errors,
        "verdict_state.remediation_ref",
        &verdict.remediation_ref,
    );

    if verdict.verdict == ValidatorVerdictKind::Pass
        && verdict.routing_outcome != VerdictRoutingOutcome::MayAdvance
    {
        errors.push(error(
            "verdict_state.routing_outcome",
            "passing verdict must use MayAdvance routing",
        ));
    }
    if verdict.verdict != ValidatorVerdictKind::Pass
        && verdict.routing_outcome == VerdictRoutingOutcome::MayAdvance
    {
        errors.push(error(
            "verdict_state.routing_outcome",
            "non-pass verdict cannot advance dependents",
        ));
    }
}

fn blocked_prerequisites(contract: &MtLoopSchedulerContractV1) -> Vec<LoopPrerequisiteKind> {
    let mut blocked = Vec::new();
    if !contract.claim_lease.active
        || contract.claim_lease.expired
        || !contract.claim_lease.exclusive
    {
        blocked.push(LoopPrerequisiteKind::ClaimLease);
    }
    if !contract.current_coder.completed {
        blocked.push(LoopPrerequisiteKind::CurrentCoderCompletion);
    }
    if !contract.dependency_state.blocked_by_refs.is_empty() {
        blocked.push(LoopPrerequisiteKind::DependencyState);
    }
    if contract.retry_budget.remaining_attempts == 0 {
        blocked.push(LoopPrerequisiteKind::RetryBudget);
    }
    if contract.verdict_state.verdict != ValidatorVerdictKind::Pass
        || contract.verdict_state.routing_outcome != VerdictRoutingOutcome::MayAdvance
    {
        blocked.push(LoopPrerequisiteKind::VerdictState);
    }
    blocked
}

fn dispatch_decision(
    contract: &MtLoopSchedulerContractV1,
    blocked: &[LoopPrerequisiteKind],
) -> DispatchDecisionKind {
    if blocked.is_empty() {
        return DispatchDecisionKind::DispatchNextCoder;
    }
    if blocked.contains(&LoopPrerequisiteKind::ClaimLease) {
        return DispatchDecisionKind::HoldForLease;
    }
    if blocked.contains(&LoopPrerequisiteKind::CurrentCoderCompletion) {
        return DispatchDecisionKind::HoldForCoderCompletion;
    }
    if blocked.contains(&LoopPrerequisiteKind::DependencyState)
        || blocked.contains(&LoopPrerequisiteKind::RetryBudget)
        || blocked.contains(&LoopPrerequisiteKind::VerdictState)
    {
        if contract.verdict_state.routing_outcome == VerdictRoutingOutcome::MustEscalate {
            DispatchDecisionKind::Escalate
        } else {
            DispatchDecisionKind::RouteToRemediation
        }
    } else {
        DispatchDecisionKind::RouteToRemediation
    }
}

fn require_non_empty(errors: &mut Vec<MtLoopSchedulerError>, field: &'static str, value: &str) {
    if value.trim().is_empty() {
        errors.push(error(field, "field must not be empty"));
    }
}

fn require_vec<T>(errors: &mut Vec<MtLoopSchedulerError>, field: &'static str, value: &[T]) {
    if value.is_empty() {
        errors.push(error(field, "field must not be empty"));
    }
}

fn error(field: &'static str, message: &'static str) -> MtLoopSchedulerError {
    MtLoopSchedulerError { field, message }
}
