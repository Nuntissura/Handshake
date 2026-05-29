use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use handshake_core::sandbox::{
    docker_run_args, AdapterCapabilities, AdapterId, BindMode, BindSpec, Command, ExecResult,
    GpuPassthrough, ImageRef, IsolationStrength, IsolationTier, NetPolicy, ProcessHandle,
    ProcessSpec, ProcessStatus, RequiredCapability, ResourceLimits, SandboxAdapter,
    SandboxAdapterError, SandboxAdapterRegistry, SandboxSelectionFailure, Signal, ThroughputClass,
    ValidationJobSpec, ValidationProcessSpecBuilder, ValidationRunnerBindingError,
    ValidationSandboxRunner, WindowsNativeJailAdapter,
};

#[derive(Debug, Clone)]
struct RecordingAdapter {
    capabilities: AdapterCapabilities,
    spawned: Arc<Mutex<Vec<ProcessSpec>>>,
}

impl RecordingAdapter {
    fn new(capabilities: AdapterCapabilities) -> Self {
        Self {
            capabilities,
            spawned: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn spawned(&self) -> Vec<ProcessSpec> {
        self.spawned.lock().expect("spawn log").clone()
    }

    fn unavailable(&self) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: self.capabilities.adapter_id.clone(),
            reason: "recording adapter has no runtime backend".to_string(),
        }
    }
}

#[async_trait]
impl SandboxAdapter for RecordingAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        let adapter_id = self.capabilities.adapter_id.clone();
        self.spawned.lock().expect("spawn log").push(spec);
        Ok(ProcessHandle::new(
            adapter_id,
            Some(4321),
            "validation-spawn",
        ))
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

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }
}

#[test]
fn validation_job_builder_defaults_model_written_code_to_deny_all() {
    let spec = ValidationProcessSpecBuilder
        .build(validation_job(None))
        .expect("validation process spec");

    assert_eq!(spec.id, AdapterId::new("validation-job:VALJOB-001"));
    assert_eq!(spec.image_or_root, ImageRef::new("rust:1.82"));
    assert_eq!(spec.cmd, vec!["cargo", "test", "--locked"]);
    assert_eq!(spec.net_policy, NetPolicy::DenyAll);
    assert_eq!(spec.binds.len(), 1);
    assert_eq!(spec.binds[0].mode, BindMode::ReadWrite);
    assert_eq!(
        spec.metadata.get("validation_lane").map(String::as_str),
        Some("model_written_code")
    );
}

#[test]
fn validation_job_builder_preserves_explicit_network_policy() {
    let spec = ValidationProcessSpecBuilder
        .build(validation_job(Some(NetPolicy::LoopbackOnly)))
        .expect("validation process spec");

    assert_eq!(spec.net_policy, NetPolicy::LoopbackOnly);
}

#[tokio::test]
async fn injected_validation_runner_spawns_through_sandbox_adapter() {
    // Validation jobs run untrusted model-written code, so the builder marks
    // them UntrustedAgent (Master Spec v02.187 §3.5.4) which demands a Tier-3
    // microVM-class adapter. Simulate a strong-isolation adapter so this
    // plumbing test exercises spawn-through without weakening the tier guard.
    let mut strong = capabilities("docker", ThroughputClass::High);
    strong.isolation_tier = IsolationTier::Tier3Microvm;
    let adapter = RecordingAdapter::new(strong);
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.set_docker_explicit_opt_in(true);
    registry.register(Arc::new(adapter.clone()));
    let mut job = validation_job(None);
    job.work_profile_override = Some(AdapterId::new("docker"));
    let runner = ValidationSandboxRunner::from_registry(&registry, &job)
        .expect("docker validation runner selected through explicit opt-in override");

    let handle = runner.spawn().await.expect("validation runner spawn");

    assert_eq!(handle.adapter_id, AdapterId::new("docker"));
    assert_eq!(handle.pid, Some(4321));
    let spawned = adapter.spawned();
    assert_eq!(spawned.len(), 1);
    assert_eq!(spawned[0].net_policy, NetPolicy::DenyAll);
    assert_eq!(
        spawned[0]
            .metadata
            .get("validation_job_id")
            .map(String::as_str),
        Some("VALJOB-001")
    );
}

#[test]
fn registry_selection_honors_validation_work_profile_override_without_fallback() {
    let low_stdio = RecordingAdapter::new(capabilities("docker", ThroughputClass::Medium));
    let fallback_high_stdio =
        RecordingAdapter::new(capabilities("wsl2_podman", ThroughputClass::High));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.set_docker_explicit_opt_in(true);
    registry.register(Arc::new(low_stdio.clone()));
    registry.register(Arc::new(fallback_high_stdio.clone()));
    let mut job = validation_job(None);
    job.required_capabilities = BTreeSet::from([RequiredCapability::HighStdioThroughput]);
    job.work_profile_override = Some(AdapterId::new("docker"));

    let error = match ValidationSandboxRunner::from_registry(&registry, &job) {
        Ok(_) => panic!("override should fail capability check"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        ValidationRunnerBindingError::SandboxSelection(
            SandboxSelectionFailure::OverrideCapabilityMismatch { .. }
        )
    ));
    assert!(low_stdio.spawned().is_empty());
    assert!(fallback_high_stdio.spawned().is_empty());
}

#[test]
fn win32_native_validation_job_does_not_fallback_when_windows_jail_unavailable() {
    let wsl2 = RecordingAdapter::new(capabilities("wsl2_podman", ThroughputClass::High));
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(Arc::new(wsl2.clone()));
    registry.register(Arc::new(
        WindowsNativeJailAdapter::unavailable_for_current_host(),
    ));
    let mut job = validation_job(None);
    job.required_capabilities = BTreeSet::from([RequiredCapability::Win32NativeFidelity]);

    let error = match ValidationSandboxRunner::from_registry(&registry, &job) {
        Ok(_) => panic!("unavailable Windows native jail must fail selection before spawn"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        ValidationRunnerBindingError::SandboxSelection(
            SandboxSelectionFailure::CapabilityUnsatisfied { .. }
        )
    ));
    assert!(wsl2.spawned().is_empty());
}

#[test]
fn docker_argv_parity_fixture_matches_validation_process_spec() {
    let process_spec = ValidationProcessSpecBuilder
        .build(validation_job(None))
        .expect("validation process spec");
    let args = docker_run_args(&process_spec, "hsk-validation-fixture").expect("docker args");
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/kernel_003_validation_argv.json");
    let expected: Vec<String> =
        serde_json::from_slice(&fs::read(fixture_path).expect("fixture")).expect("fixture json");

    assert_eq!(args, expected);
}

#[test]
fn kernel_003_source_audit_has_no_direct_docker_runner_to_replace() {
    let repo = repo_root();
    let promotion_source =
        fs::read_to_string(repo.join("src/backend/handshake_core/src/kernel/promotion.rs"))
            .expect("promotion source");
    assert!(promotion_source.contains("pub struct ValidationRunner"));
    assert!(!promotion_source.contains("DockerRunner"));
    assert!(!promotion_source.contains("DocketAdapter"));

    let bridge_source = fs::read_to_string(
        repo.join("src/backend/handshake_core/src/sandbox/docker/kernel_003_bridge.rs"),
    )
    .expect("kernel 003 bridge source");
    assert!(bridge_source.contains("non_executing_stub_no_docket_adapter_found"));
}

fn validation_job(net_policy: Option<NetPolicy>) -> ValidationJobSpec {
    let mut job = ValidationJobSpec::new(
        "VALJOB-001",
        ImageRef::new("rust:1.82"),
        ["cargo", "test", "--locked"],
    );
    job.env = BTreeMap::from([("RUST_LOG".to_string(), "info".to_string())]);
    job.cwd = Some(PathBuf::from("/workspace"));
    job.binds = vec![BindSpec {
        host_path: PathBuf::from("D:/kernel003/work"),
        guest_path: PathBuf::from("/workspace"),
        mode: BindMode::ReadWrite,
    }];
    job.net_policy = net_policy;
    job.resource_limits = ResourceLimits {
        memory_bytes: Some(134_217_728),
        cpu_cores: Some(2),
        timeout_ms: None,
    };
    job.metadata = BTreeMap::from([("source".to_string(), "kernel_003_parity".to_string())]);
    job
}

fn capabilities(adapter_id: &str, stdio: ThroughputClass) -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(adapter_id),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: stdio,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}
