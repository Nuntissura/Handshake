//! WP-1 MT-004: Dexterity promotion/routing runtime proof.
//!
//! These tests use real PostgreSQL plus the kernel EventLedger. They prove that
//! model-lane outputs remain advisory until Dexterity records an explicit,
//! replayable promotion decision with CRDT/schema/version guards.

mod knowledge_pg_support;
mod model_lane_cloud_support;

use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLanePromotionDenialReason, ModelLanePromotionOutcome,
    ModelLanePromotionState, ModelLaneProviderKind, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneRoutingPolicy, ModelLaneStatus, ModelLaneStore,
    ModelLaneTarget, NewModelLane, NewModelLaneMessage, NewModelLanePromotionDecision,
    NewModelLaneRun, RuntimeBinding,
};
use serde_json::json;

#[tokio::test]
async fn model_lane_promotion_appends_eventledger_and_replays_decision() {
    let (pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;

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
        "Dexterity must expose every MT-004 routing policy as typed Rust data"
    );

    let decision = sample_decision(
        "decision-approved-001",
        "idem-promotion-approved-001",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    let stored = store
        .record_promotion_decision(decision.clone())
        .await
        .expect("record approved promotion decision");

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
        ],
        "promotion must be a deterministic state machine, not a boolean"
    );
    assert_eq!(
        stored.canonical_input_refs,
        vec![
            "model-lane-message://msg-critique-001".to_string(),
            "model-lane-message://msg-proposal-001".to_string(),
        ]
    );
    assert_eq!(
        stored.selected_input_refs,
        vec!["model-lane-message://msg-proposal-001".to_string()]
    );
    assert_eq!(
        stored.rejected_input_refs,
        vec!["model-lane-message://msg-critique-001".to_string()]
    );
    assert_eq!(
        stored.validator_authority_ref.as_deref(),
        Some("validator://mt004/v1")
    );
    assert_eq!(
        stored.operator_authority_ref.as_deref(),
        Some("operator://mt004/approve")
    );
    assert_eq!(stored.promotion_gate_ref, "promotion-gate://mt004/approved");
    assert_eq!(
        stored.promotion_receipt_ref.as_deref(),
        Some("promotion-receipt://mt004/approved")
    );
    assert_eq!(stored.denial_reason, None);
    assert!(stored.event_ledger_event_id.starts_with("KE-"));
    assert!(
        stored.event_ledger_seq > seeded.proposal_event_seq,
        "promotion decision must append after the selected advisory message"
    );
    assert_eq!(
        stored.canonical_hash_basis["input_refs"],
        json!([
            "model-lane-message://msg-critique-001",
            "model-lane-message://msg-proposal-001"
        ])
    );
    assert_eq!(
        stored.canonical_hash_basis["selected_input_refs"],
        json!(["model-lane-message://msg-proposal-001"])
    );
    assert_eq!(
        stored.canonical_hash_basis["promoted_artifact"]["ref"],
        json!("artifact://mt004/promoted/msg-promoted-001")
    );

    let ledger_row: (String, String, String) = sqlx::query_as(
        "SELECT event_type, aggregate_type, aggregate_id \
         FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&stored.event_ledger_event_id)
    .fetch_one(&pool)
    .await
    .expect("promotion decision EventLedger row");
    assert_eq!(ledger_row.0, "PROMOTION_ACCEPTED");
    assert_eq!(ledger_row.1, "model_lane_promotion_decision");
    assert_eq!(ledger_row.2, "decision-approved-001");

    let replay = store
        .replay_promotion_decisions("run-mt004")
        .await
        .expect("replay promotion decisions");
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].decision_id, stored.decision_id);
    assert_eq!(
        replay[0].canonical_decision_hash,
        stored.canonical_decision_hash
    );

    let duplicate = store
        .record_promotion_decision(decision)
        .await
        .expect("same idempotency and same content replays existing decision");
    assert_eq!(
        duplicate.event_ledger_event_id,
        stored.event_ledger_event_id
    );
    assert_eq!(
        duplicate.canonical_decision_hash,
        stored.canonical_decision_hash
    );

    let promoted = store
        .record_message(promoted_message(
            "msg-promoted-001",
            "idem-message-promoted-001",
            "decision-approved-001",
            "promotion-gate://mt004/approved",
            "promotion-receipt://mt004/approved",
        ))
        .await
        .expect("accepted PromotionGate decision allows promoted authority message");
    assert_eq!(promoted.authority, ModelLaneAuthority::Promoted);

    let wrong_artifact_err = store
        .record_message(promoted_message(
            "msg-promoted-wrong-artifact",
            "idem-message-promoted-wrong-artifact",
            "decision-approved-001",
            "promotion-gate://mt004/approved",
            "promotion-receipt://mt004/approved",
        ))
        .await
        .expect_err("approved decision cannot authorize a different promoted artifact");
    assert!(
        wrong_artifact_err.to_string().contains("artifact binding"),
        "promoted message must bind to the exact approved artifact: {wrong_artifact_err}"
    );

    let registry_rows = store
        .schema_registry_rows()
        .await
        .expect("schema registry rows");
    assert!(
        registry_rows
            .iter()
            .any(|row| row.schema_id == "hsk.model_lane_promotion_decision@1"),
        "promotion decision schema must be registered for state recovery"
    );
}

#[tokio::test]
async fn model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation() {
    let (_pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;

    let mut stale_base = sample_decision(
        "decision-denied-stale-base",
        "idem-promotion-denied-stale-base",
        ModelLaneRoutingPolicy::CloudPlanLocalExecute,
        seeded.proposal_event_seq,
    );
    stale_base.base_snapshot_ref = "crdt-snapshot://mt004/base-stale".into();
    stale_base.current_base_snapshot_ref = "crdt-snapshot://mt004/base-stale".into();
    let stale_record = store
        .record_promotion_decision(stale_base.clone())
        .await
        .expect("stale base denial is durable");
    assert_eq!(stale_record.outcome, ModelLanePromotionOutcome::Denied);
    assert_eq!(stale_record.final_state, ModelLanePromotionState::Denied);
    assert_eq!(
        stale_record.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );
    assert_eq!(
        stale_record.state_history,
        vec![
            ModelLanePromotionState::Advisory,
            ModelLanePromotionState::PromotionRequested,
            ModelLanePromotionState::PendingPolicy,
            ModelLanePromotionState::Denied,
        ]
    );
    assert_eq!(
        stale_record.current_base_snapshot_ref, "not-applicable",
        "nonshared advisory input must not manufacture a current CRDT base from caller input"
    );
    assert!(stale_record
        .recovery_hint_ref
        .as_deref()
        .expect("denial recovery hint")
        .contains("usermanual://model-lane-promotion"));

    let mut duplicate_changed = stale_base;
    duplicate_changed.schema_id = "hsk.model_lane_message@2".into();
    let duplicate_err = store
        .record_promotion_decision(duplicate_changed)
        .await
        .expect_err("same idempotency with changed content must conflict");
    assert!(
        duplicate_err.to_string().contains("idempotency"),
        "duplicate idempotency conflict must be explicit: {duplicate_err}"
    );

    let mut schema_mismatch = sample_decision(
        "decision-denied-schema",
        "idem-promotion-denied-schema",
        ModelLaneRoutingPolicy::CloudReview,
        seeded.proposal_event_seq,
    );
    schema_mismatch.schema_id = "hsk.model_lane_message@2".into();
    let schema_record = store
        .record_promotion_decision(schema_mismatch)
        .await
        .expect("schema mismatch denial is durable");
    assert_eq!(
        schema_record.denial_reason,
        Some(ModelLanePromotionDenialReason::SchemaMismatch)
    );
    assert_eq!(
        schema_record.current_schema_id.as_deref(),
        Some("hsk.model_lane_message@1")
    );

    let mut stale_state = sample_decision(
        "decision-denied-stale-state",
        "idem-promotion-denied-stale-state",
        ModelLaneRoutingPolicy::ParallelDebate,
        seeded.proposal_event_seq,
    );
    stale_state.state_vector = "sv:mt004:stale".into();
    stale_state.current_state_vector = "sv:mt004:stale".into();
    let stale_state_record = store
        .record_promotion_decision(stale_state)
        .await
        .expect("stale state-vector denial is durable");
    assert_eq!(
        stale_state_record.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );
    assert_eq!(
        stale_state_record.current_state_vector, "not-applicable",
        "nonshared advisory input must not manufacture a current CRDT state vector"
    );

    let mut version_mismatch = sample_decision(
        "decision-denied-version",
        "idem-promotion-denied-version",
        ModelLaneRoutingPolicy::ValidatorLane,
        seeded.proposal_event_seq - 1,
    );
    version_mismatch.expected_event_ledger_version = seeded.proposal_event_seq - 1;
    let version_record = store
        .record_promotion_decision(version_mismatch)
        .await
        .expect("EventLedger version mismatch denial is durable");
    assert_eq!(
        version_record.denial_reason,
        Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
    );
    assert_eq!(
        version_record.current_event_ledger_version,
        Some(seeded.proposal_event_seq)
    );

    let mut direct_mutation = sample_decision(
        "decision-denied-direct-mutation",
        "idem-promotion-denied-direct-mutation",
        ModelLaneRoutingPolicy::OperatorLane,
        seeded.proposal_event_seq,
    );
    direct_mutation.direct_authority_mutation_attempt_ref =
        Some("model-lane-message://msg-direct-promoted".into());
    let direct_record = store
        .record_promotion_decision(direct_mutation)
        .await
        .expect("direct mutation denial is durable");
    assert_eq!(
        direct_record.denial_reason,
        Some(ModelLanePromotionDenialReason::DirectAuthorityMutation)
    );

    let mut phantom_ref = sample_decision(
        "decision-denied-phantom-ref",
        "idem-promotion-denied-phantom-ref",
        ModelLaneRoutingPolicy::CloudReview,
        seeded.proposal_event_seq,
    );
    phantom_ref.input_refs = vec![
        "model-lane-message://msg-proposal-001".into(),
        "model-lane-message://msg-phantom-404".into(),
    ];
    phantom_ref.selected_input_refs = vec!["model-lane-message://msg-phantom-404".into()];
    phantom_ref.rejected_input_refs = vec!["model-lane-message://msg-proposal-001".into()];
    phantom_ref.expected_event_ledger_aggregate_id = "msg-phantom-404".into();
    let phantom_record = store
        .record_promotion_decision(phantom_ref)
        .await
        .expect("phantom advisory ref denial is durable");
    assert_eq!(
        phantom_record.denial_reason,
        Some(ModelLanePromotionDenialReason::InputRefMismatch)
    );

    let direct_message_err = store
        .record_message(promoted_message(
            "msg-direct-promoted",
            "idem-message-direct-promoted",
            "decision-missing",
            "promotion-gate://mt004/missing",
            "promotion-receipt://mt004/missing",
        ))
        .await
        .expect_err("promoted authority message cannot bypass PromotionGate");
    assert!(
        direct_message_err
            .to_string()
            .contains("PromotionGate resolution"),
        "direct authority mutation must fail closed with recovery wording: {direct_message_err}"
    );

    let replay = store
        .replay_promotion_decisions("run-mt004")
        .await
        .expect("denial replay");
    assert_eq!(replay.len(), 6);
    assert!(
        replay
            .iter()
            .all(|record| record.outcome == ModelLanePromotionOutcome::Denied),
        "all negative cases must remain durable/replayable denials"
    );
}

#[tokio::test]
async fn model_lane_promotion_reordered_inputs_keep_same_decision_hash() {
    let (_pool, store) = model_lane_store().await;
    let seeded = seed_run_with_advisory_messages(&store).await;

    let first = store
        .record_promotion_decision(sample_decision(
            "decision-hash-001",
            "idem-promotion-hash-001",
            ModelLaneRoutingPolicy::LocalFirst,
            seeded.proposal_event_seq,
        ))
        .await
        .expect("first decision");

    let mut reordered = sample_decision(
        "decision-hash-002",
        "idem-promotion-hash-002",
        ModelLaneRoutingPolicy::LocalFirst,
        seeded.proposal_event_seq,
    );
    reordered.input_refs = vec![
        "model-lane-message://msg-proposal-001".into(),
        "model-lane-message://msg-critique-001".into(),
    ];
    reordered.selected_input_refs = vec!["model-lane-message://msg-proposal-001".into()];
    reordered.rejected_input_refs = vec!["model-lane-message://msg-critique-001".into()];
    let second = store
        .record_promotion_decision(reordered)
        .await
        .expect("reordered decision");

    assert_eq!(first.canonical_input_refs, second.canonical_input_refs);
    assert_eq!(first.canonical_hash_basis, second.canonical_hash_basis);
    assert_eq!(
        first.canonical_decision_hash, second.canonical_decision_hash,
        "input ref order, decision row id, and idempotency key must not change canonical decision hash"
    );
}

struct SeededRun {
    proposal_event_seq: i64,
}

async fn model_lane_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-004 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated model-lane promotion schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

async fn seed_run_with_advisory_messages(store: &ModelLaneStore) -> SeededRun {
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
            ModelLaneProviderKind::LocalRuntime,
        ))
        .await
        .expect("record local lane");

    // Cloud lanes fail closed unless durable ProjectionPlan/ConsentReceipt
    // authority already exists (spec 4.3.9.2.5, CX-MM-007). Seed the cloud
    // lane's authority before recording it, matching the identity that
    // `sample_lane("lane-cloud", Cloud, ...)` stamps.
    model_lane_cloud_support::seed_cloud_lane_authority(
        store,
        model_lane_cloud_support::CloudLaneAuthoritySpec {
            run_id: "run-mt004",
            lane_id: "lane-cloud",
            model_session_id: "model-session-lane-cloud",
            provider_kind: ModelLaneProviderKind::OpenAi.as_str(),
            requested_model_id: "model://mt004/lane-cloud",
            projection_plan_id: "projection-plan://lane-cloud",
            consent_receipt_id: "consent://lane-cloud",
            event_ledger_stream_id: "mlane-stream-run-mt004",
            work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
            micro_task_id: "MT-004",
            task_board_id: "task-board://wp-1",
            owner_session: "KERNEL_BUILDER-MT004",
        },
    )
    .await;

    store
        .record_lane(sample_lane(
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
            ModelLaneProviderKind::OpenAi,
        ))
        .await
        .expect("record cloud lane");

    let proposal = store
        .record_message(advisory_message(
            "msg-proposal-001",
            "idem-message-proposal-001",
            "lane-local",
            ModelLaneMessageKind::Proposal,
            "local lane proposes CRDT edit",
        ))
        .await
        .expect("record advisory proposal");
    store
        .record_message(advisory_message(
            "msg-critique-001",
            "idem-message-critique-001",
            "lane-cloud",
            ModelLaneMessageKind::Critique,
            "cloud lane critiques local plan",
        ))
        .await
        .expect("record advisory critique");

    SeededRun {
        proposal_event_seq: proposal.event_ledger_seq,
    }
}

fn sample_run() -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        run_span_id: "span-run-mt004".into(),
        coordinator_session_id: "coordinator-session-mt004".into(),
        routing_policy: ModelLaneRoutingPolicy::ParallelDebate.as_str().into(),
        context_bundle_id: "context-bundle://mt004".into(),
        lane_ids: vec!["lane-local".into(), "lane-cloud".into()],
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        artifact_namespace: "artifact://model-lane/mt004".into(),
        projection_plan_ref: Some("projection-plan://mt004/cloud-review".into()),
        consent_receipt_ref: Some("consent://mt004/cloud-review".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        idempotency_key: "idem-run-mt004".into(),
        replay_order_key: "00000000/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#run".into()),
        locus_binding: Some(sample_locus("session-run", "model-session-run")),
        memory_pack_ref: "memory-pack://mt004".into(),
        memory_pack_hash: sample_sha256('a'),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt004".into(),
        selected_model_id: Some("model://mt004/local".into()),
        candidate_model_ids: vec!["model://mt004/local".into(), "model://mt004/cloud".into()],
        procedural_review_status: "runtime_promotion_preflight".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec!["rejection://mt004/no-direct-authority".into()],
    }
}

fn sample_lane(
    lane_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
    provider_kind: ModelLaneProviderKind,
) -> NewModelLane {
    let process_backed = !matches!(
        runtime_binding,
        RuntimeBinding::Human | RuntimeBinding::Subagent | RuntimeBinding::Validator
    );
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        kind,
        role: format!("role-{lane_id}"),
        backend: format!("backend-{lane_id}"),
        model_id: Some(format!("model://mt004/{lane_id}")),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding: runtime_binding.clone(),
        launch_authority,
        provider_kind,
        capability_token_ids: vec!["capability://mt004/read-context".into()],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://{lane_id}")),
        provider_feature_profile_ref: Some(format!("provider-feature-profile://{lane_id}")),
        requested_execution_policy_ref: Some(format!("execution-policy://requested/{lane_id}")),
        effective_execution_policy_ref: Some(format!("execution-policy://effective/{lane_id}")),
        projection_plan_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some(format!("projection-plan://{lane_id}")),
        consent_receipt_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some(format!("consent://{lane_id}")),
        tool_gate_decision_refs: vec!["toolgate://mt004/read-context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-29T08:00:00Z".into()),
        lease_expires_at_utc: Some("2026-06-29T08:05:00Z".into()),
        reclaim_after_utc: Some("2026-06-29T08:06:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt004".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt004".into()),
        process_ownership_ref: process_backed.then_some(format!("process-ledger://{lane_id}")),
        no_os_process_reason_ref: (!process_backed).then_some(format!("no-os://{lane_id}")),
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt004".into()),
        last_runtime_status_ref: Some("runtime-status://ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#lane".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus(
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
    }
}

fn advisory_message(
    message_id: &str,
    idempotency_key: &str,
    lane_id: &str,
    kind: ModelLaneMessageKind,
    summary: &str,
) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec!["span-coordinator-mt004".into()],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(sample_routing(
            &format!("corr-{message_id}"),
            "coordinator",
            "coordinator-session-mt004",
        )),
        kind,
        payload_ref: format!("artifact://mt004/{message_id}"),
        payload_sha256: sample_sha256('b'),
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        summary: summary.into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt004/read-context".into()],
        coordinator_session_id: "coordinator-session-mt004".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus(
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000010/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#advisory".into()),
        created_at_utc: "2026-06-29T08:01:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger-backed",
            "locus": "locus://wp1/mt004/coordinator-session-mt004",
            "palmistry": "external watcher link expected when feature is available"
        }),
    }
}

fn promoted_message(
    message_id: &str,
    idempotency_key: &str,
    promotion_decision_id: &str,
    promotion_gate_ref: &str,
    promotion_receipt_ref: &str,
) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some("span-coordinator-mt004".into()),
        linked_span_contexts: vec!["span-msg-proposal-001".into()],
        from_lane_id: "lane-local".into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(sample_routing(
            &format!("corr-{message_id}"),
            "coordinator",
            "coordinator-session-mt004",
        )),
        kind: ModelLaneMessageKind::PromotionRequest,
        payload_ref: format!("artifact://mt004/{message_id}"),
        payload_sha256: sample_sha256('c'),
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        summary: "promoted model-lane output after explicit PromotionGate decision".into(),
        authority: ModelLaneAuthority::Promoted,
        promotion_decision_id: Some(promotion_decision_id.into()),
        promotion_gate_ref: Some(promotion_gate_ref.into()),
        promotion_receipt_ref: Some(promotion_receipt_ref.into()),
        validator_verdict_ref: Some("validator://mt004/v1".into()),
        operator_decision_ref: Some("operator://mt004/approve".into()),
        promoted_artifact_ref: Some(format!("artifact://mt004/promoted/{message_id}")),
        promoted_artifact_sha256: Some(sample_sha256('d')),
        promoted_artifact_version: Some("1".into()),
        tool_gate_decision_refs: vec!["toolgate://mt004/read-context".into()],
        coordinator_session_id: "coordinator-session-mt004".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        locus_binding: Some(sample_locus(
            "session-lane-local",
            "model-session-lane-local",
        )),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000030/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-promotion#promoted".into()),
        created_at_utc: "2026-06-29T08:03:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "PromotionGate EventLedger row required",
            "direct_authority_mutation": "denied_without_decision"
        }),
    }
}

fn sample_decision(
    decision_id: &str,
    idempotency_key: &str,
    routing_policy: ModelLaneRoutingPolicy,
    expected_event_ledger_version: i64,
) -> NewModelLanePromotionDecision {
    NewModelLanePromotionDecision {
        decision_id: decision_id.into(),
        run_id: "run-mt004".into(),
        trace_id: "trace-mt004".into(),
        decision_span_id: format!("span-{decision_id}"),
        parent_span_id: Some("span-coordinator-mt004".into()),
        linked_span_contexts: vec![
            "span-msg-proposal-001".into(),
            "span-msg-critique-001".into(),
        ],
        coordinator_session_id: "coordinator-session-mt004".into(),
        routing_policy,
        routing_launch_plan: Vec::new(),
        input_refs: vec![
            "model-lane-message://msg-critique-001".into(),
            "model-lane-message://msg-proposal-001".into(),
        ],
        selected_input_refs: vec!["model-lane-message://msg-proposal-001".into()],
        rejected_input_refs: vec!["model-lane-message://msg-critique-001".into()],
        validator_authority_ref: Some("validator://mt004/v1".into()),
        operator_authority_ref: Some("operator://mt004/approve".into()),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: "msg-proposal-001".into(),
        expected_event_ledger_version,
        base_snapshot_ref: "not-applicable".into(),
        current_base_snapshot_ref: "not-applicable".into(),
        state_vector: "not-applicable".into(),
        current_state_vector: "not-applicable".into(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "lexicographic_selected_ref_then_lowest_event_seq".into(),
        promotion_gate_ref: "promotion-gate://mt004/approved".into(),
        promotion_receipt_ref: Some("promotion-receipt://mt004/approved".into()),
        promoted_artifact_ref: Some("artifact://mt004/promoted/msg-promoted-001".into()),
        promoted_artifact_sha256: Some(sample_sha256('d')),
        promoted_artifact_version: Some("1".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: "mlane-stream-run-mt004".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT004".into(),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000020/{decision_id}"),
        recovery_hint_ref: Some("usermanual://model-lane-promotion#decision".into()),
        created_at_utc: "2026-06-29T08:02:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger append required before authority mutation",
            "loom": "promotion is deterministic host-side state",
            "locus": "locus://wp1/mt004/coordinator-session-mt004",
            "palmistry": "external watcher link expected when feature is available"
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

fn sample_routing(
    correlation_id: &str,
    target_role: &str,
    target_session: &str,
) -> ModelLaneRoutingMetadata {
    ModelLaneRoutingMetadata {
        target_role: target_role.into(),
        target_session: target_session.into(),
        correlation_id: correlation_id.into(),
        requires_ack: true,
        ack_for: None,
    }
}

fn sample_sha256(ch: char) -> String {
    std::iter::repeat(ch).take(64).collect()
}
