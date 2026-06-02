use std::{
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};

use bytes::Bytes;
use tokio::{io::AsyncWriteExt, process::Command as TokioCommand};

use super::adapter::Wsl2PodmanConfig;
use crate::sandbox::{
    AdapterId, BindMode, BindSpec, NetAllowlistEntry, NetPolicy, ProcessSpec, ProcessStatus,
    ResourceLimits, SandboxAdapterError, WSL2_PODMAN_ADAPTER_ID,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub exit_code: i32,
    pub stdout: Bytes,
    pub stderr: Bytes,
    pub duration_ms: u64,
}

impl CliOutput {
    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }
}

pub fn podman_run_args(spec: &ProcessSpec) -> Result<Vec<String>, SandboxAdapterError> {
    if spec.cmd.is_empty() {
        return Err(spawn_failed("ProcessSpec.cmd must not be empty"));
    }
    validate_supported_resource_limits(&spec.resource_limits)?;

    let mut args = vec![
        "--remote=false".to_string(),
        "run".to_string(),
        "-d".to_string(),
        "--userns".to_string(),
        "keep-id".to_string(),
        "--read-only".to_string(),
        "--tmpfs".to_string(),
        "/tmp:rw,noexec,nosuid,nodev,mode=1777".to_string(),
        "--pids-limit".to_string(),
        "4096".to_string(),
        "--network".to_string(),
        network_mode(&spec.net_policy)?,
    ];

    if let Some(cwd) = &spec.cwd {
        args.push("-w".to_string());
        args.push(guest_path_to_podman_path(cwd)?);
    }

    for (key, value) in &spec.env {
        args.push("-e".to_string());
        args.push(format!("{key}={value}"));
    }

    for bind in &spec.binds {
        args.push("-v".to_string());
        args.push(bind_arg(bind)?);
    }

    if let Some(memory_bytes) = spec.resource_limits.memory_bytes {
        args.push("--memory".to_string());
        args.push(memory_bytes.to_string());
    }
    if let Some(cpu_cores) = spec.resource_limits.cpu_cores {
        args.push("--cpus".to_string());
        args.push(cpu_cores.to_string());
    }

    args.push(spec.image_or_root.as_str().to_string());
    args.extend(spec.cmd.iter().cloned());
    Ok(args)
}

fn validate_supported_resource_limits(limits: &ResourceLimits) -> Result<(), SandboxAdapterError> {
    if limits.disk_read_bytes_per_sec.is_some()
        || limits.disk_write_bytes_per_sec.is_some()
        || limits.net_bandwidth_bytes_per_sec.is_some()
    {
        return Err(spawn_failed(
            "WSL2-Podman ResourceLimits disk/net bytes-per-second token-bucket limits are not \
             enforceable by this adapter path yet; refusing to silently ignore requested \
             per-device rate limits",
        ));
    }
    Ok(())
}

pub fn podman_exec_args(
    container_id: &str,
    cmd: &crate::sandbox::Command,
) -> Result<Vec<String>, SandboxAdapterError> {
    if cmd.argv.is_empty() {
        return Err(spawn_failed("Command.argv must not be empty"));
    }
    let mut args = vec!["--remote=false".to_string(), "exec".to_string()];
    if cmd.stdin.is_some() {
        args.push("--interactive".to_string());
    }
    for (key, value) in &cmd.env_overlay {
        args.push("-e".to_string());
        args.push(format!("{key}={value}"));
    }
    args.push(container_id.to_string());
    args.extend(cmd.argv.iter().cloned());
    Ok(args)
}

pub fn windows_path_to_wsl_mount_path(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        let rest = raw[2..].trim_start_matches('/');
        if rest.is_empty() {
            format!("/mnt/{drive}")
        } else {
            format!("/mnt/{drive}/{rest}")
        }
    } else {
        raw
    }
}

pub fn parse_podman_status(status: &str, exit_code: Option<i32>) -> ProcessStatus {
    match status.trim().to_ascii_lowercase().as_str() {
        "running" | "paused" | "restarting" => ProcessStatus::Running,
        "exited" | "stopped" => ProcessStatus::Exited {
            code: exit_code.unwrap_or(0),
        },
        "dead" | "removing" => ProcessStatus::Orphaned,
        other => ProcessStatus::FailedToStart {
            reason: format!("podman container status {other}"),
        },
    }
}

pub fn parse_podman_exit_code(text: &str) -> Result<Option<i32>, SandboxAdapterError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i32>()
        .map(Some)
        .map_err(|error| spawn_failed(format!("invalid podman exit code `{trimmed}`: {error}")))
}

pub fn parse_podman_rootless_info(text: &str) -> Result<bool, SandboxAdapterError> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(spawn_failed(format!(
            "invalid podman rootless probe output `{other}`"
        ))),
    }
}

pub async fn run_podman_command(
    config: &Wsl2PodmanConfig,
    args: &[String],
    stdin: Option<Bytes>,
    timeout_ms: Option<u64>,
) -> Result<CliOutput, SandboxAdapterError> {
    let mut wsl_args = vec!["podman".to_string()];
    wsl_args.extend_from_slice(args);
    run_wsl_distribution_command(config, &wsl_args, stdin, timeout_ms).await
}

pub async fn run_wsl_distribution_command(
    config: &Wsl2PodmanConfig,
    args: &[String],
    stdin: Option<Bytes>,
    timeout_ms: Option<u64>,
) -> Result<CliOutput, SandboxAdapterError> {
    let mut full_args = vec![
        "-d".to_string(),
        config.distro().to_string(),
        "--".to_string(),
    ];
    full_args.extend_from_slice(args);
    run_host_command(config.wsl_exe(), &full_args, stdin, timeout_ms).await
}

pub async fn run_host_command(
    executable: &Path,
    args: &[String],
    stdin: Option<Bytes>,
    timeout_ms: Option<u64>,
) -> Result<CliOutput, SandboxAdapterError> {
    let start = Instant::now();
    let mut command = TokioCommand::new(executable);
    command
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    hide_command_window(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            reason: format!(
                "failed to spawn `{}`: {error}",
                executable.to_string_lossy()
            ),
        })?;

    if let Some(input) = stdin {
        if let Some(mut child_stdin) = child.stdin.take() {
            tokio::spawn(async move {
                let _ = child_stdin.write_all(&input).await;
            });
        }
    }

    let wait = child.wait_with_output();
    let output = if let Some(timeout_ms) = timeout_ms {
        tokio::time::timeout(Duration::from_millis(timeout_ms), wait)
            .await
            .map_err(|_| spawn_failed(format!("command timed out after {timeout_ms}ms")))?
    } else {
        wait.await
    }
    .map_err(|error| spawn_failed(format!("command wait failed: {error}")))?;

    Ok(CliOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: Bytes::from(output.stdout),
        stderr: Bytes::from(output.stderr),
        duration_ms: start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    })
}

fn bind_arg(bind: &BindSpec) -> Result<String, SandboxAdapterError> {
    let host = windows_path_to_wsl_mount_path(&bind.host_path);
    let guest = guest_path_to_podman_path(&bind.guest_path)?;
    let mode = match bind.mode {
        BindMode::ReadOnly => "ro",
        BindMode::ReadWrite => "rw",
        BindMode::NoExec => "ro,noexec",
    };
    Ok(format!("{host}:{guest}:{mode}"))
}

fn guest_path_to_podman_path(path: &Path) -> Result<String, SandboxAdapterError> {
    let value = path.to_string_lossy().replace('\\', "/");
    if !value.starts_with('/') {
        return Err(SandboxAdapterError::BindGuestPathInvalid {
            guest_path: path.to_path_buf(),
            reason: "guest path must be absolute in the container".to_string(),
        });
    }
    Ok(value)
}

/// Podman `--network` value for loopback-only access to host services
/// (Master Spec v02.188 §3.5.1 #5 "loopback-only" network policy honesty,
/// §3.5.7). slirp4netns user-mode networking with `allow_host_loopback=true`
/// is the field-standard rootless-podman path that lets the workload reach a
/// host service bound on 127.0.0.1 (e.g. a llama-server on 127.0.0.1:11434)
/// WITHOUT granting external/internet egress. This is the honest realization of
/// `NetPolicy::LoopbackOnly` — it MUST NOT collapse to `none` (which is the
/// `DenyAll` value and would silently deny the loopback the policy promised).
const PODMAN_LOOPBACK_NETWORK_MODE: &str =
    "slirp4netns:port_handler=slirp4netns,allow_host_loopback=true";

fn network_mode(policy: &NetPolicy) -> Result<String, SandboxAdapterError> {
    match policy {
        NetPolicy::DenyAll => Ok("none".to_string()),
        // Honest loopback path: reach host 127.0.0.1 services, no external egress.
        // NEVER downgrade to "none" — that is the DenyAll value and would be a
        // silent network-policy downgrade (spec §3.5.1 #5 honesty requirement).
        NetPolicy::LoopbackOnly => Ok(PODMAN_LOOPBACK_NETWORK_MODE.to_string()),
        NetPolicy::Allowlist(entries) => {
            if entries.iter().all(is_loopback_allowlist_entry) {
                // A loopback-only allowlist (e.g. 127.0.0.1:11434) expects to
                // reach a host loopback service; honor it with the same honest
                // slirp4netns loopback mode rather than silently denying it.
                Ok(PODMAN_LOOPBACK_NETWORK_MODE.to_string())
            } else {
                Err(SandboxAdapterError::NetPolicyApplyFailed {
                    adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
                    reason: "Podman allowlist spawn currently supports loopback entries only; external allowlist entries require post-spawn firewall seeding and fail closed".to_string(),
                })
            }
        }
    }
}

fn is_loopback_allowlist_entry(entry: &NetAllowlistEntry) -> bool {
    matches!(
        entry.host.as_str(),
        "127.0.0.1" | "localhost" | "::1" | "[::1]"
    )
}

fn spawn_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn hide_command_window(command: &mut TokioCommand) {
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}
