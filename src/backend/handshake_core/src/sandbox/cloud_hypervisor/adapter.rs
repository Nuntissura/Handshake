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
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, Command as TokioCommand},
};
use uuid::Uuid;

use crate::sandbox::wsl2_podman::wsl_detection::default_wsl_exe;
use crate::sandbox::{
    encode_guest_channel_exec_request, parse_guest_channel_exec_result, AdapterCapabilities,
    AdapterId, BindMode, BindSpec, Command, ExecResult, GpuPassthrough, IsolationStrength,
    IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus, ResourceLimits,
    SandboxAdapter, SandboxAdapterError, Signal, SnapshotRef, ThroughputClass,
};

use super::guest_agent::{
    warm_agent_unavailable_detail, CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT,
    CLOUD_HYPERVISOR_WARM_AGENT_UNAVAILABLE_REASON,
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
/bin/busybox mkdir -p /proc /sys /dev /bin /tmp
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

/// `ProcessSpec.metadata` key whose value `"persistent"` switches `spawn` from
/// the default ephemeral-per-exec model to the persistent-VM model required for
/// snapshot/restore. Any other value (or absence) keeps the proven ephemeral
/// path byte-for-byte unchanged.
pub const SANDBOX_MODE_METADATA_KEY: &str = "hsk.sandbox.mode";
/// The `SANDBOX_MODE_METADATA_KEY` value that selects the persistent-VM model.
pub const SANDBOX_MODE_PERSISTENT: &str = "persistent";
/// Legacy `ProcessSpec.metadata` key carrying the persistent-VM idle timeout in
/// milliseconds (Master Spec §3.5.7 #6 idle auto-kill). New callers should use
/// the typed `ProcessSpec.idle_timeout_ms`; this key remains a compatibility
/// fallback for pre-field persisted specs and scripts.
pub const SANDBOX_IDLE_TIMEOUT_METADATA_KEY: &str = "hsk.sandbox.idle_timeout_ms";

/// Marker the persistent `/init` records once in guest RAM. State-preservation
/// tests use the adjacent `/tmp/hsk-tick` value to prove restore resumed instead
/// of rebooting.
const HSK_BOOT_ONCE_MARKER: &str = "HSK-BOOT-ONCE";

/// The `/init` baked into the persistent-VM initramfs. It mounts the synthetic
/// filesystems, installs busybox applets, records the one-shot boot marker and
/// tick counter in guest RAM under `/tmp`, then starts a small serial-socket
/// command agent on `/dev/ttyS0`. The foreground loop keeps updating
/// `/tmp/hsk-tick` and NEVER powers off, so the VM stays live for
/// `ch-remote pause` + `snapshot`. The incrementing tick counter is the
/// observable RAM state that proves a restore resumed rather than rebooted.
const AGENT_INIT_SCRIPT: &str = r#"#!/bin/busybox sh
/bin/busybox mkdir -p /proc /sys /dev /bin /tmp
/bin/busybox mount -t proc proc /proc
/bin/busybox mount -t sysfs sysfs /sys
/bin/busybox mount -t devtmpfs devtmpfs /dev 2>/dev/null || true
/bin/busybox --install -s /bin
echo "HSK-BOOT-ONCE" >/tmp/hsk-boot-once
agent_loop() {
  stty -F /dev/ttyS0 -echo 2>/dev/null || true
  exec 3<>/dev/ttyS0 || exit 1
  echo "HSK-AGENT-READY protocol=hsk.guest_channel version=1 agent=busybox" >&3
  while IFS= read -r line <&3; do
    case "$line" in
      HSK-EXEC\ *)
        set -- $line
        req="$2"
        cmd_b64="$3"
        stdin_b64="$4"
        out="/tmp/hsk-out-$req"
        err="/tmp/hsk-err-$req"
        in="/tmp/hsk-in-$req"
        dec="/tmp/hsk-decode-err-$req"
        cmd=$(echo "$cmd_b64" | base64 -d 2>"$dec") || {
          msg=$(base64 -w0 "$dec" 2>/dev/null)
          [ -n "$msg" ] || msg="-"
          echo "HSK-AGENT-ERROR $req DECODE $msg" >&3
          rm -f "$dec"
          continue
        }
        if [ "$stdin_b64" != "-" ]; then
          echo "$stdin_b64" | base64 -d > "$in" 2>"$err"
          if [ $? -ne 0 ]; then
            msg=$(base64 -w0 "$err" 2>/dev/null)
            [ -n "$msg" ] || msg="-"
            echo "HSK-AGENT-ERROR $req STDIN_DECODE $msg" >&3
            rm -f "$out" "$err" "$in" "$dec"
            continue
          fi
          sh -c "$cmd" < "$in" > "$out" 2> "$err"
        else
          sh -c "$cmd" > "$out" 2> "$err"
        fi
        rc=$?
        out_b64=$(base64 -w0 "$out" 2>/dev/null)
        err_b64=$(base64 -w0 "$err" 2>/dev/null)
        [ -n "$out_b64" ] || out_b64="-"
        [ -n "$err_b64" ] || err_b64="-"
        echo "HSK-EXEC-DONE $req $rc $out_b64 $err_b64" >&3
        rm -f "$out" "$err" "$in" "$dec"
        ;;
      HSK-PING\ *)
        set -- $line
        echo "HSK-PONG $2" >&3
        ;;
      *)
        ;;
    esac
  done
}
while true; do agent_loop 2>/tmp/hsk-agent.err; /bin/busybox sleep 1; done &
i=0
while true; do echo "$i" >/tmp/hsk-tick; i=$((i+1)); /bin/busybox sleep 1; done"#;

/// How long to wait for the persistent VM's API socket to appear after the CH
/// child is launched (the guest must boot far enough to create it).
const PERSISTENT_BOOT_TIMEOUT_MS: u64 = 30_000;
/// Poll interval while waiting for WSL-side socket/file paths to appear.
const PERSISTENT_POLL_INTERVAL_MS: u64 = 250;
const SERIAL_AGENT_BRIDGE_MAX_ARG_FRAME_BYTES: usize = 16 * 1024;
const VM_ROOT_OWNER_PID_FILE: &str = ".hsk-owner-pid";

/// Proven-working defaults for the host WSL2 sandbox layout. Every field is
/// overridable via a `HANDSHAKE_CH_*` environment variable so the adapter stays
/// disk-agnostic per [GLOBAL-PORTABILITY] (no hardcoded absolute path is baked
/// into a build the operator cannot redirect after a project move).
const DEFAULT_DISTRO: &str = "Ubuntu";
const DEFAULT_WORK_DIR: &str = "/home/ilja_smets/handshake-sandbox";
const DEFAULT_CH_BIN: &str = "/home/ilja_smets/handshake-sandbox/bin/cloud-hypervisor";
/// `ch-remote` CLI used to drive a live persistent VM (pause + snapshot). Lives
/// beside `cloud-hypervisor` in the proven layout; derived from `ch_bin`'s
/// directory when unset, overridable via `HANDSHAKE_CH_REMOTE_BIN`.
const DEFAULT_CH_REMOTE_BIN: &str = "/home/ilja_smets/handshake-sandbox/bin/ch-remote";
const DEFAULT_KERNEL: &str = "/home/ilja_smets/handshake-sandbox/vmlinux-6.1.102";
const DEFAULT_INITRAMFS: &str = "/home/ilja_smets/handshake-sandbox/initramfs.cpio";
/// WSL-side busybox used as the only guest userland baked into the per-exec
/// initramfs. Overridable via `HANDSHAKE_CH_BUSYBOX` so the adapter stays
/// portable per [GLOBAL-PORTABILITY].
const DEFAULT_BUSYBOX: &str = "/usr/bin/busybox";
const DEFAULT_MEMORY_MIB: u32 = 256;
const DEFAULT_VCPUS: u32 = 1;
const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_BALLOON_SIZE_MIB: u32 = 0;
const DEFAULT_BALLOON_DEFLATE_ON_OOM: bool = true;
const DEFAULT_BALLOON_FREE_PAGE_REPORTING: bool = true;
/// Probe / log-read commands are quick; keep them well under the boot timeout.
const PROBE_TIMEOUT_MS: u64 = 15_000;

/// Optional Cloud Hypervisor virtio-balloon device configuration.
///
/// The device is disabled by default (`size_mib == None`) to keep the proven
/// VM boot shape unchanged. Operators can enable it with
/// `HANDSHAKE_CH_BALLOON_SIZE_MIB`; the generated CH argv uses the documented
/// `--balloon size=<n>M,deflate_on_oom=on|off,free_page_reporting=on|off`
/// form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudHypervisorBalloonConfig {
    size_mib: Option<u32>,
    deflate_on_oom: bool,
    free_page_reporting: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudHypervisorWarmAgentStatus {
    pub adapter_id: AdapterId,
    pub snapshot_supported: bool,
    pub persistent_exec_supported: bool,
    pub warm_agent_supported: bool,
    pub live_token_stream_supported: bool,
    pub required_transport: String,
    pub unavailable_reason: Option<String>,
}

impl CloudHypervisorWarmAgentStatus {
    fn unsupported() -> Self {
        Self {
            adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            snapshot_supported: true,
            persistent_exec_supported: true,
            warm_agent_supported: false,
            live_token_stream_supported: false,
            required_transport: CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT.to_string(),
            unavailable_reason: Some(CLOUD_HYPERVISOR_WARM_AGENT_UNAVAILABLE_REASON.to_string()),
        }
    }
}

impl CloudHypervisorBalloonConfig {
    pub fn disabled() -> Self {
        Self {
            size_mib: None,
            deflate_on_oom: DEFAULT_BALLOON_DEFLATE_ON_OOM,
            free_page_reporting: DEFAULT_BALLOON_FREE_PAGE_REPORTING,
        }
    }

    pub fn new(size_mib: u32, deflate_on_oom: bool, free_page_reporting: bool) -> Self {
        Self {
            size_mib: if size_mib == 0 { None } else { Some(size_mib) },
            deflate_on_oom,
            free_page_reporting,
        }
    }

    pub fn size_mib(&self) -> Option<u32> {
        self.size_mib
    }

    pub fn deflate_on_oom(&self) -> bool {
        self.deflate_on_oom
    }

    pub fn free_page_reporting(&self) -> bool {
        self.free_page_reporting
    }
}

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
/// | `balloon.size_mib` | `HANDSHAKE_CH_BALLOON_SIZE_MIB` |
/// | `balloon.deflate_on_oom` | `HANDSHAKE_CH_BALLOON_DEFLATE_ON_OOM` |
/// | `balloon.free_page_reporting` | `HANDSHAKE_CH_BALLOON_FREE_PAGE_REPORTING` |
///
/// The host-side `wsl.exe` launcher resolves via `PATH` (`HANDSHAKE_CH_WSL_EXE`
/// overrides it) so the Windows side stays portable too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudHypervisorConfig {
    distro: String,
    wsl_exe: PathBuf,
    ch_bin: String,
    ch_remote_bin: String,
    kernel: String,
    initramfs: String,
    busybox: String,
    work_dir: String,
    memory_mib: u32,
    vcpus: u32,
    command_timeout_ms: u64,
    balloon: CloudHypervisorBalloonConfig,
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

    pub fn ch_remote_bin(&self) -> &str {
        &self.ch_remote_bin
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

    pub fn balloon(&self) -> &CloudHypervisorBalloonConfig {
        &self.balloon
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
            ch_remote_bin: env_string("HANDSHAKE_CH_REMOTE_BIN", DEFAULT_CH_REMOTE_BIN),
            kernel: env_string("HANDSHAKE_CH_KERNEL", DEFAULT_KERNEL),
            initramfs: env_string("HANDSHAKE_CH_INITRAMFS", DEFAULT_INITRAMFS),
            busybox: env_string("HANDSHAKE_CH_BUSYBOX", DEFAULT_BUSYBOX),
            work_dir: env_string("HANDSHAKE_CH_WORK_DIR", DEFAULT_WORK_DIR),
            memory_mib: env_u32("HANDSHAKE_CH_MEMORY_MIB", DEFAULT_MEMORY_MIB),
            vcpus: env_u32("HANDSHAKE_CH_VCPUS", DEFAULT_VCPUS),
            command_timeout_ms: env_u64("HANDSHAKE_CH_TIMEOUT_MS", DEFAULT_COMMAND_TIMEOUT_MS),
            balloon: CloudHypervisorBalloonConfig::new(
                env_u32("HANDSHAKE_CH_BALLOON_SIZE_MIB", DEFAULT_BALLOON_SIZE_MIB),
                env_bool(
                    "HANDSHAKE_CH_BALLOON_DEFLATE_ON_OOM",
                    DEFAULT_BALLOON_DEFLATE_ON_OOM,
                ),
                env_bool(
                    "HANDSHAKE_CH_BALLOON_FREE_PAGE_REPORTING",
                    DEFAULT_BALLOON_FREE_PAGE_REPORTING,
                ),
            ),
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

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
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

/// WSL-side paths and scratch roots that identify one live persistent VM. The
/// CH child process itself is held separately (see [`PersistentChildren`])
/// because [`Child`] is neither `Clone` nor `Debug`.
#[derive(Debug, Clone)]
struct PersistentVm {
    /// Absolute WSL path of the CH API socket driving this VM (pause/snapshot).
    api_socket: String,
    /// Absolute WSL Unix socket connected to the guest serial device. The
    /// persistent initramfs uses `/dev/ttyS0` as a command-agent channel.
    agent_socket: String,
    /// Absolute WSL path of the current handle's optional console observation
    /// log. The field name is kept for compatibility with `read_handle_serial`,
    /// but persistent state proofs use the guest RAM tick read through the serial
    /// command agent. Restored handles get their own restore-owned log when
    /// `restore()` rewrites the copied CH snapshot config.
    serial_log: String,
    /// Per-VM scratch root inside WSL holding the idle initramfs build tree +
    /// cpio; removed on `kill` for atomic cleanup.
    vm_root: String,
}

/// Per-handle bookkeeping. The ephemeral-microVM model (default) boots a
/// brand-new VM per `exec`, so it only needs the last status, a kill flag, and
/// the declared binds to bake into the next exec's initramfs. A persistent
/// handle additionally carries its live-VM identity in [`PersistentVm`].
#[derive(Debug)]
struct HandleState {
    status: ProcessStatus,
    exit_code: Option<i32>,
    killed: bool,
    binds: Vec<BindRecord>,
    resource_limits: ResourceLimits,
    /// `Some` for a persistent handle (snapshot/restore model); `None` for the
    /// ephemeral default model.
    persistent: Option<PersistentVm>,
    /// Idle auto-kill (§3.5.7 #6) configuration for a persistent handle: when
    /// `Some`, the idle reaper terminates the VM once `last_active` is older than
    /// this many ms. `None` disables auto-reap.
    idle_timeout_ms: Option<u64>,
    /// Last time this handle saw activity (spawn or snapshot); drives idle reaping.
    last_active: Instant,
    /// MT (snapshot-clone uniqueness): for a handle produced by `restore()`, the
    /// source `SnapshotRef.snapshot_dir` it was restored FROM. `None` for spawned
    /// (non-restored) handles. Used to enforce clone-safety: a single snapshot
    /// must not be resumed into two concurrently-LIVE VMs, because Cloud
    /// Hypervisor resume preserves the guest's in-memory identity (system UUID,
    /// entropy pool, any baked-in secrets) — there is no VMGenID device to
    /// reseed a resumed guest, so two live restores of one snapshot would
    /// silently share identity/secrets/RNG (the Firecracker random-for-clones
    /// caveat). The original host-side isolation (separate scratch/console/socket)
    /// does NOT cover guest-internal identity.
    restored_from: Option<String>,
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
            resource_limits: ResourceLimits::default(),
            persistent: None,
            idle_timeout_ms: None,
            last_active: Instant::now(),
            restored_from: None,
        }
    }
}

/// Live CH child processes for persistent handles, keyed by handle id. Kept out
/// of [`HandleState`] (and thus out of `Debug`) because [`Child`] is not
/// `Clone`/`Debug`. `kill_on_drop(true)` means dropping the [`Child`] also
/// terminates the CH process, so removing an entry here tears down the VM.
type PersistentChildren = Arc<tokio::sync::Mutex<HashMap<Uuid, Child>>>;
type PersistentExecLocks = Arc<Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<()>>>>>;

#[derive(Debug, Clone)]
pub struct CloudHypervisorAdapter {
    config: CloudHypervisorConfig,
    handles: Arc<Mutex<HashMap<Uuid, HandleState>>>,
    /// Live CH child processes for persistent handles. Skipped in `Debug`.
    persistent_children: PersistentChildren,
    /// One in-flight serial command per persistent VM handle. Different VMs may
    /// execute independently, but one CH serial socket is a single command stream.
    persistent_exec_locks: PersistentExecLocks,
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
    pub async fn try_new(config: CloudHypervisorConfig) -> Result<Self, SandboxAdapterError> {
        verify_available(&config).await?;
        Ok(Self {
            config,
            handles: Arc::new(Mutex::new(HashMap::new())),
            persistent_children: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            persistent_exec_locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn config(&self) -> &CloudHypervisorConfig {
        &self.config
    }

    /// Read the current persistent-handle optional console observation log. The
    /// method name is retained from the file-backed serial era; persistent VMs
    /// now use serial as the command channel, and state-preservation tests read
    /// `/tmp/hsk-tick` through that channel. Returns `None` for an ephemeral
    /// handle, a stale handle, or when the log cannot be read.
    pub async fn read_handle_serial(&self, handle: &ProcessHandle) -> Option<String> {
        let serial_log = self
            .handles
            .lock()
            .ok()
            .and_then(|map| {
                map.get(&handle.id)
                    .and_then(|state| state.persistent.as_ref().map(|vm| vm.serial_log.clone()))
            })
            .filter(|path| !path.is_empty())?;
        read_serial_log(&self.config, &serial_log)
            .await
            .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
    }

    /// WSL-side observation-log path for this persistent handle's current VM.
    /// Restored handles expose their own restore-owned log rather than appending
    /// to the source VM's log. Returns `None` for ephemeral or stale handles.
    pub fn handle_serial_log_path(&self, handle: &ProcessHandle) -> Option<String> {
        self.handles.lock().ok().and_then(|map| {
            map.get(&handle.id)
                .and_then(|state| state.persistent.as_ref().map(|vm| vm.serial_log.clone()))
                .filter(|path| !path.is_empty())
        })
    }

    pub fn warm_agent_status(&self) -> CloudHypervisorWarmAgentStatus {
        CloudHypervisorWarmAgentStatus::unsupported()
    }

    /// Enumerate the live persistent (snapshot-capable) handle ids this adapter
    /// is currently tracking in-process (Master Spec v02.187 §3.5.7 #8 —
    /// discovery for reclaim). Killed and ephemeral handles are excluded.
    pub fn live_persistent_handle_ids(&self) -> Vec<Uuid> {
        self.handles
            .lock()
            .map(|map| {
                map.iter()
                    .filter(|(_, state)| state.persistent.is_some() && !state.killed)
                    .map(|(id, _)| *id)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// WSL scratch roots of the persistent VMs this adapter currently owns
    /// in-process — used to distinguish live VMs from on-disk orphans.
    fn live_vm_roots(&self) -> Vec<String> {
        self.handles
            .lock()
            .map(|map| {
                map.values()
                    .filter(|state| !state.killed)
                    .filter_map(|state| state.persistent.as_ref().map(|vm| vm.vm_root.clone()))
                    .filter(|root| !root.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Discover persistent/restore VM scratch roots left on disk that this
    /// adapter does NOT own in-process — orphans from a crashed or restarted
    /// prior run (Master Spec v02.187 §3.5.7 #8/#9 — no leaked VMs across
    /// restart). Returns absolute WSL dir paths.
    pub async fn discover_orphan_vm_dirs(&self) -> Vec<String> {
        let listing = match run_wsl_sh(
            &self.config,
            &discover_orphan_vm_dirs_script(&self.config.work_dir, std::process::id()),
            PROBE_TIMEOUT_MS,
        )
        .await
        {
            Ok(listing) => listing,
            Err(_) => return Vec::new(),
        };
        let live = self.live_vm_roots();
        String::from_utf8_lossy(&listing.stdout)
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && !live.contains(line))
            .collect()
    }

    /// Reclaim orphaned persistent/restore VMs left on disk by a prior run:
    /// best-effort terminate any Cloud Hypervisor process still bound to the
    /// orphan's scratch root, then remove the scratch dir. Returns the number of
    /// orphan roots cleaned. Safe to call at adapter bring-up for crash recovery
    /// (it never touches a VM this adapter currently owns). Master Spec §3.5.7 #9.
    pub async fn reclaim_orphan_vm_dirs(&self) -> usize {
        let orphans = self.discover_orphan_vm_dirs().await;
        let mut reclaimed = 0;
        for dir in orphans {
            // Terminate any CH process whose argv references this scratch root
            // (its --api-socket / --initramfs live under it).
            let _ = run_wsl_sh(
                &self.config,
                &format!("pkill -f {d} 2>/dev/null || true", d = sh_quote_wsl(&dir)),
                PROBE_TIMEOUT_MS,
            )
            .await;
            if remove_wsl_path(&self.config, &dir).await {
                reclaimed += 1;
            }
        }
        reclaimed
    }

    /// Release a restore reservation/handle from the registry (snapshot-clone
    /// safety: a failed restore must not leave its snapshot marked live forever).
    /// Best-effort under the std lock — a poisoned lock cannot block teardown.
    fn release_restore_reservation(&self, handle_id: Uuid) {
        let _ = self.handles.lock().map(|mut map| map.remove(&handle_id));
    }

    /// ATOMIC snapshot-clone gate + reservation. Under a SINGLE acquisition of
    /// the `handles` lock this (1) refuses if `snapshot_dir` is already restored
    /// into a live (non-killed) VM, and otherwise (2) inserts a placeholder
    /// `restored_from` reservation for `handle_id` so a concurrent restore of the
    /// same snapshot sees a live clone and is refused. Holding the check and the
    /// insert under one lock is what closes the TOCTOU: two concurrent restores
    /// cannot both pass the check before either records its reservation.
    ///
    /// Extracted from [`restore`] so the clone-safety gate is unit-testable
    /// against the real adapter registry without a live WSL/KVM host; `restore`
    /// calls exactly this, so the production path is genuinely exercised.
    ///
    /// [`restore`]: CloudHypervisorAdapter::restore
    fn try_reserve_restore(
        &self,
        snapshot_dir: &str,
        handle_id: Uuid,
    ) -> Result<(), SandboxAdapterError> {
        let mut guard = self
            .handles
            .lock()
            .map_err(|error| snapshot_failed(format!("handle registry poisoned: {error}")))?;
        if snapshot_has_live_clone(
            guard
                .values()
                .map(|s| (s.restored_from.as_deref(), s.killed)),
            snapshot_dir,
        ) {
            return Err(snapshot_failed(format!(
                "snapshot `{snapshot_dir}` is already restored into a live VM; refusing a \
                 concurrent clone -- Cloud Hypervisor resume cannot reseed a running guest's \
                 identity (no VMGenID device), so two live restores would silently share the \
                 guest system UUID, entropy pool, and any baked-in secrets. Kill the existing \
                 restored VM first, or capture distinct per-worktree snapshots.",
            )));
        }
        let mut reservation = HandleState::default();
        reservation.restored_from = Some(snapshot_dir.to_string());
        guard.insert(handle_id, reservation);
        Ok(())
    }

    /// Test-only constructor that builds an adapter with empty registries and a
    /// default config WITHOUT probing WSL/KVM. Used by the snapshot-clone
    /// concurrency + reservation-leak fail-scenario tests to exercise the real
    /// `try_reserve_restore` / `release_restore_reservation` gate logic on the
    /// real adapter state. It never boots a VM, so it is host-agnostic.
    #[cfg(test)]
    fn new_for_test() -> Self {
        Self {
            config: CloudHypervisorConfig::default(),
            handles: Arc::new(Mutex::new(HashMap::new())),
            persistent_children: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            persistent_exec_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn persistent_exec_lock_for(
        &self,
        handle_id: Uuid,
    ) -> Result<Arc<tokio::sync::Mutex<()>>, SandboxAdapterError> {
        let mut locks = self.persistent_exec_locks.lock().map_err(|error| {
            spawn_failed(format!("persistent exec lock registry poisoned: {error}"))
        })?;
        Ok(locks
            .entry(handle_id)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone())
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
        memory_mib: u32,
    ) -> Result<Vec<String>, SandboxAdapterError> {
        let cmdline = if rw_b64.is_empty() {
            format!("console=ttyS0 hsk.cmd={command_b64}")
        } else {
            format!("console=ttyS0 hsk.cmd={command_b64} hsk.rw={rw_b64}")
        };
        let mut args = vec![
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
            format!("size={memory_mib}M"),
        ];
        append_balloon_args(&mut args, memory_mib, self.config.balloon())?;
        Ok(args)
    }

    fn persistent_boot_args(
        &self,
        api_socket: &str,
        idle_cpio: &str,
        serial_log: &str,
        agent_socket: &str,
        memory_mib: u32,
    ) -> Result<Vec<String>, SandboxAdapterError> {
        let mut args = vec![
            "-d".to_string(),
            self.config.distro.clone(),
            "-e".to_string(),
            self.config.ch_bin.clone(),
            "--api-socket".to_string(),
            api_socket.to_string(),
            "--kernel".to_string(),
            self.config.kernel.clone(),
            "--initramfs".to_string(),
            idle_cpio.to_string(),
            "--cmdline".to_string(),
            "console=hvc0".to_string(),
            "--serial".to_string(),
            format!("socket={agent_socket}"),
            "--console".to_string(),
            format!("file={serial_log}"),
            "--cpus".to_string(),
            format!("boot={}", self.config.vcpus),
            "--memory".to_string(),
            format!("size={memory_mib}M"),
        ];
        append_balloon_args(&mut args, memory_mib, self.config.balloon())?;
        Ok(args)
    }

    /// Boot a long-lived idle persistent VM with an API socket (the snapshot/
    /// restore model). Builds a per-VM idle initramfs (busybox + an `/init` that
    /// loops updating `/tmp/hsk-tick` and never powers off), with declared
    /// read-only binds already baked into the guest image so warm-started model
    /// agents can see the same model/runner paths after restore. Launches CH as
    /// a retained background child, waits for the API socket to appear, and
    /// registers a persistent handle. Absolute WSL paths are used throughout so
    /// the snapshot config records an absolute serial path that a restored VM
    /// resolves regardless of the CH process working directory.
    async fn spawn_persistent(
        &self,
        idle_timeout_ms: Option<u64>,
        resource_limits: ResourceLimits,
        binds: Vec<BindRecord>,
    ) -> Result<ProcessHandle, SandboxAdapterError> {
        let vm_id = Uuid::now_v7().simple().to_string();
        let vm_root = format!("{}/persistent-{vm_id}", self.config.work_dir);
        let api_socket = format!("{vm_root}/ch.sock");
        let agent_socket = format!("{vm_root}/agent.sock");
        let serial_log = format!("{vm_root}/serial.log");
        let idle_cpio = format!("{vm_root}/idle.cpio");

        if let Err(error) = build_idle_initramfs(&self.config, &vm_root, &idle_cpio, &binds).await {
            let _ = remove_wsl_path(&self.config, &vm_root).await;
            return Err(error);
        }
        if let Err(error) = mark_vm_root_owned(&self.config, &vm_root).await {
            let _ = remove_wsl_path(&self.config, &vm_root).await;
            return Err(error);
        }

        // Boot args for the persistent idle VM (absolute serial path so the
        // snapshot config is CWD-independent).
        let memory_mib = memory_mib_from_limits(&resource_limits, self.config.memory_mib);
        let boot_args = self.persistent_boot_args(
            &api_socket,
            &idle_cpio,
            &serial_log,
            &agent_socket,
            memory_mib,
        )?;

        let child = match spawn_persistent_child(self.config.wsl_exe(), &boot_args) {
            Ok(child) => child,
            Err(error) => {
                let _ = remove_wsl_path(&self.config, &vm_root).await;
                return Err(error);
            }
        };

        // Wait for the guest to boot far enough that CH creates the API socket
        // (the readiness signal). The idle `/init` also emits HSK-BOOT-ONCE on
        // the serial console, but that marker is asserted by the snapshot test,
        // not gated here.
        if let Err(error) =
            wait_for_wsl_path(&self.config, &api_socket, PERSISTENT_BOOT_TIMEOUT_MS).await
        {
            let _ = remove_wsl_path(&self.config, &vm_root).await;
            return Err(error);
        }
        if let Err(error) =
            wait_for_wsl_path(&self.config, &agent_socket, PERSISTENT_BOOT_TIMEOUT_MS).await
        {
            let _ = remove_wsl_path(&self.config, &vm_root).await;
            return Err(error);
        }

        let handle = ProcessHandle::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            None,
            format!("hsk-ch-persistent-{vm_id}"),
        );
        let vm = PersistentVm {
            api_socket,
            agent_socket,
            serial_log,
            vm_root,
        };
        // Register the handle state FIRST (the fallible std-lock insert); only
        // after it succeeds do we park the retained CH child. If we parked the
        // child first and the handles lock were poisoned, the early return would
        // strand a live CH process with no `handles` entry — unreachable by
        // `kill` (which resolves the VM via `handles`) and leaked until the whole
        // adapter Arc drops.
        let mut hstate = HandleState::default();
        hstate.persistent = Some(vm);
        hstate.idle_timeout_ms = idle_timeout_ms;
        hstate.resource_limits = resource_limits;
        hstate.binds = binds;
        self.handles
            .lock()
            .map_err(|error| spawn_failed(format!("handle registry poisoned: {error}")))?
            .insert(handle.id, hstate);
        self.persistent_children
            .lock()
            .await
            .insert(handle.id, child);

        // Idle auto-kill (Master Spec v02.187 §3.5.7 #6): if an idle timeout was
        // requested, arm a background reaper so a persistent VM whose owner never
        // calls kill() still self-reaps once idle (reinforces CX-503D).
        if idle_timeout_ms.is_some() {
            self.spawn_idle_reaper(handle.clone());
        }
        Ok(handle)
    }

    /// Mark a handle as active (resets its idle clock). Called on spawn and on
    /// snapshot so the idle reaper only fires after genuine inactivity.
    fn touch_activity(&self, id: Uuid) {
        if let Ok(mut map) = self.handles.lock() {
            if let Some(state) = map.get_mut(&id) {
                state.last_active = Instant::now();
            }
        }
    }

    /// Background idle reaper for a persistent handle (§3.5.7 #6). Polls the
    /// handle's `last_active`; once idle longer than its `idle_timeout_ms`, it
    /// terminates the VM. Exits cleanly when the handle is killed or removed.
    fn spawn_idle_reaper(&self, handle: ProcessHandle) {
        let adapter = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let (killed, timeout_ms, last_active) = match adapter.handles.lock() {
                    Ok(map) => match map.get(&handle.id) {
                        Some(state) => (state.killed, state.idle_timeout_ms, state.last_active),
                        None => return,
                    },
                    Err(_) => return,
                };
                if killed {
                    return;
                }
                match timeout_ms {
                    Some(ms) if last_active.elapsed() >= Duration::from_millis(ms) => {
                        let _ = adapter.kill(&handle, Signal::Term).await;
                        return;
                    }
                    _ => continue,
                }
            }
        });
    }

    async fn exec_persistent_agent(
        &self,
        handle: &ProcessHandle,
        vm: PersistentVm,
        cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        if vm.agent_socket.trim().is_empty() {
            return Err(spawn_failed(
                "persistent guest agent socket is missing; refusing dirty fallback to cold exec",
            ));
        }
        self.touch_activity(handle.id);
        let request_id = Uuid::now_v7().simple().to_string();
        let command_line = build_command_line(&cmd);
        let frame =
            encode_guest_channel_exec_request(&request_id, &command_line, cmd.stdin.as_ref())?;
        let frame_arg = encode_serial_agent_bridge_frame_arg(&frame)?;
        let timeout_ms = cmd.timeout_ms.unwrap_or(self.config.command_timeout_ms);
        let started = Instant::now();
        let exec_lock = self.persistent_exec_lock_for(handle.id)?;
        let _guard =
            match tokio::time::timeout(Duration::from_millis(timeout_ms), exec_lock.lock()).await {
                Ok(guard) => guard,
                Err(_) => {
                    return Err(spawn_failed(format!(
                        "persistent guest channel is busy for handle {}; timed out waiting for \
                     the in-flight exec after {timeout_ms}ms",
                        handle.id
                    )));
                }
            };
        let remaining_ms = timeout_ms
            .saturating_sub(started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64);
        if remaining_ms == 0 {
            return Err(spawn_failed(format!(
                "persistent guest channel is busy for handle {}; no timeout budget remains after \
                 waiting for the in-flight exec",
                handle.id
            )));
        }
        let line = match run_serial_agent_rpc(
            &self.config,
            &vm.agent_socket,
            &request_id,
            frame_arg,
            remaining_ms,
            handle.id,
        )
        .await
        {
            Ok(line) => line,
            Err(error) => {
                let _ = self.kill(handle, Signal::Kill).await;
                return Err(error);
            }
        };
        let parsed = match parse_guest_channel_exec_result(&line, &request_id) {
            Ok(parsed) => parsed,
            Err(error) => {
                let _ = self.kill(handle, Signal::Kill).await;
                return Err(error);
            }
        };
        Ok(ExecResult {
            exit_code: parsed.exit_code,
            stdout: parsed.stdout,
            stderr: parsed.stderr,
            duration_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        })
    }
}

#[async_trait]
impl SandboxAdapter for CloudHypervisorAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        validate_supported_resource_limits(&spec.resource_limits)?;

        // Re-probe so a handle is never minted against a runtime that has gone
        // away (mirrors DockerAdapter::ensure_runtime_available).
        verify_available(&self.config).await?;

        // Network policy is declared at create time (Master Spec §3.5.7 #5). CH
        // microVMs boot with no network device, so deny-all / loopback-only / an
        // empty allowlist are satisfied by construction; a NON-empty allowlist
        // cannot be honored without a tap/virtio-net bind, so fail closed at
        // spawn instead of silently accepting an unenforceable policy (the
        // separate net_policy() applies the same guard for post-spawn calls).
        if let NetPolicy::Allowlist(entries) = &spec.net_policy {
            if !entries.is_empty() {
                return Err(net_policy_failed(
                    "cloud_hypervisor microVMs boot with no network device; a non-empty \
                     net_policy allowlist cannot be honored and fails closed at spawn",
                ));
            }
        }

        // Persistent-VM model (snapshot/restore): boot a long-lived idle VM with
        // an API socket. Selected ONLY when the caller marks the spec; otherwise
        // the proven ephemeral-per-exec path below is byte-for-byte unchanged.
        let persistent = spec
            .metadata
            .get(SANDBOX_MODE_METADATA_KEY)
            .map(|value| value == SANDBOX_MODE_PERSISTENT)
            .unwrap_or(false);
        if persistent {
            let idle_timeout_ms = persistent_idle_timeout_ms(&spec)?;
            let binds = persistent_bind_records_from_spec(&spec.binds)?;
            return self
                .spawn_persistent(idle_timeout_ms, spec.resource_limits.clone(), binds)
                .await;
        }
        if spec.idle_timeout_ms.is_some() {
            return Err(spawn_failed(
                "ProcessSpec.idle_timeout_ms requires hsk.sandbox.mode=persistent on \
                 cloud_hypervisor; refusing to silently ignore a typed idle timeout on \
                 the ephemeral per-exec path",
            ));
        }

        // Ephemeral model: the VM itself is not booted here, a fresh VM is
        // booted per exec.
        let handle = ProcessHandle::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            None,
            format!("hsk-ch-{}", Uuid::now_v7().simple()),
        );
        self.handles
            .lock()
            .map_err(|error| spawn_failed(format!("handle registry poisoned: {error}")))?
            .insert(
                handle.id,
                HandleState {
                    resource_limits: spec.resource_limits,
                    ..HandleState::default()
                },
            );
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
        let persistent_vm = {
            self.handles
                .lock()
                .map_err(|error| spawn_failed(format!("handle registry poisoned: {error}")))?
                .get(&handle.id)
                .and_then(|state| state.persistent.clone())
        };
        if let Some(vm) = persistent_vm {
            return self.exec_persistent_agent(handle, vm, cmd).await;
        }

        // Snapshot the binds declared for this handle so the per-exec initramfs
        // can bake them in. RW binds also drive the serial-tar write-back.
        let (binds, resource_limits) = self
            .handles
            .lock()
            .map(|map| {
                map.get(&handle.id)
                    .map(|state| (state.binds.clone(), state.resource_limits.clone()))
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

        let boot_args = self.boot_args(
            &command_b64,
            &rw_b64,
            &initramfs_cpio,
            &serial_log,
            memory_mib_from_limits(&resource_limits, self.config.memory_mib),
        )?;

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
        if state.persistent.is_some() {
            return Err(spawn_failed(
                "persistent cloud_hypervisor fs_bind after boot is unsupported; declare binds on \
                 ProcessSpec.binds before spawn so they are baked into the persistent initramfs",
            ));
        }
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

        // Persistent handle: terminate the live CH child and clean its socket +
        // scratch root. For the ephemeral model there is no long-lived VM to
        // signal between execs (any in-flight child is reaped by
        // run_host_command's kill_on_drop once its exec future is dropped).
        let persistent = self.handles.lock().ok().and_then(|map| {
            map.get(&handle.id)
                .and_then(|state| state.persistent.clone())
        });
        if let Some(vm) = persistent {
            // Dropping the Child terminates the CH process (kill_on_drop). Also
            // issue an explicit kill so termination does not depend solely on
            // drop ordering, then best-effort clean the socket + scratch root.
            //
            // Take the child OUT of the registry under a tight lock scope and
            // wait OUTSIDE the lock with a bound: holding the async mutex across
            // an unbounded `child.wait().await` would serialize every other
            // persistent spawn/restore/kill on the same lock, and a wedged CH
            // child would hold it forever, deadlocking the whole persistent path.
            let child = self.persistent_children.lock().await.remove(&handle.id);
            if let Some(mut child) = child {
                let _ = child.start_kill();
                // kill_on_drop(true) still reaps the process if the wait elapses.
                let _ = tokio::time::timeout(Duration::from_millis(PROBE_TIMEOUT_MS), child.wait())
                    .await;
            }
            let _ = remove_wsl_path(&self.config, &vm.api_socket).await;
            let _ = remove_wsl_path(&self.config, &vm.vm_root).await;
        }

        if let Ok(mut map) = self.handles.lock() {
            if let Some(state) = map.get_mut(&handle.id) {
                state.killed = true;
                state.status = ProcessStatus::Killed { by_signal: _signal };
            }
        }
        if let Ok(mut locks) = self.persistent_exec_locks.lock() {
            locks.remove(&handle.id);
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

    /// Capture the full live state of a persistent VM (Master Spec v02.187
    /// §3.5.7 #7). Pauses the guest via `ch-remote pause`, then `ch-remote
    /// snapshot file://<dir>` into a fresh, empty, per-snapshot directory
    /// (CH requires the destination to exist and be empty). The resulting
    /// `config.json` + `state.json` + memory-range files fully describe the
    /// paused CPU + RAM + device state so [`restore`] can resume it.
    ///
    /// [`restore`]: CloudHypervisorAdapter::restore
    async fn snapshot(&self, handle: &ProcessHandle) -> Result<SnapshotRef, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        // Snapshotting is activity: reset the idle clock so the reaper does not
        // race a VM that is actively being checkpointed.
        self.touch_activity(handle.id);
        let vm = self
            .handles
            .lock()
            .ok()
            .and_then(|map| {
                map.get(&handle.id)
                    .and_then(|state| state.persistent.clone())
            })
            .ok_or_else(|| {
                snapshot_failed(
                    "snapshot requires a persistent handle; spawn with metadata \
                     `hsk.sandbox.mode=persistent` (the ephemeral model has no live VM to capture)",
                )
            })?;

        // Record the source VM's absolute serial path as snapshot observation
        // metadata. Restore now copies the snapshot and rewrites the copy's
        // `serial.file` to a restore-owned log, so this path describes the
        // captured source stream only; restored handles attach their own serial
        // path during `restore`.
        let snap_ref = SnapshotRef::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            format!("{}/snap-{}", self.config.work_dir, Uuid::now_v7().simple()),
        )
        .with_observe_path(vm.serial_log.clone());

        // CH requires the snapshot destination to exist and be empty.
        let mkdir = run_wsl_sh(
            &self.config,
            &format!(
                "rm -rf {dir} && mkdir -p {dir} && echo HSK_SNAPDIR_OK",
                dir = sh_quote_wsl(&snap_ref.snapshot_dir)
            ),
            PROBE_TIMEOUT_MS,
        )
        .await?;
        if mkdir.exit_code != 0
            || !String::from_utf8_lossy(&mkdir.stdout).contains("HSK_SNAPDIR_OK")
        {
            return Err(snapshot_failed(format!(
                "failed to prepare empty snapshot dir `{}`: {}",
                snap_ref.snapshot_dir,
                mkdir.stderr_text()
            )));
        }

        // 1. Pause the live guest so the captured memory is consistent.
        let pause = run_host_command(
            self.config.wsl_exe(),
            &ch_remote_args(&self.config, &vm.api_socket, &["pause".to_string()]),
            None,
            Some(PROBE_TIMEOUT_MS),
            handle.id,
        )
        .await?;
        if pause.exit_code != 0 {
            // Pause may have left the VM in an indeterminate state; best-effort
            // resume + drop the empty snapshot dir created above so a failed
            // capture leaves no wedged VM or stray dir.
            best_effort_resume(&self.config, &vm.api_socket).await;
            let _ = remove_wsl_path(&self.config, &snap_ref.snapshot_dir).await;
            return Err(snapshot_failed(format!(
                "ch-remote pause failed (exit {}): {}",
                pause.exit_code,
                pause.stderr_text()
            )));
        }

        // 2. Snapshot to the prepared dir (positional file:// URL).
        let snapshot = run_host_command(
            self.config.wsl_exe(),
            &ch_remote_args(
                &self.config,
                &vm.api_socket,
                &[
                    "snapshot".to_string(),
                    format!("file://{}", snap_ref.snapshot_dir),
                ],
            ),
            None,
            Some(self.config.command_timeout_ms),
            handle.id,
        )
        .await?;
        if snapshot.exit_code != 0 {
            // VM is paused (pause succeeded above); resume it and drop the
            // partial snapshot dir so a failed capture leaves no wedged VM or
            // half-written snapshot behind.
            best_effort_resume(&self.config, &vm.api_socket).await;
            let _ = remove_wsl_path(&self.config, &snap_ref.snapshot_dir).await;
            return Err(snapshot_failed(format!(
                "ch-remote snapshot failed (exit {}): {}",
                snapshot.exit_code,
                snapshot.stderr_text()
            )));
        }

        // Verify CH actually produced the expected snapshot artifacts so a
        // silent partial capture cannot masquerade as success.
        let verify = run_wsl_sh(
            &self.config,
            &format!(
                "test -f {dir}/config.json && test -f {dir}/state.json && echo HSK_SNAP_OK",
                dir = sh_quote_wsl(&snap_ref.snapshot_dir)
            ),
            PROBE_TIMEOUT_MS,
        )
        .await?;
        if !String::from_utf8_lossy(&verify.stdout).contains("HSK_SNAP_OK") {
            // Snapshot reported success but the expected artifacts are absent;
            // resume the paused VM and drop the partial dir.
            best_effort_resume(&self.config, &vm.api_socket).await;
            let _ = remove_wsl_path(&self.config, &snap_ref.snapshot_dir).await;
            return Err(snapshot_failed(format!(
                "snapshot dir `{}` is missing config.json/state.json after ch-remote snapshot",
                snap_ref.snapshot_dir
            )));
        }

        Ok(snap_ref)
    }

    /// Restore a snapshot into a brand-new, fully independent persistent VM that
    /// resumes from the captured live state (no reboot). The snapshot is first
    /// copied into a per-restore scratch root and the copy's CH console/serial
    /// endpoints are rewritten to restore-owned paths; CH then restores from the COPY with
    /// `--restore source_url=file://<copy>,resume=true` on a new API socket. The
    /// original snapshot is left intact (re-restorable; two restores are
    /// independent), and the restored guest writes its serial to its own log, so
    /// tearing down the original VM cannot delete the snapshot or the restored
    /// VM's console.
    async fn restore(&self, snapshot: &SnapshotRef) -> Result<ProcessHandle, SandboxAdapterError> {
        verify_available(&self.config).await?;
        if snapshot.adapter_id != AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID) {
            return Err(snapshot_failed(format!(
                "snapshot was produced by adapter `{}`, not cloud_hypervisor",
                snapshot.adapter_id
            )));
        }

        // Snapshot-clone uniqueness gate: refuse to resume a snapshot that is
        // ALREADY live in another VM. CH resume preserves the guest's in-memory
        // identity/entropy (there is no VMGenID device to reseed a resumed
        // guest), so a second CONCURRENT restore would silently replicate the
        // guest's system UUID, entropy pool, and any baked-in secrets across two
        // live VMs. The per-restore scratch/console/socket isolation below covers
        // only HOST-side resources, not guest-internal identity. Sequential
        // restore-after-kill is fine (the prior VM is flagged `killed`); only
        // concurrently-live clones are refused.
        // Mint the restored handle id EARLY so the clone-safety check and the
        // reservation are ATOMIC under one lock acquisition: otherwise two
        // concurrent restore() calls of the same snapshot could both pass the
        // check before either records its handle (TOCTOU), defeating the gate.
        let handle = ProcessHandle::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            None,
            format!("hsk-ch-restored-{}", Uuid::now_v7().simple()),
        );
        // ATOMIC clone-safety gate + reservation under one lock (TOCTOU-safe):
        // refuse if the snapshot is already live in another VM, otherwise insert
        // a placeholder `restored_from` reservation so a concurrent restore of the
        // same snapshot sees a live clone and is refused. The placeholder is
        // promoted to the real PersistentVm on success, and REMOVED on every
        // failure path below (otherwise a failed restore would block the snapshot
        // forever).
        self.try_reserve_restore(&snapshot.snapshot_dir, handle.id)?;

        // Restore into a FULLY INDEPENDENT sandbox (Master Spec v02.187 §3.5.7
        // #7: a captured state is "re-spawned … never mutated in place as
        // authority"). We do NOT restore from the snapshot dir directly and we
        // do NOT reuse the original VM's serial log:
        //   * Restoring from the snapshot dir made the dir the restored VM's
        //     scratch root, so kill() recursively deleted the snapshot —
        //     single-use capture + double-restore clobber.
        //   * Reusing the original's baked-in serial path tied the restored VM's
        //     console to the original's; killing the original unlinked it.
        // Instead, copy the snapshot into a per-restore scratch root, rewrite the
        // copy's CH console log + serial agent socket to restore-owned paths, and
        // restore from the COPY. The original snapshot is left intact and
        // re-restorable, and the restored VM owns its own channel + scratch.
        let restore_root = format!(
            "{}/restore-{}",
            self.config.work_dir,
            Uuid::now_v7().simple()
        );
        let restore_serial = format!("{restore_root}/serial.log");
        let restore_agent_socket = format!("{restore_root}/agent.sock");
        let restore_config = format!("{restore_root}/config.json");
        let api_socket = format!("{restore_root}/ch.sock");

        // New snapshots contain a serial socket for the command agent plus a
        // console file for observations. Older wave-1 snapshots only contain a
        // file-backed serial log; preserve that shape so restore stays backward
        // compatible and persistent exec fails closed on old handles rather than
        // pretending an agent exists.
        let prep = run_wsl_sh(
            &self.config,
            &format!(
                "set -e; rm -rf {root}; mkdir -p {root}; cp -a {src}/. {root}/; \
                 test -f {cfg} || {{ echo HSK_NO_CONFIG; exit 1; }}; \
                 if command -v jq >/dev/null 2>&1; then \
                   if jq -e '.serial.socket? // empty' {cfg} >/dev/null; then has_agent=1; else has_agent=0; fi; \
                   jq --arg ser {ser} --arg agent {agent} 'if (.serial.socket? // null) != null then del(.serial.file) | .serial.socket=$agent | .console.file=$ser else .serial.file=$ser end' {cfg} > {cfg}.tmp && mv {cfg}.tmp {cfg}; \
                 elif command -v python3 >/dev/null 2>&1; then \
                   has_agent=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); serial=d.setdefault(\"serial\",{{}}); has=1 if serial.get(\"socket\") else 0; (serial.pop(\"file\",None), serial.__setitem__(\"socket\",sys.argv[2]), d.setdefault(\"console\",{{}}).__setitem__(\"file\",sys.argv[3])) if has else serial.__setitem__(\"file\",sys.argv[3]); json.dump(d,open(sys.argv[1],\"w\")); print(has)' {cfg} {agent} {ser}); \
                 else echo HSK_NO_JSON_TOOL; exit 1; fi; \
                 echo HSK_RESTORE_AGENT_SOCKET=$has_agent; \
                 echo HSK_RESTORE_PREP_OK",
                root = sh_quote_wsl(&restore_root),
                src = sh_quote_wsl(&snapshot.snapshot_dir),
                cfg = sh_quote_wsl(&restore_config),
                ser = sh_quote_wsl(&restore_serial),
                agent = sh_quote_wsl(&restore_agent_socket),
            ),
            self.config.command_timeout_ms,
        )
        .await;
        let prep = match prep {
            Ok(prep) => prep,
            Err(error) => {
                let _ = remove_wsl_path(&self.config, &restore_root).await;
                self.release_restore_reservation(handle.id);
                return Err(error);
            }
        };
        if !String::from_utf8_lossy(&prep.stdout).contains("HSK_RESTORE_PREP_OK") {
            let _ = remove_wsl_path(&self.config, &restore_root).await;
            self.release_restore_reservation(handle.id);
            return Err(snapshot_failed(format!(
                "failed to prepare independent restore copy from `{}` (exit {}): {}",
                snapshot.snapshot_dir,
                prep.exit_code,
                prep.stderr_text()
            )));
        }
        let restore_has_agent_socket =
            String::from_utf8_lossy(&prep.stdout).contains("HSK_RESTORE_AGENT_SOCKET=1");
        if let Err(error) = mark_vm_root_owned(&self.config, &restore_root).await {
            let _ = remove_wsl_path(&self.config, &restore_root).await;
            self.release_restore_reservation(handle.id);
            return Err(error);
        }

        let restore_args = vec![
            "-d".to_string(),
            self.config.distro.clone(),
            "-e".to_string(),
            self.config.ch_bin.clone(),
            "--api-socket".to_string(),
            api_socket.clone(),
            "--restore".to_string(),
            format!("source_url=file://{restore_root},resume=true"),
        ];

        let child = match spawn_persistent_child(self.config.wsl_exe(), &restore_args) {
            Ok(child) => child,
            Err(error) => {
                let _ = remove_wsl_path(&self.config, &restore_root).await;
                self.release_restore_reservation(handle.id);
                return Err(error);
            }
        };

        // Wait for the restored VM's API socket to appear (proves CH came up).
        // On timeout, clean the whole restore root (mirrors spawn_persistent's
        // clean-on-every-failure discipline); the `child` drops on this early
        // return, so kill_on_drop reaps the process.
        if let Err(error) =
            wait_for_wsl_path(&self.config, &api_socket, PERSISTENT_BOOT_TIMEOUT_MS).await
        {
            let _ = remove_wsl_path(&self.config, &restore_root).await;
            self.release_restore_reservation(handle.id);
            return Err(error);
        }
        if restore_has_agent_socket {
            if let Err(error) = wait_for_wsl_path(
                &self.config,
                &restore_agent_socket,
                PERSISTENT_BOOT_TIMEOUT_MS,
            )
            .await
            {
                let _ = remove_wsl_path(&self.config, &restore_root).await;
                self.release_restore_reservation(handle.id);
                return Err(error);
            }
        }

        // The restored VM owns its private restore root (the snapshot COPY) as
        // its scratch + console. kill() reclaims only this copy, never the
        // original snapshot, so the capture stays re-restorable and two restores
        // from one snapshot are fully independent.
        let vm = PersistentVm {
            api_socket,
            agent_socket: if restore_has_agent_socket {
                restore_agent_socket
            } else {
                String::new()
            },
            serial_log: restore_serial,
            vm_root: restore_root,
        };

        // Register handle state FIRST (fallible std-lock), then park the retained
        // child — same ordering rationale as spawn_persistent: never strand a
        // live CH child without a reachable `handles` entry on a poisoned lock.
        let mut state = HandleState::default();
        state.persistent = Some(vm);
        // Record the source snapshot so the clone-safety gate refuses a second
        // concurrent restore of the same snapshot; kill() flags this handle
        // `killed`, which releases the snapshot for a subsequent sequential restore.
        state.restored_from = Some(snapshot.snapshot_dir.clone());
        self.handles
            .lock()
            .map_err(|error| snapshot_failed(format!("handle registry poisoned: {error}")))?
            .insert(handle.id, state);
        self.persistent_children
            .lock()
            .await
            .insert(handle.id, child);

        Ok(handle)
    }

    async fn warm_agent_transport(
        &self,
        handle: &ProcessHandle,
    ) -> Result<Arc<dyn crate::model_runtime::WarmAgentTransport>, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let status = self.warm_agent_status();
        Err(spawn_failed(format!(
            "Cloud Hypervisor warm-agent transport unavailable: required_transport={}, \
             persistent_exec_supported={}, warm_agent_supported={}, \
             live_token_stream_supported={}, {}",
            status.required_transport,
            status.persistent_exec_supported,
            status.warm_agent_supported,
            status.live_token_stream_supported,
            warm_agent_unavailable_detail()
        )))
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
            // The persistent-VM model implements real pause+snapshot+restore via
            // Cloud Hypervisor + ch-remote (Master Spec v02.187 §3.5.7 #7).
            supports_snapshot: true,
            // Persistent exec is served by the busybox serial-socket command
            // agent. Warm-model RPC and live token streaming still require a
            // resident model-serving guest agent/image, so advertise only the
            // generic command channel here.
            supports_persistent_exec: true,
            supports_warm_agent: false,
            supports_live_token_stream: false,
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

fn memory_mib_from_limits(limits: &ResourceLimits, default_mib: u32) -> u32 {
    const MIB: u64 = 1024 * 1024;
    limits
        .memory_bytes
        .filter(|bytes| *bytes > 0)
        .map(|bytes| bytes.saturating_add(MIB - 1) / MIB)
        .map(|mib| mib.clamp(1, u64::from(u32::MAX)) as u32)
        .unwrap_or(default_mib.max(1))
}

fn append_balloon_args(
    args: &mut Vec<String>,
    memory_mib: u32,
    balloon: &CloudHypervisorBalloonConfig,
) -> Result<(), SandboxAdapterError> {
    let Some(size_mib) = balloon.size_mib() else {
        return Ok(());
    };
    if size_mib > memory_mib {
        return Err(spawn_failed(format!(
            "cloud_hypervisor balloon size {size_mib}M exceeds guest memory {memory_mib}M; \
             refusing invalid --balloon configuration"
        )));
    }
    args.push("--balloon".to_string());
    args.push(format!(
        "size={size_mib}M,deflate_on_oom={},free_page_reporting={}",
        on_off(balloon.deflate_on_oom()),
        on_off(balloon.free_page_reporting())
    ));
    Ok(())
}

fn on_off(value: bool) -> &'static str {
    if value {
        "on"
    } else {
        "off"
    }
}

const CH_RATE_LIMIT_REFILL_TIME_MS: u64 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CloudHypervisorRateLimiterArgs {
    disk: Option<String>,
    net: Option<String>,
}

fn ch_token_bucket_arg(bytes_per_sec: u64, scope: &str) -> Result<String, SandboxAdapterError> {
    if bytes_per_sec == 0 {
        return Err(spawn_failed(format!(
            "cloud_hypervisor {scope} bandwidth limit must be greater than zero bytes/sec; \
             use None to leave the device unlimited"
        )));
    }
    Ok(format!(
        "bw_size={bytes_per_sec},bw_one_time_burst={bytes_per_sec},bw_refill_time={CH_RATE_LIMIT_REFILL_TIME_MS}"
    ))
}

fn cloud_hypervisor_rate_limiter_args(
    limits: &ResourceLimits,
) -> Result<CloudHypervisorRateLimiterArgs, SandboxAdapterError> {
    let disk = match (
        limits.disk_read_bytes_per_sec,
        limits.disk_write_bytes_per_sec,
    ) {
        (None, None) => None,
        (Some(read), Some(write)) if read == write => {
            Some(ch_token_bucket_arg(read, "disk read/write")?)
        }
        (Some(_), Some(_)) => {
            return Err(spawn_failed(
                "cloud_hypervisor virtio-block exposes one shared bandwidth token bucket; \
                 asymmetric ResourceLimits disk_read_bytes_per_sec and \
                 disk_write_bytes_per_sec cannot be enforced without silently weakening one side",
            ));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(spawn_failed(
                "cloud_hypervisor virtio-block exposes one shared bandwidth token bucket; \
                 set both disk_read_bytes_per_sec and disk_write_bytes_per_sec to the same \
                 value or leave both unset",
            ));
        }
    };
    let net = limits
        .net_bandwidth_bytes_per_sec
        .map(|bytes_per_sec| ch_token_bucket_arg(bytes_per_sec, "network"))
        .transpose()?;
    Ok(CloudHypervisorRateLimiterArgs { disk, net })
}

fn validate_supported_resource_limits(limits: &ResourceLimits) -> Result<(), SandboxAdapterError> {
    let rate_limits = cloud_hypervisor_rate_limiter_args(limits)?;
    if rate_limits.disk.is_some() || rate_limits.net.is_some() {
        return Err(spawn_failed(
            "cloud_hypervisor ResourceLimits disk/net bytes-per-second token-bucket limits map \
             to CH device bw_size/bw_refill_time arguments, but this direct-initramfs adapter \
             path currently boots with no virtio-block or virtio-net device to attach them to; \
             refusing to silently ignore requested per-device rate limits",
        ));
    }
    Ok(())
}

fn persistent_idle_timeout_ms(spec: &ProcessSpec) -> Result<Option<u64>, SandboxAdapterError> {
    match spec.idle_timeout_ms {
        Some(0) => Err(spawn_failed(
            "ProcessSpec.idle_timeout_ms must be greater than zero when set; use None to disable \
             persistent VM idle auto-reaping",
        )),
        Some(value) => Ok(Some(value)),
        None => Ok(spec
            .metadata
            .get(SANDBOX_IDLE_TIMEOUT_METADATA_KEY)
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| *value > 0)),
    }
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
    let stdout = body
        .strip_prefix("\r\n")
        .or_else(|| body.strip_prefix('\n'))
        .unwrap_or(body);
    let stdout = stdout.trim_end_matches(['\r', '\n']).to_string();

    let after_prefix = &serial_text[end + HSK_END_PREFIX.len()..];
    let code_str = after_prefix
        .find(HSK_END_SUFFIX)
        .map(|idx| &after_prefix[..idx])?;
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
        "test -x {bin} && test -f {kernel} && test -f {initramfs} && test -r /dev/kvm && test -w /dev/kvm && echo CH_OK",
        bin = sh_quote_wsl(config.ch_bin()),
        kernel = sh_quote_wsl(config.kernel()),
        initramfs = sh_quote_wsl(config.initramfs()),
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

async fn read_serial_log(config: &CloudHypervisorConfig, serial_log: &str) -> Option<Vec<u8>> {
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

/// Best-effort `ch-remote resume` of a persistent VM that a failed snapshot
/// attempt left paused, so a failed capture does not wedge the source VM in the
/// paused state. Errors are ignored (the VM may already be gone or never paused).
async fn best_effort_resume(config: &CloudHypervisorConfig, api_socket: &str) {
    let _ = run_host_command(
        config.wsl_exe(),
        &ch_remote_args(config, api_socket, &["resume".to_string()]),
        None,
        Some(PROBE_TIMEOUT_MS),
        Uuid::nil(),
    )
    .await;
}

/// Recursively remove a WSL-side path (per-exec scratch root). Best-effort.
async fn remove_wsl_path(config: &CloudHypervisorConfig, path: &str) -> bool {
    run_wsl_sh(
        config,
        &format!("rm -rf {}", sh_quote_wsl(path)),
        PROBE_TIMEOUT_MS,
    )
    .await
    .map(|output| output.exit_code == 0)
    .unwrap_or(false)
}

async fn mark_vm_root_owned(
    config: &CloudHypervisorConfig,
    vm_root: &str,
) -> Result<(), SandboxAdapterError> {
    let marker = format!("{vm_root}/{VM_ROOT_OWNER_PID_FILE}");
    let output = run_wsl_sh(
        config,
        &format!(
            "printf '%s\\n' {} > {} && echo HSK_OWNER_MARKER_OK",
            std::process::id(),
            sh_quote_wsl(&marker)
        ),
        PROBE_TIMEOUT_MS,
    )
    .await?;
    if output.exit_code != 0
        || !String::from_utf8_lossy(&output.stdout).contains("HSK_OWNER_MARKER_OK")
    {
        return Err(spawn_failed(format!(
            "failed to mark persistent VM root `{vm_root}` as owned: {}",
            output.stderr_text()
        )));
    }
    Ok(())
}

fn discover_orphan_vm_dirs_script(work_dir: &str, owner_pid: u32) -> String {
    format!(
        "for d in {wd}/persistent-* {wd}/restore-*; do \
           [ -d \"$d\" ] || continue; \
           if [ \"$(cat \"$d/{owner}\" 2>/dev/null)\" = \"{owner_pid}\" ]; then continue; fi; \
           echo \"$d\"; \
         done",
        wd = sh_quote_wsl(work_dir),
        owner = VM_ROOT_OWNER_PID_FILE,
    )
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

fn persistent_bind_records_from_spec(
    binds: &[BindSpec],
) -> Result<Vec<BindRecord>, SandboxAdapterError> {
    let mut records = Vec::with_capacity(binds.len());
    for bind in binds {
        if bind.mode != BindMode::ReadOnly {
            return Err(spawn_failed(format!(
                "persistent cloud_hypervisor only supports ReadOnly binds today; bind {} at {} \
                 requested {:?}, but warm VM binds are baked into the initramfs and have no \
                 write-back or noexec remount path",
                bind.host_path.display(),
                bind.guest_path.display(),
                bind.mode
            )));
        }
        records.push(BindRecord {
            host_path: bind.host_path.clone(),
            guest_path: bind.guest_path.clone(),
            guest_rel: validate_persistent_guest_path(&bind.guest_path)?,
            mode: bind.mode,
        });
    }
    Ok(records)
}

fn validate_persistent_guest_path(guest_path: &Path) -> Result<String, SandboxAdapterError> {
    let rel = validate_guest_path(guest_path)?;
    let first = rel.split('/').next().unwrap_or_default();
    if first == "tmp" {
        return Err(SandboxAdapterError::BindGuestPathInvalid {
            guest_path: guest_path.to_path_buf(),
            reason: "persistent cloud_hypervisor binds must not target /tmp; the serial agent owns /tmp/hsk-* for exec temp files, boot proof, and snapshot tick state".to_string(),
        });
    }
    Ok(rel)
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

/// Build the per-VM agent initramfs for the persistent-VM model: a fresh
/// `<vm_root>/ir` tree with busybox + the `/init` command agent (which loops
/// updating `/tmp/hsk-tick` and never powers off), plus declared read-only
/// binds, packed to `<idle_cpio>` via `cpio -o -H newc`.
/// This mirrors `build_per_exec_initramfs` but uses the idle init script instead
/// of the per-exec one and intentionally rejects ReadWrite binds before it is
/// called because there is no persistent write-back path.
async fn build_idle_initramfs(
    config: &CloudHypervisorConfig,
    vm_root: &str,
    idle_cpio: &str,
    binds: &[BindRecord],
) -> Result<(), SandboxAdapterError> {
    let ir = format!("{vm_root}/ir");
    let mut script = String::new();
    script.push_str("set -e; ");
    script.push_str(&format!(
        "rm -rf {root}; mkdir -p {ir_bin}; cp {busybox} {ir}/bin/busybox; ",
        root = sh_quote_wsl(vm_root),
        ir_bin = sh_quote_wsl(&format!("{ir}/bin")),
        busybox = sh_quote_wsl(config.busybox()),
        ir = sh_quote_wsl(&ir),
    ));
    script.push_str(&format!(
        "cat > {init} <<'HSKIDLEEOF'\n{body}\nHSKIDLEEOF\n",
        init = sh_quote_wsl(&format!("{ir}/init")),
        body = AGENT_INIT_SCRIPT,
    ));
    script.push_str(&format!(
        "chmod +x {init}; ",
        init = sh_quote_wsl(&format!("{ir}/init"))
    ));
    for bind in binds {
        let src = windows_to_wsl_path(&bind.host_path)?;
        let dst = format!("{ir}/{}", bind.guest_rel);
        script.push_str(&format!(
            "mkdir -p {dst}; cp -a {src}/. {dst}/; ",
            dst = sh_quote_wsl(&dst),
            src = sh_quote_wsl(&src),
        ));
    }
    script.push_str(&format!(
        "(cd {ir} && find . -print0 | cpio --null -o -H newc 2>/dev/null > {cpio}); echo HSK_IDLE_OK",
        ir = sh_quote_wsl(&ir),
        cpio = sh_quote_wsl(idle_cpio),
    ));

    let output = run_wsl_sh(config, &script, PROBE_TIMEOUT_MS).await?;
    if output.exit_code != 0 || !String::from_utf8_lossy(&output.stdout).contains("HSK_IDLE_OK") {
        return Err(spawn_failed(format!(
            "idle initramfs build failed (exit {}): {}",
            output.exit_code,
            output.stderr_text()
        )));
    }
    Ok(())
}

/// Spawn a long-lived `wsl.exe ... cloud-hypervisor ...` child for a persistent
/// VM and RETAIN the [`Child`] (unlike `run_host_command`, which waits for the
/// process to exit). `kill_on_drop(true)` means dropping the returned child also
/// terminates the CH process, so the adapter can tear the VM down deterministically.
fn spawn_persistent_child(wsl_exe: &Path, args: &[String]) -> Result<Child, SandboxAdapterError> {
    let mut command = TokioCommand::new(wsl_exe);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    hide_command_window(&mut command);
    command
        .spawn()
        .map_err(|error| SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            reason: format!(
                "failed to spawn persistent cloud-hypervisor via `{}`: {error}",
                wsl_exe.to_string_lossy()
            ),
        })
}

const SERIAL_AGENT_BRIDGE_PY: &str = r#"
import base64
import select
import socket
import sys
import time

MAX_FRAME_BYTES = 1024 * 1024
path = sys.argv[1]
request_id = sys.argv[2]
timeout_ms = int(sys.argv[3])
payload = base64.b64decode(sys.argv[4].encode("ascii"))
if len(payload) > MAX_FRAME_BYTES:
    sys.stderr.write("guest channel request exceeded max frame bytes for " + request_id + "\n")
    sys.exit(125)
deadline = time.monotonic() + (timeout_ms / 1000.0)

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.settimeout(min(5.0, max(0.1, timeout_ms / 1000.0)))
sock.connect(path)
sock.setblocking(False)

buf = b""
seen = []
wanted_ready = b"HSK-AGENT-READY "
wanted_done = ("HSK-EXEC-DONE " + request_id + " ").encode("utf-8")
wanted_error = ("HSK-AGENT-ERROR " + request_id + " ").encode("utf-8")

def remember(stage, line):
    if len(seen) < 12:
        seen.append(stage + ":" + repr(line[:220]))

def debug_preview():
    tail = repr(buf[-220:])
    if seen:
        return " seen=" + " | ".join(seen) + " tail=" + tail
    return " seen=<none> tail=" + tail

try:
    sock.sendall(payload)
except OSError as error:
    sys.stderr.write("failed to send guest channel request for " + request_id + ": " + str(error) + "\n")
    sys.exit(126)

while time.monotonic() < deadline:
    remaining = max(0.0, deadline - time.monotonic())
    ready, _, _ = select.select([sock], [], [], min(0.1, remaining))
    if not ready:
        continue
    chunk = sock.recv(4096)
    if not chunk:
        break
    buf += chunk
    if len(buf) > MAX_FRAME_BYTES:
        sys.stderr.write("guest channel response exceeded max frame bytes for " + request_id + "\n")
        sys.exit(125)
    while b"\n" in buf:
        line, buf = buf.split(b"\n", 1)
        remember("response", line)
        if line.startswith(wanted_ready):
            continue
        if line.startswith(wanted_done) or line.startswith(wanted_error):
            sys.stdout.buffer.write(line + b"\n")
            sys.exit(0)

sys.stderr.write("timed out waiting for guest channel response for " + request_id + debug_preview() + "\n")
sys.exit(124)
"#;

async fn run_serial_agent_rpc(
    config: &CloudHypervisorConfig,
    agent_socket: &str,
    request_id: &str,
    frame_arg: String,
    timeout_ms: u64,
    handle_id: Uuid,
) -> Result<String, SandboxAdapterError> {
    let args = vec![
        "-d".to_string(),
        config.distro.clone(),
        "-e".to_string(),
        "python3".to_string(),
        "-u".to_string(),
        "-c".to_string(),
        SERIAL_AGENT_BRIDGE_PY.to_string(),
        agent_socket.to_string(),
        request_id.to_string(),
        timeout_ms.to_string(),
        frame_arg,
    ];
    let output = run_host_command(
        config.wsl_exe(),
        &args,
        None,
        Some(timeout_ms.saturating_add(PROBE_TIMEOUT_MS)),
        handle_id,
    )
    .await?;
    if output.exit_code != 0 {
        return Err(spawn_failed(format!(
            "persistent guest channel bridge failed (exit {}): {}",
            output.exit_code,
            output.stderr_text()
        )));
    }
    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if line.is_empty() {
        return Err(spawn_failed(
            "persistent guest channel bridge returned no response",
        ));
    }
    Ok(line)
}

fn encode_serial_agent_bridge_frame_arg(frame: &str) -> Result<String, SandboxAdapterError> {
    if frame.len() > SERIAL_AGENT_BRIDGE_MAX_ARG_FRAME_BYTES {
        return Err(spawn_failed(format!(
            "persistent guest channel frame exceeds argv bridge limit: {} > {}",
            frame.len(),
            SERIAL_AGENT_BRIDGE_MAX_ARG_FRAME_BYTES
        )));
    }
    Ok(BASE64.encode(frame.as_bytes()))
}

/// Poll inside WSL until `path` exists (for example an API socket or serial
/// agent socket a booting persistent VM creates) or `timeout_ms` elapses. Used to confirm a persistent
/// or restored VM actually came up before a handle is minted against it.
async fn wait_for_wsl_path(
    config: &CloudHypervisorConfig,
    path: &str,
    timeout_ms: u64,
) -> Result<(), SandboxAdapterError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let probe = run_wsl_sh(
            config,
            &format!(
                "test -e {p} && echo HSK_PATH_OK || echo HSK_PATH_MISSING",
                p = sh_quote_wsl(path)
            ),
            PROBE_TIMEOUT_MS,
        )
        .await?;
        if String::from_utf8_lossy(&probe.stdout).contains("HSK_PATH_OK") {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(snapshot_failed(format!(
                "persistent VM did not create `{path}` within {timeout_ms}ms (CH failed to boot)"
            )));
        }
        tokio::time::sleep(Duration::from_millis(PERSISTENT_POLL_INTERVAL_MS)).await;
    }
}

/// Build the `wsl.exe -d <distro> -e <ch_remote> --api-socket <sock> <sub...>`
/// argv used to drive a live persistent VM (pause / snapshot).
fn ch_remote_args(
    config: &CloudHypervisorConfig,
    api_socket: &str,
    subcommand: &[String],
) -> Vec<String> {
    let mut args = vec![
        "-d".to_string(),
        config.distro().to_string(),
        "-e".to_string(),
        config.ch_remote_bin().to_string(),
        "--api-socket".to_string(),
        api_socket.to_string(),
    ];
    args.extend(subcommand.iter().cloned());
    args
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

    let mut child = command
        .spawn()
        .map_err(|error| SandboxAdapterError::AdapterUnavailable {
            adapter_id: AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            reason: format!(
                "failed to spawn `{}`: {error}",
                executable.to_string_lossy()
            ),
        })?;

    if let Some(input) = stdin {
        if let Some(mut child_stdin) = child.stdin.take() {
            child_stdin
                .write_all(&input)
                .await
                .map_err(|error| spawn_failed(format!("command stdin write failed: {error}")))?;
            child_stdin
                .shutdown()
                .await
                .map_err(|error| spawn_failed(format!("command stdin close failed: {error}")))?;
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

/// Snapshot-clone uniqueness check (pure, unit-testable). Given each handle's
/// `(restored_from, killed)` provenance, return `true` when resuming
/// `snapshot_dir` would create a SECOND concurrently-live VM from the same
/// snapshot — i.e. a live (non-killed) handle was already restored from it.
///
/// CH resume cannot reseed a running guest's identity/entropy (no VMGenID
/// device), so two live restores of one snapshot silently share the guest
/// system UUID, entropy pool, and any baked-in secrets. `restore()` refuses when
/// this returns `true`. Killed handles are excluded (a snapshot is released for a
/// subsequent sequential restore once its prior VM is torn down).
fn snapshot_has_live_clone<'a>(
    handles: impl Iterator<Item = (Option<&'a str>, bool)>,
    snapshot_dir: &str,
) -> bool {
    handles
        .filter(|(_, killed)| !*killed)
        .any(|(restored_from, _)| restored_from == Some(snapshot_dir))
}

fn snapshot_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SnapshotFailed {
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
    use std::collections::BTreeMap;

    use crate::sandbox::{ImageRef, TrustClass};

    use super::*;

    #[test]
    fn snapshot_clone_safety_refuses_concurrent_live_restore() {
        let dir = "/snap/golden-a";
        // No handles yet -> safe to restore.
        assert!(!snapshot_has_live_clone(std::iter::empty(), dir));

        // A live handle restored from this snapshot -> a second restore is a clone.
        let live = vec![(Some(dir), false)];
        assert!(snapshot_has_live_clone(live.into_iter(), dir));

        // Unrelated live restore + a spawned (None) handle -> still safe.
        let others = vec![(Some("/snap/other-b"), false), (None, false)];
        assert!(!snapshot_has_live_clone(others.into_iter(), dir));
    }

    #[test]
    fn snapshot_clone_safety_allows_sequential_restore_after_kill() {
        let dir = "/snap/golden-a";
        // The prior restore of this snapshot was killed (torn down) -> the
        // snapshot is released; a fresh sequential restore is NOT a live clone.
        let after_kill = vec![(Some(dir), true)];
        assert!(!snapshot_has_live_clone(after_kill.into_iter(), dir));

        // But if one clone is killed and ANOTHER is still live, refuse.
        let one_live = vec![(Some(dir), true), (Some(dir), false)];
        assert!(snapshot_has_live_clone(one_live.into_iter(), dir));
    }

    // ===================================================================
    // FAIL-SCENARIO (1): SNAPSHOT-CLONE CONCURRENCY. Two concurrent restores
    // of ONE snapshot race through the real adapter reservation gate
    // (`try_reserve_restore`, the exact step `restore()` runs). Exactly one
    // reservation wins; the second is refused by the clone-safety gate. This
    // exercises the TOCTOU-closing atomic check-and-reserve on the real adapter
    // state without a live WSL/KVM host.
    // ===================================================================
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn fail_scenario_concurrent_restore_of_one_snapshot_admits_exactly_one() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let adapter = Arc::new(CloudHypervisorAdapter::new_for_test());
        let snapshot_dir = "/work/snap-golden-concurrent".to_string();

        // Fire N concurrent reservation attempts for the SAME snapshot, each with
        // its own freshly-minted handle id (as restore() mints one per call).
        let n = 8usize;
        let admitted = Arc::new(AtomicUsize::new(0));
        let refused = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::with_capacity(n);
        for _ in 0..n {
            let adapter = adapter.clone();
            let snapshot_dir = snapshot_dir.clone();
            let admitted = admitted.clone();
            let refused = refused.clone();
            tasks.push(tokio::spawn(async move {
                let handle_id = Uuid::now_v7();
                match adapter.try_reserve_restore(&snapshot_dir, handle_id) {
                    Ok(()) => admitted.fetch_add(1, Ordering::SeqCst),
                    Err(_) => refused.fetch_add(1, Ordering::SeqCst),
                };
            }));
        }
        for task in tasks {
            task.await.expect("reservation task joined");
        }

        assert_eq!(
            admitted.load(Ordering::SeqCst),
            1,
            "exactly one concurrent restore of a single snapshot may reserve it"
        );
        assert_eq!(
            refused.load(Ordering::SeqCst),
            n - 1,
            "every other concurrent restore of the same snapshot is refused by the clone-safety gate"
        );
        // Exactly one live (non-killed) reservation remains in the registry.
        let live = adapter
            .handles
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.restored_from.as_deref() == Some(snapshot_dir.as_str()) && !s.killed)
            .count();
        assert_eq!(live, 1, "exactly one live reservation for the snapshot");
    }

    // ===================================================================
    // FAIL-SCENARIO (5): RESERVATION LEAK on failed restore. A restore that
    // reserves the snapshot then FAILS (e.g. the WSL prep / boot step errors)
    // must RELEASE the reservation via `release_restore_reservation`, so a
    // follow-up restore of the SAME snapshot is admitted again (the snapshot is
    // not blocked forever). Exercises the real release semantics `restore`'s
    // every failure path calls.
    // ===================================================================
    #[test]
    fn fail_scenario_failed_restore_releases_reservation_for_followup() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let snapshot_dir = "/work/snap-golden-release".to_string();

        // First restore attempt reserves the snapshot...
        let first = Uuid::now_v7();
        adapter
            .try_reserve_restore(&snapshot_dir, first)
            .expect("first reservation admitted");
        // ...while reserved, a concurrent/follow-up restore is refused.
        let blocked = Uuid::now_v7();
        assert!(
            adapter.try_reserve_restore(&snapshot_dir, blocked).is_err(),
            "while a reservation is live the snapshot is refused"
        );

        // Simulate the first restore FAILING after reserving: it releases.
        adapter.release_restore_reservation(first);

        // A follow-up restore of the SAME snapshot now succeeds (no leak): the
        // failed attempt did not block the snapshot forever.
        let followup = Uuid::now_v7();
        adapter
            .try_reserve_restore(&snapshot_dir, followup)
            .expect("follow-up restore admitted after the failed restore released its reservation");

        // Exactly one live reservation (the follow-up) tracks the snapshot.
        let live = adapter
            .handles
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.restored_from.as_deref() == Some(snapshot_dir.as_str()) && !s.killed)
            .count();
        assert_eq!(live, 1, "only the admitted follow-up reservation remains");
    }

    // A killed reservation (the prior restore was torn down) does NOT block a
    // sequential restore: `try_reserve_restore` admits when the only matching
    // entry is killed, mirroring the snapshot_has_live_clone after-kill rule.
    #[test]
    fn fail_scenario_killed_reservation_does_not_block_sequential_restore() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let snapshot_dir = "/work/snap-golden-sequential".to_string();

        let first = Uuid::now_v7();
        adapter
            .try_reserve_restore(&snapshot_dir, first)
            .expect("first reservation admitted");
        // Flag the prior reservation killed (as kill() does), releasing the clone.
        adapter
            .handles
            .lock()
            .unwrap()
            .get_mut(&first)
            .unwrap()
            .killed = true;

        // A sequential restore after the kill is admitted (not a live clone).
        let second = Uuid::now_v7();
        adapter
            .try_reserve_restore(&snapshot_dir, second)
            .expect("sequential restore after kill is admitted");
    }

    #[test]
    fn agent_init_loops_serves_serial_agent_and_never_powers_off() {
        // The persistent init must print the one-shot boot marker, serve the
        // serial command agent, maintain the in-guest tick, and NEVER poweroff.
        assert!(AGENT_INIT_SCRIPT.contains(HSK_BOOT_ONCE_MARKER));
        assert!(AGENT_INIT_SCRIPT.contains("mkdir -p /proc /sys /dev /bin /tmp"));
        assert!(AGENT_INIT_SCRIPT.contains("devtmpfs"));
        assert!(AGENT_INIT_SCRIPT.contains("HSK-AGENT-READY"));
        assert!(AGENT_INIT_SCRIPT.contains("/dev/ttyS0"));
        assert!(AGENT_INIT_SCRIPT.contains("stty -F /dev/ttyS0 -echo"));
        assert!(AGENT_INIT_SCRIPT.contains("exec 3<>/dev/ttyS0"));
        assert!(AGENT_INIT_SCRIPT.contains("/tmp/hsk-agent.err"));
        assert!(AGENT_INIT_SCRIPT.contains("/tmp/hsk-tick"));
        assert!(AGENT_INIT_SCRIPT.contains("while true"));
        assert!(
            !AGENT_INIT_SCRIPT.contains("poweroff"),
            "persistent agent init must not power off the persistent VM"
        );
    }

    #[test]
    fn persistent_mode_marker_constants() {
        assert_eq!(SANDBOX_MODE_METADATA_KEY, "hsk.sandbox.mode");
        assert_eq!(SANDBOX_MODE_PERSISTENT, "persistent");
    }

    #[test]
    fn persistent_idle_timeout_prefers_typed_field_and_keeps_metadata_fallback() {
        let mut spec = ProcessSpec {
            id: AdapterId::new("typed-idle-timeout"),
            image_or_root: ImageRef::new("worktree_idle"),
            cmd: vec![],
            env: BTreeMap::new(),
            cwd: None,
            binds: vec![],
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            idle_timeout_ms: Some(1_500),
            required_capabilities: Default::default(),
            trust_class: TrustClass::UntrustedAgent,
            metadata: BTreeMap::from([(
                SANDBOX_IDLE_TIMEOUT_METADATA_KEY.to_string(),
                "2500".to_string(),
            )]),
        };
        assert_eq!(persistent_idle_timeout_ms(&spec).unwrap(), Some(1_500));

        spec.idle_timeout_ms = None;
        assert_eq!(persistent_idle_timeout_ms(&spec).unwrap(), Some(2_500));

        spec.metadata.insert(
            SANDBOX_IDLE_TIMEOUT_METADATA_KEY.to_string(),
            "0".to_string(),
        );
        assert_eq!(persistent_idle_timeout_ms(&spec).unwrap(), None);

        spec.metadata.insert(
            SANDBOX_IDLE_TIMEOUT_METADATA_KEY.to_string(),
            "not-a-number".to_string(),
        );
        assert_eq!(persistent_idle_timeout_ms(&spec).unwrap(), None);

        spec.idle_timeout_ms = Some(0);
        let err = persistent_idle_timeout_ms(&spec).expect_err("typed zero must be rejected");
        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("idle_timeout_ms"));
                assert!(reason.contains("greater than zero"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn non_persistent_typed_idle_timeout_fails_closed_before_boot() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let spec = ProcessSpec {
            id: AdapterId::new("non-persistent-idle-timeout"),
            image_or_root: ImageRef::new("initramfs"),
            cmd: vec!["true".to_string()],
            env: BTreeMap::new(),
            cwd: None,
            binds: vec![],
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            idle_timeout_ms: Some(1_500),
            required_capabilities: Default::default(),
            trust_class: TrustClass::UntrustedAgent,
            metadata: BTreeMap::new(),
        };

        let err = adapter
            .spawn(spec)
            .await
            .expect_err("ephemeral CH path must not silently ignore typed idle timeout");
        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("idle_timeout_ms"));
                assert!(reason.contains("persistent"));
                assert!(reason.contains("refusing to silently ignore"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[test]
    fn persistent_bind_records_preserve_declared_read_only_binds() {
        let binds = vec![BindSpec {
            host_path: PathBuf::from("D:/models"),
            guest_path: PathBuf::from("/models"),
            mode: BindMode::ReadOnly,
        }];

        let records = persistent_bind_records_from_spec(&binds).expect("persistent binds");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].host_path, PathBuf::from("D:/models"));
        assert_eq!(records[0].guest_path, PathBuf::from("/models"));
        assert_eq!(records[0].guest_rel, "models");
        assert_eq!(records[0].mode, BindMode::ReadOnly);
    }

    #[test]
    fn persistent_bind_records_fail_closed_for_unsupported_modes() {
        for mode in [BindMode::ReadWrite, BindMode::NoExec] {
            let binds = vec![BindSpec {
                host_path: PathBuf::from("D:/models"),
                guest_path: PathBuf::from("/models"),
                mode,
            }];
            let err = persistent_bind_records_from_spec(&binds)
                .expect_err("unsupported persistent bind mode must fail closed");
            match err {
                SandboxAdapterError::SpawnFailed { reason, .. } => {
                    assert!(reason.contains("only supports ReadOnly binds"));
                    assert!(reason.contains("initramfs"));
                    assert!(reason.contains("write-back or noexec"));
                }
                other => panic!("expected typed SpawnFailed, got {other:?}"),
            }
        }
    }

    #[test]
    fn persistent_bind_records_reject_agent_owned_tmp_paths() {
        for guest_path in ["/tmp", "/tmp/hsk-tick", "/tmp/hsk-agent.err"] {
            let binds = vec![BindSpec {
                host_path: PathBuf::from("D:/models"),
                guest_path: PathBuf::from(guest_path),
                mode: BindMode::ReadOnly,
            }];
            let err = persistent_bind_records_from_spec(&binds)
                .expect_err("persistent binds must not clobber serial agent /tmp paths");
            match err {
                SandboxAdapterError::BindGuestPathInvalid {
                    guest_path: rejected,
                    reason,
                } => {
                    assert_eq!(rejected, PathBuf::from(guest_path));
                    assert!(reason.contains("must not target /tmp"));
                    assert!(reason.contains("serial agent owns /tmp/hsk-*"));
                    assert!(reason.contains("snapshot tick state"));
                }
                other => panic!("expected typed BindGuestPathInvalid, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn fs_bind_on_persistent_handle_fails_closed_after_boot() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let handle = ProcessHandle::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            None,
            "hsk-ch-persistent-bind-test",
        );
        let mut state = HandleState::default();
        state.persistent = Some(PersistentVm {
            api_socket: "/work/persistent-test/api.sock".to_string(),
            agent_socket: "/work/persistent-test/agent.sock".to_string(),
            serial_log: "/work/persistent-test/serial.log".to_string(),
            vm_root: "/work/persistent-test".to_string(),
        });
        adapter.handles.lock().unwrap().insert(handle.id, state);

        let err = adapter
            .fs_bind(
                &handle,
                PathBuf::from("D:/models"),
                PathBuf::from("/models"),
                BindMode::ReadOnly,
            )
            .await
            .expect_err("late persistent bind must not be silently accepted");

        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("fs_bind after boot is unsupported"));
                assert!(reason.contains("ProcessSpec.binds before spawn"));
                assert!(reason.contains("persistent initramfs"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[test]
    fn orphan_discovery_script_skips_current_process_owned_roots() {
        let script = discover_orphan_vm_dirs_script("/tmp/handshake sandbox", 4242);
        assert!(script.contains(VM_ROOT_OWNER_PID_FILE));
        assert!(script.contains("/persistent-*"));
        assert!(script.contains("/restore-*"));
        assert!(script.contains("cat \"$d/.hsk-owner-pid\""));
        assert!(script.contains("= \"4242\""));
        assert!(
            script.contains("then continue"),
            "current-process owned VM roots must be skipped, not reclaimed"
        );
    }

    #[test]
    fn owner_marker_uses_process_id_file_inside_vm_root() {
        let marker = format!("/tmp/vm-root/{VM_ROOT_OWNER_PID_FILE}");
        assert_eq!(marker, "/tmp/vm-root/.hsk-owner-pid");
        assert_eq!(VM_ROOT_OWNER_PID_FILE, ".hsk-owner-pid");
    }

    #[test]
    fn persistent_exec_lock_is_per_handle_not_global() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let first = Uuid::now_v7();
        let second = Uuid::now_v7();
        let first_a = adapter.persistent_exec_lock_for(first).expect("first lock");
        let first_b = adapter
            .persistent_exec_lock_for(first)
            .expect("same handle lock");
        let second_lock = adapter
            .persistent_exec_lock_for(second)
            .expect("second handle lock");

        assert!(
            Arc::ptr_eq(&first_a, &first_b),
            "same persistent handle must reuse one serial exec lock"
        );
        assert!(
            !Arc::ptr_eq(&first_a, &second_lock),
            "different persistent handles must not share a global exec lock"
        );
    }

    #[test]
    fn capabilities_are_honest_about_persistent_exec_without_warm_streaming() {
        let caps = CloudHypervisorAdapter::new_for_test().capabilities();
        assert!(caps.supports_snapshot);
        assert!(
            caps.supports_persistent_exec,
            "persistent exec is served by the serial guest channel"
        );
        assert!(
            !caps.supports_warm_agent,
            "warm-model RPC is not live until a model-serving guest agent exists"
        );
        assert!(
            !caps.supports_live_token_stream,
            "current sandbox streaming is post-hoc chunking, not live decode"
        );
    }

    #[test]
    fn warm_agent_status_is_machine_readable_and_fail_closed() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let status = adapter.warm_agent_status();
        assert_eq!(
            status.adapter_id,
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID)
        );
        assert!(status.snapshot_supported);
        assert!(status.persistent_exec_supported);
        assert!(!status.warm_agent_supported);
        assert!(!status.live_token_stream_supported);
        assert_eq!(
            status.required_transport,
            CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT
        );
        let reason = status
            .unavailable_reason
            .as_deref()
            .expect("unsupported status carries reason");
        assert!(reason.contains("serial-socket command channel"));
        assert!(reason.contains("model-serving"));
        assert!(reason.contains("virtio-vsock"));

        let encoded = serde_json::to_value(&status).expect("serialize warm status");
        assert_eq!(encoded["warm_agent_supported"], false);
        assert_eq!(encoded["live_token_stream_supported"], false);
        assert_eq!(
            encoded["required_transport"],
            CLOUD_HYPERVISOR_WARM_AGENT_REQUIRED_TRANSPORT
        );
    }

    #[test]
    fn persistent_boot_args_use_serial_agent_socket_without_vsock() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let args = adapter
            .persistent_boot_args(
                "/work/vm/ch.sock",
                "/work/vm/idle.cpio",
                "/work/vm/serial.log",
                "/work/vm/agent.sock",
                256,
            )
            .expect("persistent boot args");

        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--api-socket" && pair[1] == "/work/vm/ch.sock"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--serial" && pair[1] == "socket=/work/vm/agent.sock"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--console" && pair[1] == "file=/work/vm/serial.log"));
        assert!(args
            .windows(2)
            .any(|pair| pair[0] == "--cmdline" && pair[1] == "console=hvc0"));
        assert!(
            !args.iter().any(|arg| arg.contains("vsock")),
            "vsock remains the follow-on transport and must not be implied by the serial agent"
        );
        assert!(
            !args.iter().any(|arg| arg.contains("hsk.warm_agent")),
            "persistent init must not imply a warm-agent protocol until the guest serves it"
        );
    }

    #[test]
    fn serial_agent_bridge_enforces_frame_bound_before_buffering_forever() {
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("MAX_FRAME_BYTES = 1024 * 1024"));
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("base64.b64decode(sys.argv[4]"));
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("HSK-AGENT-READY"));
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("sock.sendall(payload)"));
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("line.startswith(wanted_ready)"));
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("len(payload) > MAX_FRAME_BYTES"));
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("len(buf) > MAX_FRAME_BYTES"));
        assert!(SERIAL_AGENT_BRIDGE_PY.contains("guest channel response exceeded max frame bytes"));
    }

    #[test]
    fn serial_agent_bridge_rejects_frames_too_large_for_argv_boundary() {
        let frame = "x".repeat(SERIAL_AGENT_BRIDGE_MAX_ARG_FRAME_BYTES + 1);
        let err = encode_serial_agent_bridge_frame_arg(&frame)
            .expect_err("oversized argv bridge frame must fail closed");
        assert!(format!("{err}").contains("argv bridge limit"));
    }

    #[tokio::test]
    async fn exec_on_persistent_handle_without_agent_socket_fails_closed() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let handle = ProcessHandle::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            None,
            "hsk-ch-persistent-test",
        );
        let mut state = HandleState::default();
        state.persistent = Some(PersistentVm {
            api_socket: "/work/persistent-test/api.sock".to_string(),
            agent_socket: String::new(),
            serial_log: "/work/persistent-test/serial.log".to_string(),
            vm_root: "/work/persistent-test".to_string(),
        });
        adapter.handles.lock().unwrap().insert(handle.id, state);

        let err = adapter
            .exec(
                &handle,
                Command {
                    argv: vec!["true".to_string()],
                    env_overlay: Default::default(),
                    stdin: None,
                    timeout_ms: Some(1),
                },
            )
            .await
            .expect_err("missing agent socket must fail closed");

        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("guest agent socket is missing"));
                assert!(reason.contains("refusing dirty fallback"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[test]
    fn ch_remote_args_shape() {
        let config = CloudHypervisorConfig::default();
        let args = ch_remote_args(&config, "/run/ch.sock", &["pause".to_string()]);
        assert_eq!(args[0], "-d");
        assert!(args.contains(&"--api-socket".to_string()));
        assert!(args.contains(&"/run/ch.sock".to_string()));
        assert_eq!(args.last().unwrap(), "pause");
    }

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
    fn resource_limit_memory_bytes_rounds_up_to_cloud_hypervisor_mib() {
        let default_mib = 256;
        assert_eq!(
            memory_mib_from_limits(&ResourceLimits::default(), default_mib),
            default_mib
        );
        assert_eq!(
            memory_mib_from_limits(
                &ResourceLimits {
                    memory_bytes: Some(1),
                    ..Default::default()
                },
                default_mib,
            ),
            1,
            "a sub-MiB reservation rounds up to CH's smallest MiB unit"
        );
        assert_eq!(
            memory_mib_from_limits(
                &ResourceLimits {
                    memory_bytes: Some(257 * 1024 * 1024 - 1),
                    ..Default::default()
                },
                default_mib,
            ),
            257,
            "CH memory size must be ceil(bytes/MiB), never rounded down"
        );
    }

    #[test]
    fn boot_args_use_process_spec_memory_limit_not_global_default() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let limits = ResourceLimits {
            memory_bytes: Some(768 * 1024 * 1024),
            ..Default::default()
        };
        let args = adapter
            .boot_args(
                "Y21k",
                "",
                "/work/initramfs.cpio",
                "/work/serial.log",
                memory_mib_from_limits(&limits, adapter.config.memory_mib),
            )
            .expect("valid CH argv");
        let memory_pos = args
            .iter()
            .position(|arg| arg == "--memory")
            .expect("CH argv includes --memory");
        assert_eq!(
            args.get(memory_pos + 1).map(String::as_str),
            Some("size=768M")
        );
    }

    #[test]
    fn boot_args_include_cloud_hypervisor_balloon_when_configured() {
        let mut adapter = CloudHypervisorAdapter::new_for_test();
        adapter.config.balloon = CloudHypervisorBalloonConfig::new(128, true, false);
        let args = adapter
            .boot_args("Y21k", "", "/work/initramfs.cpio", "/work/serial.log", 512)
            .expect("balloon fits inside guest memory");
        let balloon_pos = args
            .iter()
            .position(|arg| arg == "--balloon")
            .expect("CH argv includes --balloon");
        assert_eq!(
            args.get(balloon_pos + 1).map(String::as_str),
            Some("size=128M,deflate_on_oom=on,free_page_reporting=off")
        );
    }

    #[test]
    fn balloon_larger_than_guest_memory_fails_before_spawn() {
        let mut args = Vec::new();
        let err = append_balloon_args(
            &mut args,
            256,
            &CloudHypervisorBalloonConfig::new(512, true, true),
        )
        .expect_err("invalid balloon size must fail closed");
        assert!(args.is_empty(), "no partial --balloon argv on failure");
        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("balloon size 512M"));
                assert!(reason.contains("guest memory 256M"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[test]
    fn rate_limits_translate_to_cloud_hypervisor_token_bucket_args() {
        let limits = ResourceLimits {
            disk_read_bytes_per_sec: Some(1_000_000),
            disk_write_bytes_per_sec: Some(1_000_000),
            net_bandwidth_bytes_per_sec: Some(2_000_000),
            ..Default::default()
        };
        let args =
            cloud_hypervisor_rate_limiter_args(&limits).expect("symmetric limits are mappable");
        assert_eq!(
            args.disk.as_deref(),
            Some("bw_size=1000000,bw_one_time_burst=1000000,bw_refill_time=1000")
        );
        assert_eq!(
            args.net.as_deref(),
            Some("bw_size=2000000,bw_one_time_burst=2000000,bw_refill_time=1000")
        );
    }

    #[test]
    fn asymmetric_disk_rate_limits_fail_closed_before_argv_construction() {
        let err = cloud_hypervisor_rate_limiter_args(&ResourceLimits {
            disk_read_bytes_per_sec: Some(1_000_000),
            disk_write_bytes_per_sec: Some(2_000_000),
            ..Default::default()
        })
        .expect_err("CH disk has one shared token bucket, not directional limits");
        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("one shared bandwidth token bucket"));
                assert!(reason.contains("asymmetric"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[test]
    fn zero_rate_limit_fails_closed_instead_of_encoding_zero_bandwidth() {
        let err = cloud_hypervisor_rate_limiter_args(&ResourceLimits {
            net_bandwidth_bytes_per_sec: Some(0),
            ..Default::default()
        })
        .expect_err("zero is ambiguous and must not become a CH token bucket");
        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("network bandwidth limit"));
                assert!(reason.contains("greater than zero"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[test]
    fn disk_and_net_rate_limits_fail_closed_until_direct_initramfs_has_devices() {
        let limits = ResourceLimits {
            disk_read_bytes_per_sec: Some(1_000_000),
            disk_write_bytes_per_sec: Some(1_000_000),
            net_bandwidth_bytes_per_sec: Some(1_000_000),
            ..Default::default()
        };
        let err = validate_supported_resource_limits(&limits)
            .expect_err("mappable CH rate limits still need attachable devices");
        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("bw_size/bw_refill_time"));
                assert!(reason.contains("no virtio-block or virtio-net device"));
                assert!(reason.contains("per-device rate limits"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn oversized_persistent_exec_frame_does_not_kill_healthy_vm() {
        let adapter = CloudHypervisorAdapter::new_for_test();
        let handle = ProcessHandle::new(
            AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID),
            None,
            "persistent-local-frame-limit".to_string(),
        );
        let mut state = HandleState::default();
        state.persistent = Some(PersistentVm {
            api_socket: "/work/persistent-test/api.sock".to_string(),
            agent_socket: "/work/persistent-test/agent.sock".to_string(),
            serial_log: "/work/persistent-test/serial.log".to_string(),
            vm_root: "/work/persistent-test".to_string(),
        });
        adapter.handles.lock().unwrap().insert(handle.id, state);

        let err = adapter
            .exec(
                &handle,
                Command {
                    argv: vec!["cat".to_string()],
                    env_overlay: BTreeMap::new(),
                    stdin: Some(Bytes::from(vec![
                        b'x';
                        SERIAL_AGENT_BRIDGE_MAX_ARG_FRAME_BYTES + 1
                    ])),
                    timeout_ms: Some(1_000),
                },
            )
            .await
            .expect_err("oversized local bridge frame must fail before touching the VM");
        match err {
            SandboxAdapterError::SpawnFailed { reason, .. } => {
                assert!(reason.contains("argv bridge limit"));
            }
            other => panic!("expected typed SpawnFailed, got {other:?}"),
        }
        match adapter
            .status(&handle)
            .await
            .expect("status after local failure")
        {
            ProcessStatus::Running => {}
            other => panic!("local frame validation failure must not kill the VM, got {other:?}"),
        }
    }

    #[test]
    fn default_config_uses_proven_values_when_env_unset() {
        // Note: this asserts the compiled-in defaults, not env overrides.
        let config = CloudHypervisorConfig {
            distro: DEFAULT_DISTRO.to_string(),
            wsl_exe: PathBuf::from("wsl.exe"),
            ch_bin: DEFAULT_CH_BIN.to_string(),
            ch_remote_bin: DEFAULT_CH_REMOTE_BIN.to_string(),
            kernel: DEFAULT_KERNEL.to_string(),
            initramfs: DEFAULT_INITRAMFS.to_string(),
            busybox: DEFAULT_BUSYBOX.to_string(),
            work_dir: DEFAULT_WORK_DIR.to_string(),
            memory_mib: DEFAULT_MEMORY_MIB,
            vcpus: DEFAULT_VCPUS,
            command_timeout_ms: DEFAULT_COMMAND_TIMEOUT_MS,
            balloon: CloudHypervisorBalloonConfig::disabled(),
        };
        assert_eq!(config.memory_mib(), 256);
        assert_eq!(config.vcpus(), 1);
        assert_eq!(config.command_timeout_ms(), 60_000);
        assert_eq!(config.balloon().size_mib(), None);
    }
}
