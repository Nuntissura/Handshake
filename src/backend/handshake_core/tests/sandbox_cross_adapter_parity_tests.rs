//! MT-057 — cross-adapter sandbox parity test surface.
//!
//! Loads the 8 fixture JSONs in `tests/fixtures/cross_adapter_parity/`
//! and drives them through the `CrossAdapterParityHarness` against:
//!   - a pair of always-on `StubAdapter` instances tagged
//!     `stub_alpha` / `stub_beta` so the harness mechanics + parity
//!     assertion run on every CI loop without any external runtime,
//!   - real `Wsl2PodmanAdapter` and `DockerAdapter` instances under
//!     the existing `wsl2-integration` / `docker-integration` Cargo
//!     features, with the canonical runtime-fall-through skip pattern
//!     from `wsl2_podman_adapter_tests.rs` / `docker_adapter_tests.rs`,
//!   - an explicit skip slot for the `windows_native_jail` adapter
//!     (MT-046 pending per `sandbox/bootstrap.rs:39`).
//!
//! Coverage gaps surface as `skipped_adapters` in the `ParityReport`;
//! the validator should read that list to decide whether real-adapter
//! parity has been exercised on this host.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use handshake_core::sandbox::adapter::{
    AdapterCapabilities, GpuPassthrough, IsolationStrength, IsolationTier, SandboxAdapter,
    ThroughputClass,
};
use handshake_core::sandbox::types::{
    AdapterId, BindMode, Command, ExecResult, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapterError, Signal,
};
use handshake_core::sandbox::wsl2_podman::adapter::WSL2_PODMAN_ADAPTER_ID;
use handshake_core::test_harness::cross_adapter::{
    AdapterSlot, CrossAdapterParityHarness, ParityStdoutSource, ScenarioOutcome, StdoutPredicate,
    WorkloadFixture,
};

const FIXTURE_DIR: &str = "tests/fixtures/cross_adapter_parity";
const FIXTURES: [&str; 8] = [
    "trivial_exit.json",
    "stdout_capture.json",
    "bind_readonly.json",
    "net_deny_all.json",
    "net_loopback_only.json",
    "env_passthrough.json",
    "kill_signal_term.json",
    "kill_signal_int.json",
];

const WINDOWS_NATIVE_JAIL_ADAPTER_ID: &str = "windows_native_jail";
const DOCKER_ADAPTER_ID: &str = "docker";

fn load_fixtures() -> Vec<WorkloadFixture> {
    FIXTURES
        .iter()
        .map(|file_name| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(FIXTURE_DIR)
                .join(file_name);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read fixture {path:?}: {error}"));
            serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("parse fixture {path:?}: {error}"))
        })
        .collect()
}

#[tokio::test]
async fn sandbox_cross_adapter_parity_always_runs_on_two_stub_adapters() {
    let fixtures = load_fixtures();
    assert_eq!(
        fixtures.len(),
        FIXTURES.len(),
        "fixture catalog must match the FIXTURES constant"
    );

    let stub_alpha = Arc::new(StubAdapter::new("stub_alpha"));
    let stub_beta = Arc::new(StubAdapter::new("stub_beta"));
    let harness = CrossAdapterParityHarness::new()
        .with_adapter_stdout(
            AdapterId::new("stub_alpha"),
            AdapterSlot::Available(stub_alpha.clone()),
            stub_alpha as Arc<dyn ParityStdoutSource>,
        )
        .with_adapter_stdout(
            AdapterId::new("stub_beta"),
            AdapterSlot::Available(stub_beta.clone()),
            stub_beta as Arc<dyn ParityStdoutSource>,
        );

    let report = harness.run_fixtures(&fixtures).await;
    assert_eq!(
        report.ran_count(),
        fixtures.len() * 2,
        "both stub adapters must run every fixture; got {} rans + {} skips",
        report.ran_count(),
        report.skipped_count()
    );
    assert!(
        report.skipped_adapters.is_empty(),
        "no skips expected on the always-on stub-adapter path; got {:?}",
        report.skipped_adapters
    );

    // The stub adapter responds deterministically per fixture (see
    // StubAdapter::scripted_exit_code below), so parity holds across
    // both stub instances by construction. The assertion verifies the
    // harness comparator wiring rather than adapter behavior.
    for fixture in &fixtures {
        harness_parity_assert(&report, &fixture.name);
    }

    // Every assertion (including the now-real stdout predicate) must pass
    // on the matching stdout path. This is the genuine-pass half of the
    // no-auto-pass proof: the stdout_capture fixture declares a substring
    // predicate, the stub emits the matching bytes, and the harness
    // evaluates it for real.
    for run in report.ran() {
        assert!(
            run.assertions.is_ok(),
            "fixture {} on {} should pass all assertions; failures={:?}",
            run.fixture_name,
            run.adapter_id.as_str(),
            run.assertions.failures
        );
    }

    // Pin the real-capture path explicitly for the stdout_capture row:
    // the harness must have captured the genuine bytes (not None) and the
    // stdout predicate dimension must have evaluated to passed.
    let stdout_row = report
        .ran()
        .find(|run| run.fixture_name == "stdout_capture")
        .expect("stdout_capture must have run on the stub adapters");
    assert_eq!(
        stdout_row.stdout.as_deref(),
        Some(b"handshake-mt057-stdout\n".as_slice()),
        "stdout_capture must record the real captured bytes, not None"
    );
    assert!(
        stdout_row.assertions.stdout_predicate_passed,
        "stdout predicate must genuinely pass against matching captured bytes"
    );
}

#[tokio::test]
async fn sandbox_cross_adapter_parity_stdout_predicate_fails_on_mismatch() {
    // No-auto-pass proof, failing half: an adapter that emits the WRONG
    // stdout bytes must make the stdout predicate FAIL. If the harness
    // still auto-passed the predicate (the MT-057 bug), this assertion
    // would not hold.
    let fixtures = load_fixtures();
    let liar = Arc::new(StubAdapter::new_with_corrupt_stdout("stub_liar"));
    let harness = CrossAdapterParityHarness::new().with_adapter_stdout(
        AdapterId::new("stub_liar"),
        AdapterSlot::Available(liar.clone()),
        liar as Arc<dyn ParityStdoutSource>,
    );
    let report = harness.run_fixtures(&fixtures).await;

    let stdout_row = report
        .ran()
        .find(|run| run.fixture_name == "stdout_capture")
        .expect("stdout_capture must have run");
    assert!(
        !stdout_row.assertions.stdout_predicate_passed,
        "mismatched stdout must FAIL the predicate (no auto-pass); captured={:?}",
        stdout_row.stdout.as_deref().map(String::from_utf8_lossy)
    );
    assert!(
        !stdout_row.assertions.is_ok(),
        "a failed stdout predicate must surface in the assertions failure list"
    );
    assert!(
        stdout_row
            .assertions
            .failures
            .iter()
            .any(|f| f.contains("stdout predicate")),
        "failure message must name the stdout predicate; got {:?}",
        stdout_row.assertions.failures
    );
}

#[tokio::test]
async fn sandbox_cross_adapter_parity_stdout_predicate_fails_when_uncaptured() {
    // Honesty guard: when an adapter has NO registered stdout source, a
    // declared stdout predicate must be reported as a verification
    // failure (not silently green). This is the path real podman/docker
    // adapters take today (no spawn-stdout side channel wired), and it
    // must surface as a loud coverage gap rather than a false pass.
    let fixtures = load_fixtures();
    let harness = CrossAdapterParityHarness::new().with_adapter(
        AdapterId::new("stub_no_stdout"),
        AdapterSlot::Available(Arc::new(StubAdapter::new("stub_no_stdout"))),
    );
    let report = harness.run_fixtures(&fixtures).await;

    let stdout_row = report
        .ran()
        .find(|run| run.fixture_name == "stdout_capture")
        .expect("stdout_capture must have run");
    assert!(stdout_row.stdout.is_none(), "no source => stdout is None");
    assert!(
        !stdout_row.assertions.stdout_predicate_passed,
        "uncaptured stdout against a declared predicate must NOT auto-pass"
    );

    // Fixtures that declare no stdout predicate (e.g. trivial_exit) must
    // still pass the stdout dimension — there is nothing to verify.
    let trivial_row = report
        .ran()
        .find(|run| run.fixture_name == "trivial_exit")
        .expect("trivial_exit must have run");
    assert!(
        trivial_row.assertions.stdout_predicate_passed,
        "a fixture with no stdout predicate must pass the stdout dimension even without capture"
    );

    // Cross-check the StdoutPredicate matcher directly.
    assert!(StdoutPredicate::Substring("abc".to_string()).matches(b"xxabcxx"));
    assert!(!StdoutPredicate::Substring("abc".to_string()).matches(b"xyz"));
    assert!(StdoutPredicate::ExactBytes("hi".to_string()).matches(b"hi\n"));
    assert!(!StdoutPredicate::ExactBytes("hi".to_string()).matches(b"hiya"));
    assert!(StdoutPredicate::Empty.matches(b""));
    assert!(!StdoutPredicate::Empty.matches(b"x"));
}

#[tokio::test]
async fn sandbox_cross_adapter_parity_records_skip_when_only_one_adapter_can_run() {
    // Adversarial scenario: a matrix that registers one Available
    // adapter and one Unavailable slot. Parity is necessarily
    // un-exercised (single-adapter), but the harness must NOT silently
    // pass — it must record the unavailable adapter in skipped_adapters
    // and emit Skipped rows for every fixture against it.
    let fixtures = load_fixtures();
    let harness = CrossAdapterParityHarness::new()
        .with_adapter(
            AdapterId::new("stub_alpha"),
            AdapterSlot::Available(Arc::new(StubAdapter::new("stub_alpha"))),
        )
        .with_adapter(
            AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
            AdapterSlot::Unavailable {
                reason: "MT-046 WindowsNativeJailAdapter not implemented".to_string(),
            },
        );

    let report = harness.run_fixtures(&fixtures).await;
    assert_eq!(
        report.ran_count(),
        fixtures.len(),
        "stub_alpha must run all fixtures"
    );
    assert_eq!(
        report.skipped_count(),
        fixtures.len(),
        "windows_native_jail must produce one Skipped row per fixture"
    );
    assert!(
        report
            .skipped_adapters
            .contains(&AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID)),
        "skipped_adapters must surface the coverage gap explicitly"
    );
    for fixture in &fixtures {
        assert!(
            !report.has_parity_coverage_for(&fixture.name),
            "single-adapter scenario must NOT report parity coverage for {}",
            fixture.name
        );
    }
}

#[tokio::test]
async fn sandbox_cross_adapter_parity_detects_disagreement_between_adapters() {
    // Adversarial: when two adapters disagree on exit_code for the
    // same fixture, the harness's parity assertion MUST surface that
    // failure rather than reporting Ok.
    let fixtures = load_fixtures();
    let harness = CrossAdapterParityHarness::new()
        .with_adapter(
            AdapterId::new("stub_alpha"),
            AdapterSlot::Available(Arc::new(StubAdapter::new("stub_alpha"))),
        )
        .with_adapter(
            AdapterId::new("stub_beta"),
            AdapterSlot::Available(Arc::new(StubAdapter::new_with_exit_offset("stub_beta", 1))),
        );
    let report = harness.run_fixtures(&fixtures).await;

    // Find at least one fixture where the two adapters' exit codes
    // disagree, then prove `assert_parity_for` returns Err.
    let mut found_disagreement = false;
    for fixture in &fixtures {
        if report.assert_parity_for(&fixture.name).is_err() {
            found_disagreement = true;
            break;
        }
    }
    assert!(
        found_disagreement,
        "adversarial: shifted exit codes on stub_beta must trigger a parity failure on at least one fixture"
    );
}

#[tokio::test]
#[cfg_attr(not(feature = "wsl2-integration"), ignore)]
async fn sandbox_cross_adapter_parity_env_gated_wsl2_podman_real_runtime() {
    use handshake_core::sandbox::wsl2_podman::adapter::{Wsl2PodmanAdapter, Wsl2PodmanConfig};

    let adapter_slot =
        match Wsl2PodmanAdapter::try_new(Wsl2PodmanConfig::for_distro("Ubuntu")).await {
            Ok(adapter) => AdapterSlot::Available(Arc::new(adapter)),
            Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
            | Err(SandboxAdapterError::SpawnFailed { reason, .. })
                if reason.contains("podman unavailable")
                    || reason.contains("not registered")
                    || reason.contains("WSL") =>
            {
                AdapterSlot::Unavailable { reason }
            }
            Err(error) => panic!("WSL2Podman integration setup failed unexpectedly: {error:?}"),
        };

    let fixtures = load_fixtures();
    let harness = CrossAdapterParityHarness::new()
        .with_adapter(AdapterId::new(WSL2_PODMAN_ADAPTER_ID), adapter_slot);
    let report = harness.run_fixtures(&fixtures).await;

    // Real-adapter coverage is informational on a per-host basis; the
    // assertion is that the harness ran without panic and that any
    // failures are reported as ScenarioOutcome::Skipped with a reason
    // rather than silent passes.
    for outcome in &report.rows {
        match outcome {
            ScenarioOutcome::Ran(_) => {}
            ScenarioOutcome::Skipped { reason } => {
                eprintln!("[wsl2_podman cross-adapter parity skipped]: {reason}");
            }
        }
    }
}

#[tokio::test]
#[cfg_attr(not(feature = "docker-integration"), ignore)]
async fn sandbox_cross_adapter_parity_env_gated_docker_real_runtime() {
    use handshake_core::sandbox::docker::adapter::{DockerAdapter, DockerConfig};

    let adapter_slot = match DockerAdapter::try_new(DockerConfig::new("docker")).await {
        Ok(adapter) => AdapterSlot::Available(Arc::new(adapter)),
        Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
        | Err(SandboxAdapterError::SpawnFailed { reason, .. })
            if reason.contains("docker unavailable")
                || reason.contains("failed to spawn")
                || reason.contains("Docker daemon") =>
        {
            AdapterSlot::Unavailable { reason }
        }
        Err(error) => panic!("Docker integration setup failed unexpectedly: {error:?}"),
    };

    let fixtures = load_fixtures();
    let harness = CrossAdapterParityHarness::new()
        .with_adapter(AdapterId::new(DOCKER_ADAPTER_ID), adapter_slot);
    let report = harness.run_fixtures(&fixtures).await;

    for outcome in &report.rows {
        match outcome {
            ScenarioOutcome::Ran(_) => {}
            ScenarioOutcome::Skipped { reason } => {
                eprintln!("[docker cross-adapter parity skipped]: {reason}");
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Helpers.
// ----------------------------------------------------------------------------

fn harness_parity_assert(
    report: &handshake_core::test_harness::cross_adapter::ParityReport,
    fixture_name: &str,
) {
    // The two stub adapters above produce identical exits, so parity
    // must hold for every fixture. Use the public parity assertion
    // helper so the test exercises the same path the validator would.
    report
        .assert_parity_for(fixture_name)
        .unwrap_or_else(|error| panic!("parity failure on fixture {fixture_name}: {error}"));
}

/// Per-fixture deterministic stub adapter. Returns canned
/// `ProcessStatus::Exited` codes so the parity assertion has stable
/// inputs without any external runtime.
struct StubAdapter {
    adapter_id: String,
    /// Exit-code offset added to the fixture-scripted code, used by the
    /// adversarial disagreement test to force divergence across two
    /// otherwise-identical stubs.
    exit_offset: i32,
    /// When true, the stub emits deliberately wrong stdout bytes so the
    /// stdout predicate genuinely FAILS — proving the predicate is not an
    /// auto-pass. Used only by the adversarial stdout-mismatch test.
    corrupt_stdout: bool,
    state: Mutex<StubState>,
}

#[derive(Default)]
struct StubState {
    /// Maps process_uuid -> (scripted_exit_code, killed_flag).
    by_handle: std::collections::HashMap<uuid::Uuid, ProcessState>,
}

#[derive(Clone, Debug)]
struct ProcessState {
    scripted_exit_code: i32,
    killed: bool,
    /// Real stdout bytes the stub "process" produced, computed at spawn
    /// from the workload command (see StubAdapter::scripted_stdout). The
    /// harness reads these back through the ParityStdoutSource channel so
    /// the stdout predicate is evaluated against genuine bytes and can
    /// fail on mismatch.
    stdout: Vec<u8>,
}

impl StubAdapter {
    fn new(adapter_id: &str) -> Self {
        Self {
            adapter_id: adapter_id.to_string(),
            exit_offset: 0,
            corrupt_stdout: false,
            state: Mutex::new(StubState::default()),
        }
    }

    fn new_with_exit_offset(adapter_id: &str, offset: i32) -> Self {
        Self {
            adapter_id: adapter_id.to_string(),
            exit_offset: offset,
            corrupt_stdout: false,
            state: Mutex::new(StubState::default()),
        }
    }

    /// A stub that emits wrong stdout, used to prove the stdout predicate
    /// can fail (no auto-pass).
    fn new_with_corrupt_stdout(adapter_id: &str) -> Self {
        Self {
            adapter_id: adapter_id.to_string(),
            exit_offset: 0,
            corrupt_stdout: true,
            state: Mutex::new(StubState::default()),
        }
    }

    /// Deterministic stdout bytes the stub "process" writes for a given
    /// workload. Mirrors what a real `echo`/`sh -c` would emit so the
    /// stdout predicate is exercised against genuine output. Commands
    /// other than the stdout-capture echo produce empty stdout, matching
    /// the fixtures (none of which declare a non-empty stdout predicate).
    fn scripted_stdout(spec: &ProcessSpec) -> Vec<u8> {
        let joined = spec.cmd.join(" ");
        if joined.contains("echo handshake-mt057-stdout") {
            return b"handshake-mt057-stdout\n".to_vec();
        }
        Vec::new()
    }

    fn scripted_exit_code(spec: &ProcessSpec) -> i32 {
        // Each fixture's cmd[*] strings drive a deterministic exit
        // code so two stub instances agree by construction. The
        // disagreement test adds an offset to force divergence.
        let joined = spec.cmd.join(" ");
        if joined.contains("touch /readonly_bind") {
            return 13;
        }
        if joined.contains("getent hosts handshake.invalid") {
            return 1;
        }
        if joined.contains("HANDSHAKE_MT057") {
            // env_passthrough: simulate env actually present.
            return 0;
        }
        if joined.contains("nc -l -p 47057") {
            return 0;
        }
        if joined.contains("trap 'exit 42' INT") {
            return 42;
        }
        if spec.cmd.first().map(|s| s.as_str()) == Some("sleep") {
            // kill_signal_term: caller will kill, status becomes killed
            return 143;
        }
        // trivial_exit and stdout_capture
        0
    }
}

#[async_trait]
impl SandboxAdapter for StubAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        let scripted = Self::scripted_exit_code(&spec) + self.exit_offset;
        let mut stdout = Self::scripted_stdout(&spec);
        if self.corrupt_stdout && !stdout.is_empty() {
            // Deliberately diverge from what the workload would emit so
            // the stdout predicate has something real to reject.
            stdout = b"corrupted-stdout-does-not-match\n".to_vec();
        }
        let handle = ProcessHandle::new(
            AdapterId::new(&self.adapter_id),
            Some(99_999),
            format!("{}-{}", self.adapter_id, uuid::Uuid::now_v7()),
        );
        self.state.lock().unwrap().by_handle.insert(
            handle.id,
            ProcessState {
                scripted_exit_code: scripted,
                killed: false,
                stdout,
            },
        );
        Ok(handle)
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(&self.adapter_id),
            reason: "stub adapter does not support exec".to_string(),
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
        handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        if let Some(state) = self.state.lock().unwrap().by_handle.get_mut(&handle.id) {
            state.killed = true;
        }
        Ok(())
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        let state = self
            .state
            .lock()
            .unwrap()
            .by_handle
            .get(&handle.id)
            .cloned()
            .ok_or_else(|| SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new(&self.adapter_id),
                reason: format!("unknown handle {}", handle.id),
            })?;
        if state.killed {
            Ok(ProcessStatus::Killed {
                by_signal: Signal::Term,
            })
        } else {
            Ok(ProcessStatus::Exited {
                code: state.scripted_exit_code,
            })
        }
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        let state = self
            .state
            .lock()
            .unwrap()
            .by_handle
            .get(&handle.id)
            .cloned()
            .ok_or_else(|| SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new(&self.adapter_id),
                reason: format!("unknown handle {}", handle.id),
            })?;
        Ok(Some(state.scripted_exit_code))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: AdapterId::new(&self.adapter_id),
            runtime_available: true,
            filesystem_isolation_strength: IsolationStrength::Strong,
            network_isolation_strength: IsolationStrength::Strong,
            gpu_passthrough: GpuPassthrough::None,
            stdio_throughput_class: ThroughputClass::Medium,
            win32_native_fidelity: false,
            cross_machine_portable: false,
            isolation_tier: IsolationTier::Tier1Container,
            requires_nested_virt: false,
            supports_snapshot: false,
            supports_persistent_exec: false,
            supports_warm_agent: false,
            supports_live_token_stream: false,
        }
    }
}

#[async_trait]
impl ParityStdoutSource for StubAdapter {
    async fn parity_stdout(
        &self,
        handle: &ProcessHandle,
        _fixture: &WorkloadFixture,
    ) -> Option<Vec<u8>> {
        self.state
            .lock()
            .unwrap()
            .by_handle
            .get(&handle.id)
            .map(|state| state.stdout.clone())
    }
}
