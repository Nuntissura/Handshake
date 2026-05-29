use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    AdapterCapabilities, AdapterId, GpuPassthrough, IsolationStrength, IsolationTier, ProcessSpec,
    RequiredCapability, SandboxAdapter, SandboxAdapterRegistry, ThroughputClass, DOCKER_ADAPTER_ID,
    WINDOWS_NATIVE_JAIL_ADAPTER_ID, WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
};

pub const SANDBOX_SELECTION_FAILURE_EVENT_FAMILY: &str = "FR-EVT-SANDBOX-SELECT-FAIL";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum SandboxSelectionFailure {
    #[error("sandbox selection capability unsatisfied")]
    CapabilityUnsatisfied {
        required: Vec<RequiredCapability>,
        available_by_adapter: BTreeMap<AdapterId, AdapterCapabilities>,
    },
    #[error("sandbox adapter not registered: {adapter_id}")]
    AdapterNotRegistered { adapter_id: AdapterId },
    #[error("docker sandbox selection requires explicit opt-in")]
    DockerNotExplicitlyOptedIn,
    #[error("{reason}")]
    IsolationTierUnsatisfied {
        adapter_id: AdapterId,
        required_tier: IsolationTier,
        available_tier: IsolationTier,
        reason: String,
    },
    #[error("sandbox work profile override capability mismatch: {override_id}")]
    OverrideCapabilityMismatch {
        override_id: AdapterId,
        required: Vec<RequiredCapability>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxSelectionFailureEvent {
    pub event_family: String,
    pub process_spec_id: AdapterId,
    pub required_capabilities: Vec<RequiredCapability>,
    pub failure: SandboxSelectionFailure,
}

impl SandboxSelectionFailure {
    pub fn event_family(&self) -> &'static str {
        SANDBOX_SELECTION_FAILURE_EVENT_FAMILY
    }

    pub fn to_event_payload(&self, spec: &ProcessSpec) -> SandboxSelectionFailureEvent {
        SandboxSelectionFailureEvent {
            event_family: self.event_family().to_string(),
            process_spec_id: spec.id.clone(),
            required_capabilities: spec.required_capabilities.iter().copied().collect(),
            failure: self.clone(),
        }
    }
}

pub fn select(
    registry: &SandboxAdapterRegistry,
    spec: &ProcessSpec,
    work_profile_override: Option<&AdapterId>,
) -> Result<Arc<dyn SandboxAdapter>, SandboxSelectionFailure> {
    if let Some(override_id) = work_profile_override {
        return select_candidate(registry, spec, override_id, true);
    }

    if spec
        .required_capabilities
        .contains(&RequiredCapability::Win32NativeFidelity)
    {
        return select_candidate(
            registry,
            spec,
            &AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
            false,
        );
    }

    let default_id = registry.default_adapter_id().clone();
    select_candidate(registry, spec, &default_id, false)
}

fn select_candidate(
    registry: &SandboxAdapterRegistry,
    spec: &ProcessSpec,
    adapter_id: &AdapterId,
    is_override: bool,
) -> Result<Arc<dyn SandboxAdapter>, SandboxSelectionFailure> {
    let adapter =
        registry
            .get(adapter_id)
            .ok_or_else(|| SandboxSelectionFailure::AdapterNotRegistered {
                adapter_id: adapter_id.clone(),
            })?;

    if adapter_id.as_str() == DOCKER_ADAPTER_ID
        && (!is_override || !registry.docker_explicit_opt_in())
    {
        return Err(SandboxSelectionFailure::DockerNotExplicitlyOptedIn);
    }

    let capabilities = adapter.capabilities();
    let mut missing = missing_required_capabilities(spec, &capabilities);
    // The Windows-native adapter ID is meaningful only after MT-045 approves a
    // backend and runtime capabilities can prove actual Win32 fidelity.
    if adapter_id.as_str() == WINDOWS_NATIVE_JAIL_ADAPTER_ID
        && (!WINDOWS_NATIVE_JAIL_BACKEND_APPROVED
            || !capabilities.runtime_available
            || !capabilities.win32_native_fidelity)
        && !missing.contains(&RequiredCapability::Win32NativeFidelity)
    {
        missing.push(RequiredCapability::Win32NativeFidelity);
    }
    missing.sort();
    if missing.is_empty() {
        // Master Spec v02.187 §3.5.5: enforce the trust -> isolation-tier
        // MINIMUM. A capability match is necessary but not sufficient; the
        // chosen adapter's isolation tier must be at least as strong as the
        // tier the workload's trust class demands. Never silently downgrade.
        enforce_isolation_tier_minimum(spec, &capabilities)?;
        return Ok(adapter);
    }

    if is_override {
        return Err(SandboxSelectionFailure::OverrideCapabilityMismatch {
            override_id: adapter_id.clone(),
            required: missing,
        });
    }

    Err(SandboxSelectionFailure::CapabilityUnsatisfied {
        required: missing,
        available_by_adapter: available_by_adapter(registry),
    })
}

fn enforce_isolation_tier_minimum(
    spec: &ProcessSpec,
    capabilities: &AdapterCapabilities,
) -> Result<(), SandboxSelectionFailure> {
    let required_tier = spec.trust_class.min_isolation_tier();
    let available_tier = capabilities.isolation_tier;
    if available_tier.rank() >= required_tier.rank() {
        return Ok(());
    }

    Err(SandboxSelectionFailure::IsolationTierUnsatisfied {
        adapter_id: capabilities.adapter_id.clone(),
        required_tier,
        available_tier,
        reason: format!(
            "sandbox isolation tier insufficient for trust class {:?}: required minimum tier {:?} (rank {}), but adapter {} provides only tier {:?} (rank {}); refusing to downgrade isolation",
            spec.trust_class,
            required_tier,
            required_tier.rank(),
            capabilities.adapter_id,
            available_tier,
            available_tier.rank(),
        ),
    })
}

fn missing_required_capabilities(
    spec: &ProcessSpec,
    capabilities: &AdapterCapabilities,
) -> Vec<RequiredCapability> {
    spec.required_capabilities
        .iter()
        .copied()
        .filter(|required| !capability_satisfied(*required, capabilities))
        .collect()
}

fn capability_satisfied(required: RequiredCapability, capabilities: &AdapterCapabilities) -> bool {
    match required {
        RequiredCapability::Win32NativeFidelity => capabilities.win32_native_fidelity,
        RequiredCapability::NvidiaCudaPassthrough => {
            capabilities.gpu_passthrough == GpuPassthrough::NvidiaCuda
        }
        RequiredCapability::VendorAgnosticGpu => {
            capabilities.gpu_passthrough != GpuPassthrough::None
        }
        RequiredCapability::CrossMachinePortable => capabilities.cross_machine_portable,
        RequiredCapability::VeryStrongFilesystemIsolation => {
            capabilities.filesystem_isolation_strength == IsolationStrength::VeryStrong
        }
        RequiredCapability::VeryStrongNetworkIsolation => {
            capabilities.network_isolation_strength == IsolationStrength::VeryStrong
        }
        RequiredCapability::HighStdioThroughput => {
            capabilities.stdio_throughput_class == ThroughputClass::High
        }
    }
}

fn available_by_adapter(
    registry: &SandboxAdapterRegistry,
) -> BTreeMap<AdapterId, AdapterCapabilities> {
    registry
        .list()
        .into_iter()
        .map(|capabilities| (capabilities.adapter_id.clone(), capabilities))
        .collect()
}
