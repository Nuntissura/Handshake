use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use bytes::Bytes;
use tokio::{io::AsyncWriteExt, process::Command as TokioCommand};

use crate::sandbox::{
    AdapterId, BindMode, BindSpec, NetAllowlistEntry, NetPolicy, ProcessSpec, ProcessStatus,
    SandboxAdapterError, DOCKER_ADAPTER_ID,
};

use super::adapter::DockerConfig;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerCliOutput {
    pub exit_code: i32,
    pub stdout: Bytes,
    pub stderr: Bytes,
    pub duration_ms: u64,
}

impl DockerCliOutput {
    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kernel003ContainerBridgeProvenance {
    pub adapter_id: &'static str,
    pub tier_label: &'static str,
    pub source_file: &'static str,
    pub status: &'static str,
}

pub fn kernel_003_container_bridge_provenance() -> Kernel003ContainerBridgeProvenance {
    Kernel003ContainerBridgeProvenance {
        adapter_id: "hard_isolation_container",
        tier_label: "container",
        source_file: "src/backend/handshake_core/src/kernel/sandbox/hard_isolation_container.rs",
        status: "non_executing_stub_no_docket_adapter_found",
    }
}

// MT-047 requested a bridge to the KERNEL-003 DocketAdapter. Local product
// evidence instead shows KERNEL-003 has only ContainerAdapterStub, so these
// argv helpers become the explicit compatibility surface for MT-054 callers.
pub fn docker_run_args(
    spec: &ProcessSpec,
    container_name: &str,
) -> Result<Vec<String>, SandboxAdapterError> {
    if spec.cmd.is_empty() {
        return Err(spawn_failed("ProcessSpec.cmd must not be empty"));
    }

    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.to_string(),
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
        args.push(guest_path_to_docker_path(cwd)?);
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

pub fn docker_exec_args(
    container_id: &str,
    cmd: &crate::sandbox::Command,
) -> Result<Vec<String>, SandboxAdapterError> {
    if cmd.argv.is_empty() {
        return Err(spawn_failed("Command.argv must not be empty"));
    }
    let mut args = vec!["exec".to_string()];
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

pub fn docker_host_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn parse_docker_status(status: &str, exit_code: Option<i32>) -> ProcessStatus {
    match status.trim().to_ascii_lowercase().as_str() {
        "running" | "paused" | "restarting" => ProcessStatus::Running,
        "exited" => ProcessStatus::Exited {
            code: exit_code.unwrap_or(0),
        },
        "dead" | "removing" => ProcessStatus::Orphaned,
        other => ProcessStatus::FailedToStart {
            reason: format!("docker container status {other}"),
        },
    }
}

pub fn parse_docker_exit_code(text: &str) -> Result<Option<i32>, SandboxAdapterError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i32>()
        .map(Some)
        .map_err(|error| spawn_failed(format!("invalid docker exit code `{trimmed}`: {error}")))
}

pub async fn run_docker_command(
    config: &DockerConfig,
    args: &[String],
    stdin: Option<Bytes>,
    timeout_ms: Option<u64>,
) -> Result<DockerCliOutput, SandboxAdapterError> {
    run_host_command(config.docker_exe(), args, stdin, timeout_ms).await
}

pub async fn run_host_command(
    executable: &Path,
    args: &[String],
    stdin: Option<Bytes>,
    timeout_ms: Option<u64>,
) -> Result<DockerCliOutput, SandboxAdapterError> {
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
            adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
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

    Ok(DockerCliOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: Bytes::from(output.stdout),
        stderr: Bytes::from(output.stderr),
        duration_ms: start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    })
}

fn bind_arg(bind: &BindSpec) -> Result<String, SandboxAdapterError> {
    let host = docker_host_path(&bind.host_path);
    let guest = guest_path_to_docker_path(&bind.guest_path)?;
    let mode = match bind.mode {
        BindMode::ReadOnly => "ro",
        BindMode::ReadWrite => "rw",
        BindMode::NoExec => "ro,noexec",
    };
    Ok(format!("{host}:{guest}:{mode}"))
}

fn guest_path_to_docker_path(path: &Path) -> Result<String, SandboxAdapterError> {
    let value = path.to_string_lossy().replace('\\', "/");
    if !value.starts_with('/') {
        return Err(SandboxAdapterError::BindGuestPathInvalid {
            guest_path: PathBuf::from(path),
            reason: "guest path must be absolute in the container".to_string(),
        });
    }
    Ok(value)
}

/// Loud, typed failure for any loopback-only network policy on the Docker
/// bridge. Master Spec v02.188 §3.5.1 #5 requires `NetPolicy::LoopbackOnly`
/// to actually reach host 127.0.0.1 services. Unlike rootless podman, the
/// Docker CLI exposes no slirp4netns / `allow_host_loopback` mode at this argv
/// layer (`--network=none` denies all traffic; `--network=host` would drop ALL
/// isolation — neither is an honest "loopback-only" realization). Rather than
/// silently downgrade to `none` (the DenyAll value), this adapter fails closed
/// and LOUD so callers can route loopback workloads to the WSL2-podman adapter,
/// which honestly wires slirp4netns loopback.
fn docker_loopback_unsupported() -> SandboxAdapterError {
    SandboxAdapterError::NetPolicyApplyFailed {
        adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
        reason: "Docker bridge cannot honestly realize NetPolicy::LoopbackOnly: the Docker CLI has no slirp4netns/allow_host_loopback mode at this layer (--network=none denies all, --network=host drops isolation). Refusing to silently downgrade loopback-only to none; route loopback workloads to the WSL2-podman adapter or implement a host-loopback proxy".to_string(),
    }
}

fn network_mode(policy: &NetPolicy) -> Result<String, SandboxAdapterError> {
    match policy {
        NetPolicy::DenyAll => Ok("none".to_string()),
        // NEVER silently map LoopbackOnly to "none" (the DenyAll value): that is
        // a silent network-policy downgrade (spec §3.5.1 #5 honesty). Fail loud.
        NetPolicy::LoopbackOnly => Err(docker_loopback_unsupported()),
        NetPolicy::Allowlist(entries) => {
            if entries.iter().all(is_loopback_allowlist_entry) {
                // A loopback-only allowlist (e.g. 127.0.0.1:11434) is a
                // loopback-reach request; the Docker bridge cannot honestly
                // honor it here, so it fails loud rather than denying silently.
                Err(docker_loopback_unsupported())
            } else {
                Err(SandboxAdapterError::NetPolicyApplyFailed {
                    adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
                    reason: "Docker external allowlist spawn requires firewall seeding and must fail closed for now".to_string(),
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
        adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
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
