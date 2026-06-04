use std::path::PathBuf;

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::model_runtime::WarmAgentTransport;

use super::types::{
    AdapterId, BindMode, Command, ExecResult, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapterError, Signal, SnapshotRef,
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
    #[serde(default)]
    pub supports_persistent_exec: bool,
    #[serde(default)]
    pub supports_warm_agent: bool,
    #[serde(default)]
    pub supports_live_token_stream: bool,
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
        supports_persistent_exec: false,
        supports_warm_agent: false,
        supports_live_token_stream: false,
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

    /// Capture the full live state of a running sandbox into a restorable
    /// snapshot (Master Spec v02.187 §3.5.7 #7 — the validate-then-promote
    /// flow). Adapters that cannot pause-and-checkpoint a live instance — every
    /// adapter except the hardware-virtualized microVM tier today — keep the
    /// default, which returns a typed
    /// [`SandboxAdapterError::SnapshotUnsupported`]. Only adapters whose
    /// [`AdapterCapabilities::supports_snapshot`] is `true` override this.
    async fn snapshot(&self, handle: &ProcessHandle) -> Result<SnapshotRef, SandboxAdapterError> {
        let _ = handle;
        Err(SandboxAdapterError::SnapshotUnsupported {
            adapter_id: self.capabilities().adapter_id,
        })
    }

    /// Restore a previously captured snapshot into a fresh sandbox instance that
    /// resumes from the captured live state (no reboot). Mirrors [`snapshot`];
    /// the default returns [`SandboxAdapterError::SnapshotUnsupported`].
    ///
    /// [`snapshot`]: SandboxAdapter::snapshot
    async fn restore(&self, snapshot: &SnapshotRef) -> Result<ProcessHandle, SandboxAdapterError> {
        let _ = snapshot;
        Err(SandboxAdapterError::SnapshotUnsupported {
            adapter_id: self.capabilities().adapter_id,
        })
    }

    /// Delete a previously captured snapshot. Adapters that persist snapshots
    /// outside the process lifetime override this so callers can clean up a
    /// successful capture if a later promotion/ledger step fails.
    async fn delete_snapshot(&self, snapshot: &SnapshotRef) -> Result<(), SandboxAdapterError> {
        let _ = snapshot;
        Err(SandboxAdapterError::SnapshotUnsupported {
            adapter_id: self.capabilities().adapter_id,
        })
    }

    /// Copy a file/directory from the host into the running sandbox at
    /// `guest_path` (Master Spec v02.187 §3.5.7 #4 — first-class filesystem
    /// namespace; callers must never shell out to `cp`/`cat` themselves).
    /// Adapters with a live, host-reachable guest filesystem (e.g. a persistent
    /// container) override this; the default is a typed
    /// [`SandboxAdapterError::CopyUnsupported`] for adapters whose isolation
    /// model has no live per-file channel (use `fs_bind` there instead).
    async fn copy_in(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        let _ = (handle, host_path, guest_path);
        Err(SandboxAdapterError::CopyUnsupported {
            adapter_id: self.capabilities().adapter_id,
        })
    }

    /// Copy a file/directory out of the running sandbox at `guest_path` to the
    /// host `host_path` (§3.5.7 #4). Mirrors [`copy_in`]; the default returns
    /// [`SandboxAdapterError::CopyUnsupported`].
    ///
    /// [`copy_in`]: SandboxAdapter::copy_in
    async fn copy_out(
        &self,
        handle: &ProcessHandle,
        guest_path: PathBuf,
        host_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        let _ = (handle, guest_path, host_path);
        Err(SandboxAdapterError::CopyUnsupported {
            adapter_id: self.capabilities().adapter_id,
        })
    }

    /// Return a live warm-agent transport for an already spawned/restored
    /// persistent VM handle. The default is fail-closed: generic sandbox command
    /// execution is not enough to claim warm model streaming.
    async fn warm_agent_transport(
        &self,
        handle: &ProcessHandle,
    ) -> Result<Arc<dyn WarmAgentTransport>, SandboxAdapterError> {
        let _ = handle;
        Err(SandboxAdapterError::SpawnFailed {
            adapter_id: self.capabilities().adapter_id,
            reason: "sandbox adapter does not expose a resident warm-model guest agent transport"
                .to_string(),
        })
    }

    fn capabilities(&self) -> AdapterCapabilities;
}
