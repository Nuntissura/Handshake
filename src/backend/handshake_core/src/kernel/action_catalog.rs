use std::collections::HashSet;

use super::action_envelope::{ApprovalPosture, AuthorityEffect, ExpectedWriteBoxRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequirement {
    pub capability_id: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationHook {
    pub hook_id: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DccPreviewMetadata {
    pub panel_id: String,
    pub summary: String,
    pub primary_state_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionPath {
    pub path_id: String,
    pub event_kind: String,
    pub receipt_kind: String,
    pub lawful_replacement_action_ids: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelCatalogActionV1 {
    pub action_id: &'static str,
    pub title: String,
    pub input_schema_id: String,
    pub result_schema_id: String,
    pub role_eligibility: Vec<String>,
    pub capability_requirements: Vec<CapabilityRequirement>,
    pub expected_write_boxes: Vec<ExpectedWriteBoxRef>,
    pub authority_effect: AuthorityEffect,
    pub approval_posture: ApprovalPosture,
    pub promotion_path: PromotionPath,
    pub validation_hooks: Vec<ValidationHook>,
    pub dcc_preview: DccPreviewMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelActionCatalogV1 {
    pub schema_id: &'static str,
    pub catalog_id: &'static str,
    pub version: u32,
    pub actions: Vec<KernelCatalogActionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelActionCatalogError {
    DuplicateActionId {
        action_id: &'static str,
    },
    EmptyField {
        action_id: &'static str,
        field: &'static str,
    },
}

impl KernelActionCatalogV1 {
    pub fn action(&self, action_id: &str) -> Option<&KernelCatalogActionV1> {
        self.actions
            .iter()
            .find(|action| action.action_id == action_id)
    }
}

pub fn kernel002_action_catalog() -> KernelActionCatalogV1 {
    KernelActionCatalogV1 {
        schema_id: "hsk.kernel_action_catalog@1",
        catalog_id: "kernel002-action-catalog-v1",
        version: 1,
        actions: vec![
            catalog_view_action(),
            crdt_workspace_propose_patch_action(),
            write_box_promote_action(),
            mirror_advisory_capture_action(),
            mirror_advisory_normalize_action(),
            direct_edit_deny_action(),
            software_delivery_runtime_truth_project_action(),
            workflow_transition_preview_action(),
            governance_overlay_project_action(),
            overlay_coordination_project_action(),
            overlay_lifecycle_project_action(),
            postgres_residual_project_action(),
            locus_work_tracking_project_action(),
            dcc_mvp_runtime_project_action(),
            dcc_structured_artifact_viewer_project_action(),
            dcc_layout_projection_registry_project_action(),
            role_mailbox_contract_project_action(),
            role_mailbox_loop_control_project_action(),
            role_mailbox_triage_queue_project_action(),
            role_mailbox_claim_lease_project_action(),
            role_mailbox_handoff_bundle_project_action(),
            role_mailbox_inbox_evidence_bridge_project_action(),
            fems_working_memory_checkpoint_project_action(),
            fems_write_time_safeguards_evaluate_action(),
            fems_memory_poisoning_drift_guardrails_evaluate_action(),
            fems_mt_handoff_memory_context_project_action(),
            role_turn_isolation_project_action(),
            work_profiles_project_action(),
            local_first_mcp_posture_project_action(),
            git_engine_decision_gate_project_action(),
            session_anti_pattern_registry_project_action(),
            governance_pack_instantiation_project_action(),
            session_spawn_tree_dcc_project_action(),
            session_spawn_conversation_distillation_project_action(),
            product_screenshot_capture_project_action(),
            visual_debugging_loop_project_action(),
            markdown_mirror_sync_drift_guard_project_action(),
        ],
    }
}

pub fn validate_kernel_action_catalog(
    catalog: &KernelActionCatalogV1,
) -> Result<(), Vec<KernelActionCatalogError>> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();

    for action in &catalog.actions {
        if !seen.insert(action.action_id) {
            errors.push(KernelActionCatalogError::DuplicateActionId {
                action_id: action.action_id,
            });
        }

        require_action_field(&mut errors, action, "action_id", action.action_id);
        require_action_field(&mut errors, action, "title", &action.title);
        require_action_field(
            &mut errors,
            action,
            "input_schema_id",
            &action.input_schema_id,
        );
        require_action_field(
            &mut errors,
            action,
            "result_schema_id",
            &action.result_schema_id,
        );
        require_action_vec(
            &mut errors,
            action,
            "role_eligibility",
            &action.role_eligibility,
        );
        require_action_vec(
            &mut errors,
            action,
            "capability_requirements",
            &action.capability_requirements,
        );
        require_action_vec(
            &mut errors,
            action,
            "expected_write_boxes",
            &action.expected_write_boxes,
        );
        require_action_vec(
            &mut errors,
            action,
            "validation_hooks",
            &action.validation_hooks,
        );
        require_action_field(
            &mut errors,
            action,
            "dcc_preview.panel_id",
            &action.dcc_preview.panel_id,
        );
        require_action_field(
            &mut errors,
            action,
            "dcc_preview.summary",
            &action.dcc_preview.summary,
        );
        require_action_vec(
            &mut errors,
            action,
            "dcc_preview.primary_state_fields",
            &action.dcc_preview.primary_state_fields,
        );
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn catalog_view_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.action_catalog.view",
        title: "View Kernel Action Catalog".to_string(),
        input_schema_id: "hsk.kernel.action_catalog_view_input@1".to_string(),
        result_schema_id: "hsk.kernel.action_catalog_view_result@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.catalog.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "kernel-action-catalog",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "catalog_view_projection",
            "KernelActionCatalogViewedV1",
            "STATUS",
        ),
        validation_hooks: vec![hook("catalog_schema_present")],
        dcc_preview: dcc_preview(
            "kernel-action-catalog",
            "List registered model-facing actions and their write-box effects.",
            &["action_id", "authority_effect", "approval_posture"],
        ),
    }
}

fn crdt_workspace_propose_patch_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.crdt_workspace.propose_patch",
        title: "Propose CRDT Workspace Patch".to_string(),
        input_schema_id: "hsk.kernel.crdt_patch_input@1".to_string(),
        result_schema_id: "hsk.kernel.crdt_patch_result@1".to_string(),
        role_eligibility: vec!["CODER".to_string(), "KERNEL_BUILDER".to_string()],
        capability_requirements: vec![
            capability("kernel.crdt.update.write"),
            capability("kernel.write_box.create"),
        ],
        expected_write_boxes: vec![
            expected_box(
                "CRDTWorkspaceBox",
                "hsk.write_box.crdt_workspace@1",
                "target_document",
            ),
            expected_box("ProposalBox", "hsk.write_box.proposal@1", "target_artifact"),
        ],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        promotion_path: promotion_path(
            "crdt_patch_to_promotion_box",
            "KernelCrdtPatchProposedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("schema_validity"),
            hook("state_vector_freshness"),
            hook("actor_capability"),
        ],
        dcc_preview: dcc_preview(
            "crdt-workspace",
            "Preview CRDT workspace changes before promotion.",
            &[
                "workspace_id",
                "document_id",
                "state_vector",
                "validation_state",
            ],
        ),
    }
}

fn write_box_promote_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.write_box.promote",
        title: "Promote Validated Write Box".to_string(),
        input_schema_id: "hsk.kernel.write_box_promote_input@1".to_string(),
        result_schema_id: "hsk.kernel.write_box_promote_result@1".to_string(),
        role_eligibility: vec![
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![
            capability("kernel.promotion.validate"),
            capability("event_ledger.append"),
        ],
        expected_write_boxes: vec![expected_box(
            "PromotionBox",
            "hsk.write_box.promotion@1",
            "promotion_target",
        )],
        authority_effect: AuthorityEffect::EventLedgerAuthorityWrite,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        promotion_path: promotion_path(
            "promotion_box_to_event_ledger",
            "KernelWriteBoxPromotionCommittedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("promotion_gate"),
            hook("idempotency"),
            hook("schema_validity"),
            hook("state_vector_freshness"),
        ],
        dcc_preview: dcc_preview(
            "write-box-promotion",
            "Validate write-box evidence and append authority events.",
            &["write_box_id", "promotion_state", "event_kind", "validator"],
        ),
    }
}

fn mirror_advisory_capture_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.mirror_advisory.capture",
        title: "Capture Mirror Advisory Edit".to_string(),
        input_schema_id: "hsk.kernel.mirror_advisory_input@1".to_string(),
        result_schema_id: "hsk.kernel.mirror_advisory_result@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.mirror.advisory.capture")],
        expected_write_boxes: vec![expected_box(
            "MirrorAdvisoryBox",
            "hsk.write_box.mirror_advisory@1",
            "mirror_target",
        )],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        promotion_path: promotion_path(
            "mirror_advisory_to_normalization",
            "KernelMirrorAdvisoryCapturedV1",
            "STATUS",
        ),
        validation_hooks: vec![hook("mirror_drift"), hook("normalization_candidate")],
        dcc_preview: dcc_preview(
            "mirror-advisory-queue",
            "Show advisory edits captured from generated mirrors.",
            &["mirror_path", "advisory_state", "normalization_action"],
        ),
    }
}

fn mirror_advisory_normalize_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.mirror_advisory.normalize",
        title: "Normalize Mirror Advisory Edit".to_string(),
        input_schema_id: "hsk.kernel.mirror_advisory_normalize_input@1".to_string(),
        result_schema_id: "hsk.kernel.mirror_advisory_normalize_result@1".to_string(),
        role_eligibility: vec![
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![
            capability("kernel.mirror.advisory.normalize"),
            capability("kernel.promotion.validate"),
        ],
        expected_write_boxes: vec![
            expected_box(
                "MirrorAdvisoryBox",
                "hsk.write_box.mirror_advisory@1",
                "mirror_target",
            ),
            expected_box(
                "PromotionBox",
                "hsk.write_box.promotion@1",
                "promotion_target",
            ),
        ],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        promotion_path: promotion_path(
            "mirror_advisory_normalization_to_promotion",
            "KernelMirrorAdvisoryNormalizationAcceptedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("mirror_drift"),
            hook("normalization_candidate"),
            hook("promotion_gate"),
        ],
        dcc_preview: dcc_preview(
            "mirror-advisory-normalization",
            "Validate advisory mirror edits before promotion.",
            &["mirror_path", "validation_state", "promotion_action"],
        ),
    }
}

fn direct_edit_deny_action() -> KernelCatalogActionV1 {
    let mut path = promotion_path("direct_edit_denial", "KernelDirectEditDeniedV1", "DENIAL");
    path.lawful_replacement_action_ids = vec![
        "kernel.mirror_advisory.capture",
        "kernel.crdt_workspace.propose_patch",
    ];

    KernelCatalogActionV1 {
        action_id: "kernel.direct_edit.deny",
        title: "Deny Direct Authority Edit".to_string(),
        input_schema_id: "hsk.kernel.direct_edit_attempt@1".to_string(),
        result_schema_id: "hsk.kernel.direct_edit_denial@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.direct_edit.guard")],
        expected_write_boxes: vec![expected_box(
            "DenialEvidenceBox",
            "hsk.write_box.denial_evidence@1",
            "attempt_target",
        )],
        authority_effect: AuthorityEffect::None,
        approval_posture: ApprovalPosture::Denied,
        promotion_path: path,
        validation_hooks: vec![
            hook("authority_boundary"),
            hook("replacement_action_lookup"),
        ],
        dcc_preview: dcc_preview(
            "direct-edit-denials",
            "Explain blocked authority edits and lawful replacement actions.",
            &["attempt_path", "denial_code", "replacement_actions"],
        ),
    }
}

fn software_delivery_runtime_truth_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.software_delivery_runtime_truth.project",
        title: "Project Software Delivery Runtime Truth".to_string(),
        input_schema_id: "hsk.kernel.software_delivery_runtime_posture_query@1".to_string(),
        result_schema_id: "hsk.kernel.software_delivery_runtime_posture_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.software_delivery.runtime_truth.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "software_delivery_runtime_truth",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "software_delivery_runtime_truth_projection",
            "KernelSoftwareDeliveryRuntimeTruthProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("stable_id_join"),
            hook("runtime_truth_source_kind"),
            hook("latest_record_seq"),
        ],
        dcc_preview: dcc_preview(
            "software-delivery-runtime-truth",
            "Project current software-delivery posture from product-owned runtime records.",
            &[
                "wp_id",
                "mt_id",
                "phase",
                "status",
                "record_seq",
                "next_actor",
            ],
        ),
    }
}

fn workflow_transition_preview_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.workflow_transition.preview",
        title: "Preview Workflow Transition".to_string(),
        input_schema_id: "hsk.kernel.workflow_transition_preview_input@1".to_string(),
        result_schema_id: "hsk.kernel.workflow_transition_preview_result@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.workflow_transition.preview")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "workflow_transition_registry",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "workflow_transition_preview_projection",
            "KernelWorkflowTransitionPreviewedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("transition_rule_registered"),
            hook("actor_eligibility"),
            hook("approval_boundary"),
            hook("dcc_preview_ready"),
        ],
        dcc_preview: dcc_preview(
            "workflow-transition-preview",
            "Preview workflow legality, actor eligibility, automation, and approval boundaries before mutation.",
            &[
                "rule_id",
                "governed_action_id",
                "eligible_actor_kinds",
                "approval_boundary",
            ],
        ),
    }
}

fn governance_overlay_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.governance_overlay.project",
        title: "Project Governance Overlay Boundary".to_string(),
        input_schema_id: "hsk.kernel.governance_overlay_projection_input@1".to_string(),
        result_schema_id: "hsk.kernel.governance_overlay_projection_result@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.governance_overlay.project")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "governance_overlay_boundary",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "governance_overlay_boundary_projection",
            "KernelGovernanceOverlayBoundaryProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("overlay_source_only"),
            hook("gate_required"),
            hook("no_runtime_truth_mutation"),
            hook("product_authority_refs_visible"),
        ],
        dcc_preview: dcc_preview(
            "governance-overlay-boundary",
            "Show imported .GOV overlay artifacts without elevating them to runtime authority.",
            &[
                "overlay_artifact_id",
                "overlay_role",
                "gate_refs",
                "product_runtime_authority_refs",
            ],
        ),
    }
}

fn overlay_coordination_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.overlay_coordination.project",
        title: "Project Overlay Coordination Posture".to_string(),
        input_schema_id: "hsk.kernel.overlay_coordination_query@1".to_string(),
        result_schema_id: "hsk.kernel.overlay_coordination_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.overlay_coordination.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "overlay_coordination_record",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "overlay_coordination_projection",
            "KernelOverlayCoordinationProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("coordination_source_kind"),
            hook("claim_lease_stable_id"),
            hook("queued_instruction_stable_id"),
            hook("actor_eligibility"),
        ],
        dcc_preview: dcc_preview(
            "overlay-coordination-posture",
            "Project claim, lease, takeover, follow-up, and actor eligibility from stable coordination records.",
            &[
                "coordination_id",
                "claimant",
                "lease_posture",
                "takeover_legality",
                "pending_instruction_ids",
            ],
        ),
    }
}

fn overlay_lifecycle_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.overlay_lifecycle.project",
        title: "Project Overlay Lifecycle Recovery".to_string(),
        input_schema_id: "hsk.kernel.overlay_lifecycle_query@1".to_string(),
        result_schema_id: "hsk.kernel.overlay_lifecycle_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.overlay_lifecycle.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "overlay_lifecycle_recovery_record",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "overlay_lifecycle_recovery_projection",
            "KernelOverlayLifecycleRecoveryProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("lifecycle_source_kind"),
            hook("checkpoint_lineage"),
            hook("governed_action_lineage"),
            hook("projection_safe"),
        ],
        dcc_preview: dcc_preview(
            "overlay-lifecycle-recovery",
            "Project start, steer, cancel, close, recover, checkpoint replay, partial failure, and restart posture.",
            &[
                "lifecycle_state",
                "recovery_posture",
                "checkpoint_ids",
                "partial_failure",
            ],
        ),
    }
}

fn postgres_residual_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.postgres_residual.project",
        title: "Project Postgres Residual Scope".to_string(),
        input_schema_id: "hsk.kernel.postgres_residual_scope_query@1".to_string(),
        result_schema_id: "hsk.kernel.postgres_residual_scope_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
        ],
        capability_requirements: vec![capability("kernel.postgres_residual.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "postgres_control_plane_residual_scope",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "postgres_residual_scope_projection",
            "KernelPostgresResidualScopeProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("postgres_residual_mapping"),
            hook("postgres_storage_authority"),
            hook("sqlite_boundary"),
            hook("folded_source_stubs"),
        ],
        dcc_preview: dcc_preview(
            "postgres-residual-scope",
            "Project folded Postgres control-plane residuals, target kernel mappings, blockers, and SQLite boundaries.",
            &[
                "source_stub_id",
                "kind",
                "disposition",
                "target_kernel_wp_id",
                "storage_authority_mode",
            ],
        ),
    }
}

fn locus_work_tracking_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.locus_work_tracking.project",
        title: "Project Locus Work Tracking Reset".to_string(),
        input_schema_id: "hsk.kernel.locus_work_tracking_reset_query@1".to_string(),
        result_schema_id: "hsk.kernel.locus_work_tracking_reset_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.locus_work_tracking.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "locus_work_tracking_reset",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "locus_work_tracking_reset_projection",
            "KernelLocusWorkTrackingResetProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("locus_sqlite_authority_removed"),
            hook("locus_dependency_query"),
            hook("locus_occupancy_state"),
            hook("flight_recorder_event_family"),
        ],
        dcc_preview: dcc_preview(
            "locus-work-tracking-reset",
            "Project WP/MT tracking, dependency readiness, occupancy, Task Board rows, and Flight Recorder events from product authority.",
            &["wp_id", "mt_id", "status", "active_session_ids", "blocked_by"],
        ),
    }
}

fn dcc_mvp_runtime_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.dcc_mvp_runtime.project",
        title: "Project DCC MVP Runtime Surface".to_string(),
        input_schema_id: "hsk.kernel.dcc_mvp_runtime_surface_query@1".to_string(),
        result_schema_id: "hsk.kernel.dcc_mvp_runtime_surface_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
        ],
        capability_requirements: vec![capability("kernel.dcc_mvp_runtime.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "dcc_mvp_runtime_surface",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "dcc_mvp_runtime_surface_projection",
            "KernelDccMvpRuntimeSurfaceProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("dcc_catalog_backed_actions"),
            hook("dcc_no_direct_authority_mutation"),
            hook("dcc_approval_preview"),
            hook("dcc_evidence_refs"),
        ],
        dcc_preview: dcc_preview(
            "dcc-mvp-runtime-surface",
            "Project DCC work, worktree, session, proposal, evidence, approval, and catalog action state.",
            &[
                "work_id",
                "worktree_id",
                "session_id",
                "proposal_id",
                "approval_preview_id",
            ],
        ),
    }
}

fn dcc_structured_artifact_viewer_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.dcc_structured_artifact_viewer.project",
        title: "Project DCC Structured Artifact Viewer".to_string(),
        input_schema_id: "hsk.kernel.dcc_structured_artifact_viewer_query@1".to_string(),
        result_schema_id: "hsk.kernel.dcc_structured_artifact_viewer_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
        ],
        capability_requirements: vec![capability("kernel.dcc_structured_artifact_viewer.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "dcc_structured_artifact_viewer",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "dcc_structured_artifact_viewer_projection",
            "KernelDccStructuredArtifactViewerProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("canonical_fields_first"),
            hook("mirror_state_visible"),
            hook("raw_drilldown_advanced"),
            hook("projection_layouts_only"),
        ],
        dcc_preview: dcc_preview(
            "dcc-structured-artifact-viewer",
            "Project canonical structured fields, mirror state, and advanced raw drilldown for DCC artifact viewers.",
            &[
                "record_id",
                "artifact_kind",
                "canonical_fields",
                "mirror_state",
                "raw_drilldown_mode",
            ],
        ),
    }
}

fn dcc_layout_projection_registry_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.dcc_layout_projection_registry.project",
        title: "Project DCC Layout Projection Registry".to_string(),
        input_schema_id: "hsk.kernel.dcc_layout_projection_registry_query@1".to_string(),
        result_schema_id: "hsk.kernel.dcc_layout_projection_registry_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.dcc_layout_projection_registry.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "dcc_layout_projection_registry",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "dcc_layout_projection_registry_projection",
            "KernelDccLayoutProjectionRegistryProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("dcc_layout_presets_registered"),
            hook("dcc_projection_action_bindings"),
            hook("dcc_base_envelope_fallback"),
            hook("dcc_local_queue_compact_summary"),
            hook("dcc_no_layout_local_mutation"),
        ],
        dcc_preview: dcc_preview(
            "dcc-layout-projection-registry",
            "Project DCC board, queue, list, roadmap, inbox triage, and execution queue presets with governed action bindings.",
            &[
                "preset_id",
                "layout_kind",
                "canonical_record_family_id",
                "action_bindings",
                "fallback_mode",
            ],
        ),
    }
}

fn role_mailbox_contract_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.role_mailbox_contract.project",
        title: "Project Role Mailbox Message Thread Contract".to_string(),
        input_schema_id: "hsk.kernel.role_mailbox_contract_query@1".to_string(),
        result_schema_id: "hsk.kernel.role_mailbox_contract_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.role_mailbox_contract.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "role_mailbox_contract",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "role_mailbox_contract_projection",
            "KernelRoleMailboxContractProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("mailbox_lifecycle_typed"),
            hook("mailbox_delivery_state_typed"),
            hook("mailbox_allowed_responses"),
            hook("mailbox_authority_boundary"),
            hook("mailbox_dead_letter_visible"),
        ],
        dcc_preview: dcc_preview(
            "role-mailbox-contract",
            "Project typed Role Mailbox lifecycle, delivery, allowed responses, due/dead-letter posture, and action boundaries.",
            &[
                "thread_id",
                "lifecycle_state",
                "latest_delivery_state",
                "allowed_responses",
                "dead_letter_posture",
            ],
        ),
    }
}

fn role_mailbox_loop_control_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.role_mailbox_loop_control.project",
        title: "Project Role Mailbox Micro-Task Loop Control".to_string(),
        input_schema_id: "hsk.kernel.role_mailbox_loop_control_query@1".to_string(),
        result_schema_id: "hsk.kernel.role_mailbox_loop_control_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.role_mailbox_loop_control.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "role_mailbox_loop_control",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "role_mailbox_loop_control_projection",
            "KernelRoleMailboxLoopControlProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("micro_task_loop_checkpoint"),
            hook("micro_task_verifier_outcome"),
            hook("retry_budget_visible"),
            hook("completion_transcription_posture"),
            hook("no_transcript_replay_authority"),
        ],
        dcc_preview: dcc_preview(
            "role-mailbox-loop-control",
            "Project compact Role Mailbox Micro-Task checkpoints, verifier outcomes, retry budget, escalation, completion, and dead-letter posture.",
            &[
                "mt_id",
                "loop_state",
                "remaining_retries",
                "verifier_outcome_kind",
                "completion_transcription_posture",
                "dead_letter_posture",
            ],
        ),
    }
}

fn role_mailbox_triage_queue_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.role_mailbox_triage_queue.project",
        title: "Project Role Mailbox Triage Queue Controls".to_string(),
        input_schema_id: "hsk.kernel.role_mailbox_triage_queue_query@1".to_string(),
        result_schema_id: "hsk.kernel.role_mailbox_triage_queue_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.role_mailbox_triage_queue.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "role_mailbox_triage_queue",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "role_mailbox_triage_queue_projection",
            "KernelRoleMailboxTriageQueueProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("mailbox_triage_queue_state"),
            hook("mailbox_reminder_schedule"),
            hook("mailbox_dead_letter_disposition"),
            hook("task_board_pressure_projection"),
            hook("no_transcript_queue_authority"),
        ],
        dcc_preview: dcc_preview(
            "role-mailbox-triage-queue",
            "Project Role Mailbox queue state, reminders, snooze/expiry, dead-letter remediation, and Task Board pressure overlays.",
            &[
                "thread_id",
                "queue_state",
                "reminder_schedule",
                "dead_letter_disposition",
                "pressure_level",
                "remediation_actions",
            ],
        ),
    }
}

fn role_mailbox_claim_lease_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.role_mailbox_claim_lease.project",
        title: "Project Role Mailbox Claim Lease Controls".to_string(),
        input_schema_id: "hsk.kernel.role_mailbox_claim_lease_query@1".to_string(),
        result_schema_id: "hsk.kernel.role_mailbox_claim_lease_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability("kernel.role_mailbox_claim_lease.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "role_mailbox_claim_lease",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "role_mailbox_claim_lease_projection",
            "KernelRoleMailboxClaimLeaseProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("mailbox_claim_lease_state"),
            hook("mailbox_executor_allowlist"),
            hook("mailbox_takeover_legality"),
            hook("mailbox_responder_eligibility"),
            hook("mailbox_no_work_state_authority"),
        ],
        dcc_preview: dcc_preview(
            "role-mailbox-claim-lease",
            "Project Role Mailbox claimant, claim mode, lease age/expiry, takeover legality, and responder eligibility.",
            &[
                "thread_id",
                "current_claimant",
                "claim_mode",
                "lease_expires_at",
                "takeover_legality",
                "responder_eligibility",
            ],
        ),
    }
}

fn role_mailbox_handoff_bundle_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.role_mailbox_handoff_bundle.project",
        title: "Project Role Mailbox Handoff Bundle".to_string(),
        input_schema_id: "hsk.kernel.role_mailbox_handoff_bundle_query@1".to_string(),
        result_schema_id: "hsk.kernel.role_mailbox_handoff_bundle_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.role_mailbox_handoff_bundle.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "role_mailbox_handoff_bundle",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "role_mailbox_handoff_bundle_projection",
            "KernelRoleMailboxHandoffBundleProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("mailbox_handoff_bundle_state"),
            hook("mailbox_transcription_targets"),
            hook("mailbox_announce_back_provenance"),
            hook("mailbox_advisory_completion_distinction"),
            hook("no_thread_replay_authority"),
        ],
        dcc_preview: dcc_preview(
            "role-mailbox-handoff-bundle",
            "Project Role Mailbox handoff bundles, transcription targets, recommended next actor, and announce-back provenance.",
            &[
                "bundle_id",
                "handoff_ready",
                "recommended_next_actor",
                "transcription_targets",
                "announce_back_provenance",
                "compact_summary",
            ],
        ),
    }
}

fn role_mailbox_inbox_evidence_bridge_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.role_mailbox_inbox_evidence_bridge.project",
        title: "Project Role Mailbox Inbox Evidence Bridge".to_string(),
        input_schema_id: "hsk.kernel.role_mailbox_inbox_evidence_bridge_query@1".to_string(),
        result_schema_id: "hsk.kernel.role_mailbox_inbox_evidence_bridge_projection@1"
            .to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "OPERATOR".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.role_mailbox_inbox_evidence_bridge.read",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "role_mailbox_inbox_evidence_bridge",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "role_mailbox_inbox_evidence_bridge_projection",
            "KernelRoleMailboxInboxEvidenceBridgeProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("mailbox_inbox_label_alignment"),
            hook("mailbox_telemetry_leak_safe"),
            hook("mailbox_debug_bundle_bounded_scope"),
            hook("mailbox_evidence_provenance_stable"),
            hook("no_parallel_inbox_authority"),
        ],
        dcc_preview: dcc_preview(
            "role-mailbox-inbox-evidence-bridge",
            "Project Inbox label alignment, leak-safe mailbox telemetry, and bounded debug-bundle mailbox evidence exports.",
            &[
                "label_id",
                "role_mailbox_route",
                "telemetry_event",
                "debug_bundle_id",
                "stable_provenance_refs",
                "recorder_correlation_ids",
            ],
        ),
    }
}

fn fems_working_memory_checkpoint_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.fems_working_memory_checkpoint.project",
        title: "Project FEMS Working-Memory Checkpoints".to_string(),
        input_schema_id: "hsk.kernel.fems_working_memory_checkpoint_query@1".to_string(),
        result_schema_id: "hsk.kernel.fems_working_memory_checkpoint_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
        ],
        capability_requirements: vec![capability("kernel.fems_working_memory_checkpoint.read")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "fems_working_memory_checkpoint",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "fems_working_memory_checkpoint_projection",
            "KernelFemsWorkingMemoryCheckpointProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("fems_checkpoint_type_coverage"),
            hook("fems_checkpoint_quality_gates"),
            hook("fems_session_close_extract_bridge"),
            hook("fems_repeated_insight_promotion"),
            hook("fems_working_memory_gc"),
            hook("fems_no_direct_memory_authority"),
        ],
        dcc_preview: dcc_preview(
            "fems-working-memory-checkpoints",
            "Project typed FEMS working-memory checkpoints, extract triggers, repeated insights, action-stream capture, and GC candidates.",
            &[
                "checkpoint_id",
                "checkpoint_kind",
                "session_id",
                "memory_extract_protocol_id",
                "promotion_candidate_count",
                "gc_candidate_count",
            ],
        ),
    }
}

fn fems_write_time_safeguards_evaluate_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.fems_write_time_safeguards.evaluate",
        title: "Evaluate FEMS Write-Time Safeguards".to_string(),
        input_schema_id: "hsk.kernel.fems_write_time_safeguards@1".to_string(),
        result_schema_id: "hsk.kernel.fems_write_time_safeguard_report@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability("kernel.fems_write_time_safeguards.evaluate")],
        expected_write_boxes: vec![expected_box(
            "SafeguardReportBox",
            "hsk.write_box.fems_safeguard_report@1",
            "fems_write_time_safeguards",
        )],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        promotion_path: promotion_path(
            "fems_write_time_safeguards_to_memory_promotion",
            "KernelFemsWriteTimeSafeguardsEvaluatedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("fems_write_time_mechanical_guards"),
            hook("fems_dedup_skip_rationale"),
            hook("fems_novelty_penalty_visible"),
            hook("fems_procedural_supersession"),
            hook("fems_contradiction_conflict_queue"),
            hook("fems_state_validation_scope_refs"),
            hook("fems_audit_trail_exportable"),
            hook("fems_reset_approved_storage_search"),
            hook("fems_no_direct_memory_authority"),
        ],
        dcc_preview: dcc_preview(
            "fems-write-time-safeguards",
            "Evaluate FEMS write-time dedup, novelty, supersession, contradiction, state validation, and audit outcomes before memory promotion.",
            &[
                "proposal_id",
                "summary_hash",
                "guard_outcomes",
                "skip_count",
                "conflict_count",
                "superseded_ids",
                "audit_ref",
                "storage_search_mode",
            ],
        ),
    }
}

fn fems_memory_poisoning_drift_guardrails_evaluate_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.fems_memory_poisoning_drift_guardrails.evaluate",
        title: "Evaluate FEMS Memory Poisoning and Drift Guardrails".to_string(),
        input_schema_id: "hsk.kernel.fems_memory_poisoning_drift_guardrails@1".to_string(),
        result_schema_id: "hsk.kernel.fems_memory_poisoning_drift_guardrail_report@1"
            .to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.fems_memory_poisoning_drift_guardrails.evaluate",
        )],
        expected_write_boxes: vec![expected_box(
            "SafeguardReportBox",
            "hsk.write_box.fems_memory_guardrail_report@1",
            "fems_memory_poisoning_drift_guardrails",
        )],
        authority_effect: AuthorityEffect::PrePromotionEvidenceOnly,
        approval_posture: ApprovalPosture::RequiresPromotionGate,
        promotion_path: promotion_path(
            "fems_memory_guardrails_to_memory_pack",
            "KernelFemsMemoryPoisoningDriftGuardrailsEvaluatedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("fems_procedural_trust_gate"),
            hook("fems_untrusted_long_lived_denial"),
            hook("fems_pack_budget_500_tokens"),
            hook("fems_deterministic_reduction_markers"),
            hook("fems_proposal_approval_denial_events"),
            hook("fems_effective_pack_hash"),
            hook("fems_drift_freshness_gate"),
            hook("fems_no_untrusted_drift"),
        ],
        dcc_preview: dcc_preview(
            "fems-memory-poisoning-drift-guardrails",
            "Evaluate FEMS trust gates, MemoryPack budget reduction, replay events, and effective pack hashes before model invocation.",
            &[
                "pack_id",
                "candidate_memory_id",
                "trust_level",
                "effective_pack_tokens",
                "deterministic_reduction_markers",
                "denied_memory_ids",
                "effective_pack_hash",
                "fr_event_refs",
            ],
        ),
    }
}

fn fems_mt_handoff_memory_context_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.fems_mt_handoff_memory_context.project",
        title: "Project FEMS MT Handoff Memory Context".to_string(),
        input_schema_id: "hsk.kernel.fems_mt_handoff_memory_context@1".to_string(),
        result_schema_id: "hsk.kernel.fems_mt_handoff_memory_context_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.fems_mt_handoff_memory_context.project",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "fems_mt_handoff_memory_context",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "fems_mt_handoff_memory_context_projection",
            "KernelFemsMtHandoffMemoryContextProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("fems_handoff_context_provenance"),
            hook("fems_handoff_source_target_sessions"),
            hook("fems_handoff_failed_attempts_visible"),
            hook("fems_handoff_recommended_item_boost"),
            hook("fems_handoff_budget_reduction"),
            hook("fems_handoff_no_long_term_merge"),
            hook("fems_handoff_locus_iteration_link"),
        ],
        dcc_preview: dcc_preview(
            "fems-mt-handoff-memory-context",
            "Project typed MT handoff memory context, failed attempts, recommended items, provenance, and bounded receiving-pack scoring.",
            &[
                "context_id",
                "source_session_id",
                "target_session_id",
                "handoff_reason",
                "selected_item_ids",
                "dropped_item_ids",
                "fr_event_ref",
                "locus_mt_iteration_ref",
            ],
        ),
    }
}

fn role_turn_isolation_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.role_turn_isolation.project",
        title: "Project Role Turn Isolation".to_string(),
        input_schema_id: "hsk.kernel.role_turn_isolation@1".to_string(),
        result_schema_id: "hsk.kernel.role_turn_isolation_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability("kernel.role_turn_isolation.project")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "role_turn_isolation",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "role_turn_isolation_projection",
            "KernelRoleTurnIsolationProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("role_turn_isolated_by_default"),
            hook("role_turn_reset_boundaries"),
            hook("role_turn_replay_pins_recorded"),
            hook("role_turn_requested_vs_effective_mode"),
            hook("role_turn_cross_role_bleed_denied"),
            hook("role_turn_degraded_reset_markers"),
        ],
        dcc_preview: dcc_preview(
            "role-turn-isolation",
            "Project role-pass isolation, reset boundaries, replay pins, requested/effective modes, and cross-role bleed denial.",
            &[
                "turn_id",
                "role_id",
                "pass_kind",
                "requested_mode",
                "effective_mode",
                "replay_pin_count",
                "trace_ref",
            ],
        ),
    }
}

fn work_profiles_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.work_profiles.project",
        title: "Project Work Profiles".to_string(),
        input_schema_id: "hsk.kernel.work_profiles@1".to_string(),
        result_schema_id: "hsk.kernel.work_profile_action_request_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability("kernel.work_profiles.project")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "work_profiles",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "work_profiles_projection",
            "KernelWorkProfilesProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("work_profile_storage_ref"),
            hook("work_profile_ids_immutable"),
            hook("work_profile_per_role_routes"),
            hook("work_profile_autonomy_knobs_bounded"),
            hook("work_profile_receipts_bound"),
            hook("work_profile_action_request_metadata"),
        ],
        dcc_preview: dcc_preview(
            "work-profiles",
            "Project selected Work Profile routes, autonomy knobs, profile receipts, and action request metadata bindings.",
            &[
                "profile_id",
                "role_id",
                "model_ref",
                "autonomy_max_auto_actions",
                "receipt_ref",
                "work_profile_id",
            ],
        ),
    }
}

fn local_first_mcp_posture_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.local_first_mcp_posture.project",
        title: "Project Local-First MCP Posture".to_string(),
        input_schema_id: "hsk.kernel.local_first_mcp_posture@1".to_string(),
        result_schema_id: "hsk.kernel.local_first_mcp_posture_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "LOCAL_SMALL_MODEL".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.local_first_mcp_posture.project",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "local_first_mcp_posture",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "local_first_mcp_posture_projection",
            "KernelLocalFirstMcpPostureProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("local_first_default"),
            hook("mcp_adapter_not_dependency"),
            hook("mcp_capability_gate"),
            hook("remote_artifact_cache"),
            hook("local_fallback_behavior"),
            hook("agentic_execution_recorder_ref"),
        ],
        dcc_preview: dcc_preview(
            "local-first-mcp-posture",
            "Project local-first agentic routing, MCP/cloud adapter gates, artifact caches, and deterministic local fallback behavior.",
            &[
                "route_id",
                "path_kind",
                "selected_by_default",
                "capability_gates",
                "cache_ref",
                "fallback_route_id",
            ],
        ),
    }
}

fn git_engine_decision_gate_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.git_engine_decision_gate.project",
        title: "Project Git Engine Decision Gate".to_string(),
        input_schema_id: "hsk.kernel.git_engine_decision_gate@1".to_string(),
        result_schema_id: "hsk.kernel.git_engine_decision_gate_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.git_engine_decision_gate.project",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "git_engine_decision_gate",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "git_engine_decision_gate_projection",
            "KernelGitEngineDecisionGateProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("git_single_backend_enforced"),
            hook("git_no_backend_fallback"),
            hook("git_oss_register_alignment"),
            hook("git_dangerous_actions_gated"),
            hook("git_dcc_lawful_affordances_only"),
            hook("git_flight_recorder_gate_refs"),
        ],
        dcc_preview: dcc_preview(
            "git-engine-decision-gate",
            "Project selected repo engine backend, OSS posture, dangerous action gates, and lawful DCC git affordances.",
            &[
                "selected_backend",
                "approved_backend_count",
                "action_id",
                "approval_required",
                "lawful_dcc_affordance",
                "flight_recorder_ref",
            ],
        ),
    }
}

fn session_anti_pattern_registry_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.session_anti_pattern_registry.project",
        title: "Project Session Anti-Pattern Registry".to_string(),
        input_schema_id: "hsk.kernel.session_anti_pattern_registry@1".to_string(),
        result_schema_id: "hsk.kernel.session_anti_pattern_registry_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.session_anti_pattern_registry.project",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "session_anti_pattern_registry",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "session_anti_pattern_registry_projection",
            "KernelSessionAntiPatternRegistryProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("session_anti_pattern_stable_ids"),
            hook("session_anti_pattern_detection_sources"),
            hook("session_anti_pattern_policy_outcomes"),
            hook("session_anti_pattern_flight_recorder_evidence"),
            hook("session_anti_pattern_required_coverage"),
        ],
        dcc_preview: dcc_preview(
            "session-anti-pattern-registry",
            "Project scheduler, trust-boundary, capability, and session-orchestration anti-pattern detections with deny, downgrade, consent, and stop outcomes.",
            &[
                "entry_id",
                "domain",
                "source_kind",
                "policy_outcome",
                "coverage_tags",
                "flight_recorder_refs",
            ],
        ),
    }
}

fn governance_pack_instantiation_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.governance_pack_instantiation.project",
        title: "Project Governance Pack Instantiation".to_string(),
        input_schema_id: "hsk.kernel.governance_pack_instantiation@1".to_string(),
        result_schema_id: "hsk.kernel.governance_pack_instantiation_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.governance_pack_instantiation.project",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "governance_pack_instantiation",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "governance_pack_instantiation_projection",
            "KernelGovernancePackInstantiationProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("governance_pack_project_identity"),
            hook("governance_pack_path_policy"),
            hook("governance_pack_manifest_items"),
            hook("governance_pack_conformance_harness"),
            hook("governance_pack_imported_overlay_boundary"),
            hook("governance_pack_action_write_box_law"),
        ],
        dcc_preview: dcc_preview(
            "governance-pack-instantiation",
            "Project governance pack identity, deterministic manifest paths, conformance harness checks, and imported overlay write-box boundaries.",
            &[
                "project_code",
                "manifest_item_id",
                "target_path_template",
                "imported_overlay_id",
                "conformance_check_ref",
                "kernel_law_compatible",
            ],
        ),
    }
}

fn session_spawn_tree_dcc_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.session_spawn_tree_dcc.project",
        title: "Project DCC Session Spawn Tree".to_string(),
        input_schema_id: "hsk.kernel.session_spawn_tree_dcc@1".to_string(),
        result_schema_id: "hsk.kernel.session_spawn_tree_dcc_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability("kernel.session_spawn_tree_dcc.project")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "session_spawn_tree_dcc",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "session_spawn_tree_dcc_projection",
            "KernelSessionSpawnTreeDccProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("session_spawn_tree_runtime_records"),
            hook("session_spawn_tree_parent_links"),
            hook("session_spawn_tree_visible_fields"),
            hook("session_spawn_tree_cascade_cancel"),
            hook("session_spawn_tree_announce_back_badges"),
        ],
        dcc_preview: dcc_preview(
            "session-spawn-tree",
            "Project session spawn hierarchy, depth, active child counts, cascade cancel affordances, spawn mode, and announce-back badges from runtime records.",
            &[
                "session_id",
                "parent_session_id",
                "depth",
                "child_count",
                "active_child_count",
                "spawn_mode",
                "cascade_cancel_available",
                "announce_back_badges",
            ],
        ),
    }
}

fn session_spawn_conversation_distillation_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.session_spawn_conversation_distillation.project",
        title: "Project Session Spawn Conversation Distillation".to_string(),
        input_schema_id: "hsk.kernel.session_spawn_conversation_distillation@1".to_string(),
        result_schema_id: "hsk.kernel.session_spawn_conversation_distillation_projection@1"
            .to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.session_spawn_conversation_distillation.project",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "session_spawn_conversation_distillation",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "session_spawn_conversation_distillation_projection",
            "KernelSessionSpawnConversationDistillationProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("spawn_distillation_pair_refs"),
            hook("spawn_distillation_metadata"),
            hook("spawn_distillation_artifact_refs"),
            hook("spawn_distillation_no_conversation_text_authority"),
            hook("spawn_distillation_no_training_corpus_mutation"),
        ],
        dcc_preview: dcc_preview(
            "session-spawn-conversation-distillation",
            "Project parent-child request/summary refs and spawn metadata into distillation artifacts without making conversation text authority.",
            &[
                "pair_id",
                "parent_request_ref",
                "child_summary_ref",
                "artifact_id",
                "depth",
                "child_role",
                "task_type",
                "conversation_text_authority",
            ],
        ),
    }
}

fn product_screenshot_capture_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.product_screenshot_capture.project",
        title: "Project Product Screenshot Capture".to_string(),
        input_schema_id: "hsk.kernel.product_screenshot_capture@1".to_string(),
        result_schema_id: "hsk.kernel.product_screenshot_capture_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability("kernel.product_screenshot_capture.project")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "product_screenshot_capture",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "product_screenshot_capture_projection",
            "KernelProductScreenshotCaptureProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("product_screenshot_capture_scopes"),
            hook("product_screenshot_request_metadata"),
            hook("product_screenshot_artifact_refs"),
            hook("product_screenshot_trigger_roles"),
            hook("product_screenshot_no_authority_mutation"),
        ],
        dcc_preview: dcc_preview(
            "product-screenshot-capture",
            "Project full-app, panel, and module screenshot capture requests with metadata and governed artifact refs.",
            &[
                "request_id",
                "scope",
                "target_ref",
                "trigger_kind",
                "window_title",
                "dimensions",
                "screenshot_ref",
                "metadata_ref",
            ],
        ),
    }
}

fn visual_debugging_loop_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.visual_debugging_loop.project",
        title: "Project Visual Debugging Loop".to_string(),
        input_schema_id: "hsk.kernel.visual_debugging_loop@1".to_string(),
        result_schema_id: "hsk.kernel.visual_debugging_loop_projection@1".to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability("kernel.visual_debugging_loop.project")],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "visual_debugging_loop",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "visual_debugging_loop_projection",
            "KernelVisualDebuggingLoopProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("visual_debugging_post_commit_trigger"),
            hook("visual_debugging_post_action_trigger"),
            hook("visual_debugging_baseline_comparison"),
            hook("visual_debugging_threshold_config"),
            hook("visual_debugging_evidence_artifacts"),
            hook("visual_debugging_validator_steering"),
        ],
        dcc_preview: dcc_preview(
            "visual-debugging-loop",
            "Project post-action and post-commit screenshot comparison, thresholded visual evidence, and validator steering.",
            &[
                "trigger_id",
                "trigger_kind",
                "screenshot_request_ref",
                "baseline_ref",
                "visual_diff_artifact_ref",
                "threshold_config_ref",
                "validator_steer_required",
            ],
        ),
    }
}

fn markdown_mirror_sync_drift_guard_project_action() -> KernelCatalogActionV1 {
    KernelCatalogActionV1 {
        action_id: "kernel.markdown_mirror_sync_drift_guard.project",
        title: "Project Markdown Mirror Sync Drift Guard".to_string(),
        input_schema_id: "hsk.kernel.markdown_mirror_sync_drift_guard@1".to_string(),
        result_schema_id: "hsk.kernel.markdown_mirror_sync_drift_guard_projection@1"
            .to_string(),
        role_eligibility: vec![
            "CODER".to_string(),
            "VALIDATOR".to_string(),
            "INTEGRATION_VALIDATOR".to_string(),
            "KERNEL_BUILDER".to_string(),
            "WORKFLOW_AUTOMATION".to_string(),
        ],
        capability_requirements: vec![capability(
            "kernel.markdown_mirror_sync_drift_guard.project",
        )],
        expected_write_boxes: vec![expected_box(
            "ReadOnlyProjectionBox",
            "hsk.write_box.readonly_projection@1",
            "markdown_mirror_sync_drift_guard",
        )],
        authority_effect: AuthorityEffect::ProjectionOnly,
        approval_posture: ApprovalPosture::NoApprovalRequired,
        promotion_path: promotion_path(
            "markdown_mirror_sync_drift_guard_projection",
            "KernelMarkdownMirrorSyncDriftGuardProjectedV1",
            "STATUS",
        ),
        validation_hooks: vec![
            hook("markdown_mirror_deterministic_regeneration"),
            hook("markdown_mirror_drift_states"),
            hook("markdown_mirror_advisory_handling"),
            hook("markdown_mirror_reconciliation_actions"),
            hook("markdown_mirror_dcc_queue"),
            hook("markdown_mirror_projection_banners"),
        ],
        dcc_preview: dcc_preview(
            "markdown-mirror-sync-drift-guard",
            "Project deterministic Markdown mirror regeneration, drift/advisory states, reconciliation actions, DCC queue items, and projection banners.",
            &[
                "contract_id",
                "surface_kind",
                "drift_state",
                "drift_source",
                "action_catalog_id",
                "dcc_queue_item",
                "banner_id",
            ],
        ),
    }
}

fn capability(capability_id: &str) -> CapabilityRequirement {
    CapabilityRequirement {
        capability_id: capability_id.to_string(),
        required: true,
    }
}

fn hook(hook_id: &str) -> ValidationHook {
    ValidationHook {
        hook_id: hook_id.to_string(),
        required: true,
    }
}

fn expected_box(kind: &str, schema_id: &str, target_id: &str) -> ExpectedWriteBoxRef {
    ExpectedWriteBoxRef {
        write_box_kind: kind.to_string(),
        write_box_schema_id: schema_id.to_string(),
        target_id: target_id.to_string(),
    }
}

fn promotion_path(path_id: &str, event_kind: &str, receipt_kind: &str) -> PromotionPath {
    PromotionPath {
        path_id: path_id.to_string(),
        event_kind: event_kind.to_string(),
        receipt_kind: receipt_kind.to_string(),
        lawful_replacement_action_ids: Vec::new(),
    }
}

fn dcc_preview(panel_id: &str, summary: &str, fields: &[&str]) -> DccPreviewMetadata {
    DccPreviewMetadata {
        panel_id: panel_id.to_string(),
        summary: summary.to_string(),
        primary_state_fields: fields.iter().map(|field| (*field).to_string()).collect(),
    }
}

fn require_action_field(
    errors: &mut Vec<KernelActionCatalogError>,
    action: &KernelCatalogActionV1,
    field: &'static str,
    value: &str,
) {
    if value.trim().is_empty() {
        errors.push(KernelActionCatalogError::EmptyField {
            action_id: action.action_id,
            field,
        });
    }
}

fn require_action_vec<T>(
    errors: &mut Vec<KernelActionCatalogError>,
    action: &KernelCatalogActionV1,
    field: &'static str,
    value: &[T],
) {
    if value.is_empty() {
        errors.push(KernelActionCatalogError::EmptyField {
            action_id: action.action_id,
            field,
        });
    }
}
