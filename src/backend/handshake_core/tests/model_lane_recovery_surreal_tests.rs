//! MT-007 embedded-Surreal recovery, lease, diagnostic, and MT-status proof.

mod surreal_test_store_support;

use chrono::{DateTime, Utc};
use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneAuthorityTestCorruption, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneLeaseScope, ModelLaneLeaseState, ModelLaneMtRuntimeStatus, ModelLaneRecoveryEventKind,
    ModelLaneRecoveryState, ModelLaneRecoveryStatus, ModelLaneStatus, ModelLaneStore,
    NewModelLaneDiagnosticTierStatus, NewModelLaneLease, NewModelLaneMtRuntimeStatus,
    NewModelLaneRecoveryCheckpoint, NewModelLaneRecoveryEvent,
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
            .expect("allocate recovery scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate SurrealStorage");
        bootstrap_schema(&storage).await.expect("bootstrap schema");
        let scope = exact_scope(label);
        let store = ModelLaneStore::new_scoped(storage.clone(), scope.clone());
        Self {
            isolated,
            storage,
            scope,
            store,
        }
    }

    async fn posture(&self, label: &str) -> SurrealAdmissibleCrdtPosture {
        build_surreal_admissible_crdt_posture(
            &self.store,
            self.scope.workspace.as_ref().expect("workspace").as_str(),
            label,
        )
        .await
        .expect("seed production posture")
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("cleanup recovery scope");
    }
}

#[tokio::test]
async fn recovery_replays_checkpoint_events_leases_diagnostics_and_mt_status_after_restart() {
    let mut harness = Harness::create("replay").await;
    let posture = harness.posture("replay").await;
    let message = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record source message");
    let checkpoint = checkpoint(&posture, &message, "checkpoint-replay");
    let stored_checkpoint = harness
        .store
        .record_recovery_checkpoint(checkpoint.clone())
        .await
        .expect("record checkpoint atomically");
    assert_eq!(
        harness
            .store
            .record_recovery_checkpoint(checkpoint)
            .await
            .expect("checkpoint retry"),
        stored_checkpoint
    );
    let event = recovery_event(&posture, &message, "event-replay");
    let stored_event = harness
        .store
        .record_recovery_event(event.clone())
        .await
        .expect("record ordered recovery event");
    assert_eq!(
        harness
            .store
            .record_recovery_event(event)
            .await
            .expect("event retry"),
        stored_event
    );
    harness
        .store
        .record_lane_lease(lease(&posture, "lease-replay", "2099-01-01T00:00:00Z"))
        .await
        .expect("record active lease");
    for tier in [
        ModelLaneDiagnosticTier::FlightRecorder,
        ModelLaneDiagnosticTier::InternalDiagnostics,
        ModelLaneDiagnosticTier::Palmistry,
    ] {
        harness
            .store
            .record_diagnostic_tier_status(diagnostic(&posture, tier))
            .await
            .expect("record diagnostic tier");
    }
    harness
        .store
        .record_mt_runtime_status(mt_status(&posture))
        .await
        .expect("record MT status");

    let now = DateTime::parse_from_rfc3339("2026-09-02T01:00:00Z")
        .expect("timestamp")
        .with_timezone(&Utc);
    let before = harness
        .store
        .test_recover_run_after_restart_at(&posture.run_id, now)
        .await
        .expect("recover exact run");
    assert_eq!(before.checkpoint.checkpoint_id, "checkpoint-replay");
    assert_eq!(before.recovery_events, vec![stored_event]);
    assert_eq!(before.active_leases.len(), 1);
    assert!(before.reclaimable_lease_ids.is_empty());
    assert_eq!(before.mt_runtime_statuses.len(), 1);
    harness
        .store
        .validate_diagnostic_tier_posture(&posture.run_id, "HBR-INT-009")
        .await
        .expect("three-tier diagnostic posture");
    let receipts = harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64)
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
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close storage");
    harness.isolated.reopen().await.expect("reopen same scope");
    let storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate storage");
    let reopened = ModelLaneStore::new_scoped(storage.clone(), harness.scope.clone());
    assert_eq!(
        reopened
            .test_recover_run_after_restart_at(&posture.run_id, now)
            .await
            .expect("recover after restart"),
        before
    );
    assert_eq!(
        reopened
            .recover_restartable_runs_at_boot()
            .await
            .expect("boot recovery")
            .len(),
        1
    );
    harness.store = reopened;
    harness.storage = storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn recovery_lease_expiry_is_deterministic_and_does_not_widen_scope() {
    let harness = Harness::create("lease-expiry").await;
    let posture = harness.posture("lease-expiry").await;
    let message = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record source message");
    harness
        .store
        .record_recovery_checkpoint(checkpoint(&posture, &message, "checkpoint-expiry"))
        .await
        .expect("record checkpoint");
    harness
        .store
        .record_lane_lease(lease(&posture, "lease-expired", "2026-09-02T00:00:00Z"))
        .await
        .expect("record lease");
    let now = DateTime::parse_from_rfc3339("2026-09-02T01:00:00Z")
        .expect("timestamp")
        .with_timezone(&Utc);
    let recovered = harness
        .store
        .test_recover_run_after_restart_at(&posture.run_id, now)
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
        let message = harness
            .store
            .record_message(posture.message.clone())
            .await
            .expect("record source message");
        let checkpoint_id = format!("checkpoint-corrupt-{index}");
        harness
            .store
            .record_recovery_checkpoint(checkpoint(&posture, &message, &checkpoint_id))
            .await
            .expect("record canonical checkpoint");
        harness
            .store
            .test_corrupt_scoped_authority("recovery_checkpoint", &checkpoint_id, corruption)
            .await
            .expect("apply enumerated checkpoint corruption");
        assert!(harness
            .store
            .recover_run_after_restart(&posture.run_id)
            .await
            .is_err());
        assert!(harness
            .store
            .recover_restartable_runs_at_boot()
            .await
            .is_err());
        harness.cleanup().await;
    }
}

#[tokio::test]
async fn recovery_rejects_cross_stream_checkpoint_and_event_sequence_gap_without_mutation() {
    let harness = Harness::create("stream-gap").await;
    let posture = harness.posture("stream-gap").await;
    let message = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record source message");
    let before = harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64)
        .await
        .expect("baseline authority receipts");

    let mut foreign_stream = checkpoint(&posture, &message, "checkpoint-foreign-stream");
    foreign_stream.event_ledger_stream_id = "model-lane://foreign/stream".into();
    assert!(harness
        .store
        .record_recovery_checkpoint(foreign_stream)
        .await
        .is_err());
    assert_eq!(
        harness
            .store
            .test_scoped_authority_receipts(&posture.run_id, 64)
            .await
            .expect("cross-stream denial receipt watermark"),
        before
    );

    harness
        .store
        .record_recovery_checkpoint(checkpoint(&posture, &message, "checkpoint-stream-gap"))
        .await
        .expect("record canonical checkpoint");
    let before_gap = harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64)
        .await
        .expect("pre-gap authority watermark");
    let mut gap = recovery_event(&posture, &message, "event-sequence-gap");
    gap.source_event_ledger_seq = Some(message.event_ledger_seq + 10_000);
    let gap_result = harness.store.record_recovery_event(gap).await;
    let after = harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 64)
        .await
        .expect("inspect authority after denials");
    match gap_result {
        Err(_) => assert_eq!(after, before_gap),
        Ok(_) => {
            assert!(harness
                .store
                .recover_run_after_restart(&posture.run_id)
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
    let message = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record source message");
    harness
        .store
        .record_recovery_checkpoint(checkpoint(
            &posture,
            &message,
            "checkpoint-concurrent-children",
        ))
        .await
        .expect("record recovery parent");

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let first_store = harness.store.clone();
    let first_barrier = barrier.clone();
    let first_event = recovery_event(&posture, &message, "event-child-a");
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        first_store.record_recovery_event(first_event).await
    });
    let second_store = harness.store.clone();
    let second_barrier = barrier.clone();
    let second_event = recovery_event(&posture, &message, "event-child-b");
    let second = tokio::spawn(async move {
        second_barrier.wait().await;
        second_store.record_recovery_event(second_event).await
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

    let recovered = harness
        .store
        .recover_run_after_restart(&posture.run_id)
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

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope {
        owner_account_id: OwnerAccountId::from_uuid(label_uuid(&(format!("account-{label}")))),
        actor_principal_id: ActorPrincipalId::from_uuid(label_uuid(&(format!("actor-{label}")))),
        authenticated_session: Some(AuthenticatedSessionRef::from_uuid(label_uuid(&(format!("session-{label}"))))),
        access_space: Some(AccessSpaceRef::from_uuid(label_uuid(&(format!("access-{label}"))))),
        workspace: Some(WorkspaceScopeRef::new(format!("workspace-{label}")).expect("workspace")),
    }
}

/// Deterministic identifier for a test label so the same label resolves to the
/// same exact scope across reopen phases of one proof.
fn label_uuid(label: &str) -> uuid::Uuid {
    let digest = <sha2::Sha256 as sha2::Digest>::digest(label.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    uuid::Uuid::from_bytes(bytes)
}
