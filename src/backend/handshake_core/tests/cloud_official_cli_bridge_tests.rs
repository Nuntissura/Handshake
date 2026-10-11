//! MT-127 cross-crate integration smoke for the Official CLI bridge
//! runtime scaffold. Exhaustive coverage lives in the inline tests
//! in `model_runtime::cloud::official_cli_bridge::tests`; this file
//! pins the cross-crate API surface + the red_team minimum_controls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use handshake_core::model_runtime::cloud::{
    validate_cli_executable_path, CliBridgeConfig, CliCancellationContext, CliInvocationContext,
    CliInvocationReceipt, CliKind, CliOutputFormat, CliSubprocessSpawner, LiveCliSpawner,
    OfficialCliBridgeError, OfficialCliBridgeRuntime,
};
use handshake_core::model_runtime::ModelId;
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerEvent, NoopOverflowSink, ProcessEngineKind,
    ProcessLedgerError, ProcessLedgerStore,
};
use handshake_core::sandbox::{
    AdapterCapabilities, AdapterId, AttachedNetworkMode, AttachedProcessSpec,
    AttachedSandboxProcess, AttachedStdioContract, BindMode, Command as SandboxCommand, ExecResult,
    HandshakeNativeSandboxAdapter, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapter, SandboxAdapterError, SandboxAdapterRegistry, Signal,
};

struct EchoSpawner {
    cancel_reported: Mutex<bool>,
}
impl CliSubprocessSpawner for EchoSpawner {
    fn spawn(
        &self,
        _config: &CliBridgeConfig,
        _invocation: &CliInvocationContext,
        model_name: &str,
        prompt: &str,
    ) -> Result<CliInvocationReceipt, OfficialCliBridgeError> {
        Ok(CliInvocationReceipt {
            model_id: ModelId::new_v7(),
            stdout: format!("echo model={model_name} prompt={prompt}"),
            pid: Some(42),
            exit_code: Some(0),
            cancelled: *self.cancel_reported.lock().unwrap(),
        })
    }
}

struct ReleaseOnlyPipeReader {
    emitted: bool,
    blocked: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
    finished: mpsc::Sender<()>,
}

impl std::io::Read for ReleaseOnlyPipeReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if !self.emitted {
            self.emitted = true;
            buffer[0] = b'x';
            return Ok(1);
        }
        let _ = self.blocked.send(());
        let _ = self.release.recv();
        Ok(0)
    }
}

impl Drop for ReleaseOnlyPipeReader {
    fn drop(&mut self) {
        let _ = self.finished.send(());
    }
}

struct ReapFailingAttachedProcess {
    pid: u32,
    stdout: Option<Box<dyn std::io::Read + Send>>,
    reader_blocked: mpsc::Receiver<()>,
    reader_blocked_seen: bool,
}

impl AttachedSandboxProcess for ReapFailingAttachedProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        self.stdout.take()
    }

    fn take_stderr(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        None
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>, SandboxAdapterError> {
        Ok(None)
    }

    fn wait(&mut self) -> Result<ExitStatus, SandboxAdapterError> {
        Err(reap_failure())
    }

    fn terminate_tree_and_wait(&mut self) -> Result<ExitStatus, SandboxAdapterError> {
        if !self.reader_blocked_seen {
            self.reader_blocked
                .recv_timeout(Duration::from_secs(1))
                .expect("stdout reader must be blocked before termination failure returns");
            self.reader_blocked_seen = true;
        }
        Err(reap_failure())
    }
}

struct ReapFailingSandboxAdapter {
    capabilities: AdapterCapabilities,
    release: Mutex<Option<mpsc::Sender<()>>>,
    finished: Mutex<Option<mpsc::Receiver<()>>>,
}

impl ReapFailingSandboxAdapter {
    fn new() -> Self {
        Self {
            capabilities: HandshakeNativeSandboxAdapter::new().capabilities(),
            release: Mutex::new(None),
            finished: Mutex::new(None),
        }
    }

    fn unavailable(&self) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: self.capabilities.adapter_id.clone(),
            reason: "reap-failure fixture only supports attached spawn".to_string(),
        }
    }

    fn release_reader_and_wait(&self) {
        self.release
            .lock()
            .unwrap()
            .take()
            .expect("fixture release sender")
            .send(())
            .expect("release blocked fixture reader");
        self.finished
            .lock()
            .unwrap()
            .take()
            .expect("fixture finished receiver")
            .recv_timeout(Duration::from_secs(1))
            .expect("released fixture reader must exit");
    }
}

fn reap_failure() -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new("handshake_native"),
        reason: "intentional terminate/reap failure".to_string(),
    }
}

#[async_trait]
impl SandboxAdapter for ReapFailingSandboxAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn spawn_attached(
        &self,
        _spec: AttachedProcessSpec,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        self.spawn_attached_with_stdio(_spec, AttachedStdioContract::null_stdin_piped_output())
            .await
    }

    async fn spawn_attached_with_stdio(
        &self,
        _spec: AttachedProcessSpec,
        _stdio: AttachedStdioContract,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        let (blocked_tx, blocked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        *self.release.lock().unwrap() = Some(release_tx);
        *self.finished.lock().unwrap() = Some(finished_rx);
        Ok(Box::new(ReapFailingAttachedProcess {
            pid: 9093,
            stdout: Some(Box::new(ReleaseOnlyPipeReader {
                emitted: false,
                blocked: blocked_tx,
                release: release_rx,
                finished: finished_tx,
            })),
            reader_blocked: blocked_rx,
            reader_blocked_seen: false,
        }))
    }

    fn validate_attached_network_mode(
        &self,
        _mode: AttachedNetworkMode,
    ) -> Result<(), SandboxAdapterError> {
        Ok(())
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: SandboxCommand,
    ) -> Result<ExecResult, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn fs_bind(
        &self,
        _handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn net_policy(
        &self,
        _handle: &ProcessHandle,
        _policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn kill(
        &self,
        _handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn status(&self, _handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn exit_code(&self, _handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        Err(self.unavailable())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }
}

fn invocation() -> CliInvocationContext {
    let mut context = CliInvocationContext::new("TEST_ROLE", "cloud-model");
    context.owner_wp = Some("WP-TEST".to_string());
    context.wp_id = Some("WP-TEST".to_string());
    context.mt_id = Some("MT-003".to_string());
    context.session_id = Some("session-test".to_string());
    context.trace_id = Some("trace-test".to_string());
    context.span_id = Some("span-test".to_string());
    context.cancellation_id = Some("cancel-test".to_string());
    context.reclaim_key = Some("reclaim-test".to_string());
    context.requested_trust_class = Some(handshake_core::sandbox::TrustClass::Trusted);
    context.requested_isolation_tier = Some(handshake_core::sandbox::IsolationTier::Tier1Container);
    context.requested_sandbox_capabilities = Some(std::collections::BTreeSet::from([
        handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
    ]));
    context.requested_net_policy = Some(handshake_core::sandbox::NetPolicy::HostInherited);
    context.requested_execution_policy_ref =
        Some(handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF.to_string());
    context.swarm_id = Some("test-swarm".to_string());
    context.worktree_id = Some("test-worktree".to_string());
    context
}

fn fixture_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::ClaudeCode,
        executable_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        args_template: vec![
            "--model".to_string(),
            "{model}".to_string(),
            "--prompt".to_string(),
            "{prompt}".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

#[cfg(windows)]
fn codex_npm_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().expect("Codex npm fixture root");
    let script = root
        .path()
        .join("node_modules")
        .join("@openai")
        .join("codex")
        .join("bin")
        .join("codex.js");
    std::fs::create_dir_all(script.parent().expect("script parent")).expect("script dirs");
    std::fs::write(root.path().join("node.exe"), b"node fixture").expect("node fixture");
    std::fs::write(&script, b"// launcher fixture").expect("script fixture");

    #[cfg(target_arch = "x86_64")]
    let (platform_suffix, target_triple, cpu) = ("win32-x64", "x86_64-pc-windows-msvc", "x64");
    #[cfg(target_arch = "aarch64")]
    let (platform_suffix, target_triple, cpu) = ("win32-arm64", "aarch64-pc-windows-msvc", "arm64");

    let codex_root = script.parent().unwrap().parent().unwrap();
    let dependency_name = format!("@openai/codex-{platform_suffix}");
    let mut dependencies = serde_json::Map::new();
    dependencies.insert(
        dependency_name,
        serde_json::Value::String(format!("npm:@openai/codex@1.2.3-{platform_suffix}")),
    );
    std::fs::write(
        codex_root.join("package.json"),
        serde_json::to_vec(&serde_json::json!({
            "name": "@openai/codex",
            "version": "1.2.3",
            "bin": { "codex": "bin/codex.js" },
            "optionalDependencies": dependencies
        }))
        .expect("launcher manifest JSON"),
    )
    .expect("launcher manifest");
    let platform_root = codex_root
        .join("node_modules")
        .join("@openai")
        .join(format!("codex-{platform_suffix}"));
    let native = platform_root
        .join("vendor")
        .join(target_triple)
        .join("bin")
        .join("codex.exe");
    std::fs::create_dir_all(native.parent().expect("native parent")).expect("native dirs");
    std::fs::write(
        platform_root.join("package.json"),
        serde_json::to_vec(&serde_json::json!({
            "name": "@openai/codex",
            "version": format!("1.2.3-{platform_suffix}"),
            "os": ["win32"],
            "cpu": [cpu]
        }))
        .expect("platform manifest JSON"),
    )
    .expect("platform manifest");
    std::fs::write(&native, b"native fixture v1").expect("native fixture");
    let shim = root.path().join("codex.cmd");
    std::fs::write(
        &shim,
        b"@echo off\r\n\"%dp0%\\node.exe\" \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n",
    )
    .expect("Codex shim");
    (root, shim, native)
}

#[cfg(windows)]
fn codex_config(shim: PathBuf) -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::CodexCli,
        executable_path: shim,
        args_template: vec![
            "exec".to_string(),
            "--json".to_string(),
            "--model".to_string(),
            "{model}".to_string(),
            "{prompt}".to_string(),
        ],
        output_format: CliOutputFormat::JsonStream,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

#[test]
fn cli_bridge_capabilities_are_all_false() {
    // MT-127 red_team minimum_controls[1]: no false advertising on
    // the CLI bridge. None of the inference techniques work through
    // a CLI subprocess.
    let caps = OfficialCliBridgeRuntime::cli_bridge_capabilities();
    assert!(!caps.supports_lora);
    assert!(!caps.supports_kv_prefix_cache);
    assert!(!caps.supports_activation_steering);
    assert!(!caps.supports_subquadratic);
    assert!(!caps.supports_speculative_draft);
    assert!(!caps.supports_eagle3);
}

#[test]
fn cli_bridge_invoke_routes_through_spawner() {
    let spawner = Arc::new(EchoSpawner {
        cancel_reported: Mutex::new(false),
    });
    let runtime = OfficialCliBridgeRuntime::new(spawner);
    let handle = runtime
        .register_bridge(
            fixture_config(),
            "claude-3.5-sonnet",
            "2026-05-20T06:30:00Z",
        )
        .expect("register");
    let receipt = runtime
        .invoke(handle.model_id, "hello world", &invocation())
        .expect("invoke");
    assert!(receipt.stdout.contains("claude-3.5-sonnet"));
    assert!(receipt.stdout.contains("hello world"));
    assert_eq!(receipt.exit_code, Some(0));
    assert_eq!(
        receipt.model_id, handle.model_id,
        "runtime receipt must preserve the canonical registered model id"
    );
}

#[test]
fn cli_bridge_rejects_generic_command_interpreters() {
    #[cfg(windows)]
    let interpreter =
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()))
            .join(r"System32\cmd.exe");
    #[cfg(not(windows))]
    let interpreter = PathBuf::from("/bin/sh");

    let error = validate_cli_executable_path(&interpreter)
        .expect_err("generic command interpreter must not be a CLI entrypoint");
    assert!(
        matches!(error, OfficialCliBridgeError::ExecutableIdentity(_)),
        "expected executable-identity rejection, got {error:?}"
    );
}

#[cfg(windows)]
#[test]
fn codex_bridge_pins_the_final_native_npm_graph_and_preset() {
    let (_root, shim, native) = codex_npm_fixture();
    validate_cli_executable_path(&shim).expect("official synthetic npm graph");
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("ledger");
    let runtime = OfficialCliBridgeRuntime::new(Arc::new(LiveCliSpawner::new(
        Arc::new(ledger),
        LiveCliSpawner::native_cli_registry(),
    )));
    let handle = runtime
        .register_bridge(codex_config(shim), "gpt-5.4", "2026-07-15T00:00:00Z")
        .expect("pin complete Codex graph");

    std::fs::write(native, b"native fixture v2").expect("mutate final native binary");
    let error = runtime
        .invoke(handle.model_id, "hello", &invocation())
        .expect_err("mutated final native executable must invalidate the pinned graph");
    assert!(matches!(
        error,
        OfficialCliBridgeError::ExecutableIdentity(_)
    ));
}

#[cfg(windows)]
#[test]
fn codex_bridge_rejects_non_jsonl_or_non_model_bound_presets() {
    let (_root, shim, _native) = codex_npm_fixture();
    let runtime = OfficialCliBridgeRuntime::new(Arc::new(EchoSpawner {
        cancel_reported: Mutex::new(false),
    }));
    let mut config = codex_config(shim.clone());
    config.output_format = CliOutputFormat::RawText;
    assert!(matches!(
        runtime.register_bridge(config, "gpt-5.4", "2026-07-15T00:00:00Z"),
        Err(OfficialCliBridgeError::InvalidCodexPreset(_))
    ));

    let mut config = codex_config(shim);
    config.args_template = vec![
        "exec".to_string(),
        "--json".to_string(),
        "{prompt}".to_string(),
    ];
    assert!(matches!(
        runtime.register_bridge(config, "gpt-5.4", "2026-07-15T00:00:00Z"),
        Err(OfficialCliBridgeError::InvalidModelBinding(_))
    ));
}

#[test]
fn cli_bridge_register_validates_placeholders_and_timeout() {
    let spawner = Arc::new(EchoSpawner {
        cancel_reported: Mutex::new(false),
    });
    let runtime = OfficialCliBridgeRuntime::new(spawner);
    let mut bad = fixture_config();
    bad.args_template = vec!["no-placeholder".to_string()];
    let err = runtime
        .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:30:00Z")
        .expect_err("missing placeholder");
    assert!(matches!(
        err,
        OfficialCliBridgeError::MissingPromptPlaceholder
    ));

    let mut bad = fixture_config();
    bad.timeout_seconds = 0;
    let err = runtime
        .register_bridge(bad, "claude-3.5-sonnet", "2026-05-20T06:30:00Z")
        .expect_err("zero timeout");
    assert!(matches!(err, OfficialCliBridgeError::InvalidTimeout));
}

#[test]
fn cli_bridge_render_args_substitutes_placeholders() {
    let rendered = OfficialCliBridgeRuntime::render_args(
        &vec![
            "--model".to_string(),
            "{model}".to_string(),
            "--text".to_string(),
            "<<{prompt}>>".to_string(),
        ],
        "claude-3.5-sonnet",
        "hello",
    );
    assert_eq!(rendered[1], "claude-3.5-sonnet");
    assert_eq!(rendered[3], "<<hello>>");
}

#[test]
fn cli_bridge_invoke_unregistered_model_errors() {
    let spawner = Arc::new(EchoSpawner {
        cancel_reported: Mutex::new(false),
    });
    let runtime = OfficialCliBridgeRuntime::new(spawner);
    let err = runtime
        .invoke(ModelId::new_v7(), "x", &invocation())
        .expect_err("unknown model");
    assert!(matches!(err, OfficialCliBridgeError::ModelNotRegistered(_)));
}

/// A trivially fast, host-native config: find a literal in a fixture file and
/// exit immediately. Used to prove the ledger row is registered the moment
/// the child pid is known, without relying on inherited console handles.
#[cfg(windows)]
fn fast_exit_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from(
            std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()),
        )
        .join(r"System32\findstr.exe"),
        args_template: vec!["{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

#[cfg(windows)]
fn continuous_output_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: std::env::current_exe().expect("integration-test executable path"),
        args_template: vec![
            "--exact".to_string(),
            "official_cli_long_running_child_helper".to_string(),
            "--ignored".to_string(),
            "--nocapture".to_string(),
            "--test-threads={prompt}".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 1,
    }
}

/// Subprocess-only fixture. Parent bridge tests invoke this exact pinned test
/// binary under the sandbox; ordinary test runs leave it ignored.
#[test]
#[ignore = "subprocess fixture for official CLI timeout/cancellation tests"]
fn official_cli_long_running_child_helper() {
    use std::io::Write;

    for index in 0..300 {
        println!("official-cli-child-chunk-{index}");
        std::io::stdout().flush().expect("flush child chunk");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(windows)]
fn queue_output_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from(
            std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()),
        )
        .join(r"System32\choice.exe"),
        args_template: vec![
            "/T".to_string(),
            "30".to_string(),
            "/C".to_string(),
            "YN".to_string(),
            "/D".to_string(),
            "Y".to_string(),
            "/M".to_string(),
            "{prompt}".to_string(),
        ],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 10,
    }
}

#[cfg(not(windows))]
fn continuous_output_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from("/usr/bin/yes"),
        args_template: vec!["{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 1,
    }
}

#[cfg(not(windows))]
fn queue_output_config() -> CliBridgeConfig {
    continuous_output_config()
}

fn draining_chunk_channel() -> (
    tokio::sync::mpsc::Sender<Vec<u8>>,
    std::thread::JoinHandle<()>,
) {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
    let worker = std::thread::spawn(move || while receiver.blocking_recv().is_some() {});
    (sender, worker)
}

// These tests launch real AppContainer processes and measure post-start
// timeout/cancellation behavior. Serialize only the integration fixtures so
// their wall-clock assertions do not include another test's bounded startup
// admission wait; production concurrency remains exercised by the adapter's
// bounded semaphore rather than a fail-fast global flag.
static NATIVE_ATTACHED_TEST_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn continuous_output_cannot_starve_live_timeout_polling() {
    let _native_serial = NATIVE_ATTACHED_TEST_SERIAL.lock().await;
    let store = CapturingLedgerStore::default();
    let (ledger, writer) = LedgerBatcher::spawn(
        Arc::new(store),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let ledger = Arc::new(ledger);
    let spawner = LiveCliSpawner::new(ledger.clone(), LiveCliSpawner::native_cli_registry());
    spawner
        .pin_config(&continuous_output_config())
        .expect("pin continuous-output fixture");
    let (chunk_sender, worker) = draining_chunk_channel();
    let started = std::time::Instant::now();
    let result = spawner.spawn_streaming(
        &continuous_output_config(),
        &invocation(),
        "continuous-model",
        "30",
        &chunk_sender,
    );
    drop(chunk_sender);
    worker.join().expect("bounded chunk drain terminates");
    ledger.begin_close();
    writer
        .await
        .expect("ledger writer join")
        .expect("ledger flush");
    assert!(
        matches!(result, Err(OfficialCliBridgeError::SpawnTimeout { .. })),
        "expected bounded timeout, got {result:?}"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn continuous_output_cannot_starve_live_cancellation_polling() {
    let _native_serial = NATIVE_ATTACHED_TEST_SERIAL.lock().await;
    let store = CapturingLedgerStore::default();
    let (ledger, writer) = LedgerBatcher::spawn(
        Arc::new(store),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let ledger = Arc::new(ledger);
    let spawner = LiveCliSpawner::new(ledger.clone(), LiveCliSpawner::native_cli_registry());
    spawner
        .pin_config(&continuous_output_config())
        .expect("pin continuous-output fixture");
    let token = handshake_core::model_runtime::CancellationToken::new();
    let cancel_token = token.clone();
    let cancel_worker = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        cancel_token.cancel();
    });
    let cancellation = CliCancellationContext::new(vec![token]);
    let (chunk_sender, worker) = draining_chunk_channel();
    let started = std::time::Instant::now();
    let result = spawner.spawn_streaming_cancellable(
        &continuous_output_config(),
        &invocation(),
        "continuous-model",
        "30",
        &chunk_sender,
        &cancellation,
    );
    drop(chunk_sender);
    cancel_worker
        .join()
        .expect("cancellation worker terminates");
    worker.join().expect("bounded chunk drain terminates");
    let receipt = result.expect("continuous producer must return cancelled receipt");
    ledger.begin_close();
    writer
        .await
        .expect("ledger writer join")
        .expect("ledger flush");
    assert!(receipt.cancelled);
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
}

#[cfg(not(windows))]
fn fast_exit_config() -> CliBridgeConfig {
    CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: PathBuf::from("/bin/echo"),
        args_template: vec!["{model}-{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    }
}

/// In-memory ProcessLedgerStore that captures recorded events, mirroring
/// the established pattern in `process_ledger_tests.rs`.
#[derive(Clone, Default)]
struct CapturingLedgerStore {
    events: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl CapturingLedgerStore {
    fn events(&self) -> Vec<LedgerEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for CapturingLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events.lock().unwrap().extend(events);
        Ok(())
    }
}

/// MT-127 remediation (MT-122-class), end-to-end: the LiveCliSpawner's
/// ProcessOwnershipLedger registration is UNCONDITIONAL. The ledger is
/// mandatory at construction, so every CLI-bridge subprocess spawn records
/// an attributable START row (engine_kind=OfficialCliBridge) the moment the
/// child pid is known AND a matching STOP row after the child exits. Proves
/// the spawned CLI subprocess is always attributable + reclaimable across
/// its full lifecycle, closing the optional-ledger (MT-122-class) gap.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_cli_spawner_records_official_cli_bridge_start_and_stop_rows() {
    let _native_serial = NATIVE_ATTACHED_TEST_SERIAL.lock().await;
    let mut config = fast_exit_config();
    if !config.executable_path.exists() {
        eprintln!(
            "skipping ledger-row test; executable missing: {}",
            config.executable_path.display()
        );
        return;
    }
    let working_dir = tempfile::tempdir().expect("actual child working directory");
    let probe_path = working_dir.path().join("probe.txt");
    std::fs::write(&probe_path, "official-cli-probe\n").expect("write fast-exit probe");
    config.args_template = vec![
        "{prompt}".to_string(),
        probe_path.to_string_lossy().to_string(),
    ];
    config.working_dir = Some(working_dir.path().to_path_buf());
    let working_dir_text = working_dir.path().to_string_lossy().to_string();
    let mut governed_invocation = invocation();
    governed_invocation.working_dir = Some(working_dir_text.clone());

    let store = CapturingLedgerStore::default();
    let (batcher, writer) = LedgerBatcher::spawn(
        Arc::new(store.clone()),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );

    // The ledger is MANDATORY: there is no ledger-less / optional builder.
    let batcher = Arc::new(batcher);
    let spawner = LiveCliSpawner::new(batcher.clone(), LiveCliSpawner::native_cli_registry());
    spawner.pin_config(&config).expect("pin fast-exit fixture");
    let receipt = spawner
        .spawn(
            &config,
            &governed_invocation,
            "claude-3.5-sonnet",
            "official-cli-probe",
        )
        .expect("spawn + ledger registration must succeed");
    assert!(receipt.pid.is_some(), "live spawn must capture a pid");

    batcher.begin_close();
    writer
        .await
        .expect("ledger writer join")
        .expect("ledger flush");

    let events = store.events();
    assert_eq!(
        events.len(),
        2,
        "exactly one START and one matching STOP ProcessOwnershipLedger row, got {events:?}"
    );

    let LedgerEvent::Start(start) = &events[0] else {
        panic!(
            "expected the first event to be a Start, got {:?}",
            events[0]
        );
    };
    assert_eq!(
        start.engine_kind,
        ProcessEngineKind::OfficialCliBridge,
        "START row must be attributed to engine_kind=OfficialCliBridge"
    );
    assert_eq!(start.owner_role, "TEST_ROLE");
    assert_eq!(start.owner_wp.as_deref(), Some("WP-TEST"));
    assert_eq!(start.os_pid, receipt.pid);
    assert_eq!(start.mt_id.as_deref(), Some("MT-003"));
    assert!(start.sandbox_adapter_id.is_some());
    assert_eq!(start.metadata_jsonb["session_id"], "session-test");
    assert_eq!(start.metadata_jsonb["trace_id"], "trace-test");
    assert_eq!(start.metadata_jsonb["span_id"], "span-test");
    assert_eq!(start.metadata_jsonb["cancellation_id"], "cancel-test");
    assert_eq!(start.metadata_jsonb["reclaim_key"], "reclaim-test");
    assert_eq!(
        start.metadata_jsonb["selected_model_name"],
        "claude-3.5-sonnet"
    );
    assert_eq!(start.metadata_jsonb["model_identity"], "claude-3.5-sonnet");
    assert_eq!(
        start.metadata_jsonb["requested_model_identity"],
        "cloud-model"
    );
    assert_eq!(start.metadata_jsonb["owner_wp"], "WP-TEST");
    assert_eq!(start.metadata_jsonb["mt_id"], "MT-003");
    assert_eq!(start.metadata_jsonb["requested_trust_class"], "trusted");
    assert_eq!(
        start.metadata_jsonb["requested_isolation_tier"],
        "tier1_container"
    );
    assert_eq!(
        start.metadata_jsonb["requested_sandbox_capabilities"],
        serde_json::json!(["high_stdio_throughput"])
    );
    assert_eq!(
        start.metadata_jsonb["requested_net_policy"],
        "host_inherited"
    );
    assert_eq!(
        start.metadata_jsonb["requested_execution_policy_ref"],
        handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF
    );
    assert_eq!(start.metadata_jsonb["swarm_id"], "test-swarm");
    assert_eq!(start.metadata_jsonb["worktree_id"], "test-worktree");
    assert_eq!(start.metadata_jsonb["working_dir"], working_dir_text);
    assert_eq!(start.metadata_jsonb["requested_swarm_id"], "test-swarm");
    assert_eq!(
        start.metadata_jsonb["requested_worktree_id"],
        "test-worktree"
    );
    assert_eq!(
        start.metadata_jsonb["requested_working_dir"],
        working_dir_text
    );
    assert_eq!(start.metadata_jsonb["effective_trust_class"], "trusted");
    assert_eq!(
        start.metadata_jsonb["effective_isolation_tier"],
        "tier1_container"
    );
    assert_eq!(
        start.metadata_jsonb["effective_net_policy"],
        "outbound_internet_client"
    );
    assert_eq!(
        start.metadata_jsonb["execution_policy_resolution"]["requested_ref"],
        handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF
    );
    assert_eq!(
        start.metadata_jsonb["execution_policy_resolution"]["resolution_status"],
        "resolved"
    );
    assert_eq!(
        start.metadata_jsonb["execution_policy_resolution"]["effective_ref"],
        handshake_core::sandbox::CLI_BRIDGE_EFFECTIVE_EXECUTION_POLICY_REF
    );
    assert_eq!(
        start.metadata_jsonb["execution_policy_resolution"]["sandbox_boundary_adapter"],
        "handshake_native"
    );
    assert_eq!(
        start.metadata_jsonb["execution_policy_resolution"]["effective_isolation_tier"],
        "tier1_container"
    );
    assert_eq!(
        start.metadata_jsonb["execution_policy_resolution"]["effective_net_policy"],
        "outbound_internet_client"
    );
    assert_eq!(
        start.metadata_jsonb["effective_sandbox_capabilities"]["adapter_id"],
        "handshake_native"
    );
    assert_eq!(start.metadata_jsonb["effective_swarm_id"], "test-swarm");
    assert_eq!(
        start.metadata_jsonb["effective_worktree_id"],
        "test-worktree"
    );
    assert_eq!(
        start.metadata_jsonb["effective_working_dir"],
        working_dir_text
    );
    assert_eq!(
        start.metadata_jsonb["subprocess_kind"].as_str(),
        Some("official_cli_bridge")
    );
    let LedgerEvent::Stop(stop) = &events[1] else {
        panic!(
            "expected the second event to be a Stop, got {:?}",
            events[1]
        );
    };
    // The STOP row must reconcile to the SAME process: same uuid, same pid,
    // same engine_kind/owner_role, so the row is attributable + reclaimable
    // across its full lifecycle.
    assert_eq!(
        stop.process_uuid, start.process_uuid,
        "STOP must reference the same ProcessOwnership row uuid as START"
    );
    assert_eq!(stop.os_pid, receipt.pid);
    assert_eq!(stop.engine_kind, ProcessEngineKind::OfficialCliBridge);
    assert_eq!(stop.owner_role, "TEST_ROLE");
    assert_eq!(stop.owner_wp.as_deref(), Some("WP-TEST"));
    assert_eq!(stop.wp_id.as_deref(), Some("WP-TEST"));
    assert_eq!(stop.mt_id.as_deref(), Some("MT-003"));
    assert_eq!(stop.exit_code, receipt.exit_code);
    assert_eq!(
        stop.stop_reason.as_deref(),
        Some("official_cli_bridge_exit"),
        "clean exit must record the canonical stop reason"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_cli_spawner_closed_chunk_queue_kills_child_before_recording_stop() {
    let _native_serial = NATIVE_ATTACHED_TEST_SERIAL.lock().await;
    let config = queue_output_config();
    let store = CapturingLedgerStore::default();
    let (batcher, writer) = LedgerBatcher::spawn(
        Arc::new(store.clone()),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let batcher = Arc::new(batcher);
    let spawner = LiveCliSpawner::new(batcher.clone(), LiveCliSpawner::native_cli_registry());
    spawner
        .pin_config(&config)
        .expect("pin queue-failure fixture");

    let delivery_error = {
        let (chunk_sender, chunk_receiver) = tokio::sync::mpsc::channel(1);
        drop(chunk_receiver);
        spawner.spawn_streaming(
            &config,
            &invocation(),
            "cloud-model",
            "queue-failure-probe",
            &chunk_sender,
        )
    };
    assert!(
        matches!(
            delivery_error,
            Err(OfficialCliBridgeError::SpawnFailed { .. })
        ),
        "closed bounded chunk queue must fail the live spawn"
    );
    batcher.begin_close();
    writer
        .await
        .expect("ledger writer join")
        .expect("ledger flush");
    let events = store.events();
    assert_eq!(
        events.len(),
        2,
        "queue failure must emit one START and one STOP"
    );
    let (LedgerEvent::Start(start), LedgerEvent::Stop(stop)) = (&events[0], &events[1]) else {
        panic!("expected ordered START/STOP events, got {events:?}");
    };
    assert_eq!(stop.process_uuid, start.process_uuid);
    assert_eq!(stop.os_pid, start.os_pid);
    assert_eq!(
        stop.stop_reason.as_deref(),
        Some("official_cli_bridge_chunk_delivery_failure")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failed_termination_with_never_eof_pipe_returns_within_cleanup_deadline() {
    let config = fast_exit_config();
    let store = CapturingLedgerStore::default();
    let (batcher, writer) = LedgerBatcher::spawn(
        Arc::new(store.clone()),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let batcher = Arc::new(batcher);
    let adapter = Arc::new(ReapFailingSandboxAdapter::new());
    let adapter_id = adapter.capabilities().adapter_id;
    let mut registry = SandboxAdapterRegistry::new(adapter_id);
    registry.register(adapter.clone());
    let spawner = LiveCliSpawner::new(batcher.clone(), Arc::new(registry));
    spawner
        .pin_config(&config)
        .expect("pin failing-termination fixture");

    let (chunk_sender, chunk_receiver) = tokio::sync::mpsc::channel(1);
    drop(chunk_receiver);
    let started = Instant::now();
    let error = spawner
        .spawn_streaming(
            &config,
            &invocation(),
            "cloud-model",
            "never-eof-reap-failure",
            &chunk_sender,
        )
        .expect_err("closed output queue plus failed reap must remain a typed failure");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "failed termination plus a never-EOF pipe must not hang cleanup"
    );
    let OfficialCliBridgeError::SpawnFailed { reason, .. } = error else {
        panic!("cleanup failure must stay typed as SpawnFailed")
    };
    assert!(reason.contains("could not prove process-tree termination/reap"));
    assert!(
        reason.contains("pipe-reader cleanup exceeded its bounded deadline"),
        "bounded reader timeout must remain observable: {reason}"
    );

    adapter.release_reader_and_wait();
    batcher.begin_close();
    writer
        .await
        .expect("ledger writer join")
        .expect("ledger flush");
    let events = store.events();
    assert_eq!(events.len(), 1, "failed reap must leave only START open");
    assert!(matches!(events.as_slice(), [LedgerEvent::Start(_)]));
}

#[test]
fn live_cli_spawner_refuses_before_child_spawn_when_complete_ledger_capacity_is_unavailable() {
    let config = fast_exit_config();
    let (batcher, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 1,
            batch_size: 1,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("one-slot ledger exists but cannot own START plus future STOP");
    let spawner = LiveCliSpawner::new(Arc::new(batcher), LiveCliSpawner::native_cli_registry());
    let error = spawner
        .spawn(&config, &invocation(), "cloud-model", "must-not-run")
        .expect_err("ledger preflight must reject before Command::spawn");
    assert!(matches!(
        error,
        OfficialCliBridgeError::LedgerPreflight { .. }
    ));
    let (chunk_sender, mut chunk_receiver) = tokio::sync::mpsc::channel(1);
    let streaming_error = spawner
        .spawn_streaming(
            &config,
            &invocation(),
            "cloud-model",
            "must-not-run",
            &chunk_sender,
        )
        .expect_err("streaming path must reject before Command::spawn");
    assert!(matches!(
        streaming_error,
        OfficialCliBridgeError::LedgerPreflight { .. }
    ));
    let cancellable_error = spawner
        .spawn_streaming_cancellable(
            &config,
            &invocation(),
            "cloud-model",
            "must-not-run",
            &chunk_sender,
            &CliCancellationContext::default(),
        )
        .expect_err("cancellable path must reject before Command::spawn");
    assert!(matches!(
        cancellable_error,
        OfficialCliBridgeError::LedgerPreflight { .. }
    ));
    assert!(chunk_receiver.try_recv().is_err());
    let (strict_batcher, _strict_drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("ledger for stricter-than-native rejection");
    let mut strict_invocation = invocation();
    strict_invocation.requested_trust_class = Some(handshake_core::sandbox::TrustClass::Reviewed);
    strict_invocation.requested_isolation_tier =
        Some(handshake_core::sandbox::IsolationTier::Tier1Container);
    strict_invocation.requested_sandbox_capabilities = Some(std::collections::BTreeSet::from([
        handshake_core::sandbox::RequiredCapability::VeryStrongNetworkIsolation,
    ]));
    let strict_spawner = LiveCliSpawner::new(
        Arc::new(strict_batcher),
        LiveCliSpawner::native_cli_registry(),
    );
    strict_spawner
        .pin_config(&config)
        .expect("pin strict posture fixture");
    let strict_error = strict_spawner
        .spawn(&config, &strict_invocation, "cloud-model", "must-not-run")
        .expect_err("unsupported capability must reject before child spawn");
    assert!(matches!(
        strict_error,
        OfficialCliBridgeError::SpawnFailed { .. }
    ));
}

#[test]
fn live_cli_spawner_rejects_executable_mutation_after_pin_before_spawn() {
    let temp = tempfile::tempdir().expect("identity fixture directory");
    let copied_executable = temp.path().join(if cfg!(windows) {
        "official-cli-fixture.exe"
    } else {
        "official-cli-fixture"
    });
    std::fs::copy(
        std::env::current_exe().expect("integration-test executable path"),
        &copied_executable,
    )
    .expect("copy executable identity fixture");
    let config = CliBridgeConfig {
        cli_kind: CliKind::Other,
        executable_path: copied_executable.clone(),
        args_template: vec!["{prompt}".to_string()],
        output_format: CliOutputFormat::RawText,
        env_vars: HashMap::new(),
        working_dir: None,
        timeout_seconds: 30,
    };
    let (batcher, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger");
    let spawner = LiveCliSpawner::new(Arc::new(batcher), LiveCliSpawner::native_cli_registry());
    spawner.pin_config(&config).expect("pin executable graph");

    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&copied_executable)
        .expect("open identity fixture for mutation")
        .write_all(b"identity-drift")
        .expect("mutate identity fixture");

    let error = spawner
        .spawn(&config, &invocation(), "cloud-model", "must-not-run")
        .expect_err("identity mutation must fail before sandbox spawn");
    assert!(
        matches!(error, OfficialCliBridgeError::ExecutableIdentity(_)),
        "expected executable-identity rejection, got {error:?}"
    );
}

#[test]
fn live_cli_spawner_rejects_execution_control_env_before_spawn() {
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("ledger for environment-boundary proof");
    let spawner = LiveCliSpawner::new(Arc::new(ledger), LiveCliSpawner::native_cli_registry());
    for name in [
        "NODE_OPTIONS",
        "Path",
        "COMSPEC",
        "PATHEXT",
        "DOTNET_STARTUP_HOOKS",
        "HTTP_PROXY",
        "OPENAI_API_KEY",
        "OPENAI_BASE_URL",
    ] {
        let mut config = fast_exit_config();
        config
            .env_vars
            .insert(name.to_string(), "unsafe-override".to_string());
        assert!(
            matches!(
                spawner.pin_config(&config),
                Err(OfficialCliBridgeError::UnsafeEnvironmentVariable(rejected)) if rejected == name
            ),
            "configured environment variable {name} must fail closed before identity pinning or spawn"
        );
    }

    let mut presentation_only = fast_exit_config();
    presentation_only
        .env_vars
        .insert("NO_COLOR".to_string(), "1".to_string());
    spawner
        .pin_config(&presentation_only)
        .expect("presentation-only explicit environment is accepted");
}
