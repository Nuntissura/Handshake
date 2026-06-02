//! Packaging support for the Cloud Hypervisor resident warm-agent.
//!
//! The persistent CH initramfs only contains BusyBox plus files baked from
//! declared binds. A warm-agent package therefore must produce a Linux guest
//! artifact that can run in that minimal environment. The current package path
//! targets `x86_64-unknown-linux-musl` so the bound binary is static instead of
//! depending on a glibc loader that is absent from the guest image.

use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::guest_agent::{
    validate_warm_agent_host_candidate, CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT,
    CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV, CLOUD_HYPERVISOR_WARM_AGENT_PACKAGE_MANIFEST_FILE,
};

pub const WARM_AGENT_PACKAGE_SCHEMA_VERSION: &str = "hsk_warm_agent_guest_package@1";
pub const WARM_AGENT_PACKAGE_KIND: &str = "cloud_hypervisor_warm_agent";
pub const WARM_AGENT_PACKAGE_BINARY_NAME: &str = "hsk-warm-agent";
pub const WARM_AGENT_PACKAGE_BUILD_TARGET: &str = "x86_64-unknown-linux-musl";
pub const DEFAULT_WSL_DISTRO: &str = "Ubuntu";

#[derive(Debug, thiserror::Error)]
pub enum WarmAgentPackageError {
    #[error("warm-agent package path error: {0}")]
    Path(String),
    #[error("warm-agent package prerequisite failed: {0}")]
    Prerequisite(String),
    #[error("warm-agent package build failed: {0}")]
    Build(String),
    #[error("warm-agent package I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("warm-agent package JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarmAgentPackageOptions {
    pub repo_root: PathBuf,
    pub distro: String,
    pub output_dir: PathBuf,
    pub cargo_target_dir: PathBuf,
}

impl WarmAgentPackageOptions {
    pub fn default_for_repo(repo_root: impl Into<PathBuf>) -> Result<Self, WarmAgentPackageError> {
        let repo_root = repo_root.into();
        Ok(Self {
            output_dir: default_package_output_dir(&repo_root)?,
            cargo_target_dir: default_wsl_cargo_target_dir(&repo_root)?,
            repo_root,
            distro: DEFAULT_WSL_DISTRO.to_string(),
        })
    }

    pub fn cargo_manifest_path(&self) -> PathBuf {
        self.repo_root
            .join("src")
            .join("backend")
            .join("handshake_core")
            .join("Cargo.toml")
    }

    pub fn artifact_path(&self) -> PathBuf {
        self.output_dir.join(WARM_AGENT_PACKAGE_BINARY_NAME)
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.output_dir
            .join(CLOUD_HYPERVISOR_WARM_AGENT_PACKAGE_MANIFEST_FILE)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarmAgentGuestPackageManifest {
    pub schema_version: String,
    pub package_kind: String,
    pub agent_file_name: String,
    pub build_target: String,
    pub binary_sha256: String,
    pub host_path: String,
    pub guest_root: String,
    pub guest_path: String,
    pub required_host_path_env: String,
    pub required_protocol_id: String,
    pub required_protocol_version: u16,
    pub requires_static_linking: bool,
    pub produced_by: String,
    pub notes: Vec<String>,
}

pub fn default_package_output_dir(repo_root: &Path) -> Result<PathBuf, WarmAgentPackageError> {
    let workspace_root = repo_root.parent().ok_or_else(|| {
        WarmAgentPackageError::Path(format!(
            "repo root has no parent for external artifact directory: {}",
            repo_root.display()
        ))
    })?;
    Ok(workspace_root
        .join("Handshake_Artifacts")
        .join("warm-agent")
        .join(WARM_AGENT_PACKAGE_BUILD_TARGET))
}

pub fn default_wsl_cargo_target_dir(repo_root: &Path) -> Result<PathBuf, WarmAgentPackageError> {
    let workspace_root = repo_root.parent().ok_or_else(|| {
        WarmAgentPackageError::Path(format!(
            "repo root has no parent for external Cargo target directory: {}",
            repo_root.display()
        ))
    })?;
    Ok(workspace_root
        .join("Handshake_Artifacts")
        .join("handshake-cargo-target-wsl"))
}

pub fn package_warm_agent_with_wsl(
    options: &WarmAgentPackageOptions,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    let manifest_path = options.cargo_manifest_path();
    if !manifest_path.is_file() {
        return Err(WarmAgentPackageError::Path(format!(
            "Cargo manifest missing: {}",
            manifest_path.display()
        )));
    }
    std::fs::create_dir_all(&options.output_dir)?;

    let wsl_manifest = windows_path_to_wsl(&options.distro, &manifest_path)?;
    let wsl_output_dir = windows_path_to_wsl(&options.distro, &options.output_dir)?;
    let wsl_target_dir = windows_path_to_wsl(&options.distro, &options.cargo_target_dir)?;
    let script =
        build_wsl_static_musl_package_script(&wsl_manifest, &wsl_output_dir, &wsl_target_dir);
    let output = Command::new("wsl.exe")
        .arg("-d")
        .arg(&options.distro)
        .arg("--")
        .arg("bash")
        .arg("-lc")
        .arg(script)
        .output()
        .map_err(|error| WarmAgentPackageError::Build(format!("failed to launch WSL: {error}")))?;
    if !output.status.success() {
        return Err(WarmAgentPackageError::Build(format!(
            "WSL package command failed with status {}: stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let manifest = manifest_for_artifact(&options.artifact_path())?;
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    let mut file = File::create(options.manifest_path())?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    Ok(manifest)
}

pub fn check_wsl_static_musl_prereqs(distro: &str) -> Result<(), WarmAgentPackageError> {
    let script = [
        "set -e",
        "command -v cargo >/dev/null",
        "command -v rustup >/dev/null",
        "command -v x86_64-linux-musl-gcc >/dev/null",
        "rustup target list --installed | grep -qx x86_64-unknown-linux-musl",
    ]
    .join("; ");
    let output = Command::new("wsl.exe")
        .arg("-d")
        .arg(distro)
        .arg("--")
        .arg("bash")
        .arg("-lc")
        .arg(script)
        .output()
        .map_err(|error| {
            WarmAgentPackageError::Prerequisite(format!("failed to launch WSL: {error}"))
        })?;
    if output.status.success() {
        return Ok(());
    }
    Err(WarmAgentPackageError::Prerequisite(format!(
        "WSL distro `{distro}` needs cargo, rustup target {WARM_AGENT_PACKAGE_BUILD_TARGET}, and x86_64-linux-musl-gcc before packaging; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

pub fn manifest_for_artifact(
    artifact_path: &Path,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    validate_linux_elf_agent_artifact(artifact_path)?;
    let binary_sha256 = sha256_file_hex(artifact_path)?;
    let host_path = artifact_path.to_string_lossy().replace('\\', "/");
    let guest_path = format!(
        "{}/{}",
        CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.trim_end_matches('/'),
        WARM_AGENT_PACKAGE_BINARY_NAME
    );
    Ok(WarmAgentGuestPackageManifest {
        schema_version: WARM_AGENT_PACKAGE_SCHEMA_VERSION.to_string(),
        package_kind: WARM_AGENT_PACKAGE_KIND.to_string(),
        agent_file_name: WARM_AGENT_PACKAGE_BINARY_NAME.to_string(),
        build_target: WARM_AGENT_PACKAGE_BUILD_TARGET.to_string(),
        binary_sha256,
        host_path,
        guest_root: CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.to_string(),
        guest_path,
        required_host_path_env: CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV.to_string(),
        required_protocol_id: crate::model_runtime::WARM_AGENT_PROTOCOL_ID.to_string(),
        required_protocol_version: crate::model_runtime::WARM_AGENT_PROTOCOL_VERSION,
        requires_static_linking: true,
        produced_by: "hsk-warm-agent-package".to_string(),
        notes: vec![
            "Bind this host_path by setting HANDSHAKE_CH_WARM_AGENT_HOST_PATH.".to_string(),
            "The package target is static musl because the CH initramfs has no glibc loader."
                .to_string(),
            "This package does not include llama-server; configure it inside the guest package/root before live MT-207 proof."
                .to_string(),
        ],
    })
}

pub fn validate_linux_elf_agent_artifact(path: &Path) -> Result<(), WarmAgentPackageError> {
    validate_warm_agent_host_candidate(path).map_err(WarmAgentPackageError::Path)?;
    let mut magic = [0_u8; 4];
    File::open(path)?.read_exact(&mut magic)?;
    if magic != *b"\x7fELF" {
        return Err(WarmAgentPackageError::Path(format!(
            "warm-agent package artifact must be a Linux ELF file, got non-ELF magic at {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn build_wsl_static_musl_package_script(
    wsl_manifest_path: &str,
    wsl_output_dir: &str,
    wsl_target_dir: &str,
) -> String {
    let artifact = format!("{wsl_output_dir}/{WARM_AGENT_PACKAGE_BINARY_NAME}");
    let built = format!(
        "{target}/{triple}/release/{name}",
        target = wsl_target_dir.trim_end_matches('/'),
        triple = WARM_AGENT_PACKAGE_BUILD_TARGET,
        name = WARM_AGENT_PACKAGE_BINARY_NAME
    );
    [
        "set -euo pipefail".to_string(),
        "command -v cargo >/dev/null || { echo 'missing cargo in WSL; install Rust in the WSL distro' >&2; exit 86; }".to_string(),
        "command -v rustup >/dev/null || { echo 'missing rustup in WSL; install Rust with rustup' >&2; exit 86; }".to_string(),
        "command -v x86_64-linux-musl-gcc >/dev/null || { echo 'missing x86_64-linux-musl-gcc in WSL; install musl-tools' >&2; exit 86; }".to_string(),
        format!(
            "rustup target list --installed | grep -qx {target} || rustup target add {target}",
            target = WARM_AGENT_PACKAGE_BUILD_TARGET
        ),
        format!("mkdir -p {}", sh_quote(wsl_output_dir)),
        format!(
            "cargo build --release --target {target} --bin {name} --manifest-path {manifest} --target-dir {target_dir}",
            target = WARM_AGENT_PACKAGE_BUILD_TARGET,
            name = WARM_AGENT_PACKAGE_BINARY_NAME,
            manifest = sh_quote(wsl_manifest_path),
            target_dir = sh_quote(wsl_target_dir),
        ),
        format!("cp {} {}", sh_quote(&built), sh_quote(&artifact)),
        format!("chmod 0755 {}", sh_quote(&artifact)),
        format!("{} </dev/null >/tmp/hsk-warm-agent-package-probe.out", sh_quote(&artifact)),
        "echo HSK_WARM_AGENT_PACKAGE_OK".to_string(),
    ]
    .join("; ")
}

pub fn windows_path_to_wsl(distro: &str, path: &Path) -> Result<String, WarmAgentPackageError> {
    let output = Command::new("wsl.exe")
        .arg("-d")
        .arg(distro)
        .arg("--")
        .arg("wslpath")
        .arg("-a")
        .arg(path)
        .output()
        .map_err(|error| {
            WarmAgentPackageError::Path(format!(
                "failed to launch wslpath for {}: {error}",
                path.display()
            ))
        })?;
    if !output.status.success() {
        return Err(WarmAgentPackageError::Path(format!(
            "wslpath failed for {} with status {}: {}",
            path.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn sha256_file_hex(path: &Path) -> Result<String, WarmAgentPackageError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_package_script_targets_static_musl_and_runtime_probe() {
        let script = build_wsl_static_musl_package_script(
            "/repo/src/backend/handshake_core/Cargo.toml",
            "/out",
            "/target",
        );
        assert!(script.contains("x86_64-unknown-linux-musl"));
        assert!(script.contains("x86_64-linux-musl-gcc"));
        assert!(script.contains("--bin hsk-warm-agent"));
        assert!(script.contains("</dev/null >/tmp/hsk-warm-agent-package-probe.out"));
    }

    #[test]
    fn shell_quote_handles_spaces_and_single_quotes() {
        assert_eq!(sh_quote("/a path/it's"), "'/a path/it'\"'\"'s'");
    }

    #[test]
    fn manifest_requires_elf_and_records_guest_binding_contract() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let artifact = dir.join(WARM_AGENT_PACKAGE_BINARY_NAME);
        std::fs::write(&artifact, minimal_static_x86_64_elf()).expect("write elf");
        let manifest = manifest_for_artifact(&artifact).expect("manifest");
        assert_eq!(manifest.schema_version, WARM_AGENT_PACKAGE_SCHEMA_VERSION);
        assert_eq!(manifest.build_target, WARM_AGENT_PACKAGE_BUILD_TARGET);
        assert_eq!(manifest.guest_path, "/warm-agent/hsk-warm-agent");
        assert_eq!(
            manifest.required_host_path_env,
            CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV
        );
        assert!(manifest.requires_static_linking);
        assert_eq!(manifest.binary_sha256.len(), 64);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_rejects_non_elf_candidate() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let artifact = dir.join(WARM_AGENT_PACKAGE_BINARY_NAME);
        std::fs::write(&artifact, b"#!/bin/sh\n").expect("write shell");
        assert!(manifest_for_artifact(&artifact).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn minimal_static_x86_64_elf() -> Vec<u8> {
        let mut elf = vec![0_u8; 64 + 56];
        elf[0..4].copy_from_slice(b"\x7fELF");
        elf[4] = 2;
        elf[5] = 1;
        elf[6] = 1;
        elf[16..18].copy_from_slice(&3_u16.to_le_bytes());
        elf[18..20].copy_from_slice(&62_u16.to_le_bytes());
        elf[20..24].copy_from_slice(&1_u32.to_le_bytes());
        elf[32..40].copy_from_slice(&64_u64.to_le_bytes());
        elf[52..54].copy_from_slice(&64_u16.to_le_bytes());
        elf[54..56].copy_from_slice(&56_u16.to_le_bytes());
        elf[56..58].copy_from_slice(&1_u16.to_le_bytes());
        elf[64..68].copy_from_slice(&1_u32.to_le_bytes());
        elf
    }
}
