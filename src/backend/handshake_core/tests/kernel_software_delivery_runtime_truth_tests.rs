use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    software_delivery_runtime_truth::{
        kernel002_software_delivery_governed_actions, query_software_delivery_runtime_posture,
        validate_software_delivery_runtime_truth_record, RuntimeTruthAuthoritySourceKind,
        SoftwareDeliveryPhase, SoftwareDeliveryRuntimeJoinsV1,
        SoftwareDeliveryRuntimeTruthRecordV1, SoftwareDeliveryStatus,
    },
};

#[test]
fn software_delivery_runtime_truth_records_fold_legacy_stub_into_product_owned_state() {
    let record = sample_record(RuntimeTruthAuthoritySourceKind::ProductRuntimeRecord, 7);

    validate_software_delivery_runtime_truth_record(&record)
        .expect("product-owned runtime truth record must validate");

    assert_eq!(
        record.schema_id,
        "hsk.kernel.software_delivery_runtime_truth_record@1"
    );
    assert_eq!(
        record.folded_source_refs,
        vec![
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Runtime-Truth-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Runtime-Truth-v1.md".to_string(),
        ]
    );
    assert_eq!(
        record.joins.work_packet_id,
        "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
    );
    assert_eq!(record.joins.task_board_item_id, "TASKBOARD-WP-KERNEL-002");
    assert!(record
        .governed_action_ids
        .contains(&"kernel.software_delivery_runtime_truth.transition".to_string()));
}

#[test]
fn runtime_truth_rejects_packet_prose_mailbox_order_and_markdown_freshness_as_authority() {
    for source_kind in [
        RuntimeTruthAuthoritySourceKind::PacketProse,
        RuntimeTruthAuthoritySourceKind::MailboxChronology,
        RuntimeTruthAuthoritySourceKind::MarkdownFreshness,
    ] {
        let record = sample_record(source_kind, 99);
        let errors = validate_software_delivery_runtime_truth_record(&record)
            .expect_err("non-product surfaces must not validate as runtime truth");

        assert!(
            errors.iter().any(|error| error.field == "source_kind"
                && error
                    .message
                    .contains("not an authoritative runtime truth source")),
            "expected source_kind denial for {source_kind:?}, got {errors:?}"
        );
    }
}

#[test]
fn current_posture_projects_latest_valid_product_record_and_ignores_mirror_freshness() {
    let old_product_record =
        sample_record(RuntimeTruthAuthoritySourceKind::ProductRuntimeRecord, 2);
    let mut latest_product_record =
        sample_record(RuntimeTruthAuthoritySourceKind::GovernedActionReceipt, 8);
    latest_product_record.status = SoftwareDeliveryStatus::Blocked;
    latest_product_record.waiting_on = Some("validator-review".to_string());
    let markdown_mirror = sample_record(RuntimeTruthAuthoritySourceKind::MarkdownFreshness, 99);

    let projection = query_software_delivery_runtime_posture(
        &[old_product_record, markdown_mirror, latest_product_record],
        &kernel002_software_delivery_governed_actions(),
        "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1",
    )
    .expect("runtime posture should project from valid product-owned records");

    assert_eq!(projection.current_record.record_seq, 8);
    assert_eq!(
        projection.current_record.source_kind,
        RuntimeTruthAuthoritySourceKind::GovernedActionReceipt
    );
    assert_eq!(
        projection.current_record.status,
        SoftwareDeliveryStatus::Blocked
    );
    assert_eq!(
        projection.ignored_non_authority_source_kinds,
        vec![RuntimeTruthAuthoritySourceKind::MarkdownFreshness]
    );
    assert!(projection
        .governed_actions
        .iter()
        .any(|action| action.action_id == "kernel.software_delivery_runtime_truth.transition"));
    assert!(projection
        .source_lineage_refs
        .iter()
        .any(|source| source.contains("WP-1-Software-Delivery-Runtime-Truth-v1")));
}

#[test]
fn kernel_action_catalog_exposes_runtime_truth_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate with runtime action");

    let action = catalog
        .action("kernel.software_delivery_runtime_truth.project")
        .expect("runtime truth projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "runtime_truth_source_kind"));
    assert!(action
        .dcc_preview
        .primary_state_fields
        .contains(&"record_seq".to_string()));
}

fn sample_record(
    source_kind: RuntimeTruthAuthoritySourceKind,
    record_seq: u64,
) -> SoftwareDeliveryRuntimeTruthRecordV1 {
    SoftwareDeliveryRuntimeTruthRecordV1 {
        schema_id: "hsk.kernel.software_delivery_runtime_truth_record@1".to_string(),
        record_id: format!("runtime-truth-{record_seq}"),
        wp_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
        mt_id: Some("MT-017".to_string()),
        worktree_id: "wtc-preuse-hardening-v1".to_string(),
        branch: "feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
        phase: SoftwareDeliveryPhase::Implementation,
        status: SoftwareDeliveryStatus::InProgress,
        next_actor: "CODER".to_string(),
        waiting_on: None,
        record_seq,
        source_kind,
        joins: SoftwareDeliveryRuntimeJoinsV1 {
            work_packet_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1"
                .to_string(),
            task_board_item_id: "TASKBOARD-WP-KERNEL-002".to_string(),
            role_mailbox_thread_id: "ROLEMAILBOX-KERNEL-BUILDER-MT-017".to_string(),
            workflow_state_id: "WORKFLOW-STATE-KERNEL-002-MT-017".to_string(),
            gate_state_id: "GATE-STATE-KERNEL-002-INTEGRATION".to_string(),
        },
        governed_action_ids: vec![
            "kernel.software_delivery_runtime_truth.record".to_string(),
            "kernel.software_delivery_runtime_truth.transition".to_string(),
            "kernel.software_delivery_runtime_truth.project".to_string(),
        ],
        evidence_refs: vec![
            "cargo test -p handshake_core --test kernel_software_delivery_runtime_truth_tests"
                .to_string(),
            "receipt://KERNEL_BUILDER-20260514-130219/MT-017".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Runtime-Truth-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Software-Delivery-Runtime-Truth-v1.md".to_string(),
        ],
    }
}
