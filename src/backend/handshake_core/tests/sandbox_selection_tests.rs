use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use handshake_core::sandbox::{
    select, AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    ImageRef, IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec,
    ProcessStatus, RequiredCapability, ResourceLimits, SandboxAdapter, SandboxAdapterError,
    SandboxAdapterRegistry, SandboxSelectionFailure, Signal, ThroughputClass, TrustClass,
    WindowsNativeJailAdapter, DOCKER_ADAPTER_ID, SANDBOX_SELECTION_FAILURE_EVENT_FAMILY,
    WINDOWS_NATIVE_JAIL_ADAPTER_ID,
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
            reason: "stub adapter has no isolation backend".to_string(),
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
fn default_adapter_is_returned_when_no_override_and_no_required_capabilities() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));

    let selected = select(&registry, &process_spec(BTreeSet::new()), None).expect("select default");

    assert_eq!(
        selected.capabilities().adapter_id,
        AdapterId::new("wsl2_podman")
    );
}

#[test]
fn win32_native_fidelity_fails_until_windows_native_runtime_capabilities_are_available() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(windows_native_jail_target_capabilities()));

    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::from([RequiredCapability::Win32NativeFidelity])),
        None,
    ));

    match error {
        SandboxSelectionFailure::CapabilityUnsatisfied { required, .. } => {
            assert_eq!(required, vec![RequiredCapability::Win32NativeFidelity]);
        }
        other => panic!("expected CapabilityUnsatisfied, got {other:?}"),
    }
}

#[test]
fn windows_native_jail_does_not_overclaim_network_or_stdio_strength() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(windows_native_jail_target_capabilities()));

    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::from([
            RequiredCapability::Win32NativeFidelity,
            RequiredCapability::VeryStrongNetworkIsolation,
            RequiredCapability::HighStdioThroughput,
        ])),
        None,
    ));

    match error {
        SandboxSelectionFailure::CapabilityUnsatisfied { required, .. } => {
            assert_eq!(
                required,
                vec![
                    RequiredCapability::Win32NativeFidelity,
                    RequiredCapability::VeryStrongNetworkIsolation,
                    RequiredCapability::HighStdioThroughput
                ]
            );
        }
        other => panic!("expected CapabilityUnsatisfied, got {other:?}"),
    }
}

#[test]
fn unavailable_windows_native_jail_is_not_selected_for_win32_native_fidelity() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(Arc::new(
        WindowsNativeJailAdapter::unavailable_for_current_host(),
    ));

    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::from([RequiredCapability::Win32NativeFidelity])),
        None,
    ));

    match error {
        SandboxSelectionFailure::CapabilityUnsatisfied {
            required,
            available_by_adapter,
        } => {
            assert_eq!(required, vec![RequiredCapability::Win32NativeFidelity]);
            assert!(
                !available_by_adapter
                    .get(&AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID))
                    .expect("windows native jail capabilities included")
                    .win32_native_fidelity
            );
        }
        other => panic!("expected CapabilityUnsatisfied, got {other:?}"),
    }
}

#[test]
fn unavailable_windows_native_jail_override_fails_even_without_required_capabilities() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(Arc::new(
        WindowsNativeJailAdapter::unavailable_for_current_host(),
    ));

    let windows_native_jail_id = AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID);
    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::new()),
        Some(&windows_native_jail_id),
    ));

    assert_eq!(
        error,
        SandboxSelectionFailure::OverrideCapabilityMismatch {
            override_id: windows_native_jail_id,
            required: vec![RequiredCapability::Win32NativeFidelity],
        }
    );
}

#[test]
fn unavailable_windows_native_jail_default_fails_even_without_required_capabilities() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
    registry.register(Arc::new(
        WindowsNativeJailAdapter::unavailable_for_current_host(),
    ));

    let error = expect_selection_error(select(&registry, &process_spec(BTreeSet::new()), None));

    match error {
        SandboxSelectionFailure::CapabilityUnsatisfied { required, .. } => {
            assert_eq!(required, vec![RequiredCapability::Win32NativeFidelity]);
        }
        other => panic!("expected CapabilityUnsatisfied, got {other:?}"),
    }
}

#[test]
fn registry_list_exposes_approved_windows_native_jail_runtime_availability() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(
        fake_runtime_available_windows_native_jail_capabilities(),
    ));

    let listed = registry.list();
    let windows_caps = listed
        .iter()
        .find(|caps| caps.adapter_id == AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID))
        .expect("windows native jail should be listed");

    assert!(windows_caps.runtime_available);
    assert!(windows_caps.win32_native_fidelity);
    assert_eq!(
        windows_caps.filesystem_isolation_strength,
        IsolationStrength::VeryStrong
    );
    assert_eq!(
        windows_caps.network_isolation_strength,
        IsolationStrength::VeryStrong
    );
    assert_eq!(windows_caps.stdio_throughput_class, ThroughputClass::High);
}

#[test]
fn registry_get_returns_approved_windows_native_jail_runtime_adapter() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(
        fake_runtime_available_windows_native_jail_capabilities(),
    ));

    let adapter = registry
        .get(&AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID))
        .expect("windows native jail should be registered");
    let capabilities = adapter.capabilities();

    assert!(capabilities.runtime_available);
    assert!(capabilities.win32_native_fidelity);
}

#[test]
fn selection_accepts_runtime_available_windows_native_jail_after_mt045_approval() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(
        fake_runtime_available_windows_native_jail_capabilities(),
    ));

    let selected = select(
        &registry,
        &process_spec(BTreeSet::from([RequiredCapability::Win32NativeFidelity])),
        None,
    )
    .expect("approved runtime windows-native adapter should satisfy Win32 fidelity");

    assert_eq!(
        selected.capabilities().adapter_id,
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID)
    );
}

#[test]
fn docker_selection_fails_without_explicit_opt_in() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(docker_capabilities()));

    let docker_id = AdapterId::new(DOCKER_ADAPTER_ID);
    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::new()),
        Some(&docker_id),
    ));

    assert_eq!(error, SandboxSelectionFailure::DockerNotExplicitlyOptedIn);
    assert_eq!(
        error
            .to_event_payload(&process_spec(BTreeSet::new()))
            .event_family,
        SANDBOX_SELECTION_FAILURE_EVENT_FAMILY
    );

    registry.set_docker_explicit_opt_in(true);
    let selected =
        select(&registry, &process_spec(BTreeSet::new()), Some(&docker_id)).expect("select docker");
    assert_eq!(selected.capabilities().adapter_id, docker_id);
}

#[test]
fn capability_mismatch_emits_capability_unsatisfied_without_silent_fallback() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));

    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::from([
            RequiredCapability::VeryStrongNetworkIsolation,
            RequiredCapability::HighStdioThroughput,
        ])),
        None,
    ));

    match error {
        SandboxSelectionFailure::CapabilityUnsatisfied {
            required,
            available_by_adapter,
        } => {
            assert_eq!(
                required,
                vec![
                    RequiredCapability::VeryStrongNetworkIsolation,
                    RequiredCapability::HighStdioThroughput
                ]
            );
            assert_eq!(
                available_by_adapter
                    .get(&AdapterId::new("wsl2_podman"))
                    .expect("wsl2 capabilities included")
                    .adapter_id,
                AdapterId::new("wsl2_podman")
            );
        }
        other => panic!("expected CapabilityUnsatisfied, got {other:?}"),
    }
}

#[test]
fn work_profile_override_forces_specific_adapter_or_fails_loud() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(high_stdio_capabilities()));

    let high_stdio_id = AdapterId::new("high_stdio_adapter");
    let selected = select(
        &registry,
        &process_spec(BTreeSet::new()),
        Some(&high_stdio_id),
    )
    .expect("override selects high stdio adapter");
    assert_eq!(selected.capabilities().adapter_id, high_stdio_id);

    let wsl2_id = AdapterId::new("wsl2_podman");
    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::from([RequiredCapability::HighStdioThroughput])),
        Some(&wsl2_id),
    ));

    assert_eq!(
        error,
        SandboxSelectionFailure::OverrideCapabilityMismatch {
            override_id: wsl2_id,
            required: vec![RequiredCapability::HighStdioThroughput],
        }
    );
}

#[test]
fn work_profile_override_fails_loud_when_adapter_is_not_registered() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));

    let missing_id = AdapterId::new("missing_adapter");
    let error = expect_selection_error(select(
        &registry,
        &process_spec(BTreeSet::new()),
        Some(&missing_id),
    ));

    assert_eq!(
        error,
        SandboxSelectionFailure::AdapterNotRegistered {
            adapter_id: missing_id,
        }
    );
}

#[test]
fn capability_mapping_covers_gpu_portability_isolation_and_stdio_requirements() {
    let all_caps_id = AdapterId::new("all_caps");
    let mut registry = SandboxAdapterRegistry::new(all_caps_id.clone());
    registry.register(adapter(all_capabilities(all_caps_id.clone())));

    let selected = select(
        &registry,
        &process_spec(BTreeSet::from([
            RequiredCapability::NvidiaCudaPassthrough,
            RequiredCapability::VendorAgnosticGpu,
            RequiredCapability::CrossMachinePortable,
            RequiredCapability::VeryStrongFilesystemIsolation,
            RequiredCapability::VeryStrongNetworkIsolation,
            RequiredCapability::HighStdioThroughput,
        ])),
        None,
    )
    .expect("all capabilities satisfy every non-win32 requirement");
    assert_eq!(selected.capabilities().adapter_id, all_caps_id);

    let vendor_gpu_id = AdapterId::new("vendor_gpu");
    let mut vendor_registry = SandboxAdapterRegistry::new(vendor_gpu_id.clone());
    vendor_registry.register(adapter(vendor_gpu_capabilities(vendor_gpu_id.clone())));

    let vendor_selected = select(
        &vendor_registry,
        &process_spec(BTreeSet::from([RequiredCapability::VendorAgnosticGpu])),
        None,
    )
    .expect("vendor-agnostic GPU accepts any GPU passthrough");
    assert_eq!(vendor_selected.capabilities().adapter_id, vendor_gpu_id);

    let error = expect_selection_error(select(
        &vendor_registry,
        &process_spec(BTreeSet::from([RequiredCapability::NvidiaCudaPassthrough])),
        None,
    ));
    assert!(matches!(
        error,
        SandboxSelectionFailure::CapabilityUnsatisfied { .. }
    ));
}

#[test]
#[should_panic(expected = "duplicate sandbox adapter registration")]
fn duplicate_register_panics() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(wsl2_podman_capabilities()));
}

#[test]
fn sandbox_registry_and_selection_have_no_adapter_specific_imports() {
    for module in ["registry.rs", "selection.rs"] {
        let module_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("sandbox")
            .join(module);
        let source = fs::read_to_string(module_path).expect("read sandbox selection module");
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
                "sandbox selection must not import adapter-specific crate surface `{banned}`"
            );
        }
    }
}

#[test]
fn trusted_workload_selects_tier1_adapter_ok() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));

    let spec = trust_classed_spec(TrustClass::Trusted);
    let selected = select(&registry, &spec, None).expect("trusted workload accepts tier-1 adapter");

    assert_eq!(
        selected.capabilities().adapter_id,
        AdapterId::new("wsl2_podman")
    );
    assert_eq!(
        selected.capabilities().isolation_tier,
        IsolationTier::Tier1Container
    );
}

#[test]
fn reviewed_workload_selects_tier1_adapter_ok() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));

    let spec = trust_classed_spec(TrustClass::Reviewed);
    let selected =
        select(&registry, &spec, None).expect("reviewed workload accepts tier-1 adapter");

    assert_eq!(
        selected.capabilities().adapter_id,
        AdapterId::new("wsl2_podman")
    );
}

#[test]
fn untrusted_agent_workload_fails_isolation_tier_minimum_when_only_tier1_exists() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));

    let spec = trust_classed_spec(TrustClass::UntrustedAgent);
    let error = expect_selection_error(select(&registry, &spec, None));

    match error {
        SandboxSelectionFailure::IsolationTierUnsatisfied {
            required_tier,
            available_tier,
            ref reason,
            ..
        } => {
            assert_eq!(required_tier, IsolationTier::Tier3Microvm);
            assert_eq!(available_tier, IsolationTier::Tier1Container);
            assert!(
                reason.contains("tier") || reason.contains("Tier"),
                "failure reason must name the tier requirement, got: {reason}"
            );
        }
        other => panic!("expected IsolationTierUnsatisfied, got {other:?}"),
    }
}

#[test]
fn untrusted_agent_isolation_tier_failure_routes_through_selection_event() {
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));

    let spec = trust_classed_spec(TrustClass::UntrustedAgent);
    let error = expect_selection_error(select(&registry, &spec, None));
    let event = error.to_event_payload(&spec);

    assert_eq!(event.event_family, SANDBOX_SELECTION_FAILURE_EVENT_FAMILY);
    assert!(matches!(
        event.failure,
        SandboxSelectionFailure::IsolationTierUnsatisfied { .. }
    ));
}

#[test]
fn untrusted_agent_workload_selects_tier3_microvm_when_available() {
    // End-to-end of increments 1+2: with a Tier-3 microVM adapter registered
    // alongside the Tier-1 default, an untrusted-agent workload MUST be routed
    // to the Tier-3 adapter — not rejected, and never downgraded to Tier-1.
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities())); // Tier-1 (registry default)
    registry.register(adapter(microvm_tier3_capabilities())); // Tier-3 microVM

    let spec = trust_classed_spec(TrustClass::UntrustedAgent);
    let selected = select(&registry, &spec, None)
        .expect("untrusted-agent workload must select the Tier-3 microVM adapter when present");

    assert_eq!(
        selected.capabilities().isolation_tier,
        IsolationTier::Tier3Microvm
    );
    assert_eq!(
        selected.capabilities().adapter_id,
        AdapterId::new("cloud_hypervisor")
    );
}

#[test]
fn trusted_workload_still_prefers_tier1_even_when_tier3_present() {
    // A trusted workload does not need (and should not pay for) a microVM:
    // with both tiers available it stays on the Tier-1 default.
    let mut registry = SandboxAdapterRegistry::new(AdapterId::new("wsl2_podman"));
    registry.register(adapter(wsl2_podman_capabilities()));
    registry.register(adapter(microvm_tier3_capabilities()));

    let spec = trust_classed_spec(TrustClass::Trusted);
    let selected = select(&registry, &spec, None).expect("trusted workload selects an adapter");
    assert_eq!(
        selected.capabilities().isolation_tier,
        IsolationTier::Tier1Container
    );
}

fn microvm_tier3_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new("cloud_hypervisor"),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::VeryStrong,
        network_isolation_strength: IsolationStrength::VeryStrong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::Medium,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier3Microvm,
        requires_nested_virt: true,
        supports_snapshot: false,
    }
}

fn trust_classed_spec(trust_class: TrustClass) -> ProcessSpec {
    let mut spec = process_spec(BTreeSet::new());
    spec.trust_class = trust_class;
    spec
}

fn expect_selection_error(
    result: Result<Arc<dyn SandboxAdapter>, SandboxSelectionFailure>,
) -> SandboxSelectionFailure {
    match result {
        Ok(adapter) => panic!(
            "expected selection failure, selected {}",
            adapter.capabilities().adapter_id
        ),
        Err(error) => error,
    }
}

fn adapter(capabilities: AdapterCapabilities) -> Arc<dyn SandboxAdapter> {
    Arc::new(StubAdapter::new(capabilities))
}

fn process_spec(required_capabilities: BTreeSet<RequiredCapability>) -> ProcessSpec {
    ProcessSpec {
        id: AdapterId::new("spec-1"),
        image_or_root: ImageRef::new("local-root"),
        cmd: vec!["true".to_string()],
        env: BTreeMap::new(),
        cwd: None,
        binds: Vec::new(),
        net_policy: NetPolicy::DenyAll,
        resource_limits: ResourceLimits::default(),
        required_capabilities,
        // These legacy fixtures exercise capability matching, not the trust
        // tier. Mark them Trusted so they still select the available Tier-1
        // adapters; the dedicated trust-tier suite covers the guard.
        trust_class: TrustClass::Trusted,
        metadata: BTreeMap::new(),
    }
}

fn wsl2_podman_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new("wsl2_podman"),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::Medium,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

fn windows_native_jail_target_capabilities() -> AdapterCapabilities {
    WindowsNativeJailAdapter::target_capability_contract()
}

fn fake_runtime_available_windows_native_jail_capabilities() -> AdapterCapabilities {
    let mut capabilities = all_capabilities(AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID));
    capabilities.win32_native_fidelity = true;
    capabilities.cross_machine_portable = false;
    capabilities
}

fn docker_capabilities() -> AdapterCapabilities {
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

fn high_stdio_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new("high_stdio_adapter"),
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

fn all_capabilities(adapter_id: AdapterId) -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id,
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::VeryStrong,
        network_isolation_strength: IsolationStrength::VeryStrong,
        gpu_passthrough: GpuPassthrough::NvidiaCuda,
        stdio_throughput_class: ThroughputClass::High,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

fn vendor_gpu_capabilities(adapter_id: AdapterId) -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id,
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::VendorAgnostic,
        stdio_throughput_class: ThroughputClass::Medium,
        win32_native_fidelity: false,
        cross_machine_portable: true,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}
