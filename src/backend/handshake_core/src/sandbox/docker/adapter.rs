use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::kernel_003_bridge::{
    docker_exec_args, docker_run_args, parse_docker_status,
    run_docker_command,
};
use crate::sandbox::{
    AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    RestartCleanupOutcome, SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass,
    DOCKER_ADAPTER_ID,
};

const GPU_PROBE_CACHE_TTL: Duration = Duration::from_secs(60);
pub(super) const DOCKER_PROCESS_OWNER_LABEL: &str = "io.handshake.process-id";

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
    processes: Arc<Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<DockerProcessState>>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DockerObservation {
    status: ProcessStatus,
    exit_code: Option<i32>,
    started_at: String,
}

#[derive(Debug)]
struct DockerProcessState {
    handle: Option<ProcessHandle>,
    started_at: Option<String>,
    signals: Vec<DockerSignalAttempt>,
    removal: Option<DockerRemovalAttempt>,
    terminal: Option<DockerObservation>,
    removed: bool,
}

impl DockerAdapter {
    pub async fn try_new(config: DockerConfig) -> Result<Self, SandboxAdapterError> {
        verify_docker_available(&config).await?;
        let gpu_passthrough = probe_gpu_passthrough(&config).await;
        Ok(Self {
            config,
            gpu_cache: Arc::new(Mutex::new(DockerGpuProbeCache::new(gpu_passthrough))),
            processes: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn with_config_and_gpu_for_tests(
        config: DockerConfig,
        gpu_passthrough: GpuPassthrough,
    ) -> Self {
        Self {
            config,
            gpu_cache: Arc::new(Mutex::new(DockerGpuProbeCache::new(gpu_passthrough))),
            processes: Arc::new(Mutex::new(HashMap::new())),
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

    fn restart_cleanup_args(container_id: &str) -> Option<Vec<String>> {
        if !is_valid_durable_container_id(container_id) {
            return None;
        }
        Some(vec![
            "rm".to_string(),
            "--force".to_string(),
            container_id.to_string(),
        ])
    }

    fn restart_owner_inspect_args(container_id: &str) -> Vec<String> {
        vec![
            "inspect".to_string(),
            "--format".to_string(),
            format!("{{{{ index .Config.Labels \"{DOCKER_PROCESS_OWNER_LABEL}\" }}}}"),
            container_id.to_string(),
        ]
    }

    fn ensure_handle(&self, handle: &ProcessHandle) -> Result<(), SandboxAdapterError> {
        if handle.adapter_id != AdapterId::new(DOCKER_ADAPTER_ID) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        Ok(())
    }

    fn process_state(
        &self,
        handle: &ProcessHandle,
    ) -> Result<Arc<tokio::sync::Mutex<DockerProcessState>>, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        if !is_valid_durable_container_id(&handle.sandbox_internal_id) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        let mut processes = self.processes.lock().map_err(|error| {
            spawn_failed(format!("docker process registry poisoned: {error}"))
        })?;
        Ok(processes
            .entry(handle.id)
            .or_insert_with(|| {
                Arc::new(tokio::sync::Mutex::new(DockerProcessState {
                    handle: None,
                    started_at: None,
                    signals: Vec::new(),
                    removal: None,
                    terminal: None,
                    removed: false,
                }))
            })
            .clone())
    }

    async fn inspect_owned(
        &self,
        handle: &ProcessHandle,
    ) -> Result<Option<DockerObservation>, SandboxAdapterError> {
        let args = vec![
            "inspect".to_string(),
            "--format".to_string(),
            "{{json .}}".to_string(),
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
            let detail = output.stderr_text();
            if restart_cleanup_target_absent(&detail) {
                return Ok(None);
            }
            return Err(spawn_failed(format!("docker ownership/state inspection failed: {detail}")));
        }
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| spawn_failed(format!("invalid docker inspection JSON: {error}")))?;
        let owner = value
            .get("Config")
            .and_then(|config| config.get("Labels"))
            .and_then(|labels| labels.get(DOCKER_PROCESS_OWNER_LABEL))
            .and_then(serde_json::Value::as_str);
        if value.get("Id").and_then(serde_json::Value::as_str)
            != Some(handle.sandbox_internal_id.as_str())
            || owner != Some(durable_container_owner(handle.id).as_str())
        {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        let state = value.get("State").ok_or_else(|| spawn_failed("docker state is missing"))?;
        let kind = state
            .get("Status")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| spawn_failed("docker state kind is missing"))?;
        let started_at = state
            .get("StartedAt")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| spawn_failed("docker start generation is missing"))?
            .to_string();
        let parsed_start = chrono::DateTime::parse_from_rfc3339(&started_at)
            .map_err(|_| spawn_failed("docker start generation is invalid"))?;
        if chrono::Datelike::year(&parsed_start) <= 1 {
            return Err(spawn_failed("docker start generation is unobserved"));
        }
        let normalized_kind = kind.trim().to_ascii_lowercase();
        let exit_code = if normalized_kind == "exited" {
            let code = state
                .get("ExitCode")
                .and_then(serde_json::Value::as_i64)
                .and_then(|code| i32::try_from(code).ok())
                .ok_or_else(|| spawn_failed("docker terminal exit code is unobserved"))?;
            Some(code)
        } else {
            None
        };
        Ok(Some(DockerObservation {
            status: parse_docker_status(&normalized_kind, exit_code),
            exit_code,
            started_at,
        }))
    }

    async fn observe_process(
        &self,
        handle: &ProcessHandle,
        state: &mut DockerProcessState,
    ) -> Result<DockerObservation, SandboxAdapterError> {
        if state.handle.as_ref().is_some_and(|original| original != handle) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        if state.removed {
            return state.terminal.clone().ok_or_else(|| {
                spawn_failed("docker removal has no original terminal observation")
            });
        }
        let Some(observed) = self.inspect_owned(handle).await? else {
            return match &state.terminal {
                Some(terminal) => {
                    state.removed = true;
                    Ok(terminal.clone())
                }
                None => Ok(DockerObservation {
                    status: ProcessStatus::Orphaned,
                    exit_code: None,
                    started_at: String::new(),
                }),
            };
        };
        if let Some(original) = &state.started_at {
            if original != &observed.started_at {
                return Err(SandboxAdapterError::ProcessHandleStale {
                    process_id: handle.id,
                });
            }
        } else if matches!(observed.status, ProcessStatus::Running | ProcessStatus::Exited { .. }) {
            // Commit original identity only from this owner-verified frame.
            // Earlier failed or absent observations retain serialization alone.
            state.handle = Some(handle.clone());
            state.started_at = Some(observed.started_at.clone());
        }
        if let Some(terminal) = &state.terminal {
            if terminal != &observed {
                return Err(SandboxAdapterError::ProcessHandleStale {
                    process_id: handle.id,
                });
            }
        } else if matches!(observed.status, ProcessStatus::Exited { .. } | ProcessStatus::Killed { .. }) {
            state.terminal = Some(observed.clone());
        }
        Ok(observed)
    }

    async fn cleanup_observed_terminal(
        &self,
        handle: &ProcessHandle,
        state: &mut DockerProcessState,
    ) -> Result<(), SandboxAdapterError> {
        if state.handle.as_ref().is_some_and(|original| original != handle) {
            return Err(SandboxAdapterError::ProcessHandleStale { process_id: handle.id });
        }
        for attempt in &mut state.signals {
            if attempt.original.is_some() {
                attempt.observe().await?;
            }
        }
        if let Some(removal) = &mut state.removal {
            if removal.original.is_some() {
                removal.observe().await?;
            }
            if matches!(removal.outcome, Some(DockerRemovalOutcome::Accepted)) {
                state.removed = true;
            }
        }
        let observed = self.observe_process(handle, state).await?;
        if !matches!(observed.status, ProcessStatus::Exited { .. } | ProcessStatus::Killed { .. }) {
            return Err(spawn_failed(
                "docker cleanup requires an owned observed terminal container",
            ));
        }
        if state.removed {
            return Ok(());
        }
        if let Some(removal) = &mut state.removal {
            // Only a positively settled rejection permits a new plain rm after
            // the existing fresh exact-generation terminal observation above.
            if !matches!(removal.outcome, Some(DockerRemovalOutcome::Rejected(_))) {
                return removal.observe().await;
            }
        }
        let args = vec!["rm".to_string(), handle.sandbox_internal_id.clone()];
        let config = self.config.clone();
        let registry = crate::storage::surreal::configured_capability_registry();
        state.removal = Some(DockerRemovalAttempt {
            original: Some(Box::pin(async move {
                crate::storage::surreal::with_captured_capability_registry(registry, async move {
                    let output = run_docker_command(
                        &config, &args, None, Some(config.command_timeout_ms()),
                    ).await?;
                    if output.exit_code != 0 && !restart_cleanup_target_absent(&output.stderr_text()) {
                        return Ok(Some(output.stderr_text()));
                    }
                    Ok(None)
                }).await
            })),
            outcome: None,
            _original_context: crate::storage::surreal::capture_model_lane_operation(),
        });
        state.removal.as_mut().expect("registered original Docker removal").observe().await?;
        state.removed = true;
        Ok(())
    }

    async fn kill_observed(
        &self,
        handle: &ProcessHandle,
        signal: Signal,
    ) -> Result<Option<ProcessStatus>, SandboxAdapterError> {
        let process = self.process_state(handle)?;
        let mut state = process.lock().await;
        if state.handle.as_ref().is_some_and(|original| original != handle) {
            return Err(SandboxAdapterError::ProcessHandleStale { process_id: handle.id });
        }
        // Resume original commands before the existing pre-signal observation;
        // a different explicit signal still requires observed Running afterward.
        for attempt in &mut state.signals {
            if attempt.original.is_some() {
                attempt.observe().await?;
            }
        }
        let observed = self.observe_process(handle, &mut state).await?;
        if matches!(observed.status, ProcessStatus::Exited { .. } | ProcessStatus::Killed { .. }) {
            self.cleanup_observed_terminal(handle, &mut state).await?;
            return Ok(Some(observed.status));
        }
        if !matches!(observed.status, ProcessStatus::Running) {
            return Err(spawn_failed(
                "docker signal requires an owned observed running container",
            ));
        }
        if let Some(attempt) = state.signals.iter_mut().find(|attempt| attempt.signal == signal) {
            attempt.observe().await?;
        } else {
            let args = Self::kill_args(&handle.sandbox_internal_id, signal);
            let config = self.config.clone();
            let registry = crate::storage::surreal::configured_capability_registry();
            let original = Box::pin(async move {
                crate::storage::surreal::with_captured_capability_registry(registry, async move {
                    let output = run_docker_command(
                        &config, &args, None, Some(config.command_timeout_ms()),
                    ).await?;
                    if output.exit_code != 0 {
                        return Err(spawn_failed(format!("docker kill/stop failed: {}", output.stderr_text())));
                    }
                    Ok(())
                }).await
            });
            state.signals.push(DockerSignalAttempt {
                signal, original: Some(original), outcome: DockerSignalOutcome::Attempted,
                _original_context: crate::storage::surreal::capture_model_lane_operation(),
            });
            state.signals.last_mut().expect("registered original Docker signal").observe().await?;
        }
        let observed = self.observe_process(handle, &mut state).await?;
        if matches!(observed.status, ProcessStatus::Exited { .. } | ProcessStatus::Killed { .. }) {
            self.cleanup_observed_terminal(handle, &mut state).await?;
            return Ok(Some(observed.status));
        } else if !matches!(observed.status, ProcessStatus::Running) {
            return Err(spawn_failed(
                "docker signal was acknowledged without an observed terminal state",
            ));
        }
        Ok(None)
    }

    async fn ensure_runtime_available(&self) -> Result<(), SandboxAdapterError> {
        verify_docker_available(&self.config).await
    }
}

#[async_trait]
impl SandboxAdapter for DockerAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        self.ensure_runtime_available().await?;
        let process_id = Uuid::now_v7();
        let container_name = durable_container_owner(process_id);
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
        if !is_valid_durable_container_id(&container_id) {
            return Err(spawn_failed(
                "docker run did not return a full lowercase container id",
            ));
        }
        Ok(ProcessHandle {
            id: process_id,
            adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
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
        let output = run_docker_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
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
        let output = run_docker_command(
            &self.config,
            &args,
            None,
            Some(self.config.command_timeout_ms()),
        )
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
            NetPolicy::LoopbackOnly | NetPolicy::Allowlist(_) | NetPolicy::HostInherited => Err(net_policy_failed(
                "post-spawn Docker net_policy changes are not supported yet; declare the network policy before spawn in ProcessSpec.net_policy",
            )),
        }
    }

    async fn kill(
        &self, handle: &ProcessHandle, signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        self.kill_observed(handle, signal).await.map(|_| ())
    }

    async fn reclaim_detached_observed(
        &self, identity: &crate::sandbox::DetachedProcessIdentity, signal: Signal,
    ) -> Result<crate::sandbox::DetachedReclaimOutcome, crate::sandbox::DetachedReclaimError> {
        match self.kill_observed(&identity.handle, signal).await {
            Ok(Some(status)) => Ok(crate::sandbox::DetachedReclaimOutcome::Observed(status)),
            Ok(None) => Err(crate::sandbox::DetachedReclaimError::AfterInvocation(spawn_failed(
                "docker signal was acknowledged while the container still runs",
            ))),
            Err(error) => Err(crate::sandbox::DetachedReclaimError::AfterInvocation(error)),
        }
    }

    async fn cleanup_terminal(
        &self,
        handle: &ProcessHandle,
    ) -> Result<(), SandboxAdapterError> {
        let process = self.process_state(handle)?;
        let mut state = process.lock().await;
        self.cleanup_observed_terminal(handle, &mut state).await
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
        let inspection = run_docker_command(
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
                "docker restart ownership inspection failed for durable container {}: {detail}",
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
        let cleanup = run_docker_command(
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
            "docker restart cleanup failed for durable container {}: {detail}",
            handle.sandbox_internal_id
        )))
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        let process = self.process_state(handle)?;
        let mut state = process.lock().await;
        Ok(self.observe_process(handle, &mut state).await?.status)
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        let process = self.process_state(handle)?;
        let mut state = process.lock().await;
        Ok(self.observe_process(handle, &mut state).await?.exit_code)
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
            supports_persistent_exec: false,
            supports_warm_agent: false,
            supports_live_token_stream: false,
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

#[cfg(test)]
mod restart_cleanup_tests {
    use super::*;

    #[test]
    fn restart_cleanup_is_force_remove_by_durable_id_and_absence_is_idempotent() {
        let durable_id = "a".repeat(64);
        assert_eq!(
            DockerAdapter::restart_cleanup_args(&durable_id).expect("valid durable id"),
            vec!["rm", "--force", durable_id.as_str()]
        );
        assert!(restart_cleanup_target_absent(
            "Error response from daemon: No such container: hsk-container-1"
        ));
        assert!(!restart_cleanup_target_absent(
            "Cannot connect to the Docker daemon"
        ));
    }

    #[test]
    fn restart_cleanup_rejects_malformed_or_option_like_container_ids() {
        for candidate in [
            "hsk-container-1".to_string(),
            "--all".to_string(),
            "A".repeat(64),
            format!("{}g", "a".repeat(63)),
            "a".repeat(63),
            "a".repeat(65),
            format!("{}\n", "a".repeat(64)),
            format!("{}\0", "a".repeat(63)),
        ] {
            assert!(DockerAdapter::restart_cleanup_args(&candidate).is_none());
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
            DockerAdapter::restart_owner_inspect_args(&durable_id),
            vec![
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


enum DockerSignalOutcome {
    Attempted,
    Acknowledged,
    Uncertain(String),
}

struct DockerSignalAttempt {
    signal: Signal,
    original: Option<std::pin::Pin<Box<
        dyn std::future::Future<Output = Result<(), SandboxAdapterError>> + Send,
    >>>,
    outcome: DockerSignalOutcome,
    _original_context: crate::storage::surreal::CapturedModelLaneOperation,
}

impl std::fmt::Debug for DockerSignalAttempt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let outcome = match &self.outcome {
            DockerSignalOutcome::Attempted => "attempted",
            DockerSignalOutcome::Acknowledged => "acknowledged",
            DockerSignalOutcome::Uncertain(_) => "uncertain",
        };
        formatter.debug_struct("DockerSignalAttempt")
            .field("signal", &self.signal).field("outcome", &outcome)
            .finish_non_exhaustive()
    }
}

impl DockerSignalAttempt {
    async fn observe(&mut self) -> Result<(), SandboxAdapterError> {
        if let Some(original) = self.original.as_mut() {
            let observed = original.as_mut().await;
            self.outcome = match &observed {
                Ok(()) => DockerSignalOutcome::Acknowledged,
                Err(error) => DockerSignalOutcome::Uncertain(error.to_string()),
            };
            self.original.take();
            return observed;
        }
        match &self.outcome {
            DockerSignalOutcome::Acknowledged => Ok(()),
            DockerSignalOutcome::Uncertain(reason) => Err(SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
                reason: format!("original Docker signal outcome remains uncertain: {reason}"),
            }),
            DockerSignalOutcome::Attempted => Err(SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
                reason: "original Docker signal has no retained result".into(),
            }),
        }
    }
}



enum DockerRemovalOutcome {
    Accepted,
    Rejected(String),
    Uncertain(String),
}

struct DockerRemovalAttempt {
    original: Option<std::pin::Pin<Box<
        dyn std::future::Future<Output = Result<Option<String>, SandboxAdapterError>> + Send,
    >>>,
    outcome: Option<DockerRemovalOutcome>,
    _original_context: crate::storage::surreal::CapturedModelLaneOperation,
}

impl std::fmt::Debug for DockerRemovalAttempt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let outcome = match &self.outcome {
            None => "attempted",
            Some(DockerRemovalOutcome::Accepted) => "accepted",
            Some(DockerRemovalOutcome::Rejected(_)) => "rejected",
            Some(DockerRemovalOutcome::Uncertain(_)) => "uncertain",
        };
        formatter.debug_struct("DockerRemovalAttempt").field("outcome", &outcome)
            .finish_non_exhaustive()
    }
}

impl DockerRemovalAttempt {
    async fn observe(&mut self) -> Result<(), SandboxAdapterError> {
        if let Some(original) = self.original.as_mut() {
            let observed = original.as_mut().await;
            self.outcome = Some(match &observed {
                Ok(None) => DockerRemovalOutcome::Accepted,
                Ok(Some(reason)) => DockerRemovalOutcome::Rejected(reason.clone()),
                Err(error) => DockerRemovalOutcome::Uncertain(error.to_string()),
            });
            self.original.take();
            observed?;
        }
        match &self.outcome {
            Some(DockerRemovalOutcome::Accepted) => Ok(()),
            Some(DockerRemovalOutcome::Rejected(reason)) => Err(spawn_failed(
                format!("docker terminal removal rejected: {reason}"),
            )),
            Some(DockerRemovalOutcome::Uncertain(reason)) => Err(SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new(DOCKER_ADAPTER_ID),
                reason: format!("original Docker removal remains uncertain: {reason}"),
            }),
            None => Err(spawn_failed("original Docker removal has no retained outcome")),
        }
    }
}

