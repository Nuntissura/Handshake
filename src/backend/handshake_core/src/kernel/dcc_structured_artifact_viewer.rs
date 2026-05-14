use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const FOLDED_DCC_STRUCTURED_ARTIFACT_VIEWER_STUB_ID: &str =
    "WP-1-Dev-Command-Center-Structured-Artifact-Viewer-v1";

const REQUIRED_ARTIFACT_KINDS: [DccStructuredArtifactKind; 4] = [
    DccStructuredArtifactKind::WorkPacket,
    DccStructuredArtifactKind::MicroTask,
    DccStructuredArtifactKind::TaskBoardEntry,
    DccStructuredArtifactKind::RoleMailboxThread,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccStructuredArtifactKind {
    WorkPacket,
    MicroTask,
    TaskBoardEntry,
    RoleMailboxThread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccFieldProvenance {
    Canonical,
    Mirror,
    Derived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccMirrorState {
    InSync,
    Stale,
    AdvisoryEditPending,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccRawDrilldownMode {
    AdvancedOnly,
    DefaultVisible,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccArtifactLayoutKind {
    Kanban,
    Queue,
    List,
    Roadmap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccRenderedSectionKind {
    CanonicalFields,
    Mirror,
    RawDrilldown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccCanonicalFieldV1 {
    pub field_id: String,
    pub label: String,
    pub value: String,
    pub provenance: DccFieldProvenance,
    pub display_order: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccMirrorViewV1 {
    pub mirror_ref: String,
    pub state: DccMirrorState,
    pub content_preview: String,
    pub reconciliation_action_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccStructuredArtifactRecordV1 {
    pub record_id: String,
    pub artifact_kind: DccStructuredArtifactKind,
    pub canonical_fields: Vec<DccCanonicalFieldV1>,
    pub mirror: Option<DccMirrorViewV1>,
    pub raw_structured_ref: String,
    pub raw_drilldown_mode: DccRawDrilldownMode,
    pub authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccStructuredArtifactViewerV1 {
    pub schema_id: String,
    pub viewer_id: String,
    pub folded_stub_id: String,
    pub records: Vec<DccStructuredArtifactRecordV1>,
    pub layouts: Vec<DccArtifactLayoutKind>,
    pub default_layout: DccArtifactLayoutKind,
    pub canonical_fields_render_first: bool,
    pub mirror_state_visible: bool,
    pub raw_drilldown_advanced_only: bool,
    pub direct_layout_mutation_allowed: bool,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccRenderedSectionV1 {
    pub section_kind: DccRenderedSectionKind,
    pub title: String,
    pub field_ids: Vec<String>,
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccRenderedStructuredArtifactViewV1 {
    pub schema_id: String,
    pub record_id: String,
    pub artifact_kind: DccStructuredArtifactKind,
    pub sections: Vec<DccRenderedSectionV1>,
    pub mirror_state: DccMirrorState,
    pub raw_drilldown_available: bool,
    pub raw_drilldown_visible_by_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccArtifactLayoutProjectionV1 {
    pub schema_id: String,
    pub layout_kind: DccArtifactLayoutKind,
    pub rows: Vec<DccArtifactLayoutRowV1>,
    pub mutates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccArtifactLayoutRowV1 {
    pub record_id: String,
    pub artifact_kind: DccStructuredArtifactKind,
    pub primary_status: String,
    pub mirror_state: DccMirrorState,
    pub canonical_field_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccStructuredArtifactViewerValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_dcc_structured_artifact_viewer(
    viewer: &DccStructuredArtifactViewerV1,
) -> Result<(), Vec<DccStructuredArtifactViewerValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &viewer.schema_id);
    require_non_empty(&mut errors, "viewer_id", &viewer.viewer_id);
    require_non_empty(&mut errors, "folded_stub_id", &viewer.folded_stub_id);
    require_vec(&mut errors, "records", &viewer.records);
    require_vec(&mut errors, "layouts", &viewer.layouts);
    require_vec(
        &mut errors,
        "product_authority_refs",
        &viewer.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &viewer.folded_source_refs,
    );

    if viewer.folded_stub_id != FOLDED_DCC_STRUCTURED_ARTIFACT_VIEWER_STUB_ID {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "folded_stub_id",
            message: "viewer must bind the folded DCC structured artifact viewer stub",
        });
    }

    if !contains_text(
        &viewer.folded_source_refs,
        FOLDED_DCC_STRUCTURED_ARTIFACT_VIEWER_STUB_ID,
    ) {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "folded_source_refs",
            message: "folded structured artifact viewer source must be preserved",
        });
    }

    if !viewer.canonical_fields_render_first {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "canonical_fields_render_first",
            message: "canonical structured fields must render before mirrors",
        });
    }

    if !viewer.mirror_state_visible {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "mirror_state_visible",
            message: "mirror state must be visible in the DCC viewer",
        });
    }

    if !viewer.raw_drilldown_advanced_only {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "raw_drilldown_advanced_only",
            message: "raw structured drilldown must be an advanced view",
        });
    }

    if viewer.direct_layout_mutation_allowed {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "direct_layout_mutation_allowed",
            message: "derived layouts must not mutate authority",
        });
    }

    if !viewer.layouts.contains(&viewer.default_layout) {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "default_layout",
            message: "default layout must be declared in layout set",
        });
    }

    let projection_layout_count = viewer
        .layouts
        .iter()
        .filter(|layout| {
            matches!(
                layout,
                DccArtifactLayoutKind::Kanban
                    | DccArtifactLayoutKind::Queue
                    | DccArtifactLayoutKind::List
                    | DccArtifactLayoutKind::Roadmap
            )
        })
        .count();
    if projection_layout_count < 2 {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "layouts",
            message: "at least two projection layouts are required",
        });
    }

    for required_ref in [
        "kernel.structured_artifact_records",
        "kernel.mirror_advisory",
        "kernel.dcc_mvp_runtime",
    ] {
        if !contains_exact(&viewer.product_authority_refs, required_ref) {
            errors.push(DccStructuredArtifactViewerValidationError {
                field: "product_authority_refs",
                message: "viewer must cite structured records, mirror advisory, and DCC runtime authority refs",
            });
        }
    }

    for required_kind in REQUIRED_ARTIFACT_KINDS {
        if !viewer
            .records
            .iter()
            .any(|record| record.artifact_kind == required_kind)
        {
            errors.push(DccStructuredArtifactViewerValidationError {
                field: "records.artifact_kind",
                message: "viewer must cover Work Packet, Micro-Task, Task Board, and Role Mailbox records",
            });
        }
    }

    let mut record_ids = HashSet::new();
    for record in &viewer.records {
        if !record_ids.insert(record.record_id.as_str()) {
            errors.push(DccStructuredArtifactViewerValidationError {
                field: "record_id",
                message: "record ids must be unique",
            });
        }
        validate_record(&mut errors, record);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn render_dcc_structured_artifact_view(
    viewer: &DccStructuredArtifactViewerV1,
    record_id: &str,
) -> Result<DccRenderedStructuredArtifactViewV1, Vec<DccStructuredArtifactViewerValidationError>> {
    validate_dcc_structured_artifact_viewer(viewer)?;

    let Some(record) = viewer
        .records
        .iter()
        .find(|record| record.record_id == record_id)
    else {
        return Err(vec![DccStructuredArtifactViewerValidationError {
            field: "record_id",
            message: "record does not exist",
        }]);
    };

    let mut canonical_fields = record.canonical_fields.clone();
    canonical_fields.sort_by_key(|field| field.display_order);

    let mut sections = vec![DccRenderedSectionV1 {
        section_kind: DccRenderedSectionKind::CanonicalFields,
        title: "Canonical structured fields".to_string(),
        field_ids: canonical_fields
            .iter()
            .map(|field| field.field_id.clone())
            .collect(),
        source_ref: record.authority_refs.first().cloned(),
    }];

    let mirror_state = record
        .mirror
        .as_ref()
        .map(|mirror| mirror.state)
        .unwrap_or(DccMirrorState::Missing);
    sections.push(DccRenderedSectionV1 {
        section_kind: DccRenderedSectionKind::Mirror,
        title: "Derived mirror".to_string(),
        field_ids: Vec::new(),
        source_ref: record
            .mirror
            .as_ref()
            .map(|mirror| mirror.mirror_ref.clone()),
    });
    sections.push(DccRenderedSectionV1 {
        section_kind: DccRenderedSectionKind::RawDrilldown,
        title: "Raw structured drilldown".to_string(),
        field_ids: Vec::new(),
        source_ref: Some(record.raw_structured_ref.clone()),
    });

    Ok(DccRenderedStructuredArtifactViewV1 {
        schema_id: "hsk.kernel.dcc_structured_artifact_rendered_view@1".to_string(),
        record_id: record.record_id.clone(),
        artifact_kind: record.artifact_kind,
        sections,
        mirror_state,
        raw_drilldown_available: record.raw_drilldown_mode != DccRawDrilldownMode::Disabled,
        raw_drilldown_visible_by_default: record.raw_drilldown_mode
            == DccRawDrilldownMode::DefaultVisible,
    })
}

pub fn derive_dcc_structured_artifact_layout(
    viewer: &DccStructuredArtifactViewerV1,
    layout_kind: DccArtifactLayoutKind,
) -> Result<DccArtifactLayoutProjectionV1, Vec<DccStructuredArtifactViewerValidationError>> {
    validate_dcc_structured_artifact_viewer(viewer)?;
    if !viewer.layouts.contains(&layout_kind) {
        return Err(vec![DccStructuredArtifactViewerValidationError {
            field: "layout_kind",
            message: "requested layout is not declared",
        }]);
    }

    let rows = viewer
        .records
        .iter()
        .map(|record| DccArtifactLayoutRowV1 {
            record_id: record.record_id.clone(),
            artifact_kind: record.artifact_kind,
            primary_status: primary_status(record),
            mirror_state: record
                .mirror
                .as_ref()
                .map(|mirror| mirror.state)
                .unwrap_or(DccMirrorState::Missing),
            canonical_field_count: record.canonical_fields.len(),
        })
        .collect();

    Ok(DccArtifactLayoutProjectionV1 {
        schema_id: "hsk.kernel.dcc_structured_artifact_layout_projection@1".to_string(),
        layout_kind,
        rows,
        mutates_authority: false,
    })
}

fn validate_record(
    errors: &mut Vec<DccStructuredArtifactViewerValidationError>,
    record: &DccStructuredArtifactRecordV1,
) {
    require_non_empty(errors, "record_id", &record.record_id);
    require_vec(errors, "canonical_fields", &record.canonical_fields);
    require_non_empty(errors, "raw_structured_ref", &record.raw_structured_ref);
    require_vec(errors, "authority_refs", &record.authority_refs);
    require_vec(errors, "folded_source_refs", &record.folded_source_refs);

    if !contains_text(
        &record.folded_source_refs,
        FOLDED_DCC_STRUCTURED_ARTIFACT_VIEWER_STUB_ID,
    ) {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "folded_source_refs",
            message: "record must cite the folded structured artifact viewer source",
        });
    }

    if record.raw_drilldown_mode == DccRawDrilldownMode::DefaultVisible {
        errors.push(DccStructuredArtifactViewerValidationError {
            field: "raw_drilldown_mode",
            message: "raw structured JSON must not be the default operator view",
        });
    }

    let mut field_ids = HashSet::new();
    let mut display_orders = HashSet::new();
    for field in &record.canonical_fields {
        require_non_empty(errors, "canonical_fields.field_id", &field.field_id);
        require_non_empty(errors, "canonical_fields.label", &field.label);
        require_non_empty(errors, "canonical_fields.value", &field.value);
        if field.provenance != DccFieldProvenance::Canonical {
            errors.push(DccStructuredArtifactViewerValidationError {
                field: "canonical_fields.provenance",
                message: "canonical field list may contain only canonical fields",
            });
        }
        if !field_ids.insert(field.field_id.as_str()) {
            errors.push(DccStructuredArtifactViewerValidationError {
                field: "canonical_fields.field_id",
                message: "canonical field ids must be unique per record",
            });
        }
        if !display_orders.insert(field.display_order) {
            errors.push(DccStructuredArtifactViewerValidationError {
                field: "canonical_fields.display_order",
                message: "canonical field display order must be unique per record",
            });
        }
    }

    if let Some(mirror) = &record.mirror {
        require_non_empty(errors, "mirror.mirror_ref", &mirror.mirror_ref);
        require_non_empty(errors, "mirror.content_preview", &mirror.content_preview);
        if mirror.state != DccMirrorState::InSync && mirror.reconciliation_action_id.is_none() {
            errors.push(DccStructuredArtifactViewerValidationError {
                field: "mirror.reconciliation_action_id",
                message: "non-synchronized mirror states require a reconciliation action",
            });
        }
    }
}

fn primary_status(record: &DccStructuredArtifactRecordV1) -> String {
    record
        .canonical_fields
        .iter()
        .find(|field| field.field_id == "status")
        .map(|field| field.value.clone())
        .unwrap_or_else(|| "unknown".to_string())
}

fn require_non_empty(
    errors: &mut Vec<DccStructuredArtifactViewerValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(DccStructuredArtifactViewerValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<DccStructuredArtifactViewerValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(DccStructuredArtifactViewerValidationError {
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
