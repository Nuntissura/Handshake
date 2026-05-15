use handshake_core::kernel::{
    action_catalog::kernel002_action_catalog,
    action_envelope::AuthorityEffect,
    mirror_advisory::{
        capture_mirror_advisory_edit, promote_mirror_advisory_if_valid, MirrorAdvisoryEditV1,
        MirrorAdvisoryPromotionError,
    },
    write_boxes::{WriteBoxKind, WriteBoxValidationState},
};

fn sample_edit() -> MirrorAdvisoryEditV1 {
    MirrorAdvisoryEditV1 {
        advisory_id: "advisory-001".to_string(),
        actor_id: "actor-model-1".to_string(),
        actor_kind: "model".to_string(),
        role_id: "CODER".to_string(),
        mirror_path: "generated/task-board.md".to_string(),
        source_projection_hash: "sha256:projection".to_string(),
        proposed_patch_ref: "patch-ref-001".to_string(),
        trace_id: "trace-advisory".to_string(),
    }
}

#[test]
fn generated_mirror_edit_becomes_mirror_advisory_box_without_authority_mutation() {
    let catalog = kernel002_action_catalog();
    let record = capture_mirror_advisory_edit(&sample_edit(), &catalog)
        .expect("mirror advisory capture must succeed");

    assert_eq!(record.schema_id, "hsk.mirror_advisory_record@1");
    assert_eq!(
        record.mirror_advisory_box.common.kind,
        WriteBoxKind::MirrorAdvisory
    );
    assert_eq!(
        record.mirror_advisory_box.common.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        record.mirror_advisory_box.common.validation_status.state,
        WriteBoxValidationState::Pending
    );
    assert!(!record.authority_mutation);
    assert!(record.accepted_event_ledger_ref.is_none());
    assert_eq!(
        record.normalization_action_id,
        "kernel.mirror_advisory.normalize"
    );
    assert_eq!(record.promotion_action_id, "kernel.write_box.promote");
}

#[test]
fn mirror_advisory_requires_registered_normalization_and_promotion_actions() {
    let catalog = kernel002_action_catalog();
    let record = capture_mirror_advisory_edit(&sample_edit(), &catalog)
        .expect("mirror advisory capture must succeed");

    assert!(catalog.action(&record.normalization_action_id).is_some());
    assert!(catalog.action(&record.promotion_action_id).is_some());
    assert!(record
        .mirror_advisory_box
        .common
        .projection_rules
        .contains(&"dcc.mirror_advisory_queue".to_string()));
}

#[test]
fn mirror_advisory_cannot_promote_until_validation_accepts_it() {
    let catalog = kernel002_action_catalog();
    let mut record = capture_mirror_advisory_edit(&sample_edit(), &catalog)
        .expect("mirror advisory capture must succeed");

    let error = promote_mirror_advisory_if_valid(&record, "validation-receipt-001")
        .expect_err("pending advisory must not build promotion input");
    assert_eq!(error, MirrorAdvisoryPromotionError::ValidationNotAccepted);

    record.mirror_advisory_box.common.validation_status.state = WriteBoxValidationState::Valid;
    let promotion = promote_mirror_advisory_if_valid(&record, "validation-receipt-001")
        .expect("valid advisory can build promotion input");
    assert_eq!(promotion.action_id, "kernel.mirror_advisory.normalize");
    assert_eq!(promotion.promotion_action_id, "kernel.write_box.promote");
    assert_eq!(promotion.validation_receipt_ref, "validation-receipt-001");
    assert!(promotion.authority_mutation_allowed);
}
