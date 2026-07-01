//! WP-1 MT-004: ModelLane routing and promotion proof.
//!
//! These tests use real PostgreSQL/EventLedger through `knowledge_pg_support`.
//! There is no SQLite, in-memory, or mock authority fallback.

mod knowledge_pg_support;

use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLanePromotionDenialReason, ModelLanePromotionOutcome,
    ModelLanePromotionState, ModelLaneProviderKind, ModelLaneRecoveryState, ModelLaneRoutingPolicy,
    ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneMessage,
    NewModelLanePromotionDecision, NewModelLaneRun, RuntimeBinding,
};
use serde_json::json;

#[tokio::test]
async fn model_lane_promotion_appends_eventledger_and_replays_decision() {
    let (pool, store) = promotion_store().await;
    seed_run_lanes_and_messages(&store).await;

    assert_eq!(
        ModelLaneRoutingPolicy::all()
            .iter()
            .map(ModelLaneRoutingPolicy::as_str)
            .collect::<Vec<_>>(),
        vec![
            "local_first",
            "cloud_review",
            "cloud_plan_local_execute",
            "parallel_debate",
            "validator_lane",
            "operator_lane",
        ],
        "MT-004 routing policies must be typed and exhaustive"
    );

    let expected_version = event_sequence(&pool, "model_lane_message", "msg-local-proposal").await;
    let decision = approved_decision("decision-mt004-001", "idem-mt004-001", expected_version);
    let stored = store
        .record_promotion_decision(decision.clone())
        .await
        .expect("approved promotion decision persists");

    assert_eq!(stored.outcome, ModelLanePromotionOutcome::Approved);
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
    assert_eq!(
        stored.canonical_input_refs,
        vec![
            "model-lane-message://msg-cloud-critique".to_string(),
            "model-lane-message://msg-local-proposal".to_string(),
        ],
        "decision hash basis must sort input refs"
    );
    assert_eq!(
        stored.canonical_selected_input_refs,
        vec!["model-lane-message://msg-local-proposal".to_string()]
    );
    assert_eq!(
        stored.canonical_rejected_input_refs,
        vec!["model-lane-message://msg-cloud-critique".to_string()]
    );
    assert!(stored.event_ledger_event_id.starts_with("KE-"));
    assert!(stored.event_ledger_seq > expected_version);

    let replay = store
        .replay_promotion_decisions("run-mt004")
        .await
        .expect("promotion decisions replay");
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].decision_id, "decision-mt004-001");
    assert_eq!(
        replay[0].canonical_decision_hash,
        stored.canonical_decision_hash
    );
    assert_eq!(
        replay[0].promotion_gate_ref,
        "promotion-gate://dexterity/model-lane/mt004"
    );
    assert_eq!(
        replay[0].promotion_receipt_ref.as_deref(),
        Some("promotion-receipt://operator-review/mt004-001")
    );

    let ledger_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type = 'model_lane_promotion_decision' \
           AND event_type = 'PROMOTION_ACCEPTED'",
    )
    .bind("event-ledger://mt004/run")
    .fetch_one(&pool)
    .await
    .expect("count promotion EventLedger rows");
    assert_eq!(ledger_rows, 1);

    let same_retry = store
        .record_promotion_decision(decision)
        .await
        .expect("same idempotency and payload returns existing decision");
    assert_eq!(same_retry.event_ledger_event_id, stored.event_ledger_event_id);
    assert_eq!(same_retry.canonical_decision_hash, stored.canonical_decision_hash);

    let mut divergent = approved_decision("decision-mt004-002", "idem-mt004-001", expected_version);
    divergent.selected_input_refs = vec!["model-lane-message://msg-cloud-critique".into()];
    let err = store
        .record_promotion_decision(divergent)
        .await
        .expect_err("divergent duplicate idempotency must fail closed");
    assert!(
        err.to_string().contains("idempotency"),
        "expected idempotency conflict, got {err}"
    );
}

#[tokio::test]
async fn model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation() {
    let (pool, store) = promotion_store().await;
    seed_run_lanes_and_messages(&store).await;
    let expected_version = event_sequence(&pool, "model_lane_message", "msg-local-proposal").await;

    let stale_base = store
        .record_promotion_decision({
            let mut decision = approved_decision("decision-stale-base", "idem-stale-base", expected_version);
            decision.current_base_snapshot_ref = "crdt-snapshot://mt004/newer-base".into();
            decision.current_state_vector = "sv:mt004:2".into();
            decision
        })
        .await
        .expect("stale base denial persists");
    assert_eq!(stale_base.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(
        stale_base.denial_reason,
        Some(ModelLanePromotionDenialReason::StaleBase {
            expected_base_snapshot_ref: "crdt-snapshot://mt004/base".into(),
            current_base_snapshot_ref: "crdt-snapshot://mt004/newer-base".into(),
            expected_state_vector: "sv:mt004:1".into(),
            current_state_vector: "sv:mt004:2".into(),
        })
    );
    assert_eq!(
        stale_base.recovery_hint_ref.as_deref(),
        Some("recovery://dexterity/model-lane-promotion/stale-base")
    );

    let schema_mismatch = store
        .record_promotion_decision({
            let mut decision =
                approved_decision("decision-schema-mismatch", "idem-schema-mismatch", expected_version);
            decision.schema_id = "hsk.model_lane_message@999".into();
            decision
        })
        .await
        .expect("schema mismatch denial persists");
    assert!(matches!(
        schema_mismatch.denial_reason,
        Some(ModelLanePromotionDenialReason::SchemaMismatch { .. })
    ));

    let aggregate_mismatch = store
        .record_promotion_decision({
            let mut decision =
                approved_decision("decision-aggregate-mismatch", "idem-aggregate-mismatch", 1);
            if expected_version == 1 {
                decision.expected_event_ledger_version = 0;
            }
            decision
        })
        .await
        .expect("aggregate version mismatch denial persists");
    assert!(matches!(
        aggregate_mismatch.denial_reason,
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch { .. })
    ));

    let direct_mutation = store
        .record_promotion_decision({
            let mut decision =
                approved_decision("decision-direct-mutation", "idem-direct-mutation", expected_version);
            decision.direct_authority_mutation_attempt_ref =
                Some("direct-mutation://forbidden/model-lane-message/promoted".into());
            decision
        })
        .await
        .expect("direct mutation denial persists");
    assert_eq!(
        direct_mutation.denial_reason,
        Some(ModelLanePromotionDenialReason::DirectAuthorityMutation {
            attempted_ref: "direct-mutation://forbidden/model-lane-message/promoted".into(),
        })
    );
    assert_eq!(
        direct_mutation.state_history,
        vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::Denied,
        ]
    );

    let promoted_messages: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_messages WHERE authority = 'promoted'")
            .fetch_one(&pool)
            .await
            .expect("count promoted direct messages");
    assert_eq!(
        promoted_messages, 0,
        "advisory ModelLane output must not become authority through direct mutation"
    );

    let rejected_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE session_run_id = $1 \
           AND aggregate_type = 'model_lane_promotion_decision' \
           AND event_type = 'PROMOTION_REJECTED'",
    )
    .bind("event-ledger://mt004/run")
    .fetch_one(&pool)
    .await
    .expect("count promotion rejection EventLedger rows");
    assert_eq!(rejected_events, 4);

    let replay = store
        .replay_promotion_decisions("run-mt004")
        .await
        .expect("denied promotion decisions replay");
    assert_eq!(replay.len(), 4);
    assert!(replay.iter().all(|record| record.outcome == ModelLanePromotionOutcome::Denied));
}

#[tokio::test]
async fn model_lane_promotion_reordered_inputs_keep_same_decision_hash() {
    let (pool, store) = promotion_store().await;
    seed_run_lanes_and_messages(&store).await;
    let expected_version = event_sequence(&pool, "model_lane_message", "msg-local-proposal").await;

    let left = store
        .record_promotion_decision(approved_decision(
            "decision-order-left",
            "idem-order-left",
            expected_version,
        ))
        .await
        .expect("left order decision persists");

    let mut reordered = approved_decision("decision-order-right", "idem-order-right", expected_version);
    reordered.input_refs.reverse();
    reordered.selected_input_refs.reverse();
    reordered.rejected_input_refs.reverse();
    let right = store
        .record_promotion_decision(reordered)
        .await
        .expect("right order decision persists");

    assert_eq!(
        left.canonical_decision_hash, right.canonical_decision_hash,
        "reordered input refs must not perturb deterministic decision hash"
    );
    assert_eq!(left.canonical_input_refs, right.canonical_input_refs);
    assert_eq!(left.canonical_selected_input_refs, right.canonical_selected_input_refs);
    assert_eq!(left.canonical_rejected_input_refs, right.canonical_rejected_input_refs);
}

async fn promotion_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-004 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated ModelLane promotion schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

async fn seed_run_lanes_and_messages(store: &ModelLaneStore) {
    store
        .record_run(sample_run())
        .await
        .expect("record MT-004 run");
    store
        .record_lane(sample_lane(
            "lane-local",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record local lane");
    store
        .record_lane(sample_lane(
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
        ))
        .await
        .expect("record cloud lane");
    store
        .record_message(sample_message(
            "msg-local-proposal",
            "lane-local",
            ModelLaneMessageKind::Proposal,
        ))
        .await
        .expect("record local advisory proposal");
    store
        .record_message(sample_message(
            "msg-cloud-critique",
            "lane-cloud",
            ModelLaneMessageKind::Critique,
        ))
        .await
        .expect("record cloud advisory critique");
}

async fn event_sequence(pool: &sqlx::PgPool, aggregate_type: &str, aggregate_id: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT event_sequence FROM kernel_event_ledger \
         WHERE aggregate_type = $1 AND aggregate_id = $2 \
         ORDER BY event_sequence DESC LIMIT 1",
    )
    .bind(aggregate_type)
    .bind(aggregate_id)
    .fetch_one(pool)
    .await
    .expect("read EventLedger aggregate version")
}

fn approved_decision(
    decision_id: &str,
    idempotency_key: &str,
    expected_event_ledger_version: i64,
) -> NewModelLanePromotionDecision {
    NewModelLanePromotionDecision {
        decision_id: decision_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        decision_span_id: format!("span-{decision_id}"),
        parent_span_id: Some("span-msg-local-proposal".into()),
        linked_span_contexts: vec!["span-msg-local-proposal".into(), "span-msg-cloud-critique".into()],
        coordinator_session_id: "coordinator-session-mt004".into(),
        routing_policy: ModelLaneRoutingPolicy::ParallelDebate,
        input_refs: vec![
            "model-lane-message://msg-cloud-critique".into(),
            "model-lane-message://msg-local-proposal".into(),
        ],
        selected_input_refs: vec!["model-lane-message://msg-local-proposal".into()],
        rejected_input_refs: vec!["model-lane-message://msg-cloud-critique".into()],
        validator_authority_ref: Some("validator-verdict://mt004/validator-pass".into()),
        operator_authority_ref: Some("operator-decision://mt004/approve-local-proposal".into()),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: "msg-local-proposal".into(),
        expected_event_ledger_version,
        base_snapshot_ref: "crdt-snapshot://mt004/base".into(),
        current_base_snapshot_ref: "crdt-snapshot://mt004/base".into(),
        state_vector: "sv:mt004:1".into(),
        current_state_vector: "sv:mt004:1".into(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "lexicographic_lowest_payload_sha256_then_message_id".into(),
        promotion_gate_ref: "promotion-gate://dexterity/model-lane/mt004".into(),
        promotion_receipt_ref: Some("promotion-receipt://operator-review/mt004-001".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: "event-ledger://mt004/run".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("{decision_id}:promotion"),
        recovery_hint_ref: None,
        created_at_utc: "2026-06-29T12:00:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "WIRED: kernel_event_ledger promotion decision",
            "internal_diagnostics": "DEFERRED: internal diagnostics owned by WP-KERNEL-012",
            "palmistry": "DEFERRED: Palmistry owned by separate worktree"
        }),
    }
}

fn sample_run() -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        run_span_id: "span-run-mt004".into(),
        coordinator_session_id: "coordinator-session-mt004".into(),
        routing_policy: "parallel_debate".into(),
        context_bundle_id: "context-bundle://mt004/run".into(),
        lane_ids: vec!["lane-local".into(), "lane-cloud".into()],
        event_ledger_stream_id: "event-ledger://mt004/run".into(),
        artifact_namespace: "artifact://mt004/run".into(),
        projection_plan_ref: Some("projection-plan://mt004/cloud-review".into()),
        consent_receipt_ref: Some("consent://mt004/byok-cloud-review".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        idempotency_key: "idem-run-mt004".into(),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#replay".into()),
        locus_binding: Some(sample_locus("session-run-mt004", "model-session-run-mt004")),
        memory_pack_ref: "memory-pack://fems/mt004/run".into(),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt004/run".into(),
        selected_model_id: Some("model://mt004/local-planner".into()),
        candidate_model_ids: vec![
            "model://mt004/local-planner".into(),
            "model://mt004/cloud-reviewer".into(),
        ],
        procedural_review_status: "mt004-routing-promotion-preflight".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_lane(
    lane_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
) -> NewModelLane {
    let provider_kind = match runtime_binding {
        RuntimeBinding::Local => ModelLaneProviderKind::LocalRuntime,
        RuntimeBinding::Cloud => ModelLaneProviderKind::Anthropic,
        RuntimeBinding::CliBridge => ModelLaneProviderKind::OfficialCli,
        RuntimeBinding::Human => ModelLaneProviderKind::Human,
        RuntimeBinding::Subagent => ModelLaneProviderKind::Subagent,
        RuntimeBinding::Validator => ModelLaneProviderKind::Validator,
    };
    let process_backed = matches!(
        runtime_binding,
        RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge
    );
    let session_id = format!("session-{lane_id}");
    let model_session_id = format!("model-session-{lane_id}");
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: "event-ledger://mt004/run".into(),
        kind,
        role: format!("role-{lane_id}"),
        backend: format!("{runtime_binding:?}").to_ascii_lowercase(),
        model_id: Some(format!("model://mt004/{lane_id}")),
        session_id: session_id.clone(),
        model_session_id: model_session_id.clone(),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding: runtime_binding.clone(),
        launch_authority,
        provider_kind,
        capability_token_ids: vec!["capability://mt004/read-context".into()],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://mt004/{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://mt004/{lane_id}")),
        provider_feature_profile_ref: Some(format!(
            "provider-feature-profile://{}",
            provider_kind.as_str()
        )),
        requested_execution_policy_ref: Some(format!(
            "execution-policy://requested/{}",
            runtime_binding.as_str()
        )),
        effective_execution_policy_ref: Some("execution-policy://effective/mt004".into()),
        projection_plan_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some("projection-plan://mt004/cloud-review".into()),
        consent_receipt_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some("consent://mt004/byok-cloud-review".into()),
        tool_gate_decision_refs: vec!["toolgate://mt004/allow-read-context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-29T12:00:00Z".into()),
        lease_expires_at_utc: Some("2026-06-29T12:05:00Z".into()),
        reclaim_after_utc: Some("2026-06-29T12:06:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt004/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt004/bounded".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt004/model-lane".into()),
        process_ownership_ref: process_backed.then_some(format!("process-ledger://mt004/{lane_id}")),
        no_os_process_reason_ref: (!process_backed).then_some(format!("no-os://mt004/{lane_id}")),
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt004/bounded".into()),
        last_runtime_status_ref: Some(format!("runtime-status://mt004/{lane_id}")),
        last_recovery_event_ref: Some(format!("event-ledger://mt004/{lane_id}/ready")),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#lane".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus(&session_id, &model_session_id)),
    }
}

fn sample_message(
    message_id: &str,
    from_lane_id: &str,
    kind: ModelLaneMessageKind,
) -> NewModelLaneMessage {
    let is_proposal = kind == ModelLaneMessageKind::Proposal;
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-{from_lane_id}")),
        linked_span_contexts: vec!["span-lane-local".into(), "span-lane-cloud".into()],
        from_lane_id: from_lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        kind,
        payload_ref: format!("artifact://mt004/messages/{message_id}"),
        payload_sha256: sample_sha256(),
        event_ledger_stream_id: "event-ledger://mt004/run".into(),
        summary: format!("{from_lane_id} emits advisory {message_id}"),
        authority: ModelLaneAuthority::Advisory,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt004/allow-read-context".into()],
        coordinator_session_id: "coordinator-session-mt004".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus("session-lane-local", "model-session-lane-local")),
        idempotency_key: format!("idem-{message_id}"),
        replay_order_key: format!("00000002/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: is_proposal.then_some(format!("proposal://mt004/{message_id}")),
        crdt_update_ref: is_proposal.then_some(format!("crdt-update://mt004/{message_id}")),
        crdt_base_snapshot_ref: is_proposal.then_some("crdt-snapshot://mt004/base".into()),
        crdt_state_vector: is_proposal.then_some("sv:mt004:1".into()),
        crdt_proposal_ref: is_proposal.then_some(format!("crdt-proposal://mt004/{message_id}")),
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#message".into()),
        created_at_utc: "2026-06-29T12:00:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "WIRED",
            "internal_diagnostics": "DEFERRED: WP-KERNEL-012",
            "palmistry": "DEFERRED: separate worktree"
        }),
    }
}

fn sample_locus(session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: "coordinator-session-mt004".into(),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding_ref: "locus://wp1/mt004/coordinator-session-mt004".into(),
    }
}

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}
