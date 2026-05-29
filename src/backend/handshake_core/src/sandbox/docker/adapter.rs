use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use uuid::Uuid;

use super::kernel_003_bridge::{
    docker_exec_args, docker_run_args, parse_docker_exit_code, parse_docker_status,
    run_docker_command,
};
use crate::sandbox::{
    AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass, DOCKER_ADAPTER_ID,
};

const GPU_PROBE_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerConfig {
    docker_exe: PathBuf,
    command_timeout_ms: u64,
}

impl DockerConfig {
    pub fn new(docker_exe: impl Into<PathBuf>) -> Self {
        Self {
            docker_exe: docker_exe.into(),
            command_timeout_ms: 30_000,
        }
    }

    pub fn docker_exe(&self) -> &Path {
        &self.docker_exe
    }

    pub fn command_timeout_ms(&self) -> u64 {
        self.command_timeout_ms
    }

    pub fn with_command_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.command_timeout_ms = timeout_ms;
        self
    }
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self::new("docker")
    }
}

#[derive(Debug, Clone)]
pub struct DockerAdapter {
    config: DockerConfig,
    gpu_cache: Arc<Mutex<DockerGpuProbeCache>>,
}

impl DockerAdapter {
    pub async fn try_new(config: DockerConfig) -> Result<Self, SandboxAdapterError> {
        verify_docker_available(&config).await?;
        let gpu_passthrough = probe_gpu_passthrough(&config).await;
        Ok(Self {
            config,
            gpu_cache: Arc::new(Mutex::new(DockerGpuProbeCache::new(gpu_passthrough))),
        })
    }

    pub fn with_config_and_gpu_for_tests(
        config: DockerConfig,
        gpu_passthrough: GpuPassthrough,
    ) -> Self {
        Self {
            config,
            gpu_cache: Arc::new(Mutex::new(DockerGpuProbeCache::new(gpu_passthrough))),
        }
    }

    pub fn config(&self) -> &DockerConfig {
        &self.config
    }

    pub fn kill_args(container_id: &str, signal: Signal) -> Vec<String> {
        match signal {
            Signal::Term => vec![
                "stop".to_string(),
                "--timeout".to_string(),
                "10".to_string(),
                container_id.to_string(),
            ],
            Signal::Kill => signal_kill_args(container_id, "KILL"),
            Signal::Int => signal_kill_args(container_id, "INT"),
        }
    }

    fn ensure_handle(&self, handle: &ProcessHandle) -> Result<(), SandboxAdapterError> {
        if handle.adapter_id != AdapterId::new(DOCKER_ADAPTER_ID) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        Ok(())
    }

    async fn ensure_runtime_available(&self) -> Result<(), SandboxAdapterError> {
        verify_docker_available(&self.config).await
    }
}

#[async_trait]
impl SandboxAdapter for DockerAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        self.ensure_runtime_available().await?;
        let container_name = format!("hsk-{}", Uuid::now_v7().simple());
        let args = docker_run_args(&spec, &container_name)?;
        let output = run_docker_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if output.exit_code != 0 {
            return Err(spawn_failed(format!(
                "docker run failed: {}",
                output.stderr_text()
            )));
        }
        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if container_id.is_empty() {
            return Err(spawn_failed("docker run did not return a container id"));
        }
        Ok(ProcessHandle::new(
            AdapterId::new(DOCKER_ADAPTER_ID),
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
        let args = docker_exec_args(&handle.sandbox_internal_id, &cmd)?;
        let output = run_docker_command(
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
        handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        Err(spawn_failed(
            "post-spawn fs_bind unsupported on Docker; declare in ProcessSpec.binds",
        ))
    }

    async fn copy_in(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let args = vec![
            "cp".to_string(),
            host_path.to_string_lossy().to_string(),
            format!(
                "{}:{}",
                handle.sandbox_internal_id,
                guest_path.to_string_lossy()
            ),
        ];
        let output =
            run_docker_command(&self.config, &args, None, Some(self.config.command_timeout_ms()))
                .await?;
        if output.exit_code != 0 {
            return Err(SandboxAdapterError::CopyFailed {
                adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
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
            host_path.to_string_lossy().to_string(),
        ];
        let output =
            run_docker_command(&self.config, &args, None, Some(self.config.command_timeout_ms()))
                .await?;
        if output.exit_code != 0 {
            return Err(SandboxAdapterError::CopyFailed {
                adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
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
                    "network".to_string(),
                    "disconnect".to_string(),
                    "--force".to_string(),
                    "bridge".to_string(),
                    handle.sandbox_internal_id.clone(),
                ];
                let output = run_docker_command(
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
                "post-spawn Docker net_policy changes are not supported yet; declare the network policy before spawn in ProcessSpec.net_policy",
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
        let output = run_docker_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if output.exit_code != 0 {
            return Err(spawn_failed(format!(
                "docker kill/stop failed: {}",
                output.stderr_text()
            )));
        }

        let rm_args = vec![
            "rm".to_string(),
            "--force".to_string(),
            handle.sandbox_internal_id.clone(),
        ];
        let cleanup = run_docker_command(
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
                "docker cleanup failed after kill/stop: {}",
                cleanup.stderr_text()
            )))
        }
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let status_args = vec![
            "inspect".to_string(),
            "--format".to_string(),
            "{{.State.Status}}".to_string(),
            handle.sandbox_internal_id.clone(),
        ];
        let status = run_docker_command(
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
        Ok(parse_docker_status(
            &String::from_utf8_lossy(&status.stdout),
            exit_code,
        ))
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let args = vec![
            "inspect".to_string(),
            "--format".to_string(),
            "{{.State.ExitCode}}".to_string(),
            handle.sandbox_internal_id.clone(),
        ];
        let output = run_docker_command(
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
        parse_docker_exit_code(&String::from_utf8_lossy(&output.stdout))
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
            adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
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

async fn verify_docker_available(config: &DockerConfig) -> Result<(), SandboxAdapterError> {
    let args = vec![
        "version".to_string(),
        "--format".to_string(),
        "{{.Server.Version}}".to_string(),
    ];
    let output = run_docker_command(config, &args, None, Some(config.command_timeout_ms())).await?;
    if output.exit_code != 0 {
        return Err(SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
            reason: format!(
                "docker unavailable or Docker daemon unreachable: {}",
                output.stderr_text()
            ),
        });
    }
    Ok(())
}

async fn probe_gpu_passthrough(config: &DockerConfig) -> GpuPassthrough {
    let args = vec![
        "info".to_string(),
        "--format".to_string(),
        "{{json .Runtimes}}".to_string(),
    ];
    match run_docker_command(config, &args, None, Some(config.command_timeout_ms())).await {
        Ok(output) if output.exit_code == 0 => {
            parse_docker_runtimes_for_gpu(&String::from_utf8_lossy(&output.stdout))
        }
        _ => GpuPassthrough::None,
    }
}

fn parse_docker_runtimes_for_gpu(text: &str) -> GpuPassthrough {
    if text.to_ascii_lowercase().contains("nvidia") {
        GpuPassthrough::NvidiaCuda
    } else {
        GpuPassthrough::None
    }
}

fn signal_kill_args(container_id: &str, signal: &str) -> Vec<String> {
    vec![
        "kill".to_string(),
        "--signal".to_string(),
        signal.to_string(),
        container_id.to_string(),
    ]
}

fn spawn_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn net_policy_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::NetPolicyApplyFailed {
        adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

#[derive(Debug, Clone)]
struct DockerGpuProbeCache {
    value: GpuPassthrough,
    probed_at: Instant,
}

impl DockerGpuProbeCache {
    fn new(value: GpuPassthrough) -> Self {
        Self {
            value,
            probed_at: Instant::now(),
        }
    }

    fn value(&self) -> GpuPassthrough {
        self.value
    }

    fn is_fresh(&self) -> bool {
        self.probed_at.elapsed() <= GPU_PROBE_CACHE_TTL
    }
}
