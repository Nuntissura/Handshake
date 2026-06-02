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
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model_runtime::{
    decode_warm_agent_frame, encode_warm_agent_frame, WarmAgentGuestFrame, WarmAgentHostFrame,
};

use super::guest_agent::{
    validate_static_x86_64_guest_elf, validate_warm_agent_llama_server_sibling,
    CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT, CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV,
    CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE,
    CLOUD_HYPERVISOR_WARM_AGENT_PACKAGE_MANIFEST_FILE,
};

pub const WARM_AGENT_PACKAGE_SCHEMA_VERSION: &str = "hsk_warm_agent_guest_package@1";
pub const WARM_AGENT_PACKAGE_KIND: &str = "cloud_hypervisor_warm_agent";
pub const WARM_AGENT_PACKAGE_BINARY_NAME: &str = "hsk-warm-agent";
pub const WARM_AGENT_PACKAGE_BUILD_TARGET: &str = "x86_64-unknown-linux-musl";
pub const WARM_AGENT_PACKAGE_AGENT_BINARY_MARKER: &str = "hsk-warm-agent-llama-server";
pub const WARM_AGENT_PACKAGE_PROTOCOL_PROBE_KIND: &str = "warm_agent_stdio_ping_heartbeat";
pub const WARM_AGENT_PACKAGE_PROTOCOL_PROBE_REQUEST_ID: &str = "package-probe";
pub const WARM_AGENT_PACKAGE_PROTOCOL_PROBE_RESPONSE_TYPE: &str = "heartbeat";
pub const WARM_AGENT_PACKAGE_LLAMA_SERVER_MARKERS: [&str; 2] = ["/health", "/completion"];
pub const WARM_AGENT_PACKAGE_RUNTIME_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
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
    pub llama_server_host_path: Option<PathBuf>,
}

impl WarmAgentPackageOptions {
    pub fn default_for_repo(repo_root: impl Into<PathBuf>) -> Result<Self, WarmAgentPackageError> {
        let repo_root = repo_root.into();
        Ok(Self {
            output_dir: default_package_output_dir(&repo_root)?,
            cargo_target_dir: default_wsl_cargo_target_dir(&repo_root)?,
            llama_server_host_path: None,
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

    pub fn llama_server_artifact_path(&self) -> PathBuf {
        self.output_dir
            .join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE)
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
    pub llama_server_file_name: String,
    pub build_target: String,
    pub binary_sha256: String,
    pub llama_server_sha256: String,
    pub agent_binary_marker: String,
    pub llama_server_required_markers: Vec<String>,
    pub host_path: String,
    pub guest_root: String,
    pub guest_path: String,
    pub llama_server_guest_path: String,
    pub required_host_path_env: String,
    pub required_protocol_id: String,
    pub required_protocol_version: u16,
    pub requires_static_linking: bool,
    pub package_complete: bool,
    pub protocol_probe: Option<WarmAgentPackageProtocolProbe>,
    pub produced_by: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarmAgentPackageProtocolProbe {
    pub probe_kind: String,
    pub request_id: String,
    pub response_type: String,
    pub response_request_id: String,
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
    let wsl_llama_server = options
        .llama_server_host_path
        .as_deref()
        .map(|path| windows_path_to_wsl(&options.distro, path))
        .transpose()?;
    let script = build_wsl_static_musl_package_script(
        &wsl_manifest,
        &wsl_output_dir,
        &wsl_target_dir,
        wsl_llama_server.as_deref(),
    );
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

    let manifest = manifest_for_runtime_probed_artifact(
        &options.artifact_path(),
        &options.distro,
        Path::new("wsl.exe"),
    )?;
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
    let llama_server = validate_warm_agent_llama_server_sibling(artifact_path)
        .map_err(WarmAgentPackageError::Path)?;
    validate_package_binary_identity(artifact_path, &llama_server)?;
    let binary_sha256 = sha256_file_hex(artifact_path)?;
    let llama_server_sha256 = sha256_file_hex(&llama_server)?;
    let host_path = artifact_path.to_string_lossy().replace('\\', "/");
    let guest_path = format!(
        "{}/{}",
        CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.trim_end_matches('/'),
        WARM_AGENT_PACKAGE_BINARY_NAME
    );
    let llama_server_guest_path = format!(
        "{}/{}",
        CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.trim_end_matches('/'),
        CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE
    );
    Ok(WarmAgentGuestPackageManifest {
        schema_version: WARM_AGENT_PACKAGE_SCHEMA_VERSION.to_string(),
        package_kind: WARM_AGENT_PACKAGE_KIND.to_string(),
        agent_file_name: WARM_AGENT_PACKAGE_BINARY_NAME.to_string(),
        llama_server_file_name: CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE.to_string(),
        build_target: WARM_AGENT_PACKAGE_BUILD_TARGET.to_string(),
        binary_sha256,
        llama_server_sha256,
        agent_binary_marker: WARM_AGENT_PACKAGE_AGENT_BINARY_MARKER.to_string(),
        llama_server_required_markers: WARM_AGENT_PACKAGE_LLAMA_SERVER_MARKERS
            .iter()
            .map(|marker| marker.to_string())
            .collect(),
        host_path,
        guest_root: CLOUD_HYPERVISOR_WARM_AGENT_GUEST_ROOT.to_string(),
        guest_path,
        llama_server_guest_path,
        required_host_path_env: CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV.to_string(),
        required_protocol_id: crate::model_runtime::WARM_AGENT_PROTOCOL_ID.to_string(),
        required_protocol_version: crate::model_runtime::WARM_AGENT_PROTOCOL_VERSION,
        requires_static_linking: true,
        package_complete: true,
        protocol_probe: None,
        produced_by: "hsk-warm-agent-package".to_string(),
        notes: vec![
            "Bind this host_path by setting HANDSHAKE_CH_WARM_AGENT_HOST_PATH.".to_string(),
            "The package target is static musl because the CH initramfs has no glibc loader."
                .to_string(),
            "The package includes sibling llama-server because hsk-warm-agent starts resident model serving from its own directory."
                .to_string(),
        ],
    })
}

pub fn manifest_for_runtime_probed_artifact(
    artifact_path: &Path,
    distro: &str,
    wsl_exe: &Path,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    let mut manifest = manifest_for_artifact(artifact_path)?;
    manifest.protocol_probe = Some(run_warm_agent_package_runtime_probe(
        artifact_path,
        distro,
        wsl_exe,
    )?);
    Ok(manifest)
}

#[cfg(test)]
pub(crate) fn manifest_for_test_recorded_probe_artifact(
    artifact_path: &Path,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    let mut manifest = manifest_for_artifact(artifact_path)?;
    manifest.protocol_probe = Some(expected_protocol_probe());
    Ok(manifest)
}

pub fn validate_linux_elf_agent_artifact(path: &Path) -> Result<(), WarmAgentPackageError> {
    validate_static_x86_64_guest_elf(path, "warm-agent package artifact")
        .map_err(WarmAgentPackageError::Path)
}

pub fn validate_warm_agent_package_candidate(
    artifact_path: &Path,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    validate_warm_agent_package_candidate_with_runtime_probe(
        artifact_path,
        DEFAULT_WSL_DISTRO,
        Path::new("wsl.exe"),
    )
}

pub fn validate_warm_agent_package_candidate_with_runtime_probe(
    artifact_path: &Path,
    distro: &str,
    wsl_exe: &Path,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    let manifest = validate_warm_agent_package_manifest_candidate(artifact_path)?;
    run_warm_agent_package_runtime_probe(artifact_path, distro, wsl_exe)?;
    Ok(manifest)
}

pub fn validate_warm_agent_package_manifest_candidate(
    artifact_path: &Path,
) -> Result<WarmAgentGuestPackageManifest, WarmAgentPackageError> {
    let actual = manifest_for_artifact(artifact_path)?;
    let manifest_path = package_manifest_path_for_agent(artifact_path)?;
    let file = File::open(&manifest_path).map_err(|error| {
        WarmAgentPackageError::Path(format!(
            "warm-agent package manifest missing or unreadable at {}: {error}",
            manifest_path.display()
        ))
    })?;
    let recorded: WarmAgentGuestPackageManifest = serde_json::from_reader(file)?;
    validate_manifest_field(
        "schema_version",
        &recorded.schema_version,
        WARM_AGENT_PACKAGE_SCHEMA_VERSION,
    )?;
    validate_manifest_field(
        "package_kind",
        &recorded.package_kind,
        WARM_AGENT_PACKAGE_KIND,
    )?;
    validate_manifest_field(
        "agent_file_name",
        &recorded.agent_file_name,
        WARM_AGENT_PACKAGE_BINARY_NAME,
    )?;
    validate_manifest_field(
        "llama_server_file_name",
        &recorded.llama_server_file_name,
        CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE,
    )?;
    validate_manifest_field(
        "build_target",
        &recorded.build_target,
        WARM_AGENT_PACKAGE_BUILD_TARGET,
    )?;
    validate_manifest_field(
        "binary_sha256",
        &recorded.binary_sha256,
        &actual.binary_sha256,
    )?;
    validate_manifest_field(
        "llama_server_sha256",
        &recorded.llama_server_sha256,
        &actual.llama_server_sha256,
    )?;
    validate_manifest_field(
        "agent_binary_marker",
        &recorded.agent_binary_marker,
        WARM_AGENT_PACKAGE_AGENT_BINARY_MARKER,
    )?;
    let expected_llama_markers: Vec<String> = WARM_AGENT_PACKAGE_LLAMA_SERVER_MARKERS
        .iter()
        .map(|marker| marker.to_string())
        .collect();
    if recorded.llama_server_required_markers != expected_llama_markers {
        return Err(WarmAgentPackageError::Path(format!(
            "warm-agent package manifest llama_server_required_markers mismatch: got {:?}, expected {:?}",
            recorded.llama_server_required_markers,
            expected_llama_markers
        )));
    }
    validate_manifest_field("host_path", &recorded.host_path, &actual.host_path)?;
    validate_manifest_field("guest_root", &recorded.guest_root, &actual.guest_root)?;
    validate_manifest_field("guest_path", &recorded.guest_path, &actual.guest_path)?;
    validate_manifest_field(
        "llama_server_guest_path",
        &recorded.llama_server_guest_path,
        &actual.llama_server_guest_path,
    )?;
    validate_manifest_field(
        "required_host_path_env",
        &recorded.required_host_path_env,
        CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV,
    )?;
    validate_manifest_field(
        "required_protocol_id",
        &recorded.required_protocol_id,
        crate::model_runtime::WARM_AGENT_PROTOCOL_ID,
    )?;
    if recorded.required_protocol_version != crate::model_runtime::WARM_AGENT_PROTOCOL_VERSION {
        return Err(WarmAgentPackageError::Path(format!(
            "warm-agent package manifest required_protocol_version mismatch: got {}, expected {}",
            recorded.required_protocol_version,
            crate::model_runtime::WARM_AGENT_PROTOCOL_VERSION
        )));
    }
    if !recorded.requires_static_linking {
        return Err(WarmAgentPackageError::Path(
            "warm-agent package manifest must require static linking".to_string(),
        ));
    }
    if !recorded.package_complete {
        return Err(WarmAgentPackageError::Path(
            "warm-agent package manifest package_complete must be true".to_string(),
        ));
    }
    validate_protocol_probe(recorded.protocol_probe.as_ref())?;
    Ok(recorded)
}

pub fn build_wsl_static_musl_package_script(
    wsl_manifest_path: &str,
    wsl_output_dir: &str,
    wsl_target_dir: &str,
    wsl_llama_server_source_path: Option<&str>,
) -> String {
    let artifact = format!("{wsl_output_dir}/{WARM_AGENT_PACKAGE_BINARY_NAME}");
    let llama_server = format!("{wsl_output_dir}/{CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE}");
    let built = format!(
        "{target}/{triple}/release/{name}",
        target = wsl_target_dir.trim_end_matches('/'),
        triple = WARM_AGENT_PACKAGE_BUILD_TARGET,
        name = WARM_AGENT_PACKAGE_BINARY_NAME
    );
    let llama_server_step = match wsl_llama_server_source_path {
        Some(source) => format!("cp {} {}", sh_quote(source), sh_quote(&llama_server)),
        None => format!(
            "test -f {llama} || {{ echo 'missing packaged {name}; pass --llama-server PATH to hsk-warm-agent-package' >&2; exit 87; }}",
            llama = sh_quote(&llama_server),
            name = CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE
        ),
    };
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
        llama_server_step,
        format!("chmod 0755 {}", sh_quote(&artifact)),
        format!("chmod 0755 {}", sh_quote(&llama_server)),
        format!(
            "printf '%s\\n' '{{\"type\":\"ping\",\"request_id\":\"{request_id}\"}}' | {artifact} >/tmp/hsk-warm-agent-package-probe.out",
            request_id = WARM_AGENT_PACKAGE_PROTOCOL_PROBE_REQUEST_ID,
            artifact = sh_quote(&artifact)
        ),
        "echo HSK_WARM_AGENT_PACKAGE_OK".to_string(),
    ]
    .join("; ")
}

pub fn package_manifest_path_for_agent(
    artifact_path: &Path,
) -> Result<PathBuf, WarmAgentPackageError> {
    let parent = artifact_path.parent().ok_or_else(|| {
        WarmAgentPackageError::Path(format!(
            "warm-agent package artifact has no parent directory: {}",
            artifact_path.display()
        ))
    })?;
    Ok(parent.join(CLOUD_HYPERVISOR_WARM_AGENT_PACKAGE_MANIFEST_FILE))
}

pub fn run_warm_agent_package_runtime_probe(
    artifact_path: &Path,
    distro: &str,
    wsl_exe: &Path,
) -> Result<WarmAgentPackageProtocolProbe, WarmAgentPackageError> {
    let wsl_artifact = windows_path_to_wsl_with_launcher(wsl_exe, distro, artifact_path)?;
    let ping = encode_warm_agent_frame(&WarmAgentHostFrame::Ping {
        request_id: WARM_AGENT_PACKAGE_PROTOCOL_PROBE_REQUEST_ID.to_string(),
    })
    .map_err(|error| {
        WarmAgentPackageError::Path(format!(
            "warm-agent runtime probe ping encode failed: {error}"
        ))
    })?;
    let script = format!(
        "printf '%s\\n' {} | {}",
        sh_quote(ping.trim_end()),
        sh_quote(&wsl_artifact)
    );
    let mut command = Command::new(wsl_exe);
    command
        .arg("-d")
        .arg(distro)
        .arg("--")
        .arg("sh")
        .arg("-lc")
        .arg(script);
    let output = command_output_with_timeout(command, WARM_AGENT_PACKAGE_RUNTIME_PROBE_TIMEOUT)?;
    if !output.status.success() {
        return Err(WarmAgentPackageError::Path(format!(
            "warm-agent runtime probe failed with status {}; stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    parse_runtime_probe_stdout(&output.stdout)
}

fn parse_runtime_probe_stdout(
    stdout: &[u8],
) -> Result<WarmAgentPackageProtocolProbe, WarmAgentPackageError> {
    let text = std::str::from_utf8(stdout).map_err(|error| {
        WarmAgentPackageError::Path(format!(
            "warm-agent runtime probe stdout was not UTF-8: {error}"
        ))
    })?;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let frame: WarmAgentGuestFrame = decode_warm_agent_frame(line).map_err(|error| {
            WarmAgentPackageError::Path(format!(
                "warm-agent runtime probe emitted invalid JSONL frame `{line}`: {error}"
            ))
        })?;
        match frame {
            WarmAgentGuestFrame::Heartbeat { request_id }
                if request_id.as_deref() == Some(WARM_AGENT_PACKAGE_PROTOCOL_PROBE_REQUEST_ID) =>
            {
                return Ok(expected_protocol_probe());
            }
            WarmAgentGuestFrame::Error {
                request_id,
                code,
                message,
            } => {
                return Err(WarmAgentPackageError::Path(format!(
                    "warm-agent runtime probe returned error frame request_id={request_id:?} code={code}: {message}"
                )));
            }
            other => {
                return Err(WarmAgentPackageError::Path(format!(
                    "warm-agent runtime probe expected heartbeat for request_id `{}`, got {:?}",
                    WARM_AGENT_PACKAGE_PROTOCOL_PROBE_REQUEST_ID, other
                )));
            }
        }
    }
    Err(WarmAgentPackageError::Path(
        "warm-agent runtime probe produced no JSONL frames".to_string(),
    ))
}

fn command_output_with_timeout(
    mut command: Command,
    timeout: Duration,
) -> Result<Output, WarmAgentPackageError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output().map_err(WarmAgentPackageError::Io);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            return Err(WarmAgentPackageError::Path(format!(
                "warm-agent runtime probe timed out after {:?}; stdout={} stderr={}",
                timeout,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub fn windows_path_to_wsl(distro: &str, path: &Path) -> Result<String, WarmAgentPackageError> {
    windows_path_to_wsl_with_launcher(Path::new("wsl.exe"), distro, path)
}

pub fn windows_path_to_wsl_with_launcher(
    wsl_exe: &Path,
    distro: &str,
    path: &Path,
) -> Result<String, WarmAgentPackageError> {
    let output = Command::new(wsl_exe)
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

fn validate_package_binary_identity(
    artifact_path: &Path,
    llama_server_path: &Path,
) -> Result<(), WarmAgentPackageError> {
    validate_binary_contains_marker(
        artifact_path,
        "warm-agent package artifact",
        WARM_AGENT_PACKAGE_AGENT_BINARY_MARKER,
    )?;
    for marker in WARM_AGENT_PACKAGE_LLAMA_SERVER_MARKERS {
        validate_binary_contains_marker(
            llama_server_path,
            "warm-agent companion llama-server",
            marker,
        )?;
    }
    Ok(())
}

fn validate_binary_contains_marker(
    path: &Path,
    artifact_label: &str,
    marker: &str,
) -> Result<(), WarmAgentPackageError> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    if bytes
        .windows(marker.as_bytes().len())
        .any(|window| window == marker.as_bytes())
    {
        return Ok(());
    }
    Err(WarmAgentPackageError::Path(format!(
        "{artifact_label} is missing required identity marker `{marker}`: {}",
        path.display()
    )))
}

fn expected_protocol_probe() -> WarmAgentPackageProtocolProbe {
    WarmAgentPackageProtocolProbe {
        probe_kind: WARM_AGENT_PACKAGE_PROTOCOL_PROBE_KIND.to_string(),
        request_id: WARM_AGENT_PACKAGE_PROTOCOL_PROBE_REQUEST_ID.to_string(),
        response_type: WARM_AGENT_PACKAGE_PROTOCOL_PROBE_RESPONSE_TYPE.to_string(),
        response_request_id: WARM_AGENT_PACKAGE_PROTOCOL_PROBE_REQUEST_ID.to_string(),
    }
}

fn validate_protocol_probe(
    probe: Option<&WarmAgentPackageProtocolProbe>,
) -> Result<(), WarmAgentPackageError> {
    let Some(probe) = probe else {
        return Err(WarmAgentPackageError::Path(
            "warm-agent package manifest missing protocol_probe".to_string(),
        ));
    };
    let expected = expected_protocol_probe();
    if probe != &expected {
        return Err(WarmAgentPackageError::Path(format!(
            "warm-agent package manifest protocol_probe mismatch: got {:?}, expected {:?}",
            probe, expected
        )));
    }
    Ok(())
}

fn validate_manifest_field(
    field: &str,
    actual: &str,
    expected: &str,
) -> Result<(), WarmAgentPackageError> {
    if actual == expected {
        return Ok(());
    }
    Err(WarmAgentPackageError::Path(format!(
        "warm-agent package manifest {field} mismatch: got `{actual}`, expected `{expected}`"
    )))
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
            Some("/tooling/llama-server"),
        );
        assert!(script.contains("x86_64-unknown-linux-musl"));
        assert!(script.contains("x86_64-linux-musl-gcc"));
        assert!(script.contains("--bin hsk-warm-agent"));
        assert!(script.contains("cp '/tooling/llama-server' '/out/llama-server'"));
        assert!(script.contains("\"type\":\"ping\""));
        assert!(script.contains("\"request_id\":\"package-probe\""));
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
        let llama_server = dir.join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE);
        std::fs::write(&artifact, marked_warm_agent_elf()).expect("write elf");
        std::fs::write(&llama_server, marked_llama_server_elf()).expect("write llama-server elf");
        let manifest = manifest_for_artifact(&artifact).expect("manifest");
        assert_eq!(manifest.schema_version, WARM_AGENT_PACKAGE_SCHEMA_VERSION);
        assert_eq!(manifest.build_target, WARM_AGENT_PACKAGE_BUILD_TARGET);
        assert_eq!(manifest.guest_path, "/warm-agent/hsk-warm-agent");
        assert_eq!(manifest.llama_server_guest_path, "/warm-agent/llama-server");
        assert_eq!(
            manifest.required_host_path_env,
            CLOUD_HYPERVISOR_WARM_AGENT_HOST_PATH_ENV
        );
        assert!(manifest.requires_static_linking);
        assert!(manifest.package_complete);
        assert!(manifest.protocol_probe.is_none());
        assert_eq!(manifest.binary_sha256.len(), 64);
        assert_eq!(manifest.llama_server_sha256.len(), 64);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn package_candidate_requires_matching_manifest_and_companion_server() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let artifact = dir.join(WARM_AGENT_PACKAGE_BINARY_NAME);
        let llama_server = dir.join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE);
        std::fs::write(&artifact, marked_warm_agent_elf()).expect("write elf");
        std::fs::write(&llama_server, marked_llama_server_elf()).expect("write llama-server elf");

        let err = validate_warm_agent_package_manifest_candidate(&artifact)
            .expect_err("manifest is required for production capability");
        assert!(format!("{err}").contains("manifest missing"));

        let manifest = manifest_for_test_recorded_probe_artifact(&artifact).expect("manifest");
        std::fs::write(
            package_manifest_path_for_agent(&artifact).expect("manifest path"),
            serde_json::to_vec_pretty(&manifest).expect("manifest json"),
        )
        .expect("write manifest");
        let validated = validate_warm_agent_package_manifest_candidate(&artifact)
            .expect("complete structural package validates");
        assert_eq!(validated.protocol_probe, manifest.protocol_probe);

        let mut stale_path = manifest.clone();
        stale_path.host_path = "/stale/hsk-warm-agent".to_string();
        std::fs::write(
            package_manifest_path_for_agent(&artifact).expect("manifest path"),
            serde_json::to_vec_pretty(&stale_path).expect("stale path json"),
        )
        .expect("write stale path manifest");
        let err = validate_warm_agent_package_manifest_candidate(&artifact)
            .expect_err("stale host path must fail closed");
        assert!(format!("{err}").contains("host_path mismatch"));

        let mut stale = manifest.clone();
        stale.llama_server_sha256 = "00".repeat(32);
        std::fs::write(
            package_manifest_path_for_agent(&artifact).expect("manifest path"),
            serde_json::to_vec_pretty(&stale).expect("stale json"),
        )
        .expect("write stale manifest");
        let err = validate_warm_agent_package_manifest_candidate(&artifact)
            .expect_err("stale companion hash must fail closed");
        assert!(format!("{err}").contains("llama_server_sha256 mismatch"));

        let mut missing_probe = manifest.clone();
        missing_probe.protocol_probe = None;
        std::fs::write(
            package_manifest_path_for_agent(&artifact).expect("manifest path"),
            serde_json::to_vec_pretty(&missing_probe).expect("missing probe json"),
        )
        .expect("write missing probe manifest");
        let err = validate_warm_agent_package_manifest_candidate(&artifact)
            .expect_err("missing protocol probe must fail closed");
        assert!(format!("{err}").contains("missing protocol_probe"));

        let mut wrong_probe = manifest.clone();
        wrong_probe
            .protocol_probe
            .as_mut()
            .expect("probe")
            .response_type = "token".to_string();
        std::fs::write(
            package_manifest_path_for_agent(&artifact).expect("manifest path"),
            serde_json::to_vec_pretty(&wrong_probe).expect("wrong probe json"),
        )
        .expect("write wrong probe manifest");
        let err = validate_warm_agent_package_manifest_candidate(&artifact)
            .expect_err("spoofed protocol probe must fail closed");
        assert!(format!("{err}").contains("protocol_probe mismatch"));

        let mut incomplete = manifest.clone();
        incomplete.package_complete = false;
        std::fs::write(
            package_manifest_path_for_agent(&artifact).expect("manifest path"),
            serde_json::to_vec_pretty(&incomplete).expect("incomplete json"),
        )
        .expect("write incomplete manifest");
        let err = validate_warm_agent_package_manifest_candidate(&artifact)
            .expect_err("incomplete manifest must fail closed");
        assert!(format!("{err}").contains("package_complete"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn marked_fake_package_fails_runtime_candidate_validation() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let artifact = dir.join(WARM_AGENT_PACKAGE_BINARY_NAME);
        let llama_server = dir.join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE);
        std::fs::write(&artifact, marked_warm_agent_elf()).expect("write marked fake agent");
        std::fs::write(&llama_server, marked_llama_server_elf())
            .expect("write marked fake llama-server");
        let manifest = manifest_for_test_recorded_probe_artifact(&artifact).expect("manifest");
        std::fs::write(
            package_manifest_path_for_agent(&artifact).expect("manifest path"),
            serde_json::to_vec_pretty(&manifest).expect("manifest json"),
        )
        .expect("write manifest");

        validate_warm_agent_package_manifest_candidate(&artifact)
            .expect("marked fake package is structurally self-consistent");
        validate_warm_agent_package_candidate(&artifact)
            .expect_err("marked fake ELF package must not pass runtime capability validation");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runtime_probe_parser_requires_one_matching_heartbeat_frame() {
        parse_runtime_probe_stdout(br#"{"type":"heartbeat","request_id":"package-probe"}"#)
            .expect("matching heartbeat probe accepted");

        let missing_id = parse_runtime_probe_stdout(br#"{"type":"heartbeat"}"#)
            .expect_err("heartbeat without request id must fail");
        assert!(
            format!("{missing_id}").contains("expected heartbeat"),
            "{missing_id}"
        );

        let fragmented = parse_runtime_probe_stdout(
            br#"{"type":"heartbeat"}
{"request_id":"package-probe"}"#,
        )
        .expect_err("grep-like fragmented fields must fail");
        assert!(
            format!("{fragmented}").contains("expected heartbeat"),
            "{fragmented}"
        );
    }

    #[test]
    fn random_static_elf_pair_cannot_self_manifest_into_package_support() {
        let dir =
            std::env::temp_dir().join(format!("hsk-warm-agent-{}", uuid::Uuid::now_v7().simple()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let artifact = dir.join(WARM_AGENT_PACKAGE_BINARY_NAME);
        let llama_server = dir.join(CLOUD_HYPERVISOR_WARM_AGENT_LLAMA_SERVER_FILE);
        std::fs::write(&artifact, minimal_static_x86_64_elf()).expect("write random elf");
        std::fs::write(&llama_server, minimal_static_x86_64_elf()).expect("write random server");

        let err = manifest_for_test_recorded_probe_artifact(&artifact)
            .expect_err("random ELF pair must not produce a support-enabling manifest");
        assert!(format!("{err}").contains("identity marker"));
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

    fn marked_warm_agent_elf() -> Vec<u8> {
        let mut elf = minimal_static_x86_64_elf();
        elf.extend_from_slice(WARM_AGENT_PACKAGE_AGENT_BINARY_MARKER.as_bytes());
        elf
    }

    fn marked_llama_server_elf() -> Vec<u8> {
        let mut elf = minimal_static_x86_64_elf();
        for marker in WARM_AGENT_PACKAGE_LLAMA_SERVER_MARKERS {
            elf.extend_from_slice(marker.as_bytes());
        }
        elf
    }
}
