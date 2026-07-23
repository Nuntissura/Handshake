use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use bytes::Bytes;
use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
use handshake_core::sandbox::{
    default_no_op_capabilities, AdapterCapabilities, AdapterId, AttachedNetworkMode,
    AttachedProcessSpec, AttachedSandboxProcess, AttachedStdioContract, BindMode, BindSpec,
    Command, ExecResult, GpuPassthrough, ImageRef, IsolationStrength, IsolationTier,
    LedgerDecorator, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus, RequiredCapability,
    ResourceLimits, SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass, TrustClass,
};

#[derive(Debug, Clone)]
struct NoopAdapter {
    capabilities: AdapterCapabilities,
    attached_spawn_calls: Arc<AtomicUsize>,
    attached_stdio_calls: Arc<AtomicUsize>,
    attached_network_validation_calls: Arc<AtomicUsize>,
    warm_agent_transport_calls: Arc<AtomicUsize>,
}

impl Default for NoopAdapter {
    fn default() -> Self {
        Self {
            capabilities: default_no_op_capabilities(),
            attached_spawn_calls: Arc::new(AtomicUsize::new(0)),
            attached_stdio_calls: Arc::new(AtomicUsize::new(0)),
            attached_network_validation_calls: Arc::new(AtomicUsize::new(0)),
            warm_agent_transport_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl NoopAdapter {
    fn unavailable(&self) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: self.capabilities.adapter_id.clone(),
            reason: "noop adapter has no isolation backend".to_string(),
        }
    }
}

#[async_trait]
impl SandboxAdapter for NoopAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn spawn_attached(
        &self,
        _spec: AttachedProcessSpec,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        self.attached_spawn_calls.fetch_add(1, Ordering::SeqCst);
        Err(self.unavailable())
    }

    async fn spawn_attached_with_stdio(
        &self,
        _spec: AttachedProcessSpec,
        _stdio: AttachedStdioContract,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        self.attached_stdio_calls.fetch_add(1, Ordering::SeqCst);
        Err(self.unavailable())
    }

    fn validate_attached_network_mode(
        &self,
        _mode: AttachedNetworkMode,
    ) -> Result<(), SandboxAdapterError> {
        self.attached_network_validation_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: Command,
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

    async fn warm_agent_transport(
        &self,
        _handle: &ProcessHandle,
    ) -> Result<Arc<dyn handshake_core::model_runtime::WarmAgentTransport>, SandboxAdapterError>
    {
        self.warm_agent_transport_calls
            .fetch_add(1, Ordering::SeqCst);
        Err(self.unavailable())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }
}

fn attached_spec() -> AttachedProcessSpec {
    AttachedProcessSpec {
        executable_path: PathBuf::from("attached-probe"),
        args: vec!["--probe".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        network_mode: AttachedNetworkMode::DenyAll,
        trust_class: TrustClass::Trusted,
        required_capabilities: BTreeSet::new(),
        requested_isolation_tier: IsolationTier::Tier1Container,
        requested_net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        startup_timeout_ms: 1_000,
        ephemeral_cleanup_paths: Vec::new(),
        execution_policy_ref: "execution-policy://test/decorator".to_string(),
        resolved_execution_policy: None,
        swarm_id: Some("test-swarm".to_string()),
        worktree_id: Some("test-worktree".to_string()),
        checkout_lease_id: None,
        checkout_lease_owner_generation: None,
        checkout_lease_canonical_working_dir: None,
    }
}

fn test_ledger() -> LedgerBatcher {
    let (batcher, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual ledger batcher");
    batcher
}

#[tokio::test]
async fn ledger_decorator_delegates_all_attached_contract_methods() {
    let inner = Arc::new(NoopAdapter::default());
    let decorator = LedgerDecorator::new(inner.clone(), test_ledger());

    let spawn_error = match decorator.spawn_attached(attached_spec()).await {
        Err(error) => error,
        Ok(_) => panic!("recording adapter unexpectedly returned an attached process"),
    };
    assert!(matches!(
        spawn_error,
        SandboxAdapterError::AdapterUnavailable { .. }
    ));

    let stdio_error = match decorator
        .spawn_attached_with_stdio(
            attached_spec(),
            AttachedStdioContract::null_stdin_piped_output(),
        )
        .await
    {
        Err(error) => error,
        Ok(_) => panic!("recording adapter unexpectedly returned a stdio-attached process"),
    };
    assert!(matches!(
        stdio_error,
        SandboxAdapterError::AdapterUnavailable { .. }
    ));

    decorator
        .validate_attached_network_mode(AttachedNetworkMode::OutboundInternetClient)
        .expect("inner attached-network validator result is preserved");
    let handle = ProcessHandle::new(AdapterId::new("noop"), None, "warm-agent-probe");
    let warm_error = match decorator.warm_agent_transport(&handle).await {
        Err(error) => error,
        Ok(_) => panic!("recording adapter unexpectedly returned a warm-agent transport"),
    };
    assert!(matches!(
        warm_error,
        SandboxAdapterError::AdapterUnavailable { .. }
    ));

    assert_eq!(inner.attached_spawn_calls.load(Ordering::SeqCst), 1);
    assert_eq!(inner.attached_stdio_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        inner
            .attached_network_validation_calls
            .load(Ordering::SeqCst),
        1
    );
    assert_eq!(inner.warm_agent_transport_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn sandbox_adapter_trait_object_is_constructible_and_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<NoopAdapter>();

    let adapter: Box<dyn SandboxAdapter> = Box::new(NoopAdapter::default());
    let caps = adapter.capabilities();

    assert_eq!(caps.adapter_id, AdapterId::new("noop"));
    assert_eq!(caps.filesystem_isolation_strength, IsolationStrength::Weak);
    assert_eq!(caps.network_isolation_strength, IsolationStrength::Weak);
}

#[tokio::test]
async fn noop_adapter_methods_fail_closed_without_backend() {
    let adapter = NoopAdapter::default();
    let handle = ProcessHandle::new(AdapterId::new("noop"), None, "noop-internal");

    let spec = ProcessSpec {
        id: AdapterId::new("noop"),
        image_or_root: ImageRef::new("noop-root"),
        cmd: vec!["noop".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: vec![BindSpec {
            host_path: PathBuf::from("fixtures/input"),
            guest_path: PathBuf::from("/guest/input"),
            mode: BindMode::ReadOnly,
        }],
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        idle_timeout_ms: None,
        required_capabilities: BTreeSet::from([RequiredCapability::VeryStrongNetworkIsolation]),
        trust_class: TrustClass::default(),
        metadata: BTreeMap::new(),
    };
    let command = Command {
        argv: vec!["noop".to_string()],
        env_overlay: BTreeMap::new(),
        stdin: Some(Bytes::from_static(b"input")),
        timeout_ms: Some(1_000),
    };

    assert_adapter_unavailable(adapter.spawn(spec).await);
    assert_adapter_unavailable(adapter.exec(&handle, command).await);
    assert_adapter_unavailable(
        adapter
            .fs_bind(
                &handle,
                PathBuf::from("fixtures/model.gguf"),
                PathBuf::from("/models/model.gguf"),
                BindMode::ReadOnly,
            )
            .await,
    );
    assert_adapter_unavailable(adapter.net_policy(&handle, NetPolicy::LoopbackOnly).await);
    assert_adapter_unavailable(adapter.kill(&handle, Signal::Term).await);
    assert_adapter_unavailable(adapter.status(&handle).await);
    assert_adapter_unavailable(adapter.exit_code(&handle).await);
}

#[test]
fn adapter_capabilities_are_clonable_and_serde_round_trip() {
    let capabilities = default_no_op_capabilities();
    let cloned = capabilities.clone();

    assert_eq!(cloned, capabilities);
    assert_eq!(capabilities.gpu_passthrough, GpuPassthrough::None);
    assert_eq!(capabilities.stdio_throughput_class, ThroughputClass::Low);
    assert!(!capabilities.win32_native_fidelity);
    assert!(!capabilities.cross_machine_portable);

    let encoded = serde_json::to_string(&capabilities).expect("capabilities serialize");
    assert_eq!(
        serde_json::from_str::<AdapterCapabilities>(&encoded).expect("capabilities deserialize"),
        capabilities
    );
}

#[test]
fn sandbox_adapter_trait_source_has_exact_public_method_shape() {
    let adapter_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("sandbox")
        .join("adapter.rs");
    let source = fs::read_to_string(adapter_path).expect("read sandbox adapter source");
    let trait_source = source
        .split("pub trait SandboxAdapter")
        .nth(1)
        .and_then(|body| body.split("\n}\n").next())
        .expect("trait source is present");

    let declarations = trait_source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("async fn ") || line.starts_with("fn "))
        .collect::<Vec<_>>();

    // 8 core methods + three attached-live contract methods + the Master Spec
    // v02.187 additive snapshot/copy/delete/warm-agent methods = 17.
    assert_eq!(
        declarations.len(),
        17,
        "SandboxAdapter public method shape drifted: {declarations:?}"
    );

    for method in [
        "spawn",
        "spawn_attached",
        "spawn_attached_with_stdio",
        "validate_attached_network_mode",
        "exec",
        "fs_bind",
        "net_policy",
        "kill",
        "status",
        "exit_code",
        "snapshot",
        "restore",
        "delete_snapshot",
        "copy_in",
        "copy_out",
        "warm_agent_transport",
        "capabilities",
    ] {
        assert!(
            declarations
                .iter()
                .any(|line| line.starts_with(&format!("async fn {method}("))
                    || line.starts_with(&format!("fn {method}("))),
            "missing SandboxAdapter method {method}"
        );
    }
}

#[test]
fn sandbox_adapter_trait_has_no_adapter_specific_imports() {
    let adapter_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("sandbox")
        .join("adapter.rs");
    let source = fs::read_to_string(adapter_path).expect("read sandbox adapter source");
    let lower = source.to_ascii_lowercase();

    for banned in [
        "podman::",
        "bollard::",
        "docker::",
        "win32::",
        "windows::",
        "windows_sys::",
    ] {
        assert!(
            !lower.contains(banned),
            "sandbox adapter trait must not import adapter-specific crate surface `{banned}`"
        );
    }
}

fn assert_adapter_unavailable<T: std::fmt::Debug>(result: Result<T, SandboxAdapterError>) {
    let error = result.expect_err("noop adapter must fail closed");
    match error {
        SandboxAdapterError::AdapterUnavailable { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new("noop"));
            assert!(reason.contains("noop adapter"));
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

#[tokio::test]
async fn copy_in_out_default_is_unsupported() {
    // Master Spec §3.5.7 #4: the trait exposes copy_in/copy_out; adapters with no
    // live per-file channel (the default) return a typed CopyUnsupported rather
    // than silently succeeding or panicking.
    let adapter = NoopAdapter::default();
    let handle = ProcessHandle::new(AdapterId::new("noop"), None, "noop-copy");
    let r_in = adapter
        .copy_in(&handle, PathBuf::from("/tmp/h"), PathBuf::from("/g"))
        .await;
    assert!(
        matches!(r_in, Err(SandboxAdapterError::CopyUnsupported { .. })),
        "default copy_in must be CopyUnsupported, got {r_in:?}"
    );
    let r_out = adapter
        .copy_out(&handle, PathBuf::from("/g"), PathBuf::from("/tmp/h"))
        .await;
    assert!(
        matches!(r_out, Err(SandboxAdapterError::CopyUnsupported { .. })),
        "default copy_out must be CopyUnsupported, got {r_out:?}"
    );
}
