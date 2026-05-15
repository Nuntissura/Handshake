use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::{ApprovalPosture, AuthorityEffect},
    dcc_mvp_runtime_surface::{
        preview_dcc_governed_action, select_dcc_work_item, validate_dcc_mvp_runtime_surface,
        ApprovalScope, DccApprovalPreviewV1, DccEvidenceItemV1, DccEvidenceKind,
        DccMvpRuntimeSurfaceV1, DccPanelKind, DccProposalStateV1, DccProposalStatus,
        DccRuntimePanelV1, DccSessionRuntimeStateV1, DccWorkItemV1, DccWorktreeStateV1,
    },
};

#[test]
fn dcc_runtime_surface_selects_work_and_exposes_state_without_authority_bypass() {
    let surface = sample_surface();

    validate_dcc_mvp_runtime_surface(&surface).expect("DCC runtime surface validates");
    let selection = select_dcc_work_item(&surface, "work-kernel002-mt024")
        .expect("selected DCC work item projects");

    assert!(!surface.direct_authority_mutation_allowed);
    assert!(!surface.ungoverned_tool_execution_allowed);
    assert!(surface.destructive_git_ops_require_same_turn_approval);
    for panel_kind in required_panel_kinds() {
        assert!(
            surface.panels.iter().any(|panel| panel.kind == panel_kind),
            "missing panel kind: {panel_kind:?}"
        );
    }

    assert_eq!(selection.worktree.branch, "feat/kernel002");
    assert_eq!(selection.sessions[0].session_id, "session-coder-1");
    assert_eq!(selection.proposals[0].proposal_id, "proposal-crdt-patch-1");
    assert!(selection
        .evidence
        .iter()
        .any(|evidence| evidence.kind == DccEvidenceKind::DiffPatch));
    assert_eq!(
        selection.approval_previews[0].denied_failure_code,
        "DCC_APPROVAL_DENIED"
    );
}

#[test]
fn dcc_runtime_surface_previews_governed_actions_through_catalog() {
    let surface = sample_surface();
    let catalog = kernel002_action_catalog();
    let selection = select_dcc_work_item(&surface, "work-kernel002-mt024")
        .expect("selected DCC work item projects");

    let preview = preview_dcc_governed_action(
        &surface,
        &catalog,
        "kernel.crdt_workspace.propose_patch",
        "work-kernel002-mt024",
    )
    .expect("catalog-backed action preview should build");

    assert_eq!(preview.action_id, "kernel.crdt_workspace.propose_patch");
    assert_eq!(
        preview.authority_effect,
        AuthorityEffect::PrePromotionEvidenceOnly
    );
    assert_eq!(
        preview.approval_posture,
        ApprovalPosture::RequiresPromotionGate
    );
    assert!(preview
        .expected_write_box_kinds
        .contains(&"ProposalBox".to_string()));
    assert_eq!(
        preview.approval_preview_id.as_deref(),
        Some("approval-crdt-patch")
    );

    let json = serde_json::to_string(&selection).expect("DCC selection serializes");
    let decoded: handshake_core::kernel::dcc_mvp_runtime_surface::DccSelectedWorkProjectionV1 =
        serde_json::from_str(&json).expect("DCC selection deserializes");
    assert_eq!(decoded, selection);

    let json = serde_json::to_string(&preview).expect("DCC preview serializes");
    let decoded: handshake_core::kernel::dcc_mvp_runtime_surface::DccGovernedActionPreviewV1 =
        serde_json::from_str(&json).expect("DCC preview deserializes");
    assert_eq!(decoded, preview);
}

#[test]
fn dcc_runtime_surface_rejects_ungoverned_execution_and_uncataloged_actions() {
    let mut surface = sample_surface();
    surface.ungoverned_tool_execution_allowed = true;

    let errors = validate_dcc_mvp_runtime_surface(&surface)
        .expect_err("DCC must not allow ungoverned tool execution");
    assert!(errors
        .iter()
        .any(|error| error.field == "ungoverned_tool_execution_allowed"));

    let surface = sample_surface();
    let catalog = kernel002_action_catalog();
    let preview_error =
        preview_dcc_governed_action(&surface, &catalog, "kernel.unknown", "work-kernel002-mt024")
            .expect_err("unknown action must not preview");
    assert_eq!(preview_error[0].field, "action_id");
}

#[test]
fn kernel_action_catalog_exposes_dcc_runtime_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.dcc_mvp_runtime.project")
        .expect("DCC runtime projection action must be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "dcc_catalog_backed_actions"));
}

fn sample_surface() -> DccMvpRuntimeSurfaceV1 {
    DccMvpRuntimeSurfaceV1 {
        schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1".to_string(),
        surface_id: "kernel002-dcc-mvp-runtime-mt024".to_string(),
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
                    "worktree_id".to_string(),
                    "session_id".to_string(),
                    "proposal_id".to_string(),
                    "evidence_id".to_string(),
                ],
            })
            .collect(),
        work_items: vec![DccWorkItemV1 {
            work_id: "work-kernel002-mt024".to_string(),
            wp_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
            mt_id: Some("MT-024".to_string()),
            status: "ACTIVE".to_string(),
            worktree_id: "wt-kernel002".to_string(),
            session_ids: vec!["session-coder-1".to_string()],
            proposal_ids: vec!["proposal-crdt-patch-1".to_string()],
            evidence_ids: vec!["evidence-diff-1".to_string(), "evidence-fr-1".to_string()],
            allowed_action_ids: vec!["kernel.crdt_workspace.propose_patch".to_string()],
        }],
        worktrees: vec![DccWorktreeStateV1 {
            worktree_id: "wt-kernel002".to_string(),
            path_ref: "worktree://wtc-preuse-hardening-v1".to_string(),
            branch: "feat/kernel002".to_string(),
            dirty: true,
            diff_ref: Some("evidence-diff-1".to_string()),
            linked_work_ids: vec!["work-kernel002-mt024".to_string()],
        }],
        sessions: vec![DccSessionRuntimeStateV1 {
            session_id: "session-coder-1".to_string(),
            role: "CODER".to_string(),
            model_id: "gpt-5.5".to_string(),
            backend: "codex".to_string(),
            worktree_id: "wt-kernel002".to_string(),
            wp_id: "WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1".to_string(),
            mt_id: Some("MT-024".to_string()),
            state: "ACTIVE".to_string(),
        }],
        proposals: vec![DccProposalStateV1 {
            proposal_id: "proposal-crdt-patch-1".to_string(),
            work_id: "work-kernel002-mt024".to_string(),
            action_id: "kernel.crdt_workspace.propose_patch".to_string(),
            status: DccProposalStatus::AwaitingApproval,
            evidence_ids: vec!["evidence-diff-1".to_string()],
            approval_preview_id: Some("approval-crdt-patch".to_string()),
        }],
        evidence: vec![
            DccEvidenceItemV1 {
                evidence_id: "evidence-diff-1".to_string(),
                kind: DccEvidenceKind::DiffPatch,
                evidence_ref: "diff://wt-kernel002/current".to_string(),
                work_id: "work-kernel002-mt024".to_string(),
            },
            DccEvidenceItemV1 {
                evidence_id: "evidence-fr-1".to_string(),
                kind: DccEvidenceKind::FlightRecorderEvent,
                evidence_ref: "fr://event/dcc-preview".to_string(),
                work_id: "work-kernel002-mt024".to_string(),
            },
        ],
        approval_previews: vec![DccApprovalPreviewV1 {
            preview_id: "approval-crdt-patch".to_string(),
            action_id: "kernel.crdt_workspace.propose_patch".to_string(),
            scope_options: vec![
                ApprovalScope::Once,
                ApprovalScope::Job,
                ApprovalScope::Workspace,
            ],
            requires_same_turn_approval: false,
            denied_failure_code: "DCC_APPROVAL_DENIED".to_string(),
        }],
        catalog_action_refs: vec![
            "kernel.crdt_workspace.propose_patch".to_string(),
            "kernel.write_box.promote".to_string(),
            "kernel.direct_edit.deny".to_string(),
        ],
        direct_authority_mutation_allowed: false,
        ungoverned_tool_execution_allowed: false,
        destructive_git_ops_require_same_turn_approval: true,
        flight_recorder_event_types: vec![
            "dcc.work.selected".to_string(),
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
            ".GOV/task_packets/stubs/WP-1-Dev-Command-Center-MVP-v1.md".to_string(),
        ],
    }
}

fn required_panel_kinds() -> Vec<DccPanelKind> {
    vec![
        DccPanelKind::WorkSelection,
        DccPanelKind::WorktreeState,
        DccPanelKind::SessionState,
        DccPanelKind::ActionCatalog,
        DccPanelKind::ProposalState,
        DccPanelKind::DiffEvidence,
        DccPanelKind::ApprovalPreview,
        DccPanelKind::Timeline,
    ]
}
