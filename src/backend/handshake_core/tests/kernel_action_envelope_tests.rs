use handshake_core::kernel::action_envelope::{
    validate_kernel_action_request, ApprovalPosture, AuthorityEffect, EventLedgerMapping,
    ExpectedWriteBoxRef, KernelActionDenialV1, KernelActionRequestV1, KernelActionResultStatus,
    KernelActionResultV1, KernelActorRef, KernelReceiptMapping, KernelSessionRef, KernelTargetRef,
    ValidationRequirement,
};

fn sample_request() -> KernelActionRequestV1 {
    KernelActionRequestV1 {
        schema_id: "hsk.kernel_action_request@1".to_string(),
        action_id: "kernel.crdt_workspace.propose_patch".to_string(),
        actor: KernelActorRef {
            actor_id: "actor-model-1".to_string(),
            actor_kind: "model".to_string(),
            role_id: "CODER".to_string(),
        },
        session: KernelSessionRef {
            session_id: "session-1".to_string(),
            work_profile_id: "profile-standard-coder".to_string(),
        },
        target_ids: vec![KernelTargetRef {
            target_id: "doc-1".to_string(),
            target_kind: "document".to_string(),
            authority_class: "pre_promotion_workspace".to_string(),
        }],
        input_schema_id: "hsk.kernel.crdt_patch_input@1".to_string(),
        expected_write_boxes: vec![ExpectedWriteBoxRef {
            write_box_kind: "CRDTWorkspaceBox".to_string(),
            write_box_schema_id: "hsk.write_box.crdt_workspace@1".to_string(),
            target_id: "doc-1".to_string(),
        }],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        validation_requirements: vec![
            ValidationRequirement {
                check_id: "schema-validity".to_string(),
                required: true,
            },
            ValidationRequirement {
                check_id: "state-vector-freshness".to_string(),
                required: true,
            },
        ],
        trace_id: "trace-001".to_string(),
        idempotency_key: "idem-001".to_string(),
    }
}

#[test]
fn kernel_action_request_carries_required_kernel_fields() {
    let request = sample_request();
    validate_kernel_action_request(&request).expect("complete request must validate");

    assert_eq!(request.actor.actor_id, "actor-model-1");
    assert_eq!(request.session.session_id, "session-1");
    assert_eq!(request.session.work_profile_id, "profile-standard-coder");
    assert_eq!(request.target_ids[0].target_id, "doc-1");
    assert_eq!(request.input_schema_id, "hsk.kernel.crdt_patch_input@1");
    assert_eq!(
        request.expected_write_boxes[0].write_box_kind,
        "CRDTWorkspaceBox"
    );
    assert_eq!(
        request.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        request.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert_eq!(request.trace_id, "trace-001");
}

#[test]
fn kernel_action_request_validation_rejects_missing_trace_or_write_box() {
    let mut missing_trace = sample_request();
    missing_trace.trace_id.clear();
    let errors = validate_kernel_action_request(&missing_trace)
        .expect_err("missing trace id must fail request validation");
    assert!(errors.iter().any(|error| error.field == "trace_id"));

    let mut missing_write_box = sample_request();
    missing_write_box.expected_write_boxes.clear();
    let errors = validate_kernel_action_request(&missing_write_box)
        .expect_err("missing expected write box must fail request validation");
    assert!(errors
        .iter()
        .any(|error| error.field == "expected_write_boxes"));
}

#[test]
fn kernel_action_result_maps_to_receipts_and_event_ledger() {
    let result = KernelActionResultV1 {
        schema_id: "hsk.kernel_action_result@1".to_string(),
        result_id: "result-001".to_string(),
        request_trace_id: "trace-001".to_string(),
        status: KernelActionResultStatus::WriteBoxesCreated,
        write_box_ids: vec!["wb-001".to_string()],
        receipt_mappings: vec![KernelReceiptMapping {
            receipt_kind: "STATUS".to_string(),
            receipt_schema_id: "hsk.wp_receipt@1".to_string(),
            correlation_id: "trace-001".to_string(),
        }],
        event_mappings: vec![EventLedgerMapping {
            event_kind: "KernelActionWriteBoxesCreatedV1".to_string(),
            event_schema_id: "hsk.event.kernel_action_write_boxes_created@1".to_string(),
            idempotency_key: "idem-001".to_string(),
        }],
        denial: None,
    };

    assert_eq!(result.request_trace_id, "trace-001");
    assert_eq!(result.write_box_ids, vec!["wb-001"]);
    assert_eq!(result.receipt_mappings[0].correlation_id, "trace-001");
    assert_eq!(result.event_mappings[0].idempotency_key, "idem-001");
}

#[test]
fn kernel_action_denial_is_actionable_and_replayable() {
    let denial = KernelActionDenialV1 {
        schema_id: "hsk.kernel_action_denial@1".to_string(),
        denial_id: "denial-001".to_string(),
        request_trace_id: "trace-raw-edit".to_string(),
        denial_code: "direct_authority_edit_denied".to_string(),
        reason: "Authority files require a registered write-box action.".to_string(),
        lawful_replacement_action_ids: vec![
            "kernel.mirror_advisory.capture".to_string(),
            "kernel.crdt_workspace.propose_patch".to_string(),
        ],
        evidence_refs: vec!["attempt-ref-001".to_string()],
        receipt_mappings: vec![KernelReceiptMapping {
            receipt_kind: "DENIAL".to_string(),
            receipt_schema_id: "hsk.wp_receipt@1".to_string(),
            correlation_id: "trace-raw-edit".to_string(),
        }],
        event_mappings: vec![EventLedgerMapping {
            event_kind: "KernelActionDeniedV1".to_string(),
            event_schema_id: "hsk.event.kernel_action_denied@1".to_string(),
            idempotency_key: "trace-raw-edit".to_string(),
        }],
    };

    assert!(!denial.lawful_replacement_action_ids.is_empty());
    assert!(!denial.evidence_refs.is_empty());
    assert_eq!(denial.receipt_mappings[0].receipt_kind, "DENIAL");
    assert_eq!(denial.event_mappings[0].event_kind, "KernelActionDeniedV1");
}
