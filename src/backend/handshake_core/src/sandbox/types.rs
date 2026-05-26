use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const WINDOWS_NATIVE_JAIL_ADAPTER_ID: &str = "windows_native_jail";
pub const WINDOWS_NATIVE_JAIL_BACKEND_APPROVED: bool = cfg!(all(
    target_os = "windows",
    feature = "win-native-integration"
));
pub const DOCKER_ADAPTER_ID: &str = "docker";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AdapterId(String);

impl AdapterId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AdapterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImageRef(String);

impl ImageRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ImageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredCapability {
    Win32NativeFidelity,
    NvidiaCudaPassthrough,
    VendorAgnosticGpu,
    CrossMachinePortable,
    VeryStrongFilesystemIsolation,
    VeryStrongNetworkIsolation,
    HighStdioThroughput,
}

impl RequiredCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Win32NativeFidelity => "win32_native_fidelity",
            Self::NvidiaCudaPassthrough => "nvidia_cuda_passthrough",
            Self::VendorAgnosticGpu => "vendor_agnostic_gpu",
            Self::CrossMachinePortable => "cross_machine_portable",
            Self::VeryStrongFilesystemIsolation => "very_strong_filesystem_isolation",
            Self::VeryStrongNetworkIsolation => "very_strong_network_isolation",
            Self::HighStdioThroughput => "high_stdio_throughput",
        }
    }
}

impl fmt::Display for RequiredCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessSpec {
    pub id: AdapterId,
    pub image_or_root: ImageRef,
    pub cmd: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub binds: Vec<BindSpec>,
    pub net_policy: NetPolicy,
    pub resource_limits: ResourceLimits,
    pub required_capabilities: BTreeSet<RequiredCapability>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessHandle {
    pub id: Uuid,
    pub adapter_id: AdapterId,
    pub pid: Option<u32>,
    pub sandbox_internal_id: String,
    pub spawned_at_utc: DateTime<Utc>,
}

impl ProcessHandle {
    pub fn new(
        adapter_id: AdapterId,
        pid: Option<u32>,
        sandbox_internal_id: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            adapter_id,
            pid,
            sandbox_internal_id: sandbox_internal_id.into(),
            spawned_at_utc: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindMode {
    ReadOnly,
    ReadWrite,
    NoExec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindSpec {
    pub host_path: PathBuf,
    pub guest_path: PathBuf,
    pub mode: BindMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetProtocol {
    Tcp,
    Udp,
    Unix,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetAllowlistEntry {
    pub host: String,
    pub port: Option<u16>,
    pub protocol: NetProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetPolicy {
    DenyAll,
    LoopbackOnly,
    Allowlist(Vec<NetAllowlistEntry>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_bytes: Option<u64>,
    pub cpu_cores: Option<u16>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Signal {
    Term,
    Kill,
    Int,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    Running,
    Exited { code: i32 },
    Killed { by_signal: Signal },
    Orphaned,
    FailedToStart { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Bytes,
    pub stderr: Bytes,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub argv: Vec<String>,
    pub env_overlay: BTreeMap<String, String>,
    pub stdin: Option<Bytes>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum SandboxAdapterError {
    #[error("sandbox image or root missing: {image_or_root}")]
    ImageMissing { image_or_root: ImageRef },
    #[error("sandbox bind host path missing: {}", host_path.display())]
    BindHostPathMissing { host_path: PathBuf },
    #[error("sandbox bind guest path invalid {}: {reason}", guest_path.display())]
    BindGuestPathInvalid { guest_path: PathBuf, reason: String },
    #[error("sandbox adapter {adapter_id} failed to apply net policy: {reason}")]
    NetPolicyApplyFailed {
        adapter_id: AdapterId,
        reason: String,
    },
    #[error("sandbox adapter {adapter_id} failed to spawn process: {reason}")]
    SpawnFailed {
        adapter_id: AdapterId,
        reason: String,
    },
    #[error("sandbox process handle stale: {process_id}")]
    ProcessHandleStale { process_id: Uuid },
    #[error("sandbox adapter {adapter_id} unavailable: {reason}")]
    AdapterUnavailable {
        adapter_id: AdapterId,
        reason: String,
    },
    #[error(
        "sandbox capability unsatisfied: required={}, available={}",
        format_capability_set(.required),
        format_capability_set(.available)
    )]
    CapabilityUnsatisfied {
        required: BTreeSet<RequiredCapability>,
        available: BTreeSet<RequiredCapability>,
    },
}

fn format_capability_set(capabilities: &BTreeSet<RequiredCapability>) -> String {
    let values = capabilities
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}
