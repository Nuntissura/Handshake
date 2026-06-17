use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::action_catalog::{KernelActionCatalogV1, KernelCatalogActionV1};
use super::action_envelope::{ApprovalPosture, AuthorityEffect};
use super::crdt::promotion_bridge::{
    CrdtPromotionBridgeInputV1, CrdtPromotionBridgeLedgerResultV1, CrdtPromotionBridgeStatus,
    promotion_idempotency_key,
};
use super::crdt::validity_guard::{
    CrdtPromotionValidationDecision, CrdtPromotionValidationReportV1, CrdtStateValidationError,
};
use super::write_boxes::{WriteBoxKind, WriteBoxLifecycleState, WriteBoxValidationState};

pub const FOLDED_DCC_MVP_STUB_ID: &str = "WP-1-Dev-Command-Center-MVP-v1";

const REQUIRED_PANEL_KINDS: [DccPanelKind; 12] = [
    DccPanelKind::WorkSelection,
    DccPanelKind::WorktreeState,
    DccPanelKind::SessionState,
    DccPanelKind::ActionCatalog,
    DccPanelKind::WriteBoxQueue,
    DccPanelKind::DirectEditDenialView,
    DccPanelKind::PromotionPreview,
    DccPanelKind::FreshnessBadges,
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
    WriteBoxQueue,
    DirectEditDenialView,
    PromotionPreview,
    FreshnessBadges,
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
pub struct DccCatalogActionRowV1 {
    pub action_id: String,
    pub target_authority_class: String,
    pub input_schema_id: String,
    pub result_schema_id: String,
    pub role_eligibility: Vec<String>,
    pub capability_requirements: Vec<String>,
    pub approval_posture: String,
    pub preview_behavior_summary: String,
    pub preview_panel_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccWriteBoxQueueRowV1 {
    pub row_id: String,
    pub write_box_id: String,
    pub work_id: String,
    pub kind: WriteBoxKind,
    pub lifecycle_state: WriteBoxLifecycleState,
    pub actor_id: String,
    pub target_refs: Vec<String>,
    pub validation_state: WriteBoxValidationState,
    pub denial_receipt_refs: Vec<String>,
    pub promotion_receipt_refs: Vec<String>,
    pub event_ledger_event_refs: Vec<String>,
    pub stale_state_vector: bool,
    pub stable_element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccDirectEditDenialRowV1 {
    pub row_id: String,
    pub denial_id: String,
    pub work_id: String,
    pub actor_id: String,
    pub target_ref: String,
    pub attempted_action: String,
    pub recovery_instruction: String,
    pub ui_response_ref: String,
    pub api_response_ref: String,
    pub stable_element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DccPromotionPreviewStaleRisk {
    None,
    StaleStateVector,
    DuplicateIdempotency,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccPromotionPreviewRowV1 {
    pub row_id: String,
    pub preview_id: String,
    pub work_id: String,
    pub write_box_id: String,
    pub promotion_target_ref: String,
    pub request_event_ref: Option<String>,
    pub accepted_event_ref: Option<String>,
    pub rejected_event_ref: Option<String>,
    pub state_vector: String,
    pub validation_check_summaries: Vec<String>,
    pub idempotency_key: String,
    pub expected_event_kinds: Vec<String>,
    pub stale_risk: DccPromotionPreviewStaleRisk,
    pub freshness_badge_id: String,
    pub stable_element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccFreshnessBadgeV1 {
    pub badge_id: String,
    pub source_projection_id: String,
    pub source_ref: String,
    pub state_vector: String,
    pub updated_at_ref: String,
    pub stale: bool,
    pub stable_element_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccStableElementIdV1 {
    pub element_id: String,
    pub surface_id: String,
    pub element_kind: String,
    pub source_ref: String,
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
    pub write_box_queue_rows: Vec<DccWriteBoxQueueRowV1>,
    pub direct_edit_denials: Vec<DccDirectEditDenialRowV1>,
    pub promotion_previews: Vec<DccPromotionPreviewRowV1>,
    pub freshness_badges: Vec<DccFreshnessBadgeV1>,
    pub stable_element_ids: Vec<DccStableElementIdV1>,
    pub catalog_action_refs: Vec<String>,
    pub catalog_action_rows: Vec<DccCatalogActionRowV1>,
    pub direct_authority_mutation_allowed: bool,
    pub ungoverned_tool_execution_allowed: bool,
    pub destructive_git_ops_require_same_turn_approval: bool,
    pub flight_recorder_event_types: Vec<String>,
    pub product_authority_refs: Vec<String>,
    pub folded_source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccSelectedWorkProjectionV1 {
    pub schema_id: String,
    pub work_item: DccWorkItemV1,
    pub worktree: DccWorktreeStateV1,
    pub sessions: Vec<DccSessionRuntimeStateV1>,
    pub proposals: Vec<DccProposalStateV1>,
    pub evidence: Vec<DccEvidenceItemV1>,
    pub approval_previews: Vec<DccApprovalPreviewV1>,
    pub write_box_queue_rows: Vec<DccWriteBoxQueueRowV1>,
    pub direct_edit_denials: Vec<DccDirectEditDenialRowV1>,
    pub promotion_previews: Vec<DccPromotionPreviewRowV1>,
    pub freshness_badges: Vec<DccFreshnessBadgeV1>,
    pub stable_element_ids: Vec<DccStableElementIdV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccGovernedActionPreviewV1 {
    pub schema_id: String,
    pub work_id: String,
    pub action_id: String,
    pub request_allowed: bool,
    pub authority_effect: AuthorityEffect,
    pub approval_posture: ApprovalPosture,
    pub expected_write_box_kinds: Vec<String>,
    pub approval_preview_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        "catalog_action_rows",
        &surface.catalog_action_rows,
    );
    require_vec(
        &mut errors,
        "write_box_queue_rows",
        &surface.write_box_queue_rows,
    );
    require_vec(
        &mut errors,
        "direct_edit_denials",
        &surface.direct_edit_denials,
    );
    require_vec(
        &mut errors,
        "promotion_previews",
        &surface.promotion_previews,
    );
    require_vec(&mut errors, "freshness_badges", &surface.freshness_badges);
    require_vec(
        &mut errors,
        "stable_element_ids",
        &surface.stable_element_ids,
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
    let write_box_queue_rows: Vec<DccWriteBoxQueueRowV1> = surface
        .write_box_queue_rows
        .iter()
        .filter(|row| row.work_id == work_item.work_id)
        .cloned()
        .collect();
    let direct_edit_denials: Vec<DccDirectEditDenialRowV1> = surface
        .direct_edit_denials
        .iter()
        .filter(|row| row.work_id == work_item.work_id)
        .cloned()
        .collect();
    let promotion_previews: Vec<DccPromotionPreviewRowV1> = surface
        .promotion_previews
        .iter()
        .filter(|row| row.work_id == work_item.work_id)
        .cloned()
        .collect();
    let freshness_badge_ids: HashSet<&str> = promotion_previews
        .iter()
        .map(|preview| preview.freshness_badge_id.as_str())
        .collect();
    let freshness_badges: Vec<DccFreshnessBadgeV1> = surface
        .freshness_badges
        .iter()
        .filter(|badge| freshness_badge_ids.contains(badge.badge_id.as_str()))
        .cloned()
        .collect();
    let stable_ids_needed: HashSet<&str> = write_box_queue_rows
        .iter()
        .map(|row| row.stable_element_id.as_str())
        .chain(
            direct_edit_denials
                .iter()
                .map(|row| row.stable_element_id.as_str()),
        )
        .chain(
            promotion_previews
                .iter()
                .map(|row| row.stable_element_id.as_str()),
        )
        .chain(
            surface
                .freshness_badges
                .iter()
                .map(|badge| badge.stable_element_id.as_str()),
        )
        .collect();
    let stable_element_ids: Vec<DccStableElementIdV1> = surface
        .stable_element_ids
        .iter()
        .filter(|element| stable_ids_needed.contains(element.element_id.as_str()))
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
        write_box_queue_rows,
        direct_edit_denials,
        promotion_previews,
        freshness_badges,
        stable_element_ids,
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
        action_id: action.action_id.to_string(),
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

/// Extract the EventLedger event refs surfaced by a promoted write box. The
/// projection points at the promotion-bridge ledger result: each appended
/// kernel event is exposed by its event id so the DCC viewer can deep-link to
/// the durable EventLedger row.
pub fn write_box_event_ledger_refs_from_bridge(
    ledger_result: &CrdtPromotionBridgeLedgerResultV1,
) -> Vec<String> {
    ledger_result
        .appended_events
        .iter()
        .map(|event| {
            format!(
                "eventledger://{}/{}",
                event.event_type.as_str(),
                event.event_id
            )
        })
        .collect()
}

/// Compare a write box's base state vector against the latest CRDT update for
/// the same `(workspace_id, document_id, crdt_document_id)` tuple. The base
/// state vector is considered stale when the input differs from the latest
/// state-vector-after value recorded for the same document identity.
pub fn write_box_state_vector_is_stale(
    base_state_vector: &str,
    latest_state_vector_after: &str,
) -> bool {
    !base_state_vector.is_empty()
        && !latest_state_vector_after.is_empty()
        && base_state_vector != latest_state_vector_after
}

/// Promotion preview field projection derived from a bridge input + ambient
/// state. The promotion-bridge contract owns idempotency-key shape and the
/// requested/accepted/rejected event-kind sequence; this struct projects those
/// fields plus stale-risk evidence into a DCC-viewer-friendly shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccPromotionPreviewFieldsV1 {
    pub state_vector: String,
    pub validation_check_summaries: Vec<String>,
    pub idempotency_key: String,
    pub expected_event_kinds: Vec<String>,
    pub stale_risk: DccPromotionPreviewStaleRisk,
}

/// Project the new promotion-preview fields for a single bridge input.
///
/// - `state_vector` comes from `input.state.state_vector`.
/// - `validation_check_summaries` is derived from the validation report
///   (overall decision, document-schema/state-vector alignment, and one entry
///   per recorded validation error).
/// - `idempotency_key` is the promotion bridge's `requested`-suffixed key.
/// - `expected_event_kinds` enumerates the events the bridge will append for
///   the projected status path: `Accepted` -> Requested+Accepted, `Rejected`
///   -> Requested+Rejected.
/// - `stale_risk` combines a state-vector mismatch and an idempotency-key
///   duplicate against `existing_idempotency_keys` (e.g., the
///   `kernel_event_ledger.idempotency_key` set).
pub fn derive_promotion_preview_fields(
    input: &CrdtPromotionBridgeInputV1,
    projected_status: CrdtPromotionBridgeStatus,
    latest_state_vector_after: &str,
    existing_idempotency_keys: &[String],
) -> DccPromotionPreviewFieldsV1 {
    let idempotency_key = promotion_idempotency_key(&input.bridge_id, "requested");
    let expected_event_kinds = match projected_status {
        CrdtPromotionBridgeStatus::Accepted => vec![
            "KernelCrdtPromotionRequestedV1".to_string(),
            "KernelCrdtPromotionAcceptedV1".to_string(),
        ],
        CrdtPromotionBridgeStatus::Rejected => vec![
            "KernelCrdtPromotionRequestedV1".to_string(),
            "KernelCrdtPromotionRejectedV1".to_string(),
        ],
    };
    let stale_state_vector = !input.state.state_vector.is_empty()
        && !latest_state_vector_after.is_empty()
        && input.state.state_vector != latest_state_vector_after;
    let duplicate_idempotency = existing_idempotency_keys
        .iter()
        .any(|existing| existing == &idempotency_key);
    let stale_risk = match (stale_state_vector, duplicate_idempotency) {
        (false, false) => DccPromotionPreviewStaleRisk::None,
        (true, false) => DccPromotionPreviewStaleRisk::StaleStateVector,
        (false, true) => DccPromotionPreviewStaleRisk::DuplicateIdempotency,
        (true, true) => DccPromotionPreviewStaleRisk::Both,
    };

    DccPromotionPreviewFieldsV1 {
        state_vector: input.state.state_vector.clone(),
        validation_check_summaries: validation_check_summaries_from_report(
            &input.validation_report,
        ),
        idempotency_key,
        expected_event_kinds,
        stale_risk,
    }
}

fn validation_check_summaries_from_report(report: &CrdtPromotionValidationReportV1) -> Vec<String> {
    let mut summaries = Vec::new();
    summaries.push(format!(
        "promotion_decision: {}",
        match report.decision {
            CrdtPromotionValidationDecision::Allowed => "ALLOWED",
            CrdtPromotionValidationDecision::Denied => "DENIED",
        }
    ));
    summaries.push(format!("document_schema_id: {}", report.document_schema_id));
    summaries.push(format!(
        "state_vector: {} / latest_update_seq: {}",
        report.state_vector, report.latest_update_seq
    ));
    if report.validation_errors.is_empty() {
        summaries.push("validation_errors: none".to_string());
    } else {
        for CrdtStateValidationError {
            code,
            field,
            message,
        } in &report.validation_errors
        {
            summaries.push(format!("validation_error[{field}] {code:?}: {message}"));
        }
    }
    summaries
}

/// Build the DCC action catalog viewer rows from a kernel action catalog.
///
/// Returns one row per requested action id, preserving the input order. Unknown
/// action ids are skipped silently — the surface validator enforces that every
/// `catalog_action_refs` entry maps to a row, so a missing row will surface as a
/// validation error during `validate_dcc_mvp_runtime_surface`.
pub fn dcc_catalog_action_rows_from_catalog(
    catalog: &KernelActionCatalogV1,
    action_ids: &[String],
) -> Vec<DccCatalogActionRowV1> {
    action_ids
        .iter()
        .filter_map(|action_id| catalog.action(action_id))
        .map(dcc_catalog_action_row_from_action)
        .collect()
}

fn dcc_catalog_action_row_from_action(action: &KernelCatalogActionV1) -> DccCatalogActionRowV1 {
    DccCatalogActionRowV1 {
        action_id: action.action_id.to_string(),
        target_authority_class: format!("{:?}", action.authority_effect),
        input_schema_id: action.input_schema_id.clone(),
        result_schema_id: action.result_schema_id.clone(),
        role_eligibility: action.role_eligibility.clone(),
        capability_requirements: action
            .capability_requirements
            .iter()
            .map(|requirement| requirement.capability_id.clone())
            .collect(),
        approval_posture: format!("{:?}", action.approval_posture),
        preview_behavior_summary: action.dcc_preview.summary.clone(),
        preview_panel_id: action.dcc_preview.panel_id.clone(),
    }
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
    let stable_element_ids: HashSet<&str> = surface
        .stable_element_ids
        .iter()
        .map(|element| element.element_id.as_str())
        .collect();
    let freshness_badge_ids: HashSet<&str> = surface
        .freshness_badges
        .iter()
        .map(|badge| badge.badge_id.as_str())
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

    for row in &surface.write_box_queue_rows {
        require_non_empty(errors, "write_box_queue_rows.row_id", &row.row_id);
        require_non_empty(
            errors,
            "write_box_queue_rows.write_box_id",
            &row.write_box_id,
        );
        require_non_empty(errors, "write_box_queue_rows.work_id", &row.work_id);
        require_non_empty(errors, "write_box_queue_rows.actor_id", &row.actor_id);
        require_vec(errors, "write_box_queue_rows.target_refs", &row.target_refs);
        require_non_empty(
            errors,
            "write_box_queue_rows.stable_element_id",
            &row.stable_element_id,
        );
        if !work_ids.contains(row.work_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "write_box_queue_rows.work_id",
                message: "write-box queue row references unknown work",
            });
        }
        if !stable_element_ids.contains(row.stable_element_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "write_box_queue_rows.stable_element_id",
                message: "write-box queue row must have stable element id",
            });
        }
        if matches!(row.lifecycle_state, WriteBoxLifecycleState::Promoted)
            && row.event_ledger_event_refs.is_empty()
        {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "write_box_queue_rows.event_ledger_event_refs",
                message: "promoted write box must surface event ledger event refs",
            });
        }
    }

    for row in &surface.direct_edit_denials {
        require_non_empty(errors, "direct_edit_denials.row_id", &row.row_id);
        require_non_empty(errors, "direct_edit_denials.denial_id", &row.denial_id);
        require_non_empty(errors, "direct_edit_denials.work_id", &row.work_id);
        require_non_empty(errors, "direct_edit_denials.actor_id", &row.actor_id);
        require_non_empty(errors, "direct_edit_denials.target_ref", &row.target_ref);
        require_non_empty(
            errors,
            "direct_edit_denials.attempted_action",
            &row.attempted_action,
        );
        require_non_empty(
            errors,
            "direct_edit_denials.recovery_instruction",
            &row.recovery_instruction,
        );
        require_non_empty(
            errors,
            "direct_edit_denials.ui_response_ref",
            &row.ui_response_ref,
        );
        require_non_empty(
            errors,
            "direct_edit_denials.api_response_ref",
            &row.api_response_ref,
        );
        if !work_ids.contains(row.work_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "direct_edit_denials.work_id",
                message: "direct-edit denial row references unknown work",
            });
        }
        if !stable_element_ids.contains(row.stable_element_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "direct_edit_denials.stable_element_id",
                message: "direct-edit denial row must have stable element id",
            });
        }
    }

    for row in &surface.promotion_previews {
        require_non_empty(errors, "promotion_previews.row_id", &row.row_id);
        require_non_empty(errors, "promotion_previews.preview_id", &row.preview_id);
        require_non_empty(errors, "promotion_previews.work_id", &row.work_id);
        require_non_empty(errors, "promotion_previews.write_box_id", &row.write_box_id);
        require_non_empty(
            errors,
            "promotion_previews.promotion_target_ref",
            &row.promotion_target_ref,
        );
        require_non_empty(
            errors,
            "promotion_previews.freshness_badge_id",
            &row.freshness_badge_id,
        );
        if !work_ids.contains(row.work_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "promotion_previews.work_id",
                message: "promotion preview references unknown work",
            });
        }
        if !freshness_badge_ids.contains(row.freshness_badge_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "promotion_previews.freshness_badge_id",
                message: "promotion preview must link freshness badge",
            });
        }
        if !stable_element_ids.contains(row.stable_element_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "promotion_previews.stable_element_id",
                message: "promotion preview must have stable element id",
            });
        }
        require_non_empty(errors, "promotion_previews.state_vector", &row.state_vector);
        require_non_empty(
            errors,
            "promotion_previews.idempotency_key",
            &row.idempotency_key,
        );
        require_vec(
            errors,
            "promotion_previews.validation_check_summaries",
            &row.validation_check_summaries,
        );
        require_vec(
            errors,
            "promotion_previews.expected_event_kinds",
            &row.expected_event_kinds,
        );
    }

    for badge in &surface.freshness_badges {
        require_non_empty(errors, "freshness_badges.badge_id", &badge.badge_id);
        require_non_empty(
            errors,
            "freshness_badges.source_projection_id",
            &badge.source_projection_id,
        );
        require_non_empty(errors, "freshness_badges.source_ref", &badge.source_ref);
        require_non_empty(errors, "freshness_badges.state_vector", &badge.state_vector);
        require_non_empty(
            errors,
            "freshness_badges.updated_at_ref",
            &badge.updated_at_ref,
        );
        if !stable_element_ids.contains(badge.stable_element_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "freshness_badges.stable_element_id",
                message: "freshness badge must have stable element id",
            });
        }
    }

    let mut catalog_row_action_ids: HashSet<&str> = HashSet::new();
    for row in &surface.catalog_action_rows {
        require_non_empty(errors, "catalog_action_rows.action_id", &row.action_id);
        require_non_empty(
            errors,
            "catalog_action_rows.target_authority_class",
            &row.target_authority_class,
        );
        require_non_empty(
            errors,
            "catalog_action_rows.input_schema_id",
            &row.input_schema_id,
        );
        require_non_empty(
            errors,
            "catalog_action_rows.result_schema_id",
            &row.result_schema_id,
        );
        require_vec(
            errors,
            "catalog_action_rows.role_eligibility",
            &row.role_eligibility,
        );
        require_vec(
            errors,
            "catalog_action_rows.capability_requirements",
            &row.capability_requirements,
        );
        require_non_empty(
            errors,
            "catalog_action_rows.approval_posture",
            &row.approval_posture,
        );
        require_non_empty(
            errors,
            "catalog_action_rows.preview_behavior_summary",
            &row.preview_behavior_summary,
        );
        require_non_empty(
            errors,
            "catalog_action_rows.preview_panel_id",
            &row.preview_panel_id,
        );
        if !catalog_row_action_ids.insert(row.action_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "catalog_action_rows.action_id",
                message: "catalog action row ids must be unique",
            });
        }
        if !contains_exact(&surface.catalog_action_refs, &row.action_id) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "catalog_action_rows.action_id",
                message: "catalog action row must align with catalog_action_refs",
            });
        }
    }
    for action_id in &surface.catalog_action_refs {
        if !catalog_row_action_ids.contains(action_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "catalog_action_rows",
                message: "every catalog_action_refs entry must have a catalog_action_rows entry",
            });
        }
    }

    let mut element_id_uniques = HashSet::new();
    for element in &surface.stable_element_ids {
        require_non_empty(errors, "stable_element_ids.element_id", &element.element_id);
        require_non_empty(errors, "stable_element_ids.surface_id", &element.surface_id);
        require_non_empty(
            errors,
            "stable_element_ids.element_kind",
            &element.element_kind,
        );
        require_non_empty(errors, "stable_element_ids.source_ref", &element.source_ref);
        if !element_id_uniques.insert(element.element_id.as_str()) {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "stable_element_ids.element_id",
                message: "stable element ids must be unique",
            });
        }
        if element.surface_id != surface.surface_id {
            errors.push(DccMvpRuntimeSurfaceValidationError {
                field: "stable_element_ids.surface_id",
                message: "stable element id must belong to this surface",
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
