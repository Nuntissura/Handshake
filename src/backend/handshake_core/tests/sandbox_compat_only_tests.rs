use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use handshake_core::sandbox::{
    build_registry_from_adapters, select, AdapterCapabilities, AdapterId, BindMode, Command,
    ExecResult, GpuPassthrough, ImageRef, IsolationStrength, IsolationTier, NetPolicy,
    ProcessHandle, ProcessSpec, ProcessStatus, RequiredCapability, ResourceLimits, SandboxAdapter,
    SandboxAdapterError, SandboxDefaultAdapterChoice, SandboxSelectionFailure, SandboxSettings,
    Signal, ThroughputClass, TrustClass, WindowsNativeJailAdapter, DOCKER_ADAPTER_ID,
    WSL2_PODMAN_ADAPTER_ID,
};

#[derive(Debug, Clone)]
struct StubAdapter {
    capabilities: AdapterCapabilities,
}

impl StubAdapter {
    fn new(capabilities: AdapterCapabilities) -> Self {
        Self { capabilities }
    }

    fn unavailable(&self) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: self.capabilities.adapter_id.clone(),
            reason: "compat-only test adapter has no runtime backend".to_string(),
        }
    }
}

#[async_trait]
impl SandboxAdapter for StubAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        Err(self.unavailable())
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
fn sandbox_settings_default_is_wsl2_podman_without_docker_opt_in() {
    let settings = SandboxSettings::default();

    assert_eq!(
        settings.default_adapter,
        SandboxDefaultAdapterChoice::Wsl2Podman
    );
    assert_eq!(
        settings.default_adapter.adapter_id(),
        AdapterId::new(WSL2_PODMAN_ADAPTER_ID)
    );
    assert!(!settings.docker_explicit_opt_in);
}

#[test]
fn sandbox_settings_rejects_docker_as_default_adapter() {
    let error = serde_json::from_str::<SandboxSettings>(
        r#"{"default_adapter":"docker","docker_explicit_opt_in":true}"#,
    )
    .expect_err("docker must not deserialize as a default adapter choice");

    let message = error.to_string();
    assert!(message.contains("unknown variant"), "{message}");
    assert!(message.contains("docker"), "{message}");
    assert!(message.contains("wsl2_podman"), "{message}");
}

#[test]
fn sandbox_settings_rejects_windows_native_jail_as_default_adapter() {
    let error = serde_json::from_str::<SandboxSettings>(
        r#"{"default_adapter":"windows_native_jail","docker_explicit_opt_in":false}"#,
    )
    .expect_err("windows native jail must not deserialize as a global default adapter choice");

    let message = error.to_string();
    assert!(message.contains("unknown variant"), "{message}");
    assert!(message.contains("windows_native_jail"), "{message}");
    assert!(message.contains("wsl2_podman"), "{message}");
}

#[test]
fn sandbox_settings_rejects_unknown_fields() {
    let error = serde_json::from_str::<SandboxSettings>(
        r#"{"default_adapter":"wsl2_podman","docker_explicit_opt_in":false,"default_adapter_id":"docker"}"#,
    )
    .expect_err("settings schema must fail closed on unknown default knobs");

    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn bootstrap_refuses_docker_as_preferred_default_even_when_opted_in() {
    let error = match build_registry_from_adapters(
        AdapterId::new(DOCKER_ADAPTER_ID),
        vec![adapter(docker_caps())],
        true,
    ) {
        Ok(_) => panic!("docker must never be accepted as registry default"),
        Err(error) => error,
    };

    assert_adapter_unavailable_reason(error, "docker cannot be the default sandbox adapter");
}

#[test]
fn bootstrap_refuses_windows_native_jail_as_preferred_default_even_when_registered() {
    let error = match build_registry_from_adapters(
        AdapterId::new("windows_native_jail"),
        vec![adapter(windows_native_jail_caps())],
        true,
    ) {
        Ok(registry) => panic!(
            "windows native jail must never be accepted as registry default, selected {}",
            registry.default_adapter_id()
        ),
        Err(error) => error,
    };

    assert_adapter_unavailable_reason(error, "cannot be the default sandbox adapter");
}

#[test]
fn bootstrap_does_not_fallback_to_docker_when_preferred_default_is_unavailable() {
    let error = match build_registry_from_adapters(
        AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        vec![adapter(docker_caps())],
        true,
    ) {
        Ok(_) => panic!("docker-only availability must not become implicit default fallback"),
        Err(error) => error,
    };

    assert_adapter_unavailable_reason(error, "no implicit default sandbox adapter");
}

#[test]
fn bootstrap_refuses_windows_native_jail_as_implicit_default_fallback() {
    let error = match build_registry_from_adapters(
        AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        vec![adapter(docker_caps()), adapter(windows_native_jail_caps())],
        true,
    ) {
        Ok(registry) => panic!(
            "windows native jail must require explicit selection, selected {}",
            registry.default_adapter_id()
        ),
        Err(error) => error,
    };

    assert_adapter_unavailable_reason(error, "no implicit default sandbox adapter");
}

#[test]
fn docker_override_remains_explicit_opt_in_only() {
    let registry = build_registry_from_adapters(
        AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        vec![adapter(wsl2_caps()), adapter(docker_caps())],
        false,
    )
    .expect("registry with docker compat adapter");

    let error = match select(
        &registry,
        &process_spec(BTreeSet::new()),
        Some(&AdapterId::new(DOCKER_ADAPTER_ID)),
    ) {
        Ok(adapter) => panic!(
            "docker override should require explicit opt-in, selected {}",
            adapter.capabilities().adapter_id
        ),
        Err(error) => error,
    };

    assert_eq!(error, SandboxSelectionFailure::DockerNotExplicitlyOptedIn);
}

fn assert_adapter_unavailable_reason(error: SandboxAdapterError, expected: &str) {
    match error {
        SandboxAdapterError::AdapterUnavailable { reason, .. } => {
            assert!(reason.contains(expected), "{reason}");
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

fn adapter(capabilities: AdapterCapabilities) -> Arc<dyn SandboxAdapter> {
    Arc::new(StubAdapter::new(capabilities))
}

fn process_spec(required_capabilities: BTreeSet<RequiredCapability>) -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new("compat-only-spec"),
        image_or_root: ImageRef::new("local-root"),
        cmd: vec!["true".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        required_capabilities,
        trust_class: TrustClass::Trusted,
        metadata: BTreeMap::new(),
    }
}

fn wsl2_caps() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::High,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

fn docker_caps() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::VeryStrong,
        network_isolation_strength: IsolationStrength::VeryStrong,
        gpu_passthrough: GpuPassthrough::VendorAgnostic,
        stdio_throughput_class: ThroughputClass::High,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

fn windows_native_jail_caps() -> AdapterCapabilities {
    WindowsNativeJailAdapter::target_capability_contract()
}
