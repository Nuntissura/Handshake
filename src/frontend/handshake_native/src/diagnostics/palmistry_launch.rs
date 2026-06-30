//! WP-KERNEL-012 MT-094 — Handshake LAUNCHES Palmistry + the startup IPC HANDSHAKE
//! (Master Spec v02.196 §6.13.3 "launched WITH Handshake at startup" + §6.13.2).
//!
//! This module is the Handshake-side half of the §6.13.3 lifecycle inversion: at startup Handshake
//! SPAWNS the sibling `palmistry` watcher process, hands it the watch inputs (this process's PID + the
//! diagnostic session id + the MT-081 ring backing-file path + a control-socket name), and completes a
//! BOUNDED startup handshake over the MT-089 control socket so both sides confirm the channel before the
//! app proceeds. On a clean Handshake exit it sends the explicit `Shutdown` control message so Palmistry
//! closes cleanly and records NO crash (the §6.13 clean-shutdown rule).
//!
//! # The four HARD properties (the MT-094 red-team controls)
//!
//! 1. **HBR-QUIET (AC-014-2).** The spawn is headless: `CREATE_NO_WINDOW` (no console flash), stdio
//!    redirected to null (never inherits / steals the console), and NO `SetForegroundWindow` / focus
//!    steal anywhere. This reuses the EXACT subprocess-spawn discipline the crate already uses for the
//!    MT-088 LSP server (`creation_flags(0x0800_0000)` in `code_editor/lsp_client.rs`). The product-wide
//!    focus ban (`tests/test_focus_audit_quiet.rs`, which scans every `src/**/*.rs`) covers this file.
//! 2. **NOT kill-on-job-close (AC-014-3).** Palmistry is spawned FREE-STANDING via a plain
//!    `std::process::Command::spawn`, which on Windows does NOT add the child to any Win32 Job Object.
//!    We deliberately add NO job membership (see [`SPAWN_NOT_KILL_ON_JOB_CLOSE`]) so a kill-on-job-close
//!    job can never terminate the watcher at the instant of the parent's death — the exact moment it
//!    must survive to record it. (The watcher's survives-parent-death proof is `palmistry`'s AC-009-4.)
//! 3. **Bounded handshake (AC-014-5).** The startup handshake runs on a worker thread and the caller
//!    waits on it with an explicit deadline ([`HANDSHAKE_OVERALL_DEADLINE`]). If Palmistry does not ack
//!    in time, Handshake LOGS + continues degraded — it NEVER hangs startup waiting on the watcher
//!    (which would reintroduce the very startup stall this whole substrate exists to kill).
//! 4. **Graceful degradation (AC-014-5).** The watcher is SUPPLEMENTARY (§5.8.6): if `palmistry.exe` is
//!    missing or the spawn fails, [`launch_palmistry_or_degrade`] logs a warning + records a typed
//!    internal_diagnostics event and returns `None` — Handshake starts and runs anyway.
//!
//! # Portability
//!
//! The `palmistry` binary is resolved RELATIVE to the running Handshake exe
//! (`std::env::current_exe()` -> sibling `palmistry.exe`) — the installer ships both side-by-side
//! ([`crate::installer`] self-check resolves bundled assets the same way), so there is NO hardcoded
//! path (GLOBAL-PORTABILITY). An optional [`ENV_PALMISTRY_EXE`] override lets a test / an operator pin
//! an explicit binary (the dev tree builds the two crates into different target dirs, so they are not
//! side-by-side there).

use std::collections::VecDeque;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{
    GetLastError, ERROR_BROKEN_PIPE, ERROR_PIPE_BUSY, ERROR_PIPE_NOT_CONNECTED,
};
#[cfg(windows)]
use windows_sys::Win32::System::Pipes::{PeekNamedPipe, WaitNamedPipeW};

use handshake_diag_ring::{DiagEventCode, DiagPhase, DiagSeverity};
#[cfg(not(windows))]
use interprocess::local_socket::traits::Stream as _;
use interprocess::local_socket::Stream as LocalStream;
#[cfg(not(windows))]
use interprocess::local_socket::{GenericNamespaced, ToNsName};
#[cfg(windows)]
use interprocess::os::windows::named_pipe::{
    local_socket::Stream as WindowsLocalSocketStream, pipe_mode::Bytes, DuplexPipeStream,
};
#[cfg(windows)]
use interprocess::ConnectWaitMode;
use serde::{Deserialize, Serialize};

use super::DiagSession;

/// `CREATE_NO_WINDOW` (the exact value the MT-088 LSP spawn uses). A console child spawned by the GUI
/// shell would otherwise flash a console window — this keeps the watcher headless (HBR-QUIET / AC-014-2).
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The base name of the sibling watcher binary, resolved next to the running Handshake exe.
#[cfg(windows)]
const PALMISTRY_EXE_NAME: &str = "palmistry.exe";
#[cfg(not(windows))]
const PALMISTRY_EXE_NAME: &str = "palmistry";

/// Optional override for the watcher binary path. PRIMARY use: tests + the dev tree, where Handshake and
/// Palmistry build into separate target dirs and are therefore NOT side-by-side (the installer layout).
/// When unset, the binary is resolved relative to [`std::env::current_exe`] (the portable default).
pub const ENV_PALMISTRY_EXE: &str = "HANDSHAKE_PALMISTRY_EXE";

/// HARD launcher-side half of the §6.13.3 lifecycle-inversion contract (the counterpart of
/// `palmistry::lifecycle::JOB_OBJECT_CONTRACT`): this launcher spawns Palmistry as a FREE-STANDING
/// process via a plain `std::process::Command::spawn` and DELIBERATELY adds it to NO Win32 Job Object,
/// so a kill-on-job-close job can never terminate the watcher at the instant of the parent's death — the
/// exact moment it must survive to record it. Greppable so a reviewer (AC-014-3) sees the commitment.
pub const SPAWN_NOT_KILL_ON_JOB_CLOSE: &str =
    "palmistry is spawned free-standing (plain Command::spawn, no Win32 Job Object membership) so it \
     survives parent death to record it (Master Spec 6.13.3).";

/// How long the handshake worker retries CONNECTING to Palmistry's control socket. On Windows a named
/// pipe can exist while its single server instance is temporarily busy, so this deadline covers both
/// "not bound yet" and bounded `ERROR_PIPE_BUSY` waits.
const HANDSHAKE_CONNECT_DEADLINE: Duration = Duration::from_secs(8);
/// Backoff between control-socket connect attempts (bounds the retry + log rate; never busy-spins).
const CONNECT_RETRY_INTERVAL: Duration = Duration::from_millis(25);
/// The HARD bound the CALLER waits for the whole handshake (connect + Hello + Ack) on the worker thread.
/// MUST be >= [`HANDSHAKE_CONNECT_DEADLINE`] so a slow-but-succeeding connect is not pre-empted. If this
/// elapses, startup continues degraded — Handshake never hangs on the watcher (AC-014-5).
const HANDSHAKE_OVERALL_DEADLINE: Duration = Duration::from_secs(9);
/// Bound for the post-connect `Ack` line. Windows named-pipe local sockets do not support OS receive
/// timeouts, so replies are read by a caller-owned bounded poll loop that drops the real connection on
/// timeout instead of stranding a blocking reader thread.
const HANDSHAKE_ACK_READ_DEADLINE: Duration = Duration::from_millis(900);
/// Poll cadence for bounded control replies on transports that cannot support native receive timeouts.
const CONTROL_REPLY_POLL_INTERVAL: Duration = Duration::from_millis(5);
const CONTROL_REPLY_MAX_BYTES: usize = 16 * 1024;
/// Bounded wait for Palmistry to exit after an explicit `Shutdown` on a CLEAN app exit. A clean shutdown
/// is prompt (Palmistry's lifecycle breaks immediately when the parent is still alive); the generous
/// bound only covers scheduling jitter before the kill backstop.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(5);
/// Shorter bound used from `Drop` (a backstop path) so dropping the app cannot block for long.
const DROP_WAIT: Duration = Duration::from_secs(2);
/// Bound for a best-effort reconnect to send `Shutdown` when the live control connection was never
/// established (the handshake was unconfirmed but the child is running).
const SHUTDOWN_CONNECT_DEADLINE: Duration = Duration::from_millis(500);
const MAX_PENDING_CHILD_WATCH_COMMANDS: usize = 256;

#[derive(Debug, Clone)]
enum PalmistryChildWatchCommand {
    RegisterFile {
        pid: u32,
        child_session_id: u64,
        liveness_path: PathBuf,
        threshold: Duration,
    },
    Deregister {
        pid: u32,
        child_session_id: u64,
    },
}

static PENDING_CHILD_WATCH_COMMANDS: OnceLock<Mutex<VecDeque<PalmistryChildWatchCommand>>> =
    OnceLock::new();

fn pending_child_watch_commands() -> &'static Mutex<VecDeque<PalmistryChildWatchCommand>> {
    PENDING_CHILD_WATCH_COMMANDS.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Derive the control-socket name Handshake passes to Palmistry (`--control-socket`). Palmistry binds
/// whatever name it is given (MT-089 `cli.rs`), so the launcher OWNS the name; deriving it from the
/// session id keeps it unique per session + filename/namespace-safe (CX-109A).
pub fn control_socket_name(session_id: &str) -> String {
    let safe: String = session_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("handshake-palmistry-{safe}")
}

fn enqueue_child_watch_command(command: PalmistryChildWatchCommand) {
    let Ok(mut guard) = pending_child_watch_commands().lock() else {
        return;
    };
    if guard.len() >= MAX_PENDING_CHILD_WATCH_COMMANDS {
        guard.pop_front();
    }
    guard.push_back(command);
}

/// Queue a child-process liveness watch for the app-owned Palmistry handle to send over its held control
/// socket. Child-spawning subsystems call this from whatever thread they own; `HandshakeApp::update`
/// drains it through [`drain_palmistry_child_watch_commands`].
pub fn enqueue_palmistry_child_liveness_file(
    pid: u32,
    child_session_id: u64,
    liveness_path: impl Into<PathBuf>,
    threshold: Duration,
) {
    enqueue_child_watch_command(PalmistryChildWatchCommand::RegisterFile {
        pid,
        child_session_id,
        liveness_path: liveness_path.into(),
        threshold,
    });
}

/// Queue removal of a child-process liveness watch. This uses the same app-owned drain path as
/// registration, so Palmistry's single held startup socket remains the only control channel.
pub fn enqueue_palmistry_child_deregister(pid: u32, child_session_id: u64) {
    enqueue_child_watch_command(PalmistryChildWatchCommand::Deregister {
        pid,
        child_session_id,
    });
}

/// Drain queued child-watch commands over the confirmed Palmistry control connection. Returns the number
/// of commands attempted. Failures are logged and not requeued: the child operation remains bounded by
/// its in-app watchdog, and Palmistry is supplementary.
pub fn drain_palmistry_child_watch_commands(handle: &mut PalmistryHandle) -> usize {
    let commands = {
        let Ok(mut guard) = pending_child_watch_commands().lock() else {
            return 0;
        };
        guard.drain(..).collect::<Vec<_>>()
    };
    let count = commands.len();
    for command in commands {
        let result = match command {
            PalmistryChildWatchCommand::RegisterFile {
                pid,
                child_session_id,
                liveness_path,
                threshold,
            } => handle.register_child_liveness_file(
                pid,
                child_session_id,
                &liveness_path,
                threshold,
            ),
            PalmistryChildWatchCommand::Deregister {
                pid,
                child_session_id,
            } => handle.deregister_child(pid, child_session_id),
        };
        if let Err(err) = result {
            tracing::warn!(%err, "failed to drain Palmistry child-watch command");
        }
    }
    count
}

// ----------------------------------------------------------------------------------------------------
// Control-protocol mirror (the MT-089 `palmistry::control` wire shape).
//
// handshake-native and the `palmistry` crate share NO dependency edge (verified — they are separate
// standalone workspaces), so the launcher cannot import `palmistry::control::ControlMessage`. Instead it
// speaks the EXACT same newline-delimited tagged-JSON wire form. `#[serde(tag = "type")]` reproduces the
// `{"type":"HandshakeHello","parent_pid":N,"session_id":"..."}` / `{"type":"Shutdown"}` shape that
// `palmistry`'s own `tagged_json_shape_is_explicit` test pins — so the two sides stay byte-compatible.
// ----------------------------------------------------------------------------------------------------

/// The control messages THIS launcher sends (a strict subset of `palmistry::control::ControlMessage`).
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum LauncherControlMessage {
    /// Handshake's startup handshake: announces the parent pid + session so Palmistry can ack it is
    /// watching the right process (MT-094 produces this; MT-089 acks it).
    HandshakeHello { parent_pid: u32, session_id: String },
    /// Register a spawned child process for Palmistry's passive child-stall watch (MT-106).
    RegisterChild {
        pid: u32,
        child_session_id: u64,
        liveness_kind: LauncherChildLivenessKind,
        liveness_ref: PathBuf,
        threshold_ms: u64,
    },
    /// Remove a child process from Palmistry's passive child-stall watch.
    DeregisterChild { pid: u32, child_session_id: u64 },
    /// The one deliberate exit command: Palmistry shuts down promptly + cleanly, recording NO crash.
    Shutdown,
}

#[derive(Debug, Serialize)]
enum LauncherChildLivenessKind {
    FileCounter,
}

/// The replies Palmistry sends (`palmistry::control::ControlReply`): `Ack` to Hello/Shutdown, `Pong` to
/// a Ping (the launcher never sends Ping, but the variant is modeled so a stray Pong is a clear error).
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum LauncherControlReply {
    Pong,
    Ack,
    Error { code: LauncherControlErrorCode },
}

#[derive(Debug, Deserialize)]
enum LauncherControlErrorCode {
    ChildRegisterFailed,
    ChildNotRegistered,
}

/// Serialize a control message to one newline-terminated JSON line (the MT-089 frame) and flush it.
fn write_message<W: Write>(w: &mut W, msg: &LauncherControlMessage) -> io::Result<()> {
    let mut line = serde_json::to_string(msg).map_err(io::Error::other)?;
    line.push('\n');
    w.write_all(line.as_bytes())?;
    w.flush()
}

#[cfg(windows)]
fn windows_named_pipe_path(socket_name: &str) -> String {
    format!(r"\\.\pipe\{socket_name}")
}

#[cfg(windows)]
fn wait_if_windows_pipe_busy(socket_name: &str, err: &io::Error, remaining: Duration) {
    if err.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) || remaining.is_zero() {
        return;
    }

    let wait_ms = remaining.min(Duration::from_millis(250)).as_millis();
    let wait_ms = u32::try_from(wait_ms).unwrap_or(u32::MAX).max(1);
    let pipe_path = windows_named_pipe_path(socket_name);
    let wide: Vec<u16> = OsStr::new(&pipe_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Best-effort fairness wait for interprocess' GenericNamespaced Windows named pipe. Failure just
    // falls back to the bounded connect retry loop below.
    unsafe {
        let _ = WaitNamedPipeW(wide.as_ptr(), wait_ms);
    }
}

#[cfg(windows)]
fn connect_control_once(socket_name: &str, wait: Duration) -> io::Result<LocalStream> {
    let pipe_path = windows_named_pipe_path(socket_name);
    let pipe = DuplexPipeStream::<Bytes>::connect_by_path_with_wait_mode(
        pipe_path.as_str(),
        ConnectWaitMode::Timeout(wait),
    )?;
    Ok(LocalStream::from(WindowsLocalSocketStream::from(pipe)))
}

#[cfg(not(windows))]
fn connect_control_once(socket_name: &str, _: Duration) -> io::Result<LocalStream> {
    let name = socket_name.to_ns_name::<GenericNamespaced>()?;
    LocalStream::connect(name)
}

/// Connect to Palmistry's control socket, retrying with a bounded backoff until it binds or `deadline`
/// elapses. Palmistry is launched WITH Handshake (this MT) and may not have bound the socket at the
/// instant of the spawn, so a transient connect failure is retried — but never unboundedly.
fn connect_control(socket_name: &str, deadline: Duration) -> io::Result<LocalStream> {
    let start = Instant::now();
    let mut attempts = 0u64;
    loop {
        attempts += 1;
        let remaining = deadline.saturating_sub(start.elapsed());
        match connect_control_once(socket_name, remaining.min(Duration::from_millis(250))) {
            Ok(stream) => return Ok(stream),
            Err(err) => {
                if start.elapsed() >= deadline {
                    return Err(io::Error::new(
                        err.kind(),
                        format!(
                            "could not connect to palmistry control socket '{socket_name}' within \
                             {deadline:?} after {attempts} attempts ({:?} elapsed): {err}",
                            start.elapsed()
                        ),
                    ));
                }
                #[cfg(windows)]
                wait_if_windows_pipe_busy(
                    socket_name,
                    &err,
                    deadline.saturating_sub(start.elapsed()),
                );
                std::thread::sleep(CONNECT_RETRY_INTERVAL);
            }
        }
    }
}

#[cfg(windows)]
enum ControlPipeReadiness {
    Data,
    Empty,
    Closed,
}

#[cfg(windows)]
fn control_pipe_readiness(stream: &LocalStream) -> io::Result<ControlPipeReadiness> {
    let raw_handle = match stream {
        LocalStream::NamedPipe(stream) => stream.inner().as_raw_handle(),
    };
    let mut total_available = 0u32;
    let ok = unsafe {
        PeekNamedPipe(
            raw_handle,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut total_available,
            std::ptr::null_mut(),
        )
    };
    if ok != 0 {
        return Ok(if total_available == 0 {
            ControlPipeReadiness::Empty
        } else {
            ControlPipeReadiness::Data
        });
    }

    let code = unsafe { GetLastError() };
    if code == ERROR_BROKEN_PIPE || code == ERROR_PIPE_NOT_CONNECTED {
        Ok(ControlPipeReadiness::Closed)
    } else {
        Err(io::Error::from_raw_os_error(code as i32))
    }
}

fn read_control_line_bounded(
    mut reader: BufReader<LocalStream>,
    deadline: Duration,
    context: &str,
) -> io::Result<(BufReader<LocalStream>, String)> {
    #[cfg(not(windows))]
    reader.get_ref().set_nonblocking(true).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("{context}: failed to enable bounded nonblocking control read: {err}"),
        )
    })?;
    let start = Instant::now();
    let mut bytes = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    loop {
        if start.elapsed() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{context}: no newline-delimited reply within {deadline:?}"),
            ));
        }
        if bytes.len() > CONTROL_REPLY_MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{context}: reply exceeded {CONTROL_REPLY_MAX_BYTES} bytes before newline"),
            ));
        }
        #[cfg(windows)]
        if reader.buffer().is_empty() {
            match control_pipe_readiness(reader.get_ref())? {
                ControlPipeReadiness::Data => {}
                ControlPipeReadiness::Empty => {
                    std::thread::sleep(CONTROL_REPLY_POLL_INTERVAL);
                    continue;
                }
                ControlPipeReadiness::Closed => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        format!("{context}: peer closed before a newline-delimited reply"),
                    ));
                }
            }
        }
        match reader.read(&mut byte) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("{context}: peer closed before a newline-delimited reply"),
                ));
            }
            Ok(n) => {
                bytes.extend_from_slice(&byte[..n]);
                if byte[0] == b'\n' {
                    #[cfg(not(windows))]
                    reader.get_ref().set_nonblocking(false).map_err(|err| {
                        io::Error::new(
                            err.kind(),
                            format!("{context}: failed to restore blocking control read: {err}"),
                        )
                    })?;
                    let line = String::from_utf8(bytes)
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
                    return Ok((reader, line));
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                std::thread::sleep(CONTROL_REPLY_POLL_INTERVAL);
            }
            Err(err) => return Err(err),
        }
    }
}

/// The full startup handshake, run on the worker thread: connect (bounded), send `HandshakeHello`, read
/// the `Ack`. Returns the LIVE buffered connection on success so the SAME connection carries the later
/// `Shutdown` (Palmistry's `serve_connection` reads multiple messages on one accepted connection).
fn perform_handshake(
    socket_name: String,
    parent_pid: u32,
    session_id: String,
    connect_deadline: Duration,
) -> io::Result<BufReader<LocalStream>> {
    let stream = connect_control(&socket_name, connect_deadline).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("palmistry startup handshake connect failed: {err}"),
        )
    })?;
    let mut reader = BufReader::new(stream);
    write_message(
        reader.get_mut(),
        &LauncherControlMessage::HandshakeHello {
            parent_pid,
            session_id,
        },
    )
    .map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("palmistry startup handshake write failed: {err}"),
        )
    })?;
    let (reader, line) = read_control_line_bounded(
        reader,
        HANDSHAKE_ACK_READ_DEADLINE,
        "palmistry startup handshake",
    )
    .map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("palmistry startup handshake ack read failed: {err}"),
        )
    })?;
    let reply: LauncherControlReply = serde_json::from_str(line.trim_end_matches(['\n', '\r']))
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("palmistry startup handshake ack decode failed: {err}"),
            )
        })?;
    match reply {
        LauncherControlReply::Ack => Ok(reader),
        LauncherControlReply::Pong => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "palmistry replied Pong to HandshakeHello (expected Ack)",
        )),
        LauncherControlReply::Error { code } => Err(io::Error::other(format!(
            "palmistry replied Error to HandshakeHello (expected Ack): {code:?}"
        ))),
    }
}

/// Record a typed internal_diagnostics marker for a launcher lifecycle event (handshake / shutdown /
/// degrade). All-typed integer payload (no text) — the §5.8.3 allowlist stays structural.
fn record_marker(code: DiagEventCode, phase: DiagPhase, severity: DiagSeverity, counter_a: u64) {
    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    crate::diagnostics::record_with(
        code, phase, severity, /* thread_id */ 0, /* sequence_id */ 0, counter_a,
        /* counter_b */ 0, /* metric_micros */ 0, now_nanos,
    );
}

/// The outcome of reaping the watcher on shutdown.
#[derive(Debug)]
pub enum ShutdownOutcome {
    /// Palmistry received `Shutdown` and exited within the bounded window (the clean-shutdown path).
    ExitedCleanly(ExitStatus),
    /// Palmistry did not exit within the bounded window and was killed as a backstop (no orphan leak).
    Killed,
    /// Shutdown was already performed (idempotent).
    AlreadyDone,
}

/// A handle to the spawned Palmistry watcher: the child process plus (when the startup handshake acked)
/// the live control connection used to send `Shutdown` on a clean exit. Dropping the handle reaps the
/// child (best-effort `Shutdown` then kill backstop) so a watcher never orphans.
pub struct PalmistryHandle {
    child: Child,
    /// The live control connection, `Some` once the startup handshake acked (held for the later
    /// `Shutdown`), `None` if the handshake was unconfirmed (the watcher still runs; shutdown reconnects).
    control: Option<BufReader<LocalStream>>,
    handshake_acked: bool,
    handshake_error: Option<String>,
    socket_name: String,
    shutdown_done: bool,
}

impl PalmistryHandle {
    /// Whether the startup IPC handshake completed (Palmistry acked `HandshakeHello`). `true` is the
    /// AC-014-1 proof that a real handshake crossed the control socket.
    pub fn handshake_acked(&self) -> bool {
        self.handshake_acked
    }

    /// Startup handshake failure reason when the watcher spawned but degraded before ACK.
    pub fn handshake_error(&self) -> Option<&str> {
        self.handshake_error.as_deref()
    }

    /// The OS pid of the spawned watcher (the AC-014-1 proof that a real child process exists).
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }

    /// The control-socket name Palmistry bound (for logging / a reconnect on shutdown).
    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    /// Register a child process with Palmistry's MT-106 passive stall watcher. This uses the SAME held
    /// startup control connection, because Palmistry serves one accepted connection at a time and the
    /// launcher intentionally keeps that connection open for later Shutdown.
    pub fn register_child_liveness_file(
        &mut self,
        pid: u32,
        child_session_id: u64,
        liveness_path: &Path,
        threshold: Duration,
    ) -> io::Result<()> {
        self.send_control_expect_ack(
            &LauncherControlMessage::RegisterChild {
                pid,
                child_session_id,
                liveness_kind: LauncherChildLivenessKind::FileCounter,
                liveness_ref: liveness_path.to_path_buf(),
                threshold_ms: threshold.as_millis() as u64,
            },
            "palmistry register child",
        )
    }

    /// Deregister a child process from Palmistry's MT-106 passive stall watcher.
    pub fn deregister_child(&mut self, pid: u32, child_session_id: u64) -> io::Result<()> {
        self.send_control_expect_ack(
            &LauncherControlMessage::DeregisterChild {
                pid,
                child_session_id,
            },
            "palmistry deregister child",
        )
    }

    fn send_control_expect_ack(
        &mut self,
        msg: &LauncherControlMessage,
        context: &str,
    ) -> io::Result<()> {
        let Some(mut reader) = self.control.take() else {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                format!("{context}: no confirmed palmistry control connection"),
            ));
        };
        write_message(reader.get_mut(), msg)?;
        let (reader, line) =
            match read_control_line_bounded(reader, HANDSHAKE_ACK_READ_DEADLINE, context) {
                Ok(result) => result,
                Err(err) => {
                    self.control = None;
                    return Err(err);
                }
            };
        self.control = Some(reader);
        let reply: LauncherControlReply =
            serde_json::from_str(line.trim_end_matches(['\n', '\r'])).map_err(io::Error::other)?;
        match reply {
            LauncherControlReply::Ack => Ok(()),
            LauncherControlReply::Pong => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{context}: palmistry replied Pong (expected Ack)"),
            )),
            LauncherControlReply::Error { code } => Err(io::Error::other(format!(
                "{context}: palmistry returned {code:?}"
            ))),
        }
    }

    /// Send the explicit `Shutdown` control message (§6.13.3 "closes only on explicit command") so
    /// Palmistry exits cleanly + records NO crash, then wait (bounded by `wait`) for it to exit, killing
    /// it as a backstop if it overruns. Idempotent.
    pub fn request_shutdown_and_wait(&mut self, wait: Duration) -> ShutdownOutcome {
        if self.shutdown_done {
            return ShutdownOutcome::AlreadyDone;
        }
        self.shutdown_done = true;

        if let Some(reader) = self.control.as_mut() {
            if let Err(err) = write_message(reader.get_mut(), &LauncherControlMessage::Shutdown) {
                tracing::warn!(
                    %err,
                    "failed to send Shutdown to palmistry over the held control connection; reaping by \
                     wait/kill"
                );
            }
        } else if let Ok(mut stream) = connect_control(&self.socket_name, SHUTDOWN_CONNECT_DEADLINE)
        {
            // No live connection (handshake was unconfirmed): best-effort reconnect to send Shutdown.
            let _ = write_message(&mut stream, &LauncherControlMessage::Shutdown);
        }

        record_marker(
            DiagEventCode::Shutdown,
            DiagPhase::End,
            DiagSeverity::Info,
            self.child.id() as u64,
        );

        match wait_for_exit(&mut self.child, wait) {
            Some(status) => ShutdownOutcome::ExitedCleanly(status),
            None => {
                tracing::warn!(
                    child_pid = self.child.id(),
                    "palmistry did not exit within the bounded shutdown window; killing (backstop)"
                );
                let _ = self.child.kill();
                let _ = wait_for_exit(&mut self.child, Duration::from_secs(1));
                ShutdownOutcome::Killed
            }
        }
    }

    /// Consume the handle: send `Shutdown` and reap with the default bound. The clean-shutdown path
    /// `HandshakeApp::on_exit` calls (AC-014-4).
    pub fn shutdown(mut self) -> ShutdownOutcome {
        self.request_shutdown_and_wait(SHUTDOWN_WAIT)
    }
}

impl Drop for PalmistryHandle {
    /// Backstop: if a clean `shutdown` was not already performed (e.g. the app dropped without
    /// `on_exit`), send `Shutdown` and reap with a short bound so a watcher never orphans.
    fn drop(&mut self) {
        if !self.shutdown_done {
            let _ = self.request_shutdown_and_wait(DROP_WAIT);
        }
    }
}

/// Bounded wait for a child to exit: poll `try_wait` until it exits or `timeout` elapses. NEVER an
/// unbounded `Child::wait`, so a stuck child cannot hang the caller (the MANDATORY MT-092/094 bound).
fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
}

/// Resolve the `palmistry` binary path. PORTABLE (GLOBAL-PORTABILITY): the [`ENV_PALMISTRY_EXE`] override
/// wins (tests / explicit operator config); otherwise the binary is the sibling of the running Handshake
/// exe (`current_exe()` parent dir) — the installer's side-by-side two-binary layout (AC-014-6). No
/// hardcoded path. Returns a `NotFound` error if the binary is absent (the caller degrades — AC-014-5).
pub fn resolve_palmistry_exe() -> io::Result<PathBuf> {
    if let Some(raw) = std::env::var_os(ENV_PALMISTRY_EXE) {
        let path = PathBuf::from(raw);
        if path.is_file() {
            return Ok(path);
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "{ENV_PALMISTRY_EXE} is set but points at a missing file: {}",
                path.display()
            ),
        ));
    }
    let exe = std::env::current_exe()?;
    let dir = exe.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "current_exe() has no parent directory to resolve the palmistry sibling against",
        )
    })?;
    let candidate = dir.join(PALMISTRY_EXE_NAME);
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "palmistry binary not found beside the running exe at {}",
                candidate.display()
            ),
        ))
    }
}

/// Spawn `palmistry.exe` (resolved relative to the running exe) and complete the bounded startup
/// handshake. The production entrypoint `main()` reaches this via [`launch_palmistry_or_degrade`].
/// Returns `Err` only when the SPAWN itself fails (missing/unlaunchable binary) — a handshake that does
/// not ack still returns `Ok` (the watcher is running; the handshake is just unconfirmed).
pub fn launch_palmistry(
    session: &DiagSession,
    ring_path: &Path,
    control_socket: &str,
) -> io::Result<PalmistryHandle> {
    let exe = resolve_palmistry_exe()?;
    launch_palmistry_at(&exe, session, ring_path, control_socket)
}

/// Spawn an EXPLICIT `palmistry` binary (the test entrypoint — the dev tree builds the two crates into
/// separate target dirs, so the binary is named directly) and complete the bounded startup handshake.
/// Same contract as [`launch_palmistry`]: `Err` only on a spawn failure.
pub fn launch_palmistry_at(
    exe: &Path,
    session: &DiagSession,
    ring_path: &Path,
    control_socket: &str,
) -> io::Result<PalmistryHandle> {
    let parent_pid = std::process::id();

    // The MT-089 inputs, passed as CLI args (which OVERRIDE env in palmistry's intake) so the launcher
    // never mutates its OWN process environment. Spawn FREE-STANDING (no job — SPAWN_NOT_KILL_ON_JOB_CLOSE)
    // and HEADLESS/QUIET: CREATE_NO_WINDOW + null stdio (never inherits/steals the console; HBR-QUIET).
    let mut command = Command::new(exe);
    command
        .arg("--parent-pid")
        .arg(parent_pid.to_string())
        .arg("--session-id")
        .arg(&session.session_id)
        .arg("--ring-path")
        .arg(ring_path)
        .arg("--control-socket")
        .arg(control_socket)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn()?;
    tracing::info!(
        child_pid = child.id(),
        parent_pid,
        session_id = %session.session_id,
        ring_path = %ring_path.display(),
        control_socket,
        "palmistry watcher spawned (Tier 3, §6.13.3 launched-with-Handshake); starting bounded handshake"
    );

    // BOUNDED handshake (AC-014-5): run connect+Hello+Ack on a worker thread and wait on it with an
    // explicit deadline so a slow/absent ack NEVER hangs startup. The worker returns the LIVE connection
    // so the SAME connection carries the later Shutdown.
    let (tx, rx) = mpsc::channel();
    let worker_socket = control_socket.to_string();
    let worker_session = session.session_id.clone();
    if let Err(spawn_err) = std::thread::Builder::new()
        .name("palmistry-handshake".to_string())
        .spawn(move || {
            let result = perform_handshake(
                worker_socket,
                parent_pid,
                worker_session,
                HANDSHAKE_CONNECT_DEADLINE,
            );
            let _ = tx.send(result);
        })
    {
        // The palmistry child is ALREADY spawned; if we cannot even start the handshake worker thread
        // (e.g. resource/thread exhaustion) we must REAP the child before returning Err — otherwise it
        // orphans with no handle to reap it and, when this still-alive parent later exits without a
        // Shutdown, the watcher records a FALSE abnormal-parent-exit. Reap, then propagate the error.
        let _ = child.kill();
        let _ = wait_for_exit(&mut child, Duration::from_secs(1));
        return Err(spawn_err);
    }

    let (control, handshake_acked, handshake_error) = match rx
        .recv_timeout(HANDSHAKE_OVERALL_DEADLINE)
    {
        Ok(Ok(reader)) => {
            tracing::info!(
                control_socket,
                "palmistry startup handshake acked (Tier 3 confirmed watching, §6.13.3)"
            );
            record_marker(
                DiagEventCode::PalmistryHandshake,
                DiagPhase::Start,
                DiagSeverity::Info,
                parent_pid as u64,
            );
            (Some(reader), true, None)
        }
        Ok(Err(err)) => {
            tracing::warn!(
                %err,
                "palmistry startup handshake FAILED; watcher is spawned but unconfirmed (degraded — \
                 startup not blocked)"
            );
            (None, false, Some(err.to_string()))
        }
        Err(_) => {
            tracing::warn!(
                deadline = ?HANDSHAKE_OVERALL_DEADLINE,
                "palmistry startup handshake TIMED OUT; watcher is spawned but unconfirmed (degraded — \
                 startup not blocked)"
            );
            (
                None,
                false,
                Some(format!(
                    "palmistry startup handshake timed out after {HANDSHAKE_OVERALL_DEADLINE:?}"
                )),
            )
        }
    };

    Ok(PalmistryHandle {
        child,
        control,
        handshake_acked,
        handshake_error,
        socket_name: control_socket.to_string(),
        shutdown_done: false,
    })
}

/// The production startup entrypoint `main()` calls: launch the watcher and, on ANY failure, degrade
/// GRACEFULLY (§5.8.6) — log a warning + record a typed internal_diagnostics event + return `None` so
/// Handshake starts and runs WITHOUT the watcher. The watcher is supplementary; it must never block or
/// crash startup (AC-014-5 / RISK-014-5).
pub fn launch_palmistry_or_degrade(
    session: &DiagSession,
    control_socket: &str,
) -> Option<PalmistryHandle> {
    match launch_palmistry(session, &session.ring_path, control_socket) {
        Ok(handle) => {
            tracing::info!(
                child_pid = handle.child_id(),
                handshake_acked = handle.handshake_acked(),
                control_socket,
                "palmistry watcher launched (Tier 3 external watcher up, §6.13.3)"
            );
            Some(handle)
        }
        Err(err) => {
            tracing::warn!(
                %err,
                "palmistry watcher could not be launched; Handshake continues WITHOUT the external \
                 watcher (graceful degradation, §5.8.6)"
            );
            // A typed internal_diagnostics event so the absence of the Tier-3 watcher is observable on the
            // ring + the in-app panel (a BackendUnreachable-style degradation marker, but for the watcher).
            record_marker(
                DiagEventCode::PalmistryHandshake,
                DiagPhase::Degraded,
                DiagSeverity::Warn,
                0,
            );
            None
        }
    }
}

// ----------------------------------------------------------------------------------------------------
// Preinstalled-session handoff (main() -> HandshakeApp::new).
//
// MT-094 creates the MT-081 ring + launches Palmistry in `main()` BEFORE `eframe::run_native` (so the
// whole kittest suite, which builds `HandshakeApp` directly, never spawns a palmistry child — the
// anti-leak rule). `HandshakeApp::new` then REUSES that already-created ring via this process-global slot
// instead of creating a SECOND ring. The kittest path (no `main()`) leaves the slot empty and `new`
// creates its own ring exactly as before.
// ----------------------------------------------------------------------------------------------------

static PREINSTALLED_DIAG_SESSION: Mutex<Option<DiagSession>> = Mutex::new(None);

/// Record the [`DiagSession`] `main()` already created + installed, so [`HandshakeApp::new`] reuses it
/// instead of creating a second ring.
///
/// [`HandshakeApp::new`]: crate::app::HandshakeApp::new
pub fn set_preinstalled_diag_session(session: DiagSession) {
    let mut slot = match PREINSTALLED_DIAG_SESSION.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    *slot = Some(session);
}

/// Take the preinstalled [`DiagSession`] (consuming it) if `main()` set one. Returns `None` in the
/// kittest path (no `main()`), so the caller creates its own ring.
pub fn take_preinstalled_diag_session() -> Option<DiagSession> {
    let mut slot = match PREINSTALLED_DIAG_SESSION.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    slot.take()
}

#[cfg(test)]
mod tests {
    use super::*;
    use interprocess::local_socket::traits::Listener as _;
    use interprocess::local_socket::{GenericNamespaced, ToNsName};

    fn unique_test_socket(label: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        control_socket_name(&format!("{label}-{}-{nanos}", std::process::id()))
    }

    fn bind_test_listener(socket_name: &str) -> interprocess::local_socket::Listener {
        let name = socket_name.to_ns_name::<GenericNamespaced>().unwrap();
        interprocess::local_socket::ListenerOptions::new()
            .name(name)
            .create_sync()
            .unwrap()
    }

    #[test]
    fn control_socket_name_is_namespace_safe_and_session_scoped() {
        let name = control_socket_name("sess abc/123");
        assert!(name.starts_with("handshake-palmistry-"));
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "control socket name must be namespace/filename safe (got {name})"
        );
        // Two different sessions get two different sockets (no cross-session collision).
        assert_ne!(control_socket_name("a"), control_socket_name("b"));
    }

    #[test]
    fn handshake_deadlines_cover_connect_and_ack() {
        assert!(
            HANDSHAKE_OVERALL_DEADLINE
                >= HANDSHAKE_CONNECT_DEADLINE
                    .checked_add(HANDSHAKE_ACK_READ_DEADLINE)
                    .expect("bounded handshake deadlines are tiny"),
            "overall handshake deadline must cover connect retry plus ack read"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_busy_pipe_wait_uses_named_pipe_namespace() {
        assert_eq!(
            windows_named_pipe_path("handshake-palmistry-test"),
            r"\\.\pipe\handshake-palmistry-test"
        );
    }

    #[test]
    fn bounded_control_read_handles_delayed_ack_without_os_timeout() {
        let socket_name = unique_test_socket("bounded-ack");
        let listener = bind_test_listener(&socket_name);
        let server = std::thread::spawn(move || -> io::Result<()> {
            let mut conn = listener.accept()?;
            std::thread::sleep(Duration::from_millis(50));
            conn.write_all(br#"{"type":"Ack"}"#)?;
            conn.write_all(b"\n")?;
            conn.flush()
        });

        let stream = connect_control(&socket_name, Duration::from_secs(2)).unwrap();
        let (reader, line) = read_control_line_bounded(
            BufReader::new(stream),
            Duration::from_secs(2),
            "delayed ack",
        )
        .unwrap();
        assert_eq!(line, "{\"type\":\"Ack\"}\n");
        drop(reader);
        server.join().unwrap().unwrap();
    }

    #[test]
    fn bounded_control_read_times_out_without_waiting_for_peer_write() {
        let socket_name = unique_test_socket("bounded-timeout");
        let listener = bind_test_listener(&socket_name);
        let server = std::thread::spawn(move || -> io::Result<()> {
            let mut conn = listener.accept()?;
            std::thread::sleep(Duration::from_millis(120));
            let _ = conn.write_all(br#"{"type":"Ack"}"#);
            let _ = conn.write_all(b"\n");
            let _ = conn.flush();
            Ok(())
        });

        let stream = connect_control(&socket_name, Duration::from_secs(2)).unwrap();
        let started = Instant::now();
        let err = read_control_line_bounded(
            BufReader::new(stream),
            Duration::from_millis(30),
            "timeout before ack",
        )
        .expect_err("read must time out before the delayed ack arrives");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "bounded read must return before the delayed peer write"
        );
        server.join().unwrap().unwrap();
    }

    #[test]
    fn timed_out_control_read_releases_pipe_for_peer_eof() {
        let socket_name = unique_test_socket("bounded-release");
        let listener = bind_test_listener(&socket_name);
        let (eof_tx, eof_rx) = mpsc::channel();
        let server = std::thread::spawn(move || -> io::Result<()> {
            let mut conn = listener.accept()?;
            let mut byte = [0u8; 1];
            let n = conn.read(&mut byte)?;
            let _ = eof_tx.send(n);
            Ok(())
        });

        let stream = connect_control(&socket_name, Duration::from_secs(2)).unwrap();
        let err = read_control_line_bounded(
            BufReader::new(stream),
            Duration::from_millis(30),
            "never replies",
        )
        .expect_err("silent peer must time out");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        let n = eof_rx
            .recv_timeout(Duration::from_millis(250))
            .expect("timed-out client must close the real pipe, unblocking the peer");
        assert_eq!(
            n, 0,
            "peer should observe EOF after the timed-out reader drops"
        );
        server.join().unwrap().unwrap();
    }

    #[test]
    fn bounded_control_read_reports_peer_close() {
        let socket_name = unique_test_socket("bounded-eof");
        let listener = bind_test_listener(&socket_name);
        let server = std::thread::spawn(move || -> io::Result<()> {
            let conn = listener.accept()?;
            drop(conn);
            Ok(())
        });

        let stream = connect_control(&socket_name, Duration::from_secs(2)).unwrap();
        let err =
            read_control_line_bounded(BufReader::new(stream), Duration::from_secs(2), "peer close")
                .expect_err("closed peer must return EOF, not a false Ack");
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
        server.join().unwrap().unwrap();
    }

    #[test]
    fn bounded_control_read_times_out_on_partial_line() {
        let socket_name = unique_test_socket("bounded-partial");
        let listener = bind_test_listener(&socket_name);
        let server = std::thread::spawn(move || -> io::Result<()> {
            let mut conn = listener.accept()?;
            conn.write_all(b"{")?;
            conn.flush()?;
            std::thread::sleep(Duration::from_millis(120));
            Ok(())
        });

        let stream = connect_control(&socket_name, Duration::from_secs(2)).unwrap();
        let err = read_control_line_bounded(
            BufReader::new(stream),
            Duration::from_millis(30),
            "partial line",
        )
        .expect_err("partial reply without newline must stay bounded");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        server.join().unwrap().unwrap();
    }

    #[test]
    fn launcher_does_not_use_unsupported_named_pipe_timeouts() {
        let source = include_str!("palmistry_launch.rs");
        let banned = concat!("set_", "recv_timeout(");
        assert!(
            !source.contains(banned),
            "Windows named-pipe local sockets reject receive timeouts; use bounded polling reads"
        );
    }

    #[test]
    fn launcher_does_not_detach_control_reply_reader_threads() {
        let source = include_str!("palmistry_launch.rs");
        let banned = concat!("palmistry-", "control-read");
        assert!(
            !source.contains(banned),
            "timed-out control reads must drop the real pipe, not leave a detached reader thread"
        );
    }

    #[test]
    fn hello_wire_shape_matches_palmistry() {
        // Byte-for-byte the shape palmistry::control::tagged_json_shape_is_explicit pins.
        let hello = LauncherControlMessage::HandshakeHello {
            parent_pid: 7,
            session_id: "z".to_string(),
        };
        assert_eq!(
            serde_json::to_string(&hello).unwrap(),
            r#"{"type":"HandshakeHello","parent_pid":7,"session_id":"z"}"#
        );
        assert_eq!(
            serde_json::to_string(&LauncherControlMessage::Shutdown).unwrap(),
            r#"{"type":"Shutdown"}"#
        );
        let register = LauncherControlMessage::RegisterChild {
            pid: 9,
            child_session_id: 44,
            liveness_kind: LauncherChildLivenessKind::FileCounter,
            liveness_ref: PathBuf::from("child.progress"),
            threshold_ms: 5000,
        };
        assert_eq!(
            serde_json::to_string(&register).unwrap(),
            r#"{"type":"RegisterChild","pid":9,"child_session_id":44,"liveness_kind":"FileCounter","liveness_ref":"child.progress","threshold_ms":5000}"#
        );
    }

    #[test]
    fn ack_reply_decodes() {
        let ack: LauncherControlReply = serde_json::from_str(r#"{"type":"Ack"}"#).unwrap();
        assert!(matches!(ack, LauncherControlReply::Ack));
        let pong: LauncherControlReply = serde_json::from_str(r#"{"type":"Pong"}"#).unwrap();
        assert!(matches!(pong, LauncherControlReply::Pong));
        let err: LauncherControlReply =
            serde_json::from_str(r#"{"type":"Error","code":"ChildRegisterFailed"}"#).unwrap();
        assert!(matches!(
            err,
            LauncherControlReply::Error {
                code: LauncherControlErrorCode::ChildRegisterFailed
            }
        ));
    }

    #[test]
    fn missing_exe_resolves_to_not_found_error() {
        // The env override pointing at a missing file is a NotFound error (the graceful-degradation
        // trigger). A scoped env set/remove on a unique key the rest of the suite never reads.
        let bogus = std::env::temp_dir().join("definitely-not-a-palmistry-binary-mt094.exe");
        let _ = std::fs::remove_file(&bogus);
        let prev = std::env::var_os(ENV_PALMISTRY_EXE);
        std::env::set_var(ENV_PALMISTRY_EXE, &bogus);
        let resolved = resolve_palmistry_exe();
        match prev {
            Some(v) => std::env::set_var(ENV_PALMISTRY_EXE, v),
            None => std::env::remove_var(ENV_PALMISTRY_EXE),
        }
        let err = resolved.expect_err("a missing override file must resolve to an error");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn preinstalled_session_round_trips_then_empties() {
        let session = DiagSession {
            session_id: "preinstall-test".to_string(),
            ring_path: PathBuf::from("/tmp/r.ring"),
        };
        set_preinstalled_diag_session(session.clone());
        assert_eq!(take_preinstalled_diag_session(), Some(session));
        assert_eq!(take_preinstalled_diag_session(), None, "taken once only");
    }

    #[test]
    fn job_object_contract_is_documented() {
        // AC-014-3: the no-kill-on-job-close commitment is greppable + names the spec section.
        assert!(SPAWN_NOT_KILL_ON_JOB_CLOSE.contains("6.13.3"));
        assert!(SPAWN_NOT_KILL_ON_JOB_CLOSE.contains("free-standing"));
    }
}
