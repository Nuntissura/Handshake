//! MT-007 embedded-Surreal recovery, lease, diagnostic, and MT-status proof.

mod surreal_test_store_support;

use chrono::{DateTime, Utc};
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::storage::knowledge::{KnowledgeStore, NewKnowledgeRichDocument};
use handshake_core::storage::surreal::{SurrealDatabase, SurrealStorage};
use handshake_core::storage::surreal::local_accounts::{password_verifier, LocalSessionContext};
use handshake_core::storage::surreal::resource_authority::{
    AuthorizationRequest, ResourceAction, ResourceKind,
};
use handshake_core::storage::NewWorkspace;
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneAuthority, ModelLaneAuthorityTestCorruption, ModelLaneDiagnosticTier,
    ModelLaneDiagnosticTierState, ModelLaneMessageKind,
    ModelLaneLeaseScope, ModelLaneLeaseState, ModelLaneMtRuntimeStatus, ModelLaneRecoveryEventKind,
    ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneResult, ModelLaneStatus, ModelLaneStore,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneDiagnosticTierStatus,
    NewModelLaneLease, NewModelLaneMessage, NewModelLaneMtRuntimeStatus,
    NewModelLaneRecoveryCheckpoint, NewModelLaneRecoveryEvent,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef, ExactResourceScopeAttribution, ResourceAccessLifecycleRegistry,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture_authenticated, AuthenticatedModelLaneTestOperation,
    SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::IsolatedSurrealTestStore;

struct Harness {
    isolated: IsolatedSurrealTestStore,
    storage: SurrealStorage,
    scope: ResourceScope,
    lifecycle: ResourceAccessLifecycleRegistry,
    session_token: String,
    channel_hash: String,
    document_id: String,
    store: ModelLaneStore,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let isolated = IsolatedSurrealTestStore::create()
            .await
            .expect("allocate recovery scope");
        let storage = isolated.storage().clone();
        let password = format!("recovery-password-{}", uuid::Uuid::now_v7());
        let verifier_password = password.clone();
        let verifier = tokio::task::spawn_blocking(move || password_verifier(&verifier_password))
            .await.expect("password worker").expect("production password verifier");
        let capabilities = CapabilityRegistry::new().profile_by_id("Operator")
            .expect("production Operator profile").allowed.clone();
        let account_name = format!("recovery-{label}");
        let identity = storage.setup_local_owner(&account_name, verifier, capabilities)
            .await.expect("normal first Owner setup");
        let credential = storage.login_local_owner(&account_name, password)
            .await.expect("normal Owner password login");
        assert_eq!(credential.account_id, identity.account_id);
        assert_eq!(credential.principal_id, identity.principal_id);
        assert_eq!(credential.access_space_id, identity.access_space_id);
        let channel_hash = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(
            uuid::Uuid::now_v7().as_bytes(),
        ));
        let session = storage.exchange_session_credential(
            &credential.account_id, &credential.principal_id, &credential.access_space_id,
            &credential.token, &channel_hash, std::time::Duration::from_secs(3600),
        ).await.expect("consume genuine login credential");
        let context = storage.authenticate_local_session(&session.token, &channel_hash)
            .await.expect("authenticate issued session");
        assert_eq!(context.session_id, session.session_id);
        let workspace = AuthenticatedModelLaneTestOperation::create_workspace(
            &storage, &session.token, &channel_hash,
            NewWorkspace { name: format!("recovery-{label}") },
        )
            .await.expect("canonical workspace and creator grant");
        let scope = exact_scope(&context, &workspace.id);
        let lifecycle = storage.resource_access_lifecycle();
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
            .expect("canonical five-field scope");
        lifecycle.register_active(exact.clone()).expect("activate authenticated workspace");
        let store = ModelLaneStore::new_scoped_with_lifecycle(
            storage.clone(), scope.clone(), lifecycle.clone(),
        );
        let workspace_create = AuthenticatedModelLaneTestOperation::authorize(
            storage.clone(), AuthorizationRequest {
                session_token: session.token.clone(), channel_binding_hash: Some(channel_hash.clone()),
                capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
                external_resource_id: workspace.id.clone(), action: ResourceAction::Create,
            }, exact,
        ).await.expect("authorize original workspace Create");
        let document = workspace_create.run(async {
            Ok(SurrealDatabase::new(storage.clone()).create_knowledge_rich_document(
                NewKnowledgeRichDocument {
                    workspace_id: workspace.id.clone(), document_id: None,
                    title: format!("Recovery {label}"),
                    schema_version: handshake_core::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION.to_string(),
                    content_json: json!({"type": "doc", "content": []}),
                    crdt_document_id: None, crdt_snapshot_id: None, promotion_receipt_event_id: None,
                    project_ref: None, folder_ref: None, authority_label: Some("promoted".into()),
                    owner_actor_kind: Some(context.actor_kind.clone()), owner_actor_id: Some(context.actor_id.clone()),
                },
            ).await.expect("canonical rich document and creator grant"))
        }).await.expect("create document under original authority");
        Self { isolated, storage, scope, lifecycle, session_token: session.token,
            channel_hash, document_id: document.rich_document_id, store }
    }

    async fn operation(&self, capability: &str, kind: ResourceKind, external: &str,
        action: ResourceAction) -> ModelLaneResult<AuthenticatedModelLaneTestOperation> {
        AuthenticatedModelLaneTestOperation::authorize(self.storage.clone(), AuthorizationRequest {
            session_token: self.session_token.clone(), channel_binding_hash: Some(self.channel_hash.clone()),
            capability_id: capability.into(), resource_kind: kind,
            external_resource_id: external.into(), action,
        }, ExactResourceScopeAttribution::try_from_resource_scope(&self.scope)
            .expect("canonical five-field scope")).await
    }

    async fn create_record<T>(&self, future: impl std::future::Future<Output = ModelLaneResult<T>>)
        -> ModelLaneResult<T> {
        self.operation("fs.write", ResourceKind::Workspace,
            self.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Create)
            .await?.run(future).await
    }

    async fn read<T>(&self, future: impl std::future::Future<Output = ModelLaneResult<T>>)
        -> ModelLaneResult<T> {
        self.operation("fs.read", ResourceKind::Workspace,
            self.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Read)
            .await?.run(future).await
    }

    async fn recover<T>(&self, future: impl std::future::Future<Output = ModelLaneResult<T>>)
        -> ModelLaneResult<T> {
        self.operation("model_lane.recovery.execute", ResourceKind::Workspace,
            self.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Update)
            .await?.run(future).await
    }

    async fn posture(&self, label: &str) -> SurrealAdmissibleCrdtPosture {
        let workspace = self.scope.workspace.as_ref().expect("workspace").as_str();
        let workspace_create = self.operation("fs.write", ResourceKind::Workspace,
            workspace, ResourceAction::Create).await.expect("original workspace Create");
        let document_update = self.operation("fs.write", ResourceKind::RichDocument,
            &self.document_id, ResourceAction::Update).await.expect("original Document Update");
        build_surreal_admissible_crdt_posture_authenticated(
            &self.store, workspace, label, "model://local/mt018", &self.document_id,
            &workspace_create, &document_update,
        ).await.expect("seed authenticated production posture")
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated.cleanup().await.expect("cleanup recovery scope");
    }
}

#[tokio::test]
async fn recovery_replays_checkpoint_events_leases_diagnostics_and_mt_status_after_restart() {
    let mut harness = Harness::create("replay").await;
    let posture = harness.posture("replay").await;
    let message = harness.create_record(harness
        .store
        .record_message(posture.message.clone()))
        .await
        .expect("record source message");
    let checkpoint = checkpoint(&posture, &message, "checkpoint-replay");
    let stored_checkpoint = harness.create_record(harness
        .store
        .record_recovery_checkpoint(checkpoint.clone()))
        .await
        .expect("record checkpoint atomically");
    assert_eq!(
        harness.create_record(harness
            .store
            .record_recovery_checkpoint(checkpoint))
            .await
            .expect("checkpoint retry"),
        stored_checkpoint
    );
    let event = recovery_event(&posture, &message, "event-replay");
    let stored_event = harness.create_record(harness
        .store
        .record_recovery_event(event.clone()))
        .await
        .expect("record ordered recovery event");
    assert_eq!(
        harness.create_record(harness
            .store
            .record_recovery_event(event))
            .await
            .expect("event retry"),
        stored_event
    );
    harness.create_record(harness
        .store
        .record_lane_lease(lease(&posture, "lease-replay", "2099-01-01T00:00:00Z")))
        .await
        .expect("record active lease");
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        harness.create_record(harness
            .store
            .record_diagnostic_tier_status(diagnostic(&posture, tier)))
            .await
            .expect("record diagnostic tier");
    }
    harness.create_record(harness
        .store
        .record_mt_runtime_status(mt_status(&posture)))
        .await
        .expect("record MT status");

    let now = DateTime::parse_from_rfc3339("2026-09-02T01:00:00Z")
        .expect("timestamp")
        .with_timezone(&Utc);
    let before = harness.recover(harness
        .store
        .test_recover_run_after_restart_at(&posture.run_id, now))
        .await
        .expect("recover exact run");
    assert_eq!(before.checkpoint.checkpoint_id, "checkpoint-replay");
    assert_eq!(before.recovery_events, vec![stored_event]);
    assert_eq!(before.active_leases.len(), 1);
    assert!(before.reclaimable_lease_ids.is_empty());
    assert_eq!(before.mt_runtime_statuses.len(), 1);
    harness.read(harness
        .store
        .validate_diagnostic_tier_posture(&posture.run_id, "HBR-INT-009"))
        .await
        .expect("three-tier diagnostic posture");
    let receipts = harness.read(harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64))
        .await
        .expect("inspect bounded canonical receipts");
    assert!(receipts
        .iter()
        .any(|row| row.record_kind == "recovery_checkpoint"));
    assert!(receipts
        .iter()
        .any(|row| row.record_kind == "recovery_event"));

    drop(harness.store);
    drop(harness.storage);
    harness.isolated.reopen().await.expect("reopen same scope");
    let storage = harness.isolated.storage().clone();
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope)
        .expect("retained canonical five-field scope");
    harness.lifecycle.authorize(&exact).expect("original context remains active");
    let context = storage.authenticate_local_session(&harness.session_token, &harness.channel_hash)
        .await.expect("same authenticated session remains live after reopen");
    assert_eq!(ExactResourceScopeAttribution::try_from_resource_scope(&exact_scope(
        &context, exact.workspace_id.as_str(),
    )).expect("reauthenticated five-field scope"), exact);
    AuthenticatedModelLaneTestOperation::authorize(storage.clone(), AuthorizationRequest {
        session_token: harness.session_token.clone(),
        channel_binding_hash: Some(harness.channel_hash.clone()),
        capability_id: "model_lane.recovery.execute".into(), resource_kind: ResourceKind::Workspace,
        external_resource_id: exact.workspace_id.as_str().to_owned(), action: ResourceAction::Update,
    }, exact.clone()).await.expect("reauthorize original recovery operation after reopen");
    let lifecycle = storage.resource_access_lifecycle();
    lifecycle.register_active(exact).expect("activate reauthorized context on reopened storage");
    let reopened = ModelLaneStore::new_scoped_with_lifecycle(
        storage.clone(), harness.scope.clone(), lifecycle.clone(),
    );
    harness.lifecycle = lifecycle;
    harness.store = reopened;
    harness.storage = storage;
    assert_eq!(
        harness.recover(harness.store
            .test_recover_run_after_restart_at(&posture.run_id, now))
            .await
            .expect("recover after restart"),
        before
    );
    assert_eq!(
        harness.recover(harness.store
            .recover_restartable_runs_at_boot())
            .await
            .expect("boot recovery")
            .len(),
        1
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_lease_expiry_is_deterministic_and_does_not_widen_scope() {
    let harness = Harness::create("lease-expiry").await;
    let posture = harness.posture("lease-expiry").await;
    let message = harness.create_record(harness
        .store
        .record_message(posture.message.clone()))
        .await
        .expect("record source message");
    harness.create_record(harness
        .store
        .record_recovery_checkpoint(checkpoint(&posture, &message, "checkpoint-expiry")))
        .await
        .expect("record checkpoint");
    harness.create_record(harness
        .store
        .record_lane_lease(lease(&posture, "lease-expired", "2026-09-02T00:00:00Z")))
        .await
        .expect("record lease");
    let now = DateTime::parse_from_rfc3339("2026-09-02T01:00:00Z")
        .expect("timestamp")
        .with_timezone(&Utc);
    let recovered = harness.recover(harness
        .store
        .test_recover_run_after_restart_at(&posture.run_id, now))
        .await
        .expect("recover expired lease");
    assert!(recovered.active_leases.is_empty());
    assert_eq!(recovered.reclaimable_lease_ids, vec!["lease-expired"]);
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_rejects_checkpoint_projection_and_receipt_corruption_at_boot_and_direct_read() {
    for (index, corruption) in [
        ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
        ModelLaneAuthorityTestCorruption::ProjectionScope,
        ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
        ModelLaneAuthorityTestCorruption::ReceiptScope,
    ]
    .into_iter()
    .enumerate()
    {
        let label = format!("corrupt-{index}");
        let harness = Harness::create(&label).await;
        let posture = harness.posture(&label).await;
        let message = harness.create_record(harness
            .store
            .record_message(posture.message.clone()))
            .await
            .expect("record source message");
        let checkpoint_id = format!("checkpoint-corrupt-{index}");
        harness.create_record(harness
            .store
            .record_recovery_checkpoint(checkpoint(&posture, &message, &checkpoint_id)))
            .await
            .expect("record canonical checkpoint");
        harness.create_record(harness
            .store
            .test_corrupt_scoped_authority("recovery_checkpoint", &checkpoint_id, corruption))
            .await
            .expect("apply enumerated checkpoint corruption");
        assert!(harness.recover(harness
            .store
            .recover_run_after_restart(&posture.run_id))
            .await
            .is_err());
        assert!(harness.recover(harness
            .store
            .recover_restartable_runs_at_boot())
            .await
            .is_err());
        harness.cleanup().await;
    }
}

#[tokio::test]
async fn recovery_rejects_cross_stream_checkpoint_and_event_sequence_gap_without_mutation() {
    let harness = Harness::create("stream-gap").await;
    let posture = harness.posture("stream-gap").await;
    let message = harness.create_record(harness
        .store
        .record_message(posture.message.clone()))
        .await
        .expect("record source message");
    let before = harness.read(harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64))
        .await
        .expect("baseline authority receipts");

    let mut foreign_stream = checkpoint(&posture, &message, "checkpoint-foreign-stream");
    foreign_stream.event_ledger_stream_id = "model-lane://foreign/stream".into();
    assert!(harness.create_record(harness
        .store
        .record_recovery_checkpoint(foreign_stream))
        .await
        .is_err());
    assert_eq!(
        harness.read(harness
            .store
            .test_scoped_authority_receipts(&posture.run_id, 64))
            .await
            .expect("cross-stream denial receipt watermark"),
        before
    );

    harness.create_record(harness
        .store
        .record_recovery_checkpoint(checkpoint(&posture, &message, "checkpoint-stream-gap")))
        .await
        .expect("record canonical checkpoint");
    let before_gap = harness.read(harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64))
        .await
        .expect("pre-gap authority watermark");
    let mut gap = recovery_event(&posture, &message, "event-sequence-gap");
    gap.source_event_ledger_seq = Some(message.event_ledger_seq + 10_000);
    let gap_result = harness.create_record(harness.store.record_recovery_event(gap)).await;
    let after = harness.read(harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64))
        .await
        .expect("inspect authority after denials");
    match gap_result {
        Err(_) => assert_eq!(after, before_gap),
        Ok(_) => {
            assert!(harness.recover(harness
                .store
                .recover_run_after_restart(&posture.run_id))
                .await
                .is_err());
            assert_eq!(after.len(), before_gap.len() + 1);
        }
    }
    harness.cleanup().await;
}

#[tokio::test]
async fn concurrent_recovery_child_events_receive_unique_canonical_order() {
    let harness = Harness::create("concurrent-children").await;
    let posture = harness.posture("concurrent-children").await;
    let message = harness.create_record(harness
        .store
        .record_message(posture.message.clone()))
        .await
        .expect("record source message");
    harness.create_record(harness
        .store
        .record_recovery_checkpoint(checkpoint(
            &posture,
            &message,
            "checkpoint-concurrent-children",
        )))
        .await
        .expect("record recovery parent");

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let first_operation = harness.operation("fs.write", ResourceKind::Workspace,
        harness.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Create)
        .await.expect("authorize concurrent recovery child");
    let second_operation = first_operation.clone();
    let first_store = harness.store.clone();
    let first_barrier = barrier.clone();
    let first_event = recovery_event(&posture, &message, "event-child-a");
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        first_operation.run(first_store.record_recovery_event(first_event)).await
    });
    let second_store = harness.store.clone();
    let second_barrier = barrier.clone();
    let second_event = recovery_event(&posture, &message, "event-child-b");
    let second = tokio::spawn(async move {
        second_barrier.wait().await;
        second_operation.run(second_store.record_recovery_event(second_event)).await
    });
    barrier.wait().await;
    let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(first, second)
    })
    .await
    .expect("concurrent recovery children must not deadlock");
    let first = first
        .expect("first child joins")
        .expect("first child persists");
    let second = second
        .expect("second child joins")
        .expect("second child persists");
    assert_ne!(first.replay_order_seq, second.replay_order_seq);

    let recovered = harness.recover(harness
        .store
        .recover_run_after_restart(&posture.run_id))
        .await
        .expect("recover both children");
    assert_eq!(recovered.recovery_events.len(), 2);
    assert!(
        recovered.recovery_events[0].replay_order_seq
            < recovered.recovery_events[1].replay_order_seq
    );
    harness.cleanup().await;
}

fn checkpoint(
    posture: &SurrealAdmissibleCrdtPosture,
    message: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
    checkpoint_id: &str,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: checkpoint_id.into(),
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
        idempotency_scope: format!("recovery:{}:{checkpoint_id}", posture.run_id),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some(format!("recovery-event://{checkpoint_id}")),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: message.owner_session.clone(),
        idempotency_key: format!("idem-{checkpoint_id}"),
        created_at_utc: "2026-09-02T00:10:00Z".into(),
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn recovery_event(
    posture: &SurrealAdmissibleCrdtPosture,
    message: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
    event_id: &str,
) -> NewModelLaneRecoveryEvent {
    NewModelLaneRecoveryEvent {
        recovery_event_id: event_id.into(),
        run_id: posture.run_id.clone(),
        lane_id: Some(posture.lane_id.clone()),
        trace_id: posture.trace_id.clone(),
        span_id: format!("span-{event_id}"),
        parent_span_id: Some(message.message_span_id.clone()),
        linked_span_contexts: vec![posture.trace_id.clone()],
        session_id: Some(posture.session_id.clone()),
        model_session_id: Some(posture.model_session_id.clone()),
        event_kind: ModelLaneRecoveryEventKind::MessageRecorded,
        recovery_status: ModelLaneRecoveryStatus::Observed,
        replay_order_seq: 0,
        source_event_ledger_seq: Some(message.event_ledger_seq),
        payload_refs: vec![message.payload_ref.clone()],
        artifact_refs: Vec::new(),
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_stale_base_ref: None,
        lease_id: None,
        failure_kind: None,
        error_code: None,
        replay_hint: "replay the exact scoped canonical event stream".into(),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: message.owner_session.clone(),
        idempotency_key: format!("idem-{event_id}"),
        recovery_hint_ref: Some("usermanual://model-lane/recovery".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn lease(
    posture: &SurrealAdmissibleCrdtPosture,
    lease_id: &str,
    expires: &str,
) -> NewModelLaneLease {
    NewModelLaneLease {
        lease_id: lease_id.into(),
        run_id: posture.run_id.clone(),
        lane_id: Some(posture.lane_id.clone()),
        scope: ModelLaneLeaseScope::Lane,
        scope_ref: format!("model-lane://{}/{}", posture.run_id, posture.lane_id),
        holder_actor_id: posture.actor_id.clone(),
        holder_session_id: posture.session_id.clone(),
        lease_expires_at_utc: expires.into(),
        takeover_policy_ref: "lease-policy://mt007/recover-or-reclaim".into(),
        state: ModelLaneLeaseState::Active,
        event_ledger_stream_id: posture.message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: posture.message.owner_session.clone(),
        idempotency_key: format!("idem-{lease_id}"),
        recovery_hint_ref: Some("usermanual://model-lane/recovery#lease".into()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn diagnostic(
    posture: &SurrealAdmissibleCrdtPosture,
    tier: ModelLaneDiagnosticTier,
) -> NewModelLaneDiagnosticTierStatus {
    NewModelLaneDiagnosticTierStatus {
        diagnostic_status_id: format!("diag-{}-{}", posture.run_id, tier.as_str()),
        behavior_id: "HBR-INT-009".into(),
        run_id: posture.run_id.clone(),
        tier,
        state: ModelLaneDiagnosticTierState::Wired,
        reason: "embedded recovery evidence is available".into(),
        evidence_ref: format!("eventledger://{}/diagnostics", posture.run_id),
        follow_up_ref: Some("usermanual://model-lane/recovery".into()),
        event_ledger_stream_id: posture.message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: posture.message.owner_session.clone(),
        idempotency_key: format!("diag-{}-{}", posture.run_id, tier.as_str()),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn mt_status(posture: &SurrealAdmissibleCrdtPosture) -> NewModelLaneMtRuntimeStatus {
    NewModelLaneMtRuntimeStatus {
        mt_status_id: format!("mt-status-{}", posture.run_id),
        run_id: posture.run_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        status: ModelLaneMtRuntimeStatus::ReadyForValidation,
        claimed_by_ref: Some(format!("session://{}", posture.session_id)),
        blocker_ref: None,
        missing_resource_ref: None,
        proof_status_ref: Some("proof://mt007/embedded-surreal".into()),
        hbr_status_ref: Some("hbr-int-009://model-lane/recovery".into()),
        last_recovery_event_ref: Some("recovery-event://event-replay".into()),
        last_runtime_status_ref: Some("runtime-status://mt007/ready".into()),
        event_ledger_stream_id: posture.message.event_ledger_stream_id.clone(),
        owner_session: posture.message.owner_session.clone(),
        idempotency_key: format!("idem-mt-status-{}", posture.run_id),
        diagnostic_payload: json!({"authority": "kernel_event_ledger"}),
    }
}

fn exact_scope(context: &LocalSessionContext, workspace: &str) -> ResourceScope {
    let parse = |id: &str| uuid::Uuid::parse_str(id).expect("canonical identity UUID");
    ResourceScope {
        owner_account_id: OwnerAccountId::from_uuid(parse(&context.identity.account_id)),
        actor_principal_id: ActorPrincipalId::from_uuid(parse(&context.identity.principal_id)),
        authenticated_session: Some(AuthenticatedSessionRef::from_uuid(parse(&context.session_id))),
        access_space: Some(AccessSpaceRef::from_uuid(parse(&context.identity.access_space_id))),
        workspace: Some(WorkspaceScopeRef::new(workspace.to_owned()).expect("canonical workspace")),
    }
}
#[tokio::test]
async fn recovery_rejects_ordinary_message_without_payload_authority_even_with_empty_checkpoint_refs() {
    let harness = Harness::create("ordinary-missing-payload").await;
    let posture = harness.posture("ordinary-missing-payload").await;
    let mut ordinary = posture.message.clone();
    ordinary.message_id = "message-ordinary-without-payload-authority".into();
    ordinary.message_span_id = "span-ordinary-without-payload-authority".into();
    ordinary.idempotency_key = "idem-ordinary-without-payload-authority".into();
    ordinary.replay_order_key = "ordinary-without-payload-authority".into();
    ordinary.kind = ModelLaneMessageKind::Status;
    ordinary.authority = ModelLaneAuthority::Advisory;
    ordinary.payload_ref = "artifact://recovery/ordinary-without-payload-authority".into();
    ordinary.payload_sha256 = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(
        b"ordinary status payload without a persisted artifact binding",
    ));
    ordinary.proposal_ref = None;
    ordinary.crdt_update_ref = None;
    ordinary.crdt_base_snapshot_ref = None;
    ordinary.crdt_state_vector = None;
    ordinary.crdt_proposal_ref = None;
    ordinary.crdt_stale_base_ref = None;
    ordinary.diagnostic_payload = json!({"case": "ordinary payload witness deliberately absent"});
    let message = harness.create_record(harness.store.record_message(ordinary))
        .await.expect("ordinary source message is admitted through original Create authority");
    assert!(message.crdt_authority_binding.is_none());
    let mut empty_refs = checkpoint(&posture, &message, "checkpoint-empty-payload-refs");
    empty_refs.open_payload_refs.clear();
    let stored_checkpoint = harness.create_record(harness.store.record_recovery_checkpoint(empty_refs))
        .await.expect("persist canonical checkpoint with empty payload-ref list");
    assert!(stored_checkpoint.open_payload_refs.is_empty());
    let before = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 64))
        .await.expect("canonical receipts before rejected recovery");
    let error = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect_err("empty refs cannot waive ordinary message payload authority");
    assert!(error.to_string().contains("CX-MM-006"), "canonical missing-payload code: {error}");
    assert_eq!(
        harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 64))
            .await.expect("canonical receipts after rejected recovery"),
        before,
        "missing payload authority must fail before recovery mutations"
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_bounds_returned_replay_but_reconciles_current_leases_and_forward_messages() {
    let harness = Harness::create("bounded-replay").await;
    let posture = harness.posture("bounded-replay").await;
    let first = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("record genuine CRDT-backed checkpoint message");
    let checkpoint = harness.create_record(harness.store.record_recovery_checkpoint(
        checkpoint(&posture, &first, "checkpoint-bounded-replay"),
    )).await.expect("record checkpoint before later lane");
    let source = harness.read(harness.store.replay_run(&posture.run_id))
        .await.expect("read canonical source lane");
    let mut later_lane = source.lanes.iter().find(|lane| lane.lane_id == posture.lane_id)
        .expect("original lane is canonical").inner.clone();
    later_lane.lane_id = "lane-after-checkpoint-without-message".into();
    later_lane.lane_span_id = "span-lane-after-checkpoint-without-message".into();
    let later_lane = harness.create_record(harness.store.record_lane(later_lane))
        .await.expect("create genuine later lane under original Create authority");
    assert!(later_lane.event_ledger_seq > checkpoint.last_event_ledger_seq);
    let mut active = lease(&posture, "lease-later-active", "2099-01-01T00:00:00Z");
    active.lane_id = Some(later_lane.lane_id.clone());
    active.scope_ref = format!("model-lane://{}/{}", posture.run_id, later_lane.lane_id);
    let active = harness.create_record(harness.store.record_lane_lease(active))
        .await.expect("record current active lease beyond checkpoint");
    let mut expired = lease(&posture, "lease-later-expired", "2026-09-02T00:00:00Z");
    expired.lane_id = Some(later_lane.lane_id.clone());
    expired.scope_ref = format!("model-lane://{}/{}", posture.run_id, later_lane.lane_id);
    let expired = harness.create_record(harness.store.record_lane_lease(expired))
        .await.expect("record current expired lease beyond checkpoint");
    let now = DateTime::parse_from_rfc3339("2026-09-02T01:00:00Z")
        .expect("deterministic recovery time").with_timezone(&Utc);
    let bounded = harness.recover(harness.store.test_recover_run_after_restart_at(&posture.run_id, now))
        .await.expect("recover bounded replay and current leases");
    assert_eq!(bounded.replay.messages, vec![first.clone()]);
    assert!(!bounded.replay.lanes.iter().any(|lane| lane.lane_id == later_lane.lane_id),
        "a later lane without a forward message must not escape checkpoint replay bounds");
    assert!(bounded.replay.lanes.iter().all(|lane|
        lane.event_ledger_seq <= checkpoint.last_event_ledger_seq));
    assert_eq!(bounded.active_leases, vec![active.clone()]);
    assert_eq!(bounded.reclaimable_lease_ids, vec![expired.lease_id.clone()]);
    let orphan = bounded.recovery_events.iter().find(|event|
        event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
            && event.lease_id.as_deref() == Some(expired.lease_id.as_str()))
        .expect("expired current lease emits canonical orphan event");
    assert_eq!(orphan.lane_id.as_deref(), Some(later_lane.lane_id.as_str()));
    assert_eq!(orphan.parent_span_id.as_deref(), Some(later_lane.lane_span_id.as_str()),
        "orphan attribution must use the current canonical lane outside returned replay bounds");
    let after_orphan = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 64))
        .await.expect("receipts after first orphan reconciliation");
    assert_eq!(harness.recover(harness.store.test_recover_run_after_restart_at(&posture.run_id, now))
        .await.expect("repeat bounded recovery"), bounded);
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 64))
        .await.expect("receipts after idempotent recovery"), after_orphan);

    let mut forward = posture.message.clone();
    forward.message_id = "message-forward-after-checkpoint".into();
    forward.message_span_id = "span-message-forward-after-checkpoint".into();
    forward.idempotency_key = "idem-message-forward-after-checkpoint".into();
    forward.replay_order_key = "forward-after-checkpoint".into();
    let forward = harness.create_record(harness.store.record_message(forward))
        .await.expect("record later message with genuine canonical CRDT authority");
    assert!(forward.event_ledger_seq > checkpoint.last_event_ledger_seq);
    assert!(forward.crdt_authority_binding.is_some());
    let extended = harness.recover(harness.store.test_recover_run_after_restart_at(&posture.run_id, now))
        .await.expect("forward message legitimately extends recovery replay");
    assert_eq!(extended.replay.messages.len(), 2);
    assert!(extended.replay.messages.iter().any(|message| message == &first));
    assert!(extended.replay.messages.iter().any(|message| message == &forward));
    assert!(extended.replay.lanes.iter().any(|lane| lane == &later_lane));
    assert_eq!(extended.active_leases, vec![active]);
    assert_eq!(extended.reclaimable_lease_ids, vec![expired.lease_id.clone()]);
    assert_eq!(extended.recovery_events.iter().filter(|event|
        event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
            && event.lease_id.as_deref() == Some(expired.lease_id.as_str())).count(), 1);
    harness.cleanup().await;
}
fn ordinary_payload_case(
    posture: &SurrealAdmissibleCrdtPosture,
    label: &str,
) -> (NewModelLaneMessage, NewModelLaneContextBundleArtifactBinding) {
    let payload_json = json!({"text": format!("ordinary recovery payload {label}")});
    let bytes = serde_json::to_vec(&payload_json).expect("serialize one-key canonical payload");
    let hash = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
    let mut message = posture.message.clone();
    message.message_id = format!("message-ordinary-{label}");
    message.message_span_id = format!("span-ordinary-{label}");
    message.idempotency_key = format!("idem-ordinary-{label}");
    message.replay_order_key = format!("ordinary-{label}");
    message.kind = ModelLaneMessageKind::Status;
    message.authority = ModelLaneAuthority::Advisory;
    message.payload_ref = format!("artifact://recovery/ordinary/{label}");
    message.payload_sha256 = hash.clone();
    message.proposal_ref = None;
    message.crdt_update_ref = None;
    message.crdt_base_snapshot_ref = None;
    message.crdt_state_vector = None;
    message.crdt_proposal_ref = None;
    message.crdt_stale_base_ref = None;
    message.diagnostic_payload = json!({"case": label});
    let binding = NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-ordinary-{label}"),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: hash.clone(),
        content_hash: hash,
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!("artifact-manifest://recovery/ordinary/{label}"),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json,
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: message.work_packet_id.clone().expect("actual producer WP"),
        micro_task_id: message.micro_task_id.clone().expect("actual producer MT"),
        task_board_id: message.task_board_id.clone().expect("actual producer task board"),
        owner_session: message.owner_session.clone(),
        idempotency_key: format!("idem-artifact-binding-ordinary-{label}"),
        created_at_utc: message.created_at_utc.clone(),
        diagnostic_payload: json!({"message_id": message.message_id}),
    };
    (message, binding)
}

#[tokio::test]
async fn recovery_accepts_canonical_ordinary_payload_witness_with_empty_checkpoint_refs() {
    let harness = Harness::create("ordinary-payload-positive").await;
    let posture = harness.posture("ordinary-payload-positive").await;
    let (message, binding) = ordinary_payload_case(&posture, "positive");
    let binding = harness.create_record(harness.store.record_context_bundle_artifact_binding(binding))
        .await.expect("persist real payload bytes through canonical artifact producer");
    let message = harness.create_record(harness.store.record_message(message))
        .await.expect("persist ordinary message after canonical payload witness");
    assert!(message.crdt_authority_binding.is_none());
    assert!(binding.event_ledger_seq < message.event_ledger_seq);
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-ordinary-positive");
    checkpoint.open_payload_refs.clear();
    harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("canonical empty-ref checkpoint");
    let recovered = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("ordinary message with exact bounded payload witness recovers");
    assert_eq!(recovered.replay.messages, vec![message]);
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_rejects_late_payload_repair_until_new_checkpoint_covers_actual_witness() {
    let harness = Harness::create("ordinary-late-payload").await;
    let posture = harness.posture("ordinary-late-payload").await;
    let (message, binding) = ordinary_payload_case(&posture, "late");
    let message = harness.create_record(harness.store.record_message(message))
        .await.expect("ordinary source without payload witness is admitted");
    let mut original_checkpoint = checkpoint(&posture, &message, "checkpoint-before-payload");
    original_checkpoint.open_payload_refs.clear();
    harness.create_record(harness.store.record_recovery_checkpoint(original_checkpoint))
        .await.expect("persist checkpoint before payload witness");
    let binding = harness.create_record(harness.store.record_context_bundle_artifact_binding(binding))
        .await.expect("persist genuine payload witness after original checkpoint");
    assert!(binding.event_ledger_seq > message.event_ledger_seq);
    let receipts = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 64))
        .await.expect("canonical receipts before late-repair denial");
    let error = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect_err("late payload witness cannot retroactively satisfy checkpoint authority");
    assert!(error.to_string().contains("CX-MM-006"), "late witness code: {error}");
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 64))
        .await.expect("receipts after late-repair denial"), receipts);

    let mut repaired_checkpoint = checkpoint(&posture, &message, "checkpoint-after-payload");
    repaired_checkpoint.open_payload_refs.clear();
    repaired_checkpoint.last_event_ledger_seq = binding.event_ledger_seq;
    harness.create_record(harness.store.record_recovery_checkpoint(repaired_checkpoint))
        .await.expect("new checkpoint explicitly covers actual canonical payload witness");
    let recovered = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("new checkpoint admits matching ordinary payload witness");
    assert_eq!(recovered.checkpoint.checkpoint_id, "checkpoint-after-payload");
    assert_eq!(recovered.replay.messages, vec![message]);
    harness.cleanup().await;
}
