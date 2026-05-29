use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;

use super::gpu_probe::{probe_gpu_passthrough, GpuProbeCache};
use super::podman_cli::{
    parse_podman_exit_code, parse_podman_rootless_info, parse_podman_status, podman_exec_args,
    podman_run_args, run_podman_command, windows_path_to_wsl_mount_path,
};
use super::wsl_detection::{default_wsl_exe, verify_wsl2_distro};
use crate::sandbox::{
    AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass,
};

pub const WSL2_PODMAN_ADAPTER_ID: &str = "wsl2_podman";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wsl2PodmanConfig {
    distro: String,
    wsl_exe: PathBuf,
    command_timeout_ms: u64,
}

impl Wsl2PodmanConfig {
    pub fn new(distro: impl Into<String>, wsl_exe: impl Into<PathBuf>) -> Self {
        Self {
            distro: distro.into(),
            wsl_exe: wsl_exe.into(),
            command_timeout_ms: 30_000,
        }
    }

    pub fn for_distro(distro: impl Into<String>) -> Self {
        Self::new(distro, default_wsl_exe())
    }

    pub fn distro(&self) -> &str {
        &self.distro
    }

    pub fn wsl_exe(&self) -> &Path {
        &self.wsl_exe
    }

    pub fn command_timeout_ms(&self) -> u64 {
        self.command_timeout_ms
    }

    pub fn with_command_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.command_timeout_ms = timeout_ms;
        self
    }
}

impl Default for Wsl2PodmanConfig {
    fn default() -> Self {
        Self::for_distro("Ubuntu")
    }
}

#[derive(Debug, Clone)]
pub struct Wsl2PodmanAdapter {
    config: Wsl2PodmanConfig,
    gpu_cache: Arc<Mutex<GpuProbeCache>>,
}

impl Wsl2PodmanAdapter {
    pub async fn try_new(config: Wsl2PodmanConfig) -> Result<Self, SandboxAdapterError> {
        verify_wsl2_distro(&config).await?;
        verify_podman_available(&config).await?;
        let gpu_passthrough = probe_gpu_passthrough(&config).await;
        Ok(Self {
            config,
            gpu_cache: Arc::new(Mutex::new(GpuProbeCache::new(gpu_passthrough))),
        })
    }

    pub fn with_config_and_gpu_for_tests(
        config: Wsl2PodmanConfig,
        gpu_passthrough: GpuPassthrough,
    ) -> Self {
        Self {
            config,
            gpu_cache: Arc::new(Mutex::new(GpuProbeCache::new(gpu_passthrough))),
        }
    }

    pub fn config(&self) -> &Wsl2PodmanConfig {
        &self.config
    }

    pub fn kill_args(container_id: &str, signal: Signal) -> Vec<String> {
        match signal {
            Signal::Term => vec![
                "--remote=false".to_string(),
                "stop".to_string(),
                "--time".to_string(),
                "10".to_string(),
                container_id.to_string(),
            ],
            Signal::Kill => signal_kill_args(container_id, "KILL"),
            Signal::Int => signal_kill_args(container_id, "INT"),
        }
    }

    fn ensure_handle(&self, handle: &ProcessHandle) -> Result<(), SandboxAdapterError> {
        if handle.adapter_id != AdapterId::new(WSL2_PODMAN_ADAPTER_ID) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        Ok(())
    }

    async fn ensure_runtime_available(&self) -> Result<(), SandboxAdapterError> {
        verify_wsl2_distro(&self.config).await?;
        verify_podman_available(&self.config).await
    }
}

#[async_trait]
impl SandboxAdapter for Wsl2PodmanAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        self.ensure_runtime_available().await?;
        let args = podman_run_args(&spec)?;
        let output = run_podman_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if output.exit_code != 0 {
            return Err(spawn_failed(format!(
                "podman run failed: {}",
                output.stderr_text()
            )));
        }
        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if container_id.is_empty() {
            return Err(spawn_failed("podman run did not return a container id"));
        }
        Ok(ProcessHandle::new(
            AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            None,
            container_id,
        ))
    }

    async fn exec(
        &self,
        handle: &ProcessHandle,
        cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let args = podman_exec_args(&handle.sandbox_internal_id, &cmd)?;
        let output = run_podman_command(
            &self.config,
            &args,
            cmd.stdin.clone(),
            cmd.timeout_ms.or(Some(self.config.command_timeout_ms())),
        )
        .await?;
        Ok(ExecResult {
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms: output.duration_ms,
        })
    }

    async fn fs_bind(
        &self,
        _handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        Err(spawn_failed(
            "post-spawn fs_bind unsupported on Podman; declare in ProcessSpec.binds",
        ))
    }

    async fn copy_in(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        // Podman runs inside WSL2, so the host side of `podman cp` must be a
        // WSL mount path (/mnt/<drive>/...), not a raw Windows path.
        let args = vec![
            "cp".to_string(),
            windows_path_to_wsl_mount_path(&host_path),
            format!(
                "{}:{}",
                handle.sandbox_internal_id,
                guest_path.to_string_lossy()
            ),
        ];
        let output =
            run_podman_command(&self.config, &args, None, Some(self.config.command_timeout_ms()))
                .await?;
        if output.exit_code != 0 {
            return Err(SandboxAdapterError::CopyFailed {
                adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
                reason: output.stderr_text(),
            });
        }
        Ok(())
    }

    async fn copy_out(
        &self,
        handle: &ProcessHandle,
        guest_path: PathBuf,
        host_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let args = vec![
            "cp".to_string(),
            format!(
                "{}:{}",
                handle.sandbox_internal_id,
                guest_path.to_string_lossy()
            ),
            windows_path_to_wsl_mount_path(&host_path),
        ];
        let output =
            run_podman_command(&self.config, &args, None, Some(self.config.command_timeout_ms()))
                .await?;
        if output.exit_code != 0 {
            return Err(SandboxAdapterError::CopyFailed {
                adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
                reason: output.stderr_text(),
            });
        }
        Ok(())
    }

    async fn net_policy(
        &self,
        handle: &ProcessHandle,
        policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        match policy {
            NetPolicy::DenyAll => {
                let args = vec![
                    "--remote=false".to_string(),
                    "network".to_string(),
                    "disconnect".to_string(),
                    "--force".to_string(),
                    "podman".to_string(),
                    handle.sandbox_internal_id.clone(),
                ];
                let output = run_podman_command(
                    &self.config,
                    &args,
                    None,
                    Some(self.config.command_timeout_ms()),
                )
                .await?;
                if output.exit_code == 0 {
                    Ok(())
                } else {
                    Err(net_policy_failed(output.stderr_text()))
                }
            }
            NetPolicy::LoopbackOnly | NetPolicy::Allowlist(_) => Err(net_policy_failed(
                "post-spawn Podman net_policy changes are not supported yet; declare the network policy before spawn in ProcessSpec.net_policy",
            )),
        }
    }

    async fn kill(
        &self,
        handle: &ProcessHandle,
        signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let args = Self::kill_args(&handle.sandbox_internal_id, signal);
        let output = run_podman_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if output.exit_code == 0 {
            let rm_args = vec![
                "--remote=false".to_string(),
                "rm".to_string(),
                "--force".to_string(),
                handle.sandbox_internal_id.clone(),
            ];
            let cleanup = run_podman_command(
                &self.config,
                &rm_args,
                None,
                Some(self.config.command_timeout_ms()),
            )
            .await?;
            if cleanup.exit_code == 0 {
                Ok(())
            } else {
                Err(spawn_failed(format!(
                    "podman cleanup failed after kill/stop: {}",
                    cleanup.stderr_text()
                )))
            }
        } else {
            Err(spawn_failed(format!(
                "podman kill/stop failed: {}",
                output.stderr_text()
            )))
        }
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let status_args = vec![
            "--remote=false".to_string(),
            "inspect".to_string(),
            "--format".to_string(),
            "{{.State.Status}}".to_string(),
            handle.sandbox_internal_id.clone(),
        ];
        let status = run_podman_command(
            &self.config,
            &status_args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if status.exit_code != 0 {
            return Ok(ProcessStatus::Orphaned);
        }
        let exit_code = self.exit_code(handle).await?;
        Ok(parse_podman_status(
            &String::from_utf8_lossy(&status.stdout),
            exit_code,
        ))
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let args = vec![
            "--remote=false".to_string(),
            "inspect".to_string(),
            "--format".to_string(),
            "{{.State.ExitCode}}".to_string(),
            handle.sandbox_internal_id.clone(),
        ];
        let output = run_podman_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if output.exit_code != 0 {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        parse_podman_exit_code(&String::from_utf8_lossy(&output.stdout))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        let gpu_passthrough = self
            .gpu_cache
            .lock()
            .map(|cache| {
                let _ = cache.is_fresh();
                cache.value()
            })
            .unwrap_or(GpuPassthrough::None);
        AdapterCapabilities {
            adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            runtime_available: true,
            filesystem_isolation_strength: IsolationStrength::Strong,
            network_isolation_strength: IsolationStrength::Strong,
            gpu_passthrough,
            stdio_throughput_class: ThroughputClass::High,
            win32_native_fidelity: false,
            cross_machine_portable: true,
            isolation_tier: IsolationTier::Tier1Container,
            requires_nested_virt: false,
            supports_snapshot: false,
        }
    }
}

async fn verify_podman_available(config: &Wsl2PodmanConfig) -> Result<(), SandboxAdapterError> {
    let args = vec!["podman".to_string(), "--version".to_string()];
    let output = super::podman_cli::run_wsl_distribution_command(
        config,
        &args,
        None,
        Some(config.command_timeout_ms()),
    )
    .await?;
    if output.exit_code != 0 {
        return Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            reason: format!(
                "podman unavailable inside WSL distro `{}`: {}",
                config.distro(),
                output.stderr_text()
            ),
        });
    }

    let rootless_args = vec![
        "podman".to_string(),
        "info".to_string(),
        "--format".to_string(),
        "{{.Host.Security.Rootless}}".to_string(),
    ];
    let rootless = super::podman_cli::run_wsl_distribution_command(
        config,
        &rootless_args,
        None,
        Some(config.command_timeout_ms()),
    )
    .await?;
    if rootless.exit_code != 0 {
        return Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            reason: format!(
                "podman rootless probe failed inside WSL distro `{}`: {}",
                config.distro(),
                rootless.stderr_text()
            ),
        });
    }
    let is_rootless = parse_podman_rootless_info(&String::from_utf8_lossy(&rootless.stdout))?;
    if !is_rootless {
        return Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            reason: format!(
                "podman inside WSL distro `{}` is not running rootless",
                config.distro()
            ),
        });
    }

    Ok(())
}

fn signal_kill_args(container_id: &str, signal: &str) -> Vec<String> {
    vec![
        "--remote=false".to_string(),
        "kill".to_string(),
        "--signal".to_string(),
        signal.to_string(),
        container_id.to_string(),
    ]
}

fn spawn_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn net_policy_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::NetPolicyApplyFailed {
        adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
        reason: reason.to_string(),
    }
}
