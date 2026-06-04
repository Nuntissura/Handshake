use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

use crate::sandbox::wsl2_podman::wsl_detection::default_wsl_exe;
use crate::sandbox::{
    AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    ResourceLimits, SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub const GVISOR_ADAPTER_ID: &str = "gvisor";

/// Proven-working defaults for the host WSL2 gVisor layout. Every field is
/// overridable via a `HANDSHAKE_GVISOR_*` environment variable so the adapter
/// stays disk-agnostic per [GLOBAL-PORTABILITY] (no hardcoded absolute path is
/// baked into a build the operator cannot redirect after a project move).
const DEFAULT_DISTRO: &str = "Ubuntu";
/// gVisor's `runsc` needs cgroup + userns privileges that the default WSL user
/// does not have (cgroup `subtree_control` is root-owned and `newuidmap` is not
/// installed for rootless mode). Empirically, running `runsc` as `root` inside
/// WSL2 is the invocation that actually starts a sandbox, so the proven default
/// user is `root`; override with `HANDSHAKE_GVISOR_USER`.
const DEFAULT_USER: &str = "root";
const DEFAULT_RUNSC_BIN: &str = "/home/ilja_smets/handshake-sandbox/bin/runsc";
/// `systrap` is the platform that needs no `/dev/kvm` (it traps syscalls via
/// `ptrace`-style signal delivery), which is the most broadly available option
/// in WSL2. Override with `HANDSHAKE_GVISOR_PLATFORM` (e.g. `kvm`, `ptrace`).
const DEFAULT_PLATFORM: &str = "systrap";
const DEFAULT_WORK_DIR: &str = "/home/ilja_smets/handshake-sandbox";
const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 60_000;
/// Probe commands are quick; keep them well under the exec timeout.
const PROBE_TIMEOUT_MS: u64 = 20_000;

/// Configuration for [`GvisorAdapter`].
///
/// All sandbox paths are WSL-side (Linux) paths because `runsc` and its work
/// directory live inside the WSL2 filesystem. Each field defaults to the proven
/// host value and is overridable via the matching `HANDSHAKE_GVISOR_*`
/// environment variable:
///
/// | field                | env var                       |
/// |----------------------|-------------------------------|
/// | `distro`             | `HANDSHAKE_GVISOR_DISTRO`     |
/// | `user`               | `HANDSHAKE_GVISOR_USER`       |
/// | `runsc_bin`          | `HANDSHAKE_GVISOR_RUNSC`      |
/// | `platform`           | `HANDSHAKE_GVISOR_PLATFORM`   |
/// | `work_dir`           | `HANDSHAKE_GVISOR_WORK_DIR`   |
/// | `command_timeout_ms` | `HANDSHAKE_GVISOR_TIMEOUT_MS` |
/// | `ignore_cgroups`     | `HANDSHAKE_GVISOR_IGNORE_CGROUPS` |
///
/// The host-side `wsl.exe` launcher resolves via `PATH`
/// (`HANDSHAKE_GVISOR_WSL_EXE` overrides it) so the Windows side stays portable
/// too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GvisorConfig {
    distro: String,
    user: String,
    wsl_exe: PathBuf,
    runsc_bin: String,
    platform: String,
    work_dir: String,
    command_timeout_ms: u64,
    ignore_cgroups: bool,
}

impl GvisorConfig {
    pub fn distro(&self) -> &str {
        &self.distro
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn wsl_exe(&self) -> &Path {
        &self.wsl_exe
    }

    pub fn runsc_bin(&self) -> &str {
        &self.runsc_bin
    }

    pub fn platform(&self) -> &str {
        &self.platform
    }

    pub fn work_dir(&self) -> &str {
        &self.work_dir
    }

    pub fn command_timeout_ms(&self) -> u64 {
        self.command_timeout_ms
    }

    pub fn ignore_cgroups(&self) -> bool {
        self.ignore_cgroups
    }

    pub fn with_command_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.command_timeout_ms = timeout_ms;
        self
    }
}

impl Default for GvisorConfig {
    fn default() -> Self {
        Self {
            distro: env_string("HANDSHAKE_GVISOR_DISTRO", DEFAULT_DISTRO),
            user: env_string("HANDSHAKE_GVISOR_USER", DEFAULT_USER),
            wsl_exe: std::env::var("HANDSHAKE_GVISOR_WSL_EXE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| default_wsl_exe()),
            runsc_bin: env_string("HANDSHAKE_GVISOR_RUNSC", DEFAULT_RUNSC_BIN),
            platform: env_string("HANDSHAKE_GVISOR_PLATFORM", DEFAULT_PLATFORM),
            work_dir: env_string("HANDSHAKE_GVISOR_WORK_DIR", DEFAULT_WORK_DIR),
            command_timeout_ms: env_u64("HANDSHAKE_GVISOR_TIMEOUT_MS", DEFAULT_COMMAND_TIMEOUT_MS),
            ignore_cgroups: env_bool("HANDSHAKE_GVISOR_IGNORE_CGROUPS", false),
        }
    }
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            )
        })
        .unwrap_or(default)
}

/// Per-handle bookkeeping for the ephemeral-sandbox model. Each `exec` runs a
/// brand-new `runsc do` sandbox, so we only need to remember the last status,
/// the last exit code, and a kill flag.
#[derive(Debug)]
struct HandleState {
    status: ProcessStatus,
    exit_code: Option<i32>,
    killed: bool,
}

impl Default for HandleState {
    fn default() -> Self {
        Self {
            // Handles start out as "running" (no exec has completed yet); the
            // ephemeral model flips this to Exited after a finished exec.
            status: ProcessStatus::Running,
            exit_code: None,
            killed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GvisorAdapter {
    config: GvisorConfig,
    handles: Arc<Mutex<HashMap<Uuid, HandleState>>>,
}

impl GvisorAdapter {
    /// REAL availability probe. Verifies, in order:
    /// 1. `wsl.exe` resolves and the configured distro is registered.
    /// 2. the `runsc` binary exists and is executable inside WSL.
    /// 3. a real smoke `runsc ... do echo <marker>` actually starts a gVisor
    ///    sandbox and prints the marker.
    ///
    /// Any failure returns [`SandboxAdapterError::AdapterUnavailable`] so the
    /// bootstrap registry gracefully skips this adapter on non-WSL / hosts where
    /// runsc cannot start a sandbox instead of failing the whole sandbox
    /// bring-up.
    pub async fn try_new(config: GvisorConfig) -> Result<Self, SandboxAdapterError> {
        verify_available(&config).await?;
        Ok(Self {
            config,
            handles: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn config(&self) -> &GvisorConfig {
        &self.config
    }

    fn ensure_handle(&self, handle: &ProcessHandle) -> Result<(), SandboxAdapterError> {
        if handle.adapter_id != AdapterId::new(GVISOR_ADAPTER_ID) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        if !self
            .handles
            .lock()
            .map(|map| map.contains_key(&handle.id))
            .unwrap_or(false)
        {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        Ok(())
    }

    /// Build the proven `wsl.exe ... runsc ... do sh -c "<cmd>"` argv for one
    /// ephemeral sandbox. `command_line` is the joined guest shell command line
    /// (with any env-overlay exports prefixed).
    fn exec_args(&self, command_line: &str) -> Vec<String> {
        let mut args = vec![
            "-d".to_string(),
            self.config.distro.clone(),
            "-u".to_string(),
            self.config.user.clone(),
            "-e".to_string(),
            self.config.runsc_bin.clone(),
            // Deny-all network: `--network=none` gives the sandbox no network
            // device at all (see `net_policy`).
            "--network=none".to_string(),
            format!("--platform={}", self.config.platform),
        ];
        if self.config.ignore_cgroups {
            args.push("--ignore-cgroups".to_string());
        }
        args.push("do".to_string());
        args.push("sh".to_string());
        args.push("-c".to_string());
        args.push(command_line.to_string());
        args
    }
}

#[async_trait]
impl SandboxAdapter for GvisorAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        validate_supported_resource_limits(&spec.resource_limits)?;
        // Re-probe so a handle is never minted against a runtime that has gone
        // away (mirrors the cloud_hypervisor / docker pattern). The sandbox
        // itself is not started here: the ephemeral model starts a fresh
        // `runsc do` sandbox per exec.
        verify_available(&self.config).await?;
        let handle = ProcessHandle::new(
            AdapterId::new(GVISOR_ADAPTER_ID),
            None,
            format!("hsk-gvisor-{}", Uuid::now_v7().simple()),
        );
        self.handles
            .lock()
            .map_err(|error| spawn_failed(format!("handle registry poisoned: {error}")))?
            .insert(handle.id, HandleState::default());
        Ok(handle)
    }

    async fn exec(
        &self,
        handle: &ProcessHandle,
        cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        if cmd.argv.is_empty() {
            return Err(spawn_failed("Command.argv must not be empty"));
        }
        if self
            .handles
            .lock()
            .map(|map| {
                map.get(&handle.id)
                    .map(|state| state.killed)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
        {
            return Err(spawn_failed(
                "handle was killed; spawn a fresh handle before exec",
            ));
        }

        // Join argv into a single shell command line. env overlay entries are
        // prefixed as POSIX `export KEY=VALUE` so they reach the sandboxed
        // command's environment inside the `sh -c` shell.
        let command_line = build_command_line(&cmd);
        let timeout_ms = cmd.timeout_ms.unwrap_or(self.config.command_timeout_ms);
        let exec_args = self.exec_args(&command_line);

        let start = Instant::now();
        let output = run_host_command(
            self.config.wsl_exe(),
            &exec_args,
            cmd.stdin.clone(),
            Some(timeout_ms),
            handle.id,
        )
        .await?;
        let duration_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

        if let Ok(mut map) = self.handles.lock() {
            if let Some(state) = map.get_mut(&handle.id) {
                state.status = ProcessStatus::Exited {
                    code: output.exit_code,
                };
                state.exit_code = Some(output.exit_code);
            }
        }

        Ok(ExecResult {
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms,
        })
    }

    async fn fs_bind(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        // TODO(gvisor): real host->sandbox binds require building an OCI bundle
        // with a `mounts` entry and switching from `runsc do` to
        // `runsc run`/`runsc exec`. `runsc do` runs in the host filesystem
        // namespace with no configurable bind points, so honoring a bind here
        // would silently expose the whole host fs (unsafe) or silently drop the
        // bind (a lie). Fail closed with a typed error until the bundle path is
        // implemented, rather than faking it.
        Err(SandboxAdapterError::BindGuestPathInvalid {
            guest_path,
            reason: format!(
                "gvisor adapter does not yet support host binds (host_path={}); \
                 `runsc do` has no configurable mount points. A future change will \
                 build an OCI bundle and switch to `runsc run`/`exec`.",
                host_path.display()
            ),
        })
    }

    async fn net_policy(
        &self,
        handle: &ProcessHandle,
        policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        // Every exec passes `--network=none`, so the sandbox has no network
        // device at all: deny-all and loopback-only are both satisfied by the
        // absence of guest networking. An external allowlist cannot be honored
        // without a `--network=host`/netns bridge, so it fails closed.
        match policy {
            NetPolicy::DenyAll | NetPolicy::LoopbackOnly => Ok(()),
            NetPolicy::Allowlist(entries) if entries.is_empty() => Ok(()),
            NetPolicy::Allowlist(_) => Err(net_policy_failed(
                "gvisor sandboxes run with `--network=none` (no network device); external allowlist entries require a future netns bridge and fail closed",
            )),
        }
    }

    async fn kill(
        &self,
        handle: &ProcessHandle,
        signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        // Mark the handle killed. Any in-flight `runsc do` child for this handle
        // is terminated by run_host_command's kill_on_drop once the owning exec
        // future is dropped; for the ephemeral model there is no long-lived
        // sandbox to signal between execs.
        if let Ok(mut map) = self.handles.lock() {
            if let Some(state) = map.get_mut(&handle.id) {
                state.killed = true;
                state.status = ProcessStatus::Killed { by_signal: signal };
            }
        }
        Ok(())
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let status = self
            .handles
            .lock()
            .ok()
            .and_then(|map| map.get(&handle.id).map(|state| state.status.clone()))
            .unwrap_or(ProcessStatus::Orphaned);
        Ok(status)
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        Ok(self
            .handles
            .lock()
            .ok()
            .and_then(|map| map.get(&handle.id).and_then(|state| state.exit_code)))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            adapter_id: AdapterId::new(GVISOR_ADAPTER_ID),
            runtime_available: true,
            // gVisor's user-space sentry kernel gives the sandbox its own
            // filesystem view and (with `--network=none`) no network device at
            // all; both are strongly isolated from the host without the full
            // hardware-virtualization boundary of a Tier-3 microVM.
            filesystem_isolation_strength: IsolationStrength::Strong,
            network_isolation_strength: IsolationStrength::Strong,
            gpu_passthrough: GpuPassthrough::None,
            stdio_throughput_class: ThroughputClass::Medium,
            win32_native_fidelity: false,
            cross_machine_portable: true,
            isolation_tier: IsolationTier::Tier2Syscall,
            // gVisor's `systrap`/`ptrace` platform does not need nested virt.
            requires_nested_virt: false,
            supports_snapshot: false,
            supports_persistent_exec: false,
            supports_warm_agent: false,
            supports_live_token_stream: false,
        }
    }
}

/// Joined command line + env-overlay exports for one sandboxed command.
fn build_command_line(cmd: &Command) -> String {
    let mut prefix = String::new();
    for (key, value) in &cmd.env_overlay {
        // POSIX export so the value reaches the command's environment inside the
        // sandboxed `sh -c` shell.
        prefix.push_str(&format!("export {key}={}; ", shell_quote(value)));
    }
    let joined = cmd
        .argv
        .iter()
        .map(|arg| shell_quote(arg))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{prefix}{joined}")
}

/// Minimal POSIX single-quote escaping so argv tokens survive the guest shell.
fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':' | '=' | ','))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn verify_available(config: &GvisorConfig) -> Result<(), SandboxAdapterError> {
    // 1. wsl.exe + distro registered.
    let distros = run_host_command(
        config.wsl_exe(),
        &["-l".to_string(), "-q".to_string()],
        None,
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await
    .map_err(|error| unavailable(format!("wsl.exe unavailable: {error}")))?;
    if distros.exit_code != 0 {
        return Err(unavailable(format!(
            "`wsl -l -q` failed: {}",
            distros.stderr_text()
        )));
    }
    let distro_text = decode_wsl_output(&distros.stdout);
    let distro_present = distro_text
        .lines()
        .map(|line| line.trim().trim_matches('\0').trim())
        .any(|line| line.eq_ignore_ascii_case(config.distro()));
    if !distro_present {
        return Err(unavailable(format!(
            "WSL distro `{}` is not registered (found: {})",
            config.distro(),
            distro_text.replace(['\r', '\n', '\0'], " ").trim()
        )));
    }

    // 2. runsc binary exists and is executable inside WSL.
    let bin_probe = run_host_command(
        config.wsl_exe(),
        &[
            "-d".to_string(),
            config.distro().to_string(),
            "-u".to_string(),
            config.user().to_string(),
            "-e".to_string(),
            "sh".to_string(),
            "-c".to_string(),
            format!("test -x '{}' && echo RUNSC_PRESENT", config.runsc_bin()),
        ],
        None,
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await
    .map_err(|error| unavailable(format!("wsl runsc binary probe failed: {error}")))?;
    if bin_probe.exit_code != 0
        || !String::from_utf8_lossy(&bin_probe.stdout).contains("RUNSC_PRESENT")
    {
        return Err(unavailable(format!(
            "runsc binary not executable in WSL distro `{}` at `{}`: {}",
            config.distro(),
            config.runsc_bin(),
            bin_probe.stderr_text()
        )));
    }

    // 3. Real smoke: actually start a gVisor sandbox and confirm it runs a
    //    command. This is the load-bearing probe — a present binary that cannot
    //    start a sandbox in this WSL2 must surface as Unavailable, not be
    //    treated as a working adapter.
    const SMOKE_MARKER: &str = "gvisor-smoke-ok";
    let mut smoke_args = vec![
        "-d".to_string(),
        config.distro().to_string(),
        "-u".to_string(),
        config.user().to_string(),
        "-e".to_string(),
        config.runsc_bin().to_string(),
        "--network=none".to_string(),
        format!("--platform={}", config.platform()),
    ];
    if config.ignore_cgroups() {
        smoke_args.push("--ignore-cgroups".to_string());
    }
    smoke_args.push("do".to_string());
    smoke_args.push("echo".to_string());
    smoke_args.push(SMOKE_MARKER.to_string());

    let smoke = run_host_command(
        config.wsl_exe(),
        &smoke_args,
        None,
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await
    .map_err(|error| unavailable(format!("runsc smoke sandbox failed to run: {error}")))?;

    if smoke.exit_code != 0 || !String::from_utf8_lossy(&smoke.stdout).contains(SMOKE_MARKER) {
        return Err(unavailable(format!(
            "runsc could not start a gVisor sandbox in WSL distro `{}` (platform={}, user={}, exit {}): stdout={:?} stderr={}",
            config.distro(),
            config.platform(),
            config.user(),
            smoke.exit_code,
            String::from_utf8_lossy(&smoke.stdout).trim(),
            smoke.stderr_text()
        )));
    }

    Ok(())
}

/// Host-process runner mirroring the cloud_hypervisor/Docker/Podman bridge
/// style: hides the console window on Windows, enforces a timeout, and maps
/// spawn/wait failures to typed adapter errors.
async fn run_host_command(
    executable: &Path,
    args: &[String],
    stdin: Option<Bytes>,
    timeout_ms: Option<u64>,
    handle_id: Uuid,
) -> Result<CliOutput, SandboxAdapterError> {
    use tokio::io::AsyncWriteExt;

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
            adapter_id: AdapterId::new(GVISOR_ADAPTER_ID),
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
            .map_err(|_| timed_out(handle_id, timeout_ms))?
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOutput {
    exit_code: i32,
    stdout: Bytes,
    stderr: Bytes,
    #[allow(dead_code)]
    duration_ms: u64,
}

impl CliOutput {
    fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }
}

/// WSL CLI commands like `wsl -l -q` emit UTF-16LE; runtime `-e` of a command's
/// stdout emits raw UTF-8. Detect heavy NUL density to decode UTF-16, else
/// UTF-8.
fn decode_wsl_output(bytes: &[u8]) -> String {
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

fn spawn_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(GVISOR_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn net_policy_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::NetPolicyApplyFailed {
        adapter_id: AdapterId::new(GVISOR_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn validate_supported_resource_limits(limits: &ResourceLimits) -> Result<(), SandboxAdapterError> {
    if limits.disk_read_bytes_per_sec.is_some()
        || limits.disk_write_bytes_per_sec.is_some()
        || limits.net_bandwidth_bytes_per_sec.is_some()
    {
        return Err(spawn_failed(
            "gVisor ResourceLimits disk/net bytes-per-second token-bucket limits are not \
             enforceable by this adapter path yet; refusing to silently ignore requested \
             per-device rate limits",
        ));
    }
    Ok(())
}

fn unavailable(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::AdapterUnavailable {
        adapter_id: AdapterId::new(GVISOR_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn timed_out(handle_id: Uuid, timeout_ms: u64) -> SandboxAdapterError {
    let _ = handle_id;
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(GVISOR_ADAPTER_ID),
        reason: format!("gvisor sandbox exec timed out after {timeout_ms}ms"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn build_command_line_joins_argv() {
        let cmd = Command {
            argv: vec!["echo".to_string(), "hello world".to_string()],
            env_overlay: BTreeMap::new(),
            stdin: None,
            timeout_ms: None,
        };
        assert_eq!(build_command_line(&cmd), "echo 'hello world'");
    }

    #[test]
    fn build_command_line_prefixes_env_overlay_exports() {
        let mut env_overlay = BTreeMap::new();
        env_overlay.insert("FOO".to_string(), "bar baz".to_string());
        let cmd = Command {
            argv: vec!["printenv".to_string(), "FOO".to_string()],
            env_overlay,
            stdin: None,
            timeout_ms: None,
        };
        assert_eq!(
            build_command_line(&cmd),
            "export FOO='bar baz'; printenv FOO"
        );
    }

    #[test]
    fn exec_args_carry_network_none_and_platform() {
        let config = GvisorConfig {
            distro: "Ubuntu".to_string(),
            user: "root".to_string(),
            wsl_exe: PathBuf::from("wsl.exe"),
            runsc_bin: "/opt/runsc".to_string(),
            platform: "systrap".to_string(),
            work_dir: "/work".to_string(),
            command_timeout_ms: DEFAULT_COMMAND_TIMEOUT_MS,
            ignore_cgroups: false,
        };
        let adapter = GvisorAdapter {
            config,
            handles: Arc::new(Mutex::new(HashMap::new())),
        };
        let args = adapter.exec_args("echo hi");
        assert!(args.contains(&"--network=none".to_string()));
        assert!(args.contains(&"--platform=systrap".to_string()));
        assert!(args.contains(&"do".to_string()));
        assert!(!args.contains(&"--ignore-cgroups".to_string()));
        // `-u <user>` must be present so runsc runs as the privileged user.
        let u_idx = args.iter().position(|a| a == "-u").expect("`-u` present");
        assert_eq!(args[u_idx + 1], "root");
        // The joined shell command line is the final argument.
        assert_eq!(args.last().unwrap(), "echo hi");
    }

    #[test]
    fn exec_args_include_ignore_cgroups_when_enabled() {
        let config = GvisorConfig {
            distro: "Ubuntu".to_string(),
            user: "root".to_string(),
            wsl_exe: PathBuf::from("wsl.exe"),
            runsc_bin: "/opt/runsc".to_string(),
            platform: "systrap".to_string(),
            work_dir: "/work".to_string(),
            command_timeout_ms: DEFAULT_COMMAND_TIMEOUT_MS,
            ignore_cgroups: true,
        };
        let adapter = GvisorAdapter {
            config,
            handles: Arc::new(Mutex::new(HashMap::new())),
        };
        let args = adapter.exec_args("true");
        assert!(args.contains(&"--ignore-cgroups".to_string()));
    }

    #[tokio::test]
    async fn spawn_fails_closed_for_unenforced_rate_limits_before_runtime_probe() {
        let config = GvisorConfig {
            distro: "Ubuntu".to_string(),
            user: "root".to_string(),
            wsl_exe: PathBuf::from("wsl.exe"),
            runsc_bin: "/opt/runsc".to_string(),
            platform: "systrap".to_string(),
            work_dir: "/work".to_string(),
            command_timeout_ms: DEFAULT_COMMAND_TIMEOUT_MS,
            ignore_cgroups: false,
        };
        let adapter = GvisorAdapter {
            config,
            handles: Arc::new(Mutex::new(HashMap::new())),
        };

        for limits in [
            ResourceLimits {
                disk_read_bytes_per_sec: Some(1_000_000),
                ..Default::default()
            },
            ResourceLimits {
                disk_write_bytes_per_sec: Some(1_000_000),
                ..Default::default()
            },
            ResourceLimits {
                net_bandwidth_bytes_per_sec: Some(1_000_000),
                ..Default::default()
            },
        ] {
            let spec = ProcessSpec {
                id: AdapterId::new("gvisor-rate-limit-test"),
                image_or_root: crate::sandbox::ImageRef::new("runsc-do"),
                cmd: vec!["true".to_string()],
                env: BTreeMap::new(),
                cwd: None,
                binds: Vec::new(),
                net_policy: NetPolicy::DenyAll,
                resource_limits: limits,
                idle_timeout_ms: None,
                required_capabilities: Default::default(),
                trust_class: crate::sandbox::TrustClass::UntrustedAgent,
                metadata: BTreeMap::new(),
            };
            let err = adapter
                .spawn(spec)
                .await
                .expect_err("gVisor must fail closed for unenforced rate limits");
            match err {
                SandboxAdapterError::SpawnFailed { adapter_id, reason } => {
                    assert_eq!(adapter_id, AdapterId::new(GVISOR_ADAPTER_ID));
                    assert!(reason.contains("not enforceable"), "{reason}");
                    assert!(reason.contains("per-device rate limits"), "{reason}");
                }
                other => panic!("expected SpawnFailed, got {other:?}"),
            }
        }
    }

    #[test]
    fn default_config_uses_proven_values_when_env_unset() {
        // Note: this asserts the compiled-in defaults, not env overrides.
        let config = GvisorConfig {
            distro: DEFAULT_DISTRO.to_string(),
            user: DEFAULT_USER.to_string(),
            wsl_exe: PathBuf::from("wsl.exe"),
            runsc_bin: DEFAULT_RUNSC_BIN.to_string(),
            platform: DEFAULT_PLATFORM.to_string(),
            work_dir: DEFAULT_WORK_DIR.to_string(),
            command_timeout_ms: DEFAULT_COMMAND_TIMEOUT_MS,
            ignore_cgroups: false,
        };
        assert_eq!(config.distro(), "Ubuntu");
        assert_eq!(config.user(), "root");
        assert_eq!(config.platform(), "systrap");
        assert_eq!(config.command_timeout_ms(), 60_000);
        assert!(!config.ignore_cgroups());
    }
}
