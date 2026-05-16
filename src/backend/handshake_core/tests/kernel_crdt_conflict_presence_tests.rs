use handshake_core::kernel::crdt::conflict_presence::{
    build_crdt_conflict_presence_projection, CrdtChangePromotionState, CrdtConflictPresenceInputV1,
    CrdtPendingConflictV1, CrdtPresenceRecordV1, CrdtPresenceStatus, CrdtPromotionStateRefV1,
};
use handshake_core::kernel::crdt::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};
use handshake_core::kernel::crdt::persistence::{
    new_crdt_update_record, CrdtReplayMetadataV1, CrdtUpdateRecordInputV1, CrdtUpdateRecordV1,
};

#[test]
fn kernel_crdt_conflict_presence_projection_shows_presence_conflicts_and_attribution() {
    let projection = build_crdt_conflict_presence_projection(CrdtConflictPresenceInputV1 {
        identity: sample_identity(),
        presence_records: sample_presence(),
        pending_conflicts: sample_conflicts(),
        updates: sample_updates(),
        promotion_states: vec![
            CrdtPromotionStateRefV1 {
                update_id: "crdt-update-2".to_string(),
                promotion_state: CrdtChangePromotionState::PendingPromotion,
                proposal_id: Some("artifact-proposal-pending".to_string()),
            },
            CrdtPromotionStateRefV1 {
                update_id: "crdt-update-3".to_string(),
                promotion_state: CrdtChangePromotionState::PromotionAccepted,
                proposal_id: Some("artifact-proposal-accepted".to_string()),
            },
        ],
    })
    .expect("projection must build");

    assert_eq!(projection.presence.len(), 2);
    assert_eq!(projection.pending_conflicts.len(), 1);
    assert_eq!(
        projection.pending_conflicts[0].actor_ids,
        vec!["actor-alpha".to_string(), "actor-beta".to_string()]
    );

    let update_one = projection
        .actor_attributions
        .iter()
        .find(|attribution| attribution.update_id == "crdt-update-1")
        .expect("update one attribution expected");
    assert_eq!(update_one.actor_id, "actor-alpha");
    assert_eq!(
        update_one.promotion_state,
        CrdtChangePromotionState::MergedCrdtOnly
    );

    assert_eq!(
        projection.merged_crdt_update_ids,
        vec!["crdt-update-1".to_string()]
    );
    assert_eq!(
        projection.pending_promotion_update_ids,
        vec!["crdt-update-2".to_string()]
    );
    assert_eq!(
        projection.accepted_promotion_update_ids,
        vec!["crdt-update-3".to_string()]
    );
}

#[test]
fn kernel_crdt_conflict_presence_projection_rejects_uncited_presence_or_conflict_records() {
    let mut presence = sample_presence();
    presence[0].session_id.clear();
    let error = build_crdt_conflict_presence_projection(CrdtConflictPresenceInputV1 {
        identity: sample_identity(),
        presence_records: presence,
        pending_conflicts: sample_conflicts(),
        updates: sample_updates(),
        promotion_states: Vec::new(),
    })
    .expect_err("presence without session id must fail");
    assert!(error
        .iter()
        .any(|error| error.field == "presence.session_id"));

    let mut conflicts = sample_conflicts();
    conflicts[0].actor_update_ids.clear();
    let error = build_crdt_conflict_presence_projection(CrdtConflictPresenceInputV1 {
        identity: sample_identity(),
        presence_records: sample_presence(),
        pending_conflicts: conflicts,
        updates: sample_updates(),
        promotion_states: Vec::new(),
    })
    .expect_err("conflict without update ids must fail");
    assert!(error
        .iter()
        .any(|error| error.field == "pending_conflicts.actor_update_ids"));
}

fn sample_presence() -> Vec<CrdtPresenceRecordV1> {
    vec![
        CrdtPresenceRecordV1 {
            actor_id: "actor-alpha".to_string(),
            actor_kind: "model".to_string(),
            session_id: "session-alpha".to_string(),
            cursor_field_id: "body".to_string(),
            cursor_start_byte: 0,
            cursor_end_byte: 5,
            status: CrdtPresenceStatus::Active,
            last_seen_state_vector: "sv-3".to_string(),
        },
        CrdtPresenceRecordV1 {
            actor_id: "actor-beta".to_string(),
            actor_kind: "model".to_string(),
            session_id: "session-beta".to_string(),
            cursor_field_id: "body".to_string(),
            cursor_start_byte: 6,
            cursor_end_byte: 12,
            status: CrdtPresenceStatus::Idle,
            last_seen_state_vector: "sv-3".to_string(),
        },
    ]
}

fn sample_conflicts() -> Vec<CrdtPendingConflictV1> {
    vec![CrdtPendingConflictV1 {
        conflict_id: "conflict-body-1".to_string(),
        field_id: "body".to_string(),
        actor_ids: vec!["actor-alpha".to_string(), "actor-beta".to_string()],
        actor_update_ids: vec!["crdt-update-1".to_string(), "crdt-update-2".to_string()],
        conflict_summary: "overlapping body edits".to_string(),
    }]
}

fn sample_updates() -> Vec<CrdtUpdateRecordV1> {
    vec![
        sample_update(1, "crdt-update-1", "actor-alpha", "session-alpha"),
        sample_update(2, "crdt-update-2", "actor-beta", "session-beta"),
        sample_update(3, "crdt-update-3", "actor-alpha", "session-alpha"),
    ]
}

fn sample_update(
    update_seq: u64,
    update_id: &str,
    actor_id: &str,
    session_id: &str,
) -> CrdtUpdateRecordV1 {
    let mut identity = sample_identity();
    identity.actor_id = actor_id.to_string();
    new_crdt_update_record(CrdtUpdateRecordInputV1 {
        identity: &identity,
        update_id,
        update_seq,
        update_bytes: format!("update-{update_seq}").as_bytes(),
        update_bytes_ref: &format!("postgres://kernel_crdt_updates/{update_id}/update_bytes"),
        session_id,
        trace_id: &format!("trace-{update_id}"),
        state_vector_before: &format!("sv-{}", update_seq - 1),
        state_vector_after: &format!("sv-{update_seq}"),
        replay_metadata: CrdtReplayMetadataV1 {
            replay_order_key: format!("workspace-kernel/document-kernel/{update_seq:020}"),
            dependency_update_ids: Vec::new(),
            encoding: "yjs-update-v1".to_string(),
            schema_version: "kernel-crdt-update-v1".to_string(),
        },
        event_ledger_event_id: &format!("evt-{update_id}"),
    })
}

fn sample_identity() -> CrdtWorkspaceIdentityV1 {
    CrdtWorkspaceIdentityV1 {
        schema_id: "hsk.kernel.crdt_workspace_identity@1".to_string(),
        workspace_id: "workspace-kernel".to_string(),
        document_id: "document-kernel".to_string(),
        crdt_document_id: "crdt-document-kernel".to_string(),
        actor_id: "actor-alpha".to_string(),
        actor_kind: "model".to_string(),
        crdt_site_id: "site-alpha".to_string(),
        crdt_client_id: "client-alpha".to_string(),
        document_schema_id: "hsk.kernel.crdt_document@1".to_string(),
        authority_links: CrdtAuthorityLinksV1 {
            work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-016"
                .to_string(),
            action_trace_id: "trace-crdt-workspace".to_string(),
            artifact_proposal_id: "artifact-proposal-crdt".to_string(),
            role_mailbox_thread_id: "role-mailbox-thread-crdt".to_string(),
            dcc_projection_id: "dcc-crdt-projection".to_string(),
            event_ledger_stream_id: "event-ledger-stream-crdt".to_string(),
        },
    }
}
