use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use handshake_core::test_harness::{
    default_scenario_registry, run_swarm_scenario, ScenarioRegistry, SessionStep, SwarmScenario,
};

#[tokio::test]
async fn swarm_cli_scenario_runner_executes_builtin_n8_perf_with_requested_session_count() {
    let report = run_swarm_scenario("n8-perf", 2)
        .await
        .expect("run n8-perf scenario");

    assert_eq!(report.scenario_id, "n8-perf");
    assert_eq!(report.n, 2);
    assert_eq!(report.sessions.len(), 2);
    assert_eq!(report.ledger_overflow_count, 0);
    assert!(report
        .sessions
        .iter()
        .all(|session| session.errors.is_empty()));
    assert!(report
        .sessions
        .iter()
        .all(|session| session.steps_completed == 100));
}

#[tokio::test]
async fn swarm_cli_scenario_runner_executes_lease_contention_scenario() {
    let report = run_swarm_scenario("lease-contention", 4)
        .await
        .expect("run lease-contention scenario");

    assert_eq!(report.scenario_id, "lease-contention");
    assert_eq!(report.n, 4);
    assert_eq!(report.sessions.len(), 4);
    assert!(report
        .sessions
        .iter()
        .all(|session| session.errors.is_empty()));
    assert!(report
        .sessions
        .iter()
        .all(|session| session.steps_completed >= 4));
}

#[test]
fn swarm_cli_scenario_registry_exposes_builtin_ids_and_accepts_custom_scenarios() {
    let mut registry: ScenarioRegistry = default_scenario_registry();
    let ids = registry.scenario_ids().into_iter().collect::<BTreeSet<_>>();

    for expected in ["n8-perf", "session-cancel", "lease-contention"] {
        assert!(
            ids.contains(expected),
            "missing builtin scenario {expected}"
        );
    }

    registry
        .register(CustomScenario)
        .expect("register custom scenario");
    let custom = registry
        .scenario("custom-registry-scenario")
        .expect("custom scenario lookup");

    assert_eq!(custom.scenario_id(), "custom-registry-scenario");
    assert_eq!(
        custom.session_steps(3),
        vec![
            SessionStep::OpenWorkspace {
                ws_id: "custom-workspace-3".to_string(),
            },
            SessionStep::CloseSession,
        ]
    );
}

#[test]
fn handshake_swarm_binary_is_declared_without_app_runtime_feature_gate() {
    let manifest = read(repo_root().join("src/backend/handshake_core/Cargo.toml"));

    assert!(manifest.contains("name = \"handshake-swarm\""));
    assert!(manifest.contains("path = \"src/bin/handshake-swarm.rs\""));
    let swarm_bin_start = manifest
        .find("name = \"handshake-swarm\"")
        .expect("handshake-swarm bin section");
    let swarm_bin_section = manifest[swarm_bin_start..]
        .split("\n[[bin]]")
        .next()
        .expect("handshake-swarm bin stanza");
    assert!(
        !swarm_bin_section.contains("required-features"),
        "handshake-swarm must be callable without app-runtime"
    );
}

#[test]
fn handshake_swarm_binary_executes_builtin_scenario_and_writes_report() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_handshake-swarm"));
    let report_path = artifact_root()
        .join("wp-kernel-004-test-runs")
        .join("swarm-cli-tests")
        .join("n8-perf-n2-report.json");
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).expect("create report dir");
    }

    let output = Command::new(&binary)
        .args([
            "--scenario",
            "n8-perf",
            "--n",
            "2",
            "--report",
            report_path.to_str().expect("utf-8 report path"),
        ])
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", binary.display()));

    assert!(
        output.status.success(),
        "handshake-swarm failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout json receipt");
    assert_eq!(stdout["scenario_id"], "n8-perf");
    assert_eq!(stdout["n"], 2);
    assert_eq!(
        stdout["report_path"],
        report_path.to_string_lossy().as_ref()
    );

    let report: serde_json::Value = serde_json::from_str(&read(report_path)).expect("report json");
    assert_eq!(report["scenario_id"], "n8-perf");
    assert_eq!(report["n"], 2);
    assert_eq!(report["sessions"].as_array().expect("sessions").len(), 2);
}

struct CustomScenario;

impl SwarmScenario for CustomScenario {
    fn scenario_id(&self) -> &str {
        "custom-registry-scenario"
    }

    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep> {
        vec![
            SessionStep::OpenWorkspace {
                ws_id: format!("custom-workspace-{session_idx}"),
            },
            SessionStep::CloseSession,
        ]
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn artifact_root() -> PathBuf {
    if let Ok(root) = std::env::var("HANDSHAKE_ARTIFACT_ROOT") {
        return PathBuf::from(root);
    }
    repo_root()
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("Handshake_Artifacts"))
        .unwrap_or_else(|| repo_root().join("Handshake_Artifacts"))
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    })
}
