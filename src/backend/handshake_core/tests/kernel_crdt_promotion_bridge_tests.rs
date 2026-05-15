use handshake_core::kernel::action_envelope::AuthorityEffect;
use handshake_core::kernel::crdt::context_slice::CrdtMaterializedFieldV1;
use handshake_core::kernel::crdt::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};
use handshake_core::kernel::crdt::promotion_bridge::{
    bridge_crdt_state_to_promotion, CrdtPromotionBridgeInputV1, CrdtPromotionBridgeStatus,
};
use handshake_core::kernel::crdt::validity_guard::{
    validate_crdt_state_for_promotion, CrdtMaterializedStateV1, CrdtPromotionValidationDecision,
    CrdtSchemaFieldRequirementV1, CrdtSchemaGuardContractV1,
};

#[test]
fn kernel_crdt_promotion_bridge_accepts_validated_state_and_emits_eventledger_mapping() {
    let state = sample_state();
    let validation_report = validate_crdt_state_for_promotion(&state, &sample_schema_guard());
    let result = bridge_crdt_state_to_promotion(CrdtPromotionBridgeInputV1 {
        bridge_id: "bridge-accepted".to_string(),
        artifact_proposal_id: "artifact-proposal-accepted".to_string(),
        promotion_gate_id: "promotion-gate-accepted".to_string(),
        promotion_target_ref: "authority://kernel/document/document-kernel".to_string(),
        state,
        validation_report,
    })
    .expect("accepted promotion bridge must build");

    assert_eq!(result.status, CrdtPromotionBridgeStatus::Accepted);
    let proposal = result
        .artifact_proposal
        .expect("accepted path must create artifact proposal");
    assert_eq!(
        proposal.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(proposal.state_hash.len(), 64);

    let gate = result
        .promotion_gate_input
        .expect("accepted path must create promotion gate input");
    assert_eq!(
        gate.authority_effect,
        AuthorityEffect::EventLedgerAuthorityWrite
    );
    assert_eq!(
        gate.promotion_target_ref,
        "authority://kernel/document/document-kernel"
    );

    let event = result
        .event_mapping
        .expect("accepted path must emit EventLedger mapping");
    assert_eq!(event.event_kind, "KernelCrdtPromotionAcceptedV1");
    assert_eq!(
        event.event_schema_id,
        "hsk.event.kernel_crdt_promotion_accepted@1"
    );
}

#[test]
fn kernel_crdt_promotion_bridge_rejects_invalid_state_as_non_authoritative_evidence() {
    let mut state = sample_state();
    state.fields.retain(|field| field.field_id != "body");
    let validation_report = validate_crdt_state_for_promotion(&state, &sample_schema_guard());

    let result = bridge_crdt_state_to_promotion(CrdtPromotionBridgeInputV1 {
        bridge_id: "bridge-rejected".to_string(),
        artifact_proposal_id: "artifact-proposal-rejected".to_string(),
        promotion_gate_id: "promotion-gate-rejected".to_string(),
        promotion_target_ref: "authority://kernel/document/document-kernel".to_string(),
        state,
        validation_report,
    })
    .expect("rejected promotion bridge must build evidence");

    assert_eq!(result.status, CrdtPromotionBridgeStatus::Rejected);
    assert!(result.artifact_proposal.is_none());
    assert!(result.promotion_gate_input.is_none());
    assert!(result.event_mapping.is_none());

    let evidence = result
        .rejection_evidence
        .expect("rejected path must retain CRDT evidence");
    assert_eq!(
        evidence.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(evidence.state_hash.len(), 64);
    assert!(!evidence.validation_errors.is_empty());
}

#[test]
fn kernel_crdt_promotion_bridge_requires_validation_report_alignment() {
    let state = sample_state();
    let mut validation_report = validate_crdt_state_for_promotion(&state, &sample_schema_guard());
    validation_report.decision = CrdtPromotionValidationDecision::Allowed;
    validation_report.promotion_allowed = false;

    let result = bridge_crdt_state_to_promotion(CrdtPromotionBridgeInputV1 {
        bridge_id: "bridge-misaligned".to_string(),
        artifact_proposal_id: "artifact-proposal-misaligned".to_string(),
        promotion_gate_id: "promotion-gate-misaligned".to_string(),
        promotion_target_ref: "authority://kernel/document/document-kernel".to_string(),
        state,
        validation_report,
    });

    assert!(result.is_err());
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
                text: "Kernel Promotion".to_string(),
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
            work_item_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/MT-015"
                .to_string(),
            action_trace_id: "trace-crdt-workspace".to_string(),
            artifact_proposal_id: "artifact-proposal-crdt".to_string(),
            role_mailbox_thread_id: "role-mailbox-thread-crdt".to_string(),
            dcc_projection_id: "dcc-crdt-projection".to_string(),
            event_ledger_stream_id: "event-ledger-stream-crdt".to_string(),
        },
    }
}
