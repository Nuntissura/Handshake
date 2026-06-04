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
