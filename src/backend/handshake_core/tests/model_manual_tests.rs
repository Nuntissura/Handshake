use std::collections::BTreeSet;

use regex::Regex;

use handshake_core::model_manual::{model_manual, CommandStatus, MANUAL_VERSION};

#[test]
fn manual_version_is_public_semver() {
    let semver = Regex::new(r"^\d+\.\d+\.\d+$").expect("semver regex");

    assert!(semver.is_match(MANUAL_VERSION));
    assert_eq!(model_manual().version, MANUAL_VERSION);
}

#[test]
fn command_reference_ids_are_unique_and_feature_group_refs_resolve() {
    let manual = model_manual();
    let mut command_ids = BTreeSet::new();
    for command in manual.command_reference {
        assert!(
            command_ids.insert(command.id),
            "duplicate command id {}",
            command.id
        );
        assert!(!command.name.trim().is_empty());
        assert!(!command.description.trim().is_empty());
        assert!(!command.expected_input.trim().is_empty());
        assert!(!command.expected_output.trim().is_empty());
    }

    let mut referenced_ids = BTreeSet::new();
    for group in manual.feature_groups {
        assert!(!group.id.trim().is_empty());
        assert!(
            !group.commands.is_empty(),
            "{} has no command refs",
            group.id
        );
        for command_id in group.commands {
            assert!(
                command_ids.contains(command_id),
                "feature group {} references missing command {}",
                group.id,
                command_id
            );
            referenced_ids.insert(*command_id);
        }
    }

    for command_id in command_ids {
        assert!(
            referenced_ids.contains(command_id),
            "orphan command reference {}",
            command_id
        );
    }
}

#[test]
fn v1_manual_covers_required_kernel004_feature_groups() {
    let manual = model_manual();
    let group_ids = manual
        .feature_groups
        .iter()
        .map(|group| group.id)
        .collect::<BTreeSet<_>>();

    for expected in [
        "hbr_process_diagnostics",
        "sandbox",
        "model_runtime",
        "memory_self_improvement",
    ] {
        assert!(
            group_ids.contains(expected),
            "missing feature group {expected}"
        );
    }
}

#[test]
fn model_manual_ipc_entries_are_present_as_wired_kernel004_surfaces() {
    let manual = model_manual();
    let ipc_channels = manual
        .command_reference
        .iter()
        .filter_map(|command| command.ipc_channel)
        .collect::<BTreeSet<_>>();

    for expected in [
        "kernel.model_manual.get",
        "kernel.model_manual.list_commands",
        "kernel.model_manual.search",
        "kernel.diagnostics.capture",
        "kernel.inspector.read",
    ] {
        assert!(
            ipc_channels.contains(expected),
            "missing IPC channel {expected}"
        );
    }

    for expected in [
        "model_manual_get",
        "model_manual_list_commands",
        "model_manual_search",
    ] {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == expected)
            .unwrap_or_else(|| panic!("missing command reference {expected}"));
        assert_eq!(command.status, CommandStatus::Wired);
    }

    let planned_commands = manual
        .command_reference
        .iter()
        .filter(|command| command.status == CommandStatus::Planned)
        .count();
    assert!(
        planned_commands >= 4,
        "expected planned KERNEL-004 placeholders"
    );
}

#[test]
fn model_runtime_registration_manual_matches_catalog_action_and_marks_catalog_only() {
    let manual = model_manual();
    let command = manual
        .command_reference
        .iter()
        .find(|command| command.id == "model_runtime_register_model")
        .expect("model runtime register_model manual command");

    assert_eq!(command.status, CommandStatus::Planned);
    assert_eq!(
        command.ipc_channel,
        Some("kernel.model_runtime.register_model")
    );
    assert_eq!(command.tauri_command, None);
    assert!(command.description.contains("Catalog-only"));
    assert!(command
        .recovery_steps
        .iter()
        .any(|step| step.contains("kernel.model_runtime.register_model")));
}

#[test]
fn process_ledger_wired_surface_is_manualized() {
    let manual = model_manual();
    assert_ne!(MANUAL_VERSION, "1.0.0");

    let hbr_group = manual
        .feature_groups
        .iter()
        .find(|group| group.id == "hbr_process_diagnostics")
        .expect("hbr/process diagnostics group");
    assert!(hbr_group.commands.contains(&"process_ledger_writer"));
    assert!(hbr_group
        .commands
        .contains(&"process_ledger_overflow_event"));
    assert!(hbr_group.commands.contains(&"process_ledger_reclaim"));
    assert!(hbr_group
        .commands
        .contains(&"process_ledger_staleness_reclaim"));

    let process_ledger = manual
        .command_reference
        .iter()
        .find(|command| command.id == "process_ledger_writer")
        .expect("process ledger command");
    assert_eq!(process_ledger.status, CommandStatus::Wired);
    assert!(process_ledger.schema_fields.contains(&"process_uuid"));
    assert!(process_ledger.expected_output.contains("Postgres"));

    let overflow = manual
        .command_reference
        .iter()
        .find(|command| command.id == "process_ledger_overflow_event")
        .expect("process ledger overflow command");
    assert_eq!(overflow.status, CommandStatus::Wired);
    assert!(overflow.expected_output.contains("FR_EVT_LEDGER_OVERFLOW"));

    let reclaim = manual
        .command_reference
        .iter()
        .find(|command| command.id == "process_ledger_reclaim")
        .expect("process ledger reclaim command");
    assert_eq!(reclaim.status, CommandStatus::Wired);
    assert!(reclaim.expected_output.contains("ReclaimReport"));
    assert!(reclaim
        .recovery_steps
        .iter()
        .any(|step| step.contains("STOP")));
}

#[test]
fn inspector_replay_drive_wired_surface_is_manualized() {
    let manual = model_manual();
    let hbr_group = manual
        .feature_groups
        .iter()
        .find(|group| group.id == "hbr_process_diagnostics")
        .expect("hbr/process diagnostics group");
    assert!(hbr_group.commands.contains(&"inspector_replay_drive"));

    let command = manual
        .command_reference
        .iter()
        .find(|command| command.id == "inspector_replay_drive")
        .expect("inspector replay-drive command");

    assert_eq!(command.status, CommandStatus::Wired);
    assert_eq!(command.ipc_channel, Some("/inspector/v1/replay-drive"));
    assert!(command.schema_fields.contains(&"action_id"));
    assert!(command.schema_fields.contains(&"envelope"));
    assert!(command.expected_output.contains("INSPECTOR_REPLAY_DRIVE"));
    assert!(command
        .expected_input
        .contains("exactly action_id and envelope"));
}

#[test]
fn safety_constraints_and_workflows_cover_no_context_operation() {
    let manual = model_manual();
    let safety_text = manual
        .safety_constraints
        .iter()
        .map(|constraint| constraint.constraint_text)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(safety_text.contains("HBR-MAN-001"));
    assert!(safety_text.contains("HBR-MAN-002"));
    assert!(safety_text.contains("HBR-QUIET"));

    let workflow_ids = manual
        .workflows
        .iter()
        .map(|workflow| workflow.id)
        .collect::<BTreeSet<_>>();
    for expected in [
        "startup",
        "governed_session_run",
        "diagnostics_panel_triage",
    ] {
        assert!(
            workflow_ids.contains(expected),
            "missing workflow {expected}"
        );
    }
}

/// Runtime proof for the WP-KERNEL-005 atelier (Core-Data) microtasks
/// MT-052..MT-060 and MT-073..MT-075: each atelier surface area must be a
/// real, no-context ModelManual row — a feature group, its referenced commands
/// (which must resolve to CommandReference entries), and a covering workflow.
#[test]
fn manual_covers_atelier_core_data_surfaces() {
    let manual = model_manual();

    let group_ids = manual
        .feature_groups
        .iter()
        .map(|group| group.id)
        .collect::<BTreeSet<_>>();
    let command_ids = manual
        .command_reference
        .iter()
        .map(|command| command.id)
        .collect::<BTreeSet<_>>();
    let workflow_ids = manual
        .workflows
        .iter()
        .map(|workflow| workflow.id)
        .collect::<BTreeSet<_>>();

    // (MT id, feature_group_id, representative command ids, workflow id) for
    // each of the 12 atelier microtask areas.
    let coverage: &[(&str, &str, &[&str], &str)] = &[
        (
            "MT-052",
            "atelier_character_core",
            &[
                "atelier_create_character",
                "atelier_get_character_by_public_id",
                "atelier_append_sheet_version",
                "atelier_apply_sheet_field_edits",
                "atelier_sheet_version_history",
            ],
            "atelier_character_identity_and_sheet",
        ),
        (
            "MT-053",
            "atelier_media_intake",
            &[
                "atelier_materialize_media_asset",
                "atelier_open_intake_batch",
                "atelier_list_intake_batch_items",
                "atelier_apply_intake_classification",
                "atelier_bulk_update_media_review_metadata",
            ],
            "atelier_media_library_and_intake_review",
        ),
        (
            "MT-054",
            "atelier_collections_contact_sheets",
            &[
                "atelier_create_collection",
                "atelier_add_images_to_collection",
                "atelier_create_contact_sheet",
                "atelier_render_contact_sheet_svg_artifact",
                "atelier_plan_contact_sheet_raster_export",
            ],
            "atelier_collections_and_contact_sheets",
        ),
        (
            "MT-055",
            "atelier_documents_scripts",
            &[
                "atelier_create_character_document",
                "atelier_append_character_document_version",
                "atelier_add_story_card",
                "atelier_add_story_beat",
                "atelier_create_character_script",
            ],
            "atelier_documents_story_scripts",
        ),
        (
            "MT-056",
            "atelier_moodboards",
            &[
                "atelier_record_moodboard_snapshot",
                "atelier_record_moodboard_operation",
                "atelier_request_moodboard_export",
            ],
            "atelier_moodboard_snapshot_and_export",
        ),
        (
            "MT-057",
            "atelier_relationships",
            &[
                "atelier_create_character_relationship",
                "atelier_update_character_relationship",
                "atelier_character_relationship_graph",
            ],
            "atelier_relationship_graph",
        ),
        (
            "MT-058",
            "atelier_search_tags_similarity",
            &[
                "atelier_global_search_with_lens_filters",
                "atelier_ensure_tag",
                "atelier_create_tag_rule",
                "atelier_record_ai_tag_suggestion",
                "atelier_find_similar_assets",
            ],
            "atelier_search_palette_and_similarity",
        ),
        (
            "MT-059",
            "atelier_exports",
            &["atelier_request_web_portfolio_export"],
            "atelier_web_portfolio_export",
        ),
        (
            "MT-073",
            "atelier_exports",
            &[
                "atelier_request_sheet_export",
                "atelier_build_share_pack_manifest",
            ],
            "atelier_share_pack_export",
        ),
        (
            "MT-074",
            "atelier_exports",
            &["atelier_build_llm_evidence_pack_manifest"],
            "atelier_llm_evidence_pack_export",
        ),
        (
            "MT-075",
            "atelier_exports",
            &[
                "atelier_record_backup_manifest",
                "atelier_backup_restore_preflight",
            ],
            "atelier_backup_restore_preflight",
        ),
        (
            "MT-060",
            "atelier_reset_recovery",
            &[
                "atelier_record_atelier_reset",
                "atelier_list_orphan_manifest_items",
                "atelier_adopt_orphan_manifest_item",
            ],
            "atelier_reset_recovery_and_orphan_adoption",
        ),
    ];

    for (mt, group_id, commands, workflow_id) in coverage {
        assert!(
            group_ids.contains(group_id),
            "{mt}: missing atelier feature group {group_id}"
        );

        let group = manual
            .feature_groups
            .iter()
            .find(|group| group.id == *group_id)
            .unwrap_or_else(|| panic!("{mt}: feature group {group_id} not found"));

        for command_id in *commands {
            // The command must exist as a CommandReference entry...
            assert!(
                command_ids.contains(command_id),
                "{mt}: command {command_id} has no CommandReference entry"
            );
            // ...and be referenced by the area's feature group so the
            // self-consistency invariant (no orphan refs) holds.
            assert!(
                group.commands.contains(command_id),
                "{mt}: feature group {group_id} does not reference command {command_id}"
            );
        }

        assert!(
            workflow_ids.contains(workflow_id),
            "{mt}: missing atelier workflow {workflow_id}"
        );
    }

    // The wired atelier surfaces backed by real Axum routes in
    // src/api/atelier.rs must be marked Wired with their HTTP route as the
    // ipc_channel, never invented Tauri commands.
    for (command_id, route) in [
        ("atelier_open_intake_batch", "/atelier/intake/batches"),
        (
            "atelier_list_intake_batch_items",
            "/atelier/intake/batches/:batch_id/items",
        ),
        (
            "atelier_record_ai_tag_suggestion",
            "/atelier/ai-tag-suggestions",
        ),
    ] {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == command_id)
            .unwrap_or_else(|| panic!("missing wired atelier command {command_id}"));
        assert_eq!(
            command.status,
            CommandStatus::Wired,
            "{command_id} must be Wired"
        );
        assert_eq!(
            command.ipc_channel,
            Some(route),
            "{command_id} must carry its Axum route"
        );
        assert_eq!(
            command.tauri_command, None,
            "{command_id} is an HTTP route, not a Tauri command"
        );
    }
}

/// Runtime proof for the WP-KERNEL-005 Pose / ComfyUI pipeline microtasks
/// MT-122..MT-125: each pipeline area must be a real, no-context ModelManual
/// row — a feature group, its FULL command list (which must each resolve to a
/// CommandReference entry and be referenced by the group, so a dropped row
/// fails), and a covering workflow.
#[test]
fn manual_covers_pose_comfy_surfaces() {
    let manual = model_manual();

    let group_ids = manual
        .feature_groups
        .iter()
        .map(|group| group.id)
        .collect::<BTreeSet<_>>();
    let command_ids = manual
        .command_reference
        .iter()
        .map(|command| command.id)
        .collect::<BTreeSet<_>>();
    let workflow_ids = manual
        .workflows
        .iter()
        .map(|workflow| workflow.id)
        .collect::<BTreeSet<_>>();

    // (MT id, feature_group_id, FULL command id set, workflow id) for each of
    // the 4 pose/comfy microtask areas. The command set is complete (not just
    // representative) so dropping any single manual row fails this test.
    let coverage: &[(&str, &str, &[&str], &str)] = &[
        (
            "MT-122",
            "atelier_pose_context_and_rig",
            &[
                "atelier_set_pose_context_state",
                "atelier_current_pose_context_state",
                "atelier_ingest_pose_rig",
                "atelier_list_pose_rigs",
                "atelier_get_pose_rig",
                "atelier_set_pose_calibration",
                "atelier_get_calibration",
            ],
            "atelier_pose_context_and_rig",
        ),
        (
            "MT-123",
            "atelier_pose_sidecar_and_identity",
            &[
                "atelier_record_pose_sidecar",
                "atelier_list_pose_sidecars",
                "atelier_pose_sidecar_gallery_projection",
                "atelier_append_identity_profile",
                "atelier_update_identity_profile",
                "atelier_record_identity_crop_artifact",
                "atelier_latest_identity_profile",
            ],
            "atelier_pose_sidecar_and_identity",
        ),
        (
            "MT-124",
            "atelier_comfy_workflow_receipts",
            &[
                "atelier_record_comfy_workflow_receipt",
                "atelier_get_comfy_workflow_receipt",
                "atelier_list_comfy_workflow_history",
                "atelier_produce_intake_receipt",
                "atelier_mark_saveimage_fallback",
                "atelier_record_comfy_output_registration_failure",
                "atelier_retry_comfy_output_registration_failure",
            ],
            "atelier_comfy_workflow_receipts",
        ),
        (
            "MT-125",
            "atelier_pose_comfy_deferred_boundaries",
            &[
                "atelier_register_bridge_capability",
                "atelier_list_capability_rejects",
                "atelier_record_url_image_import",
                "atelier_set_calibration_blocked",
            ],
            "atelier_pose_comfy_deferred_boundaries",
        ),
    ];

    for (mt, group_id, commands, workflow_id) in coverage {
        assert!(
            group_ids.contains(group_id),
            "{mt}: missing pose/comfy feature group {group_id}"
        );

        let group = manual
            .feature_groups
            .iter()
            .find(|group| group.id == *group_id)
            .unwrap_or_else(|| panic!("{mt}: feature group {group_id} not found"));

        // The group's command list must be exactly the contract command set
        // (same length) so neither a dropped nor a snuck-in row passes silently.
        assert_eq!(
            group.commands.len(),
            commands.len(),
            "{mt}: feature group {group_id} command count drifted from the contract set"
        );

        for command_id in *commands {
            // The command must exist as a CommandReference entry...
            assert!(
                command_ids.contains(command_id),
                "{mt}: command {command_id} has no CommandReference entry"
            );
            // ...and be referenced by the area's feature group so the
            // self-consistency invariant (no orphan refs) holds.
            assert!(
                group.commands.contains(command_id),
                "{mt}: feature group {group_id} does not reference command {command_id}"
            );
        }

        assert!(
            workflow_ids.contains(workflow_id),
            "{mt}: missing pose/comfy workflow {workflow_id}"
        );
    }

    // The one wired pose/comfy surface backed by a real Axum route in
    // src/api/atelier.rs must be marked Wired with its HTTP route as the
    // ipc_channel and no invented Tauri command. All other pose/comfy commands
    // are Planned because src/api/atelier.rs::routes() registers no pose or
    // comfy routes.
    let url_import = manual
        .command_reference
        .iter()
        .find(|command| command.id == "atelier_record_url_image_import")
        .expect("wired atelier_record_url_image_import command");
    assert_eq!(url_import.status, CommandStatus::Wired);
    assert_eq!(url_import.ipc_channel, Some("/atelier/image-import/url"));
    assert_eq!(url_import.tauri_command, None);

    for planned_id in [
        "atelier_ingest_pose_rig",
        "atelier_record_pose_sidecar",
        "atelier_record_comfy_workflow_receipt",
        "atelier_register_bridge_capability",
        "atelier_set_calibration_blocked",
    ] {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == planned_id)
            .unwrap_or_else(|| panic!("missing planned pose/comfy command {planned_id}"));
        assert_eq!(
            command.status,
            CommandStatus::Planned,
            "{planned_id} must be Planned (no Axum route exists for it)"
        );
        assert_eq!(command.ipc_channel, None, "{planned_id} must not claim a route");
    }
}

/// Runtime proof for the WP-KERNEL-005 Model-Workflow-Diagnostics microtasks
/// MT-132, MT-135, MT-159, MT-164, MT-181, MT-183, MT-185, MT-186, MT-187:
/// each diagnostics area must be a real, no-context ModelManual row — a feature
/// group, its FULL command list (each resolving to a CommandReference entry and
/// referenced by the group, so a dropped row fails), and a covering workflow.
#[test]
fn manual_covers_diagnostics_surfaces() {
    let manual = model_manual();

    let group_ids = manual
        .feature_groups
        .iter()
        .map(|group| group.id)
        .collect::<BTreeSet<_>>();
    let command_ids = manual
        .command_reference
        .iter()
        .map(|command| command.id)
        .collect::<BTreeSet<_>>();
    let workflow_ids = manual
        .workflows
        .iter()
        .map(|workflow| workflow.id)
        .collect::<BTreeSet<_>>();

    // (MT id, feature_group_id, FULL command id set, workflow id) for each of
    // the 9 diagnostics microtask areas. The command set is complete (not just
    // representative) so dropping any single manual row fails this test.
    let coverage: &[(&str, &str, &[&str], &str)] = &[
        (
            "MT-132",
            "diagnostics_source_evidence_matrix",
            &[
                "diagnostics_source_evidence_matrix",
                "kernel_action_catalog_view",
                "kernel_inspector_list_sessions",
                "kernel_inspector_loaded_models",
                "diagnostics_problem_store_query",
            ],
            "diagnostics_source_evidence_matrix",
        ),
        (
            "MT-135",
            "diagnostics_no_context_manual_structure",
            &[
                "model_manual_get",
                "model_manual_list_commands",
                "model_manual_search",
                "kernel_model_manual_update_section",
            ],
            "diagnostics_no_context_manual_structure",
        ),
        (
            "MT-159",
            "diagnostics_gui_verification_checklist",
            &[
                "visual_debug_dom_snapshot",
                "visual_debug_console_stream_start",
                "visual_debug_console_stream_stop",
                "kernel_product_screenshot_capture_execute",
                "kernel_visual_debugging_loop_project",
            ],
            "diagnostics_gui_verification_checklist",
        ),
        (
            "MT-164",
            "diagnostics_build_rules_read_evidence",
            &[
                "diagnostics_hbr_handoff_gate_evaluate",
                "hbr_matrix_check",
                "hbr_validator_scan",
                "hbr_violation_emit",
            ],
            "diagnostics_build_rules_read_evidence",
        ),
        (
            "MT-181",
            "diagnostics_integration_smoke_path",
            &[
                "kernel_role_mailbox_claim_lease_project",
                "kernel_inspector_session_state",
                "inspector_replay_drive",
                "kernel_inspector_event_ledger_tail",
                "diagnostics_debug_bundle_export",
            ],
            "diagnostics_integration_smoke_path",
        ),
        (
            "MT-183",
            "diagnostics_red_team_drift_guards",
            &[
                "diagnostics_manual_drift_guard",
                "kernel_action_catalog_view",
                "kernel_visual_debugging_loop_project",
            ],
            "diagnostics_red_team_drift_guards",
        ),
        (
            "MT-185",
            "diagnostics_core_row_merge",
            &[
                "diagnostics_core_row_merge",
                "atelier_create_character",
                "atelier_materialize_media_asset",
                "atelier_request_web_portfolio_export",
            ],
            "diagnostics_core_row_merge",
        ),
        (
            "MT-186",
            "diagnostics_pose_row_merge",
            &[
                "diagnostics_pose_row_merge",
                "atelier_ingest_pose_rig",
                "atelier_record_pose_sidecar",
                "atelier_record_comfy_workflow_receipt",
            ],
            "diagnostics_pose_row_merge",
        ),
        (
            "MT-187",
            "diagnostics_owned_row_merge",
            &[
                "diagnostics_owned_row_merge",
                "diagnostics_problem_store_query",
                "kernel_action_catalog_view",
                "kernel_inspector_session_state",
                "diagnostics_debug_bundle_export",
            ],
            "diagnostics_owned_row_merge",
        ),
    ];

    for (mt, group_id, commands, workflow_id) in coverage {
        assert!(
            group_ids.contains(group_id),
            "{mt}: missing diagnostics feature group {group_id}"
        );

        let group = manual
            .feature_groups
            .iter()
            .find(|group| group.id == *group_id)
            .unwrap_or_else(|| panic!("{mt}: feature group {group_id} not found"));

        // The group's command list must be exactly the contract command set
        // (same length) so neither a dropped nor a snuck-in row passes silently.
        assert_eq!(
            group.commands.len(),
            commands.len(),
            "{mt}: feature group {group_id} command count drifted from the contract set"
        );

        for command_id in *commands {
            // The command must exist as a CommandReference entry...
            assert!(
                command_ids.contains(command_id),
                "{mt}: command {command_id} has no CommandReference entry"
            );
            // ...and be referenced by the area's feature group so the
            // self-consistency invariant (no orphan refs) holds.
            assert!(
                group.commands.contains(command_id),
                "{mt}: feature group {group_id} does not reference command {command_id}"
            );
        }

        assert!(
            workflow_ids.contains(workflow_id),
            "{mt}: missing diagnostics workflow {workflow_id}"
        );
    }

    // Diagnostics surfaces backed by a real registered KernelActionCatalogV1
    // action (src/kernel/action_catalog.rs) are Wired with their action_id as
    // the ipc_channel and no invented Tauri command.
    for (command_id, action_id) in [
        ("kernel_action_catalog_view", "kernel.action_catalog.view"),
        (
            "kernel_model_manual_update_section",
            "kernel.model_manual.update_section",
        ),
        (
            "kernel_product_screenshot_capture_execute",
            "kernel.product_screenshot_capture.execute",
        ),
        (
            "kernel_visual_debugging_loop_project",
            "kernel.visual_debugging_loop.project",
        ),
        (
            "kernel_role_mailbox_claim_lease_project",
            "kernel.role_mailbox_claim_lease.project",
        ),
    ] {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == command_id)
            .unwrap_or_else(|| panic!("missing wired diagnostics command {command_id}"));
        assert_eq!(
            command.status,
            CommandStatus::Wired,
            "{command_id} must be Wired (registered catalog action)"
        );
        assert_eq!(
            command.ipc_channel,
            Some(action_id),
            "{command_id} must carry its registered action_id"
        );
        assert_eq!(
            command.tauri_command, None,
            "{command_id} is a catalog action, not a Tauri command"
        );
    }

    // Diagnostics surfaces that are real backend traits/methods but have no
    // registered IPC route yet must be Planned and must not claim a route.
    for planned_id in [
        "diagnostics_source_evidence_matrix",
        "diagnostics_problem_store_query",
        "diagnostics_hbr_handoff_gate_evaluate",
        "diagnostics_debug_bundle_export",
    ] {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == planned_id)
            .unwrap_or_else(|| panic!("missing planned diagnostics command {planned_id}"));
        assert_eq!(
            command.status,
            CommandStatus::Planned,
            "{planned_id} must be Planned (no registered IPC route exists for it)"
        );
        assert_eq!(
            command.ipc_channel, None,
            "{planned_id} must not claim a route"
        );
    }

    // MT-183/185/186/187 merge and drift-guard surfaces are executable backend
    // library checks (src/atelier/model_manual_merge.rs) without an IPC route:
    // Wired like the other library surfaces, but never claiming a route.
    for wired_library_id in [
        "diagnostics_manual_drift_guard",
        "diagnostics_core_row_merge",
        "diagnostics_pose_row_merge",
        "diagnostics_owned_row_merge",
    ] {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == wired_library_id)
            .unwrap_or_else(|| panic!("missing diagnostics merge/guard command {wired_library_id}"));
        assert_eq!(
            command.status,
            CommandStatus::Wired,
            "{wired_library_id} must be Wired (executable library surface)"
        );
        assert_eq!(
            command.ipc_channel, None,
            "{wired_library_id} must not claim a route"
        );
        assert_eq!(
            command.tauri_command, None,
            "{wired_library_id} is a library surface, not a Tauri command"
        );
        assert!(
            command
                .description
                .contains("src/atelier/model_manual_merge.rs"),
            "{wired_library_id} must cite its executable implementation"
        );
    }

    // The manual version moved to the merge/drift-guard wired increment
    // (HBR-MAN-001: wired-surface diff bumps MANUAL_VERSION).
    assert_eq!(MANUAL_VERSION, "1.4.0");
}
