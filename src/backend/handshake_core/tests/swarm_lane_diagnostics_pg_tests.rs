//! WP-1 MT-008: Dexterity lane diagnostics projection proof.

mod knowledge_pg_support;

use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneKind, ModelLaneLeaseScope, ModelLaneLeaseState, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneMtRuntimeStatus, ModelLaneProviderKind, ModelLaneRecoveryState,
    ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneDiagnosticTierStatus, NewModelLaneLease, NewModelLaneMessage,
    NewModelLaneMtRuntimeStatus, NewModelLaneRun, RuntimeBinding,
};
use serde_json::json;
use sha2::{Digest, Sha256};

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const OWNER: &str = "KERNEL_BUILDER-20260630-045713";

#[tokio::test]
async fn swarm_lane_diagnostics_backend_projection_matches_eventledger() {
    let (_pool, store) = diagnostics_store().await;
    store
        .record_run(sample_run("run-mt008-diag", "lane-mt008-local"))
        .await
        .expect("record diagnostics run");
    store
        .record_lane(sample_lane("lane-mt008-local", "run-mt008-diag"))
        .await
        .expect("record diagnostics lane");
    store
        .record_message(sample_message(
            "msg-mt008-001",
            "run-mt008-diag",
            "lane-mt008-local",
        ))
        .await
        .expect("record diagnostics message");
    store
        .record_lane_lease(sample_lease(
            "lease-mt008-expired",
            "run-mt008-diag",
            "lane-mt008-local",
            "2020-01-01T00:00:00Z",
        ))
        .await
        .expect("record expired active lease");
    for (tier, state, evidence) in [
        (
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/diagnostics",
        ),
        (
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::Wired,
            "hbr-int-009://dexterity/diagnostics",
        ),
        (
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://external-worktree/in-progress",
        ),
    ] {
        store
            .record_diagnostic_tier_status(sample_tier("run-mt008-diag", tier, state, evidence))
            .await
            .expect("record diagnostic tier");
    }
    store
        .record_mt_runtime_status(sample_mt_status("run-mt008-diag"))
        .await
        .expect("record MT runtime status");

    let projection = store
        .diagnostics_projection("run-mt008-diag")
        .await
        .expect("projection from PostgreSQL/EventLedger rows");

    assert_eq!(
        projection.surface_contract_id,
        "native_swarm_lane_diagnostics"
    );
    assert_eq!(projection.run.run_id, "run-mt008-diag");
    assert_eq!(
        projection.run.coordinator_session_id,
        "coordinator-run-mt008-diag"
    );
    assert_eq!(projection.run.routing_policy, "mixed_local_cloud_subagent");
    assert_eq!(
        projection.run.artifact_namespace,
        "artifact://model-lane/run-mt008-diag"
    );
    assert_eq!(projection.run.work_packet_id.as_deref(), Some(WP_ID));
    assert_eq!(projection.run.micro_task_id.as_deref(), Some("MT-008"));
    assert_eq!(
        projection.run.task_board_id.as_deref(),
        Some("task-board://wp-1")
    );
    assert_eq!(projection.run.owner_session, OWNER);
    assert_eq!(projection.run.context_bundle_id, "ctx-run-mt008-diag");
    assert_eq!(
        projection.run.memory_pack_ref,
        "memory-pack://fems/run-mt008-diag"
    );
    assert!(!projection.run.event_ledger_event_id.is_empty());
    assert!(projection.run.event_ledger_seq > 0);
    assert_eq!(
        projection.run.flight_recorder_correlation_id,
        projection.run.event_ledger_event_id
    );
    assert_eq!(projection.lanes.len(), 1);
    assert_eq!(projection.lanes[0].message_count, 1);
    assert_eq!(projection.lanes[0].role, "implementer");
    assert_eq!(projection.lanes[0].session_id, "session-lane-mt008-local");
    assert_eq!(
        projection.lanes[0].model_session_id,
        "model-session-lane-mt008-local"
    );
    assert_eq!(projection.lanes[0].launch_authority, "model_runtime");
    assert_eq!(
        projection.lanes[0].locus_ref.as_deref(),
        Some("locus://wp1/mt008/run-mt008-diag/session-lane-mt008-local")
    );
    assert_eq!(
        projection.lanes[0].last_runtime_status_ref.as_deref(),
        Some("runtime-status://mt008/running")
    );
    assert_eq!(
        projection.lanes[0].recovery_hint_ref.as_deref(),
        Some("usermanual://dexterity/diagnostics#lane")
    );
    assert_eq!(projection.lanes[0].orphan_state, "reclaimable");
    assert_eq!(
        projection.lanes[0].flight_recorder_correlation_id,
        projection.lanes[0].event_ledger_event_id
    );
    assert_eq!(projection.messages.len(), 1);
    assert_eq!(
        projection.messages[0].payload_ref,
        "artifact://model-lane/messages/msg-mt008-001"
    );
    assert_eq!(projection.messages[0].to_lane, "coordinator");
    assert_eq!(
        projection.messages[0].routing_target_role.as_deref(),
        Some("coordinator")
    );
    assert_eq!(
        projection.messages[0].routing_correlation_id.as_deref(),
        Some("corr-run-mt008-diag-msg-mt008-001")
    );
    assert!(projection.messages[0].routing_requires_ack);
    assert_eq!(
        projection.messages[0].promotion_decision_id.as_deref(),
        Some("promotion://msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].promotion_gate_ref.as_deref(),
        Some("promotion-gate://msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].promotion_receipt_ref.as_deref(),
        Some("promotion-receipt://msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].proposal_ref.as_deref(),
        Some("proposal://mt008/msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].crdt_update_ref.as_deref(),
        Some("crdt-update://mt008/msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].crdt_proposal_ref.as_deref(),
        Some("crdt-proposal://mt008/msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].recovery_hint_ref.as_deref(),
        Some("usermanual://dexterity/diagnostics#message")
    );
    assert_eq!(
        projection.messages[0].flight_recorder_correlation_id,
        projection.messages[0].event_ledger_event_id
    );
    assert_eq!(
        projection.messages[0].loom_ref.as_deref(),
        Some("loom://run-mt008-diag/msg-mt008-001")
    );
    assert_eq!(
        projection.messages[0].fems_ref.as_deref(),
        Some("fems://run-mt008-diag/msg-mt008-001")
    );
    assert_eq!(projection.diagnostic_tiers.len(), 3);
    assert!(projection
        .diagnostic_tiers
        .iter()
        .any(|tier| tier.tier == "flight_recorder" && tier.state == "wired"));
    assert_eq!(projection.mt_runtime_statuses.len(), 1);
    assert_eq!(projection.mt_runtime_statuses[0].micro_task_id, "MT-008");
    assert_eq!(
        projection.reclaimable_lease_ids,
        vec!["lease-mt008-expired"]
    );

    let latest = store
        .latest_diagnostics_projection()
        .await
        .expect("latest diagnostics projection resolves newest run");
    assert_eq!(latest.run.run_id, "run-mt008-diag");
}

#[tokio::test]
async fn swarm_lane_diagnostics_rejects_flight_recorder_only_hbr_posture() {
    let (_pool, store) = diagnostics_store().await;
    store
        .record_run(sample_run("run-mt008-fr-only", "lane-mt008-fr-only"))
        .await
        .expect("record diagnostics run");
    store
        .record_lane(sample_lane("lane-mt008-fr-only", "run-mt008-fr-only"))
        .await
        .expect("record diagnostics lane");
    store
        .record_message(sample_message(
            "msg-mt008-fr-only",
            "run-mt008-fr-only",
            "lane-mt008-fr-only",
        ))
        .await
        .expect("record diagnostics message");
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt008-fr-only",
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/diagnostics",
        ))
        .await
        .expect("record only FlightRecorder tier");

    let err = store
        .diagnostics_projection("run-mt008-fr-only")
        .await
        .expect_err("HBR-INT-009 posture must not accept FlightRecorder-only diagnostics");
    let err = err.to_string();
    assert!(
        err.contains("internal_diagnostics") || err.contains("palmistry"),
        "missing-tier error should name absent HBR tier, got {err}"
    );
}

async fn diagnostics_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-008 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated diagnostics schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

fn sample_run(run_id: &str, lane_id: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        run_span_id: format!("span-{run_id}"),
        coordinator_session_id: format!("coordinator-{run_id}"),
        routing_policy: "mixed_local_cloud_subagent".into(),
        context_bundle_id: format!("ctx-{run_id}"),
        lane_ids: vec![lane_id.into()],
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        artifact_namespace: format!("artifact://model-lane/{run_id}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some("MT-008".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-{run_id}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics".into()),
        locus_binding: Some(sample_locus(run_id, &format!("coordinator-{run_id}"))),
        memory_pack_ref: format!("memory-pack://fems/{run_id}"),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt008".into(),
        selected_model_id: Some("model://mt008/local".into()),
        candidate_model_ids: vec!["model://mt008/local".into()],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_lane(lane_id: &str, run_id: &str) -> NewModelLane {
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        kind: ModelLaneKind::LocalModel,
        role: "implementer".into(),
        backend: RuntimeBinding::Local.as_str().into(),
        model_id: Some("model://mt008/local".into()),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt008/read".into()],
        effective_capability_snapshot_ref: Some("capability-snapshot://mt008".into()),
        capability_negotiation_ref: Some("capability-negotiation://mt008".into()),
        provider_feature_profile_ref: Some("provider-feature-profile://mt008".into()),
        requested_execution_policy_ref: Some("execution-policy://requested/mt008".into()),
        effective_execution_policy_ref: Some("execution-policy://effective/mt008".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["toolgate://mt008/allow".into()],
        status: ModelLaneStatus::Running,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-30T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-01-01T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-01-01T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt008/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt008".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt008".into()),
        process_ownership_ref: Some(format!("process-ledger://mt008/{lane_id}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt008".into()),
        last_runtime_status_ref: Some("runtime-status://mt008/running".into()),
        last_recovery_event_ref: Some("recovery://mt008/running".into()),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics#lane".into()),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some("MT-008".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(run_id, &format!("session-{lane_id}"))),
    }
}

fn sample_message(message_id: &str, run_id: &str, lane_id: &str) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec![format!("trace-link://{run_id}/{lane_id}")],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-{run_id}"),
            correlation_id: format!("corr-{run_id}-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://model-lane/messages/{message_id}"),
        payload_sha256: sha256_hex(message_id.as_bytes()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        summary: "MT-008 diagnostics payload".into(),
        authority: ModelLaneAuthority::PromotionCandidate,
        promotion_decision_id: Some(format!("promotion://{message_id}")),
        promotion_gate_ref: Some(format!("promotion-gate://{message_id}")),
        promotion_receipt_ref: Some(format!("promotion-receipt://{message_id}")),
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: Some(format!("artifact://promoted/{message_id}")),
        promoted_artifact_sha256: Some(sample_sha256()),
        promoted_artifact_version: Some("1".into()),
        tool_gate_decision_refs: vec!["toolgate://mt008/allow".into()],
        coordinator_session_id: format!("coordinator-{run_id}"),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some("MT-008".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: OWNER.into(),
        locus_binding: Some(sample_locus(run_id, &format!("session-{lane_id}"))),
        idempotency_key: format!("idem-{message_id}"),
        replay_order_key: "00000002/message".into(),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: Some(format!("proposal://mt008/{message_id}")),
        crdt_update_ref: Some(format!("crdt-update://mt008/{message_id}")),
        crdt_base_snapshot_ref: Some("crdt-snapshot://mt008/base".into()),
        crdt_state_vector: Some("sv:mt008:1".into()),
        crdt_proposal_ref: Some(format!("crdt-proposal://mt008/{message_id}")),
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics#message".into()),
        created_at_utc: "2026-06-30T00:00:00Z".into(),
        diagnostic_payload: json!({
            "artifact_ref": format!("artifact://model-lane/messages/{message_id}"),
            "loom_ref": format!("loom://{run_id}/{message_id}"),
            "fems_ref": format!("fems://{run_id}/{message_id}"),
            "payload_error": null
        }),
    }
}

fn sample_lease(
    lease_id: &str,
    run_id: &str,
    lane_id: &str,
    lease_expires_at_utc: &str,
) -> NewModelLaneLease {
    NewModelLaneLease {
        lease_id: lease_id.into(),
        run_id: run_id.into(),
        lane_id: Some(lane_id.into()),
        scope: ModelLaneLeaseScope::Lane,
        scope_ref: format!("model-lane://{run_id}/{lane_id}"),
        holder_actor_id: "actor://kernel-builder/mt008".into(),
        holder_session_id: OWNER.into(),
        lease_expires_at_utc: lease_expires_at_utc.into(),
        takeover_policy_ref: "lease-policy://mt008/recover-or-reclaim".into(),
        state: ModelLaneLeaseState::Active,
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-{lease_id}"),
        recovery_hint_ref: Some("usermanual://dexterity/diagnostics#lease".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics"}),
    }
}

fn sample_tier(
    run_id: &str,
    tier: ModelLaneDiagnosticTier,
    state: ModelLaneDiagnosticTierState,
    evidence_ref: &str,
) -> NewModelLaneDiagnosticTierStatus {
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diag-{run_id}-mt008-{}", tier.as_str()),
        behavior_id: "HBR-INT-009".into(),
        run_id: run_id.into(),
        tier,
        state,
        reason: format!("MT-008 diagnostics posture for {run_id}"),
        evidence_ref: evidence_ref.into(),
        follow_up_ref: Some("palmistry://external-worktree/in-progress".into()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: OWNER.into(),
        idempotency_key: format!("idem-diag-{run_id}-mt008-{}", tier.as_str()),
        diagnostic_payload: json!({"behavior_id": "HBR-INT-009", "run_id": run_id}),
    }
}

fn sample_mt_status(run_id: &str) -> NewModelLaneMtRuntimeStatus {
    NewModelLaneMtRuntimeStatus {
        mt_status_id: "mt-status-mt008-rfv".into(),
        run_id: run_id.into(),
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: "task-board://wp-1".into(),
        status: ModelLaneMtRuntimeStatus::ReadyForValidation,
        claimed_by_ref: Some(format!("session://{OWNER}")),
        blocker_ref: None,
        missing_resource_ref: None,
        proof_status_ref: Some("proof://mt008/swarm_lane_diagnostics_pg_tests".into()),
        hbr_status_ref: Some("hbr-int-009://dexterity/diagnostics".into()),
        last_recovery_event_ref: None,
        last_runtime_status_ref: Some("runtime-status://mt008/ready-for-validation".into()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        owner_session: OWNER.into(),
        idempotency_key: "idem-mt-status-mt008-rfv".into(),
        diagnostic_payload: json!({"state_recovery": true}),
    }
}

fn sample_locus(run_id: &str, session_id: &str) -> ModelLaneLocusBinding {
    let model_session_id = if let Some(lane_suffix) = session_id.strip_prefix("session-") {
        format!("model-session-{lane_suffix}")
    } else {
        format!("model-session-{session_id}")
    };
    ModelLaneLocusBinding {
        work_packet_id: WP_ID.into(),
        micro_task_id: "MT-008".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-{run_id}"),
        session_id: session_id.into(),
        model_session_id,
        owner_session: OWNER.into(),
        locus_binding_ref: format!("locus://wp1/mt008/{run_id}/{session_id}"),
    }
}

fn sample_sha256() -> String {
    sha256_hex(b"mt008-diagnostics")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
