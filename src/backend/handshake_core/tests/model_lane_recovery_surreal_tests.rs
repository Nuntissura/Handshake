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
    ModelLaneRecoveryFailureKind, ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneResult, ModelLaneStatus, ModelLaneStore,
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
    capabilities: std::sync::Arc<CapabilityRegistry>,
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
        let registry = std::sync::Arc::new(CapabilityRegistry::new());
        let capabilities = registry.profile_by_id("Operator")
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
        let document = workspace_create.run_with_configured_registry(registry.clone(), async {
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
            channel_hash, document_id: document.rich_document_id, capabilities: registry, store }
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
            .await?.run_with_configured_registry(self.capabilities.clone(), future).await
    }

    async fn read<T>(&self, future: impl std::future::Future<Output = ModelLaneResult<T>>)
        -> ModelLaneResult<T> {
        self.operation("fs.read", ResourceKind::Workspace,
            self.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Read)
            .await?.run_with_configured_registry(self.capabilities.clone(), future).await
    }

    async fn recover<T>(&self, future: impl std::future::Future<Output = ModelLaneResult<T>>)
        -> ModelLaneResult<T> {
        self.operation("model_lane.recovery.execute", ResourceKind::Workspace,
            self.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Update)
            .await?.run_with_configured_registry(self.capabilities.clone(), future).await
    }

    async fn head(&self, run_id: &str) -> i64 {
        self.create_record(self.store.test_recovery_run_high_watermark(run_id))
            .await.expect("actual exact run-stream EventLedger head")
    }

    async fn navigation<T>(&self, future: impl std::future::Future<Output = ModelLaneResult<T>>)
        -> ModelLaneResult<T> {
        self.operation("model_lane.navigation.read", ResourceKind::Workspace,
            self.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Read)
            .await?.run_with_configured_registry(self.capabilities.clone(), future).await
    }

    async fn diagnostics<T>(&self, future: impl std::future::Future<Output = ModelLaneResult<T>>)
        -> ModelLaneResult<T> {
        self.operation("diagnostics.read", ResourceKind::Workspace,
            self.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Read)
            .await?.run_with_configured_registry(self.capabilities.clone(), future).await
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
    let status_input = mt_status(&posture);
    let stored_status = harness.create_record(harness
        .store
        .record_mt_runtime_status(status_input.clone()))
        .await
        .expect("record MT status");

    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-replay");
    checkpoint.last_event_ledger_seq = harness.head(&posture.run_id).await;
    assert_eq!(checkpoint.last_event_ledger_seq, stored_status.event_ledger_seq,
        "all intended positive inputs precede the true checkpoint cutoff");
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
    assert_eq!(before.mt_runtime_statuses[0].inner, status_input);
    assert_eq!(before.mt_runtime_statuses[0].claimed_by_ref, stored_status.claimed_by_ref);
    assert_eq!(before.mt_runtime_statuses[0].proof_status_ref, stored_status.proof_status_ref);
    assert_eq!(before.mt_runtime_statuses[0].hbr_status_ref, stored_status.hbr_status_ref);
    assert_eq!(before.mt_runtime_statuses[0].last_runtime_status_ref, stored_status.last_runtime_status_ref);
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
        let recorded_checkpoint = harness.create_record(harness
            .store
            .record_recovery_checkpoint(checkpoint(&posture, &message, &checkpoint_id)))
            .await
            .expect("record canonical checkpoint");
        let observation = if corruption == ModelLaneAuthorityTestCorruption::ProjectionScope {
            Some(harness.store.test_capture_projection_scope_observation(
                &harness.isolated, &posture.run_id, &checkpoint_id,
            ).await.expect("bind admitted physical checkpoint and immutable receipt before corruption"))
        } else {
            None
        };
        harness.create_record(harness
            .store
            .test_corrupt_scoped_authority("recovery_checkpoint", &checkpoint_id, corruption))
            .await
            .expect("apply enumerated checkpoint corruption");
        let post_corruption = if let Some((token, baseline)) = observation.as_ref() {
            let observed = harness.store.test_observe_projection_scope_corruption(
                &harness.isolated, token,
            ).await.expect("observe fixed physical checkpoint after actor-only corruption");
            assert_eq!(observed["projection_actor_principal_id"], "counterfactual-projection-principal");
            let mut normalized = observed.clone();
            normalized["projection_actor_principal_id"] = baseline["projection_actor_principal_id"].clone();
            assert_eq!(&normalized, baseline,
                "only the fixed projection actor changes; raw11, physical key, attribution and receipt pointer remain identical");
            Some(observed)
        } else {
            None
        };
        let head_after_corruption = harness.head(&posture.run_id).await;
        if corruption == ModelLaneAuthorityTestCorruption::ProjectionScope {
            assert_eq!(head_after_corruption, recorded_checkpoint.event_ledger_seq,
                "the exact-run checkpoint receipt survives the projection-scope omission");
        }
        let direct = harness.recover(harness
            .store
            .recover_run_after_restart(&posture.run_id))
            .await;
        assert!(direct.is_err());
        if let (Some((token, _)), Some(expected)) = (observation.as_ref(), post_corruption.as_ref()) {
            assert_eq!(&harness.store.test_observe_projection_scope_corruption(
                &harness.isolated, token,
            ).await.expect("observe same physical checkpoint immediately after direct denial"), expected);
        }
        let boot = harness.recover(harness
            .store
            .recover_restartable_runs_at_boot())
            .await;
        assert!(boot.is_err());
        if let (Some((token, _)), Some(expected)) = (observation.as_ref(), post_corruption.as_ref()) {
            assert_eq!(&harness.store.test_observe_projection_scope_corruption(
                &harness.isolated, token,
            ).await.expect("observe same physical checkpoint immediately after boot denial"), expected);
        }
        if corruption == ModelLaneAuthorityTestCorruption::ProjectionScope {
            let direct = direct.expect_err("projection counterpart omission must deny").to_string();
            let boot = boot.expect_err("boot counterpart omission must deny").to_string();
            assert_eq!(direct, boot);
            assert!(!direct.contains(checkpoint_id.as_str()));
            assert!(!direct.contains(posture.run_id.as_str()));
        }
        assert_eq!(harness.head(&posture.run_id).await, head_after_corruption,
            "direct and boot denial cannot append ledger receipts after the actual corruption");
        drop(observation);
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

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let first_operation = harness.operation("fs.write", ResourceKind::Workspace,
        harness.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Create)
        .await.expect("authorize concurrent recovery child");
    let second_operation = first_operation.clone();
    let first_registry = harness.capabilities.clone();
    let second_registry = harness.capabilities.clone();
    let first_store = harness.store.clone();
    let first_barrier = barrier.clone();
    let first_event = recovery_event(&posture, &message, "event-child-a");
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        first_operation.run_with_configured_registry(first_registry, first_store.record_recovery_event(first_event)).await
    });
    let second_store = harness.store.clone();
    let second_barrier = barrier.clone();
    let second_event = recovery_event(&posture, &message, "event-child-b");
    let second = tokio::spawn(async move {
        second_barrier.wait().await;
        second_operation.run_with_configured_registry(second_registry, second_store.record_recovery_event(second_event)).await
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
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-concurrent-children");
    checkpoint.last_event_ledger_seq = harness.head(&posture.run_id).await;
    assert_eq!(checkpoint.last_event_ledger_seq, first.event_ledger_seq.max(second.event_ledger_seq));
    harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("checkpoint covers both actual committed child events");

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
        replay_order_seq: 1,
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

async fn assert_recovery_failure_without_mutation(
    harness: &Harness, run_id: &str, failure: ModelLaneRecoveryFailureKind,
) {
    let before = harness.read(harness.store.test_scoped_authority_receipts(run_id, 256))
        .await.expect("canonical receipts before failed recovery");
    let error = harness.recover(harness.store.recover_run_after_restart(run_id))
        .await.expect_err("recovery must fail closed");
    assert!(error.to_string().contains(failure.code()), "expected {}, got {error}", failure.code());
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(run_id, 256))
        .await.expect("canonical receipts after failed recovery"), before);
}

#[tokio::test]
async fn recovery_checkpoint_rejects_stale_true_head_and_event_idempotency_conflict_without_mutation() {
    let harness = Harness::create("watermark-idempotency").await;
    let posture = harness.posture("watermark-idempotency").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical source message");
    let event = recovery_event(&posture, &message, "event-watermark");
    let stored = harness.create_record(harness.store.record_recovery_event(event.clone()))
        .await.expect("canonical ordered event");
    let mut conflict = event;
    conflict.recovery_event_id.push_str("-conflicting");
    let before = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("canonical conflict baseline");
    let error = harness.create_record(harness.store.record_recovery_event(conflict))
        .await.expect_err("divergent same-idempotency event must fail");
    assert!(error.to_string().contains("idempotency_key"));
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("unchanged conflict receipts"), before);
    assert!(harness.head(&posture.run_id).await > message.event_ledger_seq);
    let error = harness.create_record(harness.store.record_recovery_checkpoint(
        checkpoint(&posture, &message, "checkpoint-stale-watermark")))
        .await.expect_err("older valid in-stream sequence is not an exact checkpoint head");
    assert!(error.to_string().contains(ModelLaneRecoveryFailureKind::CorruptCheckpoint.code()));
    assert!(error.to_string().contains("exact pre-write stream high-watermark"));
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("stale watermark must not create row or ledger receipt"), before);
    let mut valid = checkpoint(&posture, &message, "checkpoint-exact-watermark");
    valid.last_event_ledger_seq = harness.head(&posture.run_id).await;
    assert_eq!(valid.last_event_ledger_seq, stored.event_ledger_seq);
    harness.create_record(harness.store.record_recovery_checkpoint(valid))
        .await.expect("actual exact-head positive after denials");
    let recovered = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("canonical positive survives conflicting/stale writes");
    assert_eq!(recovered.recovery_events, vec![stored]);
    harness.cleanup().await;
}

#[tokio::test]
async fn checkpoint_producer_head_denies_read_foreign_stale_and_revoked_authority() {
    let harness = Harness::create("producer-head-authority").await;
    let posture = harness.posture("producer-head-authority").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical checkpoint source");
    let baseline = harness.head(&posture.run_id).await;
    assert_eq!(baseline, message.event_ledger_seq);
    assert!(harness.read(harness.store.test_recovery_run_high_watermark(&posture.run_id))
        .await.is_err(), "Read authority cannot inspect producer head");
    assert!(harness.recover(harness.store.test_recovery_run_high_watermark(&posture.run_id))
        .await.is_err(), "recovery Update cannot become checkpoint Create");
    for scope in one_field_recovery_scopes(&harness.scope) {
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        lifecycle.register_active(ExactResourceScopeAttribution::try_from_resource_scope(&scope)
            .expect("complete foreign tuple")).expect("component lifecycle");
        let foreign = ModelLaneStore::new_scoped_with_lifecycle(harness.storage.clone(), scope, lifecycle);
        assert!(harness.create_record(foreign.test_recovery_run_high_watermark(&posture.run_id))
            .await.is_err(), "each foreign field denies producer observation");
    }
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope)
        .expect("actual exact tuple");
    for revoked in [false, true] {
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        lifecycle.register_active(exact.clone()).expect("component lifecycle");
        let restricted = ModelLaneStore::new_scoped_with_lifecycle(
            harness.storage.clone(), harness.scope.clone(), lifecycle.clone());
        if revoked {
            lifecycle.revoke(&exact).expect("revoke exact tuple");
        } else {
            lifecycle.mark_stale(&exact).expect("stale exact tuple");
        }
        assert!(harness.create_record(restricted.test_recovery_run_high_watermark(&posture.run_id))
            .await.is_err(), "inactive lifecycle denies producer observation");
    }
    assert_eq!(harness.head(&posture.run_id).await, baseline);
    let foreign = harness.posture("producer-head-other-run").await;
    harness.create_record(harness.store.record_message(foreign.message.clone()))
        .await.expect("genuine other-run stream commit");
    assert_eq!(harness.head(&posture.run_id).await, baseline,
        "another run and stream do not change this producer head");
    let lease = harness.create_record(harness.store.record_lane_lease(
        lease(&posture, "lease-producer-head", "2099-01-01T00:00:00Z")))
        .await.expect("non-message lease commit");
    assert_eq!(harness.head(&posture.run_id).await, lease.event_ledger_seq);
    let diagnostic = harness.create_record(harness.store.record_diagnostic_tier_status(
        diagnostic(&posture, ModelLaneDiagnosticTier::FlightRecorder)))
        .await.expect("non-message diagnostic commit");
    assert_eq!(harness.head(&posture.run_id).await, diagnostic.event_ledger_seq);
    let status = harness.create_record(harness.store.record_mt_runtime_status(mt_status(&posture)))
        .await.expect("non-message status commit");
    assert_eq!(harness.head(&posture.run_id).await, status.event_ledger_seq);
    let before = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("canonical pre-denial receipts");
    let mut stale = checkpoint(&posture, &message, "checkpoint-producer-stale-head");
    stale.last_event_ledger_seq = diagnostic.event_ledger_seq;
    assert!(harness.create_record(harness.store.record_recovery_checkpoint(stale)).await.is_err());
    assert_eq!(harness.head(&posture.run_id).await, status.event_ledger_seq,
        "stale checkpoint cannot append a ledger event");
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("canonical post-denial receipts"), before);
    let mut input = checkpoint(&posture, &message, "checkpoint-producer-exact-head");
    input.last_event_ledger_seq = harness.head(&posture.run_id).await;
    let captured = harness.create_record(harness.store.record_recovery_checkpoint(input))
        .await.expect("authenticated exact-head checkpoint capture");
    assert_eq!(harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("separate authenticated recovery Update").checkpoint, captured);
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_rejects_missing_open_payload_and_late_repair_at_the_original_checkpoint() {
    let harness = Harness::create("missing-open-ref").await;
    let posture = harness.posture("missing-open-ref").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical CRDT source");
    let (_, binding) = ordinary_payload_case(&posture, "open-ref-late");
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-open-ref");
    checkpoint.open_payload_refs.push(binding.artifact_ref.clone());
    harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("checkpoint records its unresolved open payload ref");
    assert_recovery_failure_without_mutation(&harness, &posture.run_id,
        ModelLaneRecoveryFailureKind::MissingPayloadAuthority).await;
    harness.create_record(harness.store.record_context_bundle_artifact_binding(binding))
        .await.expect("real payload bytes bound only after original checkpoint");
    assert_recovery_failure_without_mutation(&harness, &posture.run_id,
        ModelLaneRecoveryFailureKind::MissingPayloadAuthority).await;
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_rejects_stale_crdt_admission_and_in_bound_recovery_event() {
    let harness = Harness::create("stale-crdt").await;
    let posture = harness.posture("stale-crdt").await;
    let mut stale = posture.message.clone();
    stale.crdt_update_ref = None;
    stale.crdt_stale_base_ref = Some("crdt-stale-base://recovery/admission".into());
    let before = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("before stale admission");
    let error = harness.create_record(harness.store.record_message(stale))
        .await.expect_err("partial/stale CRDT authority must not be admitted");
    assert!(error.to_string().contains("partial CRDT metadata cannot be admitted without crdt_update_ref"));
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("stale admission leaves no authority receipt"), before);
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("real CRDT positive after denied admission");
    let mut event = recovery_event(&posture, &message, "event-stale-crdt");
    event.event_kind = ModelLaneRecoveryEventKind::CrdtUpdateObserved;
    event.crdt_stale_base_ref = Some("crdt-stale-base://recovery/event".into());
    let event = harness.create_record(harness.store.record_recovery_event(event))
        .await.expect("canonical event records observed stale-base failure");
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-stale-crdt");
    checkpoint.last_event_ledger_seq = event.event_ledger_seq;
    assert_eq!(checkpoint.last_event_ledger_seq, harness.head(&posture.run_id).await);
    harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("checkpoint includes actual stale-base event");
    assert_recovery_failure_without_mutation(&harness, &posture.run_id,
        ModelLaneRecoveryFailureKind::StaleCrdtBase).await;
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_diagnostic_posture_rejects_missing_and_fr_only_but_preserves_reason_states() {
    let harness = Harness::create("diagnostic-reasons").await;
    let posture = harness.posture("diagnostic-reasons").await;
    let error = harness.read(harness.store.validate_diagnostic_tier_posture(&posture.run_id, "HBR-INT-009"))
        .await.expect_err("absence of FlightRecorder/EventLedger must fail");
    assert!(error.to_string().contains("FlightRecorder"));
    harness.create_record(harness.store.record_diagnostic_tier_status(
        diagnostic(&posture, ModelLaneDiagnosticTier::FlightRecorder)))
        .await.expect("real canonical first tier");
    let error = harness.read(harness.store.validate_diagnostic_tier_posture(&posture.run_id, "HBR-INT-009"))
        .await.expect_err("FlightRecorder-only posture must fail");
    assert!(error.to_string().contains("FlightRecorder-only"));
    let mut internal = diagnostic(&posture, ModelLaneDiagnosticTier::InternalDiagnostics);
    internal.state = ModelLaneDiagnosticTierState::DeferredWithReason;
    internal.reason = "original integration proof remains pending".into();
    let mut invalid = internal.clone();
    invalid.follow_up_ref = None;
    let before = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("before malformed deferred posture");
    assert!(harness.create_record(harness.store.record_diagnostic_tier_status(invalid)).await.is_err());
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("malformed reason posture is nonmutating"), before);
    harness.create_record(harness.store.record_diagnostic_tier_status(internal.clone()))
        .await.expect("honest deferred tier with followup");
    let mut palmistry = diagnostic(&posture, ModelLaneDiagnosticTier::Palmistry);
    palmistry.state = ModelLaneDiagnosticTierState::NotApplicableWithReason;
    palmistry.reason = "this isolated storage component does not launch a native watcher".into();
    harness.create_record(harness.store.record_diagnostic_tier_status(palmistry.clone()))
        .await.expect("honest not-applicable tier with reason");
    let observed = harness.read(harness.store.validate_diagnostic_tier_posture(&posture.run_id, "HBR-INT-009"))
        .await.expect("all three real reason states are represented");
    assert_eq!(observed.run_id, posture.run_id);
    assert_eq!(observed.behavior_id, "HBR-INT-009");
    assert_eq!(observed.tiers.len(), 3);
    assert!(observed.tiers.iter().any(|tier| tier.tier == internal.tier
        && tier.state == internal.state && tier.reason == internal.reason
        && tier.follow_up_ref == internal.follow_up_ref));
    assert!(observed.tiers.iter().any(|tier| tier.tier == palmistry.tier
        && tier.state == palmistry.state && tier.reason == palmistry.reason));
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_reconciles_all_current_leases_without_advancing_checkpoint_or_status_bound() {
    let harness = Harness::create("current-lease-matrix").await;
    let posture = harness.posture("current-lease-matrix").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical source");
    let checkpoint = harness.create_record(harness.store.record_recovery_checkpoint(
        checkpoint(&posture, &message, "checkpoint-current-leases")))
        .await.expect("checkpoint before current adjuncts");
    for (id, expiry, state) in [
        ("current-active", "2099-01-01T00:00:00Z", ModelLaneLeaseState::Active),
        ("current-expired-a", "2020-01-01T00:00:00Z", ModelLaneLeaseState::Active),
        ("current-expired-b", "2020-01-02T00:00:00Z", ModelLaneLeaseState::Active),
        ("current-cancelled", "2020-01-01T00:00:00Z", ModelLaneLeaseState::Cancelled),
        ("current-released", "2020-01-01T00:00:00Z", ModelLaneLeaseState::Released),
        ("current-reclaimed", "2020-01-01T00:00:00Z", ModelLaneLeaseState::Reclaimed),
    ] {
        let mut input = lease(&posture, id, expiry);
        input.state = state;
        harness.create_record(harness.store.record_lane_lease(input)).await.expect("canonical current lease");
    }
    harness.create_record(harness.store.record_mt_runtime_status(mt_status(&posture)))
        .await.expect("status created beyond checkpoint must remain outside bounded output");
    let first = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("all current leases reconciled despite old checkpoint");
    assert_eq!(first.checkpoint, checkpoint);
    assert_eq!(first.active_leases.iter().map(|v| v.lease_id.as_str()).collect::<Vec<_>>(), vec!["current-active"]);
    assert_eq!(first.reclaimable_lease_ids, vec!["current-expired-a", "current-expired-b"]);
    assert!(first.mt_runtime_statuses.is_empty());
    assert_eq!(first.recovery_events.len(), 2);
    for (index, event) in first.recovery_events.iter().enumerate() {
        assert_eq!(event.replay_order_seq, index as i64 + 1);
        assert_eq!(event.event_kind, ModelLaneRecoveryEventKind::OrphanDetected);
        assert_eq!(event.session_id.as_deref(), Some(posture.session_id.as_str()));
        assert_eq!(event.model_session_id.as_deref(), Some(posture.model_session_id.as_str()));
        assert_eq!(event.failure_kind, Some(ModelLaneRecoveryFailureKind::OrphanedSubagent));
        assert_eq!(event.error_code.as_deref(), Some(ModelLaneRecoveryFailureKind::OrphanedSubagent.code()));
    }
    let receipts = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("durable orphan receipts");
    assert_eq!(receipts.iter().filter(|r| r.record_kind == "recovery_event").count(), 2);
    assert_eq!(harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("repeat recovery reuses canonical decisions"), first);
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("repeat recovery receipts"), receipts);
    harness.create_record(harness.store.record_lane_lease(
        lease(&posture, "current-expired-later", "2020-01-03T00:00:00Z")))
        .await.expect("later current expired lease");
    let later = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("later lease gets exactly one new orphan");
    assert_eq!(later.checkpoint, checkpoint);
    assert_eq!(later.reclaimable_lease_ids, vec!["current-expired-a", "current-expired-b", "current-expired-later"]);
    assert!(later.mt_runtime_statuses.is_empty());
    assert_eq!(later.recovery_events.iter().map(|v| v.replay_order_seq).collect::<Vec<_>>(), vec![1, 2, 3]);
    let later_receipts = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("later durable decisions");
    assert_eq!(later_receipts.iter().filter(|r| r.record_kind == "recovery_event").count(), 3);
    assert_eq!(harness.recover(harness.store.recover_restartable_runs_at_boot())
        .await.expect("boot resumes actual scoped checkpoint"), vec![later]);
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("boot recovery idempotency receipts"), later_receipts);
    harness.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn eight_concurrent_mt_status_children_serialize_and_recover_with_exact_references() {
    let harness = Harness::create("eight-status-writers").await;
    let posture = harness.posture("eight-status-writers").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical source");
    let operation = harness.operation("fs.write", ResourceKind::Workspace,
        harness.scope.workspace.as_ref().expect("workspace").as_str(), ResourceAction::Create)
        .await.expect("genuine child Create authority");
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(8));
    let mut writes = Vec::new();
    let mut inputs = Vec::new();
    for index in 0..8 {
        let mut input = mt_status(&posture);
        input.mt_status_id = format!("mt-status-concurrent-{index}");
        input.idempotency_key = format!("idem-mt-status-concurrent-{index}");
        inputs.push(input.clone());
        let (store, operation, registry, barrier) =
            (harness.store.clone(), operation.clone(), harness.capabilities.clone(), barrier.clone());
        writes.push(tokio::spawn(async move {
            barrier.wait().await;
            operation.run_with_configured_registry(registry, store.record_mt_runtime_status(input)).await
        }));
    }
    let mut child_errors = Vec::new();
    for (index, mut write) in writes.into_iter().enumerate() {
        let observed = match tokio::time::timeout(std::time::Duration::from_secs(15), &mut write).await {
            Ok(result) => result,
            Err(_) => {
                child_errors.push(format!("status child {index} exceeded its join deadline; custody retained"));
                eprintln!("status child observation failed before storage drain/retained join: {child_errors:?}");
                if let Err(error) = harness.storage.shutdown().await {
                    child_errors.push(format!("owned storage shutdown remains unresolved: {error}"));
                }
                eprintln!("status child diagnostics before unresolved retained join: {child_errors:?}");
                // A timed-out observation is not completion. Retain the handle
                // through the real storage cancellation/drain and actual join.
                (&mut write).await
            }
        };
        match observed {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                let message = format!("status child {index}: {error}");
                eprintln!("{message}");
                child_errors.push(message);
            }
            Err(error) => {
                let message = format!("status child {index} join: {error}");
                eprintln!("{message}");
                child_errors.push(message);
            }
        }
    }
    if !child_errors.is_empty() {
        eprintln!("child failures before potentially unresolved storage close/cleanup: {child_errors:?}");
        while !harness.storage.is_closed().await {
            if let Err(error) = harness.storage.shutdown().await {
                let message = format!("owned storage close not yet acknowledged: {error}");
                if !child_errors.contains(&message) {
                    eprintln!("{message}; original child failures: {child_errors:?}");
                    child_errors.push(message);
                }
            }
        }
        eprintln!("all child outcomes observed; diagnostics retained before fallible fixture cleanup: {child_errors:?}");
        harness.cleanup().await;
        panic!("all status children settled before cleanup: {child_errors:?}");
    }
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-eight-statuses");
    checkpoint.last_event_ledger_seq = harness.head(&posture.run_id).await;
    harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("actual head covers all committed child statuses");
    let recovered = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("all status references recover without chat or UI state");
    assert_eq!(recovered.mt_runtime_statuses.len(), 8);
    for input in inputs {
        assert!(recovered.mt_runtime_statuses.iter().any(|record| record.inner == input));
    }
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_children_and_ledger_receipts_preserve_all_five_scope_fields_and_navigation() {
    let harness = Harness::create("five-child-scopes").await;
    let posture = harness.posture("five-child-scopes").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical source");
    let event = harness.create_record(harness.store.record_recovery_event(
        recovery_event(&posture, &message, "event-five-child-scopes")))
        .await.expect("canonical recovery event");
    harness.create_record(harness.store.record_lane_lease(
        lease(&posture, "lease-five-child-scopes", "2099-01-01T00:00:00Z")))
        .await.expect("canonical lease");
    for tier in [ModelLaneDiagnosticTier::FlightRecorder, ModelLaneDiagnosticTier::InternalDiagnostics, ModelLaneDiagnosticTier::Palmistry] {
        harness.create_record(harness.store.record_diagnostic_tier_status(diagnostic(&posture, tier)))
            .await.expect("canonical diagnostic child");
    }
    let status = harness.create_record(harness.store.record_mt_runtime_status(mt_status(&posture)))
        .await.expect("canonical status child");
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-five-child-scopes");
    checkpoint.last_event_ledger_seq = harness.head(&posture.run_id).await;
    assert_eq!(checkpoint.last_event_ledger_seq, status.event_ledger_seq);
    harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("canonical checkpoint child");
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope).expect("actual exact scope");
    let receipts = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("joined physical child and canonical EventLedger authority");
    for kind in ["recovery_checkpoint", "recovery_event", "lease", "diagnostic_tier", "mt_runtime_status"] {
        let matching = receipts.iter().filter(|r| r.record_kind == kind).collect::<Vec<_>>();
        assert!(!matching.is_empty(), "{kind} exact-scope proof must be nonzero");
        for row in matching {
            assert_eq!(row.owner_account_id, exact.owner_account_id.as_uuid().to_string());
            assert_eq!(row.actor_principal_id, exact.actor_principal_id.as_uuid().to_string());
            assert_eq!(row.authenticated_session_id, exact.authenticated_session_id.as_uuid().to_string());
            assert_eq!(row.access_space_id, exact.access_space_id.as_uuid().to_string());
            assert_eq!(row.workspace_id, exact.workspace_id.as_str());
        }
    }
    let replay = harness.read(harness.store.replay_run(&posture.run_id)).await.expect("owner replay");
    assert_eq!(replay.run.run_id, posture.run_id);
    let projection = harness.diagnostics(harness.store.diagnostics_projection(&posture.run_id))
        .await.expect("separately authorized diagnostics positive");
    assert_eq!(projection.run.run_id, posture.run_id);
    assert_eq!(projection.mt_runtime_statuses.len(), 1);
    let navigation = harness.navigation(harness.store.navigation_by_recovery(&posture.run_id))
        .await.expect("separately authorized navigation positive");
    assert_eq!(navigation.recovery_events.len(), 1);
    assert_eq!(navigation.recovery_events[0].recovery_event_id, event.recovery_event_id);
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("passive diagnostics/navigation do not mutate"), receipts);
    for (index, foreign_scope) in one_field_recovery_scopes(&harness.scope).into_iter().enumerate() {
        let foreign_exact = ExactResourceScopeAttribution::try_from_resource_scope(&foreign_scope).expect("complete counterfactual tuple");
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        lifecycle.register_active(foreign_exact).expect("isolated component lifecycle only");
        let foreign = ModelLaneStore::new_scoped_with_lifecycle(harness.storage.clone(), foreign_scope, lifecycle);
        let attempted = recovery_event(&posture, &message, &format!("event-foreign-attach-{index}"));
        assert!(harness.create_record(foreign.record_recovery_event(attempted)).await.is_err());
        assert!(harness.read(foreign.replay_run(&posture.run_id)).await.is_err());
        assert!(harness.diagnostics(foreign.diagnostics_projection(&posture.run_id)).await.is_err());
        assert!(harness.navigation(foreign.navigation_by_recovery(&posture.run_id)).await.is_err());
        assert!(harness.read(foreign.test_scoped_authority_receipts(&posture.run_id, 256))
            .await.expect("foreign component receipt scan").is_empty());
    }
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256))
        .await.expect("foreign child attach/read attempts do not mutate"), receipts);
    harness.recover(harness.store.recover_run_after_restart(&posture.run_id)).await.expect("owner still recovers after denials");
    harness.create_record(harness.store.test_corrupt_scoped_authority("recovery_checkpoint", "checkpoint-five-child-scopes",
        ModelLaneAuthorityTestCorruption::ProjectionEventSequence)).await.expect("exact owned corruption counterfactual");
    for foreign_scope in one_field_recovery_scopes(&harness.scope) {
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&foreign_scope).expect("complete counterfactual");
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        lifecycle.register_active(exact).expect("isolated component lifecycle");
        let foreign = ModelLaneStore::new_scoped_with_lifecycle(harness.storage.clone(), foreign_scope, lifecycle);
        let error = harness.recover(foreign.recover_run_after_restart(&posture.run_id)).await.expect_err("foreign corruption remains undiscoverable");
        assert!(matches!(error, handshake_core::swarm_orchestration::model_lane::ModelLaneError::NotFound(_)
            | handshake_core::swarm_orchestration::model_lane::ModelLaneError::ScopeDenied(_)
            | handshake_core::swarm_orchestration::model_lane::ModelLaneError::AuthorityDenied(_)),
            "authority must deny before foreign integrity inspection");
    }

    harness.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn recovery_serializes_public_append_and_two_recoveries_with_current_orphans_and_later_boot() {
    let harness = Harness::create("mixed-recovery-race").await;
    let posture = harness.posture("mixed-recovery-race").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("canonical source message");
    let baseline = harness.create_record(harness.store.record_recovery_event(
        recovery_event(&posture, &message, "event-before-mixed-race")))
        .await.expect("canonical in-bound recovery event");
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-mixed-race");
    checkpoint.last_event_ledger_seq = harness.head(&posture.run_id).await;
    assert_eq!(checkpoint.last_event_ledger_seq, baseline.event_ledger_seq);
    let checkpoint = harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("checkpoint covers baseline before current lane");
    let replay = harness.read(harness.store.replay_run(&posture.run_id)).await.expect("canonical lane source");
    let mut lane = replay.lanes.iter().find(|lane| lane.lane_id == posture.lane_id)
        .expect("actual canonical source lane").inner.clone();
    lane.lane_id = "lane-current-mixed-race".into();
    lane.lane_span_id = "span-current-mixed-race".into();
    let lane = harness.create_record(harness.store.record_lane(lane)).await.expect("canonical current lane");
    for (id, expiry) in [("race-active", "2099-01-01T00:00:00Z"),
        ("race-expired-a", "2020-01-01T00:00:00Z"), ("race-expired-b", "2020-01-02T00:00:00Z")] {
        let mut input = lease(&posture, id, expiry);
        input.lane_id = Some(lane.lane_id.clone());
        input.scope_ref = format!("model-lane://{}/{}", posture.run_id, lane.lane_id);
        harness.create_record(harness.store.record_lane_lease(input)).await.expect("current canonical lease");
    }
    let workspace = harness.scope.workspace.as_ref().expect("workspace").as_str();
    let append = harness.operation("fs.write", ResourceKind::Workspace, workspace, ResourceAction::Create)
        .await.expect("genuine public append authority");
    let recover_a = harness.operation("model_lane.recovery.execute", ResourceKind::Workspace, workspace, ResourceAction::Update)
        .await.expect("genuine recovery authority");
    let recover_b = recover_a.clone();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let ordinary_input = recovery_event(&posture, &message, "event-during-mixed-race");
    let (store, registry, gate) = (harness.store.clone(), harness.capabilities.clone(), barrier.clone());
    let mut ordinary = tokio::spawn(async move {
        gate.wait().await;
        append.run_with_configured_registry(registry, store.record_recovery_event(ordinary_input)).await
    });
    let (store, registry, gate, run_id) = (harness.store.clone(), harness.capabilities.clone(), barrier.clone(), posture.run_id.clone());
    let mut first = tokio::spawn(async move {
        gate.wait().await;
        recover_a.run_with_configured_registry(registry, store.recover_run_after_restart(&run_id)).await
    });
    let (store, registry, gate, run_id) = (harness.store.clone(), harness.capabilities.clone(), barrier.clone(), posture.run_id.clone());
    let mut second = tokio::spawn(async move {
        gate.wait().await;
        recover_b.run_with_configured_registry(registry, store.recover_run_after_restart(&run_id)).await
    });
    let (mut ordinary_result, mut first_result, mut second_result) = (None, None, None);
    let mut child_errors = Vec::new();
    if tokio::time::timeout(std::time::Duration::from_secs(15), async {
        tokio::join!(
            async {
                let result = (&mut ordinary).await;
                match &result {
                    Ok(Err(error)) => eprintln!("append failed while other child custody remains held: {error}"),
                    Err(error) => eprintln!("append join failed while other child custody remains held: {error}"),
                    Ok(Ok(_)) => {}
                }
                ordinary_result = Some(result);
            },
            async {
                let result = (&mut first).await;
                match &result {
                    Ok(Err(error)) => eprintln!("first recovery failed while other child custody remains held: {error}"),
                    Err(error) => eprintln!("first recovery join failed while other child custody remains held: {error}"),
                    Ok(Ok(_)) => {}
                }
                first_result = Some(result);
            },
            async {
                let result = (&mut second).await;
                match &result {
                    Ok(Err(error)) => eprintln!("second recovery failed while other child custody remains held: {error}"),
                    Err(error) => eprintln!("second recovery join failed while other child custody remains held: {error}"),
                    Ok(Ok(_)) => {}
                }
                second_result = Some(result);
            },
        );
    }).await.is_err() {
        child_errors.push("mixed recovery join deadline exceeded; all handles and observed results retained".to_owned());
        eprintln!("mixed recovery observation failed before storage drain: {child_errors:?}");
        if let Err(error) = harness.storage.shutdown().await {
            child_errors.push(format!("owned storage shutdown remains unresolved: {error}"));
        }
    }
    if !child_errors.is_empty() {
        eprintln!("mixed recovery diagnostics before unresolved retained joins: {child_errors:?}");
    }
    // Results live outside the timed future, so completed handles are never
    // polled twice and unresolved handles are not detached or called settled.
    if ordinary_result.is_none() {
        let result = (&mut ordinary).await;
        match &result {
            Ok(Err(error)) => eprintln!("append failed before remaining retained joins: {error}"),
            Err(error) => eprintln!("append join failed before remaining retained joins: {error}"),
            Ok(Ok(_)) => {}
        }
        ordinary_result = Some(result);
    }
    if first_result.is_none() {
        let result = (&mut first).await;
        match &result {
            Ok(Err(error)) => eprintln!("first recovery failed before remaining retained joins: {error}"),
            Err(error) => eprintln!("first recovery join failed before remaining retained joins: {error}"),
            Ok(Ok(_)) => {}
        }
        first_result = Some(result);
    }
    if second_result.is_none() {
        let result = (&mut second).await;
        match &result {
            Ok(Err(error)) => eprintln!("second recovery failed before remaining retained joins: {error}"),
            Err(error) => eprintln!("second recovery join failed before remaining retained joins: {error}"),
            Ok(Ok(_)) => {}
        }
        second_result = Some(result);
    }
    let ordinary = match ordinary_result {
        Some(Ok(Ok(record))) => Some(record),
        Some(Ok(Err(error))) => { child_errors.push(format!("append failed: {error}")); None }
        Some(Err(error)) => { child_errors.push(format!("append join failed: {error}")); None }
        None => unreachable!("retained append join was observed"),
    };
    let mut recovered_runs = Vec::new();
    for (name, result) in [("first", first_result), ("second", second_result)] {
        match result {
            Some(Ok(Ok(record))) => recovered_runs.push(record),
            Some(Ok(Err(error))) => child_errors.push(format!("{name} recovery failed: {error}")),
            Some(Err(error)) => child_errors.push(format!("{name} recovery join failed: {error}")),
            None => unreachable!("retained recovery join was observed"),
        }
    }
    if !child_errors.is_empty() {
        eprintln!("child failures before potentially unresolved storage close/cleanup: {child_errors:?}");
        while !harness.storage.is_closed().await {
            if let Err(error) = harness.storage.shutdown().await {
                let message = format!("owned storage close not yet acknowledged: {error}");
                if !child_errors.contains(&message) {
                    eprintln!("{message}; original child failures: {child_errors:?}");
                    child_errors.push(message);
                }
            }
        }
        eprintln!("all child outcomes observed; diagnostics retained before fallible fixture cleanup: {child_errors:?}");
        harness.cleanup().await;
        panic!("all mixed recovery children settled before cleanup: {child_errors:?}");
    }
    let ordinary = ordinary.expect("successful append observed after every child settled");
    assert!((2..=4).contains(&ordinary.replay_order_seq));
    assert_eq!(recovered_runs.len(), 2);
    for recovered in recovered_runs {
        assert_eq!(recovered.checkpoint, checkpoint);
        assert_eq!(recovered.active_leases.iter().map(|v| v.lease_id.as_str()).collect::<Vec<_>>(), vec!["race-active"]);
        assert_eq!(recovered.reclaimable_lease_ids, vec!["race-expired-a", "race-expired-b"]);
        assert!(!recovered.replay.lanes.iter().any(|v| v.lane_id == lane.lane_id));
        assert!(!recovered.recovery_events.iter().any(|v| v.recovery_event_id == ordinary.recovery_event_id));
        for orphan in recovered.recovery_events.iter().filter(|v| v.event_kind == ModelLaneRecoveryEventKind::OrphanDetected) {
            assert_eq!(orphan.session_id.as_deref(), Some(lane.session_id.as_str()));
            assert_eq!(orphan.model_session_id.as_deref(), Some(lane.model_session_id.as_str()));
            assert_eq!(orphan.parent_span_id.as_deref(), Some(lane.lane_span_id.as_str()));
        }
    }
    let canonical = harness.navigation(harness.store.navigation_by_recovery(&posture.run_id))
        .await.expect("full canonical tail read under separate original navigation grant");
    assert_eq!(canonical.recovery_events.iter().map(|v| v.replay_order_seq).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    assert_eq!(canonical.recovery_events.iter().filter(|v| v.event_kind == ModelLaneRecoveryEventKind::OrphanDetected).count(), 2);
    let mut later = lease(&posture, "race-expired-later", "2020-01-03T00:00:00Z");
    later.lane_id = Some(lane.lane_id.clone());
    later.scope_ref = format!("model-lane://{}/{}", posture.run_id, lane.lane_id);
    harness.create_record(harness.store.record_lane_lease(later)).await.expect("later current lease");
    let boot = harness.recover(harness.store.recover_restartable_runs_at_boot()).await.expect("later boot recovery");
    assert_eq!(boot.len(), 1);
    assert_eq!(boot[0].checkpoint, checkpoint);
    assert_eq!(boot[0].reclaimable_lease_ids, vec!["race-expired-a", "race-expired-b", "race-expired-later"]);
    let receipts = harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256)).await.expect("later boot receipts");
    assert_eq!(harness.recover(harness.store.recover_restartable_runs_at_boot()).await.expect("repeat boot"), boot);
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(&posture.run_id, 256)).await.expect("repeat boot receipts"), receipts);
    let canonical = harness.navigation(harness.store.navigation_by_recovery(&posture.run_id)).await.expect("complete durable canonical tail");
    assert_eq!(canonical.recovery_events.iter().map(|v| v.replay_order_seq).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
    assert_eq!(canonical.recovery_events.iter().filter(|v| v.event_kind == ModelLaneRecoveryEventKind::OrphanDetected).count(), 3);
    harness.cleanup().await;
}

fn one_field_recovery_scopes(scope: &ResourceScope) -> Vec<ResourceScope> {
    let mut owner = scope.clone(); owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = scope.clone(); actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = scope.clone(); session.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let mut access = scope.clone(); access.access_space = Some(AccessSpaceRef::mint());
    let mut workspace = scope.clone(); workspace.workspace = Some(WorkspaceScopeRef::new("workspace-foreign-recovery").expect("workspace"));
    vec![owner, actor, session, access, workspace]
}

#[tokio::test]
async fn recovery_rejects_post_checkpoint_crdt_message_as_repair_for_in_bound_missing_base() {
    let harness = Harness::create("late-crdt-repair").await;
    let posture = harness.posture("late-crdt-repair").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("real canonical initial message");
    let mut event = recovery_event(&posture, &message, "event-missing-crdt-base");
    event.event_kind = ModelLaneRecoveryEventKind::CrdtUpdateObserved;
    let event = harness.create_record(harness.store.record_recovery_event(event))
        .await.expect("canonical observed recovery event missing CRDT base");
    let mut checkpoint = checkpoint(&posture, &message, "checkpoint-before-crdt-repair");
    checkpoint.last_event_ledger_seq = harness.head(&posture.run_id).await;
    assert_eq!(checkpoint.last_event_ledger_seq, event.event_ledger_seq);
    harness.create_record(harness.store.record_recovery_checkpoint(checkpoint))
        .await.expect("actual head includes unresolved recovery event");
    assert_recovery_failure_without_mutation(&harness, &posture.run_id,
        ModelLaneRecoveryFailureKind::StaleCrdtBase).await;
    let mut repair = posture.message.clone();
    repair.message_id = "message-late-crdt-repair".into();
    repair.message_span_id = "span-message-late-crdt-repair".into();
    repair.idempotency_key = "idem-message-late-crdt-repair".into();
    repair.replay_order_key = "late-crdt-repair".into();
    let repair = harness.create_record(harness.store.record_message(repair))
        .await.expect("later actual CRDT-backed message");
    assert!(repair.crdt_authority_binding.is_some());
    assert!(repair.event_ledger_seq > event.event_ledger_seq);
    assert_recovery_failure_without_mutation(&harness, &posture.run_id,
        ModelLaneRecoveryFailureKind::StaleCrdtBase).await;
    harness.cleanup().await;
}

#[cfg(feature = "runtime-full")]
#[tokio::test]
async fn recovery_reconstructs_projection_drift_from_canonical_checkpoint_then_denies_corrupt_receipt() {
    let harness = Harness::create("canonical-checkpoint-drift").await;
    let posture = harness.posture("canonical-checkpoint-drift").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("genuine checkpoint source");
    let id = "checkpoint-canonical-drift";
    let mut input = checkpoint(&posture, &message, id);
    input.last_event_ledger_seq = harness.head(&posture.run_id).await;
    let checkpoint = harness.create_record(harness.store.record_recovery_checkpoint(input))
        .await.expect("genuine admitted checkpoint");
    let canonical = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("owner-only component baseline");
    harness.store.test_corrupt_owned_recovery_authority(&harness.isolated, "recovery_checkpoint", id,
        ModelLaneAuthorityTestCorruption::RecoveryCheckpointProjectionDrift)
        .await.expect("finite projection-only id and sequence drift");
    let drift = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component observation of actual projection drift");
    let record: serde_json::Value = serde_json::from_str(&drift.record_json).expect("projection JSON");
    assert_eq!(record["checkpoint_id"], "checkpoint-row-drift-mutated");
    let canonical_record: serde_json::Value = serde_json::from_str(&canonical.record_json).expect("canonical projection JSON");
    let mut normalized_record = record.clone();
    normalized_record["checkpoint_id"] = canonical_record["checkpoint_id"].clone();
    assert_eq!(normalized_record, canonical_record);
    assert_eq!(drift.event_seq, 999999);
    let mut unchanged_receipt = drift.clone();
    unchanged_receipt.record_json = canonical.record_json.clone();
    unchanged_receipt.event_seq = canonical.event_seq;
    assert_eq!(unchanged_receipt, canonical, "every canonical receipt field remains unchanged");
    let recovered = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("canonical ledger reconstructs projection-only drift");
    assert_eq!(recovered.checkpoint, checkpoint);
    let boot = harness.recover(harness.store.recover_restartable_runs_at_boot())
        .await.expect("boot also reconstructs canonical checkpoint");
    assert_eq!(boot.len(), 1);
    assert_eq!(boot[0].checkpoint, checkpoint);
    assert_eq!(harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component observation after successful native recovery"), drift);
    harness.store.test_corrupt_owned_recovery_authority(&harness.isolated, "recovery_checkpoint", id,
        ModelLaneAuthorityTestCorruption::RecoveryCheckpointReceiptIdentity)
        .await.expect("finite independent ledger identity corruption after projection drift");
    let corrupt = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component observation of actual corrupt ledger");
    let payload: serde_json::Value = serde_json::from_str(&corrupt.receipt_payload_json).expect("receipt JSON");
    assert_eq!(payload["record"]["checkpoint_id"], "checkpoint-row-drift-mutated-ledger");
    let prior_payload: serde_json::Value = serde_json::from_str(&drift.receipt_payload_json).expect("prior canonical receipt JSON");
    let mut normalized_payload = payload.clone();
    normalized_payload["record"]["checkpoint_id"] = prior_payload["record"]["checkpoint_id"].clone();
    assert_eq!(normalized_payload, prior_payload);
    assert_ne!(corrupt.receipt_payload_json, drift.receipt_payload_json);
    let mut unchanged = corrupt.clone();
    unchanged.receipt_payload_json = drift.receipt_payload_json.clone();
    assert_eq!(unchanged, drift, "ledger corruption preserves the projection and stored hash");
    assert_ne!(format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(
        corrupt.receipt_payload_json.as_bytes())), corrupt.receipt_payload_hash);
    assert!(harness.recover(harness.store.recover_run_after_restart(&posture.run_id)).await.is_err());
    assert!(harness.recover(harness.store.recover_restartable_runs_at_boot()).await.is_err());
    assert_eq!(harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component nonmutation observation after native denials"), corrupt);
    harness.cleanup().await;
}

#[cfg(feature = "runtime-full")]
#[tokio::test]
async fn recovery_denies_revoked_crdt_descendant_read_with_current_parent_recovery_grant() {
    let harness = Harness::create("recovery-child-read-revoked").await;
    let posture = harness.posture("recovery-child-read-revoked").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("genuine authenticated CRDT-backed message");
    assert!(message.crdt_authority_binding.is_some());
    let id = "checkpoint-recovery-child-read-revoked";
    let mut input = checkpoint(&posture, &message, id);
    input.last_event_ledger_seq = harness.head(&posture.run_id).await;
    input.last_message_id = None;
    input.open_payload_refs.clear();
    let checkpoint = harness.create_record(harness.store.record_recovery_checkpoint(input))
        .await.expect("checkpoint with no optional message or open-payload references");
    let before_revoke = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect("genuine complete recovery before child Read revocation");
    assert_eq!(before_revoke.checkpoint, checkpoint);
    assert!(before_revoke.replay.messages.iter().any(|row| row == &message));
    let boot_before_revoke = harness.recover(harness.store.recover_restartable_runs_at_boot())
        .await.expect("genuine boot recovery before child Read revocation");
    assert_eq!(boot_before_revoke.len(), 1);
    assert_eq!(boot_before_revoke[0], before_revoke);
    let document_read = AuthorizationRequest {
        session_token: harness.session_token.clone(),
        channel_binding_hash: Some(harness.channel_hash.clone()),
        capability_id: "fs.read".into(), resource_kind: ResourceKind::RichDocument,
        external_resource_id: harness.document_id.clone(), action: ResourceAction::Read,
    };
    let parent_recovery = AuthorizationRequest {
        session_token: harness.session_token.clone(),
        channel_binding_hash: Some(harness.channel_hash.clone()),
        capability_id: "model_lane.recovery.execute".into(), resource_kind: ResourceKind::Workspace,
        external_resource_id: harness.scope.workspace.as_ref().expect("workspace").as_str().into(),
        action: ResourceAction::Update,
    };
    let child_before = harness.storage.authorize_protected_resource(document_read.clone())
        .await.expect("actual current child Read grant");
    let parent_before = harness.storage.authorize_protected_resource(parent_recovery.clone())
        .await.expect("actual current parent recovery grant");
    assert_ne!(child_before.grant_id, parent_before.grant_id);
    let checkpoint_before = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component-only owned checkpoint baseline");
    let head_before = harness.head(&posture.run_id).await;
    harness.storage.revoke_grant(&child_before.grant_id)
        .await.expect("revoke the actual child grant after genuine message admission");
    assert!(harness.storage.authorize_protected_resource(document_read).await.is_err());
    let parent_after = harness.storage.authorize_protected_resource(parent_recovery)
        .await.expect("parent Workspace recovery Update stays genuinely authorized");
    assert_eq!(parent_after.grant_id, parent_before.grant_id);
    assert_eq!(parent_after.resource_id, parent_before.resource_id);
    let direct = harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect_err("filtered required CRDT message cannot become incomplete recovery success");
    let boot = harness.recover(harness.store.recover_restartable_runs_at_boot())
        .await.expect_err("boot must deny the same required descendant omission");
    assert_eq!(direct.to_string(), boot.to_string());
    for private in [&harness.document_id, &message.message_id, &message.payload_ref, &posture.run_id] {
        assert!(!direct.to_string().contains(private.as_str()), "membership denial cannot disclose a child or candidate identity");
    }
    assert_eq!(harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component-only checkpoint after both native denials"), checkpoint_before);
    assert_eq!(harness.head(&posture.run_id).await, head_before);
    assert_eq!(harness.recover(harness.store.recover_run_after_restart(&posture.run_id))
        .await.expect_err("repeated direct recovery remains denied").to_string(), direct.to_string());
    assert_eq!(harness.recover(harness.store.recover_restartable_runs_at_boot())
        .await.expect_err("repeated boot recovery remains denied").to_string(), direct.to_string());
    assert_eq!(harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component-only checkpoint after repeated denials"), checkpoint_before);
    assert_eq!(harness.head(&posture.run_id).await, head_before);
    harness.cleanup().await;
}

#[tokio::test]
async fn boot_recovery_discovers_complete_scope_and_repeats_with_uncheckpointed_run() {
    let harness = Harness::create("boot-complete-scope").await;
    let posture = harness.posture("boot-complete-scope").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("genuine checkpoint source");
    let mut input = checkpoint(&posture, &message, "checkpoint-boot-complete-scope");
    input.last_event_ledger_seq = harness.head(&posture.run_id).await;
    let checkpoint = harness.create_record(harness.store.record_recovery_checkpoint(input))
        .await.expect("genuine checkpointed run");
    let mut uncheckpointed = harness.read(harness.store.replay_run(&posture.run_id))
        .await.expect("genuine run input lineage").run.inner;
    uncheckpointed.run_id = "run-boot-complete-uncheckpointed".into();
    uncheckpointed.trace_id = "trace-boot-complete-uncheckpointed".into();
    uncheckpointed.run_span_id = "span-boot-complete-uncheckpointed".into();
    uncheckpointed.lane_ids = vec!["lane-boot-complete-never-created".into()];
    uncheckpointed.event_ledger_stream_id = "model-lane://boot-complete-uncheckpointed".into();
    uncheckpointed.idempotency_key = "idem-boot-complete-uncheckpointed".into();
    uncheckpointed.replay_order_key = "boot-complete-uncheckpointed".into();
    let uncheckpointed = harness.create_record(harness.store.record_run(uncheckpointed))
        .await.expect("genuine run creation without checkpoint or prior coordination");
    #[cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
    {
        assert!(harness.read(harness.store.test_recovery_discovery_read_proof()).await.is_err());
        assert!(harness.create_record(harness.store.test_recovery_discovery_read_proof()).await.is_err());
        let proof = harness.recover(harness.store.test_recovery_discovery_read_proof())
            .await.expect("actual native same-mode finite field proof with scalar controls");
        let actual_keys = proof.as_object().expect("finite proof object").keys()
            .map(String::as_str).collect::<std::collections::BTreeSet<_>>();
        let expected_keys = ["candidate_count", "projection_control_count", "receipt_control_count",
            "star_denied", "alias_expression_wrap_denied", "pointers_denied", "virtual_ids_denied"]
            .into_iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual_keys, expected_keys, "finite proof cannot return any row or child data");
        assert_eq!(proof["candidate_count"].as_u64(), Some(2),
            "complete native discovery includes the genuine uncheckpointed run without prior coordination");
        assert!(proof["projection_control_count"].as_u64().expect("actual projection count") >= 2);
        assert!(proof["receipt_control_count"].as_u64().expect("actual receipt count") >= 2);
        for name in ["star_denied", "alias_expression_wrap_denied", "pointers_denied", "virtual_ids_denied"] {
            assert_eq!(proof[name].as_bool(), Some(true), "actual native field restriction {name}");
        }
    }
    let boot = harness.recover(harness.store.recover_restartable_runs_at_boot())
        .await.expect("complete same-scope native boot discovery");
    assert_eq!(boot.len(), 1);
    assert_eq!(boot[0].checkpoint, checkpoint);
    assert_eq!(harness.recover(harness.store.recover_restartable_runs_at_boot())
        .await.expect("repeated complete native boot recovery"), boot);
    assert!(harness.read(harness.store.test_scoped_authority_receipts(&uncheckpointed.run_id, 64))
        .await.expect("uncheckpointed run remains durably visible").iter()
        .all(|row| row.record_kind != "recovery_checkpoint"));
    harness.cleanup().await;
}

#[cfg(feature = "runtime-full")]
#[tokio::test]
async fn recovery_denies_persisted_receipt_stream_counterfactual_with_valid_payload_hash() {
    let harness = Harness::create("persisted-receipt-stream").await;
    let posture = harness.posture("persisted-receipt-stream").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("genuine checkpoint source");
    let id = "checkpoint-persisted-receipt-stream";
    let mut input = checkpoint(&posture, &message, id);
    input.last_event_ledger_seq = harness.head(&posture.run_id).await;
    harness.create_record(harness.store.record_recovery_checkpoint(input))
        .await.expect("genuine admitted checkpoint");
    let before = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("owner-only component baseline");
    harness.store.test_corrupt_owned_recovery_authority(&harness.isolated, "recovery_checkpoint", id,
        ModelLaneAuthorityTestCorruption::RecoveryCheckpointStream)
        .await.expect("finite receipt-only stream counterfactual");
    let changed = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component observation of actual receipt stream");
    let payload: serde_json::Value = serde_json::from_str(&changed.receipt_payload_json).expect("receipt JSON");
    assert_ne!(changed.receipt_session_run_id, before.receipt_session_run_id);
    assert_eq!(payload["record"]["event_ledger_stream_id"], changed.receipt_session_run_id);
    let original_payload: serde_json::Value = serde_json::from_str(&before.receipt_payload_json).expect("original receipt JSON");
    let mut normalized_payload = payload.clone();
    normalized_payload["record"]["event_ledger_stream_id"] = original_payload["record"]["event_ledger_stream_id"].clone();
    assert_eq!(normalized_payload, original_payload);
    assert_eq!(changed.receipt_payload_hash, format!("{:x}",
        <sha2::Sha256 as sha2::Digest>::digest(changed.receipt_payload_json.as_bytes())));
    let mut unchanged = changed.clone();
    unchanged.receipt_session_run_id = before.receipt_session_run_id.clone();
    unchanged.receipt_payload_json = before.receipt_payload_json.clone();
    unchanged.receipt_payload_hash = before.receipt_payload_hash.clone();
    assert_eq!(unchanged, before, "the projection and every other receipt field stay unchanged");
    assert!(harness.recover(harness.store.recover_run_after_restart(&posture.run_id)).await.is_err());
    assert!(harness.recover(harness.store.recover_restartable_runs_at_boot()).await.is_err());
    assert_eq!(harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_checkpoint", &posture.run_id, id)
        .await.expect("component nonmutation observation after native denials"), changed);
    harness.cleanup().await;
}

#[cfg(feature = "runtime-full")]
#[tokio::test]
async fn recovery_denies_persisted_gap_in_complete_canonical_recovery_order_without_mutation() {
    let harness = Harness::create("persisted-order-gap").await;
    let posture = harness.posture("persisted-order-gap").await;
    let message = harness.create_record(harness.store.record_message(posture.message.clone()))
        .await.expect("genuine recovery source");
    let first = harness.create_record(harness.store.record_recovery_event(
        recovery_event(&posture, &message, "event-persisted-gap-first")))
        .await.expect("first admitted recovery event");
    let second = harness.create_record(harness.store.record_recovery_event(
        recovery_event(&posture, &message, "event-persisted-gap-second")))
        .await.expect("second admitted recovery event");
    assert_eq!(first.replay_order_seq, 1);
    assert_eq!(second.replay_order_seq, 2);
    let mut input = checkpoint(&posture, &message, "checkpoint-persisted-gap");
    input.last_event_ledger_seq = harness.head(&posture.run_id).await;
    harness.create_record(harness.store.record_recovery_checkpoint(input))
        .await.expect("checkpoint covers actual admitted second event");
    let before = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_event", &posture.run_id, &second.recovery_event_id)
        .await.expect("owner-only component baseline");
    harness.store.test_corrupt_owned_recovery_authority(&harness.isolated, "recovery_event",
        &second.recovery_event_id, ModelLaneAuthorityTestCorruption::RecoveryReplayOrderGap)
        .await.expect("finite admitted second event becomes persisted order three");
    let changed = harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_event", &posture.run_id, &second.recovery_event_id)
        .await.expect("component observation of actual persisted order gap");
    let record: serde_json::Value = serde_json::from_str(&changed.record_json).expect("projection JSON");
    let payload: serde_json::Value = serde_json::from_str(&changed.receipt_payload_json).expect("receipt JSON");
    assert_eq!(record["replay_order_seq"], 3);
    assert_eq!(payload["record"]["replay_order_seq"], 3);
    let original_record: serde_json::Value = serde_json::from_str(&before.record_json).expect("original projection JSON");
    let original_payload: serde_json::Value = serde_json::from_str(&before.receipt_payload_json).expect("original receipt JSON");
    let mut normalized_record = record.clone();
    let mut normalized_payload = payload.clone();
    normalized_record["replay_order_seq"] = original_record["replay_order_seq"].clone();
    normalized_payload["record"]["replay_order_seq"] = original_payload["record"]["replay_order_seq"].clone();
    assert_eq!(normalized_record, original_record);
    assert_eq!(normalized_payload, original_payload);
    assert_eq!(changed.receipt_payload_hash, format!("{:x}",
        <sha2::Sha256 as sha2::Digest>::digest(changed.receipt_payload_json.as_bytes())));
    let mut unchanged = changed.clone();
    unchanged.record_json = before.record_json.clone();
    unchanged.receipt_payload_json = before.receipt_payload_json.clone();
    unchanged.receipt_payload_hash = before.receipt_payload_hash.clone();
    assert_eq!(unchanged, before, "only persisted order and its explicit hash adaptation change");
    assert!(harness.recover(harness.store.recover_run_after_restart(&posture.run_id)).await.is_err());
    assert!(harness.recover(harness.store.recover_restartable_runs_at_boot()).await.is_err());
    assert_eq!(harness.store.test_recovery_authority_snapshot(
        &harness.isolated, "recovery_event", &posture.run_id, &second.recovery_event_id)
        .await.expect("component nonmutation observation after native denials"), changed);
    harness.cleanup().await;
}

#[tokio::test]
async fn prepared_cloud_launch_denies_unbacked_projection_and_consent_without_lane_publication() {
    use handshake_core::swarm_orchestration::model_lane::{
        LaunchAuthority, ModelLaneError, ModelLaneKind, ModelLaneProviderKind, RuntimeBinding,
    };

    let harness = Harness::create("cloud-unbacked-authority").await;
    let posture = harness.posture("cloud-unbacked-authority").await;
    let before = harness.read(harness.store.replay_run(&posture.run_id))
        .await.expect("read genuine authenticated source run and lane");
    let before_receipts = harness.read(harness.store.test_scoped_authority_receipts(
        &posture.run_id, 256,
    )).await.expect("read genuine source ModelLane receipts");
    assert!(!before_receipts.is_empty());

    let run_id = format!("cloud-denied-run-{}", uuid::Uuid::now_v7());
    let lane_id = format!("cloud-denied-lane-{}", uuid::Uuid::now_v7());
    let stream_id = format!("model-lane://{run_id}/events");
    let projection_ref = format!("cloud-projection-plan://{run_id}/{lane_id}");
    let consent_ref = format!("cloud-consent-receipt://{run_id}/{lane_id}");
    let mut run = before.run.inner.clone();
    run.run_id = run_id.clone();
    run.lane_ids = vec![lane_id.clone()];
    run.event_ledger_stream_id = stream_id.clone();
    run.projection_plan_ref = Some(projection_ref.clone());
    run.consent_receipt_ref = Some(consent_ref.clone());
    run.idempotency_key = format!("prepared-cloud-denial-{run_id}");
    run.replay_order_key = format!("0001-{run_id}");
    run.selected_model_id = Some("model://dexterity/byok_cloud/gpt-4o-mini".into());
    run.candidate_model_ids = vec!["model://dexterity/byok_cloud/gpt-4o-mini".into()];
    let mut lane = before.lanes.iter().find(|lane| lane.lane_id == posture.lane_id)
        .expect("genuine source lane").inner.clone();
    lane.run_id = run_id.clone();
    lane.lane_id = lane_id.clone();
    lane.lane_span_id = format!("span-{lane_id}");
    lane.event_ledger_stream_id = stream_id;
    lane.kind = ModelLaneKind::CloudModel;
    lane.runtime_binding = RuntimeBinding::Cloud;
    lane.launch_authority = LaunchAuthority::CloudLane;
    lane.provider_kind = ModelLaneProviderKind::OpenAi;
    lane.model_id = run.selected_model_id.clone();
    lane.adapter_id = "openai_byok".into();
    lane.backend = "cloud_lane_openai".into();
    lane.projection_plan_ref = run.projection_plan_ref.clone();
    lane.consent_receipt_ref = run.consent_receipt_ref.clone();
    assert!(run.engine_profile.is_none() && lane.engine_profile.is_none());

    let denied = harness.create_record(harness.store.record_prepared_launch((run, lane)))
        .await.expect_err("unbacked durable cloud authority must deny prepared launch");
    let expected = format!(
        "CX-MM-007 cloud lane launch denied for run_id {run_id} lane_id {lane_id}: final cloud launch insertion fence denied: invalid model lane input: ProjectionPlan {projection_ref} is not durable"
    );
    match denied {
        ModelLaneError::InvalidInput(reason) => assert_eq!(reason, expected),
        other => panic!("expected exact CX-MM-007 durable projection denial, got {other}"),
    }
    assert!(matches!(
        harness.read(harness.store.replay_run(&run_id)).await,
        Err(ModelLaneError::NotFound(reason)) if reason == format!("run_id {run_id}")
    ), "denied prepared cloud run must not be published");
    assert!(harness.read(harness.store.test_scoped_authority_receipts(&run_id, 256))
        .await.expect("inspect denied run ModelLane authority receipts").is_empty(),
        "denial must not publish ModelLane run/lane authority or linked receipts");
    assert_eq!(harness.read(harness.store.replay_run(&posture.run_id))
        .await.expect("read original run after cloud denial"), before);
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(
        &posture.run_id, 256,
    )).await.expect("read original ModelLane receipts after cloud denial"), before_receipts);
    harness.cleanup().await;
}
#[tokio::test]
async fn prepared_cloud_launch_denies_missing_consent_after_genuine_durable_projection() {
    use handshake_core::swarm_orchestration::model_lane::{
        LaunchAuthority, ModelLaneError, ModelLaneKind, ModelLaneProviderKind, RuntimeBinding,
        CloudExportDelegation, ModelLaneCloudConsentScope, ModelLaneCloudExportPosture,
        ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy, NewModelLaneCloudProjectionPlan,
    };

    let harness = Harness::create("cloud-missing-consent").await;
    let posture = harness.posture("cloud-missing-consent").await;
    let (_, payload) = ordinary_payload_case(&posture, "cloud-missing-consent");
    let payload_ref = payload.artifact_ref.clone();
    let payload_hash = payload.artifact_sha256.clone();
    harness.create_record(harness.store.record_context_bundle_artifact_binding(payload))
        .await.expect("persist genuine projection payload bytes and hash through canonical producer");
    let before = harness.read(harness.store.replay_run(&posture.run_id))
        .await.expect("read genuine authenticated source run and lane");
    let before_receipts = harness.read(harness.store.test_scoped_authority_receipts(
        &posture.run_id, 256,
    )).await.expect("read genuine source ModelLane receipts");
    assert!(!before_receipts.is_empty());

    let run_id = format!("cloud-denied-run-{}", uuid::Uuid::now_v7());
    let lane_id = format!("cloud-denied-lane-{}", uuid::Uuid::now_v7());
    let stream_id = format!("model-lane://{run_id}/events");
    let projection_ref = format!("cloud-projection-plan://{run_id}/{lane_id}");
    let consent_ref = format!("cloud-consent-receipt://{run_id}/{lane_id}");
    let mut run = before.run.inner.clone();
    run.run_id = run_id.clone();
    run.lane_ids = vec![lane_id.clone()];
    run.event_ledger_stream_id = stream_id.clone();
    run.projection_plan_ref = Some(projection_ref.clone());
    run.consent_receipt_ref = Some(consent_ref.clone());
    run.idempotency_key = format!("prepared-cloud-denial-{run_id}");
    run.replay_order_key = format!("0001-{run_id}");
    run.selected_model_id = Some("model://dexterity/byok_cloud/gpt-4o-mini".into());
    run.candidate_model_ids = vec!["model://dexterity/byok_cloud/gpt-4o-mini".into()];
    let mut lane = before.lanes.iter().find(|lane| lane.lane_id == posture.lane_id)
        .expect("genuine source lane").inner.clone();
    lane.run_id = run_id.clone();
    lane.lane_id = lane_id.clone();
    lane.lane_span_id = format!("span-{lane_id}");
    lane.event_ledger_stream_id = stream_id;
    lane.kind = ModelLaneKind::CloudModel;
    lane.runtime_binding = RuntimeBinding::Cloud;
    lane.launch_authority = LaunchAuthority::CloudLane;
    lane.provider_kind = ModelLaneProviderKind::OpenAi;
    lane.model_id = run.selected_model_id.clone();
    lane.adapter_id = "openai_byok".into();
    lane.backend = "cloud_lane_openai".into();
    lane.projection_plan_ref = run.projection_plan_ref.clone();
    lane.consent_receipt_ref = run.consent_receipt_ref.clone();
    assert!(run.engine_profile.is_none() && lane.engine_profile.is_none());

    let scope_basis = json!({
        "run_id": &run_id,
        "lane_id": &lane_id,
        "model_session_id": &lane.model_session_id,
        "provider_kind": "openai",
        "requested_model_id": lane.model_id.as_ref().expect("cloud model fixture identity"),
        "projection_plan_id": &projection_ref,
    });
    let scope_hash = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(
        serde_json::to_vec(&scope_basis).expect("serialize actual projection scope basis"),
    ));
    let plan = harness.create_record(harness.store.record_cloud_projection_plan(
        NewModelLaneCloudProjectionPlan {
            projection_plan_id: projection_ref.clone(), run_id: run_id.clone(),
            trace_id: run.trace_id.clone(), lane_id: Some(lane_id.clone()),
            model_session_id: Some(lane.model_session_id.clone()), provider_kind: Some("openai".into()),
            requested_model_id: lane.model_id.clone(), scope_hash,
            source_artifact_refs: vec![payload_ref.clone()], payload_artifact_ref: payload_ref,
            payload_sha256: payload_hash,
            redaction_policy_ref: "redaction-policy://operator-chat/cloud-byok".into(),
            redaction_summary: "component payload contains only its explicitly authored ordinary text".into(),
            retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
            export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
            provider_profile_ref: "provider-profile://operator-chat/openai".into(),
            fan_out_targets: vec!["provider://openai/byok".into()],
            export_delegation: CloudExportDelegation {
                audience_refs: vec!["provider://openai/byok".into()],
                source_scope: handshake_core::swarm_orchestration::resource_scope::AccountBoundAuthority::from_access(harness.store.access()),
                authorization_receipt_ref: None,
            },
            consent_scope: ModelLaneCloudConsentScope::SingleLane, target_bindings: vec![],
            status: ModelLaneCloudProjectionPlanStatus::Active,
            event_ledger_stream_id: run.event_ledger_stream_id.clone(),
            work_packet_id: run.work_packet_id.clone().expect("actual producer work packet"),
            micro_task_id: run.micro_task_id.clone().expect("actual producer microtask"),
            task_board_id: run.task_board_id.clone().expect("actual producer taskboard"),
            owner_session: run.owner_session.clone(), idempotency_key: format!("projection-{run_id}"),
            created_at_utc: Utc::now().to_rfc3339(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch".into(),
            diagnostic_payload: json!({"case": "genuine-projection-without-consent"}),
        },
    )).await.expect("issue durable projection under genuine scoped Workspace Create");
    assert!(!plan.event_ledger_event_id.is_empty());
    assert!(plan.event_ledger_seq > 0);
    assert_eq!(plan.projection_plan_hash.len(), 64);
    let authority_before = harness.read(harness.store.replay_cloud_consent_authority(&run_id))
        .await.expect("independently reread native projection and absent consent");
    assert_eq!(authority_before.projection_plans, vec![plan]);
    assert!(authority_before.consent_receipts.is_empty());
    let denied = harness.create_record(harness.store.record_prepared_launch((run, lane)))
        .await.expect_err("genuine projection without durable consent must deny prepared launch");
    let expected = format!(
        "CX-MM-007 cloud lane launch denied for run_id {run_id} lane_id {lane_id}: final cloud launch insertion fence denied: invalid model lane input: ConsentReceipt {consent_ref} is not durable"
    );
    match denied {
        ModelLaneError::InvalidInput(reason) => assert_eq!(reason, expected),
        other => panic!("expected exact CX-MM-007 missing durable consent denial, got {other}"),
    }
    assert!(matches!(
        harness.read(harness.store.replay_run(&run_id)).await,
        Err(ModelLaneError::NotFound(reason)) if reason == format!("run_id {run_id}")
    ), "denied prepared cloud run must not be published");
    assert!(harness.read(harness.store.test_scoped_authority_receipts(&run_id, 256))
        .await.expect("inspect denied run ModelLane authority receipts").is_empty(),
        "denial must not publish ModelLane run/lane authority or linked receipts");
    assert_eq!(harness.read(harness.store.replay_run(&posture.run_id))
        .await.expect("read original run after cloud denial"), before);
    assert_eq!(harness.read(harness.store.test_scoped_authority_receipts(
        &posture.run_id, 256,
    )).await.expect("read original ModelLane receipts after cloud denial"), before_receipts);
    assert_eq!(harness.read(harness.store.replay_cloud_consent_authority(&run_id))
        .await.expect("reread native projection and absent consent after denial"), authority_before);
    harness.cleanup().await;
}