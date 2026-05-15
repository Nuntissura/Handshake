use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    role_mailbox_inbox_evidence_bridge::{
        preview_mailbox_debug_bundle_export, project_role_mailbox_inbox_evidence_bridge,
        validate_role_mailbox_inbox_evidence_bridge, DebugBundleMailboxEvidenceExportV1,
        InboxLabelAlignmentV1, InboxLabelTarget, MailboxTelemetryEventKind,
        MailboxTelemetryEventV1, RoleMailboxInboxEvidenceBridgeV1,
    },
};

#[test]
fn inbox_evidence_bridge_validates_role_mailbox_only_labels_and_leak_safe_telemetry() {
    let bridge = sample_bridge();

    validate_role_mailbox_inbox_evidence_bridge(&bridge).expect("bridge validates");

    assert_eq!(
        bridge.inbox_label_alignments[0].target,
        InboxLabelTarget::RoleMailbox
    );
    assert!(!bridge.inbox_label_alignments[0].parallel_inbox_semantics_allowed);
    assert!(bridge.telemetry_events.iter().all(|event| {
        !event.includes_inline_body
            && event.payload_redacted
            && !event.stable_provenance_ref.is_empty()
    }));
}

#[test]
fn inbox_evidence_bridge_projects_debug_bundle_evidence_without_authority_transfer() {
    let bridge = sample_bridge();
    let projection = project_role_mailbox_inbox_evidence_bridge(&bridge).expect("projection");

    assert_eq!(projection.role_mailbox_label_count, 2);
    assert_eq!(projection.telemetry_event_count, 3);
    assert_eq!(projection.debug_bundle_export_count, 1);
    assert!(projection.leak_safe);
    assert!(projection.provenance_preserved);
    assert!(!projection.mutates_mailbox_authority);
    assert!(!projection.mutates_debug_bundle_authority);
}

#[test]
fn debug_bundle_export_preview_preserves_stable_provenance_and_recorder_correlation() {
    let bridge = sample_bridge();
    let preview = preview_mailbox_debug_bundle_export(&bridge, "mailbox-export-mt032")
        .expect("export preview exists");

    assert_eq!(preview.debug_bundle_id, "debug-bundle-mt032");
    assert_eq!(preview.stable_provenance_refs.len(), 2);
    assert_eq!(preview.recorder_correlation_ids.len(), 2);
    assert!(preview.bounded_scope);
    assert!(preview.leak_safe);
}

#[test]
fn inbox_evidence_bridge_rejects_parallel_inbox_and_leaky_exports() {
    let mut bridge = sample_bridge();
    bridge.inbox_parallel_semantics_allowed = true;
    bridge.mailbox_telemetry_leak_safe = false;
    bridge.debug_bundle_export_preserves_provenance = false;
    bridge.inbox_label_alignments[0].target = InboxLabelTarget::ParallelInbox;
    bridge.inbox_label_alignments[0].parallel_inbox_semantics_allowed = true;
    bridge.telemetry_events[0].includes_inline_body = true;
    bridge.telemetry_events[0].payload_redacted = false;
    bridge.evidence_exports[0].bounded_scope = false;
    bridge.evidence_exports[0].stable_provenance_refs.clear();

    let errors =
        validate_role_mailbox_inbox_evidence_bridge(&bridge).expect_err("unsafe bridge must fail");

    assert!(errors
        .iter()
        .any(|error| error.field == "inbox_parallel_semantics_allowed"));
    assert!(errors
        .iter()
        .any(|error| error.field == "inbox_label_alignments.target"));
    assert!(errors
        .iter()
        .any(|error| error.field == "telemetry_events.includes_inline_body"));
    assert!(errors
        .iter()
        .any(|error| error.field == "telemetry_events.payload_redacted"));
    assert!(errors
        .iter()
        .any(|error| error.field == "evidence_exports.bounded_scope"));
    assert!(errors
        .iter()
        .any(|error| error.field == "evidence_exports.stable_provenance_refs"));
}

#[test]
fn kernel_action_catalog_exposes_role_mailbox_inbox_evidence_bridge_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.role_mailbox_inbox_evidence_bridge.project")
        .expect("Role Mailbox inbox evidence bridge action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "mailbox_inbox_label_alignment"));
}

fn sample_bridge() -> RoleMailboxInboxEvidenceBridgeV1 {
    RoleMailboxInboxEvidenceBridgeV1 {
        schema_id: "hsk.kernel.role_mailbox_inbox_evidence_bridge@1".to_string(),
        bridge_id: "kernel002-role-mailbox-inbox-evidence-mt032".to_string(),
        folded_stub_ids: vec![
            "WP-1-Inbox-Role-Mailbox-Alignment-v1".to_string(),
            "WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1".to_string(),
        ],
        inbox_label_alignments: vec![
            InboxLabelAlignmentV1 {
                label_id: "inbox-main".to_string(),
                display_label: "Inbox".to_string(),
                target: InboxLabelTarget::RoleMailbox,
                role_mailbox_route: "role-mailbox://inbox".to_string(),
                parallel_inbox_semantics_allowed: false,
            },
            InboxLabelAlignmentV1 {
                label_id: "inbox-triage".to_string(),
                display_label: "Inbox triage".to_string(),
                target: InboxLabelTarget::RoleMailbox,
                role_mailbox_route: "role-mailbox://triage".to_string(),
                parallel_inbox_semantics_allowed: false,
            },
        ],
        telemetry_events: vec![
            telemetry(
                "mailbox-event-created",
                MailboxTelemetryEventKind::MessageCreated,
            ),
            telemetry(
                "mailbox-event-transcription",
                MailboxTelemetryEventKind::TranscriptionLinkUpdated,
            ),
            telemetry(
                "mailbox-event-export",
                MailboxTelemetryEventKind::ExportCompleted,
            ),
        ],
        evidence_exports: vec![DebugBundleMailboxEvidenceExportV1 {
            export_id: "mailbox-export-mt032".to_string(),
            debug_bundle_id: "debug-bundle-mt032".to_string(),
            thread_ids: vec!["role-mailbox-thread-mt032".to_string()],
            message_refs: vec!["message://mt032".to_string()],
            transcription_link_refs: vec!["transcription://mt032".to_string()],
            repo_export_manifest_ref: "manifest://mailbox-mt032".to_string(),
            stable_provenance_refs: vec![
                "provenance://mailbox-message".to_string(),
                "provenance://debug-bundle-export".to_string(),
            ],
            recorder_correlation_ids: vec![
                "FR-EVT-RUNTIME-MAILBOX-101".to_string(),
                "FR-EVT-RUNTIME-MAILBOX-106".to_string(),
            ],
            retention_class: "bounded-mailbox-evidence".to_string(),
            bounded_scope: true,
            leak_safe: true,
        }],
        compact_summary_first: true,
        inbox_parallel_semantics_allowed: false,
        mailbox_telemetry_leak_safe: true,
        debug_bundle_export_preserves_provenance: true,
        product_authority_refs: vec![
            "kernel.role_mailbox_contract".to_string(),
            "kernel.role_mailbox_handoff_bundle".to_string(),
            "kernel.software_delivery_runtime_truth".to_string(),
            "kernel.dcc_structured_artifact_viewer".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Inbox-Role-Mailbox-Alignment-v1.contract.json"
                .to_string(),
            ".GOV/task_packets/stubs/WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1.contract.json"
                .to_string(),
        ],
    }
}

fn telemetry(event_id: &str, kind: MailboxTelemetryEventKind) -> MailboxTelemetryEventV1 {
    MailboxTelemetryEventV1 {
        event_id: event_id.to_string(),
        kind,
        thread_id: "role-mailbox-thread-mt032".to_string(),
        message_ref: "message://mt032".to_string(),
        recorder_correlation_id: format!("recorder://{event_id}"),
        stable_provenance_ref: format!("provenance://{event_id}"),
        includes_inline_body: false,
        payload_redacted: true,
        retention_class: "bounded-mailbox-telemetry".to_string(),
    }
}
