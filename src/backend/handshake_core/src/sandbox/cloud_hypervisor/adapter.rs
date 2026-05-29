use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use bytes::Bytes;
use tokio::{io::AsyncWriteExt, process::Command as TokioCommand};
use uuid::Uuid;

use crate::sandbox::wsl2_podman::wsl_detection::default_wsl_exe;
use crate::sandbox::{
    AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
    IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
    SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub const CLOUD_HYPERVISOR_ADAPTER_ID: &str = "cloud_hypervisor";

/// Serial-console framing markers emitted by the initramfs `/init`. The guest
/// prints `BEGIN`, then the command's combined stdout/stderr, then
/// `END rc=<code>` before powering off.
const HSK_BEGIN_MARKER: &str = "---HSK-BEGIN---";
const HSK_END_PREFIX: &str = "---HSK-END rc=";
const HSK_END_SUFFIX: &str = "---";

/// Read-write write-back framing. For every `hsk.rw=` path the init script tars
/// the guest paths to serial as a single base64 blob between these markers; the
/// host extracts that tar and copies changed files back to the host bind path.
const HSK_FILES_BEGIN_MARKER: &str = "---HSK-FILES-BEGIN---";
const HSK_FILES_END_MARKER: &str = "---HSK-FILES-END---";

/// The PROVEN per-exec `/init` (verbatim logic). It mounts the synthetic
/// filesystems, installs busybox applets, base64-decodes the command from
/// `hsk.cmd=`, runs it between the BEGIN/END markers, and — when `hsk.rw=` lists
/// guest-relative paths — tars those paths to serial as base64 between the
/// FILES markers so the host can write them back. Finally it powers off.
const INIT_SCRIPT: &str = r#"#!/bin/busybox sh
/bin/busybox mkdir -p /proc /sys /dev /bin
/bin/busybox mount -t proc proc /proc
/bin/busybox mount -t sysfs sysfs /sys
/bin/busybox --install -s /bin
CMD=$(cat /proc/cmdline | tr ' ' '\n' | grep '^hsk.cmd=' | cut -d= -f2- | base64 -d)
RW=$(cat /proc/cmdline | tr ' ' '\n' | grep '^hsk.rw=' | cut -d= -f2- | base64 -d 2>/dev/null)
echo "---HSK-BEGIN---"
sh -c "$CMD"
echo "---HSK-END rc=$?---"
if [ -n "$RW" ]; then
  echo "---HSK-FILES-BEGIN---"
  tar -C / -cf - $RW 2>/dev/null | base64 -w0
  echo ""
  echo "---HSK-FILES-END---"
fi
poweroff -f"#;

/// Proven-working defaults for the host WSL2 sandbox layout. Every field is
/// overridable via a `HANDSHAKE_CH_*` environment variable so the adapter stays
/// disk-agnostic per [GLOBAL-PORTABILITY] (no hardcoded absolute path is baked
/// into a build the operator cannot redirect after a project move).
const DEFAULT_DISTRO: &str = "Ubuntu";
const DEFAULT_WORK_DIR: &str = "/home/ilja_smets/handshake-sandbox";
const DEFAULT_CH_BIN: &str = "/home/ilja_smets/handshake-sandbox/bin/cloud-hypervisor";
const DEFAULT_KERNEL: &str = "/home/ilja_smets/handshake-sandbox/vmlinux-6.1.102";
const DEFAULT_INITRAMFS: &str = "/home/ilja_smets/handshake-sandbox/initramfs.cpio";
/// WSL-side busybox used as the only guest userland baked into the per-exec
/// initramfs. Overridable via `HANDSHAKE_CH_BUSYBOX` so the adapter stays
/// portable per [GLOBAL-PORTABILITY].
const DEFAULT_BUSYBOX: &str = "/usr/bin/busybox";
const DEFAULT_MEMORY_MIB: u32 = 256;
const DEFAULT_VCPUS: u32 = 1;
const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 60_000;
/// Probe / log-read commands are quick; keep them well under the boot timeout.
const PROBE_TIMEOUT_MS: u64 = 15_000;

/// Configuration for [`CloudHypervisorAdapter`].
///
/// All paths are WSL-side (Linux) paths because the VM artifacts live inside
/// the WSL2 filesystem. Each field defaults to the proven host value and is
/// overridable via the matching `HANDSHAKE_CH_*` environment variable:
///
/// | field            | env var                       |
/// |------------------|-------------------------------|
/// | `distro`         | `HANDSHAKE_CH_DISTRO`         |
/// | `ch_bin`         | `HANDSHAKE_CH_BIN`            |
/// | `kernel`         | `HANDSHAKE_CH_KERNEL`         |
/// | `initramfs`      | `HANDSHAKE_CH_INITRAMFS`      |
/// | `work_dir`       | `HANDSHAKE_CH_WORK_DIR`       |
/// | `memory_mib`     | `HANDSHAKE_CH_MEMORY_MIB`     |
/// | `vcpus`          | `HANDSHAKE_CH_VCPUS`          |
/// | `command_timeout_ms` | `HANDSHAKE_CH_TIMEOUT_MS` |
///
/// The host-side `wsl.exe` launcher resolves via `PATH` (`HANDSHAKE_CH_WSL_EXE`
/// overrides it) so the Windows side stays portable too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudHypervisorConfig {
    distro: String,
    wsl_exe: PathBuf,
    ch_bin: String,
    kernel: String,
    initramfs: String,
    busybox: String,
    work_dir: String,
    memory_mib: u32,
    vcpus: u32,
    command_timeout_ms: u64,
}

impl CloudHypervisorConfig {
    pub fn distro(&self) -> &str {
        &self.distro
    }

    pub fn wsl_exe(&self) -> &Path {
        &self.wsl_exe
    }

    pub fn ch_bin(&self) -> &str {
        &self.ch_bin
    }

    pub fn kernel(&self) -> &str {
        &self.kernel
    }

    pub fn initramfs(&self) -> &str {
        &self.initramfs
    }

    pub fn busybox(&self) -> &str {
        &self.busybox
    }

    pub fn work_dir(&self) -> &str {
        &self.work_dir
    }

    pub fn memory_mib(&self) -> u32 {
        self.memory_mib
    }

    pub fn vcpus(&self) -> u32 {
        self.vcpus
    }

    pub fn command_timeout_ms(&self) -> u64 {
        self.command_timeout_ms
    }

    pub fn with_command_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.command_timeout_ms = timeout_ms;
        self
    }
}

impl Default for CloudHypervisorConfig {
    fn default() -> Self {
        Self {
            distro: env_string("HANDSHAKE_CH_DISTRO", DEFAULT_DISTRO),
            wsl_exe: std::env::var("HANDSHAKE_CH_WSL_EXE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| default_wsl_exe()),
            ch_bin: env_string("HANDSHAKE_CH_BIN", DEFAULT_CH_BIN),
            kernel: env_string("HANDSHAKE_CH_KERNEL", DEFAULT_KERNEL),
            initramfs: env_string("HANDSHAKE_CH_INITRAMFS", DEFAULT_INITRAMFS),
            busybox: env_string("HANDSHAKE_CH_BUSYBOX", DEFAULT_BUSYBOX),
            work_dir: env_string("HANDSHAKE_CH_WORK_DIR", DEFAULT_WORK_DIR),
            memory_mib: env_u32("HANDSHAKE_CH_MEMORY_MIB", DEFAULT_MEMORY_MIB),
            vcpus: env_u32("HANDSHAKE_CH_VCPUS", DEFAULT_VCPUS),
            command_timeout_ms: env_u64("HANDSHAKE_CH_TIMEOUT_MS", DEFAULT_COMMAND_TIMEOUT_MS),
        }
    }
}

fn env_string(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// One recorded host->guest directory bind for a handle. Binds are baked into
/// the per-exec initramfs at boot time (there is no virtio-fs/virtio-pci in the
/// guest kernel), and `ReadWrite` binds are written back via the serial-tar
/// channel after the guest powers off.
#[derive(Debug, Clone)]
struct BindRecord {
    /// Windows (or already-WSL) host path whose contents are baked in.
    host_path: PathBuf,
    /// Absolute guest mount point (e.g. `/work`).
    guest_path: PathBuf,
    /// Guest-relative path (e.g. `work`) used for `tar -C / $rel` write-back.
    guest_rel: String,
    mode: BindMode,
}

/// Per-handle bookkeeping for the ephemeral-microVM model. Each `exec` boots a
/// brand-new VM, so we only need to remember the last status, a kill flag, and
/// the declared binds to bake into the next exec's initramfs.
#[derive(Debug)]
struct HandleState {
    status: ProcessStatus,
    exit_code: Option<i32>,
    killed: bool,
    binds: Vec<BindRecord>,
}

impl Default for HandleState {
    fn default() -> Self {
        Self {
            // Handles start out as "running" (no exec has completed yet); the
            // ephemeral model flips this to Exited after a finished exec.
            status: ProcessStatus::Running,
            exit_code: None,
            killed: false,
            binds: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloudHypervisorAdapter {
    config: CloudHypervisorConfig,
    handles: Arc<Mutex<HashMap<Uuid, HandleState>>>,
}

impl CloudHypervisorAdapter {
    /// REAL availability probe. Verifies, in order:
    /// 1. `wsl.exe` resolves and the configured distro is registered.
    /// 2. the Cloud Hypervisor binary, kernel and initramfs all exist in WSL.
    /// 3. `/dev/kvm` is present and readable+writable to the WSL user.
    ///
    /// Any failure returns [`SandboxAdapterError::AdapterUnavailable`] so the
    /// bootstrap registry gracefully skips this adapter on non-WSL / non-KVM
    /// hosts instead of failing the whole sandbox bring-up.
    pub async fn try_new(
        config: CloudHypervisorConfig,
    ) -> Result<Self, SandboxAdapterError> {
        verify_available(&config).await?;
        Ok(Self {
            config,
            handles: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn config(&self) -> &CloudHypervisorConfig {
        &self.config
    }

    fn ensure_handle(&self, handle: &ProcessHandle) -> Result<(), SandboxAdapterError> {
        if handle.adapter_id != AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID) {
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

    /// Build the proven `wsl.exe ... cloud-hypervisor ...` argv for one
    /// ephemeral boot. `serial_log` is the WSL-side path the guest console is
    /// written to so the host can read it back after power-off. `initramfs` is
    /// the per-exec cpio (which bakes in any bound host directories), and
    /// `rw_b64` (when non-empty) is the base64-encoded space-joined list of
    /// guest-relative read-write paths the init script tars back to serial.
    fn boot_args(
        &self,
        command_b64: &str,
        rw_b64: &str,
        initramfs: &str,
        serial_log: &str,
    ) -> Vec<String> {
        let cmdline = if rw_b64.is_empty() {
            format!("console=ttyS0 hsk.cmd={command_b64}")
        } else {
            format!("console=ttyS0 hsk.cmd={command_b64} hsk.rw={rw_b64}")
        };
        vec![
            "-d".to_string(),
            self.config.distro.clone(),
            "-e".to_string(),
            self.config.ch_bin.clone(),
            "--kernel".to_string(),
            self.config.kernel.clone(),
            "--initramfs".to_string(),
            initramfs.to_string(),
            "--cmdline".to_string(),
            cmdline,
            "--serial".to_string(),
            format!("file={serial_log}"),
            "--console".to_string(),
            "off".to_string(),
            "--cpus".to_string(),
            format!("boot={}", self.config.vcpus),
            "--memory".to_string(),
            format!("size={}M", self.config.memory_mib),
        ]
    }
}

#[async_trait]
impl SandboxAdapter for CloudHypervisorAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        // Re-probe so a handle is never minted against a runtime that has gone
        // away (mirrors DockerAdapter::ensure_runtime_available). The VM itself
        // is not booted here: the ephemeral model boots a fresh VM per exec.
        verify_available(&self.config).await?;
        let handle = ProcessHandle::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            None,
            format!("hsk-ch-{}", Uuid::now_v7().simple()),
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
            .map(|map| map.get(&handle.id).map(|state| state.killed).unwrap_or(false))
            .unwrap_or(false)
        {
            return Err(spawn_failed(
                "handle was killed; spawn a fresh handle before exec",
            ));
        }

        // Snapshot the binds declared for this handle so the per-exec initramfs
        // can bake them in. RW binds also drive the serial-tar write-back.
        let binds = self
            .handles
            .lock()
            .map(|map| {
                map.get(&handle.id)
                    .map(|state| state.binds.clone())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let rw_rels: Vec<String> = binds
            .iter()
            .filter(|bind| bind.mode == BindMode::ReadWrite)
            .map(|bind| bind.guest_rel.clone())
            .collect();

        // Join argv into a single shell command line and base64-encode it for
        // the kernel cmdline. env overlay entries are prefixed as `KEY=VALUE`
        // exports so they reach the guest command's environment.
        let command_line = build_command_line(&cmd);
        let command_b64 = BASE64.encode(command_line.as_bytes());
        // `hsk.rw=` carries the base64 of the space-joined guest-relative RW
        // paths; the proven init script base64-decodes it and tars each path.
        let rw_b64 = if rw_rels.is_empty() {
            String::new()
        } else {
            BASE64.encode(rw_rels.join(" ").as_bytes())
        };

        let run_id = Uuid::now_v7().simple().to_string();
        let serial_log = format!("{}/run-{run_id}.log", self.config.work_dir);
        // Per-exec scratch root inside WSL: the initramfs build tree, the cpio,
        // and the write-back staging dir all live under it for atomic cleanup.
        let exec_root = format!("{}/exec-{run_id}", self.config.work_dir);
        let initramfs_cpio = format!("{exec_root}/initramfs.cpio");
        let timeout_ms = cmd.timeout_ms.unwrap_or(self.config.command_timeout_ms);

        // Build the per-exec initramfs (busybox + init + baked binds) inside WSL.
        // On failure we still clean up before returning.
        if let Err(error) = build_per_exec_initramfs(&self.config, &exec_root, &binds).await {
            let _ = remove_wsl_path(&self.config, &exec_root).await;
            return Err(error);
        }

        let boot_args = self.boot_args(&command_b64, &rw_b64, &initramfs_cpio, &serial_log);

        let start = Instant::now();
        let boot = run_host_command(
            self.config.wsl_exe(),
            &boot_args,
            None,
            Some(timeout_ms),
            handle.id,
        )
        .await;

        // Always attempt to read + clean up the serial log regardless of how
        // the boot child terminated; the guest may have written framed output
        // even if cloud-hypervisor returned a non-zero host exit.
        let log_bytes = read_serial_log(&self.config, &serial_log).await;
        let _ = remove_serial_log(&self.config, &serial_log).await;

        let boot = match boot {
            Ok(boot) => boot,
            Err(error) => {
                let _ = remove_wsl_path(&self.config, &exec_root).await;
                return Err(error);
            }
        };
        let duration_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

        let serial_text = log_bytes
            .as_ref()
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .unwrap_or_default();

        let parsed = match parse_serial_markers(&serial_text) {
            Some(parsed) => parsed,
            None => {
                let _ = remove_wsl_path(&self.config, &exec_root).await;
                return Err(spawn_failed(format!(
                    "microVM serial output did not contain HSK markers (host ch exit {}): stderr={}",
                    boot.exit_code,
                    boot.stderr_text()
                )));
            }
        };

        // Write-back: for every ReadWrite bind, extract the serial-tar emitted
        // between the FILES markers and copy each guest path's contents back to
        // its translated host bind path. A missing/empty files section with RW
        // binds declared is a hard error (the bind would silently not persist).
        if !rw_rels.is_empty() {
            if let Err(error) =
                write_back_rw_binds(&self.config, &exec_root, &serial_text, &binds).await
            {
                let _ = remove_wsl_path(&self.config, &exec_root).await;
                return Err(error);
            }
        }

        let _ = remove_wsl_path(&self.config, &exec_root).await;

        if let Ok(mut map) = self.handles.lock() {
            if let Some(state) = map.get_mut(&handle.id) {
                state.status = ProcessStatus::Exited {
                    code: parsed.exit_code,
                };
                state.exit_code = Some(parsed.exit_code);
            }
        }

        Ok(ExecResult {
            exit_code: parsed.exit_code,
            stdout: Bytes::from(parsed.stdout.into_bytes()),
            stderr: Bytes::new(),
            duration_ms,
        })
    }

    async fn fs_bind(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
        mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;

        // Validate the guest mount point: must be an absolute, normal path that
        // does not collide with the synthetic/kernel filesystems the init
        // script owns. The bind is baked into the initramfs at `exec` time.
        let guest_rel = validate_guest_path(&guest_path)?;

        let record = BindRecord {
            host_path,
            guest_path,
            guest_rel,
            mode,
        };

        let mut map = self
            .handles
            .lock()
            .map_err(|error| spawn_failed(format!("handle registry poisoned: {error}")))?;
        let state = map
            .get_mut(&handle.id)
            .ok_or(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            })?;
        // Replace any prior bind at the same guest path so re-binding is
        // idempotent rather than baking the directory in twice.
        state
            .binds
            .retain(|existing| existing.guest_path != record.guest_path);
        state.binds.push(record);
        Ok(())
    }

    async fn net_policy(
        &self,
        handle: &ProcessHandle,
        policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        // The ephemeral boot passes no `--net` flag, so the guest has no network
        // device at all: deny-all and loopback-only are both satisfied by the
        // absence of guest networking. An external allowlist cannot be honored
        // without wiring a tap device, so it fails closed.
        match policy {
            NetPolicy::DenyAll | NetPolicy::LoopbackOnly => Ok(()),
            NetPolicy::Allowlist(entries) if entries.is_empty() => Ok(()),
            NetPolicy::Allowlist(_) => Err(net_policy_failed(
                "cloud_hypervisor microVMs boot with no network device; external allowlist entries require a future tap/virtio-net bind and fail closed",
            )),
        }
    }

    async fn kill(
        &self,
        handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        // Mark the handle killed. Any in-flight cloud-hypervisor child for this
        // handle is terminated by run_host_command's kill_on_drop once the
        // owning exec future is dropped; for the ephemeral model there is no
        // long-lived VM to signal between execs.
        if let Ok(mut map) = self.handles.lock() {
            if let Some(state) = map.get_mut(&handle.id) {
                state.killed = true;
                state.status = ProcessStatus::Killed {
                    by_signal: _signal,
                };
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
            adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            runtime_available: true,
            filesystem_isolation_strength: IsolationStrength::VeryStrong,
            network_isolation_strength: IsolationStrength::VeryStrong,
            gpu_passthrough: GpuPassthrough::None,
            stdio_throughput_class: ThroughputClass::Low,
            win32_native_fidelity: false,
            cross_machine_portable: true,
            isolation_tier: IsolationTier::Tier3Microvm,
            requires_nested_virt: true,
            supports_snapshot: false,
        }
    }
}

/// Joined command line + env-overlay exports for one guest command.
fn build_command_line(cmd: &Command) -> String {
    let mut prefix = String::new();
    for (key, value) in &cmd.env_overlay {
        // POSIX export so the value reaches the command's environment inside the
        // busybox guest shell.
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

struct ParsedSerial {
    stdout: String,
    exit_code: i32,
}

/// Extract the text between `---HSK-BEGIN---` and `---HSK-END rc=N---` and parse
/// `N` as the guest command exit code.
fn parse_serial_markers(serial_text: &str) -> Option<ParsedSerial> {
    let begin = serial_text.find(HSK_BEGIN_MARKER)?;
    let after_begin = begin + HSK_BEGIN_MARKER.len();
    let rel_end = serial_text[after_begin..].find(HSK_END_PREFIX)?;
    let end = after_begin + rel_end;

    let body = &serial_text[after_begin..end];
    // Drop the single newline that follows the BEGIN marker line.
    let stdout = body.strip_prefix("\r\n").or_else(|| body.strip_prefix('\n')).unwrap_or(body);
    let stdout = stdout.trim_end_matches(['\r', '\n']).to_string();

    let after_prefix = &serial_text[end + HSK_END_PREFIX.len()..];
    let code_str = after_prefix.find(HSK_END_SUFFIX).map(|idx| &after_prefix[..idx])?;
    let exit_code = code_str.trim().parse::<i32>().ok()?;

    Some(ParsedSerial { stdout, exit_code })
}

async fn verify_available(config: &CloudHypervisorConfig) -> Result<(), SandboxAdapterError> {
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

    // 2. ch binary, kernel and initramfs all exist inside WSL, and 3. /dev/kvm
    // is readable+writable. A single `test` chain keeps this to one wsl call.
    let probe_script = format!(
        "test -x '{bin}' && test -f '{kernel}' && test -f '{initramfs}' && test -r /dev/kvm && test -w /dev/kvm && echo CH_OK",
        bin = config.ch_bin(),
        kernel = config.kernel(),
        initramfs = config.initramfs(),
    );
    let probe = run_host_command(
        config.wsl_exe(),
        &[
            "-d".to_string(),
            config.distro().to_string(),
            "-e".to_string(),
            "sh".to_string(),
            "-c".to_string(),
            probe_script,
        ],
        None,
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await
    .map_err(|error| unavailable(format!("wsl artifact probe failed: {error}")))?;

    if probe.exit_code != 0 || !String::from_utf8_lossy(&probe.stdout).contains("CH_OK") {
        return Err(unavailable(format!(
            "Cloud Hypervisor prerequisites missing in WSL distro `{}` (ch_bin={}, kernel={}, initramfs={}, /dev/kvm rw): {}",
            config.distro(),
            config.ch_bin(),
            config.kernel(),
            config.initramfs(),
            probe.stderr_text()
        )));
    }

    Ok(())
}

async fn read_serial_log(
    config: &CloudHypervisorConfig,
    serial_log: &str,
) -> Option<Vec<u8>> {
    let output = run_host_command(
        config.wsl_exe(),
        &[
            "-d".to_string(),
            config.distro().to_string(),
            "-e".to_string(),
            "cat".to_string(),
            serial_log.to_string(),
        ],
        None,
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await
    .ok()?;
    if output.exit_code != 0 {
        return None;
    }
    Some(output.stdout.to_vec())
}

async fn remove_serial_log(config: &CloudHypervisorConfig, serial_log: &str) -> bool {
    run_host_command(
        config.wsl_exe(),
        &[
            "-d".to_string(),
            config.distro().to_string(),
            "-e".to_string(),
            "rm".to_string(),
            "-f".to_string(),
            serial_log.to_string(),
        ],
        None,
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await
    .map(|output| output.exit_code == 0)
    .unwrap_or(false)
}

/// Recursively remove a WSL-side path (per-exec scratch root). Best-effort.
async fn remove_wsl_path(config: &CloudHypervisorConfig, path: &str) -> bool {
    run_wsl_sh(config, &format!("rm -rf {}", sh_quote_wsl(path)), PROBE_TIMEOUT_MS)
        .await
        .map(|output| output.exit_code == 0)
        .unwrap_or(false)
}

/// Run a `sh -c <script>` inside the configured WSL distro and return the raw
/// CLI output. Centralizes the `-d <distro> -e sh -c <script>` argv shape used
/// for the per-exec initramfs build and write-back staging.
async fn run_wsl_sh(
    config: &CloudHypervisorConfig,
    script: &str,
    timeout_ms: u64,
) -> Result<CliOutput, SandboxAdapterError> {
    run_host_command(
        config.wsl_exe(),
        &[
            "-d".to_string(),
            config.distro().to_string(),
            "-e".to_string(),
            "sh".to_string(),
            "-c".to_string(),
            script.to_string(),
        ],
        None,
        Some(timeout_ms),
        Uuid::nil(),
    )
    .await
}

/// Single-quote a value for safe interpolation into a WSL `sh -c` script.
fn sh_quote_wsl(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Validate a guest mount point and return its guest-relative form (the path
/// with the leading `/` stripped, e.g. `/work` -> `work`). Rejects relative
/// paths, traversal, and the synthetic/kernel filesystems owned by `/init`.
fn validate_guest_path(guest_path: &Path) -> Result<String, SandboxAdapterError> {
    let invalid = |reason: &str| SandboxAdapterError::BindGuestPathInvalid {
        guest_path: guest_path.to_path_buf(),
        reason: reason.to_string(),
    };

    // Normalize to forward-slash string regardless of how the PathBuf was
    // constructed on Windows (`PathBuf::from("/work")` keeps `/`, but be safe).
    let raw = guest_path.to_string_lossy().replace('\\', "/");
    if !raw.starts_with('/') {
        return Err(invalid("guest path must be absolute (start with `/`)"));
    }
    let trimmed = raw.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(invalid("guest path must not be the root `/`"));
    }
    let rel = trimmed.trim_start_matches('/');
    if rel.is_empty() {
        return Err(invalid("guest path must not be the root `/`"));
    }
    for segment in rel.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(invalid(
                "guest path must not contain empty, `.`, or `..` segments",
            ));
        }
    }
    // Reject mount points that overlap the kernel/synthetic filesystems the
    // init script mounts and tars, which would clobber the guest or the
    // framing channel. Compare on the first path segment.
    let first = rel.split('/').next().unwrap_or_default();
    const RESERVED_ROOTS: &[&str] = &["proc", "sys", "dev", "bin", "init", "proc/", "sys/"];
    if RESERVED_ROOTS.contains(&first) {
        return Err(invalid(
            "guest path collides with a reserved guest root (/proc, /sys, /dev, /bin, /init)",
        ));
    }
    Ok(rel.to_string())
}

/// Translate a Windows host path to its WSL `/mnt/<drive>/...` mount path so the
/// per-exec initramfs build (which runs inside WSL) can read host data. A path
/// that already looks like a POSIX path (starts with `/`) is returned as-is so
/// the adapter also works when fed WSL-native bind sources.
fn windows_to_wsl_path(host_path: &Path) -> Result<String, SandboxAdapterError> {
    let raw = host_path.to_string_lossy().to_string();

    // Already a POSIX/WSL path: use verbatim (forward slashes only).
    if raw.starts_with('/') {
        return Ok(raw.replace('\\', "/"));
    }

    // Expect a drive-letter path like `D:\a\b` or `D:/a/b`.
    let bytes = raw.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' || !bytes[0].is_ascii_alphabetic() {
        return Err(spawn_failed(format!(
            "host bind path `{raw}` is not an absolute Windows drive path (expected e.g. `D:\\dir`)"
        )));
    }
    let drive = (bytes[0] as char).to_ascii_lowercase();
    // Strip `D:` and normalize backslashes; collapse a leading separator.
    let rest = raw[2..].replace('\\', "/");
    let rest = rest.trim_start_matches('/');
    if rest.is_empty() {
        Ok(format!("/mnt/{drive}"))
    } else {
        Ok(format!("/mnt/{drive}/{rest}"))
    }
}

/// Build the per-exec initramfs inside WSL: a fresh `<exec_root>/ir` tree with
/// busybox + the proven `/init` script, with every bound host directory copied
/// in at its guest path, then packed to `<exec_root>/initramfs.cpio` via
/// `cpio -o -H newc`. Host bind sources are read through `/mnt/<drive>/...`.
async fn build_per_exec_initramfs(
    config: &CloudHypervisorConfig,
    exec_root: &str,
    binds: &[BindRecord],
) -> Result<(), SandboxAdapterError> {
    let ir = format!("{exec_root}/ir");

    // Assemble one shell script so the whole build is a single WSL round-trip.
    let mut script = String::new();
    script.push_str("set -e; ");
    script.push_str(&format!(
        "rm -rf {root}; mkdir -p {ir_bin}; cp {busybox} {ir}/bin/busybox; ",
        root = sh_quote_wsl(exec_root),
        ir_bin = sh_quote_wsl(&format!("{ir}/bin")),
        busybox = sh_quote_wsl(config.busybox()),
        ir = sh_quote_wsl(&ir),
    ));

    // Write the proven init script via a heredoc with a quoted terminator so no
    // expansion happens inside it.
    script.push_str(&format!(
        "cat > {init} <<'HSKINITEOF'\n{body}\nHSKINITEOF\n",
        init = sh_quote_wsl(&format!("{ir}/init")),
        body = INIT_SCRIPT,
    ));
    script.push_str(&format!(
        "chmod +x {init}; ",
        init = sh_quote_wsl(&format!("{ir}/init"))
    ));

    // Bake each bound directory in at its guest path. We copy the *contents*
    // (`/.`) so `/work` ends up populated rather than nested under `/work/work`.
    for bind in binds {
        let src = windows_to_wsl_path(&bind.host_path)?;
        let dst = format!("{ir}/{}", bind.guest_rel);
        script.push_str(&format!(
            "mkdir -p {dst}; cp -a {src}/. {dst}/; ",
            dst = sh_quote_wsl(&dst),
            src = sh_quote_wsl(&src),
        ));
    }

    // Pack the cpio (newc) exactly as the proven prototype does.
    script.push_str(&format!(
        "(cd {ir} && find . -print0 | cpio --null -o -H newc 2>/dev/null > {cpio}); echo HSK_BUILD_OK",
        ir = sh_quote_wsl(&ir),
        cpio = sh_quote_wsl(&format!("{exec_root}/initramfs.cpio")),
    ));

    let output = run_wsl_sh(config, &script, PROBE_TIMEOUT_MS).await?;
    if output.exit_code != 0 || !String::from_utf8_lossy(&output.stdout).contains("HSK_BUILD_OK") {
        return Err(spawn_failed(format!(
            "per-exec initramfs build failed (exit {}): {}",
            output.exit_code,
            output.stderr_text()
        )));
    }
    Ok(())
}

/// Extract the base64 serial-tar between `---HSK-FILES-BEGIN---` and
/// `---HSK-FILES-END---`, decode + untar it inside WSL to a staging dir, then
/// copy each ReadWrite bind's guest-path contents back to its host bind path
/// (via `/mnt/<drive>/...`). Fails closed if the files section is absent.
async fn write_back_rw_binds(
    config: &CloudHypervisorConfig,
    exec_root: &str,
    serial_text: &str,
    binds: &[BindRecord],
) -> Result<(), SandboxAdapterError> {
    let files_b64 = parse_files_section(serial_text).ok_or_else(|| {
        spawn_failed(
            "read-write bind declared but guest emitted no ---HSK-FILES--- section; \
             write-back cannot be honored (failing closed rather than silently dropping writes)",
        )
    })?;

    let stage = format!("{exec_root}/back");
    // Decode the base64 tar on stdin, write it, untar into the staging dir.
    let untar_script = format!(
        "set -e; mkdir -p {stage}; base64 -d > {tar}; tar -C {stage} -xf {tar}; echo HSK_UNTAR_OK",
        stage = sh_quote_wsl(&stage),
        tar = sh_quote_wsl(&format!("{exec_root}/back.tar")),
    );
    let untar = run_host_command(
        config.wsl_exe(),
        &[
            "-d".to_string(),
            config.distro().to_string(),
            "-e".to_string(),
            "sh".to_string(),
            "-c".to_string(),
            untar_script,
        ],
        Some(Bytes::from(files_b64.into_bytes())),
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await?;
    if untar.exit_code != 0 || !String::from_utf8_lossy(&untar.stdout).contains("HSK_UNTAR_OK") {
        return Err(spawn_failed(format!(
            "write-back untar failed (exit {}): {}",
            untar.exit_code,
            untar.stderr_text()
        )));
    }

    // Copy each RW bind's staged contents back onto the host bind path.
    for bind in binds.iter().filter(|b| b.mode == BindMode::ReadWrite) {
        let host_dst = windows_to_wsl_path(&bind.host_path)?;
        let staged = format!("{stage}/{}", bind.guest_rel);
        let copy_script = format!(
            "set -e; if [ -d {staged} ]; then mkdir -p {dst}; cp -a {staged}/. {dst}/; fi; echo HSK_COPY_OK",
            staged = sh_quote_wsl(&staged),
            dst = sh_quote_wsl(&host_dst),
        );
        let copy = run_wsl_sh(config, &copy_script, PROBE_TIMEOUT_MS).await?;
        if copy.exit_code != 0 || !String::from_utf8_lossy(&copy.stdout).contains("HSK_COPY_OK") {
            return Err(spawn_failed(format!(
                "write-back copy to host bind `{}` failed (exit {}): {}",
                bind.host_path.display(),
                copy.exit_code,
                copy.stderr_text()
            )));
        }
    }
    Ok(())
}

/// Extract the single base64 blob between the FILES markers (the init script
/// emits exactly one `base64 -w0` line, then a blank line, then the END marker).
fn parse_files_section(serial_text: &str) -> Option<String> {
    let begin = serial_text.find(HSK_FILES_BEGIN_MARKER)?;
    let after_begin = begin + HSK_FILES_BEGIN_MARKER.len();
    let rel_end = serial_text[after_begin..].find(HSK_FILES_END_MARKER)?;
    let end = after_begin + rel_end;
    // Keep only base64 alphabet chars; this drops the framing newlines/CRs the
    // serial console interleaves around the single `base64 -w0` payload line.
    let blob: String = serial_text[after_begin..end]
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '='))
        .collect();
    if blob.is_empty() {
        None
    } else {
        Some(blob)
    }
}

/// Host-process runner mirroring the Docker/Podman bridge style: hides the
/// console window on Windows, enforces a timeout, and maps spawn/wait failures
/// to typed adapter errors.
async fn run_host_command(
    executable: &Path,
    args: &[String],
    stdin: Option<Bytes>,
    timeout_ms: Option<u64>,
    handle_id: Uuid,
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

    let mut child = command.spawn().map_err(|error| {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            reason: format!("failed to spawn `{}`: {error}", executable.to_string_lossy()),
        }
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

/// WSL CLI commands like `wsl -l -q` emit UTF-16LE; runtime `-e cat` of a serial
/// log emits raw UTF-8. Detect heavy NUL density to decode UTF-16, else UTF-8.
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
        adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn net_policy_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::NetPolicyApplyFailed {
        adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn unavailable(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::AdapterUnavailable {
        adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn timed_out(handle_id: Uuid, timeout_ms: u64) -> SandboxAdapterError {
    let _ = handle_id;
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
        reason: format!("microVM boot/exec timed out after {timeout_ms}ms"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_begin_end_markers_with_zero_exit() {
        let serial = "[ 0.7] Run /init as init process\r\n---HSK-BEGIN---\r\nLinux\r\nhello\r\n---HSK-END rc=0---\r\n[ 0.8] reboot: Power down";
        let parsed = parse_serial_markers(serial).expect("markers present");
        assert_eq!(parsed.exit_code, 0);
        assert!(parsed.stdout.contains("Linux"));
        assert!(parsed.stdout.contains("hello"));
        assert!(!parsed.stdout.contains("HSK"));
    }

    #[test]
    fn parses_nonzero_exit_code() {
        let serial = "---HSK-BEGIN---\nboom\n---HSK-END rc=42---\n";
        let parsed = parse_serial_markers(serial).expect("markers present");
        assert_eq!(parsed.exit_code, 42);
        assert_eq!(parsed.stdout, "boom");
    }

    #[test]
    fn missing_markers_returns_none() {
        assert!(parse_serial_markers("no markers here at all").is_none());
        assert!(parse_serial_markers("---HSK-BEGIN---\nonly begin").is_none());
    }

    #[test]
    fn build_command_line_joins_argv() {
        let cmd = Command {
            argv: vec!["echo".to_string(), "hello world".to_string()],
            env_overlay: Default::default(),
            stdin: None,
            timeout_ms: None,
        };
        assert_eq!(build_command_line(&cmd), "echo 'hello world'");
    }

    #[test]
    fn windows_paths_translate_to_mnt() {
        assert_eq!(
            windows_to_wsl_path(Path::new(r"D:\a\b")).unwrap(),
            "/mnt/d/a/b"
        );
        assert_eq!(
            windows_to_wsl_path(Path::new("D:/foo")).unwrap(),
            "/mnt/d/foo"
        );
        assert_eq!(windows_to_wsl_path(Path::new(r"C:\")).unwrap(), "/mnt/c");
        // Already-POSIX paths pass through verbatim.
        assert_eq!(
            windows_to_wsl_path(Path::new("/home/x/y")).unwrap(),
            "/home/x/y"
        );
        // Non-drive relative paths are rejected.
        assert!(windows_to_wsl_path(Path::new("relative\\path")).is_err());
    }

    #[test]
    fn guest_path_validation_accepts_safe_and_rejects_reserved() {
        assert_eq!(validate_guest_path(Path::new("/work")).unwrap(), "work");
        assert_eq!(
            validate_guest_path(Path::new("/work/sub")).unwrap(),
            "work/sub"
        );
        assert!(validate_guest_path(Path::new("relative")).is_err());
        assert!(validate_guest_path(Path::new("/")).is_err());
        assert!(validate_guest_path(Path::new("/proc")).is_err());
        assert!(validate_guest_path(Path::new("/sys/x")).is_err());
        assert!(validate_guest_path(Path::new("/dev")).is_err());
        assert!(validate_guest_path(Path::new("/bin")).is_err());
        assert!(validate_guest_path(Path::new("/work/../etc")).is_err());
    }

    #[test]
    fn parses_files_section_blob_between_markers() {
        let serial = "---HSK-BEGIN---\nout\n---HSK-END rc=0---\n---HSK-FILES-BEGIN---\r\nQUJD\r\n---HSK-FILES-END---\r\n";
        assert_eq!(parse_files_section(serial).unwrap(), "QUJD");
        // No files section -> None (drives the fail-closed write-back path).
        assert!(parse_files_section("---HSK-BEGIN---\nx\n---HSK-END rc=0---\n").is_none());
    }

    #[test]
    fn default_config_uses_proven_values_when_env_unset() {
        // Note: this asserts the compiled-in defaults, not env overrides.
        let config = CloudHypervisorConfig {
            distro: DEFAULT_DISTRO.to_string(),
            wsl_exe: PathBuf::from("wsl.exe"),
            ch_bin: DEFAULT_CH_BIN.to_string(),
            kernel: DEFAULT_KERNEL.to_string(),
            initramfs: DEFAULT_INITRAMFS.to_string(),
            busybox: DEFAULT_BUSYBOX.to_string(),
            work_dir: DEFAULT_WORK_DIR.to_string(),
            memory_mib: DEFAULT_MEMORY_MIB,
            vcpus: DEFAULT_VCPUS,
            command_timeout_ms: DEFAULT_COMMAND_TIMEOUT_MS,
        };
        assert_eq!(config.memory_mib(), 256);
        assert_eq!(config.vcpus(), 1);
        assert_eq!(config.command_timeout_ms(), 60_000);
    }
}
