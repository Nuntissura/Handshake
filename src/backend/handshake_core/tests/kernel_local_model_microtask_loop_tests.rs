use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    local_model_microtask_loop::{
        build_kernel002_local_model_microtask_loop, validate_local_model_microtask_loop,
        FinalMicrotaskOutcomeKind, LoopFailureState, LoopReceiptKind,
        LOCAL_MODEL_FRESH_CONTEXT_MT_LOOP_SCHEMA_ID,
    },
};

#[test]
fn local_model_loop_defines_fresh_context_input_bundle_without_unrelated_wp_scope() {
    let contract = build_kernel002_local_model_microtask_loop();

    validate_local_model_microtask_loop(&contract).expect("local model loop contract validates");

    assert_eq!(
        contract.schema_id,
        LOCAL_MODEL_FRESH_CONTEXT_MT_LOOP_SCHEMA_ID
    );
    assert!(contract.fresh_context_per_attempt);
    assert!(contract.one_mt_per_context);
    assert!(!contract.requires_unrelated_wp_scope);
    assert!(!contract.input_bundle.unrelated_wp_scope_available);
    assert!(contract
        .input_bundle
        .microtask_contract_ref
        .ends_with("MT-054.json"));
    assert!(contract
        .input_bundle
        .required_context_refs
        .contains(&"microtask_contract".to_string()));
    assert!(contract
        .input_bundle
        .forbidden_context_refs
        .contains(&"unrelated_work_packet_full_scope".to_string()));
    assert_eq!(contract.input_bundle.wp_id, contract.wp_id);
    assert_eq!(contract.input_bundle.mt_id, contract.mt_id);
    assert!(contract.input_bundle.max_input_tokens <= 8000);
}

#[test]
fn local_model_loop_uses_locus_actions_statuses_and_local_first_route() {
    let contract = build_kernel002_local_model_microtask_loop();

    validate_local_model_microtask_loop(&contract).expect("local model loop contract validates");

    assert_eq!(contract.executor_kind, "local_small_model");
    assert_eq!(contract.workflow_state_family, "ready");
    assert_eq!(contract.queue_reason_code, "ready_for_local_small_model");
    assert_eq!(
        contract.selected_route_model_ref,
        contract.local_model_route_id
    );
    assert!(contract.locus_statuses.contains(&"in_progress".to_string()));
    assert!(contract
        .locus_iteration_outcomes
        .contains(&"RETRY".to_string()));
    for action_id in [
        "assign_micro_task",
        "continue_micro_task",
        "retry_micro_task",
        "resolve_micro_task_blocker",
        "validate",
        "review",
        "request_changes",
        "archive_micro_task",
    ] {
        assert!(
            contract
                .allowed_actions
                .iter()
                .any(|action| action.action_id == action_id && action.registered_locus_action),
            "{action_id} must be a registered Locus governed action"
        );
    }
}

#[test]
fn local_model_loop_covers_write_boxes_retry_handoff_memory_receipts_and_outcomes() {
    let contract = build_kernel002_local_model_microtask_loop();

    validate_local_model_microtask_loop(&contract).expect("local model loop contract validates");

    assert!(contract
        .allowed_actions
        .iter()
        .all(
            |action| action.authority_effect == AuthorityEffect::PrePromotionEvidenceOnly
                || action.authority_effect == AuthorityEffect::ProjectionOnly
        ));
    for box_kind in [
        "ExecutionBox",
        "ArtifactBox",
        "MemoryBox",
        "ProposalBox",
        "PromotionBox",
    ] {
        assert!(
            contract
                .expected_write_boxes
                .iter()
                .any(|write_box| write_box.write_box_kind == box_kind),
            "{box_kind} must be part of the loop write-box contract"
        );
    }
    assert_eq!(contract.retry_budget.max_attempts, 3);
    assert!(contract.retry_budget.failure_requeue_allowed);
    assert!(contract
        .failure_requeue
        .queue_reason_codes
        .contains(&"timer_wait".to_string()));
    assert!(contract
        .failure_requeue
        .queue_reason_codes
        .contains(&"ready_for_local_small_model".to_string()));
    assert!(contract.verifier_handoff.required);
    assert!(contract
        .verifier_handoff
        .failure_requeue_action_id
        .is_some());
    assert_eq!(
        contract.memory_checkpoint_input.schema_id,
        "hsk.kernel.fems_mt_handoff_memory_context@1"
    );
    assert_eq!(
        contract.memory_checkpoint_input.memory_extract_protocol_id,
        "memory_extract_v0.1"
    );
    assert!(contract.memory_checkpoint_input.max_handoff_tokens <= 500);
    assert!(
        !contract
            .memory_checkpoint_input
            .automatic_long_term_memory_merge_allowed
    );
    assert!(contract
        .receipt_emissions
        .iter()
        .any(|receipt| receipt.kind == LoopReceiptKind::ClaimRecorded));
    assert!(contract
        .receipt_emissions
        .iter()
        .any(|receipt| receipt.kind == LoopReceiptKind::FinalOutcomeRecorded));
    assert!(contract
        .final_outcomes
        .contains(&FinalMicrotaskOutcomeKind::Completed));
    assert!(contract
        .final_outcomes
        .contains(&FinalMicrotaskOutcomeKind::RequeuedForRetry));
}

#[test]
fn local_model_loop_rejects_context_bleed_missing_receipts_bad_memory_and_bad_retry_budget() {
    let mut contract = build_kernel002_local_model_microtask_loop();
    contract.input_bundle.unrelated_wp_scope_available = true;
    let errors = validate_local_model_microtask_loop(&contract)
        .expect_err("unrelated WP scope must not be present");
    assert!(errors
        .iter()
        .any(|error| error.field == "input_bundle.unrelated_wp_scope_available"));

    let mut contract = build_kernel002_local_model_microtask_loop();
    contract.retry_budget.max_attempts = 0;
    let errors = validate_local_model_microtask_loop(&contract)
        .expect_err("retry budget must allow at least one attempt");
    assert!(errors
        .iter()
        .any(|error| error.field == "retry_budget.max_attempts"));

    let mut contract = build_kernel002_local_model_microtask_loop();
    contract
        .memory_checkpoint_input
        .automatic_long_term_memory_merge_allowed = true;
    let errors = validate_local_model_microtask_loop(&contract)
        .expect_err("automatic durable-memory merge must be rejected");
    assert!(errors.iter().any(|error| {
        error.field == "memory_checkpoint_input.automatic_long_term_memory_merge_allowed"
    }));

    let mut contract = build_kernel002_local_model_microtask_loop();
    contract
        .receipt_emissions
        .retain(|receipt| receipt.kind != LoopReceiptKind::FinalOutcomeRecorded);
    let errors = validate_local_model_microtask_loop(&contract)
        .expect_err("final outcome receipt emission is required");
    assert!(errors
        .iter()
        .any(|error| error.field == "receipt_emissions"));

    assert!(contract
        .failure_states
        .contains(&LoopFailureState::ContextBleed));
    assert!(contract
        .failure_states
        .contains(&LoopFailureState::DirectAuthorityMutation));
}

#[test]
fn kernel_action_catalog_exposes_local_model_microtask_loop_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.local_model_microtask_loop.project")
        .expect("local model microtask loop projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "local_model_fresh_context_input_bundle"));
}
