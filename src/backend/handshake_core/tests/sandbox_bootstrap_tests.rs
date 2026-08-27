use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use handshake_core::sandbox::{
    build_registry_from_adapters, build_startup_recovery_registry_from_adapters,
    AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass, WindowsNativeJailAdapter,
    CLOUD_HYPERVISOR_ADAPTER_ID, DOCKER_ADAPTER_ID, GVISOR_ADAPTER_ID,
    WINDOWS_NATIVE_JAIL_ADAPTER_ID, WSL2_PODMAN_ADAPTER_ID,
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
            reason: "bootstrap stub has no runtime backend".to_string(),
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
fn sandbox_bootstrap_tests_prefers_wsl2_default_when_registered() {
    let registry = build_registry_from_adapters(
        AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        vec![adapter(wsl2_caps()), adapter(docker_caps())],
        false,
    )
    .expect("registry builds from static adapters");

    assert_eq!(
        registry.default_adapter_id().as_str(),
        WSL2_PODMAN_ADAPTER_ID
    );
    assert_eq!(registry.list().len(), 2);
    assert!(!registry.docker_explicit_opt_in());
}

#[test]
fn sandbox_bootstrap_tests_refuses_docker_only_default_fallback() {
    let error = match build_registry_from_adapters(
        AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        vec![adapter(docker_caps())],
        false,
    ) {
        Ok(_) => panic!("docker-only registry must fail closed instead of becoming default"),
        Err(error) => error,
    };

    match error {
        SandboxAdapterError::AdapterUnavailable { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WSL2_PODMAN_ADAPTER_ID));
            assert!(reason.contains("no implicit default sandbox adapter"));
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

#[test]
fn sandbox_bootstrap_tests_refuses_windows_native_jail_as_preferred_default() {
    let error = match build_registry_from_adapters(
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        vec![adapter(windows_native_jail_caps())],
        false,
    ) {
        Ok(registry) => panic!(
            "windows native jail must not become global default, selected {}",
            registry.default_adapter_id()
        ),
        Err(error) => error,
    };

    match error {
        SandboxAdapterError::AdapterUnavailable { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
            assert!(
                reason.contains("cannot be the default sandbox adapter"),
                "{reason}"
            );
            assert!(reason.contains("MT-045 approval"), "{reason}");
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

#[test]
fn sandbox_bootstrap_tests_refuses_windows_native_jail_as_implicit_fallback() {
    let error = match build_registry_from_adapters(
        AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        vec![adapter(docker_caps()), adapter(windows_native_jail_caps())],
        false,
    ) {
        Ok(registry) => panic!(
            "windows native jail must not become implicit default, selected {}",
            registry.default_adapter_id()
        ),
        Err(error) => error,
    };

    match error {
        SandboxAdapterError::AdapterUnavailable { adapter_id, reason } => {
            assert_eq!(adapter_id, AdapterId::new(WSL2_PODMAN_ADAPTER_ID));
            assert!(
                reason.contains("no implicit default sandbox adapter"),
                "{reason}"
            );
            assert!(reason.contains("windows_native_jail"), "{reason}");
        }
        other => panic!("expected AdapterUnavailable, got {other:?}"),
    }
}

#[test]
fn sandbox_bootstrap_tests_zero_adapters_boot_empty_and_fail_closed_on_selection() {
    // WP-KERNEL-005 contract update: app startup must not depend on any
    // outside-app sandbox runtime being installed, so an empty adapter set no
    // longer hard-fails bootstrap. Fail-closed moves to the selection boundary:
    // the registry boots empty and resolving the default adapter yields None,
    // so any sandbox job selection fails closed.
    let registry =
        build_registry_from_adapters(AdapterId::new(WSL2_PODMAN_ADAPTER_ID), Vec::new(), false)
            .expect("zero adapters must not block app startup (no outside-app dependency)");

    assert!(registry.list().is_empty());
    assert_eq!(
        registry.default_adapter_id().as_str(),
        WSL2_PODMAN_ADAPTER_ID
    );
    assert!(
        registry.get(registry.default_adapter_id()).is_none(),
        "selecting the default adapter from an empty registry must fail closed"
    );
}

#[test]
fn startup_recovery_registry_accepts_docker_only_for_exact_lookup() {
    assert_explicit_only_recovery_registry(docker_caps());
}

#[test]
fn startup_recovery_registry_accepts_windows_native_only_for_exact_lookup() {
    assert_explicit_only_recovery_registry(windows_native_jail_caps());
}

#[test]
fn startup_recovery_registry_accepts_gvisor_only_for_exact_lookup() {
    assert_explicit_only_recovery_registry(explicit_only_caps(
        GVISOR_ADAPTER_ID,
        IsolationTier::Tier2Syscall,
    ));
}

#[test]
fn startup_recovery_registry_accepts_cloud_hypervisor_only_for_exact_lookup() {
    assert_explicit_only_recovery_registry(explicit_only_caps(
        CLOUD_HYPERVISOR_ADAPTER_ID,
        IsolationTier::Tier3Microvm,
    ));
}

#[test]
fn ordinary_bootstrap_still_refuses_isolation_tier_only_default_fallbacks() {
    for capabilities in [
        explicit_only_caps(GVISOR_ADAPTER_ID, IsolationTier::Tier2Syscall),
        explicit_only_caps(CLOUD_HYPERVISOR_ADAPTER_ID, IsolationTier::Tier3Microvm),
    ] {
        let explicit_only_id = capabilities.adapter_id.clone();
        let error = match build_registry_from_adapters(
            AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            vec![adapter(capabilities)],
            false,
        ) {
            Ok(registry) => panic!(
                "{explicit_only_id} must not become execution default, selected {}",
                registry.default_adapter_id()
            ),
            Err(error) => error,
        };

        assert!(
            matches!(
                &error,
                SandboxAdapterError::AdapterUnavailable { reason, .. }
                    if reason.contains("no implicit default sandbox adapter")
            ),
            "unexpected {explicit_only_id} bootstrap result: {error:?}"
        );
    }
}

#[test]
fn startup_recovery_registry_accepts_empty_available_adapter_set() {
    let registry = build_startup_recovery_registry_from_adapters(Vec::new(), false);

    assert!(registry.list().is_empty());
    assert_eq!(
        registry.default_adapter_id(),
        &AdapterId::new(WSL2_PODMAN_ADAPTER_ID)
    );
    assert!(registry.get(registry.default_adapter_id()).is_none());
}

fn assert_explicit_only_recovery_registry(capabilities: AdapterCapabilities) {
    let expected_id = capabilities.adapter_id.clone();
    let registry =
        build_startup_recovery_registry_from_adapters(vec![adapter(capabilities)], false);

    assert_eq!(registry.list().len(), 1);
    assert_eq!(registry.list()[0].adapter_id, expected_id);
    assert!(
        registry.get(&expected_id).is_some(),
        "recovery must retain exact lookup for {expected_id}"
    );
    assert_eq!(
        registry.default_adapter_id(),
        &AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        "recovery lookup must not promote {expected_id} to execution default"
    );
}

fn adapter(capabilities: AdapterCapabilities) -> Arc<dyn SandboxAdapter> {
    Arc::new(StubAdapter::new(capabilities))
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
        supports_persistent_exec: false,
        supports_warm_agent: false,
        supports_live_token_stream: false,
    }
}

fn docker_caps() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
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
        supports_persistent_exec: false,
        supports_warm_agent: false,
        supports_live_token_stream: false,
    }
}

fn windows_native_jail_caps() -> AdapterCapabilities {
    WindowsNativeJailAdapter::target_capability_contract()
}

fn explicit_only_caps(adapter_id: &str, isolation_tier: IsolationTier) -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(adapter_id),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::High,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier,
        requires_nested_virt: matches!(isolation_tier, IsolationTier::Tier3Microvm),
        supports_snapshot: false,
        supports_persistent_exec: false,
        supports_warm_agent: false,
        supports_live_token_stream: false,
    }
}
