use handshake_core::kernel::action_envelope::AuthorityEffect;
use handshake_core::kernel::crdt::context_slice::CrdtMaterializedFieldV1;
use handshake_core::kernel::crdt::identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1};
use handshake_core::kernel::crdt::promotion_bridge::{
    bridge_crdt_state_to_promotion, promote_crdt_state_through_event_ledger,
    required_crdt_promotion_failure_receipts, CrdtPromotionBridgeInputV1,
    CrdtPromotionBridgeStatus, CrdtPromotionFailureReceiptV1,
};
use handshake_core::kernel::crdt::validity_guard::{
    validate_crdt_state_for_promotion, CrdtMaterializedStateV1, CrdtPromotionValidationDecision,
    CrdtSchemaFieldRequirementV1, CrdtSchemaGuardContractV1,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig,
};
use handshake_core::storage::tests::{embedded_test_backend, EmbeddedTestBackend};
use handshake_core::storage::Database;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

async fn reopen_embedded_store(
    backend: &EmbeddedTestBackend,
) -> (SurrealStorage, Arc<dyn Database>) {
    backend
        .storage
        .shutdown()
        .await
        .expect("close original embedded CRDT promotion store");
    let reopened = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&backend.data_dir)
            .expect("configure reopened embedded CRDT promotion store"),
    )
    .await
    .expect("reopen embedded CRDT promotion store");
    bootstrap_schema(&reopened)
        .await
        .expect("bootstrap reopened CRDT promotion schema");
    let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(reopened.clone()));
    (reopened, database)
}

fn assert_typed_failure_receipt_contract(receipts: &[CrdtPromotionFailureReceiptV1]) {
    assert_eq!(receipts.len(), 6, "complete failure receipt taxonomy");
    let mut failure_codes = HashSet::new();
    let mut idempotency_keys = HashSet::new();
    for receipt in receipts {
        assert_eq!(receipt.receipt_kind, "PROMOTION_FAILURE");
        assert_eq!(
            receipt.receipt_schema_id,
            "hsk.kernel.crdt_promotion_failure_receipt@1"
        );
        assert!(receipt.replayable);
        assert!(!receipt.failure_code.trim().is_empty());
        assert!(
            failure_codes.insert(receipt.failure_code.as_str()),
            "failure codes must be unique"
        );
        assert!(
            receipt.idempotency_key.starts_with("promotion-failure:"),
            "failure receipt must carry a typed promotion idempotency key"
        );
        assert!(
            idempotency_keys.insert(receipt.idempotency_key.as_str()),
            "failure receipt idempotency keys must be unique"
        );
        assert!(!receipt.recovery_instruction.trim().is_empty());
    }
    for required in [
        "duplicate_promotion_request",
        "stale_state_vector",
        "simultaneous_operator_model_promotion",
        "validation_failed_after_merge",
        "projection_rebuild_failed",
    ] {
        assert!(failure_codes.contains(required), "missing {required}");
    }
}

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
    let event_kinds: Vec<_> = result
        .event_mappings
        .iter()
        .map(|mapping| mapping.event_kind.as_str())
        .collect();
    assert_eq!(
        event_kinds,
        vec![
            "KernelCrdtPromotionRequestedV1",
            "KernelCrdtPromotionAcceptedV1"
        ]
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
    let event = result
        .event_mapping
        .expect("rejected path must emit rejected EventLedger mapping");
    assert_eq!(event.event_kind, "KernelCrdtPromotionRejectedV1");
    let event_kinds: Vec<_> = result
        .event_mappings
        .iter()
        .map(|mapping| mapping.event_kind.as_str())
        .collect();
    assert_eq!(
        event_kinds,
        vec![
            "KernelCrdtPromotionRequestedV1",
            "KernelCrdtPromotionRejectedV1"
        ]
    );

    let evidence = result
        .rejection_evidence
        .expect("rejected path must retain CRDT evidence");
    assert_eq!(
        evidence.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(evidence.state_hash.len(), 64);
    assert!(!evidence.validation_errors.is_empty());
    assert_typed_failure_receipt_contract(&evidence.failure_receipts);
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

#[tokio::test]
async fn kernel_crdt_promotion_bridge_appends_request_and_decision_events_to_surreal_ledger() {
    let backend = embedded_test_backend()
        .await
        .expect("failed to init embedded SurrealDB backend");
    let db = backend.database.clone();
    let suffix = Uuid::now_v7().simple().to_string();
    let state = sample_state_for_suffix(&suffix);
    let validation_report = validate_crdt_state_for_promotion(&state, &sample_schema_guard());
    let input = CrdtPromotionBridgeInputV1 {
        bridge_id: format!("bridge-ledger-{suffix}"),
        artifact_proposal_id: format!("artifact-proposal-ledger-{suffix}"),
        promotion_gate_id: format!("promotion-gate-ledger-{suffix}"),
        promotion_target_ref: format!("authority://kernel/document/document-kernel-{suffix}"),
        state,
        validation_report,
    };
    let promotion_gate_id = input.promotion_gate_id.clone();

    let result = promote_crdt_state_through_event_ledger(db.as_ref(), input)
        .await
        .expect("promotion bridge must append EventLedger request and accepted events");

    assert_eq!(
        result.bridge_result.status,
        CrdtPromotionBridgeStatus::Accepted
    );
    assert_eq!(result.appended_events.len(), 2);
    assert_eq!(
        result.appended_events[0].event_type,
        KernelEventType::PromotionRequested
    );
    assert_eq!(
        result.appended_events[1].event_type,
        KernelEventType::PromotionAccepted
    );
    assert_eq!(
        result.appended_events[1].causation_id.as_deref(),
        Some(result.appended_events[0].event_id.as_str())
    );
    assert_eq!(
        result.appended_events[1].payload["promotion_target_ref"],
        format!("authority://kernel/document/document-kernel-{suffix}")
    );

    let mapping_keys: Vec<_> = result
        .bridge_result
        .event_mappings
        .iter()
        .map(|mapping| mapping.idempotency_key.as_str())
        .collect();
    let persisted_keys: Vec<_> = result
        .appended_events
        .iter()
        .map(|event| event.idempotency_key.as_str())
        .collect();
    assert_eq!(
        mapping_keys, persisted_keys,
        "promotion bridge mappings must cite the exact persisted EventLedger idempotency keys"
    );

    drop(db);
    let (reopened, reopened_db) = reopen_embedded_store(&backend).await;
    let persisted = reopened_db
        .list_kernel_events_for_aggregate("crdt_promotion", &promotion_gate_id)
        .await
        .expect("read CRDT promotion events after reopen");
    assert_eq!(persisted.len(), 2);
    assert_eq!(persisted[0].event_type, KernelEventType::PromotionRequested);
    assert_eq!(persisted[1].event_type, KernelEventType::PromotionAccepted);
    assert_eq!(
        persisted[1].causation_id.as_deref(),
        Some(persisted[0].event_id.as_str())
    );
    assert_eq!(
        persisted
            .iter()
            .map(|event| event.idempotency_key.as_str())
            .collect::<Vec<_>>(),
        mapping_keys
    );
    drop(reopened_db);
    reopened
        .shutdown()
        .await
        .expect("close reopened CRDT promotion store");
    drop(reopened);
    backend
        .close_and_remove()
        .await
        .expect("embedded SurrealDB storage cleanup");
}

#[test]
fn kernel_crdt_promotion_failure_receipts_cover_required_replay_cases() {
    let receipts = required_crdt_promotion_failure_receipts("bridge-failure-proof");
    assert_typed_failure_receipt_contract(&receipts);
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

fn sample_state_for_suffix(suffix: &str) -> CrdtMaterializedStateV1 {
    let mut state = sample_state();
    state.identity.workspace_id = format!("workspace-kernel-{suffix}");
    state.identity.document_id = format!("document-kernel-{suffix}");
    state.identity.crdt_document_id = format!("crdt-document-kernel-{suffix}");
    state.identity.authority_links.event_ledger_stream_id = format!("event-ledger-stream-{suffix}");
    state
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
