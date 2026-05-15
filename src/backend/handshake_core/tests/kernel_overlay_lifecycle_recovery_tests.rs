use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    overlay_lifecycle_recovery::{
        query_overlay_lifecycle_recovery_posture, validate_overlay_lifecycle_recovery_record,
        CheckpointRefV1, CheckpointStatus, ControlActionKind, GovernedActionLineageV1,
        LifecycleRecoverySourceKind, LifecycleState, OverlayLifecycleRecoveryRecordV1,
        PartialFailureKind, RecoveryPosture,
    },
};

#[test]
fn lifecycle_recovery_record_exposes_control_actions_and_checkpoint_lineage() {
    let record = sample_record(LifecycleRecoverySourceKind::ProductLifecycleRecord, 4);

    validate_overlay_lifecycle_recovery_record(&record).expect("lifecycle record must validate");

    assert_eq!(record.lifecycle_state, LifecycleState::Running);
    assert_eq!(record.recovery_posture, RecoveryPosture::ReplayReady);
    for action in [
        ControlActionKind::Start,
        ControlActionKind::Steer,
        ControlActionKind::Cancel,
        ControlActionKind::Close,
        ControlActionKind::Recover,
        ControlActionKind::CheckpointReplay,
        ControlActionKind::Restart,
    ] {
        assert!(
            record.available_control_actions.contains(&action),
            "missing control action: {action:?}"
        );
    }
    assert_eq!(
        record.checkpoints[0].status,
        CheckpointStatus::ReplayValidated
    );
    assert!(record.restart_safe);
    assert!(record.projection_safe);
    assert!(record.folded_source_refs.iter().any(|source| source
        .contains("WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1")));
}

#[test]
fn lifecycle_validation_rejects_transcript_packet_and_ui_local_state_as_authority() {
    for source_kind in [
        LifecycleRecoverySourceKind::TranscriptHistory,
        LifecycleRecoverySourceKind::PacketEdit,
        LifecycleRecoverySourceKind::UiLocalState,
    ] {
        let record = sample_record(source_kind, 99);
        let errors = validate_overlay_lifecycle_recovery_record(&record)
            .expect_err("non-authority lifecycle source must fail");

        assert!(
            errors
                .iter()
                .any(|error| error.field == "source_kind"
                    && error.message.contains("not authoritative")),
            "expected source_kind denial for {source_kind:?}, got {errors:?}"
        );
    }
}

#[test]
fn lifecycle_posture_query_uses_latest_valid_record_and_ignores_transcript_history() {
    let old_record = sample_record(LifecycleRecoverySourceKind::ProductLifecycleRecord, 1);
    let mut latest_record = sample_record(LifecycleRecoverySourceKind::CheckpointLineageRecord, 7);
    latest_record.lifecycle_state = LifecycleState::Restarting;
    latest_record.partial_failure = PartialFailureKind::ActorCrash;
    let transcript_history = sample_record(LifecycleRecoverySourceKind::TranscriptHistory, 99);

    let projection = query_overlay_lifecycle_recovery_posture(
        &[old_record, transcript_history, latest_record],
        "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-021",
    )
    .expect("posture should project from valid lifecycle records");

    assert_eq!(projection.current_record.record_seq, 7);
    assert_eq!(projection.lifecycle_state, LifecycleState::Restarting);
    assert_eq!(projection.partial_failure, PartialFailureKind::ActorCrash);
    assert_eq!(
        projection.ignored_non_authority_source_kinds,
        vec![LifecycleRecoverySourceKind::TranscriptHistory]
    );
    assert!(projection
        .replay_checkpoint_ids
        .contains(&"checkpoint-002".to_string()));
}

#[test]
fn kernel_action_catalog_exposes_lifecycle_recovery_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.overlay_lifecycle.project")
        .expect("overlay lifecycle projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "checkpoint_lineage"));
    assert!(action
        .dcc_preview
        .primary_state_fields
        .contains(&"recovery_posture".to_string()));
}

fn sample_record(
    source_kind: LifecycleRecoverySourceKind,
    record_seq: u64,
) -> OverlayLifecycleRecoveryRecordV1 {
    OverlayLifecycleRecoveryRecordV1 {
        schema_id: "hsk.kernel.overlay_lifecycle_recovery_record@1".to_string(),
        lifecycle_id: format!("lifecycle-mt021-{record_seq}"),
        work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-021"
            .to_string(),
        record_seq,
        lifecycle_state: LifecycleState::Running,
        recovery_posture: RecoveryPosture::ReplayReady,
        available_control_actions: vec![
            ControlActionKind::Start,
            ControlActionKind::Steer,
            ControlActionKind::Cancel,
            ControlActionKind::Close,
            ControlActionKind::Recover,
            ControlActionKind::CheckpointReplay,
            ControlActionKind::Restart,
        ],
        checkpoints: vec![
            CheckpointRefV1 {
                checkpoint_id: "checkpoint-001".to_string(),
                sequence: 1,
                state_hash: "sha256:checkpoint-001".to_string(),
                event_ledger_ref: "event-ledger://checkpoint/001".to_string(),
                status: CheckpointStatus::ReplayValidated,
            },
            CheckpointRefV1 {
                checkpoint_id: "checkpoint-002".to_string(),
                sequence: 2,
                state_hash: "sha256:checkpoint-002".to_string(),
                event_ledger_ref: "event-ledger://checkpoint/002".to_string(),
                status: CheckpointStatus::ReplayValidated,
            },
        ],
        partial_failure: PartialFailureKind::None,
        restart_safe: true,
        projection_safe: true,
        governed_action_lineage: vec![GovernedActionLineageV1 {
            action_id: "kernel.workflow_transition.preview".to_string(),
            trace_id: "trace-mt021".to_string(),
            result_id: "result-mt021".to_string(),
        }],
        source_kind,
        evidence_refs: vec!["receipt://KERNEL_BUILDER-20260514-130219/MT-021".to_string()],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1.contract.json".to_string(),
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Lifecycle-Recovery-Control-Plane-v1.md".to_string(),
        ],
    }
}
