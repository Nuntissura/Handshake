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
use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::storage::{tests::postgres_backend_from_env, StorageError};
use serde_json::json;
use uuid::Uuid;

async fn postgres_or_environment_blocked() -> std::sync::Arc<dyn handshake_core::storage::Database>
{
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!("ENVIRONMENT_BLOCKED: Kernel002 CRDT snapshot tests require POSTGRES_TEST_URL; {msg}");
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

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

#[tokio::test]
async fn kernel_crdt_snapshot_persists_in_postgres_and_bounds_replay_after_restart() {
    let db = postgres_or_environment_blocked().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let mut update =
        sample_update_for_workspace(&suffix, 4, "crdt-update-4", b"update-4", "sv-3", "sv-4");
    let mut snapshot = sample_snapshot_for_workspace(&suffix, 3, &["crdt-update-2"]);
    snapshot.event_ledger_event_id =
        append_kernel_crdt_event(db.as_ref(), &suffix, &snapshot.snapshot_id, "snapshot").await;
    update.event_ledger_event_id =
        append_kernel_crdt_event(db.as_ref(), &suffix, &update.update_id, "update").await;

    db.append_kernel_crdt_snapshot(snapshot.clone(), b"snapshot-state-3".to_vec())
        .await
        .expect("append CRDT snapshot to Postgres");
    db.append_kernel_crdt_update(update.clone(), b"update-4".to_vec())
        .await
        .expect("append CRDT update after snapshot");

    let snapshots = db
        .list_kernel_crdt_snapshots(
            &snapshot.workspace_id,
            &snapshot.document_id,
            &snapshot.crdt_document_id,
        )
        .await
        .expect("list persisted CRDT snapshots");
    let updates = db
        .list_kernel_crdt_updates(
            &update.workspace_id,
            &update.document_id,
            &update.crdt_document_id,
        )
        .await
        .expect("list persisted CRDT updates");
    let bounded = build_snapshot_bounded_replay_plan(&snapshots[0], &updates)
        .expect("persisted snapshot bounds replay");

    assert_eq!(snapshots[0].snapshot_id, snapshot.snapshot_id);
    assert_eq!(bounded.base_snapshot_state_vector, "sv-3");
    assert_eq!(bounded.final_state_vector, "sv-4");
    assert_eq!(bounded.ordered_updates[0].update_id, "crdt-update-4");
    assert_eq!(
        db.read_kernel_crdt_snapshot_bytes(&snapshot.snapshot_bytes_ref)
            .await
            .expect("read CRDT snapshot bytes"),
        b"snapshot-state-3".to_vec()
    );
    assert_eq!(
        db.read_kernel_crdt_update_bytes(&update.update_bytes_ref)
            .await
            .expect("read post-snapshot CRDT update bytes"),
        b"update-4".to_vec()
    );
}

#[tokio::test]
async fn kernel_crdt_snapshot_persistence_rejects_missing_eventledger_ref() {
    let db = postgres_or_environment_blocked().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let missing_event = sample_snapshot_for_workspace(&suffix, 3, &["crdt-update-2"]);

    let error = db
        .append_kernel_crdt_snapshot(missing_event, b"snapshot-state-3".to_vec())
        .await
        .expect_err("CRDT snapshots must cite an existing EventLedger event");
    assert!(matches!(
        error,
        StorageError::Validation(message)
            if message.contains("kernel CRDT EventLedger event ref is missing")
    ));
}

async fn append_kernel_crdt_event(
    db: &(dyn handshake_core::storage::Database + '_),
    suffix: &str,
    item_id: &str,
    item_kind: &str,
) -> String {
    let event = NewKernelEvent::builder(
        format!("KTR-CRDT-SNAPSHOT-{suffix}"),
        format!("SR-CRDT-SNAPSHOT-{suffix}"),
        KernelEventType::ArtifactStored,
        KernelActor::System("kernel-crdt-snapshot-test".to_string()),
    )
    .aggregate(format!("kernel_crdt_{item_kind}"), item_id.to_string())
    .idempotency_key(format!("kernel-crdt-{item_kind}:{suffix}:{item_id}"))
    .source_component("kernel_crdt_snapshot_test")
    .payload(json!({
        "suffix": suffix,
        "item_id": item_id,
        "item_kind": item_kind
    }))
    .build()
    .expect("valid CRDT snapshot EventLedger event");

    db.append_kernel_event(event)
        .await
        .expect("append CRDT snapshot EventLedger event")
        .event_id
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

fn sample_snapshot_for_workspace(
    suffix: &str,
    covered_update_seq: u64,
    promotion_evidence_update_ids: &[&str],
) -> handshake_core::kernel::crdt::snapshot::CrdtSnapshotRecordV1 {
    let mut identity = sample_identity();
    identity.workspace_id = format!("workspace-kernel-{suffix}");
    identity.document_id = format!("document-kernel-{suffix}");
    identity.crdt_document_id = format!("crdt-document-kernel-{suffix}");
    identity.authority_links.event_ledger_stream_id = format!("event-ledger-stream-{suffix}");
    new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity: &identity,
        snapshot_id: &format!("snapshot-{suffix}-{covered_update_seq}"),
        covered_update_seq,
        snapshot_bytes: format!("snapshot-state-{covered_update_seq}").as_bytes(),
        snapshot_bytes_ref: &format!(
            "postgres://kernel_crdt_snapshots/{}/snapshot-{covered_update_seq}/snapshot_bytes",
            identity.crdt_document_id
        ),
        state_vector: &format!("sv-{covered_update_seq}"),
        event_ledger_event_id: &format!("evt-{suffix}-snapshot-{covered_update_seq}"),
        promotion_evidence_update_ids,
    })
}

fn sample_update_for_workspace(
    suffix: &str,
    update_seq: u64,
    update_id: &str,
    update_bytes: &[u8],
    state_vector_before: &str,
    state_vector_after: &str,
) -> CrdtUpdateRecordV1 {
    let mut identity = sample_identity();
    identity.workspace_id = format!("workspace-kernel-{suffix}");
    identity.document_id = format!("document-kernel-{suffix}");
    identity.crdt_document_id = format!("crdt-document-kernel-{suffix}");
    identity.authority_links.event_ledger_stream_id = format!("event-ledger-stream-{suffix}");
    new_crdt_update_record(CrdtUpdateRecordInputV1 {
        identity: &identity,
        update_id,
        update_seq,
        update_bytes,
        update_bytes_ref: &format!(
            "postgres://kernel_crdt_updates/{}/{update_id}/update_bytes",
            identity.crdt_document_id
        ),
        session_id: "session-kernel-builder",
        trace_id: &format!("trace-{suffix}-{update_id}"),
        state_vector_before,
        state_vector_after,
        replay_metadata: CrdtReplayMetadataV1 {
            replay_order_key: format!(
                "{}/{}/{update_seq:020}",
                identity.workspace_id, identity.document_id
            ),
            dependency_update_ids: Vec::new(),
            encoding: "yjs-update-v1".to_string(),
            schema_version: "kernel-crdt-update-v1".to_string(),
        },
        event_ledger_event_id: &format!("evt-{suffix}-{update_id}"),
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
