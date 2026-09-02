//! MT-004 PromotionGate authority over one embedded SurrealDB namespace/database.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneAuthorityTestCorruption, ModelLaneKind,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLanePromotionDenialReason,
    ModelLanePromotionOutcome, ModelLanePromotionState, ModelLaneProviderKind,
    ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneRoutingPolicy, ModelLaneStatus,
    ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneMessage, NewModelLanePromotionDecision, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use surreal_test_store_support::EmbeddedSurrealTestScope;

struct Harness {
    isolated: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    scope: ResourceScope,
    store: ModelLaneStore,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let mut isolated = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate MT-004 embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical schema");
        let scope = exact_scope(label);
        let store = ModelLaneStore::new_scoped(storage.clone(), scope.clone());
        Self {
            isolated,
            storage,
            scope,
            store,
        }
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated.cleanup().await.expect("clean MT-004 scope");
    }
}

struct Seeded {
    proposal_version: i64,
}

#[tokio::test]
async fn model_lane_promotion_appends_eventledger_and_replays_decision() {
    let mut harness = Harness::create("promotion-positive").await;
    let seeded = seed_authority(&harness.store, "promotion-positive", true).await;
    let decision = sample_decision(
        "promotion-positive",
        "decision-approved",
        "idem-approved",
        seeded.proposal_version,
    );
    let stored = harness
        .store
        .record_promotion_decision(decision.clone())
        .await
        .expect("record canonical PromotionGate decision");
    assert_eq!(stored.outcome, ModelLanePromotionOutcome::Approved);
    assert_eq!(stored.final_state, ModelLanePromotionState::Executed);
    assert_eq!(
        stored.state_history,
        vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::PendingApproval,
            ModelLanePromotionState::Approved,
            ModelLanePromotionState::Executing,
            ModelLanePromotionState::Executed,
        ]
    );
    assert!(stored.event_ledger_seq > 0);
    assert_eq!(
        harness
            .store
            .record_promotion_decision(decision)
            .await
            .expect("identical decision retry"),
        stored
    );
    assert_eq!(
        harness
            .store
            .replay_promotion_decisions("run-mt004-promotion-positive")
            .await
            .expect("replay decisions"),
        vec![stored.clone()]
    );

    drop(harness.store);
    drop(harness.storage);
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close before restart");
    harness.isolated.reopen().await.expect("reopen same scope");
    let storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate same namespace/database");
    let reopened = ModelLaneStore::new_scoped(storage.clone(), harness.scope.clone());
    assert_eq!(
        reopened
            .replay_promotion_decisions("run-mt004-promotion-positive")
            .await
            .expect("decision survives restart"),
        vec![stored]
    );
    harness.store = reopened;
    harness.storage = storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation() {
    let harness = Harness::create("promotion-denials").await;
    let seeded = seed_authority(&harness.store, "promotion-denials", true).await;

    let mut stale = sample_decision(
        "promotion-denials",
        "decision-stale",
        "idem-stale",
        seeded.proposal_version,
    );
    stale.base_snapshot_ref = "snapshot://stale".into();
    let stale = harness
        .store
        .record_promotion_decision(stale)
        .await
        .expect("stale base is a durable denial");
    assert_eq!(stale.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(
        stale.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );

    let mut schema = sample_decision(
        "promotion-denials",
        "decision-schema",
        "idem-schema",
        seeded.proposal_version,
    );
    schema.schema_id = "hsk.model_lane_message@999".into();
    let schema = harness
        .store
        .record_promotion_decision(schema)
        .await
        .expect("schema mismatch is a durable denial");
    assert_eq!(
        schema.denial_reason,
        Some(ModelLanePromotionDenialReason::SchemaMismatch)
    );

    let mut direct = sample_decision(
        "promotion-denials",
        "decision-direct",
        "idem-direct",
        seeded.proposal_version,
    );
    direct.direct_authority_mutation_attempt_ref = Some("mutation://forbidden".into());
    let direct = harness
        .store
        .record_promotion_decision(direct)
        .await
        .expect("direct mutation is a durable denial");
    assert_eq!(
        direct.denial_reason,
        Some(ModelLanePromotionDenialReason::DirectAuthorityMutation)
    );
    assert_eq!(
        harness
            .store
            .replay_promotion_decisions("run-mt004-promotion-denials")
            .await
            .expect("replay durable denials")
            .len(),
        3
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_promotion_reordered_inputs_keep_same_decision_hash() {
    let harness = Harness::create("promotion-order").await;
    let seeded = seed_authority(&harness.store, "promotion-order", true).await;
    let first = harness
        .store
        .record_promotion_decision(sample_decision(
            "promotion-order",
            "decision-order-a",
            "idem-order-a",
            seeded.proposal_version,
        ))
        .await
        .expect("first canonical decision");
    let mut reordered = sample_decision(
        "promotion-order",
        "decision-order-b",
        "idem-order-b",
        seeded.proposal_version,
    );
    reordered.input_refs.reverse();
    let second = harness
        .store
        .record_promotion_decision(reordered)
        .await
        .expect("reordered canonical decision");
    assert_eq!(first.canonical_input_refs, second.canonical_input_refs);
    assert_eq!(first.canonical_hash_basis, second.canonical_hash_basis);
    assert_eq!(
        first.canonical_decision_hash,
        second.canonical_decision_hash
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_promotion_preserves_exact_scope_and_denies_foreign_or_mixed_sources() {
    let harness = Harness::create("promotion-scope").await;
    let seeded = seed_authority(&harness.store, "promotion-scope", true).await;
    let owner = harness
        .store
        .record_promotion_decision(sample_decision(
            "promotion-scope",
            "decision-owner",
            "idem-owner",
            seeded.proposal_version,
        ))
        .await
        .expect("owner decision");
    assert_eq!(owner.outcome, ModelLanePromotionOutcome::Approved);

    for (index, foreign_scope) in one_field_mismatches(&harness.scope).into_iter().enumerate() {
        let foreign = ModelLaneStore::new_scoped(harness.storage.clone(), foreign_scope);
        seed_authority(&foreign, "promotion-scope", false).await;
        let foreign_decision = foreign
            .record_promotion_decision(sample_decision(
                "promotion-scope",
                &format!("decision-foreign-{index}"),
                &format!("idem-foreign-{index}"),
                seeded.proposal_version,
            ))
            .await
            .expect("foreign mixed-source decision is durably denied");
        assert_eq!(foreign_decision.outcome, ModelLanePromotionOutcome::Denied);
        assert_eq!(
            foreign_decision.denial_reason,
            Some(ModelLanePromotionDenialReason::InputRefMismatch)
        );
    }
    assert_eq!(
        harness
            .store
            .replay_promotion_decisions("run-mt004-promotion-scope")
            .await
            .expect("owner decision unchanged"),
        vec![owner]
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn promotion_projection_and_event_receipt_tamper_fail_every_consumer_closed() {
    for (index, corruption) in [
        ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
        ModelLaneAuthorityTestCorruption::ProjectionScope,
        ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
        ModelLaneAuthorityTestCorruption::ReceiptScope,
    ]
    .into_iter()
    .enumerate()
    {
        let label = format!("promotion-tamper-{index}");
        let harness = Harness::create(&label).await;
        let seeded = seed_authority(&harness.store, &label, true).await;
        let decision_id = format!("decision-tamper-{index}");
        let decision = sample_decision(
            &label,
            &decision_id,
            &format!("idem-tamper-{index}"),
            seeded.proposal_version,
        );
        harness
            .store
            .record_promotion_decision(decision.clone())
            .await
            .expect("seed canonical promotion decision");
        harness
            .store
            .test_corrupt_scoped_authority("promotion_decision", &decision_id, corruption)
            .await
            .expect("apply enumerated exact-scope corruption");

        let run_id = format!("run-mt004-{label}");
        assert!(harness
            .store
            .replay_promotion_decisions(&run_id)
            .await
            .is_err());
        assert!(harness.store.replay_run(&run_id).await.is_err());
        assert!(harness.store.navigation_by_run(&run_id).await.is_err());
        assert!(harness
            .store
            .record_promotion_decision(decision)
            .await
            .is_err());
        assert!(harness
            .store
            .record_message(sample_promoted_message(&label, &decision_id, index))
            .await
            .is_err());
        harness.cleanup().await;
    }
}

#[tokio::test]
async fn navigation_rejects_tampered_run_lane_and_message_origins_before_redirect() {
    for (index, origin) in ["run", "lane", "message"].into_iter().enumerate() {
        let label = format!("origin-{origin}-{index}");
        let harness = Harness::create(&label).await;
        seed_authority(&harness.store, &label, true).await;
        let run_id = format!("run-mt004-{label}");
        let lane_id = format!("lane-mt004-{label}");
        let message_id = format!("message-mt004-{label}-proposal");
        let aggregate_id = match origin {
            "run" => run_id.as_str(),
            "lane" => lane_id.as_str(),
            "message" => message_id.as_str(),
            _ => unreachable!(),
        };
        harness
            .store
            .test_corrupt_scoped_authority(
                origin,
                aggregate_id,
                ModelLaneAuthorityTestCorruption::ProjectionScope,
            )
            .await
            .expect("retarget typed origin projection outside its receipt scope");

        let denied = match origin {
            "run" => harness.store.navigation_by_run(&run_id).await.is_err(),
            "lane" => harness.store.navigation_by_lane(&lane_id).await.is_err(),
            "message" => harness
                .store
                .navigation_by_message(&message_id)
                .await
                .is_err(),
            _ => unreachable!(),
        };
        assert!(denied, "{origin} origin must validate before run redirect");
        harness.cleanup().await;
    }
}

async fn seed_authority(store: &ModelLaneStore, label: &str, include_messages: bool) -> Seeded {
    store
        .record_run(sample_run(label))
        .await
        .expect("seed promotion run");
    store
        .record_lane(sample_lane(label))
        .await
        .expect("seed promotion lane");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding(label))
        .await
        .expect("seed promoted artifact authority");
    if !include_messages {
        return Seeded {
            proposal_version: 1,
        };
    }
    let proposal = store
        .record_message(sample_message(label, "proposal"))
        .await
        .expect("seed proposal advisory");
    store
        .record_message(sample_message(label, "critique"))
        .await
        .expect("seed critique advisory");
    Seeded {
        proposal_version: proposal.event_stream_version,
    }
}

fn sample_decision(
    label: &str,
    decision_id: &str,
    idempotency_key: &str,
    expected_version: i64,
) -> NewModelLanePromotionDecision {
    NewModelLanePromotionDecision {
        decision_id: decision_id.into(),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        decision_span_id: format!("span-{decision_id}"),
        parent_span_id: Some(format!("span-lane-mt004-{label}")),
        linked_span_contexts: vec![
            format!("span-message-mt004-{label}-proposal"),
            format!("span-message-mt004-{label}-critique"),
        ],
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane,
        routing_launch_plan: Vec::new(),
        input_refs: vec![
            format!("model-lane-message://message-mt004-{label}-critique"),
            format!("model-lane-message://message-mt004-{label}-proposal"),
        ],
        selected_input_refs: vec![format!(
            "model-lane-message://message-mt004-{label}-proposal"
        )],
        rejected_input_refs: vec![format!(
            "model-lane-message://message-mt004-{label}-critique"
        )],
        validator_authority_ref: None,
        operator_authority_ref: Some(format!("operator://mt004/{label}")),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: format!("message-mt004-{label}-proposal"),
        expected_event_ledger_version: expected_version,
        base_snapshot_ref: "not-applicable".into(),
        current_base_snapshot_ref: "not-applicable".into(),
        state_vector: "not-applicable".into(),
        current_state_vector: "not-applicable".into(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "lexicographic_selected_ref_then_lowest_event_seq".into(),
        promotion_gate_ref: format!("promotion-gate://mt004/{label}"),
        promotion_receipt_ref: Some(format!("promotion-receipt://mt004/{label}")),
        promoted_artifact_ref: Some(format!("artifact://mt004/{label}/promoted")),
        promoted_artifact_sha256: Some(promoted_artifact_hash(label)),
        promoted_artifact_version: Some("1".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("0020-{decision_id}"),
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:02:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "kernel_event_ledger",
            "operator_authority_ref": format!("operator://mt004/{label}")
        }),
    }
}

fn sample_artifact_binding(label: &str) -> NewModelLaneContextBundleArtifactBinding {
    let payload = promoted_artifact_payload(label);
    let hash = promoted_artifact_hash(label);
    let artifact_ref = format!("artifact://mt004/{label}/promoted");
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-mt004-{label}"),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        artifact_ref: artifact_ref.clone(),
        artifact_sha256: hash.clone(),
        content_hash: hash,
        artifact_kind: "model_lane_promoted_artifact".into(),
        artifact_manifest_ref: format!("artifact-manifest://mt004/{label}"),
        artifact_payload_ref: artifact_ref,
        payload_json: payload,
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: format!("artifact-binding-mt004-{label}"),
        created_at_utc: "2026-09-02T00:01:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn promoted_artifact_payload(label: &str) -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_promoted_artifact@1",
        "artifact_version": "1",
        "body": format!("deterministic promoted artifact {label}")
    })
}

fn promoted_artifact_hash(label: &str) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(&promoted_artifact_payload(label)).expect("serialize artifact"),
    ))
}

fn sample_run(label: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        run_span_id: format!("span-run-mt004-{label}"),
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane.as_str().into(),
        context_bundle_id: format!("context-mt004-{label}"),
        lane_ids: vec![format!("lane-mt004-{label}")],
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        artifact_namespace: format!("artifact://mt004/{label}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: format!("run-mt004-{label}"),
        replay_order_key: format!("0001-{label}"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        locus_binding: Some(sample_locus(label)),
        memory_pack_ref: format!("memory-pack://mt004/{label}"),
        memory_pack_hash: "a".repeat(64),
        determinism_mode: "strict".into(),
        budget_summary_ref: format!("budget://mt004/{label}"),
        selected_model_id: Some("model://local/mt004".into()),
        candidate_model_ids: vec!["model://local/mt004".into()],
        procedural_review_status: "approved".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

fn sample_lane(label: &str) -> NewModelLane {
    let session = format!("session-mt004-{label}");
    let model_session = format!("model-session-mt004-{label}");
    NewModelLane {
        lane_id: format!("lane-mt004-{label}"),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        lane_span_id: format!("span-lane-mt004-{label}"),
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        kind: ModelLaneKind::LocalModel,
        role: "proposal-author".into(),
        backend: "embedded-model-runtime".into(),
        model_id: Some("model://local/mt004".into()),
        session_id: session.clone(),
        model_session_id: model_session.clone(),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt004/context".into()],
        effective_capability_snapshot_ref: Some("capability://mt004/snapshot".into()),
        capability_negotiation_ref: Some("capability://mt004/negotiation".into()),
        provider_feature_profile_ref: Some("provider://mt004/local".into()),
        requested_execution_policy_ref: Some("execution://mt004/requested".into()),
        effective_execution_policy_ref: Some("execution://mt004/effective".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["tool-gate://mt004/context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-09-02T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-09-02T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-09-02T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel://mt004/{label}")),
        reclaim_policy_ref: Some("reclaim://mt004".into()),
        terminal_status_mapping_ref: Some("terminal://mt004".into()),
        process_ownership_ref: Some(format!("process://mt004/{label}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop://mt004".into()),
        last_runtime_status_ref: Some("runtime://mt004/ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding: Some(sample_locus_for(label, &session, &model_session)),
    }
}

fn sample_message(label: &str, kind: &str) -> NewModelLaneMessage {
    let message_id = format!("message-mt004-{label}-{kind}");
    NewModelLaneMessage {
        message_id: message_id.clone(),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-lane-mt004-{label}")),
        linked_span_contexts: vec![format!("trace-mt004-{label}")],
        from_lane_id: format!("lane-mt004-{label}"),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-mt004-{label}"),
            correlation_id: format!("correlation-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://mt004/{label}/{kind}"),
        payload_sha256: if kind == "proposal" {
            "b".repeat(64)
        } else {
            "c".repeat(64)
        },
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        summary: format!("{kind} advisory"),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["tool-gate://mt004/context".into()],
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding: Some(sample_locus(label)),
        idempotency_key: format!("message-mt004-{label}-{kind}"),
        replay_order_key: format!("0010-{kind}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:01:30Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn sample_promoted_message(label: &str, decision_id: &str, index: usize) -> NewModelLaneMessage {
    let artifact_ref = format!("artifact://mt004/{label}/promoted");
    let mut message = sample_message(label, &format!("promoted-{index}"));
    message.kind = ModelLaneMessageKind::PromotionRequest;
    message.authority = ModelLaneAuthority::Promoted;
    message.payload_ref = artifact_ref.clone();
    message.payload_sha256 = promoted_artifact_hash(label);
    message.promotion_decision_id = Some(decision_id.into());
    message.promotion_gate_ref = Some(format!("promotion-gate://mt004/{label}"));
    message.promotion_receipt_ref = Some(format!("promotion-receipt://mt004/{label}"));
    message.validator_verdict_ref = Some(format!("validator://mt004/{label}"));
    message.operator_decision_ref = Some(format!("operator://mt004/{label}"));
    message.promoted_artifact_ref = Some(artifact_ref);
    message.promoted_artifact_sha256 = Some(promoted_artifact_hash(label));
    message.promoted_artifact_version = Some("1".into());
    message
}

fn sample_locus(label: &str) -> ModelLaneLocusBinding {
    sample_locus_for(
        label,
        &format!("session-mt004-{label}"),
        &format!("model-session-mt004-{label}"),
    )
}

fn sample_locus_for(
    label: &str,
    session_id: &str,
    model_session_id: &str,
) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding_ref: format!("locus://wp1/mt004/{label}"),
    }
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt004-{label}")).expect("nonblank workspace"),
        )
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let workspace = scope.workspace.clone().expect("exact workspace");
    let session = scope.authenticated_session.expect("exact session");
    let access_space = scope.access_space.expect("exact access space");
    vec![
        ResourceScope::new(OwnerAccountId::mint(), scope.actor_principal_id)
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, ActorPrincipalId::mint())
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(session)
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(workspace),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(
                WorkspaceScopeRef::new("workspace-mt004-foreign").expect("nonblank workspace"),
            ),
    ]
}
