//! MT-002/004 embedded-SurrealDB ModelLane schema and replay proof.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneRecoveryState, ModelLaneRoutingMetadata,
    ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneMessage,
    NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
};
use serde_json::json;
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
            .expect("allocate exact ModelLane embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical embedded schema");
        let scope = exact_scope(label);
        let store = ModelLaneStore::new_scoped(storage.clone(), scope.clone());
        Self {
            isolated,
            storage,
            scope,
            store,
        }
    }

    fn store_for(&self, scope: ResourceScope) -> ModelLaneStore {
        ModelLaneStore::new_scoped(self.storage.clone(), scope)
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("clean exact ModelLane embedded scope");
    }
}

#[tokio::test]
async fn model_lane_schema_persists_and_replays_eventledger_rows() {
    let mut harness = Harness::create("schema-replay").await;
    let run = sample_run("schema-replay");
    let lane = sample_lane("schema-replay");
    let message = sample_message("schema-replay");

    let stored_run = harness
        .store
        .record_run(run.clone())
        .await
        .expect("record run");
    let stored_lane = harness
        .store
        .record_lane(lane.clone())
        .await
        .expect("record lane");
    let stored_message = harness
        .store
        .record_message(message.clone())
        .await
        .expect("record message");
    assert!(stored_run.event_ledger_seq > 0);
    assert!(stored_lane.event_ledger_seq > stored_run.event_ledger_seq);
    assert!(stored_message.event_ledger_seq > stored_lane.event_ledger_seq);

    let replay = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("replay exact scoped run");
    assert_eq!(replay.run.run_id, run.run_id);
    assert_eq!(replay.lanes, vec![stored_lane.clone()]);
    assert_eq!(replay.messages, vec![stored_message.clone()]);
    assert_eq!(
        harness
            .store
            .record_message(message.clone())
            .await
            .expect("identical retry"),
        stored_message
    );

    let registry = harness
        .store
        .schema_registry_rows()
        .await
        .expect("read exact scoped schema registry");
    assert!(registry
        .iter()
        .any(|row| row.schema_id == "hsk.model_lane_run@1"));
    assert!(registry
        .iter()
        .any(|row| row.schema_id == "hsk.model_lane_message@1"));

    drop(harness.store);
    drop(harness.storage);
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close storage before restart");
    harness.isolated.reopen().await.expect("reopen same scope");
    let reopened_storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate same namespace/database");
    let reopened = ModelLaneStore::new_scoped(reopened_storage.clone(), harness.scope.clone());
    let restarted = reopened
        .replay_run(&run.run_id)
        .await
        .expect("replay survives same-store restart");
    assert_eq!(restarted.run, stored_run);
    assert_eq!(restarted.lanes, vec![stored_lane]);
    assert_eq!(restarted.messages, vec![stored_message]);
    harness.store = reopened;
    harness.storage = reopened_storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_schema_rejects_missing_locus_binding_and_idempotency_conflict() {
    let harness = Harness::create("schema-denials").await;
    let mut missing_locus = sample_run("missing-locus");
    missing_locus.locus_binding = None;
    let missing = harness
        .store
        .record_run(missing_locus)
        .await
        .expect_err("missing Locus binding must fail before persistence");
    assert!(missing.to_string().contains("locus"));

    let run = sample_run("schema-denials");
    harness
        .store
        .record_run(run.clone())
        .await
        .expect("record canonical run");
    harness
        .store
        .record_lane(sample_lane("schema-denials"))
        .await
        .expect("record source lane");
    let message = sample_message("schema-denials");
    harness
        .store
        .record_message(message.clone())
        .await
        .expect("record canonical message");
    let mut conflict = message;
    conflict.payload_sha256 = "f".repeat(64);
    let conflict = harness
        .store
        .record_message(conflict)
        .await
        .expect_err("divergent immutable retry must fail");
    assert!(conflict.to_string().contains("idempotency"));

    let incomplete = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint());
    let denied = ModelLaneStore::new_scoped(harness.storage.clone(), incomplete)
        .record_run(sample_run("incomplete-scope"))
        .await
        .expect_err("incomplete five-field scope must fail closed");
    assert!(denied.to_string().contains("exact owner"));
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_schema_isolates_all_five_scope_fields_with_equal_logical_ids() {
    let harness = Harness::create("scope-owner").await;
    let run = sample_run("scope-collision");
    let owner_record = harness
        .store
        .record_run(run.clone())
        .await
        .expect("owner stores run");

    for foreign in one_field_mismatches(&harness.scope) {
        let foreign_store = harness.store_for(foreign);
        assert!(foreign_store.replay_run(&run.run_id).await.is_err());
        let foreign_record = foreign_store
            .record_run(run.clone())
            .await
            .expect("same logical id is independent in another exact scope");
        assert_ne!(
            foreign_record.event_ledger_event_id,
            owner_record.event_ledger_event_id
        );
    }
    assert_eq!(
        harness
            .store
            .replay_run(&run.run_id)
            .await
            .expect("owner remains available")
            .run,
        owner_record
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_launch_path_persists_one_atomic_run_lane_pair() {
    let harness = Harness::create("prepared-launch").await;
    let run = sample_run("prepared-launch");
    let lane = sample_lane("prepared-launch");
    let (stored_run, stored_lane) = harness
        .store
        .record_prepared_launch((run.clone(), lane.clone()))
        .await
        .expect("production prepared-launch path persists the pair");
    assert_eq!(stored_run.run_id, run.run_id);
    assert_eq!(stored_lane.lane_id, lane.lane_id);
    assert_eq!(stored_lane.run_id, stored_run.run_id);
    assert!(stored_lane.event_ledger_seq > stored_run.event_ledger_seq);

    let replay = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("prepared launch replays from canonical authority");
    assert_eq!(replay.run, stored_run);
    assert_eq!(replay.lanes, vec![stored_lane]);
    harness.cleanup().await;
}

#[tokio::test]
async fn competing_terminal_updates_use_versioned_cas_and_failure_is_non_mutating() {
    let harness = Harness::create("terminal-cas").await;
    let run = sample_run("terminal-cas");
    let lane = sample_lane("terminal-cas");
    harness
        .store
        .record_prepared_launch((run.clone(), lane.clone()))
        .await
        .expect("seed ready launch");

    let control = harness.store.test_terminal_commit_control();
    control.fail_next();
    let injected = harness
        .store
        .record_lane_terminal_status(&lane.lane_id, ModelLaneStatus::Failed, "injected")
        .await
        .expect_err("pre-commit failure must propagate");
    assert!(injected.to_string().contains("before durable mutation"));
    assert_eq!(
        harness
            .store
            .replay_run(&run.run_id)
            .await
            .expect("failure leaves launch readable")
            .lanes[0]
            .status,
        ModelLaneStatus::Ready
    );

    control.pause_next();
    let paused_store = harness.store.clone();
    let paused_lane_id = lane.lane_id.clone();
    let paused = tokio::spawn(async move {
        paused_store
            .record_lane_terminal_status(
                &paused_lane_id,
                ModelLaneStatus::Completed,
                "completed concurrently",
            )
            .await
    });
    control.wait_until_paused().await;

    let winner = harness
        .store
        .record_lane_terminal_status(
            &lane.lane_id,
            ModelLaneStatus::Cancelled,
            "cancelled concurrently",
        )
        .await
        .expect("one terminal writer wins");
    control.release_paused();
    let loser = paused
        .await
        .expect("paused writer task joins")
        .expect_err("stale terminal writer loses its compare-and-set");
    assert!(loser
        .to_string()
        .contains("changed while terminal status was committing"));
    assert_eq!(winner.status, ModelLaneStatus::Cancelled);

    let replay = harness
        .store
        .replay_run(&run.run_id)
        .await
        .expect("terminal winner is canonical");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(
        replay.lanes[0].event_ledger_event_id,
        winner.event_ledger_event_id
    );
    harness.cleanup().await;
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt002-{label}")).expect("nonblank workspace"),
        )
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let workspace = scope.workspace.clone().expect("exact workspace");
    vec![
        ResourceScope::new(OwnerAccountId::mint(), scope.actor_principal_id)
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, ActorPrincipalId::mint())
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(workspace),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(scope.authenticated_session.expect("exact session"))
            .with_access_space(scope.access_space.expect("exact access space"))
            .with_workspace(
                WorkspaceScopeRef::new("workspace-mt002-foreign").expect("nonblank workspace"),
            ),
    ]
}

fn sample_run(label: &str) -> NewModelLaneRun {
    let run_id = format!("run-mt002-{label}");
    let lane_id = format!("lane-mt002-{label}");
    NewModelLaneRun {
        run_id,
        trace_id: format!("trace-mt002-{label}"),
        run_span_id: format!("span-run-mt002-{label}"),
        coordinator_session_id: format!("coordinator-mt002-{label}"),
        routing_policy: "local_first".into(),
        context_bundle_id: format!("context-mt002-{label}"),
        lane_ids: vec![lane_id],
        event_ledger_stream_id: format!("model-lane://mt002/{label}"),
        artifact_namespace: format!("artifact://mt002/{label}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-002".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt002-{label}"),
        idempotency_key: format!("mt002-run-{label}"),
        replay_order_key: format!("0001-{label}"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        locus_binding: Some(sample_locus(label)),
        memory_pack_ref: format!("memory-pack://mt002/{label}"),
        memory_pack_hash: "a".repeat(64),
        determinism_mode: "strict".into(),
        budget_summary_ref: format!("budget://mt002/{label}"),
        selected_model_id: Some("model://local/mt002".into()),
        candidate_model_ids: vec!["model://local/mt002".into()],
        procedural_review_status: "approved".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

fn sample_lane(label: &str) -> NewModelLane {
    let lane_id = format!("lane-mt002-{label}");
    let session_id = format!("session-mt002-{label}");
    let model_session_id = format!("model-session-mt002-{label}");
    NewModelLane {
        lane_id,
        run_id: format!("run-mt002-{label}"),
        trace_id: format!("trace-mt002-{label}"),
        lane_span_id: format!("span-lane-mt002-{label}"),
        event_ledger_stream_id: format!("model-lane://mt002/{label}"),
        kind: ModelLaneKind::LocalModel,
        role: "worker".into(),
        backend: "embedded-model-runtime".into(),
        model_id: Some("model://local/mt002".into()),
        session_id: session_id.clone(),
        model_session_id: model_session_id.clone(),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt002/read".into()],
        effective_capability_snapshot_ref: Some("capability://mt002/snapshot".into()),
        capability_negotiation_ref: Some("capability://mt002/negotiation".into()),
        provider_feature_profile_ref: Some("provider://mt002/local".into()),
        requested_execution_policy_ref: Some("execution://mt002/requested".into()),
        effective_execution_policy_ref: Some("execution://mt002/effective".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["tool-gate://mt002/read".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-09-02T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-09-02T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-09-02T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel://mt002/{label}")),
        reclaim_policy_ref: Some("reclaim://mt002".into()),
        terminal_status_mapping_ref: Some("terminal://mt002".into()),
        process_ownership_ref: Some(format!("process://mt002/{label}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop://mt002".into()),
        last_runtime_status_ref: Some("runtime://mt002/ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-002".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt002-{label}"),
        locus_binding: Some(sample_locus_for(label, &session_id, &model_session_id)),
    }
}

fn sample_message(label: &str) -> NewModelLaneMessage {
    NewModelLaneMessage {
        message_id: format!("message-mt002-{label}"),
        run_id: format!("run-mt002-{label}"),
        trace_id: format!("trace-mt002-{label}"),
        message_span_id: format!("span-message-mt002-{label}"),
        parent_span_id: Some(format!("span-lane-mt002-{label}")),
        linked_span_contexts: vec![format!("trace-mt002-{label}")],
        from_lane_id: format!("lane-mt002-{label}"),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-mt002-{label}"),
            correlation_id: format!("correlation-mt002-{label}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://mt002/{label}/message"),
        payload_sha256: "b".repeat(64),
        event_ledger_stream_id: format!("model-lane://mt002/{label}"),
        summary: "typed local advisory proposal".into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["tool-gate://mt002/read".into()],
        coordinator_session_id: format!("coordinator-mt002-{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-002".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt002-{label}"),
        locus_binding: Some(sample_locus(label)),
        idempotency_key: format!("mt002-message-{label}"),
        replay_order_key: format!("0002-{label}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: Some(format!("proposal://mt002/{label}")),
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/message-replay".into()),
        created_at_utc: "2026-09-02T00:00:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "wired",
            "internal_diagnostics": "deferred",
            "palmistry": "deferred"
        }),
    }
}

fn sample_locus(label: &str) -> ModelLaneLocusBinding {
    sample_locus_for(
        label,
        &format!("session-mt002-{label}"),
        &format!("model-session-mt002-{label}"),
    )
}

fn sample_locus_for(
    label: &str,
    session_id: &str,
    model_session_id: &str,
) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-002".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-mt002-{label}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: format!("owner-mt002-{label}"),
        locus_binding_ref: format!("locus://wp1/mt002/{label}"),
    }
}
