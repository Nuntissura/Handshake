//! WP-1 MT-007 V5: account-facing recovery scope proof over real PostgreSQL
//! and the real Axum runtime route.

mod knowledge_pg_support;
mod user_manual_support;

use axum::Extension;
use handshake_core::api;
use handshake_core::api::account_scope::ProductLocalResourceScope;
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneKind, ModelLaneLocusBinding, ModelLaneProviderKind,
    ModelLaneRecoveryEventKind, ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneStatus,
    ModelLaneStore, NewModelLane, NewModelLaneRecoveryCheckpoint, NewModelLaneRecoveryEvent,
    NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use user_manual_support::{app_state_for, start_server};

const RUN_ID: &str = "run-mt007-scope-api";
const LANE_ID: &str = "lane-mt007-scope-api";
const RECOVERY_EVENT_ID: &str = "recovery-event-mt007-scope-api";
const CHECKPOINT_ID: &str = "checkpoint-mt007-scope-api";

#[tokio::test]
async fn recovery_route_is_exact_scoped_and_revoked_authority_is_absent_shaped() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for MT-007 recovery API scope proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated recovery API scope schema");
    let exact = exact_scope("owner");
    let owner_store = ModelLaneStore::new_scoped(pool.clone(), resource_scope(&exact));
    owner_store
        .record_run(sample_run())
        .await
        .expect("record scoped recovery API run");
    owner_store
        .record_lane(sample_lane())
        .await
        .expect("record scoped recovery API lane");
    owner_store
        .record_recovery_event(sample_recovery_event())
        .await
        .expect("record scoped recovery API event");
    let high_watermark = event_stream_high_watermark(&pool).await;
    owner_store
        .record_recovery_checkpoint(sample_checkpoint(high_watermark))
        .await
        .expect("record scoped recovery API checkpoint");

    let state = app_state_for(&kpg.schema_url).await;
    let path = format!("/swarm/model-lanes/navigation/recovery/{RUN_ID}");
    let owner_authority =
        ProductLocalResourceScope::from_exact(exact.clone()).expect("complete exact owner scope");
    let (owner_base, owner_server) = start_server(
        api::model_lane_navigation::routes(state.clone()).layer(Extension(owner_authority)),
    )
    .await;
    let owner_response = reqwest::get(format!("{owner_base}{path}"))
        .await
        .expect("owner recovery route request");
    assert_eq!(owner_response.status(), reqwest::StatusCode::OK);
    let owner_body: Value = owner_response.json().await.expect("owner recovery body");
    assert_eq!(owner_body["run"]["run_id"], RUN_ID);
    assert_eq!(
        owner_body["recovery_events"][0]["recovery_event_id"],
        RECOVERY_EVENT_ID
    );
    assert_eq!(
        owner_body["recovery_checkpoints"][0]["checkpoint_id"],
        CHECKPOINT_ID
    );
    owner_server.abort();

    for wrong in wrong_exact_scopes(&exact) {
        let authority =
            ProductLocalResourceScope::from_exact(wrong).expect("complete wrong exact scope");
        let (base, server) = start_server(
            api::model_lane_navigation::routes(state.clone()).layer(Extension(authority)),
        )
        .await;
        let response = reqwest::get(format!("{base}{path}"))
            .await
            .expect("wrong-scope recovery route request");
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
        let body = response.text().await.expect("wrong-scope denial body");
        assert!(body.contains("not_found"));
        assert!(!body.contains(RECOVERY_EVENT_ID));
        assert!(!body.contains(CHECKPOINT_ID));
        assert!(!body.contains(LANE_ID));
        server.abort();
    }

    // Bounded pre-K006 revocation proof: once the product removes the current
    // server authority, the same route fails before storage and returns only a
    // fixed reason code. MT-007 does not invent a session revocation registry.
    let (revoked_base, revoked_server) =
        start_server(api::model_lane_navigation::routes(state)).await;
    let revoked = reqwest::get(format!("{revoked_base}{path}"))
        .await
        .expect("revoked/no-authority recovery route request");
    assert_eq!(revoked.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    let revoked_body = revoked.text().await.expect("revoked denial body");
    assert!(revoked_body.contains("RESOURCE_SCOPE_AUTHORITY_UNAVAILABLE"));
    for secret in [RECOVERY_EVENT_ID, CHECKPOINT_ID, LANE_ID] {
        assert!(
            !revoked_body.contains(secret),
            "revoked denial must not disclose {secret}: {revoked_body}"
        );
    }
    revoked_server.abort();
}

fn exact_scope(label: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("workspace-mt007-api-{label}"))
            .expect("valid workspace"),
    }
}

fn resource_scope(exact: &ExactResourceScopeAttribution) -> ResourceScope {
    ResourceScope::new(exact.owner_account_id, exact.actor_principal_id)
        .with_session(exact.authenticated_session_id)
        .with_access_space(exact.access_space_id)
        .with_workspace(exact.workspace_id.clone())
}

fn wrong_exact_scopes(exact: &ExactResourceScopeAttribution) -> [ExactResourceScopeAttribution; 5] {
    [
        ExactResourceScopeAttribution {
            owner_account_id: OwnerAccountId::mint(),
            ..exact.clone()
        },
        ExactResourceScopeAttribution {
            actor_principal_id: ActorPrincipalId::mint(),
            ..exact.clone()
        },
        ExactResourceScopeAttribution {
            authenticated_session_id: AuthenticatedSessionRef::mint(),
            ..exact.clone()
        },
        ExactResourceScopeAttribution {
            access_space_id: AccessSpaceRef::mint(),
            ..exact.clone()
        },
        ExactResourceScopeAttribution {
            workspace_id: WorkspaceScopeRef::new("workspace-mt007-api-wrong")
                .expect("valid wrong workspace"),
            ..exact.clone()
        },
    ]
}

fn sample_run() -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: RUN_ID.into(),
        trace_id: format!("trace-{RUN_ID}"),
        run_span_id: format!("span-{RUN_ID}"),
        coordinator_session_id: format!("coordinator-{RUN_ID}"),
        routing_policy: "local_recovery_scope_api".into(),
        context_bundle_id: format!("ctx-{RUN_ID}"),
        lane_ids: vec![LANE_ID.into()],
        event_ledger_stream_id: event_stream_id(),
        artifact_namespace: format!("artifact://model-lane/{RUN_ID}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-007".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT007-V5".into(),
        idempotency_key: format!("idem-{RUN_ID}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-recovery".into()),
        locus_binding: Some(sample_locus(
            &format!("coordinator-{RUN_ID}"),
            &format!("model-session-coordinator-{RUN_ID}"),
        )),
        memory_pack_ref: format!("memory-pack://fems/{RUN_ID}"),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt007/scope-api".into(),
        selected_model_id: Some("model://mt007/local".into()),
        candidate_model_ids: vec!["model://mt007/local".into()],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

fn sample_lane() -> NewModelLane {
    NewModelLane {
        lane_id: LANE_ID.into(),
        run_id: RUN_ID.into(),
        trace_id: format!("trace-{RUN_ID}"),
        lane_span_id: format!("span-{LANE_ID}"),
        event_ledger_stream_id: event_stream_id(),
        kind: ModelLaneKind::LocalModel,
        role: "recovery_scope_probe".into(),
        backend: "local".into(),
        model_id: Some("model://mt007/local".into()),
        session_id: format!("session-{LANE_ID}"),
        model_session_id: format!("model-session-{LANE_ID}"),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt007/read".into()],
        effective_capability_snapshot_ref: Some("capability-snapshot://mt007".into()),
        capability_negotiation_ref: Some("capability-negotiation://mt007".into()),
        provider_feature_profile_ref: Some("provider-feature-profile://mt007".into()),
        requested_execution_policy_ref: Some("execution-policy://requested/mt007".into()),
        effective_execution_policy_ref: Some("execution-policy://effective/mt007".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["toolgate://mt007/allow".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-08-13T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-01-01T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-01-01T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some("cancel-token://mt007/scope-api".into()),
        reclaim_policy_ref: Some("reclaim-policy://mt007/scope-api".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt007/scope-api".into()),
        process_ownership_ref: Some("process-ledger://mt007/scope-api".into()),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt007/scope-api".into()),
        last_runtime_status_ref: Some("runtime-status://mt007/scope-api".into()),
        last_recovery_event_ref: Some(format!("recovery-event://{RECOVERY_EVENT_ID}")),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-recovery#lane".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-007".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT007-V5".into(),
        locus_binding: Some(sample_locus(
            &format!("session-{LANE_ID}"),
            &format!("model-session-{LANE_ID}"),
        )),
    }
}

fn sample_recovery_event() -> NewModelLaneRecoveryEvent {
    NewModelLaneRecoveryEvent {
        recovery_event_id: RECOVERY_EVENT_ID.into(),
        run_id: RUN_ID.into(),
        lane_id: Some(LANE_ID.into()),
        trace_id: format!("trace-{RUN_ID}"),
        span_id: format!("span-{RECOVERY_EVENT_ID}"),
        parent_span_id: Some(format!("span-{LANE_ID}")),
        linked_span_contexts: vec![format!("trace-link://{RUN_ID}/{RECOVERY_EVENT_ID}")],
        session_id: Some(format!("session-{LANE_ID}")),
        model_session_id: Some(format!("model-session-{LANE_ID}")),
        event_kind: ModelLaneRecoveryEventKind::CheckpointRestored,
        recovery_status: ModelLaneRecoveryStatus::Observed,
        replay_order_seq: 1,
        source_event_ledger_seq: None,
        payload_refs: Vec::new(),
        artifact_refs: Vec::new(),
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_stale_base_ref: None,
        lease_id: None,
        failure_kind: None,
        error_code: None,
        replay_hint: "Replay only through the current exact server scope".into(),
        event_ledger_stream_id: event_stream_id(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-MT007-V5".into(),
        idempotency_key: format!("idem-{RECOVERY_EVENT_ID}"),
        recovery_hint_ref: Some("usermanual://model-lane-recovery#event".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics"}),
    }
}

fn sample_checkpoint(last_event_ledger_seq: i64) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: CHECKPOINT_ID.into(),
        run_id: RUN_ID.into(),
        lane_id: Some(LANE_ID.into()),
        session_id: format!("session-{LANE_ID}"),
        model_session_id: format!("model-session-{LANE_ID}"),
        lane_status: ModelLaneStatus::Ready,
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq,
        last_message_id: None,
        open_payload_refs: Vec::new(),
        lease_id: None,
        idempotency_scope: format!("model-lane-recovery:{RUN_ID}:{CHECKPOINT_ID}"),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some(format!("recovery-event://{RECOVERY_EVENT_ID}")),
        event_ledger_stream_id: event_stream_id(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-MT007-V5".into(),
        idempotency_key: format!("idem-{CHECKPOINT_ID}"),
        created_at_utc: "2026-08-13T00:00:01Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane-recovery#checkpoint".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics"}),
    }
}

fn sample_locus(session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-{RUN_ID}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: "KERNEL_BUILDER-MT007-V5".into(),
        locus_binding_ref: format!("locus://wp1/mt007/{RUN_ID}/{session_id}"),
    }
}

async fn event_stream_high_watermark(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(MAX(event_sequence), 0) FROM kernel_event_ledger WHERE session_run_id = $1",
    )
    .bind(event_stream_id())
    .fetch_one(pool)
    .await
    .expect("query recovery API EventLedger high-watermark")
}

fn event_stream_id() -> String {
    format!("mlane-stream-{RUN_ID}")
}

fn sample_sha256() -> String {
    format!("{:x}", Sha256::digest(b"mt007-recovery-scope-api"))
}
