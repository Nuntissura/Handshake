use handshake_core::kernel::crdt::identity::{
    validate_crdt_workspace_identity, CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1,
};

fn sample_identity() -> CrdtWorkspaceIdentityV1 {
    CrdtWorkspaceIdentityV1 {
        schema_id: "hsk.crdt_workspace_identity@1".to_string(),
        workspace_id: "workspace-001".to_string(),
        document_id: "document-001".to_string(),
        crdt_document_id: "crdt-doc-001".to_string(),
        actor_id: "actor-model-1".to_string(),
        actor_kind: "model".to_string(),
        crdt_site_id: "site-001".to_string(),
        crdt_client_id: "client-001".to_string(),
        document_schema_id: "hsk.tiptap_doc@1".to_string(),
        authority_links: CrdtAuthorityLinksV1 {
            work_item_id: "WP-KERNEL-002".to_string(),
            action_trace_id: "trace-001".to_string(),
            artifact_proposal_id: "proposal-001".to_string(),
            role_mailbox_thread_id: "thread-001".to_string(),
            dcc_projection_id: "dcc-crdt-workspace".to_string(),
            event_ledger_stream_id: "event-stream-001".to_string(),
        },
    }
}

#[test]
fn crdt_workspace_identity_carries_required_ids() {
    let identity = sample_identity();
    validate_crdt_workspace_identity(&identity).expect("identity must validate");

    assert_eq!(identity.workspace_id, "workspace-001");
    assert_eq!(identity.document_id, "document-001");
    assert_eq!(identity.crdt_document_id, "crdt-doc-001");
    assert_eq!(identity.actor_id, "actor-model-1");
    assert_eq!(identity.crdt_site_id, "site-001");
    assert_eq!(identity.crdt_client_id, "client-001");
    assert_eq!(identity.document_schema_id, "hsk.tiptap_doc@1");
}

#[test]
fn crdt_workspace_identity_links_to_kernel_authority_surfaces() {
    let identity = sample_identity();
    let links = &identity.authority_links;

    assert_eq!(links.work_item_id, "WP-KERNEL-002");
    assert_eq!(links.action_trace_id, "trace-001");
    assert_eq!(links.artifact_proposal_id, "proposal-001");
    assert_eq!(links.role_mailbox_thread_id, "thread-001");
    assert_eq!(links.dcc_projection_id, "dcc-crdt-workspace");
    assert_eq!(links.event_ledger_stream_id, "event-stream-001");
}

#[test]
fn crdt_workspace_identity_validation_rejects_missing_link_or_schema() {
    let mut missing_schema = sample_identity();
    missing_schema.document_schema_id.clear();
    let errors = validate_crdt_workspace_identity(&missing_schema)
        .expect_err("missing document schema must fail");
    assert!(errors
        .iter()
        .any(|error| error.field == "document_schema_id"));

    let mut missing_event_stream = sample_identity();
    missing_event_stream
        .authority_links
        .event_ledger_stream_id
        .clear();
    let errors = validate_crdt_workspace_identity(&missing_event_stream)
        .expect_err("missing EventLedger link must fail");
    assert!(errors
        .iter()
        .any(|error| error.field == "authority_links.event_ledger_stream_id"));
}
