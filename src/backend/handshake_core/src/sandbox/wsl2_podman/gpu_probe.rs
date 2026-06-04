use std::time::{Duration, Instant};

use crate::sandbox::GpuPassthrough;

use super::adapter::Wsl2PodmanConfig;
use super::podman_cli::run_wsl_distribution_command;

pub const GPU_PROBE_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct GpuProbeCache {
    value: GpuPassthrough,
    probed_at: Instant,
}

impl GpuProbeCache {
    pub fn new(value: GpuPassthrough) -> Self {
        Self {
            value,
            probed_at: Instant::now(),
        }
    }

    pub fn value(&self) -> GpuPassthrough {
        self.value
    }

    pub fn is_fresh(&self) -> bool {
        self.probed_at.elapsed() <= GPU_PROBE_CACHE_TTL
    }
}

pub fn parse_nvidia_smi_output(text: &str) -> GpuPassthrough {
    if text
        .lines()
        .map(str::trim)
        .any(|line| !line.is_empty() && line.to_ascii_lowercase().contains("nvidia"))
    {
        GpuPassthrough::NvidiaCuda
    } else {
        GpuPassthrough::None
    }
}

pub async fn probe_gpu_passthrough(config: &Wsl2PodmanConfig) -> GpuPassthrough {
    let args = vec![
        "nvidia-smi".to_string(),
        "--query-gpu=name".to_string(),
        "--format=csv,noheader".to_string(),
    ];
    match run_wsl_distribution_command(config, &args, None, Some(config.command_timeout_ms())).await
    {
        Ok(output) if output.exit_code == 0 => {
            parse_nvidia_smi_output(&String::from_utf8_lossy(&output.stdout))
        }
        _ => GpuPassthrough::None,
    }
}
