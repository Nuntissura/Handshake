//! Dedicated exact-five-field ModelLane and ModelRuntime registry scope proof.

mod surreal_test_store_support;

use std::path::PathBuf;

use chrono::Utc;
use handshake_core::model_runtime::{
    BaseModelTag, ModelCapabilities, ModelId, ModelRegistration, ModelRegistryStore,
    ModelRuntimeRole, OperatorId, ProviderKind, RoleBoundModelRegistration, RuntimeBinding,
};
use handshake_core::storage::surreal::{
    bootstrap_model_registry_schema, bootstrap_schema, SurrealStorage,
};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneStatus, ModelLaneStore,
    NewModelLaneRecoveryCheckpoint,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::EmbeddedSurrealTestScope;

#[tokio::test]
async fn exact_five_scope_isolates_equal_ids_and_boot_recovery_without_leakage() {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate resource-scope database");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate injected SurrealStorage");
    bootstrap_schema(&storage).await.expect("bootstrap schema");
    let exact = exact_scope("owner");
    let owner = ModelLaneStore::new_scoped(storage.clone(), resource_scope(&exact));
    let owner_posture = posture(&owner, &exact, "scope-collision").await;
    let owner_message = owner
        .record_message(owner_posture.message.clone())
        .await
        .expect("record owner message");
    owner
        .record_recovery_checkpoint(checkpoint(&owner_posture, &owner_message))
        .await
        .expect("record owner recovery checkpoint");
    let owner_receipts = owner
        .test_scoped_authority_receipts(&owner_posture.run_id, 128)
        .await
        .expect("owner receipt snapshot");

    for (index, foreign_exact) in one_field_mismatches(&exact).into_iter().enumerate() {
        let foreign = ModelLaneStore::new_scoped(storage.clone(), resource_scope(&foreign_exact));
        assert!(foreign.replay_run(&owner_posture.run_id).await.is_err());
        assert!(foreign
            .recover_restartable_runs_at_boot()
            .await
            .expect("foreign boot scan is non-leaking")
            .is_empty());
        assert!(foreign
            .test_scoped_authority_receipts(&owner_posture.run_id, 128)
            .await
            .expect("foreign receipt scan is non-leaking")
            .is_empty());

        let foreign_posture = posture(&foreign, &foreign_exact, "scope-collision").await;
        let foreign_message = foreign
            .record_message(foreign_posture.message.clone())
            .await
            .expect("equal logical IDs are independent in a foreign exact scope");
        assert_eq!(foreign_message.message_id, owner_message.message_id);
        assert_ne!(
            foreign_message.event_ledger_event_id, owner_message.event_ledger_event_id,
            "scope collision {index} must have independent canonical receipt"
        );
        assert!(owner.record_message(foreign_posture.message).await.is_err());
    }

    let incomplete = ResourceScope::new(exact.owner_account_id, exact.actor_principal_id);
    let incomplete_store = ModelLaneStore::new_scoped(storage.clone(), incomplete);
    assert!(incomplete_store
        .replay_run(&owner_posture.run_id)
        .await
        .is_err());
    assert_eq!(
        owner
            .test_scoped_authority_receipts(&owner_posture.run_id, 128)
            .await
            .expect("scope denials are non-mutating"),
        owner_receipts
    );
    assert_eq!(
        owner
            .recover_restartable_runs_at_boot()
            .await
            .expect("owner boot recovery")
            .len(),
        1
    );

    drop(owner);
    drop(storage);
    isolated.cleanup().await.expect("cleanup resource scope");
}

#[tokio::test]
async fn model_registry_reads_are_exact_five_scoped_for_every_field() {
    let mut isolated = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate registry scope database");
    let storage = isolated
        .activate_storage()
        .await
        .expect("activate injected SurrealStorage");
    bootstrap_schema(&storage).await.expect("bootstrap schema");
    bootstrap_model_registry_schema(&storage)
        .await
        .expect("bootstrap registry schema");
    let registry = ModelRegistryStore::new(storage.clone());
    let exact = exact_scope("registry-owner");
    registry
        .ensure_workspace_for_tests(&exact)
        .await
        .expect("create owner workspace predecessor");
    let registration = completion_registration(0x51, "scope-registry");
    let owner_rows = registry
        .persist_role_bound_boot_set_and_read_back(&exact, &[registration])
        .await
        .expect("persist owner registry row");
    assert_eq!(owner_rows.len(), 1);

    for foreign in one_field_mismatches(&exact) {
        registry
            .ensure_workspace_for_tests(&foreign)
            .await
            .expect("create foreign workspace predecessor");
        assert!(registry
            .list_recoverable(&foreign)
            .await
            .expect("foreign registry list is non-leaking")
            .is_empty());
    }
    assert_eq!(
        registry
            .list_recoverable(&exact)
            .await
            .expect("owner registry row remains readable")
            .len(),
        1
    );

    drop(registry);
    drop(storage);
    isolated.cleanup().await.expect("cleanup registry scope");
}

async fn posture(
    store: &ModelLaneStore,
    exact: &ExactResourceScopeAttribution,
    label: &str,
) -> SurrealAdmissibleCrdtPosture {
    build_surreal_admissible_crdt_posture(store, exact.workspace_id.as_str(), label)
        .await
        .expect("build exact-scope production posture")
}

fn checkpoint(
    posture: &SurrealAdmissibleCrdtPosture,
    message: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: "checkpoint-resource-scope".into(),
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
        idempotency_scope: format!("recovery:{}:scope", posture.run_id),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some("recovery-event://resource-scope".into()),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-011".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: message.owner_session.clone(),
        idempotency_key: "idem-checkpoint-resource-scope".into(),
        created_at_utc: "2026-09-02T02:10:00Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn completion_registration(artifact_byte: u8, label: &str) -> RoleBoundModelRegistration {
    RoleBoundModelRegistration::completion(ModelRegistration {
        model_id: ModelId::new_v7(),
        artifact_path: PathBuf::from(format!("fixtures/models/{label}.safetensors")),
        sha256: [artifact_byte; 32],
        runtime_binding: RuntimeBinding::Candle,
        declared_capabilities: ModelCapabilities::default(),
        base_model_tag: BaseModelTag::new(label),
        registered_at_utc: Utc::now(),
        registered_by: OperatorId::new("resource-scope-surreal-proof"),
        provider: ProviderKind::Local,
    })
}

fn exact_scope(label: &str) -> ExactResourceScopeAttribution {
    ExactResourceScopeAttribution {
        owner_account_id: OwnerAccountId::mint(),
        actor_principal_id: ActorPrincipalId::mint(),
        authenticated_session_id: AuthenticatedSessionRef::mint(),
        access_space_id: AccessSpaceRef::mint(),
        workspace_id: WorkspaceScopeRef::new(format!("workspace-{label}")).expect("workspace"),
    }
}

fn resource_scope(exact: &ExactResourceScopeAttribution) -> ResourceScope {
    ResourceScope::new(exact.owner_account_id, exact.actor_principal_id)
        .with_session(exact.authenticated_session_id)
        .with_access_space(exact.access_space_id)
        .with_workspace(exact.workspace_id.clone())
}

fn one_field_mismatches(
    exact: &ExactResourceScopeAttribution,
) -> Vec<ExactResourceScopeAttribution> {
    let mut owner = exact.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = exact.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = exact.clone();
    session.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access = exact.clone();
    access.access_space_id = AccessSpaceRef::mint();
    let mut workspace = exact.clone();
    workspace.workspace_id = WorkspaceScopeRef::new("workspace-foreign").expect("workspace");
    vec![owner, actor, session, access, workspace]
}
