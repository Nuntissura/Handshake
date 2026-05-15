use handshake_core::kernel::crdt::context_slice::{
    build_crdt_context_slice, validate_crdt_context_slice, CrdtContextSliceKind,
    CrdtContextSliceRequestV1, CrdtContextVersionRefV1, CrdtMaterializedFieldV1,
    CrdtSelectionRangeV1,
};
use handshake_core::kernel::crdt::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};
use handshake_core::kernel::crdt::persistence::{
    new_crdt_update_record, CrdtReplayMetadataV1, CrdtUpdateRecordInputV1, CrdtUpdateRecordV1,
};

#[test]
fn kernel_crdt_context_slice_returns_summary_range_digests_and_deltas_with_citations() {
    let request = sample_request(96, 2, Some(selection("body", 6, 22)));
    let fields = sample_fields();
    let updates = sample_updates();

    let slice = build_crdt_context_slice(&request, &fields, &updates)
        .expect("bounded context slice must build");

    let summary = slice.summary.as_ref().expect("summary slice expected");
    assert!(summary.text.len() <= 96);
    assert_eq!(summary.citation.workspace_id, "workspace-kernel");
    assert_eq!(summary.citation.state_vector, "sv-3");
    assert!(summary
        .citation
        .source_ids
        .contains(&"crdt-update-1".to_string()));

    let selected = slice
        .selected_ranges
        .first()
        .expect("selected range expected");
    assert_eq!(selected.field_id, "body");
    assert_eq!(selected.text, "second paragraph");
    assert!(selected
        .citation
        .source_ids
        .contains(&"crdt-update-2".to_string()));

    assert_eq!(slice.field_digests.len(), 2);
    assert!(slice
        .field_digests
        .iter()
        .all(|digest| digest.content_sha256.len() == 64));
    assert_eq!(slice.operation_deltas.len(), 2);
    assert!(slice.operation_deltas.iter().all(|delta| {
        delta.citation.workspace_id == "workspace-kernel"
            && delta.citation.latest_update_seq == 3
            && !delta.citation.source_ids.is_empty()
    }));

    validate_crdt_context_slice(&slice).expect("all slice outputs must cite source ids");
}

#[test]
fn kernel_crdt_context_slice_enforces_prompt_bounds_without_whole_document_load() {
    let request = sample_request(32, 1, None);
    let fields = vec![CrdtMaterializedFieldV1 {
        field_id: "large-body".to_string(),
        field_path: "body.large".to_string(),
        text: "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_string(),
        source_update_ids: vec!["crdt-update-1".to_string()],
    }];
    let updates = sample_updates();

    let slice = build_crdt_context_slice(&request, &fields, &updates)
        .expect("bounded context slice must build");

    let summary = slice.summary.as_ref().expect("summary slice expected");
    assert!(summary.truncated);
    assert!(summary.text.len() <= 32);
    assert_eq!(slice.operation_deltas.len(), 1);
    assert_eq!(slice.operation_deltas[0].update_id, "crdt-update-3");
}

#[test]
fn kernel_crdt_context_slice_rejects_missing_version_or_source_citation() {
    let mut missing_version = sample_request(64, 2, None);
    missing_version.version.state_vector.clear();
    let error = build_crdt_context_slice(&missing_version, &sample_fields(), &sample_updates())
        .expect_err("missing state vector must fail");
    assert!(error
        .iter()
        .any(|error| error.field == "version.state_vector"));

    let mut uncited_field = sample_fields();
    uncited_field[0].source_update_ids.clear();
    let error = build_crdt_context_slice(
        &sample_request(64, 2, None),
        &uncited_field,
        &sample_updates(),
    )
    .expect_err("field without source ids must fail");
    assert!(error
        .iter()
        .any(|error| error.field == "fields.source_update_ids"));
}

fn sample_request(
    max_text_bytes: usize,
    max_operation_deltas: usize,
    selected_range: Option<CrdtSelectionRangeV1>,
) -> CrdtContextSliceRequestV1 {
    CrdtContextSliceRequestV1 {
        identity: sample_identity(),
        requested_kinds: vec![
            CrdtContextSliceKind::Summary,
            CrdtContextSliceKind::SelectedRange,
            CrdtContextSliceKind::FieldDigest,
            CrdtContextSliceKind::OperationDelta,
        ],
        selected_range,
        max_text_bytes,
        max_operation_deltas,
        version: CrdtContextVersionRefV1 {
            state_vector: "sv-3".to_string(),
            latest_update_seq: 3,
            snapshot_id: Some("snapshot-3".to_string()),
        },
    }
}

fn selection(field_id: &str, start_byte: usize, end_byte: usize) -> CrdtSelectionRangeV1 {
    CrdtSelectionRangeV1 {
        field_id: field_id.to_string(),
        start_byte,
        end_byte,
    }
}

fn sample_fields() -> Vec<CrdtMaterializedFieldV1> {
    vec![
        CrdtMaterializedFieldV1 {
            field_id: "title".to_string(),
            field_path: "title".to_string(),
            text: "Kernel CRDT Context".to_string(),
            source_update_ids: vec!["crdt-update-1".to_string()],
        },
        CrdtMaterializedFieldV1 {
            field_id: "body".to_string(),
            field_path: "body.blocks.0".to_string(),
            text: "first second paragraph third".to_string(),
            source_update_ids: vec!["crdt-update-2".to_string(), "crdt-update-3".to_string()],
        },
    ]
}

fn sample_updates() -> Vec<CrdtUpdateRecordV1> {
    vec![
        sample_update(1, "crdt-update-1", b"update-1", "sv-0", "sv-1"),
        sample_update(2, "crdt-update-2", b"update-2", "sv-1", "sv-2"),
        sample_update(3, "crdt-update-3", b"update-3", "sv-2", "sv-3"),
    ]
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
            work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-013"
                .to_string(),
            action_trace_id: "trace-crdt-workspace".to_string(),
            artifact_proposal_id: "artifact-proposal-crdt".to_string(),
            role_mailbox_thread_id: "role-mailbox-thread-crdt".to_string(),
            dcc_projection_id: "dcc-crdt-projection".to_string(),
            event_ledger_stream_id: "event-ledger-stream-crdt".to_string(),
        },
    }
}
