use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    fems_mt_handoff_memory_context::{
        project_fems_mt_handoff_memory_context, validate_fems_mt_handoff_memory_context,
        FemsMtFailedAttemptV1, FemsMtHandoffItemKind, FemsMtHandoffMemoryContextV1,
        FemsMtHandoffMemoryItemV1, FemsMtHandoffReason,
    },
};

#[test]
fn kernel_collaboration_memory_handoff_validates_typed_session_context() {
    let context = sample_context();

    validate_fems_mt_handoff_memory_context(&context).expect("handoff context validates");

    assert_eq!(context.handoff_reason, FemsMtHandoffReason::Escalation);
    assert_eq!(context.source_session_id, "session-local-small-1");
    assert_eq!(context.target_session_id, "session-cloud-escalated-2");
    assert_eq!(context.failed_attempts.len(), 1);
    assert_eq!(
        context.recommended_item_ids,
        vec!["item-procedure-recommended"]
    );
    assert!(!context.automatic_long_term_merge_allowed);
    assert!(!context.cross_wp_handoff_allowed);
}

#[test]
fn kernel_collaboration_memory_handoff_projects_bounded_receiving_pack_items() {
    let context = sample_context();
    let projection =
        project_fems_mt_handoff_memory_context(&context).expect("handoff projection builds");

    assert!(projection.effective_handoff_tokens <= context.max_handoff_tokens);
    assert!(projection
        .selected_item_ids
        .contains(&"item-procedure-recommended".to_string()));
    assert!(projection
        .boosted_item_ids
        .contains(&"item-procedure-recommended".to_string()));
    assert!(projection
        .failed_attempt_ids
        .contains(&"attempt-local-small-timeout".to_string()));
    assert!(projection
        .dropped_item_ids
        .contains(&"item-over-budget".to_string()));
    assert!(projection
        .deterministic_reduction_markers
        .iter()
        .any(|marker| marker.contains("item-over-budget")));
    assert_eq!(projection.fr_event_ref, "FR-EVT-MEM-004-mt036-handoff");
    assert_eq!(
        projection.locus_mt_iteration_ref,
        "locus://wp-kernel-002/MT-036/iteration-2"
    );
    assert!(!projection.mutates_long_term_memory);
}

#[test]
fn kernel_collaboration_memory_handoff_rejects_cross_wp_promotion_and_bad_provenance() {
    let mut context = sample_context();
    context.cross_wp_handoff_allowed = true;
    context.automatic_long_term_merge_allowed = true;
    context.fr_event_ref = "FR-EVT-MEM-999-wrong".to_string();
    context.target_session_id = context.source_session_id.clone();
    context.carried_items[0].provenance_ref.clear();
    context.carried_items[1].scope_refs.clear();
    context.recommended_item_ids = vec!["item-memory-fact".to_string()];

    let errors = validate_fems_mt_handoff_memory_context(&context)
        .expect_err("unsafe handoff context must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "cross_wp_handoff_allowed"));
    assert!(errors
        .iter()
        .any(|error| error.field == "automatic_long_term_merge_allowed"));
    assert!(errors.iter().any(|error| error.field == "fr_event_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "target_session_id"));
    assert!(errors
        .iter()
        .any(|error| error.field == "carried_items.provenance_ref"));
    assert!(errors
        .iter()
        .any(|error| error.field == "carried_items.scope_refs"));
    assert!(errors
        .iter()
        .any(|error| error.field == "recommended_item_ids"));
}

#[test]
fn kernel_collaboration_memory_handoff_catalogs_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog validates");

    let action = catalog
        .action("kernel.fems_mt_handoff_memory_context.project")
        .expect("FEMS MT handoff memory context action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "fems_handoff_context_provenance"));
}

fn sample_context() -> FemsMtHandoffMemoryContextV1 {
    FemsMtHandoffMemoryContextV1 {
        schema_id: "hsk.kernel.fems_mt_handoff_memory_context@1".to_string(),
        context_id: "handoff-context-mt036".to_string(),
        folded_stub_ids: vec!["WP-1-FEMS-MT-Handoff-Memory-Context-v1".to_string()],
        wp_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
        mt_id: "MT-036".to_string(),
        source_session_id: "session-local-small-1".to_string(),
        target_session_id: "session-cloud-escalated-2".to_string(),
        handoff_reason: FemsMtHandoffReason::Escalation,
        carried_items: vec![
            item(
                "item-procedure-recommended",
                FemsMtHandoffItemKind::RecommendedProceduralItem,
                180,
                72,
                true,
                false,
            ),
            item(
                "item-memory-fact",
                FemsMtHandoffItemKind::MemoryPackItem,
                120,
                65,
                false,
                false,
            ),
            item(
                "item-insight-checkpoint",
                FemsMtHandoffItemKind::InsightCheckpoint,
                90,
                60,
                false,
                false,
            ),
            item(
                "item-failed-attempt-summary",
                FemsMtHandoffItemKind::FailedAttempt,
                80,
                55,
                false,
                true,
            ),
            item(
                "item-over-budget",
                FemsMtHandoffItemKind::MemoryPackItem,
                260,
                20,
                false,
                true,
            ),
        ],
        failed_attempts: vec![FemsMtFailedAttemptV1 {
            attempt_id: "attempt-local-small-timeout".to_string(),
            source_session_id: "session-local-small-1".to_string(),
            failure_summary:
                "Local small model timed out while validating the handoff proof target.".to_string(),
            evidence_refs: vec!["tool://cargo/test-timeout".to_string()],
            retryable: true,
            score_penalty_x100: 30,
        }],
        recommended_item_ids: vec!["item-procedure-recommended".to_string()],
        max_handoff_tokens: 400,
        fr_event_ref: "FR-EVT-MEM-004-mt036-handoff".to_string(),
        locus_mt_iteration_ref: "locus://wp-kernel-002/MT-036/iteration-2".to_string(),
        automatic_long_term_merge_allowed: false,
        cross_wp_handoff_allowed: false,
        product_authority_refs: vec![
            "ace.memory_pack".to_string(),
            "flight_recorder.memory_handoff_context".to_string(),
            "locus.mt_iteration".to_string(),
            "kernel.fems_memory_poisoning_drift_guardrails".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-FEMS-MT-Handoff-Memory-Context-v1.contract.json"
                .to_string(),
        ],
    }
}

fn item(
    item_id: &str,
    kind: FemsMtHandoffItemKind,
    token_count: u32,
    base_score_x100: u8,
    predecessor_recommended: bool,
    source_attempt_failed: bool,
) -> FemsMtHandoffMemoryItemV1 {
    FemsMtHandoffMemoryItemV1 {
        item_id: item_id.to_string(),
        kind,
        source_session_id: "session-local-small-1".to_string(),
        memory_ref: format!("memory://{item_id}"),
        scope_refs: vec![
            "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
            "MT-036".to_string(),
        ],
        provenance_ref: format!("provenance://session-local-small-1/{item_id}"),
        token_count,
        base_score_x100,
        predecessor_recommended,
        source_attempt_failed,
    }
}
