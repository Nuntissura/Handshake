use handshake_core::kernel::{
    action_envelope::AuthorityEffect,
    dcc_mvp_runtime_surface::{DccEvidenceKind, DccProposalStatus},
    direct_edit_guard::DirectEditDecisionStatus,
    pre_use_kernel_acceptance_run::{
        build_kernel002_pre_use_acceptance_run, validate_pre_use_kernel_acceptance_run,
        PreUseKernelAcceptanceOutcome, PreUseKernelAcceptanceStepKind,
    },
    write_boxes::{WriteBoxKind, WriteBoxLifecycleState},
};

#[test]
fn pre_use_acceptance_run_covers_no_context_crdt_to_promotion_path() {
    let run = build_kernel002_pre_use_acceptance_run();

    validate_pre_use_kernel_acceptance_run(&run).expect("pre-use acceptance run validates");

    assert_eq!(run.schema_id, "hsk.kernel.pre_use_acceptance_run@1");
    assert_eq!(run.manual_id, "kernel002-no-context-model-manual-v1");
    assert!(run.no_context_model_ready);
    assert_eq!(
        run.promotion_or_denial_observed,
        PreUseKernelAcceptanceOutcome::PromotionQueued
    );
    assert_eq!(
        run.crdt_workspace_box.common.kind,
        WriteBoxKind::CrdtWorkspace
    );
    assert_eq!(run.proposal_box.common.kind, WriteBoxKind::Proposal);
    assert_eq!(run.promotion_box.common.kind, WriteBoxKind::Promotion);
    assert_eq!(
        run.crdt_workspace_box.common.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        run.proposal_box.common.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        run.promotion_box.common.authority_effect,
        AuthorityEffect::EventLedgerAuthorityWrite
    );
    assert_eq!(
        run.promotion_box.common.lifecycle_state,
        WriteBoxLifecycleState::PromotionQueued
    );
    assert!(run.promotion_box.event_ledger_ref.is_none());
    assert!(run
        .catalog_action_refs
        .contains(&"kernel.action_catalog.view".to_string()));
    assert!(run
        .catalog_action_refs
        .contains(&"kernel.crdt_workspace.propose_patch".to_string()));
    assert!(run
        .catalog_action_refs
        .contains(&"kernel.write_box.promote".to_string()));

    for step_kind in required_step_kinds() {
        assert!(
            run.steps.iter().any(|step| step.kind == step_kind),
            "missing acceptance step: {step_kind:?}"
        );
    }
}

#[test]
fn pre_use_acceptance_run_exposes_dcc_projection_and_inspectable_evidence() {
    let run = build_kernel002_pre_use_acceptance_run();

    validate_pre_use_kernel_acceptance_run(&run).expect("pre-use acceptance run validates");

    assert_eq!(
        run.dcc_projection.work_item.mt_id.as_deref(),
        Some("MT-050")
    );
    assert!(run
        .dcc_projection
        .work_item
        .allowed_action_ids
        .contains(&"kernel.crdt_workspace.propose_patch".to_string()));
    assert!(run
        .dcc_projection
        .work_item
        .allowed_action_ids
        .contains(&"kernel.write_box.promote".to_string()));
    assert!(run.dcc_projection.proposals.iter().any(|proposal| {
        proposal.action_id == "kernel.crdt_workspace.propose_patch"
            && proposal.status == DccProposalStatus::Approved
    }));
    assert!(run
        .dcc_projection
        .evidence
        .iter()
        .any(|evidence| evidence.kind == DccEvidenceKind::DiffPatch));
    assert!(run
        .dcc_projection
        .evidence
        .iter()
        .any(|evidence| evidence.kind == DccEvidenceKind::ValidationOutput));
    assert!(run
        .dcc_projection
        .evidence
        .iter()
        .any(|evidence| evidence.kind == DccEvidenceKind::Receipt));
    assert!(run
        .evidence_refs
        .iter()
        .any(|evidence_ref| evidence_ref.contains("validation-report")));
    assert!(run
        .evidence_refs
        .iter()
        .any(|evidence_ref| evidence_ref.contains("dcc://")));
}

#[test]
fn pre_use_acceptance_run_blocks_direct_authority_file_edits() {
    let run = build_kernel002_pre_use_acceptance_run();

    validate_pre_use_kernel_acceptance_run(&run).expect("pre-use acceptance run validates");

    assert!(run.no_direct_authority_file_edits);
    assert!(run.steps.iter().all(|step| !step.authority_file_mutation));
    assert_eq!(
        run.direct_edit_decision.status,
        DirectEditDecisionStatus::Denied
    );
    let denial = run
        .direct_edit_decision
        .denial
        .as_ref()
        .expect("direct authority edit should be denied");
    assert_eq!(denial.denial_code, "direct_authority_edit_denied");
    assert!(denial
        .lawful_replacement_action_ids
        .contains(&"kernel.crdt_workspace.propose_patch".to_string()));

    let mut unsafe_run = run.clone();
    unsafe_run.no_direct_authority_file_edits = false;
    let errors = validate_pre_use_kernel_acceptance_run(&unsafe_run)
        .expect_err("acceptance run must reject direct authority-file edit flags");
    assert!(errors
        .iter()
        .any(|error| error.field == "no_direct_authority_file_edits"));
}

fn required_step_kinds() -> Vec<PreUseKernelAcceptanceStepKind> {
    vec![
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
