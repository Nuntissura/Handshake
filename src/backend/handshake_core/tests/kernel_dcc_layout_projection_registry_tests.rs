use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::{ApprovalPosture, AuthorityEffect},
    dcc_layout_projection_registry::{
        derive_dcc_layout_projection, preview_dcc_projection_action,
        validate_dcc_layout_projection_registry, DccActionEvidencePosture, DccLayoutKind,
        DccLayoutProjectionRegistryV1, DccPresetFallbackMode, DccProjectionGestureKind,
        DevCommandCenterViewPresetV1, ProjectionActionBindingV1, TaskBoardLaneDefinitionV1,
    },
};

#[test]
fn layout_registry_validates_all_required_presets_over_one_record_family() {
    let registry = sample_registry();

    validate_dcc_layout_projection_registry(&registry).expect("layout registry validates");

    for layout_kind in required_layouts() {
        assert!(
            registry
                .presets
                .iter()
                .any(|preset| preset.layout_kind == layout_kind),
            "missing preset for {layout_kind:?}"
        );
    }
    assert!(registry
        .presets
        .iter()
        .all(|preset| preset.canonical_record_family_id == "structured-collaboration-artifact"));
}

#[test]
fn layout_registry_derives_views_and_previews_gestures_without_authority_mutation() {
    let registry = sample_registry();
    let available_fields = vec![
        "status".to_string(),
        "priority".to_string(),
        "due_date".to_string(),
        "mailbox.blocker_count".to_string(),
        "escalation_posture".to_string(),
        "validation_posture".to_string(),
        "hsk.base_envelope@1".to_string(),
    ];

    let board = derive_dcc_layout_projection(&registry, DccLayoutKind::Board, &available_fields)
        .expect("board layout derives");
    assert_eq!(board.layout_kind, DccLayoutKind::Board);
    assert!(!board.mutates_authority);
    assert!(!board.fallback_active);
    assert!(board
        .action_previews
        .iter()
        .any(|preview| preview.gesture == DccProjectionGestureKind::Drag));

    let quick_action = preview_dcc_projection_action(
        &registry,
        "dcc-board-v1",
        DccProjectionGestureKind::QuickAction,
        "kernel.workflow_transition.preview",
    )
    .expect("quick action preview exists");
    assert!(quick_action.preview_required);
    assert!(!quick_action.mutates_on_gesture);
    assert_eq!(
        quick_action.approval_posture,
        ApprovalPosture::NoApprovalRequired
    );
}

#[test]
fn layout_registry_falls_back_to_base_envelope_when_view_fields_are_missing() {
    let registry = sample_registry();
    let only_base_envelope = vec!["hsk.base_envelope@1".to_string()];

    let list = derive_dcc_layout_projection(&registry, DccLayoutKind::List, &only_base_envelope)
        .expect("base envelope fallback derives");

    assert!(list.fallback_active);
    assert_eq!(list.lanes[0].lane_id, "base-envelope-fallback");
    assert!(list
        .visible_field_paths
        .contains(&"hsk.base_envelope@1".to_string()));
}

#[test]
fn layout_registry_rejects_layout_local_mutation_and_long_form_local_model_queues() {
    let mut registry = sample_registry();
    registry.direct_layout_mutation_allowed = true;
    registry.presets[0].action_bindings[0].preview_required = false;
    registry.presets[0].action_bindings[0].mutates_on_gesture = true;
    let execution_queue = registry
        .presets
        .iter_mut()
        .find(|preset| preset.layout_kind == DccLayoutKind::ExecutionQueue)
        .expect("execution queue preset exists");
    execution_queue.compact_summary_first = false;
    execution_queue.long_form_record_ingestion_required = true;

    let errors = validate_dcc_layout_projection_registry(&registry)
        .expect_err("unsafe layout registry must be rejected");

    assert!(errors
        .iter()
        .any(|error| error.field == "direct_layout_mutation_allowed"));
    assert!(errors
        .iter()
        .any(|error| error.field == "action_bindings.preview_required"));
    assert!(errors
        .iter()
        .any(|error| error.field == "compact_summary_first"));
    assert!(errors
        .iter()
        .any(|error| error.field == "long_form_record_ingestion_required"));
}

#[test]
fn kernel_action_catalog_exposes_dcc_layout_projection_registry_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.dcc_layout_projection_registry.project")
        .expect("DCC layout projection registry action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "dcc_layout_presets_registered"));
}

fn sample_registry() -> DccLayoutProjectionRegistryV1 {
    DccLayoutProjectionRegistryV1 {
        schema_id: "hsk.kernel.dcc_layout_projection_registry@1".to_string(),
        registry_id: "kernel002-dcc-layout-registry-mt026".to_string(),
        folded_stub_id: "WP-1-Dev-Command-Center-Layout-Projection-Registry-v1".to_string(),
        presets: required_layouts()
            .into_iter()
            .map(|layout_kind| preset(layout_kind))
            .collect(),
        direct_layout_mutation_allowed: false,
        product_authority_refs: vec![
            "kernel.dcc_structured_artifact_viewer".to_string(),
            "kernel.action_catalog".to_string(),
            "kernel.workflow_transition_registry".to_string(),
            "kernel.role_mailbox".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-Layout-Projection-Registry-v1.contract.json".to_string(),
            ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-Layout-Projection-Registry-v1.md".to_string(),
        ],
    }
}

fn required_layouts() -> Vec<DccLayoutKind> {
    vec![
        DccLayoutKind::Board,
        DccLayoutKind::Queue,
        DccLayoutKind::List,
        DccLayoutKind::Roadmap,
        DccLayoutKind::InboxTriage,
        DccLayoutKind::ExecutionQueue,
    ]
}

fn preset(layout_kind: DccLayoutKind) -> DevCommandCenterViewPresetV1 {
    let preset_id = match layout_kind {
        DccLayoutKind::Board => "dcc-board-v1",
        DccLayoutKind::Queue => "dcc-queue-v1",
        DccLayoutKind::List => "dcc-list-v1",
        DccLayoutKind::Roadmap => "dcc-roadmap-v1",
        DccLayoutKind::InboxTriage => "dcc-inbox-triage-v1",
        DccLayoutKind::ExecutionQueue => "dcc-execution-queue-v1",
    };

    DevCommandCenterViewPresetV1 {
        preset_id: preset_id.to_string(),
        version: 1,
        layout_kind,
        canonical_record_family_id: "structured-collaboration-artifact".to_string(),
        base_envelope_schema_id: "hsk.base_envelope@1".to_string(),
        grouping_field_paths: vec!["status".to_string()],
        sort_field_paths: vec!["priority".to_string(), "due_date".to_string()],
        lane_definitions: vec![
            lane("lane-ready", "status", &["READY"]),
            lane("lane-active", "status", &["ACTIVE"]),
        ],
        action_bindings: vec![
            binding("drag-transition", DccProjectionGestureKind::Drag),
            binding("reorder-priority", DccProjectionGestureKind::Reorder),
            binding("quick-preview", DccProjectionGestureKind::QuickAction),
            binding("bulk-transition", DccProjectionGestureKind::BulkAction),
        ],
        fallback_mode: DccPresetFallbackMode::BaseEnvelope,
        compact_summary_first: matches!(
            layout_kind,
            DccLayoutKind::InboxTriage | DccLayoutKind::ExecutionQueue
        ),
        local_small_model_safe: matches!(
            layout_kind,
            DccLayoutKind::InboxTriage | DccLayoutKind::ExecutionQueue
        ),
        long_form_record_ingestion_required: false,
        exposed_blocker_fields: vec![
            "mailbox.blocker_count".to_string(),
            "escalation_posture".to_string(),
            "validation_posture".to_string(),
        ],
    }
}

fn lane(id: &str, field: &str, values: &[&str]) -> TaskBoardLaneDefinitionV1 {
    TaskBoardLaneDefinitionV1 {
        lane_id: id.to_string(),
        label: id.replace("lane-", ""),
        match_field_path: field.to_string(),
        match_values: values.iter().map(|value| (*value).to_string()).collect(),
        sort_field_paths: vec!["priority".to_string()],
        wip_limit: Some(4),
        projection_only: true,
        source_field_refs: vec![field.to_string()],
    }
}

fn binding(id: &str, gesture: DccProjectionGestureKind) -> ProjectionActionBindingV1 {
    ProjectionActionBindingV1 {
        binding_id: id.to_string(),
        gesture,
        action_id: "kernel.workflow_transition.preview".to_string(),
        target_record_selector: "selected_records".to_string(),
        target_field_paths: vec!["status".to_string()],
        approval_posture: ApprovalPosture::NoApprovalRequired,
        evidence_posture: DccActionEvidencePosture::Required,
        preview_required: true,
        mutates_on_gesture: false,
    }
}
