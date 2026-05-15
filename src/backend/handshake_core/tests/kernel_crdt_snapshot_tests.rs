use handshake_core::kernel::crdt::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};
use handshake_core::kernel::crdt::persistence::{
    new_crdt_update_record, CrdtReplayMetadataV1, CrdtStorageAuthorityPosture,
    CrdtUpdateRecordInputV1, CrdtUpdateRecordV1,
};
use handshake_core::kernel::crdt::snapshot::{
    build_snapshot_bounded_replay_plan, new_crdt_snapshot_record, plan_crdt_compaction,
    validate_crdt_snapshot_record, CrdtCompactionAuditMode, CrdtCompactionDisposition,
    CrdtCompactionPolicyV1, CrdtSnapshotRecordInputV1, CrdtSnapshotReplayError,
};

#[test]
fn kernel_crdt_snapshot_record_carries_state_vector_hash_and_postgres_authority() {
    let snapshot = sample_snapshot(3, &["crdt-update-2"]);

    assert_eq!(snapshot.schema_id, "hsk.kernel.crdt_snapshot_record@1");
    assert_eq!(snapshot.covered_update_seq, 3);
    assert_eq!(snapshot.state_vector, "sv-3");
    assert_eq!(
        snapshot.storage_authority,
        CrdtStorageAuthorityPosture::PostgresEventLedger
    );
    assert!(snapshot.snapshot_bytes_ref.starts_with("postgres://"));
    assert_eq!(
        snapshot.promotion_evidence_update_ids,
        vec!["crdt-update-2".to_string()]
    );

    validate_crdt_snapshot_record(&snapshot).expect("snapshot record must validate");
}

#[test]
fn kernel_crdt_snapshot_bounds_replay_to_updates_after_snapshot_cursor() {
    let snapshot = sample_snapshot(3, &["crdt-update-2"]);
    let updates = vec![
        sample_update(1, "crdt-update-1", b"update-1", "sv-0", "sv-1"),
        sample_update(2, "crdt-update-2", b"update-2", "sv-1", "sv-2"),
        sample_update(3, "crdt-update-3", b"update-3", "sv-2", "sv-3"),
        sample_update(5, "crdt-update-5", b"update-5", "sv-4", "sv-5"),
        sample_update(4, "crdt-update-4", b"update-4", "sv-3", "sv-4"),
    ];

    let plan = build_snapshot_bounded_replay_plan(&snapshot, &updates)
        .expect("snapshot must bound replay");

    assert_eq!(plan.base_snapshot_id, "snapshot-3");
    assert_eq!(plan.base_snapshot_state_vector, "sv-3");
    assert_eq!(plan.replay_from_update_seq, 4);
    assert_eq!(plan.ordered_updates.len(), 2);
    assert_eq!(plan.ordered_updates[0].update_seq, 4);
    assert_eq!(plan.ordered_updates[1].update_seq, 5);
    assert_eq!(plan.final_state_vector, "sv-5");
}

#[test]
fn kernel_crdt_snapshot_replay_rejects_gap_after_snapshot_cursor() {
    let snapshot = sample_snapshot(3, &["crdt-update-2"]);
    let updates = vec![
        sample_update(1, "crdt-update-1", b"update-1", "sv-0", "sv-1"),
        sample_update(3, "crdt-update-3", b"update-3", "sv-2", "sv-3"),
        sample_update(5, "crdt-update-5", b"update-5", "sv-4", "sv-5"),
    ];

    let error = build_snapshot_bounded_replay_plan(&snapshot, &updates)
        .expect_err("gap after snapshot must fail bounded replay");
    assert!(matches!(
        error,
        CrdtSnapshotReplayError::SequenceGap {
            expected: 4,
            found: 5
        }
    ));
}

#[test]
fn kernel_crdt_compaction_compacts_old_updates_but_retains_promotion_evidence() {
    let snapshot = sample_snapshot(3, &["crdt-update-2"]);
    let updates = vec![
        sample_update(1, "crdt-update-1", b"update-1", "sv-0", "sv-1"),
        sample_update(2, "crdt-update-2", b"update-2", "sv-1", "sv-2"),
        sample_update(3, "crdt-update-3", b"update-3", "sv-2", "sv-3"),
        sample_update(4, "crdt-update-4", b"update-4", "sv-3", "sv-4"),
    ];

    let plan = plan_crdt_compaction(&snapshot, &updates, &sample_policy())
        .expect("compaction plan must preserve audit and promotion evidence");

    let promotion = plan
        .decisions
        .iter()
        .find(|decision| decision.update_id == "crdt-update-2")
        .expect("promotion evidence update must have decision");
    assert_eq!(
        promotion.disposition,
        CrdtCompactionDisposition::RetainPromotionEvidence
    );
    assert!(plan.decisions.iter().any(|decision| {
        decision.update_id == "crdt-update-1"
            && decision.disposition == CrdtCompactionDisposition::CompactWithAudit
            && decision.audit_ref.starts_with("eventledger://")
    }));
    assert!(plan.decisions.iter().any(|decision| {
        decision.update_id == "crdt-update-4"
            && decision.disposition == CrdtCompactionDisposition::RetainForReplay
    }));
}

#[test]
fn kernel_crdt_compaction_policy_cannot_drop_promotion_evidence() {
    let snapshot = sample_snapshot(3, &["crdt-update-2"]);
    let updates = vec![sample_update(
        2,
        "crdt-update-2",
        b"update-2",
        "sv-1",
        "sv-2",
    )];
    let mut unsafe_policy = sample_policy();
    unsafe_policy.preserve_promotion_evidence = false;

    let error = plan_crdt_compaction(&snapshot, &updates, &unsafe_policy)
        .expect_err("unsafe compaction policy must fail");
    assert!(matches!(
        error,
        CrdtSnapshotReplayError::PromotionEvidenceWouldBeDropped {
            update_id
        } if update_id == "crdt-update-2"
    ));
}

fn sample_snapshot(
    covered_update_seq: u64,
    promotion_evidence_update_ids: &[&str],
) -> handshake_core::kernel::crdt::snapshot::CrdtSnapshotRecordV1 {
    let identity = sample_identity();
    new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity: &identity,
        snapshot_id: &format!("snapshot-{covered_update_seq}"),
        covered_update_seq,
        snapshot_bytes: format!("snapshot-state-{covered_update_seq}").as_bytes(),
        snapshot_bytes_ref: &format!(
            "postgres://kernel_crdt_snapshots/snapshot-{covered_update_seq}/snapshot_bytes"
        ),
        state_vector: &format!("sv-{covered_update_seq}"),
        event_ledger_event_id: &format!("evt-snapshot-{covered_update_seq}"),
        promotion_evidence_update_ids,
    })
}

fn sample_update(
    update_seq: u64,
    update_id: &str,
    update_bytes: &[u8],
    state_vector_before: &str,
    state_vector_after: &str,
) -> CrdtUpdateRecordV1 {
    let identity = sample_identity();
    new_crdt_update_record(CrdtUpdateRecordInputV1 {
        identity: &identity,
        update_id,
        update_seq,
        update_bytes,
        update_bytes_ref: &format!("postgres://kernel_crdt_updates/{update_id}/update_bytes"),
        session_id: "session-kernel-builder",
        trace_id: &format!("trace-{update_id}"),
        state_vector_before,
        state_vector_after,
        replay_metadata: CrdtReplayMetadataV1 {
            replay_order_key: format!("workspace-kernel/document-kernel/{update_seq:020}"),
            dependency_update_ids: Vec::new(),
            encoding: "yjs-update-v1".to_string(),
            schema_version: "kernel-crdt-update-v1".to_string(),
        },
        event_ledger_event_id: &format!("evt-{update_id}"),
    })
}

fn sample_policy() -> CrdtCompactionPolicyV1 {
    CrdtCompactionPolicyV1 {
        policy_id: "kernel-crdt-compaction-policy-v1".to_string(),
        compact_through_update_seq: 3,
        audit_mode: CrdtCompactionAuditMode::EventLedgerAuditRefs,
        preserve_promotion_evidence: true,
    }
}

fn sample_identity() -> CrdtWorkspaceIdentityV1 {
    CrdtWorkspaceIdentityV1 {
        schema_id: "hsk.kernel.crdt_workspace_identity@1".to_string(),
        workspace_id: "workspace-kernel".to_string(),
        document_id: "document-kernel".to_string(),
        crdt_document_id: "crdt-document-kernel".to_string(),
        actor_id: "actor-kernel-builder".to_string(),
        actor_kind: "model".to_string(),
        crdt_site_id: "site-kernel-builder".to_string(),
        crdt_client_id: "client-kernel-builder".to_string(),
        document_schema_id: "hsk.kernel.crdt_document@1".to_string(),
        authority_links: CrdtAuthorityLinksV1 {
            work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-012"
                .to_string(),
            action_trace_id: "trace-crdt-workspace".to_string(),
            artifact_proposal_id: "artifact-proposal-crdt".to_string(),
            role_mailbox_thread_id: "role-mailbox-thread-crdt".to_string(),
            dcc_projection_id: "dcc-crdt-projection".to_string(),
            event_ledger_stream_id: "event-ledger-stream-crdt".to_string(),
        },
    }
}
