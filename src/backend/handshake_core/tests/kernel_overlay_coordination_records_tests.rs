use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    overlay_coordination_records::{
        query_overlay_coordination_posture, validate_overlay_coordination_record,
        ActorEligibilityV1, ClaimMode, CoordinationActorRefV1, CoordinationSourceKind,
        InstructionKind, InstructionStatus, LeasePosture, OverlayCoordinationJoinsV1,
        OverlayCoordinationRecordV1, QueuedCoordinationInstructionV1, TakeoverLegality,
    },
};

#[test]
fn overlay_coordination_record_exposes_claim_lease_followup_and_takeover_by_stable_ids() {
    let record = sample_record(CoordinationSourceKind::ProductCoordinationRecord, 4);

    validate_overlay_coordination_record(&record).expect("coordination record must validate");

    assert_eq!(record.claim_mode, ClaimMode::Lease);
    assert_eq!(record.lease_posture, LeasePosture::Active);
    assert_eq!(record.takeover_legality, TakeoverLegality::Allowed);
    assert!(record
        .queued_instructions
        .iter()
        .any(|instruction| instruction.kind == InstructionKind::QueuedSteering));
    assert!(record
        .queued_instructions
        .iter()
        .any(|instruction| instruction.kind == InstructionKind::FollowUp));
    assert_eq!(
        record.joins.role_mailbox_thread_id,
        "role-mailbox-thread-mt020"
    );
    assert!(record
        .folded_source_refs
        .iter()
        .any(|source| source.contains("WP-1-Software-Delivery-Overlay-Coordination-Records-v1")));
}

#[test]
fn coordination_validation_rejects_mailbox_chronology_comments_and_transcript_order() {
    for source_kind in [
        CoordinationSourceKind::MailboxChronology,
        CoordinationSourceKind::AdvisoryComment,
        CoordinationSourceKind::TranscriptOrder,
    ] {
        let record = sample_record(source_kind, 99);
        let errors = validate_overlay_coordination_record(&record)
            .expect_err("non-authority coordination source must fail");

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
fn coordination_posture_query_uses_latest_valid_record_and_ignores_mailbox_order() {
    let old_record = sample_record(CoordinationSourceKind::ProductCoordinationRecord, 2);
    let latest_record = sample_record(CoordinationSourceKind::WorkflowStateRecord, 8);
    let mailbox_order = sample_record(CoordinationSourceKind::MailboxChronology, 99);

    let projection = query_overlay_coordination_posture(
        &[old_record, mailbox_order, latest_record],
        "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-020",
    )
    .expect("posture should query from valid coordination records");

    assert_eq!(projection.current_record.record_seq, 8);
    assert_eq!(
        projection.ignored_non_authority_source_kinds,
        vec![CoordinationSourceKind::MailboxChronology]
    );
    assert_eq!(
        projection.pending_instruction_ids,
        vec![
            "queued-steering-mt020".to_string(),
            "follow-up-mt020".to_string()
        ]
    );
    assert!(projection
        .eligible_next_actor_kinds
        .contains(&"KERNEL_BUILDER".to_string()));
}

#[test]
fn kernel_action_catalog_exposes_overlay_coordination_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.overlay_coordination.project")
        .expect("overlay coordination projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "coordination_source_kind"));
    assert!(action
        .dcc_preview
        .primary_state_fields
        .contains(&"takeover_legality".to_string()));
}

fn sample_record(
    source_kind: CoordinationSourceKind,
    record_seq: u64,
) -> OverlayCoordinationRecordV1 {
    OverlayCoordinationRecordV1 {
        schema_id: "hsk.kernel.overlay_coordination_record@1".to_string(),
        coordination_id: format!("coordination-mt020-{record_seq}"),
        work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-020"
            .to_string(),
        record_seq,
        claimant: CoordinationActorRefV1 {
            actor_id: "actor-kernel-builder".to_string(),
            actor_kind: "KERNEL_BUILDER".to_string(),
            role_id: "CODER".to_string(),
        },
        claim_mode: ClaimMode::Lease,
        lease_id: "lease-mt020".to_string(),
        lease_posture: LeasePosture::Active,
        takeover_legality: TakeoverLegality::Allowed,
        next_actor_kinds: vec!["KERNEL_BUILDER".to_string(), "CODER".to_string()],
        actor_eligibility: vec![ActorEligibilityV1 {
            actor_kind: "KERNEL_BUILDER".to_string(),
            eligible: true,
            reason: "active lease holder".to_string(),
            transition_rule_ids: vec!["kernel.mt.complete".to_string()],
        }],
        queued_instructions: vec![
            QueuedCoordinationInstructionV1 {
                instruction_id: "queued-steering-mt020".to_string(),
                kind: InstructionKind::QueuedSteering,
                status: InstructionStatus::Pending,
                target_actor_kind: "KERNEL_BUILDER".to_string(),
                governed_action_id: "kernel.workflow_transition.preview".to_string(),
                stable_source_id: "steering-record-mt020".to_string(),
            },
            QueuedCoordinationInstructionV1 {
                instruction_id: "follow-up-mt020".to_string(),
                kind: InstructionKind::FollowUp,
                status: InstructionStatus::Pending,
                target_actor_kind: "CODER".to_string(),
                governed_action_id: "kernel.overlay_coordination.project".to_string(),
                stable_source_id: "follow-up-record-mt020".to_string(),
            },
        ],
        source_kind,
        joins: OverlayCoordinationJoinsV1 {
            work_packet_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
                .to_string(),
            task_board_item_id: "TASKBOARD-WP-KERNEL-002".to_string(),
            role_mailbox_thread_id: "role-mailbox-thread-mt020".to_string(),
            dcc_projection_id: "dcc-overlay-coordination-mt020".to_string(),
            workflow_state_id: "workflow-state-mt020".to_string(),
        },
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Coordination-Records-v1.contract.json".to_string(),
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Overlay-Coordination-Records-v1.md".to_string(),
        ],
    }
}
