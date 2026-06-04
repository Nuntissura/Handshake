use std::{
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("handshake_core lives under src/backend/handshake_core")
        .to_path_buf()
}

fn audit_script(repo_root: &Path) -> PathBuf {
    repo_root
        .join(".GOV")
        .join("roles_shared")
        .join("scripts")
        .join("automation-first-audit.mjs")
}

fn run_audit(args: &[&str]) -> std::process::Output {
    let root = repo_root();
    let mut command = Command::new("node");
    command.arg(audit_script(&root));
    command.args(["--repo-root", root.to_str().expect("repo root utf8")]);
    command.args(args);
    command.output().expect("run automation-first audit")
}

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("audit stdout is json")
}

#[test]
fn automation_first_audit_tests_default_certifying_mode_requires_runtime_evidence() {
    let output = run_audit(&["--json"]);
    assert!(
        !output.status.success(),
        "plain automation-first audit must not certify from static source scan:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = stdout_json(&output);

    assert_eq!(report["schema_id"], "hsk.automation_first_audit@1");
    assert_eq!(report["certification_mode"], "runtime_probe_required");
    assert_eq!(report["runtime_probe"]["required"], true);
    assert_eq!(report["runtime_probe"]["evidence_present"], false);
    assert_eq!(report["status"], "FAIL");
    assert!(report["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .any(|violation| violation["code"] == "RUNTIME_PROBE_EVIDENCE_REQUIRED"));
}

#[test]
fn automation_first_audit_tests_static_source_scan_is_explicitly_non_certifying() {
    let output = run_audit(&["--json", "--static-source-scan-ok"]);
    assert!(
        output.status.success(),
        "automation-first audit failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report = stdout_json(&output);

    assert_eq!(report["schema_id"], "hsk.automation_first_audit@1");
    assert_eq!(report["certification_mode"], "static_source_scan_explicit");
    assert_eq!(report["runtime_probe"]["required"], false);
    assert_eq!(
        report["runtime_probe"]["static_source_scan_allowed"],
        true
    );
    assert_eq!(report["status"], "PASS");
    assert_eq!(
        report["foreground_exempt_commands"]
            .as_array()
            .expect("foreground_exempt_commands array")
            .len(),
        0
    );
    assert!(
        report["command_count"].as_u64().unwrap_or_default() >= 20,
        "audit should walk the full Tauri command inventory"
    );

    let commands = report["commands"].as_array().expect("commands array");
    for required in [
        "foreground_warning::foreground_warning_emit",
        "visual_debug::kernel_visual_debug_launch_config",
        "visual_debug::kernel_visual_debug_port",
        "visual_debug::kernel_visual_debug_screenshot",
        "visual_debug::kernel_visual_debug_console_stream_start",
        "visual_debug::kernel_visual_debug_console_stream_stop",
    ] {
        let command = commands
            .iter()
            .find(|entry| entry["command"] == required)
            .unwrap_or_else(|| panic!("missing command {required}"));
        assert_eq!(command["status"], "PASS");
        assert_eq!(command["ipc_callable"], true);
        assert_eq!(command["keyboard_injection_invocation_count"], 0);
        assert_eq!(command["focus_steal_api_count"], 0);
    }
}

#[test]
fn automation_first_audit_tests_synthetic_foreground_api_violation_fails_loudly() {
    let output = run_audit(&[
        "--json",
        "--fixture",
        "synthetic-violation",
        "--static-source-scan-ok",
    ]);
    assert!(!output.status.success(), "synthetic violation must fail");
    let report = stdout_json(&output);

    assert_eq!(report["status"], "FAIL");
    assert!(report["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .any(
            |violation| violation["code"] == "FORBIDDEN_OS_FOREGROUND_API"
                && violation["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("SetForegroundWindow")
        ));
}

#[test]
fn automation_first_audit_tests_report_is_on_demand_markdown_projection() {
    let temp = tempfile::tempdir().expect("temp dir");
    let out = temp.path().join("audit-report.md");
    let output = run_audit(&[
        "--report",
        "--out",
        out.to_str().expect("out utf8"),
        "--static-source-scan-ok",
    ]);
    assert!(
        output.status.success(),
        "report generation failed:\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = std::fs::read_to_string(out).expect("audit report written");
    assert!(report.contains("# Automation-First Audit"));
    assert!(report.contains("status: PASS"));
    assert!(report.contains("certification_mode: static_source_scan_explicit"));
    assert!(report.contains("visual_debug::kernel_visual_debug_screenshot"));
}
