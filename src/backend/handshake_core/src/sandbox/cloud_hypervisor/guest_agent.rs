//! Cloud Hypervisor resident warm-agent contract.
//!
//! The current persistent VM image contains a BusyBox serial command agent. That
//! agent proves snapshot/restore and generic exec, but it cannot keep llama.cpp
//! weights resident or emit live per-token frames. MT-207's warm path must only
//! advertise support once a guest image serves this contract over serial or
//! vsock.

use serde::{Deserialize, Serialize};

pub const CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT: &str =
    "model-bearing guest agent over serial/vsock";
pub const CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV: &str = "HANDSHAKE_CH_WARM_AGENT_HOST_PATH";
pub const CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT: &str = "/warm-agent";
pub const CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY: &str =
    "hsk.cloud_hypervisor.warm_agent_guest_path";
pub const CLOUD_HYPERVISOR_WARM_AGENT_CMDLINE_KEY: &str = "hsk.warm_agent";
pub const CLOUD_HYPERVISOR_WARM_AGENT_UNAVAILABLE_REASON: &str =
    "Cloud Hypervisor persistent VMs now expose a serial-socket command channel, \
     but warm-model RPC and live token streaming require a resident model-serving \
     guest agent/image; serial is the bootstrap transport and virtio-vsock remains \
     the hardened follow-on";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudHypervisorWarmAgentContract {
    pub required_transport: String,
    pub host_path_env: String,
    pub guest_root: String,
    pub guest_path_metadata_key: String,
    pub required_protocol_id: String,
    pub required_protocol_version: u16,
    pub requires_model_residency: bool,
    pub requires_live_token_frames: bool,
    pub permits_shell_fallback: bool,
}

impl CloudHypervisorWarmAgentContract {
    pub fn current() -> Self {
        Self {
            required_transport: CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT.to_string(),
            host_path_env: CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV.to_string(),
            guest_root: CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.to_string(),
            guest_path_metadata_key: CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY
                .to_string(),
            required_protocol_id: crate::model_runtime::WARM_AGENT_PROTOCOL_ID.to_string(),
            required_protocol_version: crate::model_runtime::WARM_AGENT_PROTOCOL_VERSION,
            requires_model_residency: true,
            requires_live_token_frames: true,
            permits_shell_fallback: false,
        }
    }
}

pub fn warm_agent_unavailable_detail() -> String {
    let contract = CloudHypervisorWarmAgentContract::current();
    format!(
        "required_transport={}, protocol={}@v{}, requires_model_residency={}, \
         requires_live_token_frames={}, permits_shell_fallback={}, host_path_env={}, \
         guest_root={}, guest_path_metadata_key={}, reason={}",
        contract.required_transport,
        contract.required_protocol_id,
        contract.required_protocol_version,
        contract.requires_model_residency,
        contract.requires_live_token_frames,
        contract.permits_shell_fallback,
        contract.host_path_env,
        contract.guest_root,
        contract.guest_path_metadata_key,
        CLOUD_HYPERVISOR_WARM_AGENT_UNAVAILABLE_REASON
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_requires_resident_model_agent_and_rejects_shell_fallback() {
        let contract = CloudHypervisorWarmAgentContract::current();
        assert!(contract.requires_model_residency);
        assert!(contract.requires_live_token_frames);
        assert!(!contract.permits_shell_fallback);
        assert!(contract.required_transport.contains("serial"));
        assert!(contract.required_transport.contains("vsock"));
        assert_eq!(
            contract.host_path_env,
            CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV
        );
        assert_eq!(
            contract.guest_path_metadata_key,
            CLOUD_HYPERVISOR_WARM_AGENT_GUEST_PATH_METADATA_KEY
        );
    }
}
