use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::types::{
    AdapterId, BindMode, Command, ExecResult, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapterError, Signal,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationStrength {
    Weak,
    Strong,
    VeryStrong,
}

/// Master Spec v02.187 §3.5.3 strong-isolation tier ladder.
///
/// Tiers are ordered by escape-resistance strength: a container namespace
/// jail (Tier 1) is weaker than a syscall-filtering substrate (Tier 2),
/// which is weaker than a hardware-virtualized microVM (Tier 3). Selection
/// uses [`IsolationTier::rank`] to compare a candidate adapter's tier
/// against the minimum tier a workload's [`TrustClass`] demands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationTier {
    /// OS-level container / namespace isolation (Docker, Podman, AppContainer).
    Tier1Container,
    /// Syscall-filtering / user-space kernel substrate (e.g. gVisor-class).
    Tier2Syscall,
    /// Hardware-virtualized microVM (e.g. Firecracker-class).
    Tier3Microvm,
}

impl IsolationTier {
    /// Comparable strength rank: 1 (weakest) .. 3 (strongest).
    pub fn rank(self) -> u8 {
        match self {
            Self::Tier1Container => 1,
            Self::Tier2Syscall => 2,
            Self::Tier3Microvm => 3,
        }
    }
}

/// Master Spec v02.187 §3.5.4 trust classification for a workload.
///
/// The trust class determines the minimum isolation tier a workload may run
/// under (see [`TrustClass::min_isolation_tier`]). The default is the most
/// conservative class, [`TrustClass::UntrustedAgent`], so that any spec built
/// without an explicit trust decision is treated as hostile until proven
/// otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustClass {
    /// First-party, operator-trusted workloads.
    Trusted,
    /// Workloads that passed human/automated review.
    Reviewed,
    /// Untrusted agent-authored or external workloads (safe default).
    UntrustedAgent,
}

impl Default for TrustClass {
    fn default() -> Self {
        Self::UntrustedAgent
    }
}

impl TrustClass {
    /// Minimum isolation tier this trust class is permitted to run under,
    /// per Master Spec v02.187 §3.5.4.
    pub fn min_isolation_tier(self) -> IsolationTier {
        match self {
            Self::Trusted | Self::Reviewed => IsolationTier::Tier1Container,
            Self::UntrustedAgent => IsolationTier::Tier3Microvm,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuPassthrough {
    None,
    NvidiaCuda,
    VendorAgnostic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThroughputClass {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    pub adapter_id: AdapterId,
    #[serde(default)]
    pub runtime_available: bool,
    pub filesystem_isolation_strength: IsolationStrength,
    pub network_isolation_strength: IsolationStrength,
    pub gpu_passthrough: GpuPassthrough,
    pub stdio_throughput_class: ThroughputClass,
    pub win32_native_fidelity: bool,
    pub cross_machine_portable: bool,
    pub isolation_tier: IsolationTier,
    #[serde(default)]
    pub requires_nested_virt: bool,
    #[serde(default)]
    pub supports_snapshot: bool,
}

impl AdapterCapabilities {
    pub fn default_no_op_capabilities() -> Self {
        default_no_op_capabilities()
    }
}

pub fn default_no_op_capabilities() -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new("noop"),
        runtime_available: false,
        filesystem_isolation_strength: IsolationStrength::Weak,
        network_isolation_strength: IsolationStrength::Weak,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::Low,
        win32_native_fidelity: false,
        cross_machine_portable: false,
        isolation_tier: IsolationTier::Tier1Container,
        requires_nested_virt: false,
        supports_snapshot: false,
    }
}

#[async_trait]
pub trait SandboxAdapter: Send + Sync {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError>;

    async fn exec(
        &self,
        handle: &ProcessHandle,
        cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError>;

    async fn fs_bind(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
        mode: BindMode,
    ) -> Result<(), SandboxAdapterError>;

    async fn net_policy(
        &self,
        handle: &ProcessHandle,
        policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError>;

    async fn kill(&self, handle: &ProcessHandle, signal: Signal)
        -> Result<(), SandboxAdapterError>;

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError>;

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError>;

    fn capabilities(&self) -> AdapterCapabilities;
}
