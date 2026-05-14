use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_INBOX_ROLE_MAILBOX_ALIGNMENT_STUB_ID: &str =
    "WP-1-Inbox-Role-Mailbox-Alignment-v1";
pub const FOLDED_ROLE_MAILBOX_DEBUG_BUNDLE_BRIDGE_STUB_ID: &str =
    "WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InboxLabelTarget {
    RoleMailbox,
    ParallelInbox,
    ExternalInbox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MailboxTelemetryEventKind {
    MessageCreated,
    TranscriptionLinkUpdated,
    ExportRequested,
    ExportCompleted,
    ClarificationThreadExported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxLabelAlignmentV1 {
    pub label_id: String,
    pub display_label: String,
    pub target: InboxLabelTarget,
    pub role_mailbox_route: String,
    pub parallel_inbox_semantics_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxTelemetryEventV1 {
    pub event_id: String,
    pub kind: MailboxTelemetryEventKind,
    pub thread_id: String,
    pub message_ref: String,
    pub recorder_correlation_id: String,
    pub stable_provenance_ref: String,
    pub includes_inline_body: bool,
    pub payload_redacted: bool,
    pub retention_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBundleMailboxEvidenceExportV1 {
    pub export_id: String,
    pub debug_bundle_id: String,
    pub thread_ids: Vec<String>,
    pub message_refs: Vec<String>,
    pub transcription_link_refs: Vec<String>,
    pub repo_export_manifest_ref: String,
    pub stable_provenance_refs: Vec<String>,
    pub recorder_correlation_ids: Vec<String>,
    pub retention_class: String,
    pub bounded_scope: bool,
    pub leak_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMailboxInboxEvidenceBridgeV1 {
    pub schema_id: String,
    pub bridge_id: String,
    pub folded_stub_ids: Vec<String>,
    pub inbox_label_alignments: Vec<InboxLabelAlignmentV1>,
    pub telemetry_events: Vec<MailboxTelemetryEventV1>,
    pub evidence_exports: Vec<DebugBundleMailboxEvidenceExportV1>,
    pub compact_summary_first: bool,
    pub inbox_parallel_semantics_allowed: bool,
    pub mailbox_telemetry_leak_safe: bool,
    pub debug_bundle_export_preserves_provenance: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxInboxEvidenceBridgeProjectionV1 {
    pub schema_id: String,
    pub bridge_id: String,
    pub role_mailbox_label_count: usize,
    pub telemetry_event_count: usize,
    pub debug_bundle_export_count: usize,
    pub leak_safe: bool,
    pub provenance_preserved: bool,
    pub mutates_mailbox_authority: bool,
    pub mutates_debug_bundle_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugBundleMailboxEvidenceExportPreviewV1 {
    pub schema_id: String,
    pub export_id: String,
    pub debug_bundle_id: String,
    pub thread_ids: Vec<String>,
    pub message_refs: Vec<String>,
    pub transcription_link_refs: Vec<String>,
    pub repo_export_manifest_ref: String,
    pub stable_provenance_refs: Vec<String>,
    pub recorder_correlation_ids: Vec<String>,
    pub retention_class: String,
    pub bounded_scope: bool,
    pub leak_safe: bool,
    pub mutates_debug_bundle_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleMailboxInboxEvidenceBridgeValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_role_mailbox_inbox_evidence_bridge(
    bridge: &RoleMailboxInboxEvidenceBridgeV1,
) -> Result<(), Vec<RoleMailboxInboxEvidenceBridgeValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &bridge.schema_id);
    require_non_empty(&mut errors, "bridge_id", &bridge.bridge_id);
    require_vec(&mut errors, "folded_stub_ids", &bridge.folded_stub_ids);
    require_vec(
        &mut errors,
        "inbox_label_alignments",
        &bridge.inbox_label_alignments,
    );
    require_vec(&mut errors, "telemetry_events", &bridge.telemetry_events);
    require_vec(&mut errors, "evidence_exports", &bridge.evidence_exports);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &bridge.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &bridge.folded_source_refs,
    );

    if !contains_exact(
        &bridge.folded_stub_ids,
        FOLDED_INBOX_ROLE_MAILBOX_ALIGNMENT_STUB_ID,
    ) || !contains_exact(
        &bridge.folded_stub_ids,
        FOLDED_ROLE_MAILBOX_DEBUG_BUNDLE_BRIDGE_STUB_ID,
    ) {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field: "folded_stub_ids",
            message:
                "bridge must preserve both folded Inbox alignment and Debug Bundle bridge stub ids",
        });
    }
    if !contains_text(
        &bridge.folded_source_refs,
        FOLDED_INBOX_ROLE_MAILBOX_ALIGNMENT_STUB_ID,
    ) || !contains_text(
        &bridge.folded_source_refs,
        FOLDED_ROLE_MAILBOX_DEBUG_BUNDLE_BRIDGE_STUB_ID,
    ) {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field: "folded_source_refs",
            message: "bridge must preserve both folded source refs",
        });
    }
    if !bridge.compact_summary_first {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field: "compact_summary_first",
            message: "inbox evidence bridge projections must be compact-summary-first",
        });
    }
    if bridge.inbox_parallel_semantics_allowed {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field: "inbox_parallel_semantics_allowed",
            message: "Inbox semantics must map to Role Mailbox only",
        });
    }
    if !bridge.mailbox_telemetry_leak_safe {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field: "mailbox_telemetry_leak_safe",
            message: "mailbox telemetry must be leak-safe",
        });
    }
    if !bridge.debug_bundle_export_preserves_provenance {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field: "debug_bundle_export_preserves_provenance",
            message: "debug bundle exports must preserve stable mailbox provenance",
        });
    }

    validate_refs(&mut errors, bridge);
    validate_label_alignments(&mut errors, &bridge.inbox_label_alignments);
    validate_telemetry_events(&mut errors, &bridge.telemetry_events);
    validate_evidence_exports(&mut errors, &bridge.evidence_exports);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn project_role_mailbox_inbox_evidence_bridge(
    bridge: &RoleMailboxInboxEvidenceBridgeV1,
) -> Result<
    RoleMailboxInboxEvidenceBridgeProjectionV1,
    Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
> {
    validate_role_mailbox_inbox_evidence_bridge(bridge)?;

    Ok(RoleMailboxInboxEvidenceBridgeProjectionV1 {
        schema_id: "hsk.kernel.role_mailbox_inbox_evidence_bridge_projection@1".to_string(),
        bridge_id: bridge.bridge_id.clone(),
        role_mailbox_label_count: bridge
            .inbox_label_alignments
            .iter()
            .filter(|label| label.target == InboxLabelTarget::RoleMailbox)
            .count(),
        telemetry_event_count: bridge.telemetry_events.len(),
        debug_bundle_export_count: bridge.evidence_exports.len(),
        leak_safe: bridge.mailbox_telemetry_leak_safe
            && bridge
                .evidence_exports
                .iter()
                .all(|export| export.leak_safe),
        provenance_preserved: bridge.debug_bundle_export_preserves_provenance
            && bridge
                .evidence_exports
                .iter()
                .all(|export| !export.stable_provenance_refs.is_empty()),
        mutates_mailbox_authority: false,
        mutates_debug_bundle_authority: false,
    })
}

pub fn preview_mailbox_debug_bundle_export(
    bridge: &RoleMailboxInboxEvidenceBridgeV1,
    export_id: &str,
) -> Result<
    DebugBundleMailboxEvidenceExportPreviewV1,
    Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
> {
    validate_role_mailbox_inbox_evidence_bridge(bridge)?;

    let Some(export) = bridge
        .evidence_exports
        .iter()
        .find(|export| export.export_id == export_id)
    else {
        return Err(vec![RoleMailboxInboxEvidenceBridgeValidationError {
            field: "export_id",
            message: "requested mailbox debug bundle export is not registered",
        }]);
    };

    Ok(DebugBundleMailboxEvidenceExportPreviewV1 {
        schema_id: "hsk.kernel.mailbox_debug_bundle_export_preview@1".to_string(),
        export_id: export.export_id.clone(),
        debug_bundle_id: export.debug_bundle_id.clone(),
        thread_ids: export.thread_ids.clone(),
        message_refs: export.message_refs.clone(),
        transcription_link_refs: export.transcription_link_refs.clone(),
        repo_export_manifest_ref: export.repo_export_manifest_ref.clone(),
        stable_provenance_refs: export.stable_provenance_refs.clone(),
        recorder_correlation_ids: export.recorder_correlation_ids.clone(),
        retention_class: export.retention_class.clone(),
        bounded_scope: export.bounded_scope,
        leak_safe: export.leak_safe,
        mutates_debug_bundle_authority: false,
    })
}

fn validate_refs(
    errors: &mut Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
    bridge: &RoleMailboxInboxEvidenceBridgeV1,
) {
    for required_ref in [
        "kernel.role_mailbox_contract",
        "kernel.role_mailbox_handoff_bundle",
        "kernel.software_delivery_runtime_truth",
        "kernel.dcc_structured_artifact_viewer",
    ] {
        if !contains_exact(&bridge.product_authority_refs, required_ref) {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "product_authority_refs",
                message: "inbox evidence bridge must cite Role Mailbox, handoff, runtime truth, and DCC refs",
            });
        }
    }
}

fn validate_label_alignments(
    errors: &mut Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
    labels: &[InboxLabelAlignmentV1],
) {
    let mut label_ids = HashSet::new();
    for label in labels {
        if !label_ids.insert(label.label_id.as_str()) {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "inbox_label_alignments.label_id",
                message: "inbox label ids must be unique",
            });
        }
        require_non_empty(errors, "inbox_label_alignments.label_id", &label.label_id);
        require_non_empty(
            errors,
            "inbox_label_alignments.display_label",
            &label.display_label,
        );
        require_non_empty(
            errors,
            "inbox_label_alignments.role_mailbox_route",
            &label.role_mailbox_route,
        );
        if label.target != InboxLabelTarget::RoleMailbox {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "inbox_label_alignments.target",
                message: "Inbox labels must map to Role Mailbox only",
            });
        }
        if label.parallel_inbox_semantics_allowed {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "inbox_label_alignments.parallel_inbox_semantics_allowed",
                message: "parallel Inbox semantics are not allowed",
            });
        }
    }
}

fn validate_telemetry_events(
    errors: &mut Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
    events: &[MailboxTelemetryEventV1],
) {
    let mut event_ids = HashSet::new();
    for event in events {
        if !event_ids.insert(event.event_id.as_str()) {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "telemetry_events.event_id",
                message: "mailbox telemetry event ids must be unique",
            });
        }
        require_non_empty(errors, "telemetry_events.event_id", &event.event_id);
        require_non_empty(errors, "telemetry_events.thread_id", &event.thread_id);
        require_non_empty(errors, "telemetry_events.message_ref", &event.message_ref);
        require_non_empty(
            errors,
            "telemetry_events.recorder_correlation_id",
            &event.recorder_correlation_id,
        );
        require_non_empty(
            errors,
            "telemetry_events.stable_provenance_ref",
            &event.stable_provenance_ref,
        );
        require_non_empty(
            errors,
            "telemetry_events.retention_class",
            &event.retention_class,
        );
        if event.includes_inline_body {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "telemetry_events.includes_inline_body",
                message: "mailbox telemetry must not include inline message bodies",
            });
        }
        if !event.payload_redacted {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "telemetry_events.payload_redacted",
                message: "mailbox telemetry payloads must be redacted",
            });
        }
    }
}

fn validate_evidence_exports(
    errors: &mut Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
    exports: &[DebugBundleMailboxEvidenceExportV1],
) {
    let mut export_ids = HashSet::new();
    for export in exports {
        if !export_ids.insert(export.export_id.as_str()) {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "evidence_exports.export_id",
                message: "mailbox evidence export ids must be unique",
            });
        }
        require_non_empty(errors, "evidence_exports.export_id", &export.export_id);
        require_non_empty(
            errors,
            "evidence_exports.debug_bundle_id",
            &export.debug_bundle_id,
        );
        require_vec(errors, "evidence_exports.thread_ids", &export.thread_ids);
        require_vec(
            errors,
            "evidence_exports.message_refs",
            &export.message_refs,
        );
        require_vec(
            errors,
            "evidence_exports.transcription_link_refs",
            &export.transcription_link_refs,
        );
        require_non_empty(
            errors,
            "evidence_exports.repo_export_manifest_ref",
            &export.repo_export_manifest_ref,
        );
        require_vec(
            errors,
            "evidence_exports.stable_provenance_refs",
            &export.stable_provenance_refs,
        );
        require_vec(
            errors,
            "evidence_exports.recorder_correlation_ids",
            &export.recorder_correlation_ids,
        );
        require_non_empty(
            errors,
            "evidence_exports.retention_class",
            &export.retention_class,
        );
        if !export.bounded_scope {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "evidence_exports.bounded_scope",
                message: "mailbox debug bundle exports must be bounded in scope",
            });
        }
        if !export.leak_safe {
            errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
                field: "evidence_exports.leak_safe",
                message: "mailbox debug bundle exports must be leak-safe",
            });
        }
    }
}

fn require_non_empty(
    errors: &mut Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<RoleMailboxInboxEvidenceBridgeValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(RoleMailboxInboxEvidenceBridgeValidationError {
            field,
            message: "at least one value is required",
        });
    }
}

fn contains_exact(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value == needle)
}

fn contains_text(values: &[String], needle: &str) -> bool {
    values.iter().any(|value| value.contains(needle))
}
