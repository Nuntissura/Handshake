use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::gpu_probe::{probe_gpu_passthrough, GpuProbeCache};
use super::podman_cli::{
    parse_podman_exit_code, parse_podman_rootless_info, parse_podman_status, podman_exec_args,
    podman_run_args_for_container, run_podman_command, windows_path_to_wsl_mount_path,
};
use super::wsl_detection::{default_wsl_exe, verify_wsl2_distro};
use crate::sandbox::{
    AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    RestartCleanupOutcome, SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass,
};

pub const WSL2_PODMAN_ADAPTER_ID: &str = "wsl2_podman";
pub(super) const PODMAN_PROCESS_OWNER_LABEL: &str = "io.handshake.process-id";

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

    fn restart_cleanup_args(container_id: &str) -> Option<Vec<String>> {
        if !is_valid_durable_container_id(container_id) {
            return None;
        }
        Some(vec![
            "--remote=false".to_string(),
            "rm".to_string(),
            "--force".to_string(),
            container_id.to_string(),
        ])
    }

    fn restart_owner_inspect_args(container_id: &str) -> Vec<String> {
        vec![
            "--remote=false".to_string(),
            "inspect".to_string(),
            "--format".to_string(),
            format!("{{{{ index .Config.Labels \"{PODMAN_PROCESS_OWNER_LABEL}\" }}}}"),
            container_id.to_string(),
        ]
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
        let process_id = Uuid::now_v7();
        let container_name = durable_container_owner(process_id);
        let args = podman_run_args_for_container(&spec, &container_name)?;
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
        if !is_valid_durable_container_id(&container_id) {
            return Err(spawn_failed(
                "podman run did not return a full lowercase container id",
            ));
        }
        Ok(ProcessHandle {
            id: process_id,
            adapter_id: AdapterId::new(WSL2_PODMAN_ADAPTER_ID),
            pid: None,
            sandbox_internal_id: container_id,
            spawned_at_utc: Utc::now(),
        })
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
        let output = run_podman_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
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
        let output = run_podman_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
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

    async fn cleanup_after_restart(
        &self,
        handle: &ProcessHandle,
    ) -> Result<RestartCleanupOutcome, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        if !is_valid_durable_container_id(&handle.sandbox_internal_id) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        let inspect_args = Self::restart_owner_inspect_args(&handle.sandbox_internal_id);
        let inspection = run_podman_command(
            &self.config,
            &inspect_args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if inspection.exit_code != 0 {
            let detail = inspection.stderr_text();
            if restart_cleanup_target_absent(&detail) {
                return Ok(RestartCleanupOutcome::AlreadyAbsent);
            }
            return Err(spawn_failed(format!(
                "podman restart ownership inspection failed for durable container {}: {detail}",
                handle.sandbox_internal_id
            )));
        }
        if !restart_owner_matches(handle.id, &inspection.stdout) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        let rm_args = Self::restart_cleanup_args(&handle.sandbox_internal_id).ok_or(
            SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            },
        )?;
        let cleanup = run_podman_command(
            &self.config,
            &rm_args,
            None,
            Some(self.config.command_timeout_ms()),
        )
        .await?;
        if cleanup.exit_code == 0 {
            return Ok(RestartCleanupOutcome::Terminated);
        }
        let detail = cleanup.stderr_text();
        if restart_cleanup_target_absent(&detail) {
            return Ok(RestartCleanupOutcome::AlreadyAbsent);
        }
        Err(spawn_failed(format!(
            "podman restart cleanup failed for durable container {}: {detail}",
            handle.sandbox_internal_id
        )))
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
            supports_persistent_exec: false,
            supports_warm_agent: false,
            supports_live_token_stream: false,
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

fn restart_cleanup_target_absent(detail: &str) -> bool {
    let detail = detail.to_ascii_lowercase();
    detail.contains("no such container") || detail.contains("no such object")
}

fn durable_container_owner(process_id: Uuid) -> String {
    format!("hsk-{}", process_id.simple())
}

fn restart_owner_matches(process_id: Uuid, stdout: &[u8]) -> bool {
    std::str::from_utf8(stdout).ok().is_some_and(|value| {
        value.trim_end_matches(['\r', '\n']) == durable_container_owner(process_id)
    })
}

fn is_valid_durable_container_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

#[cfg(test)]
mod restart_cleanup_tests {
    use super::*;

    #[test]
    fn restart_cleanup_is_force_remove_by_durable_id_and_absence_is_idempotent() {
        let durable_id = "a".repeat(64);
        assert_eq!(
            Wsl2PodmanAdapter::restart_cleanup_args(&durable_id).expect("valid durable id"),
            vec!["--remote=false", "rm", "--force", durable_id.as_str()]
        );
        assert!(restart_cleanup_target_absent(
            "Error: no such container podman-container-1"
        ));
        assert!(!restart_cleanup_target_absent("connection refused"));
    }

    #[test]
    fn restart_cleanup_rejects_malformed_or_option_like_container_ids() {
        for candidate in [
            "podman-container-1".to_string(),
            "--all".to_string(),
            "A".repeat(64),
            format!("{}g", "a".repeat(63)),
            "a".repeat(63),
            "a".repeat(65),
            format!("{}\n", "a".repeat(64)),
            format!("{}\0", "a".repeat(63)),
        ] {
            assert!(Wsl2PodmanAdapter::restart_cleanup_args(&candidate).is_none());
        }
    }

    #[test]
    fn restart_cleanup_ownership_probe_is_bound_to_process_uuid() {
        let process_id = Uuid::now_v7();
        let durable_id = "b".repeat(64);
        assert_eq!(
            durable_container_owner(process_id),
            format!("hsk-{}", process_id.simple())
        );
        assert_eq!(
            Wsl2PodmanAdapter::restart_owner_inspect_args(&durable_id),
            vec![
                "--remote=false",
                "inspect",
                "--format",
                "{{ index .Config.Labels \"io.handshake.process-id\" }}",
                durable_id.as_str(),
            ]
        );
        let owner = durable_container_owner(process_id);
        assert!(restart_owner_matches(
            process_id,
            format!("{owner}\r\n").as_bytes()
        ));
        let other_owner = durable_container_owner(Uuid::now_v7());
        for untrusted in [
            "",
            "<no value>",
            " hsk-unrelated",
            "hsk-unrelated ",
            other_owner.as_str(),
        ] {
            assert!(!restart_owner_matches(process_id, untrusted.as_bytes()));
        }
    }
}
