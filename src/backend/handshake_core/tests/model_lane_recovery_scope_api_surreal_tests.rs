//! MT-007 exact-scope recovery API denial and non-mutation proof.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneError, ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneStatus,
    ModelLaneStore, NewModelLaneRecoveryCheckpoint,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceAccessLifecycleRegistry, ResourceScope, ScopeDenied, WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::EmbeddedSurrealTestScope;

#[tokio::test]
async fn recovery_denies_each_scope_mismatch_stale_revoked_and_incomplete_context() {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate recovery scope API database");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate SurrealStorage");
    bootstrap_schema(&storage).await.expect("bootstrap schema");
    let scope = exact_scope("scope-api");
    let owner_lifecycle = active_lifecycle(&scope);
    let owner =
        ModelLaneStore::new_scoped_with_lifecycle(storage.clone(), scope.clone(), owner_lifecycle);
    let posture = build_surreal_admissible_crdt_posture(
        &owner,
        scope.workspace.as_ref().expect("workspace").as_str(),
        "scope-api",
    )
    .await
    .expect("seed production posture");
    let message = owner
        .record_message(posture.message.clone())
        .await
        .expect("record message");
    owner
        .record_recovery_checkpoint(checkpoint(&posture, &message))
        .await
        .expect("record checkpoint");
    let before = owner
        .test_scoped_authority_receipts(&posture.run_id, 64)
        .await
        .expect("baseline receipts");

    for foreign_scope in one_field_mismatches(&scope) {
        let foreign_lifecycle = active_lifecycle(&foreign_scope);
        let foreign = ModelLaneStore::new_scoped_with_lifecycle(
            storage.clone(),
            foreign_scope,
            foreign_lifecycle,
        );
        assert!(foreign
            .recover_run_after_restart(&posture.run_id)
            .await
            .is_err());
        assert!(foreign
            .recover_restartable_runs_at_boot()
            .await
            .expect("foreign boot scan is non-leaking")
            .is_empty());
        assert!(foreign
            .test_scoped_authority_receipts(&posture.run_id, 64)
            .await
            .expect("foreign receipt scan is non-leaking")
            .is_empty());
    }

    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
        .expect("exact lifecycle tuple");
    let stale_lifecycle = active_lifecycle(&scope);
    let stale = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        scope.clone(),
        stale_lifecycle.clone(),
    );
    stale_lifecycle
        .mark_stale(&exact)
        .expect("mark exact tuple stale");
    let stale_error = stale
        .recover_run_after_restart(&posture.run_id)
        .await
        .expect_err("stale exact tuple must deny recovery");
    assert!(matches!(
        stale_error,
        ModelLaneError::ScopeDenied(ScopeDenied::LifecycleStale)
    ));

    let revoked_lifecycle = active_lifecycle(&scope);
    let revoked = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(),
        scope.clone(),
        revoked_lifecycle.clone(),
    );
    revoked_lifecycle
        .revoke(&exact)
        .expect("revoke exact tuple");
    let revoked_error = revoked
        .recover_run_after_restart(&posture.run_id)
        .await
        .expect_err("revoked exact tuple must deny recovery");
    assert!(matches!(
        revoked_error,
        ModelLaneError::ScopeDenied(ScopeDenied::LifecycleRevoked)
    ));
    assert!(revoked_lifecycle.register_active(exact.clone()).is_err());
    let mut new_session_scope = scope.clone();
    new_session_scope.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let new_session_exact =
        ExactResourceScopeAttribution::try_from_resource_scope(&new_session_scope)
            .expect("new exact session tuple");
    revoked_lifecycle
        .register_active(new_session_exact)
        .expect("a new authenticated session identity may register active");
    let mut incomplete_scope = scope.clone();
    incomplete_scope.workspace = None;
    let incomplete = ModelLaneStore::new_scoped(storage.clone(), incomplete_scope);
    assert!(incomplete
        .recover_run_after_restart(&posture.run_id)
        .await
        .is_err());

    let mut conflict = checkpoint(&posture, &message);
    conflict.session_id.push_str("-conflict");
    assert!(owner.record_recovery_checkpoint(conflict).await.is_err());
    assert_eq!(
        owner
            .test_scoped_authority_receipts(&posture.run_id, 64)
            .await
            .expect("denials preserve receipt count"),
        before
    );
    assert_eq!(
        owner
            .recover_run_after_restart(&posture.run_id)
            .await
            .expect("owner recovery survives denials")
            .checkpoint
            .checkpoint_id,
        "checkpoint-scope-api"
    );

    drop(owner);
    drop(storage);
    isolated
        .cleanup()
        .await
        .expect("cleanup recovery scope API");
}

fn checkpoint(
    posture: &SurrealAdmissibleCrdtPosture,
    message: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: "checkpoint-scope-api".into(),
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
        idempotency_scope: format!("recovery:{}:scope-api", posture.run_id),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some("recovery-event://scope-api".into()),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: message.owner_session.clone(),
        idempotency_key: "idem-checkpoint-scope-api".into(),
        created_at_utc: "2026-09-02T00:10:00Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(WorkspaceScopeRef::new(format!("workspace-{label}")).expect("workspace"))
}

fn active_lifecycle(scope: &ResourceScope) -> ResourceAccessLifecycleRegistry {
    let lifecycle = ResourceAccessLifecycleRegistry::new();
    lifecycle
        .register_active(
            ExactResourceScopeAttribution::try_from_resource_scope(scope).expect("exact scope"),
        )
        .expect("register active exact scope");
    lifecycle
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = scope.clone();
    session.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let mut access = scope.clone();
    access.access_space = Some(AccessSpaceRef::mint());
    let mut workspace = scope.clone();
    workspace.workspace = Some(WorkspaceScopeRef::new("workspace-foreign").expect("workspace"));
    vec![owner, actor, session, access, workspace]
}
