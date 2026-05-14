use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_envelope::ApprovalPosture;

pub const FOLDED_DCC_LAYOUT_PROJECTION_REGISTRY_STUB_ID: &str =
    "WP-1-Dev-Command-Center-Layout-Projection-Registry-v1";

const REQUIRED_LAYOUT_KINDS: [DccLayoutKind; 6] = [
    DccLayoutKind::Board,
    DccLayoutKind::Queue,
    DccLayoutKind::List,
    DccLayoutKind::Roadmap,
    DccLayoutKind::InboxTriage,
    DccLayoutKind::ExecutionQueue,
];

const REQUIRED_GESTURES: [DccProjectionGestureKind; 4] = [
    DccProjectionGestureKind::Drag,
    DccProjectionGestureKind::Reorder,
    DccProjectionGestureKind::QuickAction,
    DccProjectionGestureKind::BulkAction,
];

const LOCAL_QUEUE_REQUIRED_FIELDS: [&str; 3] = [
    "mailbox.blocker_count",
    "escalation_posture",
    "validation_posture",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccLayoutKind {
    Board,
    Queue,
    List,
    Roadmap,
    InboxTriage,
    ExecutionQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccProjectionGestureKind {
    Drag,
    Reorder,
    QuickAction,
    BulkAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccPresetFallbackMode {
    BaseEnvelope,
    NoFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccActionEvidencePosture {
    Required,
    Optional,
    NotDeclared,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardLaneDefinitionV1 {
    pub lane_id: String,
    pub label: String,
    pub match_field_path: String,
    pub match_values: Vec<String>,
    pub sort_field_paths: Vec<String>,
    pub wip_limit: Option<u32>,
    pub projection_only: bool,
    pub source_field_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionActionBindingV1 {
    pub binding_id: String,
    pub gesture: DccProjectionGestureKind,
    pub action_id: String,
    pub target_record_selector: String,
    pub target_field_paths: Vec<String>,
    pub approval_posture: ApprovalPosture,
    pub evidence_posture: DccActionEvidencePosture,
    pub preview_required: bool,
    pub mutates_on_gesture: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevCommandCenterViewPresetV1 {
    pub preset_id: String,
    pub version: u32,
    pub layout_kind: DccLayoutKind,
    pub canonical_record_family_id: String,
    pub base_envelope_schema_id: String,
    pub grouping_field_paths: Vec<String>,
    pub sort_field_paths: Vec<String>,
    pub lane_definitions: Vec<TaskBoardLaneDefinitionV1>,
    pub action_bindings: Vec<ProjectionActionBindingV1>,
    pub fallback_mode: DccPresetFallbackMode,
    pub compact_summary_first: bool,
    pub local_small_model_safe: bool,
    pub long_form_record_ingestion_required: bool,
    pub exposed_blocker_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccLayoutProjectionRegistryV1 {
    pub schema_id: String,
    pub registry_id: String,
    pub folded_stub_id: String,
    pub presets: Vec<DevCommandCenterViewPresetV1>,
    pub direct_layout_mutation_allowed: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccProjectionActionPreviewV1 {
    pub binding_id: String,
    pub gesture: DccProjectionGestureKind,
    pub action_id: String,
    pub target_record_selector: String,
    pub target_field_paths: Vec<String>,
    pub approval_posture: ApprovalPosture,
    pub evidence_posture: DccActionEvidencePosture,
    pub preview_required: bool,
    pub mutates_on_gesture: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccLayoutProjectionViewV1 {
    pub schema_id: String,
    pub preset_id: String,
    pub layout_kind: DccLayoutKind,
    pub canonical_record_family_id: String,
    pub lanes: Vec<TaskBoardLaneDefinitionV1>,
    pub action_previews: Vec<DccProjectionActionPreviewV1>,
    pub visible_field_paths: Vec<String>,
    pub fallback_active: bool,
    pub compact_summary_first: bool,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccLayoutProjectionRegistryValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_dcc_layout_projection_registry(
    registry: &DccLayoutProjectionRegistryV1,
) -> Result<(), Vec<DccLayoutProjectionRegistryValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &registry.schema_id);
    require_non_empty(&mut errors, "registry_id", &registry.registry_id);
    require_non_empty(&mut errors, "folded_stub_id", &registry.folded_stub_id);
    require_vec(&mut errors, "presets", &registry.presets);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &registry.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &registry.folded_source_refs,
    );

    if registry.folded_stub_id != FOLDED_DCC_LAYOUT_PROJECTION_REGISTRY_STUB_ID {
        errors.push(DccLayoutProjectionRegistryValidationError {
            field: "folded_stub_id",
            message: "registry must bind the folded DCC layout projection registry stub",
        });
    }

    if !contains_text(
        &registry.folded_source_refs,
        FOLDED_DCC_LAYOUT_PROJECTION_REGISTRY_STUB_ID,
    ) {
        errors.push(DccLayoutProjectionRegistryValidationError {
            field: "folded_source_refs",
            message: "folded layout projection registry source must be preserved",
        });
    }

    if registry.direct_layout_mutation_allowed {
        errors.push(DccLayoutProjectionRegistryValidationError {
            field: "direct_layout_mutation_allowed",
            message: "layout gestures and view switches must not mutate authority directly",
        });
    }

    for required_ref in [
        "kernel.dcc_structured_artifact_viewer",
        "kernel.action_catalog",
        "kernel.workflow_transition_registry",
        "kernel.role_mailbox",
    ] {
        if !contains_exact(&registry.product_authority_refs, required_ref) {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "product_authority_refs",
                message: "registry must cite DCC viewer, action catalog, workflow, and mailbox authority refs",
            });
        }
    }

    validate_registry_presets(&mut errors, &registry.presets);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn derive_dcc_layout_projection(
    registry: &DccLayoutProjectionRegistryV1,
    layout_kind: DccLayoutKind,
    available_field_paths: &[String],
) -> Result<DccLayoutProjectionViewV1, Vec<DccLayoutProjectionRegistryValidationError>> {
    validate_dcc_layout_projection_registry(registry)?;

    let Some(preset) = registry
        .presets
        .iter()
        .find(|preset| preset.layout_kind == layout_kind)
    else {
        return Err(vec![DccLayoutProjectionRegistryValidationError {
            field: "layout_kind",
            message: "requested layout kind is not registered",
        }]);
    };

    let fallback_active = required_preset_fields(preset)
        .iter()
        .any(|field| !contains_exact(available_field_paths, field));

    let lanes = if fallback_active {
        if preset.fallback_mode != DccPresetFallbackMode::BaseEnvelope
            || !contains_exact(available_field_paths, &preset.base_envelope_schema_id)
        {
            return Err(vec![DccLayoutProjectionRegistryValidationError {
                field: "fallback_mode",
                message: "missing layout fields require a base-envelope fallback",
            }]);
        }
        vec![base_envelope_lane(&preset.base_envelope_schema_id)]
    } else {
        preset.lane_definitions.clone()
    };

    Ok(DccLayoutProjectionViewV1 {
        schema_id: "hsk.kernel.dcc_layout_projection_view@1".to_string(),
        preset_id: preset.preset_id.clone(),
        layout_kind: preset.layout_kind,
        canonical_record_family_id: preset.canonical_record_family_id.clone(),
        lanes,
        action_previews: preset.action_bindings.iter().map(action_preview).collect(),
        visible_field_paths: visible_field_paths(preset, fallback_active),
        fallback_active,
        compact_summary_first: preset.compact_summary_first,
        mutates_authority: false,
    })
}

pub fn preview_dcc_projection_action(
    registry: &DccLayoutProjectionRegistryV1,
    preset_id: &str,
    gesture: DccProjectionGestureKind,
    action_id: &str,
) -> Result<DccProjectionActionPreviewV1, Vec<DccLayoutProjectionRegistryValidationError>> {
    validate_dcc_layout_projection_registry(registry)?;

    let Some(preset) = registry
        .presets
        .iter()
        .find(|preset| preset.preset_id == preset_id)
    else {
        return Err(vec![DccLayoutProjectionRegistryValidationError {
            field: "preset_id",
            message: "requested preset is not registered",
        }]);
    };

    let Some(binding) = preset
        .action_bindings
        .iter()
        .find(|binding| binding.gesture == gesture && binding.action_id == action_id)
    else {
        return Err(vec![DccLayoutProjectionRegistryValidationError {
            field: "action_bindings",
            message: "requested gesture/action binding is not registered",
        }]);
    };

    Ok(action_preview(binding))
}

fn validate_registry_presets(
    errors: &mut Vec<DccLayoutProjectionRegistryValidationError>,
    presets: &[DevCommandCenterViewPresetV1],
) {
    for required_kind in REQUIRED_LAYOUT_KINDS {
        if !presets
            .iter()
            .any(|preset| preset.layout_kind == required_kind)
        {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "presets.layout_kind",
                message: "board, queue, list, roadmap, inbox-triage, and execution-queue presets are required",
            });
        }
    }

    let mut preset_ids = HashSet::new();
    let mut layout_kinds = HashSet::new();
    let mut record_family: Option<&str> = None;
    let mut gestures = HashSet::new();

    for preset in presets {
        if !preset_ids.insert(preset.preset_id.as_str()) {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "preset_id",
                message: "preset ids must be unique",
            });
        }
        if !layout_kinds.insert(preset.layout_kind) {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "presets.layout_kind",
                message: "only one preset per layout kind is allowed",
            });
        }

        if let Some(expected_family) = record_family {
            if expected_family != preset.canonical_record_family_id {
                errors.push(DccLayoutProjectionRegistryValidationError {
                    field: "canonical_record_family_id",
                    message:
                        "all DCC layout presets must derive from the same canonical record family",
                });
            }
        } else if !preset.canonical_record_family_id.trim().is_empty() {
            record_family = Some(preset.canonical_record_family_id.as_str());
        }

        validate_preset(errors, preset);
        for binding in &preset.action_bindings {
            gestures.insert(binding.gesture);
        }
    }

    for required_gesture in REQUIRED_GESTURES {
        if !gestures.contains(&required_gesture) {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "action_bindings.gesture",
                message: "drag, reorder, quick, and bulk action bindings are required",
            });
        }
    }
}

fn validate_preset(
    errors: &mut Vec<DccLayoutProjectionRegistryValidationError>,
    preset: &DevCommandCenterViewPresetV1,
) {
    require_non_empty(errors, "preset_id", &preset.preset_id);
    require_non_empty(
        errors,
        "canonical_record_family_id",
        &preset.canonical_record_family_id,
    );
    require_non_empty(
        errors,
        "base_envelope_schema_id",
        &preset.base_envelope_schema_id,
    );
    require_vec(errors, "grouping_field_paths", &preset.grouping_field_paths);
    require_vec(errors, "sort_field_paths", &preset.sort_field_paths);
    require_vec(errors, "lane_definitions", &preset.lane_definitions);
    require_vec(errors, "action_bindings", &preset.action_bindings);

    if preset.version == 0 {
        errors.push(DccLayoutProjectionRegistryValidationError {
            field: "version",
            message: "preset version must be non-zero",
        });
    }

    if preset.fallback_mode == DccPresetFallbackMode::NoFallback {
        errors.push(DccLayoutProjectionRegistryValidationError {
            field: "fallback_mode",
            message: "preset must keep a base-envelope fallback usable",
        });
    }

    if is_local_small_model_queue(preset.layout_kind) {
        if !preset.compact_summary_first {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "compact_summary_first",
                message: "local-small-model queue presets must be compact-summary-first",
            });
        }
        if !preset.local_small_model_safe {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "local_small_model_safe",
                message: "local-small-model queue presets must be marked safe for bounded context",
            });
        }
        if preset.long_form_record_ingestion_required {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "long_form_record_ingestion_required",
                message: "local-small-model queues must not require long-form record ingestion",
            });
        }
        for required_field in LOCAL_QUEUE_REQUIRED_FIELDS {
            if !contains_exact(&preset.exposed_blocker_fields, required_field) {
                errors.push(DccLayoutProjectionRegistryValidationError {
                    field: "exposed_blocker_fields",
                    message: "local-small-model queues must expose blockers, escalation, and validation posture",
                });
            }
        }
    }

    validate_lanes(errors, &preset.lane_definitions);
    validate_action_bindings(errors, &preset.action_bindings);
}

fn validate_lanes(
    errors: &mut Vec<DccLayoutProjectionRegistryValidationError>,
    lanes: &[TaskBoardLaneDefinitionV1],
) {
    let mut lane_ids = HashSet::new();
    for lane in lanes {
        if !lane_ids.insert(lane.lane_id.as_str()) {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "lane_id",
                message: "lane ids must be unique per preset",
            });
        }
        require_non_empty(errors, "lane_id", &lane.lane_id);
        require_non_empty(errors, "lane.label", &lane.label);
        require_non_empty(errors, "lane.match_field_path", &lane.match_field_path);
        require_vec(errors, "lane.match_values", &lane.match_values);
        require_vec(errors, "lane.sort_field_paths", &lane.sort_field_paths);
        require_vec(errors, "lane.source_field_refs", &lane.source_field_refs);
        if !lane.projection_only {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "lane.projection_only",
                message: "lane definitions must be projection-only",
            });
        }
    }
}

fn validate_action_bindings(
    errors: &mut Vec<DccLayoutProjectionRegistryValidationError>,
    bindings: &[ProjectionActionBindingV1],
) {
    let mut binding_ids = HashSet::new();
    for binding in bindings {
        if !binding_ids.insert(binding.binding_id.as_str()) {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "binding_id",
                message: "action binding ids must be unique per preset",
            });
        }
        require_non_empty(errors, "binding_id", &binding.binding_id);
        require_non_empty(errors, "action_id", &binding.action_id);
        require_non_empty(
            errors,
            "target_record_selector",
            &binding.target_record_selector,
        );
        require_vec(errors, "target_field_paths", &binding.target_field_paths);

        if !binding.preview_required {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "action_bindings.preview_required",
                message: "drag, reorder, quick, and bulk actions must preview before state changes",
            });
        }
        if binding.mutates_on_gesture {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "action_bindings.mutates_on_gesture",
                message: "layout gestures must not mutate authority without a governed action",
            });
        }
        if binding.evidence_posture == DccActionEvidencePosture::NotDeclared {
            errors.push(DccLayoutProjectionRegistryValidationError {
                field: "action_bindings.evidence_posture",
                message: "action bindings must declare evidence posture",
            });
        }
    }
}

fn required_preset_fields(preset: &DevCommandCenterViewPresetV1) -> Vec<String> {
    let mut fields = Vec::new();
    extend_unique(&mut fields, &preset.grouping_field_paths);
    extend_unique(&mut fields, &preset.sort_field_paths);
    for lane in &preset.lane_definitions {
        push_unique(&mut fields, lane.match_field_path.clone());
        extend_unique(&mut fields, &lane.source_field_refs);
    }
    if is_local_small_model_queue(preset.layout_kind) {
        extend_unique(&mut fields, &preset.exposed_blocker_fields);
    }
    fields
}

fn visible_field_paths(
    preset: &DevCommandCenterViewPresetV1,
    fallback_active: bool,
) -> Vec<String> {
    if fallback_active {
        return vec![preset.base_envelope_schema_id.clone()];
    }

    let mut fields = required_preset_fields(preset);
    push_unique(&mut fields, preset.base_envelope_schema_id.clone());
    fields
}

fn action_preview(binding: &ProjectionActionBindingV1) -> DccProjectionActionPreviewV1 {
    DccProjectionActionPreviewV1 {
        binding_id: binding.binding_id.clone(),
        gesture: binding.gesture,
        action_id: binding.action_id.clone(),
        target_record_selector: binding.target_record_selector.clone(),
        target_field_paths: binding.target_field_paths.clone(),
        approval_posture: binding.approval_posture,
        evidence_posture: binding.evidence_posture,
        preview_required: binding.preview_required,
        mutates_on_gesture: binding.mutates_on_gesture,
    }
}

fn base_envelope_lane(base_envelope_schema_id: &str) -> TaskBoardLaneDefinitionV1 {
    TaskBoardLaneDefinitionV1 {
        lane_id: "base-envelope-fallback".to_string(),
        label: "Base envelope fallback".to_string(),
        match_field_path: base_envelope_schema_id.to_string(),
        match_values: vec![base_envelope_schema_id.to_string()],
        sort_field_paths: vec![base_envelope_schema_id.to_string()],
        wip_limit: None,
        projection_only: true,
        source_field_refs: vec![base_envelope_schema_id.to_string()],
    }
}

fn is_local_small_model_queue(layout_kind: DccLayoutKind) -> bool {
    matches!(
        layout_kind,
        DccLayoutKind::InboxTriage | DccLayoutKind::ExecutionQueue
    )
}

fn extend_unique(target: &mut Vec<String>, source: &[String]) {
    for value in source {
        push_unique(target, value.clone());
    }
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

fn require_non_empty(
    errors: &mut Vec<DccLayoutProjectionRegistryValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(DccLayoutProjectionRegistryValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<DccLayoutProjectionRegistryValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(DccLayoutProjectionRegistryValidationError {
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
