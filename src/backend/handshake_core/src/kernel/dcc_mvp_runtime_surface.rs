use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_catalog::KernelActionCatalogV1;
use super::action_envelope::{ApprovalPosture, AuthorityEffect};

pub const FOLDED_DCC_MVP_STUB_ID: &str = "WP-1-Dev-Command-Center-MVP-v1";

const REQUIRED_PANEL_KINDS: [DccPanelKind; 8] = [
    DccPanelKind::WorkSelection,
    DccPanelKind::WorktreeState,
    DccPanelKind::SessionState,
    DccPanelKind::ActionCatalog,
    DccPanelKind::ProposalState,
    DccPanelKind::DiffEvidence,
    DccPanelKind::ApprovalPreview,
    DccPanelKind::Timeline,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccPanelKind {
    WorkSelection,
    WorktreeState,
    SessionState,
    ActionCatalog,
    ProposalState,
    DiffEvidence,
    ApprovalPreview,
    Timeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccEvidenceKind {
    DiffPatch,
    FlightRecorderEvent,
    Receipt,
    Screenshot,
    ValidationOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DccProposalStatus {
    Draft,
    AwaitingApproval,
    Approved,
    Denied,
    Promoted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovalScope {
    Once,
    Job,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccRuntimePanelV1 {
    pub panel_id: String,
    pub kind: DccPanelKind,
    pub projection_only: bool,
    pub source_refs: Vec<String>,
    pub visible_state_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccWorkItemV1 {
    pub work_id: String,
    pub wp_id: String,
    pub mt_id: Option<String>,
    pub status: String,
    pub worktree_id: String,
    pub session_ids: Vec<String>,
    pub proposal_ids: Vec<String>,
    pub evidence_ids: Vec<String>,
    pub allowed_action_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccWorktreeStateV1 {
    pub worktree_id: String,
    pub path_ref: String,
    pub branch: String,
    pub dirty: bool,
    pub diff_ref: Option<String>,
    pub linked_work_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccSessionRuntimeStateV1 {
    pub session_id: String,
    pub role: String,
    pub model_id: String,
    pub backend: String,
    pub worktree_id: String,
    pub wp_id: String,
    pub mt_id: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccProposalStateV1 {
    pub proposal_id: String,
    pub work_id: String,
    pub action_id: String,
    pub status: DccProposalStatus,
    pub evidence_ids: Vec<String>,
    pub approval_preview_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccEvidenceItemV1 {
    pub evidence_id: String,
    pub kind: DccEvidenceKind,
    pub evidence_ref: String,
    pub work_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccApprovalPreviewV1 {
    pub preview_id: String,
    pub action_id: String,
    pub scope_options: Vec<ApprovalScope>,
    pub requires_same_turn_approval: bool,
    pub denied_failure_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccMvpRuntimeSurfaceV1 {
    pub schema_id: String,
    pub surface_id: String,
    pub folded_stub_id: String,
    pub panels: Vec<DccRuntimePanelV1>,
    pub work_items: Vec<DccWorkItemV1>,
    pub worktrees: Vec<DccWorktreeStateV1>,
    pub sessions: Vec<DccSessionRuntimeStateV1>,
    pub proposals: Vec<DccProposalStateV1>,
    pub evidence: Vec<DccEvidenceItemV1>,
    pub approval_previews: Vec<DccApprovalPreviewV1>,
    pub catalog_action_refs: Vec<String>,
    pub direct_authority_mutation_allowed: bool,
    pub ungoverned_tool_execution_allowed: bool,
    pub destructive_git_ops_require_same_turn_approval: bool,
    pub flight_recorder_event_types: Vec<String>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccSelectedWorkProjectionV1 {
    pub schema_id: String,
    pub work_item: DccWorkItemV1,
    pub worktree: DccWorktreeStateV1,
    pub sessions: Vec<DccSessionRuntimeStateV1>,
    pub proposals: Vec<DccProposalStateV1>,
    pub evidence: Vec<DccEvidenceItemV1>,
    pub approval_previews: Vec<DccApprovalPreviewV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccGovernedActionPreviewV1 {
    pub schema_id: String,
    pub work_id: String,
    pub action_id: &'static str,
    pub request_allowed: bool,
    pub authority_effect: AuthorityEffect,
    pub approval_posture: ApprovalPosture,
    pub expected_write_box_kinds: Vec<String>,
    pub approval_preview_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccMvpRuntimeSurfaceValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn validate_dcc_mvp_runtime_surface(
    surface: &DccMvpRuntimeSurfaceV1,
) -> Result<(), Vec<DccMvpRuntimeSurfaceValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "schema_id", &surface.schema_id);
    require_non_empty(&mut errors, "surface_id", &surface.surface_id);
    require_non_empty(&mut errors, "folded_stub_id", &surface.folded_stub_id);
    require_vec(&mut errors, "panels", &surface.panels);
    require_vec(&mut errors, "work_items", &surface.work_items);
    require_vec(&mut errors, "worktrees", &surface.worktrees);
    require_vec(&mut errors, "sessions", &surface.sessions);
    require_vec(
        &mut errors,
        "catalog_action_refs",
        &surface.catalog_action_refs,
    );
    require_vec(
        &mut errors,
        "flight_recorder_event_types",
        &surface.flight_recorder_event_types,
    );
    require_vec(
        &mut errors,
        "product_authority_refs",
        &surface.product_authority_refs,
    );
    require_vec(
        &mut errors,
        "folded_source_refs",
        &surface.folded_source_refs,
    );

    if surface.folded_stub_id != FOLDED_DCC_MVP_STUB_ID {
        errors.push(DccMvpRuntimeSurfaceValidationError {
            field: "folded_stub_id",
            message: "DCC runtime surface must bind the folded DCC MVP stub",
        });
    }

    if !contains_text(&surface.folded_source_refs, FOLDED_DCC_MVP_STUB_ID) {
        errors.push(DccMvpRuntimeSurfaceValidationError {
            field: "folded_source_refs",
            message: "folded DCC MVP source must be preserved",
        });
    }

    if surface.direct_authority_mutation_allowed {
        errors.push(DccMvpRuntimeSurfaceValidationError {
            field: "direct_authority_mutation_allowed",
            message: "DCC must remain a projection and steering surface",
        });
    }

    if surface.ungoverned_tool_execution_allowed {
        errors.push(DccMvpRuntimeSurfaceValidationError {
            field: "ungoverned_tool_execution_allowed",
            message: "DCC actions must go through governed catalog actions",
        });
    }

    if !surface.destructive_git_ops_require_same_turn_approval {
        errors.push(DccMvpRuntimeSurfaceValidationError {
            field: "destructive_git_ops_require_same_turn_approval",
            message: "destructive git operations require same-turn approval",
        });
    }

    for required_ref in [
        "kernel.action_catalog",
        "kernel.write_box.queue",
        "kernel.flight_recorder",
        "kernel.locus_work_tracking",
    ] {
        if !contains_exact(&surface.product_authority_refs, required_ref) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "product_authority_refs",
                message: "DCC runtime surface must cite product authority refs",
            });
        }
    }

    validate_panels(&mut errors, &surface.panels);
    validate_state_refs(&mut errors, surface);

    for event_type in &surface.flight_recorder_event_types {
        if !is_known_dcc_event_type(event_type) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "flight_recorder_event_types",
                message: "unknown DCC Flight Recorder event type",
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn select_dcc_work_item(
    surface: &DccMvpRuntimeSurfaceV1,
    work_id: &str,
) -> Result<DccSelectedWorkProjectionV1, Vec<DccMvpRuntimeSurfaceValidationError>> {
    validate_dcc_mvp_runtime_surface(surface)?;

    let Some(work_item) = surface
        .work_items
        .iter()
        .find(|work_item| work_item.work_id == work_id)
        .cloned()
    else {
        return Err(vec![DccMvpRuntimeSurfaceValidationError {
            field: "work_id",
            message: "selected work item does not exist",
        }]);
    };

    let worktree = surface
        .worktrees
        .iter()
        .find(|worktree| worktree.worktree_id == work_item.worktree_id)
        .cloned()
        .expect("validated worktree reference must exist");

    let sessions = surface
        .sessions
        .iter()
        .filter(|session| work_item.session_ids.contains(&session.session_id))
        .cloned()
        .collect();

    let proposals: Vec<DccProposalStateV1> = surface
        .proposals
        .iter()
        .filter(|proposal| work_item.proposal_ids.contains(&proposal.proposal_id))
        .cloned()
        .collect();

    let mut evidence_ids = work_item.evidence_ids.clone();
    for proposal in &proposals {
        for evidence_id in &proposal.evidence_ids {
            if !evidence_ids.contains(evidence_id) {
                evidence_ids.push(evidence_id.clone());
            }
        }
    }

    let evidence = surface
        .evidence
        .iter()
        .filter(|evidence| evidence_ids.contains(&evidence.evidence_id))
        .cloned()
        .collect();

    let approval_ids: HashSet<&str> = proposals
        .iter()
        .filter_map(|proposal| proposal.approval_preview_id.as_deref())
        .collect();
    let approval_previews = surface
        .approval_previews
        .iter()
        .filter(|approval| approval_ids.contains(approval.preview_id.as_str()))
        .cloned()
        .collect();

    Ok(DccSelectedWorkProjectionV1 {
        schema_id: "hsk.kernel.dcc_selected_work_projection@1".to_string(),
        work_item,
        worktree,
        sessions,
        proposals,
        evidence,
        approval_previews,
    })
}

pub fn preview_dcc_governed_action(
    surface: &DccMvpRuntimeSurfaceV1,
    catalog: &KernelActionCatalogV1,
    action_id: &str,
    work_id: &str,
) -> Result<DccGovernedActionPreviewV1, Vec<DccMvpRuntimeSurfaceValidationError>> {
    let selection = select_dcc_work_item(surface, work_id)?;

    if !surface
        .catalog_action_refs
        .iter()
        .any(|catalog_action_id| catalog_action_id == action_id)
        || !selection
            .work_item
            .allowed_action_ids
            .iter()
            .any(|allowed_action_id| allowed_action_id == action_id)
    {
        return Err(vec![DccMvpRuntimeSurfaceValidationError {
            field: "action_id",
            message: "DCC action must be catalog-referenced and allowed for selected work",
        }]);
    }

    let Some(action) = catalog.action(action_id) else {
        return Err(vec![DccMvpRuntimeSurfaceValidationError {
            field: "action_id",
            message: "DCC action does not exist in the kernel action catalog",
        }]);
    };

    let approval_preview_id = selection
        .proposals
        .iter()
        .find(|proposal| proposal.action_id == action_id)
        .and_then(|proposal| proposal.approval_preview_id.clone())
        .or_else(|| {
            surface
                .approval_previews
                .iter()
                .find(|preview| preview.action_id == action_id)
                .map(|preview| preview.preview_id.clone())
        });

    Ok(DccGovernedActionPreviewV1 {
        schema_id: "hsk.kernel.dcc_governed_action_preview@1".to_string(),
        work_id: selection.work_item.work_id,
        action_id: action.action_id,
        request_allowed: true,
        authority_effect: action.authority_effect,
        approval_posture: action.approval_posture,
        expected_write_box_kinds: action
            .expected_write_boxes
            .iter()
            .map(|write_box| write_box.write_box_kind.clone())
            .collect(),
        approval_preview_id,
    })
}

fn validate_panels(
    errors: &mut Vec<DccMvpRuntimeSurfaceValidationError>,
    panels: &[DccRuntimePanelV1],
) {
    for required_kind in REQUIRED_PANEL_KINDS {
        if !panels.iter().any(|panel| panel.kind == required_kind) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "panels.kind",
                message: "DCC MVP panel is missing",
            });
        }
    }

    let mut panel_ids = HashSet::new();
    for panel in panels {
        if !panel_ids.insert(panel.panel_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "panel_id",
                message: "panel ids must be unique",
            });
        }
        require_non_empty(errors, "panel_id", &panel.panel_id);
        require_vec(errors, "source_refs", &panel.source_refs);
        require_vec(errors, "visible_state_fields", &panel.visible_state_fields);
        if !panel.projection_only {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "projection_only",
                message: "DCC panels must be read-only projections",
            });
        }
    }
}

fn validate_state_refs(
    errors: &mut Vec<DccMvpRuntimeSurfaceValidationError>,
    surface: &DccMvpRuntimeSurfaceV1,
) {
    let worktree_ids: HashSet<&str> = surface
        .worktrees
        .iter()
        .map(|worktree| worktree.worktree_id.as_str())
        .collect();
    let session_ids: HashSet<&str> = surface
        .sessions
        .iter()
        .map(|session| session.session_id.as_str())
        .collect();
    let work_ids: HashSet<&str> = surface
        .work_items
        .iter()
        .map(|work| work.work_id.as_str())
        .collect();
    let proposal_ids: HashSet<&str> = surface
        .proposals
        .iter()
        .map(|proposal| proposal.proposal_id.as_str())
        .collect();
    let evidence_ids: HashSet<&str> = surface
        .evidence
        .iter()
        .map(|evidence| evidence.evidence_id.as_str())
        .collect();
    let approval_ids: HashSet<&str> = surface
        .approval_previews
        .iter()
        .map(|approval| approval.preview_id.as_str())
        .collect();

    for work in &surface.work_items {
        require_non_empty(errors, "work_items.work_id", &work.work_id);
        require_non_empty(errors, "work_items.wp_id", &work.wp_id);
        require_non_empty(errors, "work_items.status", &work.status);
        require_vec(errors, "work_items.session_ids", &work.session_ids);
        require_vec(
            errors,
            "work_items.allowed_action_ids",
            &work.allowed_action_ids,
        );

        if !worktree_ids.contains(work.worktree_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "work_items.worktree_id",
                message: "work item references an unknown worktree",
            });
        }
        for session_id in &work.session_ids {
            if !session_ids.contains(session_id.as_str()) {
                errors.push(DccMvpRuntimeSurfaceValidationError {
                    field: "work_items.session_ids",
                    message: "work item references an unknown session",
                });
            }
        }
        for proposal_id in &work.proposal_ids {
            if !proposal_ids.contains(proposal_id.as_str()) {
                errors.push(DccMvpRuntimeSurfaceValidationError {
                    field: "work_items.proposal_ids",
                    message: "work item references an unknown proposal",
                });
            }
        }
        for evidence_id in &work.evidence_ids {
            if !evidence_ids.contains(evidence_id.as_str()) {
                errors.push(DccMvpRuntimeSurfaceValidationError {
                    field: "work_items.evidence_ids",
                    message: "work item references unknown evidence",
                });
            }
        }
        for action_id in &work.allowed_action_ids {
            if !contains_exact(&surface.catalog_action_refs, action_id) {
                errors.push(DccMvpRuntimeSurfaceValidationError {
                    field: "work_items.allowed_action_ids",
                    message: "allowed action must be catalog referenced",
                });
            }
        }
    }

    for worktree in &surface.worktrees {
        require_non_empty(errors, "worktrees.worktree_id", &worktree.worktree_id);
        require_non_empty(errors, "worktrees.path_ref", &worktree.path_ref);
        require_non_empty(errors, "worktrees.branch", &worktree.branch);
        require_vec(
            errors,
            "worktrees.linked_work_ids",
            &worktree.linked_work_ids,
        );
        for work_id in &worktree.linked_work_ids {
            if !work_ids.contains(work_id.as_str()) {
                errors.push(DccMvpRuntimeSurfaceValidationError {
                    field: "worktrees.linked_work_ids",
                    message: "worktree references unknown work",
                });
            }
        }
        if let Some(diff_ref) = &worktree.diff_ref {
            if !evidence_ids.contains(diff_ref.as_str()) {
                errors.push(DccMvpRuntimeSurfaceValidationError {
                    field: "worktrees.diff_ref",
                    message: "worktree diff ref must point to evidence",
                });
            }
        }
    }

    for session in &surface.sessions {
        require_non_empty(errors, "sessions.session_id", &session.session_id);
        require_non_empty(errors, "sessions.role", &session.role);
        require_non_empty(errors, "sessions.model_id", &session.model_id);
        require_non_empty(errors, "sessions.backend", &session.backend);
        require_non_empty(errors, "sessions.wp_id", &session.wp_id);
        require_non_empty(errors, "sessions.state", &session.state);
        if !worktree_ids.contains(session.worktree_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "sessions.worktree_id",
                message: "session references unknown worktree",
            });
        }
    }

    for proposal in &surface.proposals {
        require_non_empty(errors, "proposals.proposal_id", &proposal.proposal_id);
        require_non_empty(errors, "proposals.action_id", &proposal.action_id);
        require_vec(errors, "proposals.evidence_ids", &proposal.evidence_ids);
        if !work_ids.contains(proposal.work_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "proposals.work_id",
                message: "proposal references unknown work",
            });
        }
        if !contains_exact(&surface.catalog_action_refs, &proposal.action_id) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "proposals.action_id",
                message: "proposal action must be catalog referenced",
            });
        }
        for evidence_id in &proposal.evidence_ids {
            if !evidence_ids.contains(evidence_id.as_str()) {
                errors.push(DccMvpRuntimeSurfaceValidationError {
                    field: "proposals.evidence_ids",
                    message: "proposal references unknown evidence",
                });
            }
        }
        if proposal.status == DccProposalStatus::AwaitingApproval
            && proposal
                .approval_preview_id
                .as_deref()
                .is_none_or(|preview_id| !approval_ids.contains(preview_id))
        {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "proposals.approval_preview_id",
                message: "awaiting-approval proposal must link an approval preview",
            });
        }
    }

    for evidence in &surface.evidence {
        require_non_empty(errors, "evidence.evidence_id", &evidence.evidence_id);
        require_non_empty(errors, "evidence.evidence_ref", &evidence.evidence_ref);
        if !work_ids.contains(evidence.work_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "evidence.work_id",
                message: "evidence references unknown work",
            });
        }
    }

    for approval in &surface.approval_previews {
        require_non_empty(errors, "approval_previews.preview_id", &approval.preview_id);
        require_non_empty(errors, "approval_previews.action_id", &approval.action_id);
        require_vec(
            errors,
            "approval_previews.scope_options",
            &approval.scope_options,
        );
        require_non_empty(
            errors,
            "approval_previews.denied_failure_code",
            &approval.denied_failure_code,
        );
        if !contains_exact(&surface.catalog_action_refs, &approval.action_id) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "approval_previews.action_id",
                message: "approval preview action must be catalog referenced",
            });
        }
    }
}

fn is_known_dcc_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        "dcc.work.selected"
            | "dcc.worktree.opened"
            | "dcc.diff.viewed"
            | "dcc.evidence.viewed"
            | "dcc.approval.previewed"
            | "dcc.action.previewed"
            | "dcc.timeline.viewed"
            | "dcc.commit.requested"
    )
}

fn require_non_empty(
    errors: &mut Vec<DccMvpRuntimeSurfaceValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(DccMvpRuntimeSurfaceValidationError {
            field,
            message: "value must not be empty",
        });
    }
}

fn require_vec<T>(
    errors: &mut Vec<DccMvpRuntimeSurfaceValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(DccMvpRuntimeSurfaceValidationError {
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
