use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::{AuthorityEffect, ExpectedWriteBoxRef};

pub const LOCAL_MODEL_FRESH_CONTEXT_MT_LOOP_SCHEMA_ID: &str =
    "hsk.kernel.local_model_fresh_context_mt_loop@1";
pub const LOCAL_MODEL_FRESH_CONTEXT_MT_LOOP_PROJECTION_SCHEMA_ID: &str =
    "hsk.kernel.local_model_fresh_context_mt_loop_projection@1";
pub const FEMS_MT_HANDOFF_MEMORY_CONTEXT_SCHEMA_ID: &str =
    "hsk.kernel.fems_mt_handoff_memory_context@1";
pub const MEMORY_EXTRACT_PROTOCOL_V0_1: &str = "memory_extract_v0.1";

const REQUIRED_CONTEXT_REFS: [&str; 6] = [
    "compact_summary",
    "microtask_contract",
    "work_packet_scope",
    "done_criteria",
    "proof_targets",
    "memory_pack",
];

const FORBIDDEN_CONTEXT_REFS: [&str; 3] = [
    "unrelated_work_packet_full_scope",
    "full_taskboard_snapshot",
    "hidden_conversation_history",
];

const REQUIRED_LOCUS_ACTION_IDS: [&str; 8] = [
    "assign_micro_task",
    "continue_micro_task",
    "retry_micro_task",
    "resolve_micro_task_blocker",
    "validate",
    "review",
    "request_changes",
    "archive_micro_task",
];

const REQUIRED_WRITE_BOX_KINDS: [&str; 5] = [
    "ExecutionBox",
    "ArtifactBox",
    "MemoryBox",
    "ProposalBox",
    "PromotionBox",
];

const REQUIRED_RECEIPT_KINDS: [LoopReceiptKind; 7] = [
    LoopReceiptKind::ClaimRecorded,
    LoopReceiptKind::AttemptStarted,
    LoopReceiptKind::AttemptStatusRecorded,
    LoopReceiptKind::VerifierHandoffEmitted,
    LoopReceiptKind::FailureRequeueEmitted,
    LoopReceiptKind::MemoryCheckpointRecorded,
    LoopReceiptKind::FinalOutcomeRecorded,
];

const REQUIRED_FINAL_OUTCOMES: [FinalMicrotaskOutcomeKind; 5] = [
    FinalMicrotaskOutcomeKind::Completed,
    FinalMicrotaskOutcomeKind::RequeuedForRetry,
    FinalMicrotaskOutcomeKind::Escalated,
    FinalMicrotaskOutcomeKind::Blocked,
    FinalMicrotaskOutcomeKind::FailedTerminal,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshContextInputBundleV1 {
    pub bundle_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub compact_summary_ref: String,
    pub microtask_contract_ref: String,
    pub work_packet_scope_ref: String,
    pub allowed_paths: Vec<String>,
    pub done_criteria_refs: Vec<String>,
    pub proof_target_refs: Vec<String>,
    pub memory_pack_ref: String,
    pub previous_attempt_refs: Vec<String>,
    pub required_context_refs: Vec<String>,
    pub forbidden_context_refs: Vec<String>,
    pub unrelated_wp_scope_available: bool,
    pub max_input_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelLoopActionBindingV1 {
    pub action_id: String,
    pub workflow_family: String,
    pub registered_locus_action: bool,
    pub authority_effect: AuthorityEffect,
    pub write_box_kinds: Vec<String>,
    pub receipt_kind: LoopReceiptKind,
    pub validation_hooks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelRetryBudgetV1 {
    pub max_attempts: u32,
    pub remaining_attempts: u32,
    pub max_iterations_per_attempt: u32,
    pub retry_backoff: String,
    pub failure_requeue_allowed: bool,
    pub terminal_after_max_attempts: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelVerifierHandoffV1 {
    pub required: bool,
    pub verifier_role: String,
    pub validation_requirement_ids: Vec<String>,
    pub handoff_bundle_ref: String,
    pub review_receipt_required: bool,
    pub failure_requeue_action_id: Option<String>,
    pub promotion_requires_verifier_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelFailureRequeueV1 {
    pub enabled: bool,
    pub queue_reason_codes: Vec<String>,
    pub retry_after_utc_required: bool,
    pub failure_category_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelMemoryCheckpointInputV1 {
    pub schema_id: String,
    pub checkpoint_kinds: Vec<String>,
    pub checkpoint_ids: Vec<String>,
    pub session_close_triggers_memory_extract: bool,
    pub memory_extract_protocol_id: String,
    pub max_handoff_tokens: u32,
    pub automatic_long_term_memory_merge_allowed: bool,
    pub memory_box_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopReceiptKind {
    ClaimRecorded,
    AttemptStarted,
    AttemptStatusRecorded,
    VerifierHandoffEmitted,
    FailureRequeueEmitted,
    MemoryCheckpointRecorded,
    FinalOutcomeRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelReceiptEmissionV1 {
    pub kind: LoopReceiptKind,
    pub receipt_kind: String,
    pub schema_id: String,
    pub correlation_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FinalMicrotaskOutcomeKind {
    Completed,
    RequeuedForRetry,
    Escalated,
    Blocked,
    FailedTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelFinalOutcomePathV1 {
    pub kind: FinalMicrotaskOutcomeKind,
    pub locus_status: String,
    pub locus_iteration_outcome: String,
    pub validation_passed: Option<bool>,
    pub evidence_required: bool,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelScopeGuardV1 {
    pub wp_id: String,
    pub mt_id: String,
    pub out_of_scope_action_policy: String,
    pub mutates_unrelated_wp_state: bool,
    pub mutates_task_board_directly: bool,
    pub mutates_mailbox_state_directly: bool,
    pub mutates_durable_memory_directly: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopFailureState {
    ContextBleed,
    UnregisteredGovernedAction,
    MissingWriteBox,
    RetryBudgetExhausted,
    VerifierHandoffMissing,
    FailureRequeueMissing,
    MemoryCheckpointUnsafe,
    ReceiptOmitted,
    DirectAuthorityMutation,
    FinalOutcomeMissing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelFreshContextMicrotaskLoopV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub work_profile_id: String,
    pub local_model_route_id: String,
    pub selected_route_model_ref: String,
    pub executor_kind: String,
    pub workflow_state_family: String,
    pub queue_reason_code: String,
    pub compact_summary_required: bool,
    pub fresh_context_per_attempt: bool,
    pub one_mt_per_context: bool,
    pub requires_unrelated_wp_scope: bool,
    pub write_boxes_required: bool,
    pub input_bundle: FreshContextInputBundleV1,
    pub locus_statuses: Vec<String>,
    pub locus_iteration_outcomes: Vec<String>,
    pub allowed_actions: Vec<LocalModelLoopActionBindingV1>,
    pub expected_write_boxes: Vec<ExpectedWriteBoxRef>,
    pub retry_budget: LocalModelRetryBudgetV1,
    pub verifier_handoff: LocalModelVerifierHandoffV1,
    pub failure_requeue: LocalModelFailureRequeueV1,
    pub memory_checkpoint_input: LocalModelMemoryCheckpointInputV1,
    pub receipt_emissions: Vec<LocalModelReceiptEmissionV1>,
    pub final_outcomes: Vec<FinalMicrotaskOutcomeKind>,
    pub final_outcome_paths: Vec<LocalModelFinalOutcomePathV1>,
    pub scope_guard: LocalModelScopeGuardV1,
    pub failure_states: Vec<LoopFailureState>,
    pub product_authority_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelMicrotaskLoopProjectionV1 {
    pub schema_id: String,
    pub contract_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub executor_kind: String,
    pub action_ids: Vec<String>,
    pub write_box_kinds: Vec<String>,
    pub remaining_attempts: u32,
    pub verifier_role: String,
    pub memory_extract_protocol_id: String,
    pub final_outcomes: Vec<FinalMicrotaskOutcomeKind>,
    pub mutates_authority_directly: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalModelMicrotaskLoopValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_local_model_microtask_loop() -> LocalModelFreshContextMicrotaskLoopV1 {
    let wp_id = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string();
    let mt_id = "MT-054".to_string();
    let local_route_id = "work-profile:local-small-model:mt-loop".to_string();

    LocalModelFreshContextMicrotaskLoopV1 {
        schema_id: LOCAL_MODEL_FRESH_CONTEXT_MT_LOOP_SCHEMA_ID.to_string(),
        contract_id: "kernel002-local-model-fresh-context-mt-loop-v1".to_string(),
        wp_id: wp_id.clone(),
        mt_id: mt_id.clone(),
        work_profile_id: "kernel-builder-local-small-model-v1".to_string(),
        local_model_route_id: local_route_id.clone(),
        selected_route_model_ref: local_route_id,
        executor_kind: "local_small_model".to_string(),
        workflow_state_family: "ready".to_string(),
        queue_reason_code: "ready_for_local_small_model".to_string(),
        compact_summary_required: true,
        fresh_context_per_attempt: true,
        one_mt_per_context: true,
        requires_unrelated_wp_scope: false,
        write_boxes_required: true,
        input_bundle: FreshContextInputBundleV1 {
            bundle_id: "kernel002-mt-054-fresh-context-input-bundle".to_string(),
            wp_id: wp_id.clone(),
            mt_id: mt_id.clone(),
            compact_summary_ref: "runtime://locus/compact-summary/WP-KERNEL-002/MT-054".to_string(),
            microtask_contract_ref: format!(".GOV/task_packets/{wp_id}/MT-054.json"),
            work_packet_scope_ref: format!(".GOV/task_packets/{wp_id}/packet.json"),
            allowed_paths: vec![
                "src/backend/handshake_core/src/locus/**".to_string(),
                "src/backend/handshake_core/src/kernel/**".to_string(),
                "src/backend/handshake_core/tests/**".to_string(),
            ],
            done_criteria_refs: vec![format!(".GOV/task_packets/{wp_id}/MT-054.json#acceptance")],
            proof_target_refs: vec![
                "cargo test --test kernel_local_model_microtask_loop_tests".to_string(),
                "cargo test --test local_model_microtask_loop_harness".to_string(),
            ],
            memory_pack_ref: "runtime://fems/memory-pack/WP-KERNEL-002/MT-054".to_string(),
            previous_attempt_refs: vec![
                "runtime://locus/mt-iterations/WP-KERNEL-002/MT-054".to_string(),
            ],
            required_context_refs: strings(&REQUIRED_CONTEXT_REFS),
            forbidden_context_refs: strings(&FORBIDDEN_CONTEXT_REFS),
            unrelated_wp_scope_available: false,
            max_input_tokens: 8000,
        },
        locus_statuses: vec![
            "pending".to_string(),
            "in_progress".to_string(),
            "completed".to_string(),
            "failed".to_string(),
            "blocked".to_string(),
            "skipped".to_string(),
        ],
        locus_iteration_outcomes: vec![
            "SUCCESS".to_string(),
            "RETRY".to_string(),
            "ESCALATE".to_string(),
            "BLOCKED".to_string(),
            "SKIPPED".to_string(),
        ],
        allowed_actions: vec![
            action(
                "assign_micro_task",
                "ready",
                &["ArtifactBox"],
                LoopReceiptKind::ClaimRecorded,
                &["locus_ready_family", "compact_summary_available"],
            ),
            action(
                "continue_micro_task",
                "active",
                &["ExecutionBox", "ArtifactBox"],
                LoopReceiptKind::AttemptStarted,
                &["write_box_execution_output", "iteration_record_opened"],
            ),
            action(
                "retry_micro_task",
                "blocked",
                &["ProposalBox", "MemoryBox"],
                LoopReceiptKind::FailureRequeueEmitted,
                &["retry_budget_remaining", "failure_category_recorded"],
            ),
            action(
                "resolve_micro_task_blocker",
                "blocked",
                &["ProposalBox"],
                LoopReceiptKind::AttemptStatusRecorded,
                &["blocker_resolution_evidence"],
            ),
            action(
                "validate",
                "validation",
                &["ArtifactBox"],
                LoopReceiptKind::VerifierHandoffEmitted,
                &["validation_artifact_ref", "verifier_handoff_bundle"],
            ),
            action(
                "review",
                "review",
                &["ArtifactBox", "PromotionBox"],
                LoopReceiptKind::VerifierHandoffEmitted,
                &["review_receipt", "promotion_gate_required"],
            ),
            action(
                "request_changes",
                "review",
                &["ProposalBox"],
                LoopReceiptKind::FailureRequeueEmitted,
                &["change_request_ref", "failure_requeue_ref"],
            ),
            action(
                "archive_micro_task",
                "done",
                &["ArtifactBox"],
                LoopReceiptKind::FinalOutcomeRecorded,
                &["final_outcome_receipt", "archive_after_validation"],
            ),
        ],
        expected_write_boxes: vec![
            expected_box(
                "ExecutionBox",
                "hsk.write_box.execution@1",
                "model_attempt_output",
            ),
            expected_box(
                "ArtifactBox",
                "hsk.write_box.artifact@1",
                "bundle_and_evidence",
            ),
            expected_box(
                "MemoryBox",
                "hsk.write_box.memory@1",
                "memory_extract_proposal",
            ),
            expected_box(
                "ProposalBox",
                "hsk.write_box.proposal@1",
                "requeue_or_failure",
            ),
            expected_box(
                "PromotionBox",
                "hsk.write_box.promotion@1",
                "verifier_approved_closeout",
            ),
        ],
        retry_budget: LocalModelRetryBudgetV1 {
            max_attempts: 3,
            remaining_attempts: 3,
            max_iterations_per_attempt: 1,
            retry_backoff: "deterministic_timer_wait".to_string(),
            failure_requeue_allowed: true,
            terminal_after_max_attempts: true,
        },
        verifier_handoff: LocalModelVerifierHandoffV1 {
            required: true,
            verifier_role: "VALIDATOR".to_string(),
            validation_requirement_ids: vec![
                "focused_product_test".to_string(),
                "proof_harness_test".to_string(),
                "write_box_evidence_review".to_string(),
            ],
            handoff_bundle_ref: "runtime://role-mailbox/verifier-handoff/WP-KERNEL-002/MT-054"
                .to_string(),
            review_receipt_required: true,
            failure_requeue_action_id: Some("retry_micro_task".to_string()),
            promotion_requires_verifier_approval: true,
        },
        failure_requeue: LocalModelFailureRequeueV1 {
            enabled: true,
            queue_reason_codes: vec![
                "timer_wait".to_string(),
                "ready_for_local_small_model".to_string(),
            ],
            retry_after_utc_required: true,
            failure_category_required: true,
        },
        memory_checkpoint_input: LocalModelMemoryCheckpointInputV1 {
            schema_id: FEMS_MT_HANDOFF_MEMORY_CONTEXT_SCHEMA_ID.to_string(),
            checkpoint_kinds: vec![
                "SessionOpen".to_string(),
                "PreTask".to_string(),
                "TaskComplete".to_string(),
                "SessionClose".to_string(),
            ],
            checkpoint_ids: vec![
                "fems:checkpoint:session-open".to_string(),
                "fems:checkpoint:pre-task".to_string(),
                "fems:checkpoint:task-complete".to_string(),
                "fems:checkpoint:session-close".to_string(),
            ],
            session_close_triggers_memory_extract: true,
            memory_extract_protocol_id: MEMORY_EXTRACT_PROTOCOL_V0_1.to_string(),
            max_handoff_tokens: 500,
            automatic_long_term_memory_merge_allowed: false,
            memory_box_required: true,
        },
        receipt_emissions: vec![
            receipt(LoopReceiptKind::ClaimRecorded, "CLAIM", "claim"),
            receipt(LoopReceiptKind::AttemptStarted, "STATUS", "attempt-started"),
            receipt(
                LoopReceiptKind::AttemptStatusRecorded,
                "STATUS",
                "attempt-status",
            ),
            receipt(
                LoopReceiptKind::VerifierHandoffEmitted,
                "REVIEW_REQUEST",
                "verifier-handoff",
            ),
            receipt(
                LoopReceiptKind::FailureRequeueEmitted,
                "STATUS",
                "failure-requeue",
            ),
            receipt(
                LoopReceiptKind::MemoryCheckpointRecorded,
                "STATUS",
                "memory-checkpoint",
            ),
            receipt(
                LoopReceiptKind::FinalOutcomeRecorded,
                "STATUS",
                "final-outcome",
            ),
        ],
        final_outcomes: REQUIRED_FINAL_OUTCOMES.to_vec(),
        final_outcome_paths: vec![
            final_path(
                FinalMicrotaskOutcomeKind::Completed,
                "completed",
                "SUCCESS",
                Some(true),
                "archive_micro_task",
            ),
            final_path(
                FinalMicrotaskOutcomeKind::RequeuedForRetry,
                "failed",
                "RETRY",
                Some(false),
                "retry_micro_task",
            ),
            final_path(
                FinalMicrotaskOutcomeKind::Escalated,
                "blocked",
                "ESCALATE",
                Some(false),
                "review",
            ),
            final_path(
                FinalMicrotaskOutcomeKind::Blocked,
                "blocked",
                "BLOCKED",
                None,
                "resolve_micro_task_blocker",
            ),
            final_path(
                FinalMicrotaskOutcomeKind::FailedTerminal,
                "failed",
                "BLOCKED",
                Some(false),
                "request_changes",
            ),
        ],
        scope_guard: LocalModelScopeGuardV1 {
            wp_id,
            mt_id,
            out_of_scope_action_policy: "deny".to_string(),
            mutates_unrelated_wp_state: false,
            mutates_task_board_directly: false,
            mutates_mailbox_state_directly: false,
            mutates_durable_memory_directly: false,
        },
        failure_states: vec![
            LoopFailureState::ContextBleed,
            LoopFailureState::UnregisteredGovernedAction,
            LoopFailureState::MissingWriteBox,
            LoopFailureState::RetryBudgetExhausted,
            LoopFailureState::VerifierHandoffMissing,
            LoopFailureState::FailureRequeueMissing,
            LoopFailureState::MemoryCheckpointUnsafe,
            LoopFailureState::ReceiptOmitted,
            LoopFailureState::DirectAuthorityMutation,
            LoopFailureState::FinalOutcomeMissing,
        ],
        product_authority_refs: vec![
            "locus.tracked_micro_task".to_string(),
            "kernel.write_boxes".to_string(),
            "kernel.role_mailbox_loop_control".to_string(),
            "kernel.fems_mt_handoff_memory_context".to_string(),
            "kernel.work_profiles".to_string(),
            "kernel.local_first_mcp_posture".to_string(),
            "flight_recorder.micro_task_events".to_string(),
        ],
    }
}

pub fn validate_local_model_microtask_loop(
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) -> Result<(), Vec<LocalModelMicrotaskLoopValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "contract_id", &contract.contract_id);
    require_non_empty(&mut errors, "wp_id", &contract.wp_id);
    require_non_empty(&mut errors, "mt_id", &contract.mt_id);
    require_non_empty(&mut errors, "work_profile_id", &contract.work_profile_id);
    require_non_empty(
        &mut errors,
        "local_model_route_id",
        &contract.local_model_route_id,
    );
    require_non_empty(
        &mut errors,
        "selected_route_model_ref",
        &contract.selected_route_model_ref,
    );

    if contract.schema_id != LOCAL_MODEL_FRESH_CONTEXT_MT_LOOP_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "local model MT loop must use the fresh-context schema",
        ));
    }
    if contract.executor_kind != "local_small_model" {
        errors.push(error(
            "executor_kind",
            "loop must be routed to the Locus local-small-model executor kind",
        ));
    }
    if contract.workflow_state_family != "ready" {
        errors.push(error(
            "workflow_state_family",
            "local-small-model eligibility must start from the Ready family",
        ));
    }
    if contract.queue_reason_code != "ready_for_local_small_model" {
        errors.push(error(
            "queue_reason_code",
            "loop must use the Locus ready_for_local_small_model queue reason",
        ));
    }
    if !contract.compact_summary_required {
        errors.push(error(
            "compact_summary_required",
            "local-small-model eligibility requires a compact summary",
        ));
    }
    if !contract.fresh_context_per_attempt {
        errors.push(error(
            "fresh_context_per_attempt",
            "each model attempt must start from a fresh context bundle",
        ));
    }
    if !contract.one_mt_per_context {
        errors.push(error(
            "one_mt_per_context",
            "fresh-context loop must execute exactly one MT at a time",
        ));
    }
    if contract.requires_unrelated_wp_scope {
        errors.push(error(
            "requires_unrelated_wp_scope",
            "local MT execution must not require unrelated WP scope",
        ));
    }
    if !contract.write_boxes_required {
        errors.push(error(
            "write_boxes_required",
            "local MT execution must emit write-box evidence instead of direct authority writes",
        ));
    }
    if contract.selected_route_model_ref != contract.local_model_route_id {
        errors.push(error(
            "selected_route_model_ref",
            "selected work-profile route must be recorded in job metadata",
        ));
    }

    validate_input_bundle(&mut errors, contract);
    validate_locus_vocabulary(&mut errors, contract);
    validate_actions(&mut errors, contract);
    validate_write_boxes(&mut errors, contract);
    validate_retry_budget(&mut errors, contract);
    validate_verifier_handoff(&mut errors, contract);
    validate_failure_requeue(&mut errors, contract);
    validate_memory_checkpoint(&mut errors, contract);
    validate_receipts(&mut errors, contract);
    validate_final_outcomes(&mut errors, contract);
    validate_scope_guard(&mut errors, contract);
    validate_authority_refs(&mut errors, contract);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_local_model_microtask_loop(
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) -> Result<LocalModelMicrotaskLoopProjectionV1, Vec<LocalModelMicrotaskLoopValidationError>> {
    validate_local_model_microtask_loop(contract)?;

    Ok(LocalModelMicrotaskLoopProjectionV1 {
        schema_id: LOCAL_MODEL_FRESH_CONTEXT_MT_LOOP_PROJECTION_SCHEMA_ID.to_string(),
        contract_id: contract.contract_id.clone(),
        wp_id: contract.wp_id.clone(),
        mt_id: contract.mt_id.clone(),
        executor_kind: contract.executor_kind.clone(),
        action_ids: contract
            .allowed_actions
            .iter()
            .map(|action| action.action_id.clone())
            .collect(),
        write_box_kinds: contract
            .expected_write_boxes
            .iter()
            .map(|write_box| write_box.write_box_kind.clone())
            .collect(),
        remaining_attempts: contract.retry_budget.remaining_attempts,
        verifier_role: contract.verifier_handoff.verifier_role.clone(),
        memory_extract_protocol_id: contract
            .memory_checkpoint_input
            .memory_extract_protocol_id
            .clone(),
        final_outcomes: contract.final_outcomes.clone(),
        mutates_authority_directly: false,
    })
}

fn validate_input_bundle(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    let bundle = &contract.input_bundle;
    require_non_empty(errors, "input_bundle.bundle_id", &bundle.bundle_id);
    require_non_empty(
        errors,
        "input_bundle.compact_summary_ref",
        &bundle.compact_summary_ref,
    );
    require_non_empty(
        errors,
        "input_bundle.microtask_contract_ref",
        &bundle.microtask_contract_ref,
    );
    require_non_empty(
        errors,
        "input_bundle.work_packet_scope_ref",
        &bundle.work_packet_scope_ref,
    );
    require_non_empty(
        errors,
        "input_bundle.memory_pack_ref",
        &bundle.memory_pack_ref,
    );
    require_vec(errors, "input_bundle.allowed_paths", &bundle.allowed_paths);
    require_vec(
        errors,
        "input_bundle.done_criteria_refs",
        &bundle.done_criteria_refs,
    );
    require_vec(
        errors,
        "input_bundle.proof_target_refs",
        &bundle.proof_target_refs,
    );
    require_vec(
        errors,
        "input_bundle.previous_attempt_refs",
        &bundle.previous_attempt_refs,
    );

    if bundle.wp_id != contract.wp_id {
        errors.push(error("input_bundle.wp_id", "input bundle must be WP-bound"));
    }
    if bundle.mt_id != contract.mt_id {
        errors.push(error("input_bundle.mt_id", "input bundle must be MT-bound"));
    }
    if bundle.unrelated_wp_scope_available {
        errors.push(error(
            "input_bundle.unrelated_wp_scope_available",
            "input bundle must not include unrelated WP scope",
        ));
    }
    if bundle.max_input_tokens == 0 || bundle.max_input_tokens > 8000 {
        errors.push(error(
            "input_bundle.max_input_tokens",
            "fresh context input bundle must have a bounded token budget",
        ));
    }

    for required in REQUIRED_CONTEXT_REFS {
        require_contains(
            errors,
            "input_bundle.required_context_refs",
            &bundle.required_context_refs,
            required,
            "fresh context bundle is missing a required context ref",
        );
    }
    for forbidden in FORBIDDEN_CONTEXT_REFS {
        require_contains(
            errors,
            "input_bundle.forbidden_context_refs",
            &bundle.forbidden_context_refs,
            forbidden,
            "fresh context bundle must declare forbidden context refs",
        );
    }
}

fn validate_locus_vocabulary(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    for status in [
        "pending",
        "in_progress",
        "completed",
        "failed",
        "blocked",
        "skipped",
    ] {
        require_contains(
            errors,
            "locus_statuses",
            &contract.locus_statuses,
            status,
            "loop must preserve Locus microtask statuses",
        );
    }
    for outcome in ["SUCCESS", "RETRY", "ESCALATE", "BLOCKED", "SKIPPED"] {
        require_contains(
            errors,
            "locus_iteration_outcomes",
            &contract.locus_iteration_outcomes,
            outcome,
            "loop must preserve Locus iteration outcomes",
        );
    }
}

fn validate_actions(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    let mut seen = HashSet::new();
    for action in &contract.allowed_actions {
        if !seen.insert(action.action_id.as_str()) {
            errors.push(error(
                "allowed_actions.action_id",
                "loop governed action ids must be unique",
            ));
        }
        require_non_empty(errors, "allowed_actions.action_id", &action.action_id);
        require_non_empty(
            errors,
            "allowed_actions.workflow_family",
            &action.workflow_family,
        );
        require_vec(
            errors,
            "allowed_actions.write_box_kinds",
            &action.write_box_kinds,
        );
        require_vec(
            errors,
            "allowed_actions.validation_hooks",
            &action.validation_hooks,
        );
        if !REQUIRED_LOCUS_ACTION_IDS.contains(&action.action_id.as_str()) {
            errors.push(error(
                "allowed_actions.action_id",
                "local loop action must come from the Locus governed action registry",
            ));
        }
        if !action.registered_locus_action {
            errors.push(error(
                "allowed_actions.registered_locus_action",
                "loop action must be marked as registered in Locus",
            ));
        }
    }

    for action_id in REQUIRED_LOCUS_ACTION_IDS {
        let Some(action) = contract
            .allowed_actions
            .iter()
            .find(|action| action.action_id == action_id)
        else {
            errors.push(error(
                "allowed_actions",
                "loop is missing a required Locus governed action",
            ));
            continue;
        };
        if !action.registered_locus_action {
            errors.push(error(
                "allowed_actions.registered_locus_action",
                "required action must be registered in Locus",
            ));
        }
    }
}

fn validate_write_boxes(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    require_vec(
        errors,
        "expected_write_boxes",
        &contract.expected_write_boxes,
    );
    for write_box in &contract.expected_write_boxes {
        require_non_empty(
            errors,
            "expected_write_boxes.write_box_kind",
            &write_box.write_box_kind,
        );
        require_non_empty(
            errors,
            "expected_write_boxes.write_box_schema_id",
            &write_box.write_box_schema_id,
        );
        require_non_empty(
            errors,
            "expected_write_boxes.target_id",
            &write_box.target_id,
        );
    }
    for box_kind in REQUIRED_WRITE_BOX_KINDS {
        if !contract
            .expected_write_boxes
            .iter()
            .any(|write_box| write_box.write_box_kind == box_kind)
        {
            errors.push(error(
                "expected_write_boxes",
                "loop must declare all expected write-box families",
            ));
        }
    }
}

fn validate_retry_budget(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
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
    if retry.max_iterations_per_attempt == 0 {
        errors.push(error(
            "retry_budget.max_iterations_per_attempt",
            "each attempt must allow at least one iteration",
        ));
    }
    if retry.retry_backoff.trim().is_empty() {
        errors.push(error(
            "retry_budget.retry_backoff",
            "retry backoff strategy must be explicit",
        ));
    }
    if !retry.failure_requeue_allowed {
        errors.push(error(
            "retry_budget.failure_requeue_allowed",
            "retryable failures must have a requeue path",
        ));
    }
    if !retry.terminal_after_max_attempts {
        errors.push(error(
            "retry_budget.terminal_after_max_attempts",
            "retry budget must define terminal behavior after exhaustion",
        ));
    }
}

fn validate_verifier_handoff(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    let handoff = &contract.verifier_handoff;
    if !handoff.required {
        errors.push(error(
            "verifier_handoff.required",
            "verifier handoff is required before final MT closeout",
        ));
    }
    require_non_empty(
        errors,
        "verifier_handoff.verifier_role",
        &handoff.verifier_role,
    );
    require_vec(
        errors,
        "verifier_handoff.validation_requirement_ids",
        &handoff.validation_requirement_ids,
    );
    require_non_empty(
        errors,
        "verifier_handoff.handoff_bundle_ref",
        &handoff.handoff_bundle_ref,
    );
    if !handoff.review_receipt_required {
        errors.push(error(
            "verifier_handoff.review_receipt_required",
            "verifier handoff must require a review receipt",
        ));
    }
    if handoff.failure_requeue_action_id.as_deref() != Some("retry_micro_task") {
        errors.push(error(
            "verifier_handoff.failure_requeue_action_id",
            "verifier failure must requeue through retry_micro_task",
        ));
    }
    if !handoff.promotion_requires_verifier_approval {
        errors.push(error(
            "verifier_handoff.promotion_requires_verifier_approval",
            "PromotionBox use must require verifier approval",
        ));
    }
}

fn validate_failure_requeue(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    let requeue = &contract.failure_requeue;
    if !requeue.enabled {
        errors.push(error(
            "failure_requeue.enabled",
            "retryable failure requeue must be enabled",
        ));
    }
    for reason in ["timer_wait", "ready_for_local_small_model"] {
        require_contains(
            errors,
            "failure_requeue.queue_reason_codes",
            &requeue.queue_reason_codes,
            reason,
            "failure requeue must encode timer wait and return-to-local queue reasons",
        );
    }
    if !requeue.retry_after_utc_required {
        errors.push(error(
            "failure_requeue.retry_after_utc_required",
            "timer-wait requeue must include retry_after_utc",
        ));
    }
    if !requeue.failure_category_required {
        errors.push(error(
            "failure_requeue.failure_category_required",
            "failure requeue must preserve failure_category",
        ));
    }
}

fn validate_memory_checkpoint(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    let memory = &contract.memory_checkpoint_input;
    if memory.schema_id != FEMS_MT_HANDOFF_MEMORY_CONTEXT_SCHEMA_ID {
        errors.push(error(
            "memory_checkpoint_input.schema_id",
            "loop must reuse the FEMS MT handoff memory context schema",
        ));
    }
    for kind in ["SessionOpen", "PreTask", "TaskComplete", "SessionClose"] {
        require_contains(
            errors,
            "memory_checkpoint_input.checkpoint_kinds",
            &memory.checkpoint_kinds,
            kind,
            "loop must preserve working-memory checkpoint kinds",
        );
    }
    require_vec(
        errors,
        "memory_checkpoint_input.checkpoint_ids",
        &memory.checkpoint_ids,
    );
    if !memory.session_close_triggers_memory_extract {
        errors.push(error(
            "memory_checkpoint_input.session_close_triggers_memory_extract",
            "session close must trigger memory extraction",
        ));
    }
    if memory.memory_extract_protocol_id != MEMORY_EXTRACT_PROTOCOL_V0_1 {
        errors.push(error(
            "memory_checkpoint_input.memory_extract_protocol_id",
            "memory extraction must use memory_extract_v0.1",
        ));
    }
    if memory.max_handoff_tokens == 0 || memory.max_handoff_tokens > 500 {
        errors.push(error(
            "memory_checkpoint_input.max_handoff_tokens",
            "handoff memory context must be capped at <= 500 tokens",
        ));
    }
    if memory.automatic_long_term_memory_merge_allowed {
        errors.push(error(
            "memory_checkpoint_input.automatic_long_term_memory_merge_allowed",
            "loop must not merge handoff memory into durable memory automatically",
        ));
    }
    if !memory.memory_box_required {
        errors.push(error(
            "memory_checkpoint_input.memory_box_required",
            "memory checkpoint output must go through MemoryBox evidence",
        ));
    }
}

fn validate_receipts(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    let mut seen = HashSet::new();
    for receipt in &contract.receipt_emissions {
        if !seen.insert(receipt.kind) {
            errors.push(error(
                "receipt_emissions.kind",
                "receipt emission kinds must be unique",
            ));
        }
        require_non_empty(
            errors,
            "receipt_emissions.receipt_kind",
            &receipt.receipt_kind,
        );
        require_non_empty(errors, "receipt_emissions.schema_id", &receipt.schema_id);
        require_non_empty(
            errors,
            "receipt_emissions.correlation_id",
            &receipt.correlation_id,
        );
        require_non_empty(
            errors,
            "receipt_emissions.idempotency_key",
            &receipt.idempotency_key,
        );
    }

    for required in REQUIRED_RECEIPT_KINDS {
        if !contract
            .receipt_emissions
            .iter()
            .any(|receipt| receipt.kind == required)
        {
            errors.push(error(
                "receipt_emissions",
                "loop is missing a required receipt emission",
            ));
        }
    }
}

fn validate_final_outcomes(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    for required in REQUIRED_FINAL_OUTCOMES {
        if !contract.final_outcomes.contains(&required) {
            errors.push(error(
                "final_outcomes",
                "loop is missing a required final MT outcome",
            ));
        }
    }

    for required in REQUIRED_FINAL_OUTCOMES {
        if !contract
            .final_outcome_paths
            .iter()
            .any(|path| path.kind == required)
        {
            errors.push(error(
                "final_outcome_paths",
                "each final MT outcome must have a Locus outcome path",
            ));
        }
    }
}

fn validate_scope_guard(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    let guard = &contract.scope_guard;
    if guard.wp_id != contract.wp_id {
        errors.push(error("scope_guard.wp_id", "scope guard must be WP-bound"));
    }
    if guard.mt_id != contract.mt_id {
        errors.push(error("scope_guard.mt_id", "scope guard must be MT-bound"));
    }
    if guard.out_of_scope_action_policy != "deny" {
        errors.push(error(
            "scope_guard.out_of_scope_action_policy",
            "out-of-scope local-model actions must be denied",
        ));
    }
    if guard.mutates_unrelated_wp_state
        || guard.mutates_task_board_directly
        || guard.mutates_mailbox_state_directly
        || guard.mutates_durable_memory_directly
    {
        errors.push(error(
            "scope_guard",
            "local loop must emit receipts/write boxes, not mutate authority directly",
        ));
    }
}

fn validate_authority_refs(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    contract: &LocalModelFreshContextMicrotaskLoopV1,
) {
    for required in [
        "locus.tracked_micro_task",
        "kernel.write_boxes",
        "kernel.role_mailbox_loop_control",
        "kernel.fems_mt_handoff_memory_context",
        "kernel.work_profiles",
        "kernel.local_first_mcp_posture",
        "flight_recorder.micro_task_events",
    ] {
        require_contains(
            errors,
            "product_authority_refs",
            &contract.product_authority_refs,
            required,
            "loop must cite the product authorities it composes",
        );
    }
}

fn action(
    action_id: &str,
    workflow_family: &str,
    write_box_kinds: &[&str],
    receipt_kind: LoopReceiptKind,
    validation_hooks: &[&str],
) -> LocalModelLoopActionBindingV1 {
    LocalModelLoopActionBindingV1 {
        action_id: action_id.to_string(),
        workflow_family: workflow_family.to_string(),
        registered_locus_action: true,
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        write_box_kinds: strings(write_box_kinds),
        receipt_kind,
        validation_hooks: strings(validation_hooks),
    }
}

fn expected_box(kind: &str, schema_id: &str, target_id: &str) -> ExpectedWriteBoxRef {
    ExpectedWriteBoxRef {
        write_box_kind: kind.to_string(),
        write_box_schema_id: schema_id.to_string(),
        target_id: target_id.to_string(),
    }
}

fn receipt(
    kind: LoopReceiptKind,
    receipt_kind: &str,
    segment: &str,
) -> LocalModelReceiptEmissionV1 {
    LocalModelReceiptEmissionV1 {
        kind,
        receipt_kind: receipt_kind.to_string(),
        schema_id: "hsk.kernel.local_model_mt_loop_receipt@1".to_string(),
        correlation_id: format!("kernel002-mt-054-{segment}"),
        idempotency_key: format!("kernel002-mt-054-{segment}-v1"),
    }
}

fn final_path(
    kind: FinalMicrotaskOutcomeKind,
    locus_status: &str,
    locus_iteration_outcome: &str,
    validation_passed: Option<bool>,
    next_action: &str,
) -> LocalModelFinalOutcomePathV1 {
    LocalModelFinalOutcomePathV1 {
        kind,
        locus_status: locus_status.to_string(),
        locus_iteration_outcome: locus_iteration_outcome.to_string(),
        validation_passed,
        evidence_required: true,
        next_action: next_action.to_string(),
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn require_non_empty(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}

fn require_contains(
    errors: &mut Vec<LocalModelMicrotaskLoopValidationError>,
    field: &'static str,
    values: &[String],
    required: &str,
    message: &'static str,
) {
    if !values.iter().any(|value| value == required) {
        errors.push(error(field, message));
    }
}

fn error(field: &'static str, message: &'static str) -> LocalModelMicrotaskLoopValidationError {
    LocalModelMicrotaskLoopValidationError { field, message }
}
