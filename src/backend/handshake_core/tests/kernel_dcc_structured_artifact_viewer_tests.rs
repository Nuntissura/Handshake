use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
    dcc_structured_artifact_viewer::{
        derive_dcc_structured_artifact_layout, render_dcc_structured_artifact_view,
        validate_dcc_structured_artifact_viewer, DccArtifactLayoutKind, DccCanonicalFieldV1,
        DccFieldProvenance, DccMirrorState, DccMirrorViewV1, DccRawDrilldownMode,
        DccRenderedSectionKind, DccStructuredArtifactKind, DccStructuredArtifactRecordV1,
        DccStructuredArtifactViewerV1,
    },
};

#[test]
fn structured_artifact_viewer_renders_canonical_fields_before_mirrors_and_raw_advanced() {
    let viewer = sample_viewer();

    validate_dcc_structured_artifact_viewer(&viewer).expect("viewer validates");
    let rendered =
        render_dcc_structured_artifact_view(&viewer, "wp-record-1").expect("record should render");

    assert_eq!(
        rendered.sections[0].section_kind,
        DccRenderedSectionKind::CanonicalFields
    );
    assert_eq!(
        rendered.sections[1].section_kind,
        DccRenderedSectionKind::Mirror
    );
    assert_eq!(
        rendered.sections[2].section_kind,
        DccRenderedSectionKind::RawDrilldown
    );
    assert_eq!(rendered.mirror_state, DccMirrorState::Stale);
    assert!(rendered.raw_drilldown_available);
    assert!(!rendered.raw_drilldown_visible_by_default);
}

#[test]
fn structured_artifact_viewer_derives_multiple_projection_layouts_from_same_records() {
    let viewer = sample_viewer();

    let kanban = derive_dcc_structured_artifact_layout(&viewer, DccArtifactLayoutKind::Kanban)
        .expect("kanban layout derives");
    let queue = derive_dcc_structured_artifact_layout(&viewer, DccArtifactLayoutKind::Queue)
        .expect("queue layout derives");

    assert_eq!(kanban.rows.len(), viewer.records.len());
    assert_eq!(queue.rows.len(), viewer.records.len());
    assert!(!kanban.mutates_authority);
    assert!(!queue.mutates_authority);
    assert_eq!(kanban.rows[0].record_id, queue.rows[0].record_id);
}

#[test]
fn structured_artifact_viewer_rejects_raw_default_and_mirror_only_surfaces() {
    let mut viewer = sample_viewer();
    viewer.canonical_fields_render_first = false;
    viewer.raw_drilldown_advanced_only = false;
    viewer.records[0].raw_drilldown_mode = DccRawDrilldownMode::DefaultVisible;

    let errors = validate_dcc_structured_artifact_viewer(&viewer)
        .expect_err("viewer must not default to mirror/raw surfaces");

    assert!(errors
        .iter()
        .any(|error| error.field == "canonical_fields_render_first"));
    assert!(errors
        .iter()
        .any(|error| error.field == "raw_drilldown_advanced_only"));
    assert!(errors
        .iter()
        .any(|error| error.field == "raw_drilldown_mode"));
}

#[test]
fn kernel_action_catalog_exposes_structured_artifact_viewer_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.dcc_structured_artifact_viewer.project")
        .expect("structured artifact viewer projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "canonical_fields_first"));
}

fn sample_viewer() -> DccStructuredArtifactViewerV1 {
    DccStructuredArtifactViewerV1 {
        schema_id: "hsk.kernel.dcc_structured_artifact_viewer@1".to_string(),
        viewer_id: "kernel002-dcc-structured-artifact-viewer-mt025".to_string(),
        folded_stub_id: "WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1".to_string(),
        records: vec![
            record(
                "wp-record-1",
                DccStructuredArtifactKind::WorkPacket,
                DccMirrorState::Stale,
                "Ready for Dev",
            ),
            record(
                "mt-record-1",
                DccStructuredArtifactKind::MicroTask,
                DccMirrorState::InSync,
                "Pending",
            ),
            record(
                "task-board-row-1",
                DccStructuredArtifactKind::TaskBoardEntry,
                DccMirrorState::AdvisoryEditPending,
                "Active",
            ),
            record(
                "mailbox-thread-1",
                DccStructuredArtifactKind::RoleMailboxThread,
                DccMirrorState::Missing,
                "Waiting",
            ),
        ],
        layouts: vec![
            DccArtifactLayoutKind::Kanban,
            DccArtifactLayoutKind::Queue,
            DccArtifactLayoutKind::List,
        ],
        default_layout: DccArtifactLayoutKind::Kanban,
        canonical_fields_render_first: true,
        mirror_state_visible: true,
        raw_drilldown_advanced_only: true,
        direct_layout_mutation_allowed: false,
        product_authority_refs: vec![
            "kernel.structured_artifact_records".to_string(),
            "kernel.mirror_advisory".to_string(),
            "kernel.dcc_mvp_runtime".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1.contract.json".to_string(),
            ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1.md".to_string(),
        ],
    }
}

fn record(
    record_id: &str,
    artifact_kind: DccStructuredArtifactKind,
    mirror_state: DccMirrorState,
    status: &str,
) -> DccStructuredArtifactRecordV1 {
    DccStructuredArtifactRecordV1 {
        record_id: record_id.to_string(),
        artifact_kind,
        canonical_fields: vec![
            DccCanonicalFieldV1 {
                field_id: "status".to_string(),
                label: "Status".to_string(),
                value: status.to_string(),
                provenance: DccFieldProvenance::Canonical,
                display_order: 10,
            },
            DccCanonicalFieldV1 {
                field_id: "owner".to_string(),
                label: "Owner".to_string(),
                value: "KERNEL_BUILDER".to_string(),
                provenance: DccFieldProvenance::Canonical,
                display_order: 20,
            },
        ],
        mirror: Some(DccMirrorViewV1 {
            mirror_ref: format!("mirror://{record_id}.md"),
            state: mirror_state,
            content_preview: "Generated mirror preview".to_string(),
            reconciliation_action_id: Some("kernel.mirror_advisory.capture".to_string()),
        }),
        raw_structured_ref: format!("structured://{record_id}.json"),
        raw_drilldown_mode: DccRawDrilldownMode::AdvancedOnly,
        authority_refs: vec!["kernel.structured_artifact_records".to_string()],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1.contract.json".to_string(),
        ],
    }
}
