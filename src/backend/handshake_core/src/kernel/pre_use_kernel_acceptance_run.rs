use serde::{Deserialize, Serialize};

use super::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::{
        AuthorityEffect, EventLedgerMapping, KernelActionDenialV1, KernelActionResultStatus,
        KernelActionResultV1, KernelReceiptMapping,
    },
    dcc_mvp_runtime_surface::{
        dcc_catalog_action_rows_from_catalog, select_dcc_work_item,
        validate_dcc_mvp_runtime_surface, ApprovalScope, DccApprovalPreviewV1,
        DccDirectEditDenialRowV1, DccEvidenceItemV1, DccEvidenceKind, DccFreshnessBadgeV1,
        DccMvpRuntimeSurfaceV1, DccPanelKind, DccPromotionPreviewRowV1,
        DccPromotionPreviewStaleRisk, DccProposalStateV1, DccProposalStatus, DccRuntimePanelV1,
        DccSelectedWorkProjectionV1, DccSessionRuntimeStateV1, DccStableElementIdV1,
        DccWorkItemV1, DccWorktreeStateV1, DccWriteBoxQueueRowV1,
    },
    direct_edit_guard::{
        guard_direct_edit_attempt, DirectEditAttemptV1, DirectEditDecisionStatus,
        DirectEditDecisionV1, DirectEditTargetClass,
    },
    model_manual::{kernel002_no_context_model_manual, ManualTopic},
    write_boxes::{
        validate_write_box_common, CRDTWorkspaceBox, PromotionBox, ProposalBox, WriteBoxCommon,
        WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef, WriteBoxPayloadRef,
        WriteBoxReplayMetadataV1, WriteBoxTargetRef, WriteBoxValidationState,
        WriteBoxValidationStatus,
    },
};

pub const PRE_USE_KERNEL_ACCEPTANCE_RUN_SCHEMA_ID: &str = "hsk.kernel.pre_use_acceptance_run@1";

const WP_ID: &str = "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1";
const MT_ID: &str = "MT-050";
const WORK_ID: &str = "work-kernel002-mt050-preuse";
const WORKTREE_ID: &str = "wtc-preuse-hardening-v1";
const SESSION_ID: &str = "session-kernel002-preuse-coder";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PreUseKernelAcceptanceStepKind {
    ManualOpened,
    CatalogActionSelected,
    CrdtDraftCreated,
    ProposalSubmitted,
    ValidationTriggered,
    PromotionOrDenialObserved,
    DccProjectionViewed,
    EvidenceInspected,
    DirectAuthorityEditBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreUseKernelAcceptanceOutcome {
    PromotionQueued,
    Promoted,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreUseKernelAcceptanceStepV1 {
    pub step_id: String,
    pub kind: PreUseKernelAcceptanceStepKind,
    pub action_id: Option<String>,
    pub write_box_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub authority_file_mutation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreUseKernelAcceptanceResearchBasisV1 {
    pub sources_checked: Vec<String>,
    pub patterns_found: Vec<String>,
    pub reuse_opportunities: Vec<String>,
    pub rejected_options: Vec<String>,
    pub selected_approach: String,
    pub risks: Vec<String>,
    pub mitigations: Vec<String>,
    pub validation_plan: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreUseKernelAcceptanceRunV1 {
    pub schema_id: String,
    pub run_id: String,
    pub wp_id: String,
    pub mt_id: String,
    pub manual_id: String,
    pub no_context_model_ready: bool,
    pub manual_topics_confirmed: Vec<String>,
    pub catalog_action_refs: Vec<String>,
    pub crdt_workspace_box: CRDTWorkspaceBox,
    pub proposal_box: ProposalBox,
    pub promotion_box: PromotionBox,
    pub action_results: Vec<KernelActionResultV1>,
    pub promotion_or_denial_observed: PreUseKernelAcceptanceOutcome,
    pub dcc_projection: DccSelectedWorkProjectionV1,
    pub direct_edit_decision: DirectEditDecisionV1,
    pub steps: Vec<PreUseKernelAcceptanceStepV1>,
    pub evidence_refs: Vec<String>,
    pub research_basis: PreUseKernelAcceptanceResearchBasisV1,
    pub no_direct_authority_file_edits: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreUseKernelAcceptanceValidationError {
    pub field: &'static str,
    pub message: &'static str,
}

pub fn build_kernel002_pre_use_acceptance_run() -> PreUseKernelAcceptanceRunV1 {
    let manual = kernel002_no_context_model_manual();
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("kernel002 action catalog validates");

    let crdt_workspace_box = CRDTWorkspaceBox {
        common: write_box_common(
            "writebox-crdt-preuse-mt050",
            WriteBoxKind::CrdtWorkspace,
            WriteBoxLifecycleState::Validated,
            AuthorityEffect::PrePromotionEvidenceOnly,
            &[
                "crdt-update://kernel002/preuse/title",
                "crdt-update://kernel002/preuse/body",
                "validation-report://kernel002/preuse-crdt",
            ],
            &["schema_validity", "state_vector_freshness"],
            &["dcc.crdt_workspace", "dcc.conflict_projection"],
        ),
        state_vector: "sv-kernel002-preuse-3".to_string(),
        update_refs: vec![
            "crdt-update://kernel002/preuse/title".to_string(),
            "crdt-update://kernel002/preuse/body".to_string(),
        ],
    };
    let proposal_box = ProposalBox {
        common: write_box_common(
            "writebox-proposal-preuse-mt050",
            WriteBoxKind::Proposal,
            WriteBoxLifecycleState::Validated,
            AuthorityEffect::PrePromotionEvidenceOnly,
            &[
                "proposal://kernel002/preuse-crdt-patch",
                "validation-report://kernel002/preuse-crdt",
            ],
            &["actor_capability", "target_authority_class"],
            &["dcc.proposal_queue"],
        ),
        proposal_ref: "proposal://kernel002/preuse-crdt-patch".to_string(),
    };
    let promotion_box = PromotionBox {
        common: write_box_common(
            "writebox-promotion-preuse-mt050",
            WriteBoxKind::Promotion,
            WriteBoxLifecycleState::Promoted,
            AuthorityEffect::EventLedgerAuthorityWrite,
            &[
                "proposal://kernel002/preuse-crdt-patch",
                "validation-report://kernel002/preuse-crdt",
                "receipt://kernel002/preuse-promotion-committed",
                "event-ledger://kernel002/preuse-promotion-committed",
            ],
            &["promotion_gate", "idempotency", "event_ledger_append"],
            &["dcc.promotion_committed", "dcc.event_ledger_preview"],
        ),
        promotion_target_ref: "authority://kernel002/preuse/document".to_string(),
        event_ledger_ref: Some("event-ledger://kernel002/preuse-promotion-committed".to_string()),
    };

    let action_results = vec![
        KernelActionResultV1 {
            schema_id: "hsk.kernel_action_result@1".to_string(),
            result_id: "result-preuse-crdt-writeboxes".to_string(),
            request_trace_id: "trace-preuse-crdt-propose".to_string(),
            status: KernelActionResultStatus::WriteBoxesCreated,
            write_box_ids: vec![
                crdt_workspace_box.common.write_box_id.clone(),
                proposal_box.common.write_box_id.clone(),
            ],
            receipt_mappings: vec![receipt("STATUS", "trace-preuse-crdt-propose")],
            event_mappings: Vec::new(),
            denial: None,
        },
        KernelActionResultV1 {
            schema_id: "hsk.kernel_action_result@1".to_string(),
            result_id: "result-preuse-promotion-committed".to_string(),
            request_trace_id: "trace-preuse-promote".to_string(),
            status: KernelActionResultStatus::Promoted,
            write_box_ids: vec![promotion_box.common.write_box_id.clone()],
            receipt_mappings: vec![receipt("STATUS", "trace-preuse-promote")],
            event_mappings: vec![event(
                "KernelWriteBoxPromotionCommittedV1",
                "hsk.event.kernel_write_box_promotion_committed@1",
                "kernel002-preuse-promote-v1",
            )],
            denial: None,
        },
    ];

    let dcc_surface = pre_use_dcc_surface();
    validate_dcc_mvp_runtime_surface(&dcc_surface).expect("pre-use DCC surface validates");
    let dcc_projection =
        select_dcc_work_item(&dcc_surface, WORK_ID).expect("pre-use DCC work item selects");
    let direct_edit_decision = guard_direct_edit_attempt(&direct_authority_attempt(), &catalog);

    let catalog_action_refs = vec![
        "kernel.action_catalog.view".to_string(),
        "kernel.crdt_workspace.propose_patch".to_string(),
        "kernel.write_box.promote".to_string(),
        "kernel.direct_edit.deny".to_string(),
        "kernel.dcc_mvp_runtime.project".to_string(),
    ];

    let evidence_refs = vec![
        "manual://kernel002-no-context-model-manual-v1".to_string(),
        "catalog://kernel002-action-catalog-v1/kernel.crdt_workspace.propose_patch".to_string(),
        "crdt-update://kernel002/preuse/title".to_string(),
        "crdt-update://kernel002/preuse/body".to_string(),
        "proposal://kernel002/preuse-crdt-patch".to_string(),
        "validation-report://kernel002/preuse-crdt".to_string(),
        "receipt://kernel002/preuse-promotion-committed".to_string(),
        "event-ledger://kernel002/preuse-promotion-committed".to_string(),
        "dcc://kernel002/preuse/selected-work".to_string(),
        "direct-edit-denial://kernel002/preuse-authority-attempt".to_string(),
    ];

    PreUseKernelAcceptanceRunV1 {
        schema_id: PRE_USE_KERNEL_ACCEPTANCE_RUN_SCHEMA_ID.to_string(),
        run_id: "kernel002-pre-use-acceptance-mt050".to_string(),
        wp_id: WP_ID.to_string(),
        mt_id: MT_ID.to_string(),
        manual_id: manual.manual_id.to_string(),
        no_context_model_ready: manual.no_prior_context_required,
        manual_topics_confirmed: required_manual_topics()
            .iter()
            .filter_map(|topic| manual.section(*topic))
            .map(|section| format!("{:?}", section.topic))
            .collect(),
        catalog_action_refs,
        crdt_workspace_box,
        proposal_box,
        promotion_box,
        action_results,
        promotion_or_denial_observed: PreUseKernelAcceptanceOutcome::Promoted,
        dcc_projection,
        direct_edit_decision,
        steps: pre_use_steps(),
        evidence_refs,
        research_basis: research_basis(),
        no_direct_authority_file_edits: true,
    }
}

pub fn validate_pre_use_kernel_acceptance_run(
    run: &PreUseKernelAcceptanceRunV1,
) -> Result<(), Vec<PreUseKernelAcceptanceValidationError>> {
    let mut errors = Vec::new();

    require_non_empty(&mut errors, "run_id", &run.run_id);
    require_non_empty(&mut errors, "wp_id", &run.wp_id);
    require_non_empty(&mut errors, "mt_id", &run.mt_id);
    require_vec(&mut errors, "catalog_action_refs", &run.catalog_action_refs);
    require_vec(&mut errors, "steps", &run.steps);
    require_vec(&mut errors, "evidence_refs", &run.evidence_refs);
    require_vec(
        &mut errors,
        "manual_topics_confirmed",
        &run.manual_topics_confirmed,
    );
    require_research_basis(&mut errors, &run.research_basis);

    if run.schema_id != PRE_USE_KERNEL_ACCEPTANCE_RUN_SCHEMA_ID {
        errors.push(error(
            "schema_id",
            "pre-use acceptance run schema id is required",
        ));
    }
    if run.manual_id != "kernel002-no-context-model-manual-v1" {
        errors.push(error(
            "manual_id",
            "Kernel002 no-context manual must be the operating manual",
        ));
    }
    if !run.no_context_model_ready {
        errors.push(error(
            "no_context_model_ready",
            "acceptance run must be usable by a no-context model",
        ));
    }
    if !run.no_direct_authority_file_edits {
        errors.push(error(
            "no_direct_authority_file_edits",
            "pre-use acceptance must not rely on direct authority-file edits",
        ));
    }
    if run.steps.iter().any(|step| step.authority_file_mutation) {
        errors.push(error(
            "steps.authority_file_mutation",
            "acceptance steps must not mutate authority files directly",
        ));
    }

    validate_catalog_refs(&mut errors, &run.catalog_action_refs);
    validate_manual_topics(&mut errors, &run.manual_topics_confirmed);
    validate_steps(&mut errors, &run.steps);
    validate_write_boxes(run, &mut errors);
    validate_action_results(run, &mut errors);
    validate_dcc_projection(run, &mut errors);
    validate_direct_edit_decision(run, &mut errors);
    validate_evidence_refs(run, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_catalog_refs(errors: &mut Vec<PreUseKernelAcceptanceValidationError>, refs: &[String]) {
    let catalog = kernel002_action_catalog();
    for action_id in required_catalog_actions() {
        if !refs.iter().any(|value| value == action_id) {
            errors.push(error(
                "catalog_action_refs",
                "required catalog action is missing from acceptance run",
            ));
        }
        if catalog.action(action_id).is_none() {
            errors.push(error(
                "catalog_action_refs",
                "required acceptance action is not registered in catalog",
            ));
        }
    }
}

fn validate_manual_topics(
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
    topics: &[String],
) {
    for topic in required_manual_topics() {
        let topic_name = format!("{topic:?}");
        if !topics.iter().any(|value| value == &topic_name) {
            errors.push(error(
                "manual_topics_confirmed",
                "required no-context manual topic is missing",
            ));
        }
    }
}

fn validate_steps(
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
    steps: &[PreUseKernelAcceptanceStepV1],
) {
    for kind in required_step_kinds() {
        if !steps.iter().any(|step| step.kind == kind) {
            errors.push(error("steps.kind", "required acceptance step is missing"));
        }
    }
    for step in steps {
        require_non_empty(errors, "steps.step_id", &step.step_id);
        require_vec(errors, "steps.evidence_refs", &step.evidence_refs);
    }
}

fn validate_write_boxes(
    run: &PreUseKernelAcceptanceRunV1,
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
) {
    for common in [
        &run.crdt_workspace_box.common,
        &run.proposal_box.common,
        &run.promotion_box.common,
    ] {
        if validate_write_box_common(common).is_err() {
            errors.push(error(
                "write_box.common",
                "acceptance write-box common record must validate",
            ));
        }
    }

    require_write_box(
        errors,
        &run.crdt_workspace_box.common,
        WriteBoxKind::CrdtWorkspace,
        AuthorityEffect::PrePromotionEvidenceOnly,
    );
    require_write_box(
        errors,
        &run.proposal_box.common,
        WriteBoxKind::Proposal,
        AuthorityEffect::PrePromotionEvidenceOnly,
    );
    require_write_box(
        errors,
        &run.promotion_box.common,
        WriteBoxKind::Promotion,
        AuthorityEffect::EventLedgerAuthorityWrite,
    );

    match run.promotion_or_denial_observed {
        PreUseKernelAcceptanceOutcome::Promoted => {
            if run.promotion_box.common.lifecycle_state != WriteBoxLifecycleState::Promoted {
                errors.push(error(
                    "promotion_box.lifecycle_state",
                    "promoted pre-use acceptance must commit the promotion write box",
                ));
            }
            if run.promotion_box.event_ledger_ref.is_none() {
                errors.push(error(
                    "promotion_box.event_ledger_ref",
                    "promoted pre-use acceptance must include an EventLedger authority append reference",
                ));
            }
        }
        PreUseKernelAcceptanceOutcome::PromotionQueued => {
            if run.promotion_box.common.lifecycle_state != WriteBoxLifecycleState::PromotionQueued {
                errors.push(error(
                    "promotion_box.lifecycle_state",
                    "queued pre-use acceptance must leave the promotion write box queued",
                ));
            }
            if run.promotion_box.event_ledger_ref.is_some() {
                errors.push(error(
                    "promotion_box.event_ledger_ref",
                    "queued pre-use acceptance must not include an EventLedger authority append",
                ));
            }
        }
        PreUseKernelAcceptanceOutcome::Denied => {
            if run.promotion_box.common.lifecycle_state != WriteBoxLifecycleState::Denied {
                errors.push(error(
                    "promotion_box.lifecycle_state",
                    "denied pre-use acceptance must deny the promotion write box",
                ));
            }
            if run.promotion_box.event_ledger_ref.is_none() {
                errors.push(error(
                    "promotion_box.event_ledger_ref",
                    "denied pre-use acceptance must include an EventLedger denial reference",
                ));
            }
        }
    }
    require_vec(
        errors,
        "crdt_workspace_box.update_refs",
        &run.crdt_workspace_box.update_refs,
    );
    require_non_empty(
        errors,
        "crdt_workspace_box.state_vector",
        &run.crdt_workspace_box.state_vector,
    );
    require_non_empty(
        errors,
        "proposal_box.proposal_ref",
        &run.proposal_box.proposal_ref,
    );
}

fn validate_action_results(
    run: &PreUseKernelAcceptanceRunV1,
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
) {
    if !run.action_results.iter().any(|result| {
        result.status == KernelActionResultStatus::WriteBoxesCreated
            && result
                .write_box_ids
                .contains(&run.crdt_workspace_box.common.write_box_id)
            && result
                .write_box_ids
                .contains(&run.proposal_box.common.write_box_id)
    }) {
        errors.push(error(
            "action_results",
            "CRDT draft and proposal write boxes must be created by an action result",
        ));
    }

    match run.promotion_or_denial_observed {
        PreUseKernelAcceptanceOutcome::PromotionQueued => {
            if !run.action_results.iter().any(|result| {
                result.status == KernelActionResultStatus::PromotionQueued
                    && result
                        .write_box_ids
                        .contains(&run.promotion_box.common.write_box_id)
                    && result.event_mappings.is_empty()
            }) {
                errors.push(error(
                    "promotion_or_denial_observed",
                    "promotion-queued outcome must be observed without authority event mappings",
                ));
            }
        }
        PreUseKernelAcceptanceOutcome::Promoted => {
            if !run.action_results.iter().any(|result| {
                result.status == KernelActionResultStatus::Promoted
                    && result
                        .write_box_ids
                        .contains(&run.promotion_box.common.write_box_id)
                    && has_valid_event_mapping(result)
            }) {
                errors.push(error(
                    "promotion_or_denial_observed",
                    "promoted outcome must include EventLedger-backed promotion evidence",
                ));
            }
        }
        PreUseKernelAcceptanceOutcome::Denied => {
            if !run.action_results.iter().any(|result| {
                result.status == KernelActionResultStatus::Denied
                    && result
                        .write_box_ids
                        .contains(&run.promotion_box.common.write_box_id)
                    && has_valid_event_mapping(result)
                    && result
                        .denial
                        .as_ref()
                        .is_some_and(|denial| has_valid_denial_mapping(denial))
            }) {
                errors.push(error(
                    "promotion_or_denial_observed",
                    "denial outcome must include EventLedger-backed promotion denial evidence",
                ));
            }
        }
    }
}

fn validate_dcc_projection(
    run: &PreUseKernelAcceptanceRunV1,
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
) {
    for action_id in [
        "kernel.crdt_workspace.propose_patch",
        "kernel.write_box.promote",
        "kernel.direct_edit.deny",
    ] {
        if !run
            .dcc_projection
            .work_item
            .allowed_action_ids
            .iter()
            .any(|value| value == action_id)
        {
            errors.push(error(
                "dcc_projection.allowed_action_ids",
                "DCC projection must expose required governed action",
            ));
        }
    }

    if !run.dcc_projection.proposals.iter().any(|proposal| {
        proposal.action_id == "kernel.crdt_workspace.propose_patch"
            && proposal.status == DccProposalStatus::Approved
    }) {
        errors.push(error(
            "dcc_projection.proposals",
            "DCC projection must expose the CRDT proposal state",
        ));
    }

    for evidence_kind in [
        DccEvidenceKind::DiffPatch,
        DccEvidenceKind::ValidationOutput,
        DccEvidenceKind::Receipt,
    ] {
        if !run
            .dcc_projection
            .evidence
            .iter()
            .any(|evidence| evidence.kind == evidence_kind)
        {
            errors.push(error(
                "dcc_projection.evidence",
                "DCC projection must expose inspectable evidence kinds",
            ));
        }
    }

    if run.dcc_projection.approval_previews.is_empty() {
        errors.push(error(
            "dcc_projection.approval_previews",
            "DCC projection must expose promotion or denial preview state",
        ));
    }
}

fn validate_direct_edit_decision(
    run: &PreUseKernelAcceptanceRunV1,
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
) {
    if run.direct_edit_decision.status != DirectEditDecisionStatus::Denied {
        errors.push(error(
            "direct_edit_decision.status",
            "raw authority-file edit must be denied",
        ));
    }
    let Some(denial) = &run.direct_edit_decision.denial else {
        errors.push(error(
            "direct_edit_decision.denial",
            "direct-edit denial evidence is required",
        ));
        return;
    };
    if denial.denial_code != "direct_authority_edit_denied" {
        errors.push(error(
            "direct_edit_decision.denial_code",
            "direct authority edit denial code is required",
        ));
    }
    if !denial
        .lawful_replacement_action_ids
        .iter()
        .any(|action_id| action_id == "kernel.crdt_workspace.propose_patch")
    {
        errors.push(error(
            "direct_edit_decision.lawful_replacement_action_ids",
            "direct-edit denial must route back to CRDT proposal action",
        ));
    }
    if !has_valid_denial_mapping(denial) {
        errors.push(error(
            "direct_edit_decision.denial.event_mappings",
            "direct-edit denial must be backed by a valid EventLedger mapping",
        ));
    }
}

fn validate_evidence_refs(
    run: &PreUseKernelAcceptanceRunV1,
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
) {
    for required_fragment in [
        "manual://",
        "catalog://",
        "crdt-update://",
        "proposal://",
        "validation-report://",
        "receipt://",
        "dcc://",
        "direct-edit-denial://",
    ] {
        if !run
            .evidence_refs
            .iter()
            .any(|evidence_ref| evidence_ref.contains(required_fragment))
        {
            errors.push(error(
                "evidence_refs",
                "acceptance run is missing required evidence reference class",
            ));
        }
    }
}

fn required_catalog_actions() -> [&'static str; 5] {
    [
        "kernel.action_catalog.view",
        "kernel.crdt_workspace.propose_patch",
        "kernel.write_box.promote",
        "kernel.direct_edit.deny",
        "kernel.dcc_mvp_runtime.project",
    ]
}

fn required_manual_topics() -> [ManualTopic; 8] {
    [
        ManualTopic::Purpose,
        ManualTopic::Startup,
        ManualTopic::ActionCatalog,
        ManualTopic::WriteBoxes,
        ManualTopic::DccPaths,
        ManualTopic::CrdtWorkflow,
        ManualTopic::SafetyConstraints,
        ManualTopic::ValidationEvidence,
    ]
}

fn required_step_kinds() -> [PreUseKernelAcceptanceStepKind; 9] {
    [
        PreUseKernelAcceptanceStepKind::ManualOpened,
        PreUseKernelAcceptanceStepKind::CatalogActionSelected,
        PreUseKernelAcceptanceStepKind::CrdtDraftCreated,
        PreUseKernelAcceptanceStepKind::ProposalSubmitted,
        PreUseKernelAcceptanceStepKind::ValidationTriggered,
        PreUseKernelAcceptanceStepKind::PromotionOrDenialObserved,
        PreUseKernelAcceptanceStepKind::DccProjectionViewed,
        PreUseKernelAcceptanceStepKind::EvidenceInspected,
        PreUseKernelAcceptanceStepKind::DirectAuthorityEditBlocked,
    ]
}

fn pre_use_steps() -> Vec<PreUseKernelAcceptanceStepV1> {
    vec![
        step(
            "step-manual-opened",
            PreUseKernelAcceptanceStepKind::ManualOpened,
            Some("kernel.action_catalog.view"),
            None,
            &["manual://kernel002-no-context-model-manual-v1"],
        ),
        step(
            "step-catalog-action-selected",
            PreUseKernelAcceptanceStepKind::CatalogActionSelected,
            Some("kernel.crdt_workspace.propose_patch"),
            None,
            &["catalog://kernel002-action-catalog-v1/kernel.crdt_workspace.propose_patch"],
        ),
        step(
            "step-crdt-draft-created",
            PreUseKernelAcceptanceStepKind::CrdtDraftCreated,
            Some("kernel.crdt_workspace.propose_patch"),
            Some("writebox-crdt-preuse-mt050"),
            &[
                "crdt-update://kernel002/preuse/title",
                "crdt-update://kernel002/preuse/body",
            ],
        ),
        step(
            "step-proposal-submitted",
            PreUseKernelAcceptanceStepKind::ProposalSubmitted,
            Some("kernel.crdt_workspace.propose_patch"),
            Some("writebox-proposal-preuse-mt050"),
            &["proposal://kernel002/preuse-crdt-patch"],
        ),
        step(
            "step-validation-triggered",
            PreUseKernelAcceptanceStepKind::ValidationTriggered,
            Some("kernel.write_box.promote"),
            Some("writebox-proposal-preuse-mt050"),
            &["validation-report://kernel002/preuse-crdt"],
        ),
        step(
            "step-promotion-observed",
            PreUseKernelAcceptanceStepKind::PromotionOrDenialObserved,
            Some("kernel.write_box.promote"),
            Some("writebox-promotion-preuse-mt050"),
            &[
                "receipt://kernel002/preuse-promotion-committed",
                "event-ledger://kernel002/preuse-promotion-committed",
            ],
        ),
        step(
            "step-dcc-projection-viewed",
            PreUseKernelAcceptanceStepKind::DccProjectionViewed,
            Some("kernel.dcc_mvp_runtime.project"),
            None,
            &["dcc://kernel002/preuse/selected-work"],
        ),
        step(
            "step-evidence-inspected",
            PreUseKernelAcceptanceStepKind::EvidenceInspected,
            None,
            None,
            &[
                "diff://kernel002/preuse/crdt",
                "validation-output://kernel002/preuse-crdt",
                "receipt://kernel002/preuse-promotion-committed",
                "event-ledger://kernel002/preuse-promotion-committed",
            ],
        ),
        step(
            "step-direct-authority-edit-blocked",
            PreUseKernelAcceptanceStepKind::DirectAuthorityEditBlocked,
            Some("kernel.direct_edit.deny"),
            None,
            &["direct-edit-denial://kernel002/preuse-authority-attempt"],
        ),
    ]
}

fn step(
    step_id: &str,
    kind: PreUseKernelAcceptanceStepKind,
    action_id: Option<&str>,
    write_box_id: Option<&str>,
    evidence_refs: &[&str],
) -> PreUseKernelAcceptanceStepV1 {
    PreUseKernelAcceptanceStepV1 {
        step_id: step_id.to_string(),
        kind,
        action_id: action_id.map(str::to_string),
        write_box_id: write_box_id.map(str::to_string),
        evidence_refs: evidence_refs
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        authority_file_mutation: false,
    }
}

pub fn build_pre_use_dcc_mvp_runtime_surface() -> DccMvpRuntimeSurfaceV1 {
    pre_use_dcc_surface()
}

fn pre_use_dcc_surface() -> DccMvpRuntimeSurfaceV1 {
    DccMvpRuntimeSurfaceV1 {
        schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1".to_string(),
        surface_id: "dcc-preuse-acceptance-mt050".to_string(),
        folded_stub_id: "WP-1-Dev-Command-Center-MVP-v1".to_string(),
        panels: required_panel_kinds()
            .into_iter()
            .map(|kind| DccRuntimePanelV1 {
                panel_id: format!("panel-{kind:?}").to_lowercase(),
                kind,
                projection_only: true,
                source_refs: vec![
                    "kernel.action_catalog".to_string(),
                    "kernel.write_box.queue".to_string(),
                    "kernel.flight_recorder".to_string(),
                ],
                visible_state_fields: vec![
                    "wp_id".to_string(),
                    "mt_id".to_string(),
                    "proposal_id".to_string(),
                    "validation_state".to_string(),
                    "evidence_id".to_string(),
                ],
            })
            .collect(),
        work_items: vec![DccWorkItemV1 {
            work_id: WORK_ID.to_string(),
            wp_id: WP_ID.to_string(),
            mt_id: Some(MT_ID.to_string()),
            status: "PRE_USE_ACCEPTANCE".to_string(),
            worktree_id: WORKTREE_ID.to_string(),
            session_ids: vec![SESSION_ID.to_string()],
            proposal_ids: vec![
                "proposal-preuse-crdt-patch".to_string(),
                "proposal-preuse-promote".to_string(),
            ],
            evidence_ids: vec![
                "evidence-preuse-diff".to_string(),
                "evidence-preuse-validation".to_string(),
                "evidence-preuse-receipt".to_string(),
                "evidence-preuse-flight-recorder".to_string(),
            ],
            allowed_action_ids: required_catalog_actions()
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        }],
        worktrees: vec![DccWorktreeStateV1 {
            worktree_id: WORKTREE_ID.to_string(),
            path_ref: "worktree://wtc-preuse-hardening-v1".to_string(),
            branch: "feat/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
            dirty: true,
            diff_ref: Some("evidence-preuse-diff".to_string()),
            linked_work_ids: vec![WORK_ID.to_string()],
        }],
        sessions: vec![DccSessionRuntimeStateV1 {
            session_id: SESSION_ID.to_string(),
            role: "CODER".to_string(),
            model_id: "gpt-5.5".to_string(),
            backend: "codex".to_string(),
            worktree_id: WORKTREE_ID.to_string(),
            wp_id: WP_ID.to_string(),
            mt_id: Some(MT_ID.to_string()),
            state: "ACTIVE".to_string(),
        }],
        proposals: vec![
            DccProposalStateV1 {
                proposal_id: "proposal-preuse-crdt-patch".to_string(),
                work_id: WORK_ID.to_string(),
                action_id: "kernel.crdt_workspace.propose_patch".to_string(),
                status: DccProposalStatus::Approved,
                evidence_ids: vec![
                    "evidence-preuse-diff".to_string(),
                    "evidence-preuse-validation".to_string(),
                ],
                approval_preview_id: Some("approval-preuse-crdt-patch".to_string()),
            },
            DccProposalStateV1 {
                proposal_id: "proposal-preuse-promote".to_string(),
                work_id: WORK_ID.to_string(),
                action_id: "kernel.write_box.promote".to_string(),
                status: DccProposalStatus::Approved,
                evidence_ids: vec![
                    "evidence-preuse-validation".to_string(),
                    "evidence-preuse-receipt".to_string(),
                ],
                approval_preview_id: Some("approval-preuse-promote".to_string()),
            },
        ],
        evidence: vec![
            evidence(
                "evidence-preuse-diff",
                DccEvidenceKind::DiffPatch,
                "diff://kernel002/preuse/crdt",
            ),
            evidence(
                "evidence-preuse-validation",
                DccEvidenceKind::ValidationOutput,
                "validation-output://kernel002/preuse-crdt",
            ),
            evidence(
                "evidence-preuse-receipt",
                DccEvidenceKind::Receipt,
                "receipt://kernel002/preuse-promotion-committed",
            ),
            evidence(
                "evidence-preuse-flight-recorder",
                DccEvidenceKind::FlightRecorderEvent,
                "fr://event/kernel002/preuse-dcc-projection",
            ),
        ],
        approval_previews: vec![
            approval_preview(
                "approval-preuse-crdt-patch",
                "kernel.crdt_workspace.propose_patch",
                "CRDT_PATCH_DENIED",
            ),
            approval_preview(
                "approval-preuse-promote",
                "kernel.write_box.promote",
                "PROMOTION_GATE_DENIED",
            ),
        ],
        write_box_queue_rows: vec![
            DccWriteBoxQueueRowV1 {
                row_id: "write-box-row-preuse-crdt".to_string(),
                write_box_id: "wb-preuse-crdt-workspace".to_string(),
                work_id: WORK_ID.to_string(),
                kind: WriteBoxKind::CrdtWorkspace,
                lifecycle_state: WriteBoxLifecycleState::Validated,
                actor_id: "actor-kernel-builder".to_string(),
                target_refs: vec!["authority://kernel/document/preuse".to_string()],
                validation_state: WriteBoxValidationState::Valid,
                denial_receipt_refs: Vec::new(),
                promotion_receipt_refs: vec!["receipt://kernel002/preuse-promotion-queued".to_string()],
                event_ledger_event_refs: Vec::new(),
                stale_state_vector: false,
                stable_element_id: "dcc.write_box_queue.row.wb-preuse-crdt-workspace".to_string(),
            },
            DccWriteBoxQueueRowV1 {
                row_id: "write-box-row-preuse-promotion".to_string(),
                write_box_id: "wb-preuse-promotion".to_string(),
                work_id: WORK_ID.to_string(),
                kind: WriteBoxKind::Promotion,
                lifecycle_state: WriteBoxLifecycleState::PromotionQueued,
                actor_id: "actor-kernel-builder".to_string(),
                target_refs: vec!["authority://kernel/document/preuse".to_string()],
                validation_state: WriteBoxValidationState::Valid,
                denial_receipt_refs: Vec::new(),
                promotion_receipt_refs: vec!["receipt://kernel002/preuse-promotion-requested".to_string()],
                event_ledger_event_refs: Vec::new(),
                stale_state_vector: false,
                stable_element_id: "dcc.write_box_queue.row.wb-preuse-promotion".to_string(),
            },
        ],
        direct_edit_denials: vec![DccDirectEditDenialRowV1 {
            row_id: "direct-edit-denial-row-preuse".to_string(),
            denial_id: "denial-preuse-authority-attempt".to_string(),
            work_id: WORK_ID.to_string(),
            actor_id: "actor-kernel-builder".to_string(),
            target_ref: ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json".to_string(),
            attempted_action: "raw_authority_file_write".to_string(),
            recovery_instruction: "Use a registered write-box action".to_string(),
            ui_response_ref: "dcc://direct-edit-denials/preuse-authority-attempt".to_string(),
            api_response_ref: "api://kernel/direct-edit-denials/preuse-authority-attempt".to_string(),
            stable_element_id: "dcc.direct_edit_denial.row.preuse-authority-attempt".to_string(),
        }],
        promotion_previews: vec![DccPromotionPreviewRowV1 {
            row_id: "promotion-preview-row-preuse".to_string(),
            preview_id: "promotion-preview-preuse".to_string(),
            work_id: WORK_ID.to_string(),
            write_box_id: "wb-preuse-promotion".to_string(),
            promotion_target_ref: "authority://kernel/document/preuse".to_string(),
            request_event_ref: Some("eventledger://stream-preuse/promotion-requested".to_string()),
            accepted_event_ref: None,
            rejected_event_ref: None,
            state_vector: "sv-preuse-validated".to_string(),
            validation_check_summaries: vec![
                "promotion_gate_input_alignment: PASS".to_string(),
                "crdt_state_vector_match: PASS".to_string(),
            ],
            idempotency_key: crate::kernel::crdt::promotion_bridge::promotion_idempotency_key(
                "bridge-preuse",
                "requested",
            ),
            expected_event_kinds: vec![
                "KernelCrdtPromotionRequestedV1".to_string(),
                "KernelCrdtPromotionAcceptedV1".to_string(),
            ],
            stale_risk: DccPromotionPreviewStaleRisk::None,
            freshness_badge_id: "freshness-preuse-crdt".to_string(),
            stable_element_id: "dcc.promotion_preview.row.wb-preuse-promotion".to_string(),
        }],
        freshness_badges: vec![DccFreshnessBadgeV1 {
            badge_id: "freshness-preuse-crdt".to_string(),
            source_projection_id: "dcc-preuse-crdt-projection".to_string(),
            source_ref: "eventledger://stream-preuse".to_string(),
            state_vector: "sv-preuse-validated".to_string(),
            updated_at_ref: "eventledger://stream-preuse/latest".to_string(),
            stale: false,
            stable_element_id: "dcc.freshness_badge.preuse-crdt".to_string(),
        }],
        stable_element_ids: vec![
            DccStableElementIdV1 {
                element_id: "dcc.write_box_queue.row.wb-preuse-crdt-workspace".to_string(),
                surface_id: "dcc-preuse-acceptance-mt050".to_string(),
                element_kind: "write_box_queue_row".to_string(),
                source_ref: "writebox://wb-preuse-crdt-workspace".to_string(),
            },
            DccStableElementIdV1 {
                element_id: "dcc.write_box_queue.row.wb-preuse-promotion".to_string(),
                surface_id: "dcc-preuse-acceptance-mt050".to_string(),
                element_kind: "write_box_queue_row".to_string(),
                source_ref: "writebox://wb-preuse-promotion".to_string(),
            },
            DccStableElementIdV1 {
                element_id: "dcc.direct_edit_denial.row.preuse-authority-attempt".to_string(),
                surface_id: "dcc-preuse-acceptance-mt050".to_string(),
                element_kind: "direct_edit_denial_row".to_string(),
                source_ref: "denial://denial-preuse-authority-attempt".to_string(),
            },
            DccStableElementIdV1 {
                element_id: "dcc.promotion_preview.row.wb-preuse-promotion".to_string(),
                surface_id: "dcc-preuse-acceptance-mt050".to_string(),
                element_kind: "promotion_preview_row".to_string(),
                source_ref: "writebox://wb-preuse-promotion".to_string(),
            },
            DccStableElementIdV1 {
                element_id: "dcc.freshness_badge.preuse-crdt".to_string(),
                surface_id: "dcc-preuse-acceptance-mt050".to_string(),
                element_kind: "freshness_badge".to_string(),
                source_ref: "eventledger://stream-preuse".to_string(),
            },
        ],
        catalog_action_refs: required_catalog_actions()
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        catalog_action_rows: dcc_catalog_action_rows_from_catalog(
            &kernel002_action_catalog(),
            &required_catalog_actions()
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        ),
        direct_authority_mutation_allowed: false,
        ungoverned_tool_execution_allowed: false,
        destructive_git_ops_require_same_turn_approval: true,
        flight_recorder_event_types: vec![
            "dcc.work.selected".to_string(),
            "dcc.evidence.viewed".to_string(),
            "dcc.approval.previewed".to_string(),
            "dcc.action.previewed".to_string(),
        ],
        product_authority_refs: vec![
            "kernel.action_catalog".to_string(),
            "kernel.write_box.queue".to_string(),
            "kernel.flight_recorder".to_string(),
            "kernel.locus_work_tracking".to_string(),
        ],
        folded_source_refs: vec![
            ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.contract.json".to_string(),
        ],
    }
}

fn required_panel_kinds() -> Vec<DccPanelKind> {
    vec![
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
    ]
}

fn evidence(evidence_id: &str, kind: DccEvidenceKind, evidence_ref: &str) -> DccEvidenceItemV1 {
    DccEvidenceItemV1 {
        evidence_id: evidence_id.to_string(),
        kind,
        evidence_ref: evidence_ref.to_string(),
        work_id: WORK_ID.to_string(),
    }
}

fn approval_preview(
    preview_id: &str,
    action_id: &str,
    denied_failure_code: &str,
) -> DccApprovalPreviewV1 {
    DccApprovalPreviewV1 {
        preview_id: preview_id.to_string(),
        action_id: action_id.to_string(),
        scope_options: vec![
            ApprovalScope::Once,
            ApprovalScope::Job,
            ApprovalScope::Workspace,
        ],
        requires_same_turn_approval: false,
        denied_failure_code: denied_failure_code.to_string(),
    }
}

fn direct_authority_attempt() -> DirectEditAttemptV1 {
    DirectEditAttemptV1 {
        attempt_id: "preuse-authority-attempt".to_string(),
        actor_id: "actor-kernel-builder".to_string(),
        actor_kind: "model".to_string(),
        role_id: "CODER".to_string(),
        target_path: ".GOV/task_packets/WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1/packet.json".to_string(),
        target_class: DirectEditTargetClass::AuthorityArtifact,
        operation: "raw_authority_file_write".to_string(),
        trace_id: "trace-preuse-direct-edit".to_string(),
    }
}

fn write_box_common(
    write_box_id: &str,
    kind: WriteBoxKind,
    lifecycle_state: WriteBoxLifecycleState,
    authority_effect: AuthorityEffect,
    evidence_refs: &[&str],
    check_ids: &[&str],
    projection_rules: &[&str],
) -> WriteBoxCommon {
    WriteBoxCommon {
        write_box_id: write_box_id.to_string(),
        kind,
        schema_version: "hsk.write_box.v1".to_string(),
        workspace_id: "workspace-kernel002-preuse".to_string(),
        owner: WriteBoxOwnerRef {
            actor_id: "actor-kernel-builder".to_string(),
            actor_kind: "model".to_string(),
            role_id: "CODER".to_string(),
        },
        crdt_site_id: "site-kernel002-preuse".to_string(),
        target_refs: vec![WriteBoxTargetRef {
            target_id: "document-kernel002-preuse".to_string(),
            target_kind: "crdt_document".to_string(),
            authority_class: "pre_promotion_workspace".to_string(),
        }],
        base_snapshot_refs: vec!["postgres://kernel_crdt_snapshots/preuse-base".to_string()],
        intent_summary: format!("Kernel002 pre-use acceptance envelope for {write_box_id}"),
        operation_payload_refs: vec![WriteBoxPayloadRef {
            payload_id: format!("payload-{write_box_id}"),
            payload_kind: "kernel002_preuse_evidence".to_string(),
            payload_ref: format!("postgres://kernel_crdt_updates/{write_box_id}/payload"),
            payload_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
        }],
        lifecycle_state,
        allowed_transitions: vec![
            WriteBoxLifecycleState::Open,
            WriteBoxLifecycleState::ReadyForValidation,
            WriteBoxLifecycleState::Validated,
            WriteBoxLifecycleState::PromotionQueued,
            WriteBoxLifecycleState::Promoted,
            WriteBoxLifecycleState::Denied,
        ],
        authority_effect,
        evidence_refs: evidence_refs
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        receipt_refs: vec![format!("receipt://write-box-created/{write_box_id}")],
        denial_receipt_refs: if lifecycle_state == WriteBoxLifecycleState::Denied {
            vec![format!("receipt://write-box-denied/{write_box_id}")]
        } else {
            Vec::new()
        },
        promotion_receipt_refs: if authority_effect == AuthorityEffect::EventLedgerAuthorityWrite {
            vec![format!("receipt://promotion-requested/{write_box_id}")]
        } else {
            Vec::new()
        },
        validation_status: WriteBoxValidationStatus {
            state: WriteBoxValidationState::Valid,
            check_ids: check_ids.iter().map(|value| (*value).to_string()).collect(),
        },
        projection_rules: projection_rules
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        replay_metadata: WriteBoxReplayMetadataV1 {
            replay_plan_ref:
                "crdt-replay-plan://workspace-kernel002-preuse/document-kernel002-preuse"
                    .to_string(),
            replay_order_key: format!(
                "workspace-kernel002-preuse/document-kernel002-preuse/{write_box_id}"
            ),
            idempotency_key: format!("write-box:{write_box_id}:preuse"),
            source_event_refs: vec![format!(
                "eventledger://event-ledger-stream-preuse/{write_box_id}"
            )],
        },
    }
}

fn receipt(kind: &str, correlation_id: &str) -> KernelReceiptMapping {
    KernelReceiptMapping {
        receipt_kind: kind.to_string(),
        receipt_schema_id: "hsk.wp_receipt@1".to_string(),
        correlation_id: correlation_id.to_string(),
    }
}

fn event(event_kind: &str, event_schema_id: &str, idempotency_key: &str) -> EventLedgerMapping {
    EventLedgerMapping {
        event_kind: event_kind.to_string(),
        event_schema_id: event_schema_id.to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn has_valid_event_mapping(result: &KernelActionResultV1) -> bool {
    !result.event_mappings.is_empty() && result.event_mappings.iter().all(valid_event_mapping)
}

fn has_valid_denial_mapping(denial: &KernelActionDenialV1) -> bool {
    !denial.event_mappings.is_empty() && denial.event_mappings.iter().all(valid_event_mapping)
}

fn valid_event_mapping(mapping: &EventLedgerMapping) -> bool {
    !mapping.event_kind.trim().is_empty()
        && mapping.event_schema_id.starts_with("hsk.event.")
        && !mapping.idempotency_key.trim().is_empty()
}

fn research_basis() -> PreUseKernelAcceptanceResearchBasisV1 {
    PreUseKernelAcceptanceResearchBasisV1 {
        sources_checked: vec![
            "https://github.com/yjs/yjs#document-updates".to_string(),
            "https://automerge.org/docs/reference/documents/".to_string(),
            "https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations".to_string(),
        ],
        patterns_found: vec![
            "CRDT updates and state vectors stay as replayable evidence before authority promotion."
                .to_string(),
            "Operator-facing projections should remain views over durable records.".to_string(),
            "Validation and provenance artifacts should be inspectable independently from mutation."
                .to_string(),
        ],
        reuse_opportunities: vec![
            "Reuse Kernel002 model manual sections as the no-context startup contract.".to_string(),
            "Reuse action catalog, write boxes, DCC projections, and direct-edit guard records."
                .to_string(),
        ],
        rejected_options: vec![
            "Do not mutate .GOV packet files as acceptance proof.".to_string(),
            "Do not represent DCC quick actions as direct filesystem writes.".to_string(),
        ],
        selected_approach:
            "Build a deterministic acceptance projection over existing product-owned kernel records."
                .to_string(),
        risks: vec![
            "A passing proof could hide direct authority mutation if the DCC and guard evidence are omitted."
                .to_string(),
            "Promotion proof could be accepted without an EventLedger-backed result mapping."
                .to_string(),
        ],
        mitigations: vec![
            "Validation requires direct-edit denial evidence and no authority-file mutation flags."
                .to_string(),
            "Validation requires promotion and denial outcomes to carry EventLedger mappings."
                .to_string(),
        ],
        validation_plan: vec![
            "Run focused product acceptance tests.".to_string(),
            "Run artifact harness acceptance tests.".to_string(),
            "Run full proof harness and formatter/diff checks.".to_string(),
        ],
    }
}

fn require_write_box(
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
    common: &WriteBoxCommon,
    kind: WriteBoxKind,
    authority_effect: AuthorityEffect,
) {
    if common.kind != kind {
        errors.push(error(
            "write_box.kind",
            "acceptance run write box has unexpected kind",
        ));
    }
    if common.authority_effect != authority_effect {
        errors.push(error(
            "write_box.authority_effect",
            "acceptance run write box has unexpected authority effect",
        ));
    }
}

fn require_research_basis(
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
    basis: &PreUseKernelAcceptanceResearchBasisV1,
) {
    require_vec(
        errors,
        "research_basis.sources_checked",
        &basis.sources_checked,
    );
    require_vec(
        errors,
        "research_basis.patterns_found",
        &basis.patterns_found,
    );
    require_vec(
        errors,
        "research_basis.reuse_opportunities",
        &basis.reuse_opportunities,
    );
    require_vec(
        errors,
        "research_basis.rejected_options",
        &basis.rejected_options,
    );
    require_non_empty(
        errors,
        "research_basis.selected_approach",
        &basis.selected_approach,
    );
    require_vec(errors, "research_basis.risks", &basis.risks);
    require_vec(errors, "research_basis.mitigations", &basis.mitigations);
    require_vec(
        errors,
        "research_basis.validation_plan",
        &basis.validation_plan,
    );
}

fn require_non_empty(
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(error(field, "value must not be empty"));
    }
}

fn require_vec<T>(
    errors: &mut Vec<PreUseKernelAcceptanceValidationError>,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(error(field, "at least one value is required"));
    }
}

fn error(field: &'static str, message: &'static str) -> PreUseKernelAcceptanceValidationError {
    PreUseKernelAcceptanceValidationError { field, message }
}

#[allow(dead_code)]
fn denied_action_result(
    denial: KernelActionDenialV1,
    event_mappings: Vec<EventLedgerMapping>,
) -> KernelActionResultV1 {
    KernelActionResultV1 {
        schema_id: "hsk.kernel_action_result@1".to_string(),
        result_id: "result-preuse-denied".to_string(),
        request_trace_id: denial.request_trace_id.clone(),
        status: KernelActionResultStatus::Denied,
        write_box_ids: Vec::new(),
        receipt_mappings: denial.receipt_mappings.clone(),
        event_mappings,
        denial: Some(denial),
    }
}
