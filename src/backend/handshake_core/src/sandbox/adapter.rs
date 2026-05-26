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
