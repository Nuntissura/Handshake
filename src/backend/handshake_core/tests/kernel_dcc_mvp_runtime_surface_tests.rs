use handshake_core::kernel::write_boxes::{
    WriteBoxKind, WriteBoxLifecycleState, WriteBoxValidationState,
};
use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::{ApprovalPosture, AuthorityEffect},
    crdt::{
        context_slice::CrdtMaterializedFieldV1,
        identity::{CrdtAuthorityLinksV1, CrdtWorkspaceIdentityV1},
        promotion_bridge::{
            promotion_idempotency_key, CrdtPromotionBridgeInputV1,
            CrdtPromotionBridgeLedgerResultV1, CrdtPromotionBridgeResultV1,
            CrdtPromotionBridgeStatus,
        },
        validity_guard::{
            CrdtMaterializedStateV1, CrdtPromotionValidationDecision,
            CrdtPromotionValidationReportV1,
        },
    },
    dcc_mvp_runtime_surface::{
        dcc_catalog_action_rows_from_catalog, derive_promotion_preview_fields,
        preview_dcc_governed_action, select_dcc_work_item, validate_dcc_mvp_runtime_surface,
        write_box_event_ledger_refs_from_bridge, write_box_state_vector_is_stale, ApprovalScope,
        DccApprovalPreviewV1, DccDirectEditDenialRowV1, DccEvidenceItemV1, DccEvidenceKind,
        DccFreshnessBadgeV1, DccMvpRuntimeSurfaceV1, DccPanelKind, DccPromotionPreviewRowV1,
        DccPromotionPreviewStaleRisk, DccProposalStateV1, DccProposalStatus, DccRuntimePanelV1,
        DccSessionRuntimeStateV1, DccStableElementIdV1, DccWorkItemV1, DccWorktreeStateV1,
        DccWriteBoxQueueRowV1,
    },
    KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
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
    assert_eq!(
        selection.write_box_queue_rows[0].write_box_id,
        "wb-crdt-patch-1"
    );
    assert_eq!(
        selection.direct_edit_denials[0].attempted_action,
        "raw_authority_patch"
    );
    assert_eq!(
        selection.promotion_previews[0].promotion_target_ref,
        "authority://kernel/document/document-kernel"
    );
    assert_eq!(selection.freshness_badges[0].state_vector, "sv-3");
    assert!(selection
        .stable_element_ids
        .iter()
        .any(|element| element.element_id == "dcc.write_box_queue.row.wb-crdt-patch-1"));
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
        write_box_queue_rows: vec![DccWriteBoxQueueRowV1 {
            row_id: "write-box-row-crdt-patch-1".to_string(),
            write_box_id: "wb-crdt-patch-1".to_string(),
            work_id: "work-kernel002-mt024".to_string(),
            kind: WriteBoxKind::CrdtWorkspace,
            lifecycle_state: WriteBoxLifecycleState::ReadyForValidation,
            actor_id: "actor-kernel-builder".to_string(),
            target_refs: vec!["authority://kernel/document/document-kernel".to_string()],
            validation_state: WriteBoxValidationState::Pending,
            denial_receipt_refs: Vec::new(),
            promotion_receipt_refs: vec![
                "receipt://promotion/requested/wb-crdt-patch-1".to_string()
            ],
            event_ledger_event_refs: Vec::new(),
            stale_state_vector: false,
            stable_element_id: "dcc.write_box_queue.row.wb-crdt-patch-1".to_string(),
        }],
        direct_edit_denials: vec![DccDirectEditDenialRowV1 {
            row_id: "direct-edit-denial-row-attempt-1".to_string(),
            denial_id: "denial-attempt-1".to_string(),
            work_id: "work-kernel002-mt024".to_string(),
            actor_id: "actor-kernel-builder".to_string(),
            target_ref: ".GOV/task_packets/WP-KERNEL-002/packet.json".to_string(),
            attempted_action: "raw_authority_patch".to_string(),
            recovery_instruction: "Use a registered write-box action".to_string(),
            ui_response_ref: "dcc://direct-edit-denials/attempt-1".to_string(),
            api_response_ref: "api://kernel/direct-edit-denials/attempt-1".to_string(),
            stable_element_id: "dcc.direct_edit_denial.row.denial-attempt-1".to_string(),
        }],
        promotion_previews: vec![DccPromotionPreviewRowV1 {
            row_id: "promotion-preview-row-wb-crdt-patch-1".to_string(),
            preview_id: "promotion-preview-crdt-patch-1".to_string(),
            work_id: "work-kernel002-mt024".to_string(),
            write_box_id: "wb-crdt-patch-1".to_string(),
            promotion_target_ref: "authority://kernel/document/document-kernel".to_string(),
            request_event_ref: Some("eventledger://event-ledger-stream-crdt/requested".to_string()),
            accepted_event_ref: None,
            rejected_event_ref: None,
            state_vector: "sv-3".to_string(),
            validation_check_summaries: vec![
                "promotion_gate_input_alignment: PASS".to_string(),
                "crdt_state_vector_match: PASS".to_string(),
            ],
            idempotency_key: promotion_idempotency_key("bridge-crdt-patch-1", "requested"),
            expected_event_kinds: vec![
                "KernelCrdtPromotionRequestedV1".to_string(),
                "KernelCrdtPromotionAcceptedV1".to_string(),
            ],
            stale_risk: DccPromotionPreviewStaleRisk::None,
            freshness_badge_id: "freshness-crdt-patch-1".to_string(),
            stable_element_id: "dcc.promotion_preview.row.wb-crdt-patch-1".to_string(),
        }],
        freshness_badges: vec![DccFreshnessBadgeV1 {
            badge_id: "freshness-crdt-patch-1".to_string(),
            source_projection_id: "dcc-crdt-projection".to_string(),
            source_ref: "eventledger://event-ledger-stream-crdt".to_string(),
            state_vector: "sv-3".to_string(),
            updated_at_ref: "eventledger://event-ledger-stream-crdt/update-3".to_string(),
            stale: false,
            stable_element_id: "dcc.freshness_badge.crdt-patch-1".to_string(),
        }],
        stable_element_ids: vec![
            DccStableElementIdV1 {
                element_id: "dcc.write_box_queue.row.wb-crdt-patch-1".to_string(),
                surface_id: "kernel002-dcc-mvp-runtime-mt024".to_string(),
                element_kind: "write_box_queue_row".to_string(),
                source_ref: "writebox://wb-crdt-patch-1".to_string(),
            },
            DccStableElementIdV1 {
                element_id: "dcc.direct_edit_denial.row.denial-attempt-1".to_string(),
                surface_id: "kernel002-dcc-mvp-runtime-mt024".to_string(),
                element_kind: "direct_edit_denial_row".to_string(),
                source_ref: "denial://denial-attempt-1".to_string(),
            },
            DccStableElementIdV1 {
                element_id: "dcc.promotion_preview.row.wb-crdt-patch-1".to_string(),
                surface_id: "kernel002-dcc-mvp-runtime-mt024".to_string(),
                element_kind: "promotion_preview_row".to_string(),
                source_ref: "writebox://wb-crdt-patch-1".to_string(),
            },
            DccStableElementIdV1 {
                element_id: "dcc.freshness_badge.crdt-patch-1".to_string(),
                surface_id: "kernel002-dcc-mvp-runtime-mt024".to_string(),
                element_kind: "freshness_badge".to_string(),
                source_ref: "eventledger://event-ledger-stream-crdt".to_string(),
            },
        ],
        catalog_action_refs: vec![
            "kernel.crdt_workspace.propose_patch".to_string(),
            "kernel.write_box.promote".to_string(),
            "kernel.direct_edit.deny".to_string(),
        ],
        catalog_action_rows: dcc_catalog_action_rows_from_catalog(
            &kernel002_action_catalog(),
            &[
                "kernel.crdt_workspace.propose_patch".to_string(),
                "kernel.write_box.promote".to_string(),
                "kernel.direct_edit.deny".to_string(),
            ],
        ),
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

#[test]
fn kernel_dcc_catalog_action_rows_carry_required_field_depth_for_every_ref() {
    let surface = sample_surface();

    assert_eq!(
        surface.catalog_action_rows.len(),
        surface.catalog_action_refs.len(),
        "every catalog_action_refs entry needs a row"
    );

    let catalog = kernel002_action_catalog();
    for action_id in &surface.catalog_action_refs {
        let row = surface
            .catalog_action_rows
            .iter()
            .find(|row| &row.action_id == action_id)
            .unwrap_or_else(|| panic!("missing row for action {action_id}"));
        let catalog_action = catalog
            .action(action_id)
            .unwrap_or_else(|| panic!("unknown catalog action {action_id}"));

        assert!(!row.action_id.is_empty());
        assert!(!row.target_authority_class.is_empty());
        assert_eq!(row.input_schema_id, catalog_action.input_schema_id);
        assert_eq!(row.result_schema_id, catalog_action.result_schema_id);
        assert!(!row.role_eligibility.is_empty());
        assert!(!row.capability_requirements.is_empty());
        assert!(!row.approval_posture.is_empty());
        assert_eq!(
            row.preview_behavior_summary,
            catalog_action.dcc_preview.summary
        );
        assert_eq!(row.preview_panel_id, catalog_action.dcc_preview.panel_id);
    }
}

#[test]
fn kernel_dcc_runtime_surface_rejects_catalog_rows_misaligned_with_catalog_action_refs() {
    let mut surface = sample_surface();
    surface.catalog_action_rows.pop();

    let errors = validate_dcc_mvp_runtime_surface(&surface)
        .expect_err("missing catalog_action_rows row must be rejected");
    assert!(errors
        .iter()
        .any(|error| error.field == "catalog_action_rows"));
}

#[test]
fn kernel_dcc_write_box_event_ledger_refs_match_appended_promotion_events() {
    let promoted_events = vec![
        synthetic_kernel_event(KernelEventType::PromotionRequested, "bridge-test-1"),
        synthetic_kernel_event(KernelEventType::PromotionAccepted, "bridge-test-1"),
    ];
    let ledger_result = synthetic_bridge_ledger_result(
        "bridge-test-1",
        CrdtPromotionBridgeStatus::Accepted,
        promoted_events.clone(),
    );

    let refs = write_box_event_ledger_refs_from_bridge(&ledger_result);

    assert_eq!(refs.len(), 2);
    assert!(refs[0].starts_with("eventledger://"));
    assert!(refs[0].contains(promoted_events[0].event_type.as_str()));
    assert!(refs[0].ends_with(&promoted_events[0].event_id));
    assert!(refs[1].contains(promoted_events[1].event_type.as_str()));
    assert!(refs[1].ends_with(&promoted_events[1].event_id));

    let empty_ledger_result = synthetic_bridge_ledger_result(
        "bridge-test-1",
        CrdtPromotionBridgeStatus::Accepted,
        Vec::new(),
    );
    assert!(write_box_event_ledger_refs_from_bridge(&empty_ledger_result).is_empty());
}

#[test]
fn kernel_dcc_write_box_stale_state_vector_flips_when_newer_crdt_update_lands() {
    assert!(!write_box_state_vector_is_stale("sv-1", "sv-1"));
    assert!(write_box_state_vector_is_stale("sv-1", "sv-2"));
    // Treat unknown freshness as "not stale" so untracked write boxes default to fresh.
    assert!(!write_box_state_vector_is_stale("", "sv-2"));
    assert!(!write_box_state_vector_is_stale("sv-1", ""));
}

#[test]
fn kernel_dcc_promotion_preview_fields_populate_state_vector_idempotency_and_event_kinds() {
    let input = synthetic_bridge_input("bridge-preview-1", "sv-fresh-1");

    let fields = derive_promotion_preview_fields(
        &input,
        CrdtPromotionBridgeStatus::Accepted,
        "sv-fresh-1",
        &[],
    );

    assert_eq!(fields.state_vector, "sv-fresh-1");
    assert_eq!(
        fields.idempotency_key,
        promotion_idempotency_key("bridge-preview-1", "requested")
    );
    assert!(fields
        .expected_event_kinds
        .contains(&"KernelCrdtPromotionRequestedV1".to_string()));
    assert!(fields
        .expected_event_kinds
        .contains(&"KernelCrdtPromotionAcceptedV1".to_string()));
    assert!(fields
        .validation_check_summaries
        .iter()
        .any(|summary| summary.contains("promotion_decision: ALLOWED")));
    assert!(fields
        .validation_check_summaries
        .iter()
        .any(|summary| summary.contains("state_vector: sv-fresh-1")));
    assert!(matches!(
        fields.stale_risk,
        DccPromotionPreviewStaleRisk::None
    ));
}

#[test]
fn kernel_dcc_promotion_preview_stale_risk_reflects_idempotency_duplicate_and_state_vector_drift() {
    let input = synthetic_bridge_input("bridge-preview-2", "sv-stale-base");

    let duplicate = derive_promotion_preview_fields(
        &input,
        CrdtPromotionBridgeStatus::Accepted,
        "sv-stale-base",
        &[promotion_idempotency_key("bridge-preview-2", "requested")],
    );
    assert!(matches!(
        duplicate.stale_risk,
        DccPromotionPreviewStaleRisk::DuplicateIdempotency
    ));

    let drift = derive_promotion_preview_fields(
        &input,
        CrdtPromotionBridgeStatus::Accepted,
        "sv-newer",
        &[],
    );
    assert!(matches!(
        drift.stale_risk,
        DccPromotionPreviewStaleRisk::StaleStateVector
    ));

    let both = derive_promotion_preview_fields(
        &input,
        CrdtPromotionBridgeStatus::Rejected,
        "sv-newer",
        &[promotion_idempotency_key("bridge-preview-2", "requested")],
    );
    assert!(matches!(
        both.stale_risk,
        DccPromotionPreviewStaleRisk::Both
    ));
    assert!(both
        .expected_event_kinds
        .contains(&"KernelCrdtPromotionRejectedV1".to_string()));
}

fn synthetic_kernel_event(event_type: KernelEventType, bridge_id: &str) -> KernelEvent {
    let idempotency_suffix = match event_type {
        KernelEventType::PromotionAccepted => "accepted",
        KernelEventType::PromotionRejected => "rejected",
        _ => "requested",
    };
    let new_event = NewKernelEvent::builder(
        format!("KTR-{bridge_id}"),
        format!("SR-{bridge_id}"),
        event_type,
        KernelActor::PromotionGate(format!("gate-{bridge_id}")),
    )
    .aggregate("crdt_promotion", bridge_id.to_string())
    .idempotency_key(promotion_idempotency_key(bridge_id, idempotency_suffix))
    .correlation_id(format!("trace-{bridge_id}"))
    .source_component("kernel_crdt_promotion_bridge")
    .payload(serde_json::json!({"bridge_id": bridge_id}))
    .build()
    .expect("synthetic kernel event builds");
    KernelEvent::from_new(new_event)
}

fn synthetic_bridge_ledger_result(
    bridge_id: &str,
    status: CrdtPromotionBridgeStatus,
    appended_events: Vec<KernelEvent>,
) -> CrdtPromotionBridgeLedgerResultV1 {
    CrdtPromotionBridgeLedgerResultV1 {
        schema_id: "hsk.kernel.crdt_promotion_bridge_result@1".to_string(),
        bridge_result: CrdtPromotionBridgeResultV1 {
            schema_id: "hsk.kernel.crdt_promotion_bridge_result@1".to_string(),
            bridge_id: bridge_id.to_string(),
            status,
            artifact_proposal: None,
            promotion_gate_input: None,
            event_mapping: None,
            event_mappings: Vec::new(),
            rejection_evidence: None,
        },
        appended_events,
    }
}

fn synthetic_bridge_input(bridge_id: &str, state_vector: &str) -> CrdtPromotionBridgeInputV1 {
    let identity = CrdtWorkspaceIdentityV1 {
        schema_id: "hsk.kernel.crdt_workspace_identity@1".to_string(),
        workspace_id: format!("workspace-{bridge_id}"),
        document_id: format!("document-{bridge_id}"),
        crdt_document_id: format!("crdt-{bridge_id}"),
        actor_id: "actor-test".to_string(),
        actor_kind: "kernel_builder".to_string(),
        crdt_site_id: "site-1".to_string(),
        crdt_client_id: "client-1".to_string(),
        document_schema_id: "hsk.test.document@1".to_string(),
        authority_links: CrdtAuthorityLinksV1 {
            work_item_id: format!("work-{bridge_id}"),
            action_trace_id: format!("trace-{bridge_id}"),
            artifact_proposal_id: format!("proposal-{bridge_id}"),
            role_mailbox_thread_id: format!("thread-{bridge_id}"),
            dcc_projection_id: format!("dcc-{bridge_id}"),
            event_ledger_stream_id: format!("stream-{bridge_id}"),
        },
    };
    let state = CrdtMaterializedStateV1 {
        identity,
        document_schema_id: "hsk.test.document@1".to_string(),
        state_vector: state_vector.to_string(),
        latest_update_seq: 7,
        fields: vec![CrdtMaterializedFieldV1 {
            field_id: "field-1".to_string(),
            field_path: "$.title".to_string(),
            text: "Title".to_string(),
            source_update_ids: vec!["update-1".to_string()],
        }],
    };
    let validation_report = CrdtPromotionValidationReportV1 {
        schema_id: "hsk.kernel.crdt_promotion_validation_report@1".to_string(),
        document_schema_id: "hsk.test.document@1".to_string(),
        state_vector: state_vector.to_string(),
        latest_update_seq: 7,
        decision: CrdtPromotionValidationDecision::Allowed,
        promotion_allowed: true,
        validation_errors: Vec::new(),
    };
    CrdtPromotionBridgeInputV1 {
        bridge_id: bridge_id.to_string(),
        artifact_proposal_id: format!("proposal-{bridge_id}"),
        promotion_gate_id: format!("gate-{bridge_id}"),
        promotion_target_ref: format!("authority://kernel/document/{bridge_id}"),
        state,
        validation_report,
    }
}
