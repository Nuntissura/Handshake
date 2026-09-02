//! MT-010 eight-surface navigation proof over exact-scope embedded SurrealDB.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState, ModelLaneNavigationLookup,
    ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneStatus, ModelLaneStore,
    NewModelLaneDiagnosticTierStatus, NewModelLaneRecoveryCheckpoint,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::EmbeddedSurrealTestScope;

#[tokio::test]
async fn all_eight_navigation_surfaces_replay_after_same_namespace_restart() {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate navigation scope");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate SurrealStorage");
    bootstrap_schema(&storage).await.expect("bootstrap schema");
    let scope = exact_scope("navigation");
    let store = ModelLaneStore::new_scoped(storage.clone(), scope.clone());
    let posture = build_surreal_admissible_crdt_posture(
        &store,
        scope.workspace.as_ref().expect("workspace").as_str(),
        "navigation",
    )
    .await
    .expect("seed production posture");
    let message = store
        .record_message(posture.message.clone())
        .await
        .expect("record message");
    store
        .record_recovery_checkpoint(checkpoint(&posture, &message))
        .await
        .expect("record recovery checkpoint");
    store
        .record_diagnostic_tier_status(diagnostic(&posture))
        .await
        .expect("record diagnostic tier");
    let replay = store.replay_run(&posture.run_id).await.expect("replay run");
    let context_bundle_id = replay.run.context_bundle_id.clone();

    let projections = vec![
        store
            .navigation_by_run(&posture.run_id)
            .await
            .expect("run route"),
        store
            .navigation_by_lane(&posture.lane_id)
            .await
            .expect("lane route"),
        store
            .navigation_by_message(&message.message_id)
            .await
            .expect("message route"),
        store
            .navigation_by_artifact_or_context(None, Some(&context_bundle_id), None)
            .await
            .expect("artifact/context route"),
        store
            .navigation_by_trace(&posture.trace_id, None)
            .await
            .expect("trace route"),
        store
            .navigation_by_diagnostics(
                &posture.run_id,
                Some("HBR-INT-009"),
                Some("flight_recorder"),
                None,
            )
            .await
            .expect("diagnostics route"),
        store
            .navigation_by_recovery(&posture.run_id)
            .await
            .expect("recovery route"),
        store
            .navigation_by_lookup(ModelLaneNavigationLookup {
                model_session_id: Some(posture.model_session_id.clone()),
                ..Default::default()
            })
            .await
            .expect("lookup route"),
    ];
    assert_eq!(projections.len(), 8);
    for projection in &projections {
        assert_eq!(projection.run.as_ref().expect("run").run_id, posture.run_id);
        assert!(!projection.event_ledger_refs.is_empty());
        assert!(projection
            .event_ledger_refs
            .iter()
            .all(|reference| reference.starts_with("eventledger://")));
    }
    assert_eq!(projections[2].messages.len(), 1);
    assert_eq!(projections[5].diagnostic_tiers.len(), 1);
    assert_eq!(projections[6].recovery_checkpoints.len(), 1);

    drop(store);
    drop(storage);
    isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close storage");
    isolated.reopen().await.expect("reopen same database");
    let storage = isolated
        .activate_storage()
        .await
        .expect("reactivate storage");
    let reopened = ModelLaneStore::new_scoped(storage.clone(), scope);
    assert_eq!(
        reopened
            .navigation_by_message(&message.message_id)
            .await
            .expect("message route survives restart")
            .messages,
        projections[2].messages
    );
    drop(reopened);
    drop(storage);
    isolated.cleanup().await.expect("cleanup navigation scope");
}

#[tokio::test]
async fn navigation_denials_are_non_leaking_and_non_mutating_for_every_scope_field() {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate navigation denial scope");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate SurrealStorage");
    bootstrap_schema(&storage).await.expect("bootstrap schema");
    let scope = exact_scope("navigation-denial");
    let owner = ModelLaneStore::new_scoped(storage.clone(), scope.clone());
    let posture = build_surreal_admissible_crdt_posture(
        &owner,
        scope.workspace.as_ref().expect("workspace").as_str(),
        "navigation-denial",
    )
    .await
    .expect("seed production posture");
    let message = owner
        .record_message(posture.message.clone())
        .await
        .expect("record message");
    let before = owner
        .test_scoped_authority_receipts(&posture.run_id, 64)
        .await
        .expect("baseline receipts");

    for foreign_scope in one_field_mismatches(&scope) {
        let foreign = ModelLaneStore::new_scoped(storage.clone(), foreign_scope);
        assert!(foreign.navigation_by_run(&posture.run_id).await.is_err());
        assert!(foreign.navigation_by_lane(&posture.lane_id).await.is_err());
        assert!(foreign
            .navigation_by_message(&message.message_id)
            .await
            .is_err());
        assert!(foreign
            .navigation_by_lookup(ModelLaneNavigationLookup {
                trace_id: Some(posture.trace_id.clone()),
                ..Default::default()
            })
            .await
            .is_err());
    }
    let stale = ModelLaneStore::new_test_stale_access(storage.clone(), scope.clone());
    let revoked = ModelLaneStore::new_test_revoked_access(storage.clone(), scope.clone());
    assert!(stale.navigation_by_run(&posture.run_id).await.is_err());
    assert!(revoked.navigation_by_run(&posture.run_id).await.is_err());
    assert!(owner.navigation_by_run("absent-run").await.is_err());
    assert_eq!(
        owner
            .test_scoped_authority_receipts(&posture.run_id, 64)
            .await
            .expect("denials preserve canonical rows"),
        before
    );
    assert_eq!(
        owner
            .navigation_by_message(&message.message_id)
            .await
            .expect("owner remains authorized")
            .messages
            .len(),
        1
    );
    drop(owner);
    drop(storage);
    isolated
        .cleanup()
        .await
        .expect("cleanup navigation denial scope");
}

fn checkpoint(
    posture: &SurrealAdmissibleCrdtPosture,
    message: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: format!("checkpoint-{}", posture.run_id),
        run_id: posture.run_id.clone(),
        lane_id: Some(posture.lane_id.clone()),
        session_id: posture.session_id.clone(),
        model_session_id: posture.model_session_id.clone(),
        lane_status: ModelLaneStatus::Ready,
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq: message.event_ledger_seq,
        last_message_id: Some(message.message_id.clone()),
        open_payload_refs: vec![message.payload_ref.clone()],
        lease_id: None,
        idempotency_scope: format!("navigation:{}:checkpoint", posture.run_id),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some(format!("recovery-event://{}", posture.run_id)),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-010".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: message.owner_session.clone(),
        idempotency_key: format!("idem-checkpoint-{}", posture.run_id),
        created_at_utc: "2026-09-02T00:20:00Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane/navigation".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn diagnostic(posture: &SurrealAdmissibleCrdtPosture) -> NewModelLaneDiagnosticTierStatus {
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diag-{}-flight", posture.run_id),
        behavior_id: "HBR-INT-009".into(),
        run_id: posture.run_id.clone(),
        tier: ModelLaneDiagnosticTier::FlightRecorder,
        state: ModelLaneDiagnosticTierState::Wired,
        reason: "canonical receipt navigation is wired".into(),
        evidence_ref: format!("eventledger://{}/diagnostics", posture.run_id),
        follow_up_ref: Some("usermanual://model-lane/navigation".into()),
        event_ledger_stream_id: posture.message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-010".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: posture.message.owner_session.clone(),
        idempotency_key: format!("diag-{}-flight", posture.run_id),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope {
        owner_account_id: OwnerAccountId::new(format!("account-{label}")).expect("owner"),
        actor_principal_id: ActorPrincipalId::new(format!("actor-{label}")).expect("actor"),
        authenticated_session: AuthenticatedSessionRef::new(format!("session-{label}"))
            .expect("session"),
        access_space: AccessSpaceRef::new(format!("access-{label}")).expect("access"),
        workspace: Some(WorkspaceScopeRef::new(format!("workspace-{label}")).expect("workspace")),
    }
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::new("account-foreign").expect("owner");
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::new("actor-foreign").expect("actor");
    let mut session = scope.clone();
    session.authenticated_session =
        AuthenticatedSessionRef::new("session-foreign").expect("session");
    let mut access = scope.clone();
    access.access_space = AccessSpaceRef::new("access-foreign").expect("access");
    let mut workspace = scope.clone();
    workspace.workspace = Some(WorkspaceScopeRef::new("workspace-foreign").expect("workspace"));
    vec![owner, actor, session, access, workspace]
}
