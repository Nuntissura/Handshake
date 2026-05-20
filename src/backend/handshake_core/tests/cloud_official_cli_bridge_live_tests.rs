//! MT-127 LIVE integration test for the Official CLI bridge runtime.
//!
//! Spec-Realism Gate Sub-rule 2 ("real external-resource touch") is
//! satisfied by spawning an actually-installed CLI binary on the host
//! (claude / codex / gemini), capturing real stdout, asserting a real
//! PID and a real exit code. The test resolves the binary path at
//! run time using the OS PATH probe (`where` on Windows, `which`
//! elsewhere) so it is portable across kernel-builder hosts. If none
//! of the expected binaries is installed, the test fails with a
//! clear environmental error — that is the honest BLOCKED_ON_DEPENDENCY
//! signal the brief asks for, not silent env-gated skipping.
//!
//! The probe uses the `--version` invocation, which exits cleanly,
//! does not hit any vendor API, and produces deterministic stdout
//! containing the CLI name. This pins the live subprocess path
//! without consuming operator API credits.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use handshake_core::model_runtime::cloud::{
    CliBridgeConfig, CliKind, CliOutputFormat, LiveCliSpawner, OfficialCliBridgeRuntime,
};

fn resolve_installed_cli() -> Option<(CliKind, PathBuf, &'static str)> {
    // Try claude, codex, gemini in order. First hit wins.
    for (kind, name) in [
        (CliKind::ClaudeCode, "claude"),
        (CliKind::CodexCli, "codex"),
        (CliKind::GeminiCli, "gemini"),
    ] {
        if let Some(path) = which_probe(name) {
            return Some((kind, path, name));
        }
    }
    None
}

#[cfg(windows)]
fn which_probe(binary: &str) -> Option<PathBuf> {
    let output = Command::new("where").arg(binary).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| PathBuf::from(line.trim()))
}

#[cfg(not(windows))]
fn which_probe(binary: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(binary).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| PathBuf::from(line.trim()))
}

fn version_config(kind: CliKind, exe: PathBuf) -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: kind,
        executable_path: exe,
        // {prompt} is required by register_bridge validation. The
        // live invocation substitutes the prompt body with the
        // literal "--version" flag, so the rendered argv becomes
        // [<binary>, --version]. This is the smallest deterministic
        // live invocation that satisfies Sub-rule 2 without making
        // a vendor API call.
        args_template: vec!["{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

#[test]
fn live_cli_spawner_spawns_real_binary_and_returns_real_pid_and_stdout() {
    let Some((kind, exe, name)) = resolve_installed_cli() else {
        panic!(
            "MT-127 Spec-Realism Gate Sub-rule 2: no installed CLI binary resolved on PATH \
             (tried claude / codex / gemini). MT-127 must be BLOCKED_ON_DEPENDENCY on this host \
             until at least one of the expected CLI binaries is installed."
        );
    };

    let runtime = OfficialCliBridgeRuntime::new(Arc::new(LiveCliSpawner::new()));
    let handle = runtime
        .register_bridge(
            version_config(kind, exe.clone()),
            "version-probe-model",
            "2026-05-20T18:10:00Z",
        )
        .expect("register_bridge against installed CLI");

    let receipt = runtime
        .invoke(handle.model_id, "--version")
        .expect("LiveCliSpawner.invoke returned an error against the installed CLI");

    // PID is populated by the production spawner (mock spawners set
    // it to None or a literal sentinel). A real Some(pid) here proves
    // std::process::Command actually launched a child process.
    let pid = receipt
        .pid
        .unwrap_or_else(|| panic!("LiveCliSpawner did not record a PID for binary {name}"));
    assert!(pid > 0, "PID must be > 0 for a real subprocess");

    // Exit code 0 for `--version`.
    assert_eq!(
        receipt.exit_code,
        Some(0),
        "{} --version should exit cleanly; got exit_code={:?}",
        name,
        receipt.exit_code
    );

    // Cancellation flag must be false for a clean run.
    assert!(!receipt.cancelled, "clean --version run must not be cancelled");

    // Stdout must contain the CLI name in some form. Each of the
    // three target CLIs prints a banner that references its name.
    let stdout_lower = receipt.stdout.to_lowercase();
    assert!(
        stdout_lower.contains(name) || stdout_lower.contains("code"),
        "LiveCliSpawner stdout for {} --version must include CLI name; got {:?}",
        name,
        receipt.stdout
    );
    assert!(
        !receipt.stdout.trim().is_empty(),
        "LiveCliSpawner must capture non-empty stdout from real subprocess"
    );
}

#[test]
fn live_cli_spawner_surfaces_spawn_failure_for_missing_binary() {
    // Real-resource negative-path proof: when the configured
    // executable does not exist on the host, LiveCliSpawner must
    // return SpawnFailed (no silent fallback to mock-shaped success).
    // The register_bridge guard catches non-existent paths via
    // ExecutableNotFound; this test exercises the path where the
    // executable existed at register time but is unspawnable.
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().expect("tempdir");
    let bogus = dir.path().join("not-a-real-binary.exe");
    // Create a zero-byte file so register_bridge.executable_path
    // existence check passes, but spawn() will fail because the
    // file is not an executable image.
    fs::write(&bogus, b"").expect("create bogus exe");

    let config = CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: bogus,
        args_template: vec!["{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 5,
    };

    let runtime = OfficialCliBridgeRuntime::new(Arc::new(LiveCliSpawner::new()));
    let handle = runtime
        .register_bridge(config, "bogus-model", "2026-05-20T18:10:00Z")
        .expect("register_bridge passes because the file exists, even though it is not executable");

    let err = runtime
        .invoke(handle.model_id, "anything")
        .expect_err("LiveCliSpawner.invoke must surface a real spawn failure");

    use handshake_core::model_runtime::cloud::OfficialCliBridgeError;
    assert!(
        matches!(err, OfficialCliBridgeError::SpawnFailed { .. }),
        "expected SpawnFailed, got {err:?}"
    );
}
