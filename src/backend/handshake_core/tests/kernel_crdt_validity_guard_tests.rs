use handshake_core::kernel::crdt::context_slice::CrdtMaterializedFieldV1;
use handshake_core::kernel::crdt::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};
use handshake_core::kernel::crdt::validity_guard::{
    validate_crdt_state_for_promotion, CrdtMaterializedStateV1, CrdtPromotionValidationDecision,
    CrdtSchemaFieldRequirementV1, CrdtSchemaGuardContractV1, CrdtStateValidationErrorCode,
};

#[test]
fn kernel_crdt_validity_guard_allows_structurally_valid_authorized_state() {
    let report = validate_crdt_state_for_promotion(&sample_state(), &sample_schema_guard());

    assert_eq!(report.decision, CrdtPromotionValidationDecision::Allowed);
    assert!(report.promotion_allowed);
    assert!(report.validation_errors.is_empty());
    assert_eq!(report.document_schema_id, "hsk.kernel.crdt_document@1");
}

#[test]
fn kernel_crdt_validity_guard_denies_structurally_invalid_state_before_promotion() {
    let mut state = sample_state();
    state.fields.retain(|field| field.field_id != "body");
    state.fields[0].source_update_ids.clear();

    let report = validate_crdt_state_for_promotion(&state, &sample_schema_guard());

    assert_eq!(report.decision, CrdtPromotionValidationDecision::Denied);
    assert!(!report.promotion_allowed);
    assert!(report
        .validation_errors
        .iter()
        .any(|error| error.code == CrdtStateValidationErrorCode::MissingRequiredField));
    assert!(report
        .validation_errors
        .iter()
        .any(|error| error.code == CrdtStateValidationErrorCode::SourceCitationMissing));
}

#[test]
fn kernel_crdt_validity_guard_denies_unauthorized_actor_and_schema_drift() {
    let mut state = sample_state();
    state.identity.actor_id = "actor-unknown".to_string();
    state.document_schema_id = "hsk.kernel.legacy_document@1".to_string();
    state.state_vector = "stale-sv".to_string();

    let report = validate_crdt_state_for_promotion(&state, &sample_schema_guard());

    assert_eq!(report.decision, CrdtPromotionValidationDecision::Denied);
    assert!(report
        .validation_errors
        .iter()
        .any(|error| error.code == CrdtStateValidationErrorCode::UnauthorizedActor));
    assert!(report
        .validation_errors
        .iter()
        .any(|error| error.code == CrdtStateValidationErrorCode::SchemaDrift));
    assert!(report
        .validation_errors
        .iter()
        .any(|error| error.code == CrdtStateValidationErrorCode::FreshnessDrift));
}

#[test]
fn kernel_crdt_validity_guard_denies_unknown_source_update_ids() {
    let mut state = sample_state();
    state.fields[1]
        .source_update_ids
        .push("crdt-update-outside-log".to_string());

    let report = validate_crdt_state_for_promotion(&state, &sample_schema_guard());

    assert_eq!(report.decision, CrdtPromotionValidationDecision::Denied);
    assert!(report
        .validation_errors
        .iter()
        .any(|error| error.code == CrdtStateValidationErrorCode::UnknownSourceUpdate));
}

fn sample_state() -> CrdtMaterializedStateV1 {
    CrdtMaterializedStateV1 {
        identity: sample_identity(),
        document_schema_id: "hsk.kernel.crdt_document@1".to_string(),
        state_vector: "sv-3".to_string(),
        latest_update_seq: 3,
        fields: vec![
            CrdtMaterializedFieldV1 {
                field_id: "title".to_string(),
                field_path: "title".to_string(),
                text: "Kernel Validity".to_string(),
                source_update_ids: vec!["crdt-update-1".to_string()],
            },
            CrdtMaterializedFieldV1 {
                field_id: "body".to_string(),
                field_path: "body.blocks.0".to_string(),
                text: "Promotion-ready body".to_string(),
                source_update_ids: vec!["crdt-update-2".to_string(), "crdt-update-3".to_string()],
            },
        ],
    }
}

fn sample_schema_guard() -> CrdtSchemaGuardContractV1 {
    CrdtSchemaGuardContractV1 {
        schema_id: "hsk.kernel.crdt_schema_guard@1".to_string(),
        document_schema_id: "hsk.kernel.crdt_document@1".to_string(),
        schema_version: "kernel-crdt-document-v1".to_string(),
        required_fields: vec![
            CrdtSchemaFieldRequirementV1 {
                field_id: "title".to_string(),
                field_path: "title".to_string(),
                required: true,
                max_text_bytes: Some(80),
            },
            CrdtSchemaFieldRequirementV1 {
                field_id: "body".to_string(),
                field_path: "body.blocks.0".to_string(),
                required: true,
                max_text_bytes: Some(512),
            },
        ],
        authorized_actor_ids: vec!["actor-kernel-builder".to_string()],
        authorized_actor_kinds: vec!["model".to_string()],
        allowed_source_update_ids: vec![
            "crdt-update-1".to_string(),
            "crdt-update-2".to_string(),
            "crdt-update-3".to_string(),
        ],
        expected_state_vector: "sv-3".to_string(),
        expected_latest_update_seq: 3,
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
            work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-014"
                .to_string(),
            action_trace_id: "trace-crdt-workspace".to_string(),
            artifact_proposal_id: "artifact-proposal-crdt".to_string(),
            role_mailbox_thread_id: "role-mailbox-thread-crdt".to_string(),
            dcc_projection_id: "dcc-crdt-projection".to_string(),
            event_ledger_stream_id: "event-ledger-stream-crdt".to_string(),
        },
    }
}
