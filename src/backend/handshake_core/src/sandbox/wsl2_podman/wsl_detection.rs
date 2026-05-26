use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::adapter::Wsl2PodmanConfig;
use super::podman_cli::run_host_command;
use crate::sandbox::{AdapterId, SandboxAdapterError, WSL2_PODMAN_ADAPTER_ID};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WslStatus {
    pub default_distribution: Option<String>,
    pub default_version: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WslDistro {
    pub name: String,
    pub state: Option<String>,
    pub version: Option<u8>,
    pub is_default: bool,
}

pub fn default_wsl_exe() -> PathBuf {
    which::which("wsl.exe").unwrap_or_else(|_| PathBuf::from("wsl.exe"))
}

pub fn decode_wsl_output(bytes: &[u8]) -> String {
    let nul_count = bytes.iter().filter(|byte| **byte == 0).count();
    if nul_count > bytes.len().saturating_div(4) {
        let mut units = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks(2) {
            if chunk.len() == 2 {
                units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
        }
        String::from_utf16_lossy(&units)
            .trim_start_matches('\u{feff}')
            .to_string()
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

pub fn parse_wsl_status(text: &str) -> Result<WslStatus, SandboxAdapterError> {
    let mut default_distribution = None;
    let mut default_version = None;

    for line in normalized_lines(text) {
        if let Some(value) = line.strip_prefix("Default Distribution:") {
            default_distribution = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("Default Version:") {
            default_version = Some(
                value
                    .trim()
                    .parse::<u8>()
                    .map_err(|error| spawn_failed(error))?,
            );
        }
    }

    Ok(WslStatus {
        default_distribution,
        default_version,
    })
}

pub fn parse_wsl_list_verbose(text: &str) -> Vec<WslDistro> {
    normalized_lines(text)
        .into_iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.to_ascii_uppercase().starts_with("NAME ") {
                return None;
            }
            let is_default = trimmed.starts_with('*');
            let row = trimmed.trim_start_matches('*').trim();
            let parts = row.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 3 {
                return None;
            }
            let version = parts.last().and_then(|value| value.parse::<u8>().ok());
            let state = parts
                .get(parts.len().saturating_sub(2))
                .map(|v| v.to_string());
            let name = parts[..parts.len().saturating_sub(2)].join(" ");
            Some(WslDistro {
                name,
                state,
                version,
                is_default,
            })
        })
        .collect()
}

pub async fn verify_wsl2_distro(config: &Wsl2PodmanConfig) -> Result<(), SandboxAdapterError> {
    let status = run_host_command(
        config.wsl_exe(),
        &["--status".to_string()],
        None,
        Some(config.command_timeout_ms()),
    )
    .await?;
    if status.exit_code != 0 {
        return Err(spawn_failed(format!(
            "wsl --status failed: {}",
            status.stderr_text()
        )));
    }
    let parsed_status = parse_wsl_status(&decode_wsl_output(&status.stdout))?;
    if parsed_status.default_version != Some(2) {
        return Err(spawn_failed(format!(
            "WSL default version is {:?}; WSL2 is required",
            parsed_status.default_version
        )));
    }

    let distros = run_host_command(
        config.wsl_exe(),
        &["-l".to_string(), "-v".to_string()],
        None,
        Some(config.command_timeout_ms()),
    )
    .await?;
    if distros.exit_code != 0 {
        return Err(spawn_failed(format!(
            "wsl -l -v failed: {}",
            distros.stderr_text()
        )));
    }
    let parsed_distros = parse_wsl_list_verbose(&decode_wsl_output(&distros.stdout));
    let distro = parsed_distros
        .iter()
        .find(|entry| entry.name == config.distro())
        .ok_or_else(|| {
            spawn_failed(format!(
                "WSL distro `{}` is not registered",
                config.distro()
            ))
        })?;
    if distro.version != Some(2) {
        return Err(spawn_failed(format!(
            "WSL distro `{}` is version {:?}; WSL2 is required",
            config.distro(),
            distro.version
        )));
    }

    Ok(())
}

fn normalized_lines(text: &str) -> Vec<String> {
    text.replace('\0', "")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn spawn_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        reason: reason.to_string(),
    }
}
