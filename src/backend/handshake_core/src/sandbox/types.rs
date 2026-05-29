use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::adapter::TrustClass;

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
    #[serde(default)]
    pub trust_class: TrustClass,
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

/// Reference to a persisted sandbox snapshot (Master Spec v02.187 §3.5.7 #7).
///
/// A snapshot captures the full live state of a running sandbox (for a
/// hardware-virtualized microVM this is the paused CPU + RAM + device state) so
/// it can later be restored into a fresh sandbox instance that resumes exactly
/// where the original left off — the foundation of the validate-then-promote
/// flow. `snapshot_dir` is the adapter-specific location of the on-disk capture;
/// for the Cloud Hypervisor adapter it is the WSL-side absolute directory that
/// holds `config.json`, `state.json`, and the memory-range files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRef {
    pub id: Uuid,
    pub adapter_id: AdapterId,
    pub snapshot_dir: String,
    pub created_at_utc: DateTime<Utc>,
    /// Optional adapter-side path where the snapshot's live output stream
    /// continues to be observable after a restore (for the Cloud Hypervisor
    /// adapter this is the original VM's absolute serial-log path, which the
    /// restored VM keeps appending to). Lets callers confirm that restored
    /// state resumed rather than rebooted. `None` when the adapter exposes no
    /// such observation channel.
    #[serde(default)]
    pub observe_path: Option<String>,
}

impl SnapshotRef {
    pub fn new(adapter_id: AdapterId, snapshot_dir: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            adapter_id,
            snapshot_dir: snapshot_dir.into(),
            created_at_utc: Utc::now(),
            observe_path: None,
        }
    }

    /// Attach the adapter-side live-state observation path (see
    /// [`SnapshotRef::observe_path`]).
    pub fn with_observe_path(mut self, observe_path: impl Into<String>) -> Self {
        self.observe_path = Some(observe_path.into());
        self
    }
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
    #[error("sandbox adapter {adapter_id} does not support snapshot/restore")]
    SnapshotUnsupported { adapter_id: AdapterId },
    #[error("sandbox adapter {adapter_id} snapshot/restore failed: {reason}")]
    SnapshotFailed {
        adapter_id: AdapterId,
        reason: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::adapter::{
        AdapterCapabilities, GpuPassthrough, IsolationStrength, IsolationTier, ThroughputClass,
    };

    #[test]
    fn snapshot_ref_serde_round_trips_observe_path() {
        let with = SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/snap/dir")
            .with_observe_path("/persistent/serial.log");
        let json = serde_json::to_string(&with).expect("serialize SnapshotRef");
        let back: SnapshotRef = serde_json::from_str(&json).expect("deserialize SnapshotRef");
        assert_eq!(back, with);
        assert_eq!(back.observe_path.as_deref(), Some("/persistent/serial.log"));
    }

    #[test]
    fn snapshot_ref_decodes_legacy_row_without_observe_path() {
        // A persisted row written before observe_path existed must decode to
        // None (the field is #[serde(default)]) — back-compat guard.
        let legacy = r#"{"id":"01890000-0000-7000-8000-000000000000","adapter_id":"cloud_hypervisor","snapshot_dir":"/snap/dir","created_at_utc":"2026-05-29T00:00:00Z"}"#;
        let decoded: SnapshotRef = serde_json::from_str(legacy).expect("decode legacy SnapshotRef");
        assert_eq!(decoded.observe_path, None);
        assert_eq!(decoded.snapshot_dir, "/snap/dir");
    }

    #[test]
    fn adapter_capabilities_round_trips_snapshot_tier_flags() {
        // The pre-existing capabilities round-trip test only covers the
        // all-false no-op defaults; assert the snapshot-tier `true` shape too.
        let caps = AdapterCapabilities {
            adapter_id: AdapterId::new("cloud_hypervisor"),
            runtime_available: true,
            filesystem_isolation_strength: IsolationStrength::VeryStrong,
            network_isolation_strength: IsolationStrength::VeryStrong,
            gpu_passthrough: GpuPassthrough::None,
            stdio_throughput_class: ThroughputClass::Low,
            win32_native_fidelity: false,
            cross_machine_portable: true,
            isolation_tier: IsolationTier::Tier3Microvm,
            requires_nested_virt: true,
            supports_snapshot: true,
        };
        let json = serde_json::to_string(&caps).expect("serialize capabilities");
        let back: AdapterCapabilities =
            serde_json::from_str(&json).expect("deserialize capabilities");
        assert_eq!(back, caps);
        assert!(back.supports_snapshot);
        assert!(back.requires_nested_virt);
    }
}
