use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use handshake_core::model_manual::{model_manual, CommandStatus};

#[test]
fn swarm_ipc_bridge_is_debug_feature_gated_and_registered() {
    let repo = repo_root();
    let swarm_rs = read(repo.join("app/src-tauri/src/swarm.rs"));
    let lib_rs = read(repo.join("app/src-tauri/src/lib.rs"));
    let cargo_toml = read(repo.join("app/src-tauri/Cargo.toml"));

    assert!(cargo_toml.contains("[features]"));
    assert!(cargo_toml.contains("swarm_ipc"));
    assert!(cargo_toml.contains("handshake_core/swarm_ipc"));
    assert!(lib_rs.contains("mod swarm;"));
    assert!(lib_rs.contains("feature = \"swarm_ipc\""));
    assert!(lib_rs.contains("swarm::kernel_swarm_run"));
    assert!(swarm_rs.contains("KERNEL_SWARM_RUN_IPC_CHANNEL"));
    assert!(swarm_rs.contains("kernel.swarm.run"));
    assert!(swarm_rs.contains("debug_assertions"));
    assert!(swarm_rs.contains("feature = \"swarm_ipc\""));
    assert!(swarm_rs.contains("run_swarm_scenario"));
}

#[test]
fn swarm_ipc_model_manual_documents_cli_ipc_and_builtin_scenarios() {
    let manual = model_manual();
    let hbr_group = manual
        .feature_groups
        .iter()
        .find(|group| group.id == "hbr_process_diagnostics")
        .expect("hbr/process diagnostics group");
    let command_ids = manual
        .command_reference
        .iter()
        .map(|command| command.id)
        .collect::<BTreeSet<_>>();

    for expected in [
        "swarm_harness_run",
        "handshake_swarm_cli",
        "kernel_swarm_run",
        "swarm_scenario_n8_perf",
        "swarm_scenario_session_cancel",
        "swarm_scenario_lease_contention",
    ] {
        assert!(
            hbr_group.commands.contains(&expected),
            "feature group missing {expected}"
        );
        assert!(
            command_ids.contains(expected),
            "command reference missing {expected}"
        );
    }

    let cli = command(manual, "handshake_swarm_cli");
    assert_eq!(cli.status, CommandStatus::Wired);
    assert_eq!(cli.cli_flag, Some("--scenario"));
    assert!(cli.expected_input.contains("--scenario"));
    assert!(cli.expected_output.contains("SwarmReport"));

    let ipc = command(manual, "kernel_swarm_run");
    assert_eq!(ipc.status, CommandStatus::Wired);
    assert_eq!(ipc.ipc_channel, Some("kernel.swarm.run"));
    assert_eq!(ipc.tauri_command, Some("kernel_swarm_run"));
    assert!(ipc.expected_input.contains("scenario_id"));
    assert!(ipc.expected_output.contains("SwarmReport"));

    for (id, scenario_id) in [
        ("swarm_scenario_n8_perf", "n8-perf"),
        ("swarm_scenario_session_cancel", "session-cancel"),
        ("swarm_scenario_lease_contention", "lease-contention"),
    ] {
        let scenario = command(manual, id);
        assert_eq!(scenario.status, CommandStatus::Wired);
        assert!(scenario.expected_input.contains(scenario_id));
        assert!(scenario.description.contains(scenario_id));
    }
}

fn command(
    manual: &handshake_core::model_manual::Manual,
    id: &str,
) -> handshake_core::model_manual::CommandReference {
    *manual
        .command_reference
        .iter()
        .find(|command| command.id == id)
        .unwrap_or_else(|| panic!("missing command reference {id}"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    })
}
