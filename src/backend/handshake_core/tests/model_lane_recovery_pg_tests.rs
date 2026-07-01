//! WP-1 MT-007: Dexterity model-lane recovery proof.
//!
//! These tests use real PostgreSQL + kernel_event_ledger rows. They fail if
//! recovery is only a current-row replay, if FlightRecorder-only diagnostics
//! are accepted, or if MT runtime status cannot be reconstructed after restart.

mod knowledge_pg_support;

use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneDiagnosticTier, ModelLaneDiagnosticTierState,
    ModelLaneKind, ModelLaneLeaseScope, ModelLaneLeaseState, ModelLaneLocusBinding,
    ModelLaneMessageKind, ModelLaneMtRuntimeStatus, ModelLaneProviderKind,
    ModelLaneRecoveryEventKind, ModelLaneRecoveryFailureKind, ModelLaneRecoveryState,
    ModelLaneRecoveryStatus, ModelLaneRoutingMetadata, ModelLaneStatus, ModelLaneStore,
    ModelLaneTarget, NewModelLane, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneDiagnosticTierStatus, NewModelLaneLease, NewModelLaneMessage,
    NewModelLaneMtRuntimeStatus, NewModelLaneRecoveryCheckpoint, NewModelLaneRecoveryEvent,
    NewModelLaneRun, RuntimeBinding,
};
use serde_json::json;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn model_lane_recovery_replays_from_postgres_eventledger_checkpoint() {
    let (pool, store) = recovery_store().await;
    seed_run_lane_message(
        &store,
        "run-mt007-happy",
        "lane-local-mt007",
        "msg-mt007-001",
    )
    .await;

    let run = store
        .replay_run("run-mt007-happy")
        .await
        .expect("seeded run replays");
    let message_payload_ref = run.messages[0].payload_ref.clone();
    let _recovery_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-mt007-001",
            "run-mt007-happy",
            Some("lane-local-mt007"),
            ModelLaneRecoveryEventKind::CrdtUpdateObserved,
            1,
            Some(message_payload_ref.clone()),
            None,
        ))
        .await
        .expect("record recovery event");

    store
        .record_lane_lease(sample_lease(
            "lease-mt007-active",
            "run-mt007-happy",
            "lane-local-mt007",
            "2099-01-01T00:00:00Z",
            ModelLaneLeaseState::Active,
        ))
        .await
        .expect("record active lease");
    store
        .record_lane_lease(sample_lease(
            "lease-mt007-expired",
            "run-mt007-happy",
            "lane-local-mt007",
            "2020-01-01T00:00:00Z",
            ModelLaneLeaseState::Active,
        ))
        .await
        .expect("record expired lease");

    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt007-happy",
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/recovery",
        ))
        .await
        .expect("record flight recorder tier");
    let fr_only = store
        .validate_diagnostic_tier_posture("run-mt007-happy", "HBR-INT-009")
        .await
        .expect_err("FlightRecorder-only evidence is not enough");
    assert!(
        fr_only.to_string().contains("internal_diagnostics"),
        "expected missing internal diagnostics, got {fr_only}"
    );
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt007-happy",
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::Wired,
            "hbr-int-009://dexterity/recovery/details",
        ))
        .await
        .expect("record internal diagnostics tier");
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt007-happy",
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://wp1/mt007/external-worktree",
        ))
        .await
        .expect("record palmistry tier");
    let posture = store
        .validate_diagnostic_tier_posture("run-mt007-happy", "HBR-INT-009")
        .await
        .expect("three diagnostic tiers are represented");
    assert_eq!(posture.run_id, "run-mt007-happy");
    assert_eq!(posture.behavior_id, "HBR-INT-009");
    assert_eq!(posture.tiers.len(), 3);

    store
        .record_mt_runtime_status(sample_mt_status(
            "mt-status-mt007-rfv",
            "run-mt007-happy",
            "MT-007",
            ModelLaneMtRuntimeStatus::ReadyForValidation,
        ))
        .await
        .expect("record MT runtime status");

    let mut denied_cloud_run = sample_run("run-mt007-happy", "lane-cloud-mt007-denied");
    denied_cloud_run.lane_ids = vec!["lane-cloud-mt007-denied".into()];
    denied_cloud_run.projection_plan_ref =
        Some("cloud-projection-plan://run-mt007-happy/lane-cloud-mt007-denied".into());
    denied_cloud_run.consent_receipt_ref =
        Some("cloud-consent-receipt://run-mt007-happy/lane-cloud-mt007-denied".into());
    denied_cloud_run.idempotency_key = "idem-run-mt007-happy-cloud-denied".into();
    denied_cloud_run.replay_order_key = "00000003/cloud-denied-run".into();
    denied_cloud_run.locus_binding = Some(sample_locus(
        "run-mt007-happy",
        "coordinator-run-mt007-happy",
        "model-session-coordinator-run-mt007-happy",
    ));
    let mut denied_cloud_lane = sample_lane(
        "lane-cloud-mt007-denied",
        "run-mt007-happy",
        ModelLaneKind::CloudModel,
        RuntimeBinding::Cloud,
        LaunchAuthority::CloudLane,
    );
    denied_cloud_lane.model_id = Some("model://dexterity/byok_cloud/gpt-4o-mini".into());
    denied_cloud_lane.adapter_id = "openai_byok".into();
    denied_cloud_lane.backend = "cloud_lane_openai".into();
    denied_cloud_lane.projection_plan_ref = denied_cloud_run.projection_plan_ref.clone();
    denied_cloud_lane.consent_receipt_ref = denied_cloud_run.consent_receipt_ref.clone();
    let denial = store
        .record_prepared_launch((denied_cloud_run, denied_cloud_lane))
        .await
        .expect_err(
            "cloud lane without durable projection+consent must be denied before provider call",
        );
    assert!(
        denial.to_string().contains("CX-MM-007"),
        "expected cloud consent denial, got {denial}"
    );

    let checkpoint_high_watermark =
        event_stream_high_watermark(&pool, "mlane-stream-run-mt007-happy").await;
    let checkpoint = store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-mt007-001",
            "run-mt007-happy",
            Some("lane-local-mt007"),
            Some("msg-mt007-001"),
            Some("lease-mt007-active"),
            checkpoint_high_watermark,
            vec![message_payload_ref],
        ))
        .await
        .expect("record checkpoint");

    let recovered = store
        .recover_run_after_restart("run-mt007-happy")
        .await
        .expect("recover run from checkpoint and EventLedger");
    assert_eq!(recovered.checkpoint.checkpoint_id, checkpoint.checkpoint_id);
    assert_eq!(recovered.replay.run.run_id, "run-mt007-happy");
    assert_eq!(recovered.replay.messages.len(), 1);
    assert_eq!(recovered.recovery_events.len(), 2);
    assert!(recovered.recovery_events.iter().any(|event| {
        event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
            && event.lease_id.as_deref() == Some("lease-mt007-expired")
    }));
    assert_eq!(recovered.active_leases.len(), 1);
    assert_eq!(recovered.reclaimable_lease_ids, vec!["lease-mt007-expired"]);
    assert_eq!(recovered.cloud_consent_denials.len(), 1);
    assert_eq!(recovered.mt_runtime_statuses.len(), 1);
    assert_eq!(
        recovered.mt_runtime_statuses[0].status,
        ModelLaneMtRuntimeStatus::ReadyForValidation
    );
    let orphan_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_recovery_event' \
           AND payload->'record'->>'event_kind' = 'orphan_detected' \
           AND payload->'record'->>'lease_id' = 'lease-mt007-expired'",
    )
    .fetch_one(&pool)
    .await
    .expect("query durable orphan recovery event");
    assert_eq!(orphan_events, 1);
    let recovered_again = store
        .recover_run_after_restart("run-mt007-happy")
        .await
        .expect("second recovery returns existing orphan without duplicate write");
    assert!(recovered_again.recovery_events.iter().any(|event| {
        event.event_kind == ModelLaneRecoveryEventKind::OrphanDetected
            && event.lease_id.as_deref() == Some("lease-mt007-expired")
    }));
    let orphan_events_after_second_recovery: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_recovery_event' \
           AND payload->'record'->>'event_kind' = 'orphan_detected' \
           AND payload->'record'->>'lease_id' = 'lease-mt007-expired'",
    )
    .fetch_one(&pool)
    .await
    .expect("query durable orphan recovery event after second recovery");
    assert_eq!(orphan_events_after_second_recovery, 1);

    let ledger_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type IN \
         ('model_lane_recovery_checkpoint','model_lane_recovery_event','model_lane_lease',\
          'model_lane_diagnostic_tier','model_lane_mt_runtime_status')",
    )
    .fetch_one(&pool)
    .await
    .expect("query ledger proof rows");
    assert!(
        ledger_count >= 6,
        "MT-007 recovery writes must be EventLedger-backed, got {ledger_count}"
    );

    let corrupted_denial = sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = jsonb_set(
            payload,
            '{provider_call_attempted}',
            'true'::jsonb,
            false
        )
        WHERE aggregate_type = 'model_lane_cloud_consent_denial'
          AND payload->>'run_id' = 'run-mt007-happy'
        "#,
    )
    .execute(&pool)
    .await
    .expect("corrupt cloud denial provider-call proof");
    assert_eq!(corrupted_denial.rows_affected(), 1);
    assert_recovery_integrity_failure(&store, "run-mt007-happy").await;
}

#[tokio::test]
async fn model_lane_recovery_rejects_corrupt_checkpoint_and_event_seq_gap() {
    let (_pool, store) = recovery_store().await;

    seed_run_lane_message(&store, "run-mt007-gap", "lane-mt007-gap", "msg-mt007-gap").await;
    store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-gap-001",
            "run-mt007-gap",
            Some("lane-mt007-gap"),
            ModelLaneRecoveryEventKind::CheckpointRestored,
            1,
            None,
            None,
        ))
        .await
        .expect("record replay seq 1");
    let gap_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-gap-003",
            "run-mt007-gap",
            Some("lane-mt007-gap"),
            ModelLaneRecoveryEventKind::CheckpointRestored,
            3,
            None,
            None,
        ))
        .await
        .expect("record replay seq 3");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-gap",
            "run-mt007-gap",
            Some("lane-mt007-gap"),
            Some("msg-mt007-gap"),
            None,
            gap_event.event_ledger_seq,
            vec![],
        ))
        .await
        .expect("record gap checkpoint");
    assert_recovery_failure(
        &store,
        "run-mt007-gap",
        ModelLaneRecoveryFailureKind::EventLedgerSequenceGap,
    )
    .await;
}

#[tokio::test]
async fn model_lane_recovery_restores_mt_runtime_status_refs_after_restart() {
    let (_pool, store) = recovery_store().await;

    seed_run_lane_message(
        &store,
        "run-mt007-status",
        "lane-mt007-status",
        "msg-mt007-status",
    )
    .await;
    let run = store
        .replay_run("run-mt007-status")
        .await
        .expect("seeded run replays");
    let payload_ref = run.messages[0].payload_ref.clone();
    let _recovery_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-status",
            "run-mt007-status",
            Some("lane-mt007-status"),
            ModelLaneRecoveryEventKind::MtStatusRestored,
            1,
            Some(payload_ref.clone()),
            None,
        ))
        .await
        .expect("record status recovery event");
    store
        .record_mt_runtime_status(sample_mt_status(
            "mt-status-mt007-status",
            "run-mt007-status",
            "MT-007",
            ModelLaneMtRuntimeStatus::ReadyForValidation,
        ))
        .await
        .expect("record MT runtime status");
    let checkpoint_high_watermark =
        event_stream_high_watermark(&_pool, "mlane-stream-run-mt007-status").await;
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-status",
            "run-mt007-status",
            Some("lane-mt007-status"),
            Some("msg-mt007-status"),
            None,
            checkpoint_high_watermark,
            vec![payload_ref],
        ))
        .await
        .expect("record status checkpoint");

    let recovered = store
        .recover_run_after_restart("run-mt007-status")
        .await
        .expect("recover MT status refs");
    assert_eq!(recovered.mt_runtime_statuses.len(), 1);
    let status = &recovered.mt_runtime_statuses[0];
    assert_eq!(
        status.claimed_by_ref.as_deref(),
        Some("session://KERNEL_BUILDER-20260630-045713")
    );
    assert_eq!(
        status.proof_status_ref.as_deref(),
        Some("proof://mt007/model_lane_recovery_pg_tests")
    );
    assert_eq!(
        status.hbr_status_ref.as_deref(),
        Some("hbr-int-009://dexterity/recovery/details")
    );
    assert_eq!(
        status.last_runtime_status_ref.as_deref(),
        Some("runtime-status://mt007/ready-for-validation")
    );
}

#[tokio::test]
async fn model_lane_recovery_excludes_post_checkpoint_adjunct_state() {
    let (_pool, store) = recovery_store().await;

    seed_run_lane_message(
        &store,
        "run-mt007-adjunct-bound",
        "lane-mt007-adjunct-bound",
        "msg-mt007-adjunct-bound",
    )
    .await;
    let run = store
        .replay_run("run-mt007-adjunct-bound")
        .await
        .expect("seeded run replays");
    let payload_ref = run.messages[0].payload_ref.clone();
    let recovery_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-adjunct-bound",
            "run-mt007-adjunct-bound",
            Some("lane-mt007-adjunct-bound"),
            ModelLaneRecoveryEventKind::PayloadRefObserved,
            1,
            Some(payload_ref.clone()),
            None,
        ))
        .await
        .expect("record checkpoint-bounded recovery event");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-adjunct-bound",
            "run-mt007-adjunct-bound",
            Some("lane-mt007-adjunct-bound"),
            Some("msg-mt007-adjunct-bound"),
            None,
            recovery_event.event_ledger_seq,
            vec![payload_ref],
        ))
        .await
        .expect("record checkpoint before adjunct state");

    store
        .record_lane_lease(sample_lease(
            "lease-mt007-post-checkpoint",
            "run-mt007-adjunct-bound",
            "lane-mt007-adjunct-bound",
            "2099-01-01T00:00:00Z",
            ModelLaneLeaseState::Active,
        ))
        .await
        .expect("record post-checkpoint lease");
    store
        .record_mt_runtime_status(sample_mt_status(
            "mt-status-mt007-post-checkpoint",
            "run-mt007-adjunct-bound",
            "MT-007",
            ModelLaneMtRuntimeStatus::ReadyForValidation,
        ))
        .await
        .expect("record post-checkpoint MT status");

    let mut denied_cloud_run = sample_run(
        "run-mt007-adjunct-bound",
        "lane-cloud-mt007-post-checkpoint",
    );
    denied_cloud_run.lane_ids = vec!["lane-cloud-mt007-post-checkpoint".into()];
    denied_cloud_run.projection_plan_ref = Some(
        "cloud-projection-plan://run-mt007-adjunct-bound/lane-cloud-mt007-post-checkpoint".into(),
    );
    denied_cloud_run.consent_receipt_ref = Some(
        "cloud-consent-receipt://run-mt007-adjunct-bound/lane-cloud-mt007-post-checkpoint".into(),
    );
    denied_cloud_run.idempotency_key = "idem-run-mt007-adjunct-bound-cloud-denied".into();
    denied_cloud_run.replay_order_key = "00000003/cloud-denied-run".into();
    denied_cloud_run.locus_binding = Some(sample_locus(
        "run-mt007-adjunct-bound",
        "coordinator-run-mt007-adjunct-bound",
        "model-session-coordinator-run-mt007-adjunct-bound",
    ));
    let mut denied_cloud_lane = sample_lane(
        "lane-cloud-mt007-post-checkpoint",
        "run-mt007-adjunct-bound",
        ModelLaneKind::CloudModel,
        RuntimeBinding::Cloud,
        LaunchAuthority::CloudLane,
    );
    denied_cloud_lane.model_id = Some("model://dexterity/byok_cloud/gpt-4o-mini".into());
    denied_cloud_lane.adapter_id = "openai_byok".into();
    denied_cloud_lane.backend = "cloud_lane_openai".into();
    denied_cloud_lane.projection_plan_ref = denied_cloud_run.projection_plan_ref.clone();
    denied_cloud_lane.consent_receipt_ref = denied_cloud_run.consent_receipt_ref.clone();
    store
        .record_prepared_launch((denied_cloud_run, denied_cloud_lane))
        .await
        .expect_err("post-checkpoint cloud denial is recorded after checkpoint");

    let recovered = store
        .recover_run_after_restart("run-mt007-adjunct-bound")
        .await
        .expect("recover only checkpoint-bounded adjunct state");
    assert!(recovered.active_leases.is_empty());
    assert!(recovered.reclaimable_lease_ids.is_empty());
    assert!(recovered.cloud_consent_denials.is_empty());
    assert!(recovered.mt_runtime_statuses.is_empty());
}

#[tokio::test]
async fn diagnostic_tier_record_rejects_flight_recorder_only_evidence() {
    let (_pool, store) = recovery_store().await;
    seed_run_lane_message(
        &store,
        "run-mt007-diagnostics-no-flight",
        "lane-mt007-diagnostics-no-flight",
        "msg-mt007-diagnostics-no-flight",
    )
    .await;
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt007-diagnostics-no-flight",
            ModelLaneDiagnosticTier::InternalDiagnostics,
            ModelLaneDiagnosticTierState::Wired,
            "hbr-int-009://dexterity/recovery/details",
        ))
        .await
        .expect("record internal diagnostics without FlightRecorder");
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt007-diagnostics-no-flight",
            ModelLaneDiagnosticTier::Palmistry,
            ModelLaneDiagnosticTierState::DeferredWithReason,
            "palmistry://wp1/mt007/external-worktree",
        ))
        .await
        .expect("record palmistry without FlightRecorder");
    let no_flight = store
        .validate_diagnostic_tier_posture("run-mt007-diagnostics-no-flight", "HBR-INT-009")
        .await
        .expect_err("internal diagnostics plus Palmistry still requires FlightRecorder");
    assert!(no_flight.to_string().contains("FlightRecorder"));

    let (_pool, store) = recovery_store().await;
    seed_run_lane_message(
        &store,
        "run-mt007-diagnostics-fr-only",
        "lane-mt007-diagnostics-fr-only",
        "msg-mt007-diagnostics-fr-only",
    )
    .await;
    store
        .record_diagnostic_tier_status(sample_tier(
            "run-mt007-diagnostics-fr-only",
            ModelLaneDiagnosticTier::FlightRecorder,
            ModelLaneDiagnosticTierState::Wired,
            "eventledger://kernel/model-lane/recovery",
        ))
        .await
        .expect("record flight recorder tier");
    let err = store
        .validate_diagnostic_tier_posture("run-mt007-diagnostics-fr-only", "HBR-INT-009")
        .await
        .expect_err("FR-only evidence must fail");
    assert!(err.to_string().contains("FlightRecorder-only"));
}

#[tokio::test]
async fn model_lane_recovery_rejects_missing_payload_stale_crdt_and_duplicate_idempotency() {
    let (_pool, store) = recovery_store().await;

    store
        .record_run(sample_run(
            "run-mt007-replayed-message-missing-payload",
            "lane-mt007-replayed-message-missing-payload",
        ))
        .await
        .expect("record run without message artifact authority");
    store
        .record_lane(sample_lane(
            "lane-mt007-replayed-message-missing-payload",
            "run-mt007-replayed-message-missing-payload",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record lane without message artifact authority");
    let message_without_payload_artifact = store
        .record_message(sample_message(
            "msg-mt007-replayed-message-missing-payload",
            "run-mt007-replayed-message-missing-payload",
            "lane-mt007-replayed-message-missing-payload",
        ))
        .await
        .expect("record message without artifact binding");
    let replayed_missing_payload_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-replayed-message-missing-payload",
            "run-mt007-replayed-message-missing-payload",
            Some("lane-mt007-replayed-message-missing-payload"),
            ModelLaneRecoveryEventKind::CheckpointRestored,
            1,
            None,
            None,
        ))
        .await
        .expect("record recovery event without payload refs");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-replayed-message-missing-payload",
            "run-mt007-replayed-message-missing-payload",
            Some("lane-mt007-replayed-message-missing-payload"),
            Some("msg-mt007-replayed-message-missing-payload"),
            None,
            replayed_missing_payload_event
                .event_ledger_seq
                .max(message_without_payload_artifact.event_ledger_seq),
            vec![],
        ))
        .await
        .expect("record checkpoint that replays message with missing payload authority");
    assert_recovery_failure(
        &store,
        "run-mt007-replayed-message-missing-payload",
        ModelLaneRecoveryFailureKind::MissingPayloadAuthority,
    )
    .await;

    store
        .record_run(sample_run(
            "run-mt007-replayed-message-stale-crdt",
            "lane-mt007-replayed-message-stale-crdt",
        ))
        .await
        .expect("record stale CRDT run");
    store
        .record_lane(sample_lane(
            "lane-mt007-replayed-message-stale-crdt",
            "run-mt007-replayed-message-stale-crdt",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record stale CRDT lane");
    let mut stale_message = sample_message(
        "msg-mt007-replayed-message-stale-crdt",
        "run-mt007-replayed-message-stale-crdt",
        "lane-mt007-replayed-message-stale-crdt",
    );
    stale_message.crdt_stale_base_ref = Some("crdt-stale-base://mt007/replayed-message".into());
    let stale_message = store
        .record_message(stale_message.clone())
        .await
        .expect("record stale CRDT message");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(
            &stale_message.inner,
        ))
        .await
        .expect("record stale CRDT message payload authority");
    let stale_replay_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-replayed-message-stale-crdt",
            "run-mt007-replayed-message-stale-crdt",
            Some("lane-mt007-replayed-message-stale-crdt"),
            ModelLaneRecoveryEventKind::CheckpointRestored,
            1,
            None,
            None,
        ))
        .await
        .expect("record recovery event without CRDT refs");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-replayed-message-stale-crdt",
            "run-mt007-replayed-message-stale-crdt",
            Some("lane-mt007-replayed-message-stale-crdt"),
            Some("msg-mt007-replayed-message-stale-crdt"),
            None,
            stale_replay_event
                .event_ledger_seq
                .max(stale_message.event_ledger_seq),
            vec![],
        ))
        .await
        .expect("record checkpoint that replays stale CRDT message");
    assert_recovery_failure(
        &store,
        "run-mt007-replayed-message-stale-crdt",
        ModelLaneRecoveryFailureKind::StaleCrdtBase,
    )
    .await;

    seed_run_lane_message(
        &store,
        "run-mt007-missing-payload",
        "lane-mt007-missing-payload",
        "msg-mt007-missing-payload",
    )
    .await;
    let missing_payload_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-missing-payload",
            "run-mt007-missing-payload",
            Some("lane-mt007-missing-payload"),
            ModelLaneRecoveryEventKind::PayloadRefObserved,
            1,
            Some("artifact://model-lane/messages/not-present".into()),
            None,
        ))
        .await
        .expect("record missing payload recovery event");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-missing-payload",
            "run-mt007-missing-payload",
            Some("lane-mt007-missing-payload"),
            Some("msg-mt007-missing-payload"),
            None,
            missing_payload_event.event_ledger_seq,
            vec!["artifact://model-lane/messages/not-present".into()],
        ))
        .await
        .expect("record missing payload checkpoint");
    assert_recovery_failure(
        &store,
        "run-mt007-missing-payload",
        ModelLaneRecoveryFailureKind::MissingPayloadAuthority,
    )
    .await;

    seed_run_lane_message(
        &store,
        "run-mt007-stale-crdt",
        "lane-mt007-stale-crdt",
        "msg-mt007-stale-crdt",
    )
    .await;
    let stale_crdt_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-stale-crdt",
            "run-mt007-stale-crdt",
            Some("lane-mt007-stale-crdt"),
            ModelLaneRecoveryEventKind::CrdtUpdateObserved,
            1,
            None,
            Some("crdt-stale-base://mt007/test".into()),
        ))
        .await
        .expect("record stale CRDT recovery event");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-stale-crdt",
            "run-mt007-stale-crdt",
            Some("lane-mt007-stale-crdt"),
            Some("msg-mt007-stale-crdt"),
            None,
            stale_crdt_event.event_ledger_seq,
            vec![],
        ))
        .await
        .expect("record stale CRDT checkpoint");
    assert_recovery_failure(
        &store,
        "run-mt007-stale-crdt",
        ModelLaneRecoveryFailureKind::StaleCrdtBase,
    )
    .await;

    seed_run_lane_message(
        &store,
        "run-mt007-idem",
        "lane-mt007-idem",
        "msg-mt007-idem",
    )
    .await;
    let event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-idem",
            "run-mt007-idem",
            Some("lane-mt007-idem"),
            ModelLaneRecoveryEventKind::CheckpointRestored,
            1,
            None,
            None,
        ))
        .await
        .expect("record idempotent event");
    let mut conflicting = sample_recovery_event(
        "recovery-event-idem-other",
        "run-mt007-idem",
        Some("lane-mt007-idem"),
        ModelLaneRecoveryEventKind::CheckpointRestored,
        1,
        None,
        None,
    );
    conflicting.idempotency_key = event.idempotency_key.clone();
    let err = store
        .record_recovery_event(conflicting)
        .await
        .expect_err("divergent duplicate idempotency must fail");
    assert!(err.to_string().contains("idempotency_key"));
}

#[tokio::test]
async fn model_lane_recovery_uses_eventledger_checkpoint_authority_over_mutable_row() {
    let (pool, store) = recovery_store().await;

    seed_run_lane_message(
        &store,
        "run-mt007-checkpoint-row-drift",
        "lane-mt007-checkpoint-row-drift",
        "msg-mt007-checkpoint-row-drift",
    )
    .await;
    let run = store
        .replay_run("run-mt007-checkpoint-row-drift")
        .await
        .expect("seeded run replays");
    let payload_ref = run.messages[0].payload_ref.clone();
    let recovery_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-checkpoint-row-drift",
            "run-mt007-checkpoint-row-drift",
            Some("lane-mt007-checkpoint-row-drift"),
            ModelLaneRecoveryEventKind::PayloadRefObserved,
            1,
            Some(payload_ref.clone()),
            None,
        ))
        .await
        .expect("record valid recovery event");
    let checkpoint = store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-row-drift",
            "run-mt007-checkpoint-row-drift",
            Some("lane-mt007-checkpoint-row-drift"),
            Some("msg-mt007-checkpoint-row-drift"),
            None,
            recovery_event.event_ledger_seq,
            vec![payload_ref],
        ))
        .await
        .expect("record valid checkpoint");

    store
        .recover_run_after_restart("run-mt007-checkpoint-row-drift")
        .await
        .expect("baseline recovery succeeds before row drift");

    let updated = sqlx::query(
        r#"
        UPDATE model_lane_recovery_checkpoints
        SET record_json = jsonb_set(
            record_json,
            '{checkpoint_id}',
            to_jsonb($1::text),
            false
        ),
            event_ledger_seq = 999999
        WHERE checkpoint_id = $2
        "#,
    )
    .bind("checkpoint-row-drift-mutated")
    .bind(&checkpoint.checkpoint_id)
    .execute(&pool)
    .await
    .expect("mutate checkpoint record_json only");
    assert_eq!(
        updated.rows_affected(),
        1,
        "test must mutate exactly one mutable checkpoint row"
    );

    store
        .recover_run_after_restart("run-mt007-checkpoint-row-drift")
        .await
        .expect("mutable checkpoint row drift cannot override EventLedger authority");

    let updated = sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = jsonb_set(
            payload,
            '{record,checkpoint_id}',
            to_jsonb($1::text),
            false
        )
        WHERE event_id = $2
        "#,
    )
    .bind("checkpoint-row-drift-mutated-ledger")
    .bind(&checkpoint.event_ledger_event_id)
    .execute(&pool)
    .await
    .expect("mutate checkpoint EventLedger payload only");
    assert_eq!(
        updated.rows_affected(),
        1,
        "test must mutate exactly one checkpoint EventLedger row"
    );

    assert_recovery_integrity_failure(&store, "run-mt007-checkpoint-row-drift").await;
}

#[tokio::test]
async fn model_lane_recovery_rejects_cross_stream_checkpoint_payload() {
    let (pool, store) = recovery_store().await;
    let run_id = "run-mt007-cross-stream-checkpoint";
    let lane_id = "lane-mt007-cross-stream-checkpoint";
    let message_id = "msg-mt007-cross-stream-checkpoint";

    seed_run_lane_message(&store, run_id, lane_id, message_id).await;
    let replay = store.replay_run(run_id).await.expect("seeded run replays");
    let checkpoint = store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-cross-stream",
            run_id,
            Some(lane_id),
            Some(message_id),
            None,
            event_stream_high_watermark(&pool, &format!("mlane-stream-{run_id}")).await,
            vec![replay.messages[0].payload_ref.clone()],
        ))
        .await
        .expect("record canonical checkpoint");

    store
        .recover_run_after_restart(run_id)
        .await
        .expect("baseline recovery selects canonical checkpoint");

    let foreign_stream = format!("mlane-stream-foreign-{run_id}");
    let updated = sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET session_run_id = $1,
            payload = jsonb_set(
                payload,
                '{record,event_ledger_stream_id}',
                to_jsonb($1::text),
                false
            )
        WHERE event_id = $2
        "#,
    )
    .bind(&foreign_stream)
    .bind(&checkpoint.event_ledger_event_id)
    .execute(&pool)
    .await
    .expect("mutate checkpoint EventLedger stream");
    assert_eq!(updated.rows_affected(), 1);

    assert_recovery_failure(
        &store,
        run_id,
        ModelLaneRecoveryFailureKind::MissingCheckpoint,
    )
    .await;
}

#[tokio::test]
async fn model_lane_recovery_rejects_post_checkpoint_payload_and_crdt_repairs() {
    let (_pool, store) = recovery_store().await;

    store
        .record_run(sample_run(
            "run-mt007-post-checkpoint-payload",
            "lane-mt007-post-checkpoint-payload",
        ))
        .await
        .expect("record payload high-watermark run");
    store
        .record_lane(sample_lane(
            "lane-mt007-post-checkpoint-payload",
            "run-mt007-post-checkpoint-payload",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record payload high-watermark lane");
    let missing_payload_ref =
        "artifact://model-lane/messages/post-checkpoint-payload-repair".to_string();
    let missing_payload_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-post-checkpoint-payload",
            "run-mt007-post-checkpoint-payload",
            Some("lane-mt007-post-checkpoint-payload"),
            ModelLaneRecoveryEventKind::PayloadRefObserved,
            1,
            Some(missing_payload_ref.clone()),
            None,
        ))
        .await
        .expect("record pre-repair payload recovery event");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-post-checkpoint-payload",
            "run-mt007-post-checkpoint-payload",
            Some("lane-mt007-post-checkpoint-payload"),
            None,
            None,
            missing_payload_event.event_ledger_seq,
            vec![missing_payload_ref.clone()],
        ))
        .await
        .expect("record payload checkpoint before repair");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding(
            "artifact-binding-post-checkpoint-payload-repair",
            "run-mt007-post-checkpoint-payload",
            "trace-run-mt007-post-checkpoint-payload",
            &missing_payload_ref,
        ))
        .await
        .expect("record payload repair after checkpoint");
    assert_recovery_failure(
        &store,
        "run-mt007-post-checkpoint-payload",
        ModelLaneRecoveryFailureKind::MissingPayloadAuthority,
    )
    .await;

    store
        .record_run(sample_run(
            "run-mt007-post-checkpoint-crdt",
            "lane-mt007-post-checkpoint-crdt",
        ))
        .await
        .expect("record CRDT high-watermark run");
    store
        .record_lane(sample_lane(
            "lane-mt007-post-checkpoint-crdt",
            "run-mt007-post-checkpoint-crdt",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record CRDT high-watermark lane");
    let crdt_event = store
        .record_recovery_event(sample_recovery_event(
            "recovery-event-post-checkpoint-crdt",
            "run-mt007-post-checkpoint-crdt",
            Some("lane-mt007-post-checkpoint-crdt"),
            ModelLaneRecoveryEventKind::CrdtUpdateObserved,
            1,
            None,
            None,
        ))
        .await
        .expect("record pre-repair CRDT recovery event");
    store
        .record_recovery_checkpoint(sample_checkpoint(
            "checkpoint-post-checkpoint-crdt",
            "run-mt007-post-checkpoint-crdt",
            Some("lane-mt007-post-checkpoint-crdt"),
            None,
            None,
            crdt_event.event_ledger_seq,
            vec![],
        ))
        .await
        .expect("record CRDT checkpoint before repair");
    let message = sample_message(
        "msg-mt007-post-checkpoint-crdt-repair",
        "run-mt007-post-checkpoint-crdt",
        "lane-mt007-post-checkpoint-crdt",
    );
    store
        .record_message(message)
        .await
        .expect("record CRDT-valid message after checkpoint");
    assert_recovery_failure(
        &store,
        "run-mt007-post-checkpoint-crdt",
        ModelLaneRecoveryFailureKind::StaleCrdtBase,
    )
    .await;
}

async fn assert_recovery_failure(
    store: &ModelLaneStore,
    run_id: &str,
    failure: ModelLaneRecoveryFailureKind,
) {
    let err = store
        .recover_run_after_restart(run_id)
        .await
        .expect_err("recovery must fail closed");
    assert!(
        err.to_string().contains(failure.code()),
        "expected {}, got {err}",
        failure.code()
    );
}

async fn assert_recovery_integrity_failure(store: &ModelLaneStore, run_id: &str) {
    let err = store
        .recover_run_after_restart(run_id)
        .await
        .expect_err("recovery must fail closed on mutable row drift");
    let message = err.to_string();
    assert!(
        message.contains(ModelLaneRecoveryFailureKind::CorruptCheckpoint.code())
            || message.contains(ModelLaneRecoveryFailureKind::MissingEventLedgerRow.code())
            || message.to_ascii_lowercase().contains("integrity")
            || message.to_ascii_lowercase().contains("eventledger"),
        "expected recovery/integrity failure, got {err}"
    );
}

async fn recovery_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-007 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated model-lane recovery schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

async fn event_stream_high_watermark(pool: &sqlx::PgPool, event_ledger_stream_id: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(MAX(event_sequence), 0) \
         FROM kernel_event_ledger \
         WHERE session_run_id = $1",
    )
    .bind(event_ledger_stream_id)
    .fetch_one(pool)
    .await
    .expect("query EventLedger stream high-watermark")
}

async fn seed_run_lane_message(
    store: &ModelLaneStore,
    run_id: &str,
    lane_id: &str,
    message_id: &str,
) {
    store
        .record_run(sample_run(run_id, lane_id))
        .await
        .expect("record run");
    store
        .record_lane(sample_lane(
            lane_id,
            run_id,
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
        ))
        .await
        .expect("record lane");
    let message = sample_message(message_id, run_id, lane_id);
    store
        .record_message(message.clone())
        .await
        .expect("record message");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&message))
        .await
        .expect("record artifact binding");
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
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-007".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-{run_id}"),
        replay_order_key: "00000001/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/recovery".into()),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("coordinator-{run_id}"),
            &format!("model-session-coordinator-{run_id}"),
        )),
        memory_pack_ref: format!("memory-pack://fems/{run_id}"),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt007".into(),
        selected_model_id: Some("model://mt007/local".into()),
        candidate_model_ids: vec!["model://mt007/local".into()],
        procedural_review_status: "reviewed_by_kernel_builder".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn sample_lane(
    lane_id: &str,
    run_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
) -> NewModelLane {
    let provider_kind = match runtime_binding {
        RuntimeBinding::Local => ModelLaneProviderKind::LocalRuntime,
        RuntimeBinding::Cloud => ModelLaneProviderKind::OpenAi,
        RuntimeBinding::CliBridge => ModelLaneProviderKind::OfficialCli,
        RuntimeBinding::Human => ModelLaneProviderKind::Human,
        RuntimeBinding::Subagent => ModelLaneProviderKind::Subagent,
        RuntimeBinding::Validator => ModelLaneProviderKind::Validator,
    };
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: run_id.into(),
        trace_id: format!("trace-{run_id}"),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        kind,
        role: format!("role-{lane_id}"),
        backend: runtime_binding.as_str().into(),
        model_id: Some("model://mt007/local".into()),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding,
        launch_authority,
        provider_kind,
        capability_token_ids: vec!["capability://mt007/read".into()],
        effective_capability_snapshot_ref: Some("capability-snapshot://mt007".into()),
        capability_negotiation_ref: Some(format!("capability-negotiation://mt007/{lane_id}")),
        provider_feature_profile_ref: Some("provider-feature-profile://mt007".into()),
        requested_execution_policy_ref: Some("execution-policy://requested/mt007".into()),
        effective_execution_policy_ref: Some("execution-policy://effective/mt007".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["toolgate://mt007/allow".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-30T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-01-01T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-01-01T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://mt007/{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt007".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt007".into()),
        process_ownership_ref: Some(format!("process-ledger://mt007/{lane_id}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt007".into()),
        last_runtime_status_ref: Some("runtime-status://mt007/ready".into()),
        last_recovery_event_ref: Some("recovery://mt007/startable".into()),
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/recovery#lane".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-007".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
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
        payload_sha256: artifact_payload_hash(message_id),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        summary: "MT-007 recovery payload".into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt007/allow".into()],
        coordinator_session_id: format!("coordinator-{run_id}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-007".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        locus_binding: Some(sample_locus(
            run_id,
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
        idempotency_key: format!("idem-{message_id}"),
        replay_order_key: "00000002/message".into(),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: Some(format!("proposal://mt007/{message_id}")),
        crdt_update_ref: Some(format!("crdt-update://mt007/{message_id}")),
        crdt_base_snapshot_ref: Some("crdt-snapshot://mt007/base".into()),
        crdt_state_vector: Some("sv:1".into()),
        crdt_proposal_ref: Some(format!("crdt-proposal://mt007/{message_id}")),
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://dexterity/recovery#message".into()),
        created_at_utc: "2026-06-30T00:00:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "kernel_event_ledger",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "deferred_external_worktree"
        }),
    }
}

fn sample_recovery_event(
    event_id: &str,
    run_id: &str,
    lane_id: Option<&str>,
    kind: ModelLaneRecoveryEventKind,
    replay_order_seq: i64,
    payload_ref: Option<String>,
    crdt_stale_base_ref: Option<String>,
) -> NewModelLaneRecoveryEvent {
    NewModelLaneRecoveryEvent {
        recovery_event_id: event_id.into(),
        run_id: run_id.into(),
        lane_id: lane_id.map(str::to_string),
        trace_id: format!("trace-{run_id}"),
        span_id: format!("span-{event_id}"),
        parent_span_id: lane_id.map(|lane| format!("span-{lane}")),
        linked_span_contexts: vec![format!("trace-link://{run_id}/{event_id}")],
        session_id: lane_id.map(|lane| format!("session-{lane}")),
        model_session_id: lane_id.map(|lane| format!("model-session-{lane}")),
        event_kind: kind,
        recovery_status: ModelLaneRecoveryStatus::Observed,
        replay_order_seq,
        source_event_ledger_seq: None,
        payload_refs: payload_ref.into_iter().collect(),
        artifact_refs: vec![],
        crdt_base_snapshot_ref: Some("crdt-snapshot://mt007/base".into()),
        crdt_state_vector: Some("sv:1".into()),
        crdt_stale_base_ref,
        lease_id: None,
        failure_kind: None,
        error_code: None,
        replay_hint: "Replay from PostgreSQL/EventLedger checkpoint before provider/chat history"
            .into(),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-{event_id}"),
        recovery_hint_ref: Some("usermanual://dexterity/recovery#event".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics"}),
    }
}

fn sample_locus(run_id: &str, session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-{run_id}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        locus_binding_ref: format!("locus://wp1/mt007/{run_id}/{session_id}"),
    }
}

fn sample_checkpoint(
    checkpoint_id: &str,
    run_id: &str,
    lane_id: Option<&str>,
    last_message_id: Option<&str>,
    lease_id: Option<&str>,
    last_event_ledger_seq: i64,
    open_payload_refs: Vec<String>,
) -> NewModelLaneRecoveryCheckpoint {
    NewModelLaneRecoveryCheckpoint {
        checkpoint_id: checkpoint_id.into(),
        run_id: run_id.into(),
        lane_id: lane_id.map(str::to_string),
        session_id: lane_id
            .map(|lane| format!("session-{lane}"))
            .unwrap_or_else(|| format!("coordinator-{run_id}")),
        model_session_id: lane_id
            .map(|lane| format!("model-session-{lane}"))
            .unwrap_or_else(|| format!("model-session-coordinator-{run_id}")),
        lane_status: ModelLaneStatus::Ready,
        checkpoint_status: ModelLaneRecoveryStatus::Checkpointed,
        last_event_ledger_seq,
        last_message_id: last_message_id.map(str::to_string),
        open_payload_refs,
        lease_id: lease_id.map(str::to_string),
        idempotency_scope: format!("model-lane-recovery:{run_id}:{checkpoint_id}"),
        recovery_state: ModelLaneRecoveryState::Restartable,
        recovery_event_ref: Some(format!("recovery-event://{checkpoint_id}")),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-{checkpoint_id}"),
        created_at_utc: "2026-06-30T00:00:00Z".into(),
        recovery_hint_ref: Some("usermanual://dexterity/recovery#checkpoint".into()),
        diagnostic_payload: json!({"tier": "internal_diagnostics"}),
    }
}

fn sample_lease(
    lease_id: &str,
    run_id: &str,
    lane_id: &str,
    lease_expires_at_utc: &str,
    state: ModelLaneLeaseState,
) -> NewModelLaneLease {
    NewModelLaneLease {
        lease_id: lease_id.into(),
        run_id: run_id.into(),
        lane_id: Some(lane_id.into()),
        scope: ModelLaneLeaseScope::Lane,
        scope_ref: format!("model-lane://{run_id}/{lane_id}"),
        holder_actor_id: "actor://kernel-builder/mt007".into(),
        holder_session_id: "KERNEL_BUILDER-20260630-045713".into(),
        lease_expires_at_utc: lease_expires_at_utc.into(),
        takeover_policy_ref: "lease-policy://mt007/recover-or-reclaim".into(),
        state,
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-{lease_id}"),
        recovery_hint_ref: Some("usermanual://dexterity/recovery#lease".into()),
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
        diagnostic_status_id: format!("diag-{run_id}-HBR-INT-009-{}", tier.as_str()),
        behavior_id: "HBR-INT-009".into(),
        run_id: run_id.into(),
        tier,
        state,
        reason: format!("MT-007 recovery diagnostic posture for {run_id}"),
        evidence_ref: evidence_ref.into(),
        follow_up_ref: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-007".into()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-diag-{run_id}-HBR-INT-009-{}", tier.as_str()),
        diagnostic_payload: json!({"behavior_id": "HBR-INT-009", "run_id": run_id}),
    }
}

fn sample_mt_status(
    status_id: &str,
    run_id: &str,
    micro_task_id: &str,
    status: ModelLaneMtRuntimeStatus,
) -> NewModelLaneMtRuntimeStatus {
    NewModelLaneMtRuntimeStatus {
        mt_status_id: status_id.into(),
        run_id: run_id.into(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: micro_task_id.into(),
        task_board_id: "task-board://wp-1".into(),
        status,
        claimed_by_ref: Some("session://KERNEL_BUILDER-20260630-045713".into()),
        blocker_ref: None,
        missing_resource_ref: None,
        proof_status_ref: Some("proof://mt007/model_lane_recovery_pg_tests".into()),
        hbr_status_ref: Some("hbr-int-009://dexterity/recovery/details".into()),
        last_recovery_event_ref: Some("recovery-event://recovery-event-mt007-001".into()),
        last_runtime_status_ref: Some("runtime-status://mt007/ready-for-validation".into()),
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-{status_id}"),
        diagnostic_payload: json!({"state_recovery": true}),
    }
}

fn sample_artifact_binding_for_message(
    message: &NewModelLaneMessage,
) -> NewModelLaneContextBundleArtifactBinding {
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-{}", message.message_id),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: message.payload_sha256.clone(),
        content_hash: message.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!(
            "artifact-store://model-lane/mt007/{}/artifact.json",
            message.message_id
        ),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json: artifact_payload_json(&message.message_id),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-artifact-binding-{}", message.message_id),
        created_at_utc: "2026-06-30T00:00:01Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for MT-007 recovery",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "deferred_external_worktree"
        }),
    }
}

fn sample_artifact_binding(
    artifact_binding_id: &str,
    run_id: &str,
    trace_id: &str,
    artifact_ref: &str,
) -> NewModelLaneContextBundleArtifactBinding {
    let payload_json = json!({
        "schema_id": "hsk.model_lane_message_payload@1",
        "artifact_ref": artifact_ref,
        "body": format!("deterministic MT-007 post-checkpoint repair for {artifact_ref}")
    });
    let payload_sha256 = sha256_hex(&canonical_json_bytes(&payload_json));
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: artifact_binding_id.into(),
        run_id: run_id.into(),
        trace_id: trace_id.into(),
        artifact_ref: artifact_ref.into(),
        artifact_sha256: payload_sha256.clone(),
        content_hash: payload_sha256,
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!(
            "artifact-store://model-lane/mt007/{artifact_binding_id}/artifact.json"
        ),
        artifact_payload_ref: artifact_ref.into(),
        payload_json,
        event_ledger_stream_id: format!("mlane-stream-{run_id}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-007".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-20260630-045713".into(),
        idempotency_key: format!("idem-{artifact_binding_id}"),
        created_at_utc: "2026-06-30T00:00:02Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for MT-007 recovery",
            "internal_diagnostics": "hbr-int-009",
            "palmistry": "deferred_external_worktree"
        }),
    }
}

fn artifact_payload_json(message_id: &str) -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_message_payload@1",
        "message_id": message_id,
        "body": format!("deterministic MT-007 payload for {message_id}")
    })
}

fn artifact_payload_hash(message_id: &str) -> String {
    sha256_hex(&canonical_json_bytes(&artifact_payload_json(message_id)))
}

fn sample_sha256() -> String {
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn canonical_json_bytes(value: &serde_json::Value) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

fn write_canonical_json(output: &mut String, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => output.push_str("null"),
        serde_json::Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        serde_json::Value::Number(value) => output.push_str(&value.to_string()),
        serde_json::Value::String(value) => {
            output.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    c => output.push(c),
                }
            }
            output.push('"');
        }
        serde_json::Value::Array(values) => {
            output.push('[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        serde_json::Value::Object(map) => {
            output.push('{');
            for (index, (key, item)) in map.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, &serde_json::Value::String(key.clone()));
                output.push(':');
                write_canonical_json(output, item);
            }
            output.push('}');
        }
    }
}
