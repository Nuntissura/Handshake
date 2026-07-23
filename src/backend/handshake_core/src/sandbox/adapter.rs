use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::PathBuf,
    process::ExitStatus,
    sync::Arc,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::model_runtime::WarmAgentTransport;

use super::types::{
    AdapterId, BindMode, Command, ExecResult, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    RequiredCapability, SandboxAdapterError, Signal, SnapshotRef,
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

/// Effective network posture enforced for an attached process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachedNetworkMode {
    /// AppContainer receives no network capability.
    DenyAll,
    /// AppContainer receives only the outbound `internetClient` capability.
    /// Loopback, listeners, and private-network access remain unavailable.
    OutboundInternetClient,
}

/// Exact standard-stream disposition requested for an attached process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachedStdioMode {
    Null,
    Pipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachedStdioContract {
    pub stdin: AttachedStdioMode,
    pub stdout: AttachedStdioMode,
    pub stderr: AttachedStdioMode,
}

impl AttachedStdioContract {
    pub const fn null_stdin_piped_output() -> Self {
        Self {
            stdin: AttachedStdioMode::Null,
            stdout: AttachedStdioMode::Pipe,
            stderr: AttachedStdioMode::Pipe,
        }
    }

    pub(crate) fn validate(self, adapter_id: AdapterId) -> Result<(), SandboxAdapterError> {
        if self == Self::null_stdin_piped_output() {
            Ok(())
        } else {
            Err(SandboxAdapterError::SpawnFailed {
                adapter_id,
                reason: "attached execution supports only null stdin and piped stdout/stderr"
                    .to_string(),
            })
        }
    }
}

/// Exact launch input for a process whose stdio and lifecycle remain attached
/// to the caller. `env` replaces the child environment in full.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachedProcessSpec {
    pub executable_path: PathBuf,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<PathBuf>,
    /// Explicit host paths granted to the AppContainer. No ambient host path
    /// access is inferred from environment variables.
    pub binds: Vec<super::types::BindSpec>,
    pub network_mode: AttachedNetworkMode,
    /// Explicit canonical trust decision. Trusted and reviewed workloads may
    /// run at Tier 1; untrusted-agent workloads require a stronger tier.
    pub trust_class: TrustClass,
    /// Canonical capability requirements attached to the launch decision.
    pub required_capabilities: BTreeSet<RequiredCapability>,
    /// Independently requested minimum isolation tier. This is checked before
    /// adapter spawn and carried into the concrete attached invocation.
    pub requested_isolation_tier: IsolationTier,
    /// Original network policy whose effective attached representation is
    /// `network_mode`; adapters must not silently reinterpret it.
    pub requested_net_policy: NetPolicy,
    /// Resource limits enforced by the creation-time Job Object.
    pub resource_limits: super::types::ResourceLimits,
    /// Maximum time allowed for adapter process creation. Zero is invalid.
    pub startup_timeout_ms: u64,
    /// Backend-owned ephemeral roots whose lifetime is transferred to the
    /// attached startup supervisor and, after launch, to the child lifecycle.
    /// They are removed only after ACL rollback and terminal process cleanup.
    #[serde(default)]
    pub ephemeral_cleanup_paths: Vec<PathBuf>,
    /// Execution-policy authority selected for this concrete invocation.
    pub execution_policy_ref: String,
    /// Typed, versioned policy resolution. Production callers that resolve an
    /// execution policy carry it to this final process boundary so the adapter
    /// can reject drift from the pre-side-effect posture decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_execution_policy: Option<super::execution_policy::ResolvedExecutionPolicy>,
    /// Parallel-agent/workspace attribution carried to the adapter boundary.
    pub swarm_id: Option<String>,
    pub worktree_id: Option<String>,
    /// Fenced checkout ownership carried to the concrete process boundary.
    #[serde(default)]
    pub checkout_lease_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub checkout_lease_owner_generation: Option<u64>,
    #[serde(default)]
    pub checkout_lease_canonical_working_dir: Option<String>,
}

/// Owns backend-created ephemeral paths from the first attached-spawn boundary
/// until either startup fails or the returned child reaches terminal cleanup.
/// Taking the paths out of [`AttachedProcessSpec`] prevents any validation or
/// admission return from orphaning caller-owned startup state.
#[derive(Debug)]
pub(crate) struct AttachedEphemeralPathGuard {
    paths: Vec<PathBuf>,
    completed: bool,
}

impl AttachedEphemeralPathGuard {
    pub(crate) fn take_from(spec: &mut AttachedProcessSpec) -> Self {
        Self {
            paths: std::mem::take(&mut spec.ephemeral_cleanup_paths),
            completed: false,
        }
    }

    pub(crate) fn cleanup(&mut self) -> Result<(), String> {
        if self.completed {
            return Ok(());
        }

        let mut errors = Vec::new();
        for path in self.paths.iter().rev() {
            let result = match std::fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_dir() => std::fs::remove_dir_all(path),
                Ok(_) => std::fs::remove_file(path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                errors.push(format!("{}: {error}", path.display()));
            }
        }

        if errors.is_empty() {
            self.completed = true;
            Ok(())
        } else {
            Err(format!(
                "failed to remove {} attached ephemeral path(s): {}",
                errors.len(),
                errors.join(" | ")
            ))
        }
    }

    fn return_to(mut self, spec: &mut AttachedProcessSpec) {
        debug_assert!(spec.ephemeral_cleanup_paths.is_empty());
        spec.ephemeral_cleanup_paths = std::mem::take(&mut self.paths);
        self.completed = true;
    }
}

impl Drop for AttachedEphemeralPathGuard {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            tracing::error!(error = %error, "attached ephemeral path cleanup incomplete");
        }
    }
}

/// Owning live-process contract. Implementations retain the process-tree
/// authority until `wait`, `terminate_tree_and_wait`, or Drop reaps it.
pub trait AttachedSandboxProcess: Send {
    fn pid(&self) -> u32;
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>>;
    fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>>;
    fn try_wait(&mut self) -> Result<Option<ExitStatus>, SandboxAdapterError>;
    fn wait(&mut self) -> Result<ExitStatus, SandboxAdapterError>;
    fn terminate_tree_and_wait(&mut self) -> Result<ExitStatus, SandboxAdapterError>;
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

/// Durable identity used by the adapter that originally owned a process when
/// normal attached ownership was lost across a crash or failed reap. The
/// process ledger supplies the immutable launch evidence; the selected
/// adapter remains the only authority allowed to inspect or terminate it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetachedProcessIdentity {
    pub process_uuid: uuid::Uuid,
    pub handle: ProcessHandle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_creation_time_100ns: Option<u64>,
}

#[async_trait]
pub trait SandboxAdapter: Send + Sync {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError>;

    /// Spawn a live process whose stdout, stderr, tree-termination authority,
    /// and reap obligation are returned as one owning object. Adapters that do
    /// not implement attached execution fail closed by default.
    async fn spawn_attached(
        &self,
        mut spec: AttachedProcessSpec,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        let _ephemeral_cleanup = AttachedEphemeralPathGuard::take_from(&mut spec);
        Err(SandboxAdapterError::SpawnFailed {
            adapter_id: self.capabilities().adapter_id,
            reason: "sandbox adapter does not expose attached live-process ownership".to_string(),
        })
    }

    async fn spawn_attached_with_stdio(
        &self,
        mut spec: AttachedProcessSpec,
        stdio: AttachedStdioContract,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        let ephemeral_cleanup = AttachedEphemeralPathGuard::take_from(&mut spec);
        stdio.validate(self.capabilities().adapter_id)?;
        ephemeral_cleanup.return_to(&mut spec);
        self.spawn_attached(spec).await
    }

    /// Proves that the adapter can enforce an attached child's network
    /// contract. Detached-process policy support is not sufficient evidence.
    fn validate_attached_network_mode(
        &self,
        mode: AttachedNetworkMode,
    ) -> Result<(), SandboxAdapterError> {
        Err(SandboxAdapterError::SpawnFailed {
            adapter_id: self.capabilities().adapter_id,
            reason: format!(
                "sandbox adapter cannot enforce attached network mode {:?}",
                mode
            ),
        })
    }

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

    /// Reclaim a crash-detached process through its owning adapter. Container
    /// and VM adapters can reconstruct their durable handle and use their
    /// normal kill path; host-attached adapters override this to verify exact
    /// OS generation and executable identity before termination.
    async fn reclaim_detached(
        &self,
        identity: &DetachedProcessIdentity,
        signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        self.kill(&identity.handle, signal).await
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError>;

    /// Query the owning adapter's authoritative state for a crash-detached
    /// process. This mirrors [`SandboxAdapter::reclaim_detached`] and defaults
    /// to the adapter's normal durable-handle status path.
    async fn detached_status(
        &self,
        identity: &DetachedProcessIdentity,
    ) -> Result<ProcessStatus, SandboxAdapterError> {
        self.status(&identity.handle).await
    }

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
