//! MT-058 — sandbox escape negative-test driver.
//!
//! Drives the escape catalog defined in
//! `handshake_core::test_harness::escape_attempts::escape_catalog()` against
//! every adapter in the matrix and persists the report under the operator's
//! artifact root for Integration Validator review.
//!
//! Always-on tests verify harness mechanics (catalog completeness, JSON
//! shape, RED detection, OS-restriction skip). Env-gated tests under
//! `wsl2-integration` / `docker-integration` / `win-native-integration`
//! features exercise real adapter implementations; missing adapters skip
//! gracefully (skip-discipline matches MT-057 + the canonical pattern in
//! `wsl2_podman_adapter_tests.rs` / `docker_adapter_tests.rs`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use handshake_core::sandbox::adapter::{
    AdapterCapabilities, GpuPassthrough, IsolationStrength, SandboxAdapter, ThroughputClass,
};
use handshake_core::sandbox::types::{
    AdapterId, BindMode, Command, ExecResult, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapterError, Signal,
};
use handshake_core::sandbox::wsl2_podman::adapter::WSL2_PODMAN_ADAPTER_ID;
use handshake_core::test_harness::escape_attempts::{
    escape_catalog, EscapeAdapterSlot, EscapeAttemptId, EscapeVerdict, SandboxEscapeHarness,
};

const DOCKER_ADAPTER_ID: &str = "docker";
const WINDOWS_NATIVE_JAIL_ADAPTER_ID: &str = "windows_native_jail";

fn target_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    }
}

fn artifact_root() -> PathBuf {
    PathBuf::from("D:/Projects/LLM projects/Handshake/Handshake_Artifacts/sandbox-escape-results")
}

#[test]
fn escape_catalog_covers_all_ten_canonical_attempts() {
    let catalog = escape_catalog();
    assert_eq!(
        catalog.len(),
        10,
        "MT-058 contract pins the catalog at exactly 10 entries"
    );
    let ids: Vec<EscapeAttemptId> = catalog.iter().map(|a| a.id).collect();
    let expected = EscapeAttemptId::all();
    for id in expected {
        assert!(
            ids.contains(&id),
            "escape catalog missing canonical entry {}",
            id.as_str()
        );
    }
}

#[tokio::test]
async fn escape_harness_always_runs_against_a_stub_adapter_and_emits_green_per_attempt() {
    // Stub adapter sandboxes nothing but reports the "escape was denied"
    // exit code for every attempt — that's the canonical "passing safe
    // adapter" path. The harness must therefore record Green for every
    // attempt it can run (skipping OS-restricted attempts).
    let stub = Arc::new(StubAdapter::new("stub_safe_adapter", DenyMode::AlwaysDeny));
    let harness = SandboxEscapeHarness::new(target_os()).with_adapter(
        AdapterId::new("stub_safe_adapter"),
        EscapeAdapterSlot::Available(stub),
    );
    let catalog = escape_catalog();
    let report = harness.run(&catalog).await;

    // Each attempt is either Green (matching OS) or Skipped (OS-restricted).
    assert!(
        !report.has_any_red(),
        "stub safe adapter must never trip a Red verdict; got {:?}",
        report.red_attempts().collect::<Vec<_>>()
    );
    assert!(
        report.green_count() > 0,
        "at least one attempt must run Green on the current OS"
    );
    let path = report
        .persist_to_artifacts(&artifact_root())
        .expect("persist results JSON");
    assert!(path.exists(), "persisted report must exist at {path:?}");
}

#[tokio::test]
async fn escape_harness_records_red_when_adapter_allows_the_escape() {
    // Adversarial: a stub adapter that DOES allow the escape (reports
    // exit code 0 where the catalog expects non-zero, or vice versa)
    // MUST trip a Red verdict and surface via has_any_red().
    let leaky = Arc::new(StubAdapter::new(
        "stub_leaky_adapter",
        DenyMode::AlwaysAllow,
    ));
    let harness = SandboxEscapeHarness::new(target_os()).with_adapter(
        AdapterId::new("stub_leaky_adapter"),
        EscapeAdapterSlot::Available(leaky),
    );
    let catalog = escape_catalog();
    let report = harness.run(&catalog).await;

    assert!(
        report.has_any_red(),
        "leaky stub must trip Red on at least one attempt; got rows {:?}",
        report.rows
    );
}

#[tokio::test]
async fn escape_harness_records_skip_when_adapter_is_unavailable() {
    // MT-046 WindowsNativeJailAdapter is unimplemented; the harness
    // must mark it as Skipped per attempt, NOT silently pass.
    let harness = SandboxEscapeHarness::new(target_os()).with_adapter(
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        EscapeAdapterSlot::Unavailable {
            reason: "MT-046 WindowsNativeJailAdapter not implemented".to_string(),
        },
    );
    let catalog = escape_catalog();
    let report = harness.run(&catalog).await;

    assert_eq!(report.skipped_count(), catalog.len());
    assert_eq!(report.green_count(), 0);
    assert_eq!(report.yellow_count(), 0);
    assert!(report
        .skipped_adapters
        .iter()
        .any(|id| id == WINDOWS_NATIVE_JAIL_ADAPTER_ID));
}

#[tokio::test]
async fn escape_harness_skips_os_restricted_attempts_on_wrong_target_os() {
    // Win32 foreground-inject attempt is windows-only. On a non-windows
    // target_os, the harness must mark it Skipped without trying to
    // dispatch.
    let stub = Arc::new(StubAdapter::new("stub_safe_adapter", DenyMode::AlwaysDeny));
    let harness = SandboxEscapeHarness::new("linux").with_adapter(
        AdapterId::new("stub_safe_adapter"),
        EscapeAdapterSlot::Available(stub),
    );
    let catalog = escape_catalog();
    let report = harness.run(&catalog).await;
    let win32_row = report
        .rows
        .iter()
        .find(|row| row.attempt_id == EscapeAttemptId::EscWin32ForegroundInject.as_str())
        .expect("ESC-WIN32-FOREGROUND-INJECT row must exist");
    assert!(
        matches!(win32_row.verdict, EscapeVerdict::Skipped { .. }),
        "Win32 attempt must be skipped on linux target_os; got {:?}",
        win32_row.verdict
    );
}

#[tokio::test]
#[cfg_attr(not(feature = "wsl2-integration"), ignore)]
async fn escape_harness_env_gated_wsl2_podman_real_runtime() {
    use handshake_core::sandbox::wsl2_podman::adapter::{Wsl2PodmanAdapter, Wsl2PodmanConfig};

    let adapter_slot =
        match Wsl2PodmanAdapter::try_new(Wsl2PodmanConfig::for_distro("Ubuntu")).await {
            Ok(adapter) => EscapeAdapterSlot::Available(Arc::new(adapter)),
            Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
            | Err(SandboxAdapterError::SpawnFailed { reason, .. })
                if reason.contains("podman unavailable")
                    || reason.contains("not registered")
                    || reason.contains("WSL") =>
            {
                EscapeAdapterSlot::Unavailable { reason }
            }
            Err(error) => panic!("WSL2Podman integration setup failed unexpectedly: {error:?}"),
        };

    let harness = SandboxEscapeHarness::new(target_os())
        .with_adapter(AdapterId::new(WSL2_PODMAN_ADAPTER_ID), adapter_slot);
    let catalog = escape_catalog();
    let report = harness.run(&catalog).await;
    let path = report
        .persist_to_artifacts(&artifact_root())
        .expect("persist results JSON");
    eprintln!(
        "[wsl2_podman escape report] {} rows, written to {path:?}",
        report.rows.len()
    );

    // RED on any attempt blocks the WP integration — make that
    // failure surface as a panic so cargo test exits non-zero.
    if report.has_any_red() {
        for red in report.red_attempts() {
            eprintln!(
                "RED: {} on {}: exit_code={:?}",
                red.attempt_id, red.adapter_id, red.exit_code
            );
        }
        panic!("escape negative-test suite recorded RED verdicts under wsl2_podman");
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "docker-integration"), ignore)]
async fn escape_harness_env_gated_docker_real_runtime() {
    use handshake_core::sandbox::docker::adapter::{DockerAdapter, DockerConfig};

    let adapter_slot = match DockerAdapter::try_new(DockerConfig::new("docker")).await {
        Ok(adapter) => EscapeAdapterSlot::Available(Arc::new(adapter)),
        Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
        | Err(SandboxAdapterError::SpawnFailed { reason, .. })
            if reason.contains("docker unavailable")
                || reason.contains("failed to spawn")
                || reason.contains("Docker daemon") =>
        {
            EscapeAdapterSlot::Unavailable { reason }
        }
        Err(error) => panic!("Docker integration setup failed unexpectedly: {error:?}"),
    };

    let harness = SandboxEscapeHarness::new(target_os())
        .with_adapter(AdapterId::new(DOCKER_ADAPTER_ID), adapter_slot);
    let catalog = escape_catalog();
    let report = harness.run(&catalog).await;
    let path = report
        .persist_to_artifacts(&artifact_root())
        .expect("persist results JSON");
    eprintln!(
        "[docker escape report] {} rows, written to {path:?}",
        report.rows.len()
    );

    if report.has_any_red() {
        for red in report.red_attempts() {
            eprintln!(
                "RED: {} on {}: exit_code={:?}",
                red.attempt_id, red.adapter_id, red.exit_code
            );
        }
        panic!("escape negative-test suite recorded RED verdicts under docker");
    }
}

// ----------------------------------------------------------------------------
// Stub adapter — fakes per-attempt "deny" or "allow" behavior so the
// harness mechanics can be tested without any real sandbox runtime.
// ----------------------------------------------------------------------------

#[derive(Copy, Clone)]
enum DenyMode {
    AlwaysDeny,
    AlwaysAllow,
}

struct StubAdapter {
    adapter_id: String,
    mode: DenyMode,
    state: Mutex<HashMap<uuid::Uuid, i32>>,
}

impl StubAdapter {
    fn new(adapter_id: &str, mode: DenyMode) -> Self {
        Self {
            adapter_id: adapter_id.to_string(),
            mode,
            state: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl SandboxAdapter for StubAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        // Heuristic: if the script ends with `exit 0 || exit 1` and
        // the body fails (which it would inside a real sandbox), the
        // catalog expects exit 1 = denied. For AlwaysDeny we report
        // the catalog's expected-deny exit_code. For AlwaysAllow we
        // invert it.
        let joined = spec.cmd.join(" ");
        let deny_exit_code = if joined.contains("[ \"$u\" != \"0\" ]")
            || joined.contains("Name:[[:space:]]+(pause")
            || joined.contains("(sda|nvme0n1|vda|xvda)")
        {
            // These attempts expect Green when the script exits 0
            // (denial encoded in the script body).
            0
        } else {
            1
        };
        let allow_exit_code = if deny_exit_code == 0 { 1 } else { 0 };
        let exit = match self.mode {
            DenyMode::AlwaysDeny => deny_exit_code,
            DenyMode::AlwaysAllow => allow_exit_code,
        };
        let handle = ProcessHandle::new(
            AdapterId::new(&self.adapter_id),
            Some(99_999),
            format!("{}-{}", self.adapter_id, uuid::Uuid::now_v7()),
        );
        self.state.lock().unwrap().insert(handle.id, exit);
        Ok(handle)
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(&self.adapter_id),
            reason: "stub adapter has no exec".to_string(),
        })
    }

    async fn fs_bind(
        &self,
        _handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        Ok(())
    }

    async fn net_policy(
        &self,
        _handle: &ProcessHandle,
        _policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        Ok(())
    }

    async fn kill(
        &self,
        _handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        Ok(())
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        let exit = self
            .state
            .lock()
            .unwrap()
            .get(&handle.id)
            .copied()
            .ok_or_else(|| SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new(&self.adapter_id),
                reason: format!("unknown handle {}", handle.id),
            })?;
        Ok(ProcessStatus::Exited { code: exit })
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        let exit = self
            .state
            .lock()
            .unwrap()
            .get(&handle.id)
            .copied()
            .ok_or_else(|| SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new(&self.adapter_id),
                reason: format!("unknown handle {}", handle.id),
            })?;
        Ok(Some(exit))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: AdapterId::new(&self.adapter_id),
            filesystem_isolation_strength: IsolationStrength::Strong,
            network_isolation_strength: IsolationStrength::Strong,
            gpu_passthrough: GpuPassthrough::None,
            stdio_throughput_class: ThroughputClass::Medium,
            win32_native_fidelity: false,
            cross_machine_portable: false,
        }
    }
}
