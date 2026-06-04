use serde::{Deserialize, Serialize};

use super::{AdapterId, WSL2_PODMAN_ADAPTER_ID};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxDefaultAdapterChoice {
    Wsl2Podman,
}

impl SandboxDefaultAdapterChoice {
    pub fn adapter_id(self) -> AdapterId {
        match self {
            Self::Wsl2Podman => AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        }
    }
}

impl Default for SandboxDefaultAdapterChoice {
    fn default() -> Self {
        Self::Wsl2Podman
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SandboxSettings {
    pub default_adapter: SandboxDefaultAdapterChoice,
    pub docker_explicit_opt_in: bool,
}

impl Default for SandboxSettings {
    fn default() -> Self {
        Self {
            default_adapter: SandboxDefaultAdapterChoice::Wsl2Podman,
            docker_explicit_opt_in: false,
        }
    }
}
