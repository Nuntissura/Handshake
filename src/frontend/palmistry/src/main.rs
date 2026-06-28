//! `palmistry` — the Tier 3 EXTERNAL out-of-process watcher binary (WP-KERNEL-012 MT-089).
//!
//! Master Spec v02.196 §6.13.2 ("a separate palmistry executable, distinct from the Handshake
//! process") + §6.13.3 ("Lifecycle inversion (HARD)"). Palmistry follows the Google-Crashpad model: a
//! SIBLING process, NOT a thread, NOT an in-process subsystem. It is:
//!
//! - launched WITH Handshake (the spawn is MT-094's job),
//! - controlled over a local socket ([`control`]),
//! - and lifecycle-INVERTED ([`lifecycle`]): it closes ONLY on an explicit `Shutdown`, and it SURVIVES
//!   the parent's death so it can record + persist the evidence precisely when Handshake freezes or
//!   crashes.
//!
//! This MT builds the executable + its argument/env intake + its control socket + the inverted
//! lifecycle. The ring READER (MT-090), freeze detection (MT-091), crash/minidump (MT-092), survivor
//! store + FR forward (MT-093), and the Handshake-side launch (MT-094) build on this.
//!
//! # Run shape
//!
//! ```text
//! palmistry --parent-pid <PID> --session-id <ID> --ring-path <PATH> --control-socket <NAME>
//! # or via env: HANDSHAKE_PARENT_PID / HANDSHAKE_SESSION_ID / HANDSHAKE_RING_PATH / HANDSHAKE_CONTROL_SOCK
//! ```
//!
//! Exit codes: `0` = clean lifecycle end (Shutdown, or post-death finalize after recording an abnormal
//! parent exit). `2` = a configuration / startup error (refused a partial start, or could not bind the
//! control socket). The startup error is printed to stderr.

mod cli;
mod control;
mod crash_capture;
mod freeze_detect;
mod hung_window_probe;
mod lifecycle;
mod ring_reader;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cli::PalmistryConfig;
use control::{ControlOutcome, ControlServer};
use crash_capture::{
    minidump_path_for, persist_crash_record, CrashRecord, CrashServerHandler,
    CRASH_SERVER_STALE_TIMEOUT,
};
use freeze_detect::{FreezeDetector, FreezeState};
use hung_window_probe::HungWindowProbe;
use lifecycle::{
    build_survivor_record, run_lifecycle, spawn_parent_watch, LifecycleConfig, LifecycleState,
    ParentWatch,
};
use ring_reader::PalmistryRingReader;

/// Startup-error exit code (refused partial start / could not bind control socket).
const EXIT_CONFIG_ERROR: i32 = 2;

/// Bounded backoff slept after a control-socket accept/read error before retrying. A persistent accept
/// fault (corrupted/closed listener, repeating OS accept error) MUST NOT busy-spin the control thread —
/// that would pin a CPU core and flood the log at unbounded rate while Handshake is up, the opposite of
/// a quiet background watcher (HBR-QUIET). The sleep both throttles the retry and re-checks the run flag
/// so a clean shutdown still breaks promptly.
const CONTROL_ERROR_BACKOFF: Duration = Duration::from_millis(50);

/// Consecutive-accept-error ceiling. After this many back-to-back errors the control thread stops
/// retrying (the listener is presumed unrecoverable) rather than spinning forever; the parent-watch loop
/// and the lifecycle continue, so the watcher still records a parent death and can be reaped. A single
/// successful `serve_connection` resets the counter.
const CONTROL_ERROR_CEILING: u32 = 64;

fn main() {
    // MT-092 hardening (the cross-process proof seams): handle the headless `--crash-server-probe` /
    // `--crash-client-probe` flags BEFORE the normal watcher config parse (which rejects unknown args).
    // These run the REAL production `CrashServerHandler` (server) and the REAL crash-handler client
    // install (client) in SEPARATE processes, so an end-to-end test crosses a REAL process boundary and
    // exercises the SHIPPED server handler — not a same-process thread + an inline reimplementation. They
    // never open a window and exit deterministically (HBR-QUIET).
    if let Some(code) = handle_probe_flags() {
        std::process::exit(code);
    }

    // Palmistry's own logging, separate from Handshake's. RUST_LOG / PALMISTRY_LOG controllable; quiet
    // by default so a spawned sibling does not spam. Never opens a window (HBR-QUIET).
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("PALMISTRY_LOG")
                .or_else(|_| tracing_subscriber::EnvFilter::try_new("info"))
                .unwrap_or_default(),
        )
        .with_writer(std::io::stderr)
        .try_init();

    let config = match PalmistryConfig::parse_from_process() {
        Ok(c) => c,
        Err(err) => {
            // AC-009-2: refuse a partial / malformed start with a clear non-zero error (not a silent
            // half-start).
            eprintln!("palmistry: refusing to start: {err}");
            std::process::exit(EXIT_CONFIG_ERROR);
        }
    };

    tracing::info!(
        parent_pid = config.parent_pid,
        session_id = %config.session_id,
        ring_path = %config.ring_path,
        control_socket = %config.control_socket,
        "palmistry starting (Tier 3 external watcher, lifecycle-inverted)"
    );
    // Surface the HARD launcher contract (AC-009-5): the watcher itself assumes NO Win32 Job Object
    // membership and the MT-094 launcher must not add it to a kill-on-job-close job.
    tracing::info!(contract = lifecycle::JOB_OBJECT_CONTRACT, "launcher contract");

    match run_watcher(config, RunOptions::default()) {
        Ok(()) => std::process::exit(0),
        Err(err) => {
            eprintln!("palmistry: startup error: {err}");
            std::process::exit(EXIT_CONFIG_ERROR);
        }
    }
}

// ----------------------------------------------------------------------------------------------------
// MT-092 hardening — the CROSS-PROCESS crash proof seams (the §6.13.6 out-of-process invariant proven
// across a REAL process boundary, against the SHIPPED `CrashServerHandler`).
// ----------------------------------------------------------------------------------------------------
//
// The original AC-012-1 proof ran a `minidumper::Server` and its `minidumper::Client` + the crash on the
// SAME process (different threads), so `minidump-writer` dumped the test process's own memory from another
// THREAD — never another PROCESS. That asserts the dump is real (validated by the reader) but NOT the
// cross-PROCESS boundary that is the entire point of §6.13.6 / RISK-012-1. It also ran an INLINE
// reimplementation of the server handler, leaving the production `CrashServerHandler` untested at runtime.
//
// These two flags fix both: `--crash-server-probe` runs the REAL production `CrashServerHandler` through
// `minidumper::Server::run` and, on a captured dump, writes the REAL RICH `CrashRecord`
// (`CrashRecord::with_minidump` + `persist_crash_record`, naming the thread id captured by the handler's
// own `on_message`). `--crash-client-probe`, run as a SEPARATE process, installs the REAL crash-handler
// client, reports the faulting thread id, and fires `simulate_exception` so the server dumps a DIFFERENT
// process's memory across a real OS boundary. The integration test
// (`tests/test_crash_capture.rs::cross_process_*`) spawns both and asserts a validated minidump + the
// `detection=CrashContextMinidump` record written by the shipped handler.

/// Dispatch the headless crash-proof probe flags. Returns `Some(exit_code)` when a probe flag was handled
/// (the caller exits with it) or `None` to fall through to the normal watcher launch. Only the FIRST
/// recognized probe flag is acted on; ordinary watcher args (`--parent-pid`, env-driven launch) are left
/// for `PalmistryConfig::parse_from_process`.
fn handle_probe_flags() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // `--crash-server-probe <socket> <dump_path> <record_path>`: run the REAL CrashServerHandler.
        Some("--crash-server-probe") => Some(run_crash_server_probe(&args[1..])),
        // `--crash-client-probe <socket>`: connect + fire a simulated exception (SEPARATE process).
        Some("--crash-client-probe") => Some(run_crash_client_probe(&args[1..])),
        _ => None,
    }
}

/// SERVER PROBE (the SHIPPED handler under a real process boundary). Binds a `minidumper::Server` on
/// `args[0]` (socket), runs the REAL [`CrashServerHandler`] (writing the dump to `args[1]`), and on a
/// captured dump persists the REAL RICH [`CrashRecord`] to `args[2]` — exactly the production
/// `with_minidump` + `persist`-shaped record, naming the faulting thread id the handler's own `on_message`
/// captured. Exits 0 once a dump is captured + the record is written, 1 on any failure. Used by the
/// cross-process test; never opens a window.
fn run_crash_server_probe(args: &[String]) -> i32 {
    let (socket, dump_path, record_path) = match args {
        [s, d, r] => (s.clone(), PathBuf::from(d), PathBuf::from(r)),
        _ => {
            eprintln!(
                "palmistry --crash-server-probe needs <socket> <dump_path> <record_path>; got {args:?}"
            );
            return EXIT_CONFIG_ERROR;
        }
    };

    // Construct the SHIPPED handler (not an inline copy) so the test exercises the real
    // create_minidump_file / on_minidump_created / on_message + captured/thread-id latches.
    let handler = CrashServerHandler::new(dump_path.clone());
    let captured = handler.captured_flag();
    let thread_id = handler.faulting_thread_id_handle();
    let write_error = handler.write_error_handle();

    let mut server = match default_crash_server(&socket) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("crash-server-probe: bind failed on '{socket}': {err}");
            return 1;
        }
    };
    // Signal readiness on stdout so the test parent can launch the client only after the bind succeeds
    // (avoids a connect race). A single line; the test reads it before spawning the client probe.
    println!("CRASH_SERVER_PROBE_READY");
    let _ = std::io::stdout().flush();

    // minidumper's `shutdown` flag STOPS the loop when `true`; start it `false` so the server keeps
    // accepting until the handler returns `LoopAction::Exit` (after the first dump). (The inverted polarity
    // is the production bug this probe exists to catch — see the `crash_shutdown` flag in `run_watcher`.)
    let shutdown = Arc::new(AtomicBool::new(false));
    if let Err(err) = server.run(Box::new(handler), &shutdown, Some(CRASH_SERVER_STALE_TIMEOUT)) {
        eprintln!("crash-server-probe: server loop error: {err}");
        return 1;
    }

    if !captured.load(Ordering::SeqCst) {
        let err = write_error
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .unwrap_or_else(|| "no dump captured (client never requested one)".to_string());
        eprintln!("crash-server-probe: no minidump captured: {err}");
        return 1;
    }

    // Write the REAL RICH crash record naming the dump + the faulting thread id the SHIPPED handler's
    // on_message captured. This is the production `with_minidump` shape (detection=CrashContextMinidump).
    let faulting_thread_id = thread_id.load(Ordering::SeqCst);
    let record = CrashRecord::with_minidump(
        "crash-server-probe",
        std::process::id(),
        faulting_thread_id,
        dump_path.clone(),
        None,
        &[],
    );
    let json = match serde_json::to_string_pretty(&record) {
        Ok(j) => j,
        Err(err) => {
            eprintln!("crash-server-probe: record serialize failed: {err}");
            return 1;
        }
    };
    if let Err(err) = std::fs::write(&record_path, json) {
        eprintln!("crash-server-probe: record write failed: {err}");
        return 1;
    }
    0
}

/// CLIENT PROBE (a SEPARATE process from the server). Connects a `minidumper::Client` to `args[0]`
/// (socket), installs the REAL crash-handler, reports the faulting thread id (typed 8-byte LE u64, never
/// text), and fires `simulate_exception` so the SERVER process dumps THIS process's memory across a real
/// OS boundary. Mirrors the production handshake-native client callback exactly. Exits 0 when the dump
/// request was handled, 1 otherwise. Never opens a window.
fn run_crash_client_probe(args: &[String]) -> i32 {
    let socket = match args {
        [s] => s.clone(),
        _ => {
            eprintln!("palmistry --crash-client-probe needs <socket>; got {args:?}");
            return EXIT_CONFIG_ERROR;
        }
    };
    // Bounded connect-retry: across a REAL process boundary the server's AF_UNIX listen() backlog can
    // briefly race the client's first connect (Windows returns WSAECONNREFUSED 10061 in that window), so
    // retry with a short backoff until connected or the bounded deadline elapses. This is the field-
    // standard cross-process rendezvous hardening the MT-094 launcher will also need; it never busy-spins
    // (it sleeps between attempts) and is bounded (it gives up + reports rather than hanging).
    let connect_deadline = std::time::Instant::now() + Duration::from_secs(10);
    let client = loop {
        match minidumper::Client::with_name(minidumper::SocketName::path(&socket)) {
            Ok(c) => break Arc::new(c),
            Err(err) => {
                if std::time::Instant::now() >= connect_deadline {
                    eprintln!("crash-client-probe: connect failed on '{socket}': {err}");
                    return 1;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let client_cb = Arc::clone(&client);
    #[allow(unsafe_code)]
    let handler = match crash_handler::CrashHandler::attach(unsafe {
        crash_handler::make_crash_event(move |cc: &crash_handler::CrashContext| {
            // Report the faulting thread id FIRST (so the server's on_message can name it in the RICH
            // record), then request the OUT-OF-PROCESS dump — the EXACT production client shape.
            #[cfg(windows)]
            let tid: u64 = cc.thread_id as u64;
            #[cfg(not(windows))]
            let tid: u64 = 0;
            let _ = client_cb.send_message(
                crash_capture::MSG_KIND_FAULTING_THREAD_ID,
                tid.to_le_bytes(),
            );
            crash_handler::CrashEventResult::Handled(client_cb.request_dump(cc).is_ok())
        })
    }) {
        Ok(h) => h,
        Err(err) => {
            eprintln!("crash-client-probe: attach failed: {err}");
            return 1;
        }
    };
    // FIRE a REAL simulated exception (a real captured context) WITHOUT killing this process. The callback
    // signals the SERVER process, which writes the dump out-of-process across the boundary.
    let handled = matches!(
        handler.simulate_exception(None),
        crash_handler::CrashEventResult::Handled(true)
    );
    drop(handler);
    if handled {
        0
    } else {
        eprintln!("crash-client-probe: the simulated exception was not handled (dump request failed)");
        1
    }
}

/// Options controlling how `run_watcher` opens its OS resources. The defaults are the real production
/// behavior used by `main()` (`RunOptions::default()`), and are the path the integration tests exercise
/// end-to-end by driving the compiled binary (`CARGO_BIN_EXE_palmistry`) — the binary's `pub` items are
/// not importable by `tests/`, so the tests do not construct `RunOptions` directly. The seams here
/// (`parent_watch_factory` to swap the parent watch, `on_ready` to observe "started + staying alive")
/// exist so an in-crate caller or future `[lib]` extraction can inject a fake watch or readiness probe
/// without forking the run path.
pub struct RunOptions {
    /// Builds the parent watch for a pid. Default = the real Windows watch (or a never-exits stub off
    /// Windows so the crate still builds + the lifecycle is exercisable cross-platform).
    pub parent_watch_factory: Box<dyn Fn(u32) -> Box<dyn ParentWatch>>,
    /// Opens the MT-090 ring reader for a ring path. Default = the bounded open-retry against the real
    /// backing file ([`PalmistryRingReader::open_with_default_retry`]). The seam exists so an in-crate
    /// caller / future `[lib]` extraction can inject a faster-retry or pre-opened reader without forking
    /// the run path; the integration tests drive the real default through the compiled binary.
    pub ring_reader_factory: RingReaderFactory,
    /// Builds the MT-091 hung-window probe for the watched pid. Default = the real Win32
    /// `SendMessageTimeoutW(WM_NULL)` probe on Windows (a never-finds-a-window stub off Windows). The
    /// seam exists so an in-crate caller / future `[lib]` extraction can inject a fake probe; the
    /// freeze-detector unit + integration tests exercise the double-signal gate through fakes directly.
    pub hung_window_probe_factory: HungWindowProbeFactory,
    /// Lifecycle timing. Default = production timings.
    pub lifecycle: LifecycleConfig,
    /// MT-092: builds the Embark `minidumper::Server` for the derived crash-socket name. Default = the
    /// real `minidumper::Server::with_name(SocketName::path(..))`. The seam exists so an in-crate caller /
    /// future `[lib]` extraction can inject a pre-bound or fake server; the integration tests drive the
    /// real default through the compiled binary. An `Err` is NON-FATAL (the post-mortem floor still
    /// records a crash).
    pub crash_server_factory: CrashServerFactory,
    /// Optional callback invoked once the watcher is fully started (control socket bound, parent watch
    /// armed) and about to enter its run loop. Used by tests to assert "started + staying alive".
    pub on_ready: Option<Box<dyn FnOnce() + Send>>,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            parent_watch_factory: Box::new(default_parent_watch),
            ring_reader_factory: Box::new(default_ring_reader),
            hung_window_probe_factory: Box::new(default_hung_window_probe),
            lifecycle: LifecycleConfig::default(),
            crash_server_factory: Box::new(default_crash_server),
            on_ready: None,
        }
    }
}

/// Factory that builds the MT-092 Embark `minidumper::Server` for a crash-socket name. Boxed so a caller
/// can inject a pre-bound / fake server without forking the run path. Aliased to keep the [`RunOptions`]
/// field type legible (clippy type-complexity).
pub type CrashServerFactory =
    Box<dyn Fn(&str) -> Result<minidumper::Server, minidumper::Error>>;

/// The real crash-server factory: bind a `minidumper::Server` on the derived crash socket (a Windows
/// named pipe / a Unix domain socket path). This is the SERVER side of the §6.13.6 out-of-process dump
/// pipeline — it accepts the crashing client's `request_dump` and writes the minidump from OUTSIDE the
/// dying process. An `Err` (name taken / invalid) is propagated and the caller degrades to the
/// post-mortem floor (non-fatal).
fn default_crash_server(crash_socket: &str) -> Result<minidumper::Server, minidumper::Error> {
    minidumper::Server::with_name(minidumper::SocketName::path(crash_socket))
}

/// Factory that opens the MT-090 ring reader for a given ring path. Boxed so a caller can inject a
/// fake / faster-retry reader without forking the run path. Aliased to keep the [`RunOptions`] field
/// type legible (clippy type-complexity).
pub type RingReaderFactory = Box<dyn Fn(&str) -> std::io::Result<PalmistryRingReader>>;

/// The real ring-reader factory: bounded open-retry against the backing file at `ring_path`. Retries
/// with a bounded backoff until the ring appears + validates or the bounded deadline elapses, so a
/// startup race with Handshake is tolerated without crashing or busy-spinning (MT-090 / AC-010-3).
fn default_ring_reader(ring_path: &str) -> std::io::Result<PalmistryRingReader> {
    PalmistryRingReader::open_with_default_retry(Path::new(ring_path))
}

/// Factory that builds the MT-091 hung-window probe for a watched pid. Boxed so a caller can inject a
/// fake probe without forking the run path. Aliased to keep the [`RunOptions`] field type legible
/// (clippy type-complexity).
pub type HungWindowProbeFactory = Box<dyn Fn(u32) -> Box<dyn HungWindowProbe>>;

/// The real hung-window-probe factory: the Win32 `SendMessageTimeoutW(WM_NULL)` probe on Windows; off
/// Windows a probe that never finds a window (so the freeze detector can only ever SUSPECT, never confirm
/// a hard freeze, on a non-Windows build — the real corroboration is Windows-only, matching the proof
/// host).
fn default_hung_window_probe(pid: u32) -> Box<dyn HungWindowProbe> {
    #[cfg(windows)]
    {
        Box::new(hung_window_probe::Win32HungWindowProbe::new(pid))
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        Box::new(hung_window_probe::FakeHungWindowProbe::new(
            hung_window_probe::ProbeResult::WindowNotFound,
        ))
    }
}

/// The real parent-watch factory: the Windows `OpenProcess`/`WaitForSingleObject` watch on Windows. If
/// the process cannot be opened (already gone / denied), a watch that reports the parent vanished
/// immediately is returned so the lifecycle still records an abnormal disappearance rather than
/// blocking forever.
fn default_parent_watch(pid: u32) -> Box<dyn ParentWatch> {
    #[cfg(windows)]
    {
        match lifecycle::WindowsParentWatch::open(pid) {
            Some(w) => Box::new(w),
            None => Box::new(VanishedParentWatch),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        // Off Windows (the proof host is Windows): a watch that never reports an exit, so the lifecycle
        // is still driven by the control socket. The real cross-platform impl is a future MT; the seam
        // exists so this builds everywhere.
        Box::new(NeverExitsParentWatch)
    }
}

/// A watch that reports the parent already gone (used when OpenProcess fails — the pid was already
/// dead/denied at startup). Yields a `Vanished` exit on the first wait.
struct VanishedParentWatch;
impl ParentWatch for VanishedParentWatch {
    fn wait_for_parent_exit(&self, _timeout: std::time::Duration) -> Option<lifecycle::ParentExit> {
        Some(lifecycle::ParentExit::Vanished)
    }
}

/// A watch that never observes an exit (the non-Windows build stub). Sleeps the slice then yields None.
#[cfg(not(windows))]
struct NeverExitsParentWatch;
#[cfg(not(windows))]
impl ParentWatch for NeverExitsParentWatch {
    fn wait_for_parent_exit(&self, timeout: std::time::Duration) -> Option<lifecycle::ParentExit> {
        std::thread::sleep(timeout);
        None
    }
}

/// The latest freeze verdict the MT-091 poll thread published, shared into the watcher so the survivor
/// store (MT-093) and any observer can read the current state + whether a freeze was EVER confirmed
/// during the session. `freeze_ever_confirmed` latches true on the first confirmed freeze (a freeze that
/// later recovers is still evidence MT-093 must forward), while `latest` reflects the live current state
/// (which CAN return to Healthy on recovery — AC-011-4).
#[derive(Debug)]
pub struct FreezeStatus {
    /// The most recent [`FreezeState`] the poll thread observed.
    latest: Mutex<FreezeState>,
    /// Latches true the first time a freeze is CONFIRMED in this session (never cleared, so MT-093 can
    /// see a freeze happened even after recovery).
    freeze_ever_confirmed: AtomicBool,
}

impl Default for FreezeStatus {
    fn default() -> Self {
        Self {
            latest: Mutex::new(FreezeState::Healthy),
            freeze_ever_confirmed: AtomicBool::new(false),
        }
    }
}

impl FreezeStatus {
    /// The live current freeze state.
    pub fn latest(&self) -> FreezeState {
        *self.latest.lock().expect("freeze status mutex")
    }

    /// Whether a freeze was ever CONFIRMED in this session (latched; survives a later recovery).
    pub fn freeze_ever_confirmed(&self) -> bool {
        self.freeze_ever_confirmed.load(Ordering::SeqCst)
    }

    /// Record the latest observed state, latching `freeze_ever_confirmed` on a confirmed freeze.
    fn record(&self, state: FreezeState) {
        if state.is_frozen() {
            self.freeze_ever_confirmed.store(true, Ordering::SeqCst);
        }
        *self.latest.lock().expect("freeze status mutex") = state;
    }
}

/// Spawn the MT-091 FREEZE-POLL thread (§6.13.5). On a dedicated thread (NOT the control loop —
/// RISK-011-4: the bounded hung-window probe must never block the control path), it polls the
/// [`FreezeDetector`] every [`freeze_detect::POLL_INTERVAL`] with the heartbeat read PASSIVELY through
/// the zero-cooperation MT-090 reader and the hung-window probe. On a Healthy->Frozen transition it
/// records the freeze (a `FreezeSuspected`-style marker in Palmistry's own log + the shared
/// [`FreezeStatus`]); the durable survivor record is MT-093. It stops when `run` is cleared by the
/// lifecycle. Returns the join handle.
fn spawn_freeze_poll(
    reader: Arc<PalmistryRingReader>,
    probe: Box<dyn HungWindowProbe>,
    status: Arc<FreezeStatus>,
    run: Arc<AtomicBool>,
    poll_interval: Duration,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut detector = FreezeDetector::new();
        let mut last_was_frozen = false;
        while run.load(Ordering::SeqCst) {
            // PASSIVE liveness read (§6.13.4): a pure seqlock read of shared memory — no call into / lock
            // against / wait on the (possibly frozen) writer. A frozen writer's last heartbeat stays
            // readable, which is exactly the staleness the detector keys on.
            let heartbeat = reader.read_heartbeat();
            let state = detector.poll(std::time::Instant::now(), heartbeat, probe.as_ref());
            let now_frozen = state.is_frozen();

            // Record a `FreezeSuspected`-style marker on the Healthy->Frozen EDGE (not every tick) so the
            // log/state is not spammed while a freeze persists. MT-093 turns this into the durable
            // survivor record + the Tier-1 FR forward at recovery.
            match (last_was_frozen, &state) {
                (false, FreezeState::Frozen(report)) => {
                    tracing::warn!(
                        diag_event_code = handshake_diag_ring::DiagEventCode::FreezeSuspected.as_u16(),
                        stale_ms = report.stale_ms,
                        last_heartbeat_counter = report.last_heartbeat_counter,
                        last_heartbeat_ts_nanos = report.last_heartbeat_ts_nanos,
                        "FREEZE CONFIRMED (heartbeat stale + hung-window probe not responding) — \
                         the §6.13.5 double-signal gate fired; survivor capture is MT-093"
                    );
                }
                (true, FreezeState::Healthy) => {
                    tracing::info!("freeze RECOVERED — heartbeat resumed advancing (§6.13.5 recovery)");
                }
                (false, FreezeState::Suspected { stale_ms }) => {
                    // Suspected-only: log at debug so a borderline long frame does not flood warnings.
                    tracing::debug!(
                        stale_ms,
                        "heartbeat stale but window still responding — SUSPECTED only, not a confirmed \
                         freeze (double-signal gate held)"
                    );
                }
                _ => {}
            }

            status.record(state);
            last_was_frozen = now_frozen;
            std::thread::sleep(poll_interval);
        }
    })
}

/// Run the watcher to lifecycle completion. The library-shaped entrypoint (tested directly): it binds
/// the control socket, arms the parent watch, spawns the control thread + parent-watch thread, runs the
/// lifecycle, and persists the survivor record next to the ring. Returns `Err` only for a STARTUP
/// failure (bad ring path / control bind) — a normal lifecycle end returns `Ok(())`.
pub fn run_watcher(config: PalmistryConfig, options: RunOptions) -> std::io::Result<()> {
    // MT-090: OPEN the MT-081 ring on the read side, with BOUNDED open-retry (AC-010-3 / RISK-010-3).
    // Palmistry is launched WITH Handshake (MT-094) and may WIN the startup race, so the ring file may
    // not exist (or not yet carry a valid header) at this instant. `open_with_retry` retries with a
    // bounded backoff until the ring appears + validates or a bounded deadline elapses — it does NOT
    // crash on a missing ring and does NOT busy-spin. A still-absent / invalid ring after the deadline
    // is a clear startup error (a misconfigured ring path), not a silently broken watcher.
    //
    // The resulting reader is the ZERO-COOPERATION passive observer (§6.13.4): it reads Handshake's
    // heartbeat + last-N events from plain shared memory with no call into, lock against, or wait on the
    // (possibly frozen) writer. It is held live for the watcher's life so the freeze-probe (MT-091) and
    // the survivor store (MT-093) read through it. Wrapped in `Arc` so a future freeze-probe thread can
    // share it without re-opening the map.
    let ring_reader = Arc::new(
        (options.ring_reader_factory)(&config.ring_path).map_err(|err| {
            std::io::Error::new(
                err.kind(),
                format!(
                    "ring path '{}' is not a valid diagnostic ring (open-retry exhausted): {err}",
                    config.ring_path
                ),
            )
        })?,
    );
    tracing::info!(
        ring_path = %config.ring_path,
        capacity = ring_reader.capacity(),
        "diag ring opened (Tier 3 zero-cooperation reader ready)"
    );

    // Bind the control socket. A watcher with no control channel can never be cleanly shut down, so a
    // bind failure is fatal (startup error).
    let server = ControlServer::bind(&config.control_socket)?;
    tracing::info!(socket = %server.socket_name(), "control socket bound");

    let state = Arc::new(LifecycleState::new());
    let run = Arc::new(AtomicBool::new(true));

    // Arm the parent watch + spawn its thread (the inversion engine).
    let watch = (options.parent_watch_factory)(config.parent_pid);
    let watch_handle = spawn_parent_watch(
        watch,
        Arc::clone(&state),
        Arc::clone(&run),
        options.lifecycle.parent_watch_slice,
    );

    // Spawn the control thread: accept connections and feed Shutdown into the shared state. It loops
    // accepting connections so a dropped control connection does NOT end the watcher (only a Shutdown
    // message does — §6.13.3). It stops when `run` is cleared by the lifecycle.
    let control_state = Arc::clone(&state);
    let control_run = Arc::clone(&run);
    let control_handle = std::thread::spawn(move || {
        let mut serve_once = || {
            let mut on_msg = |msg: &control::ControlMessage| {
                tracing::debug!(?msg, "control message");
                if matches!(msg, control::ControlMessage::Shutdown) {
                    control_state.request_shutdown();
                }
            };
            server.serve_connection(&mut on_msg)
        };
        run_control_loop(
            &mut serve_once,
            &control_run,
            CONTROL_ERROR_BACKOFF,
            CONTROL_ERROR_CEILING,
            |reason| match reason {
                ControlLoopExit::Shutdown => control_state.request_shutdown(),
                ControlLoopExit::RunCleared => {}
                ControlLoopExit::ErrorCeiling { consecutive_errors } => {
                    // The control socket is presumed unrecoverable. Do NOT spin forever; record the fault
                    // and stop accepting. The parent-watch loop + lifecycle keep running so a parent death
                    // is still recorded and the watcher can be reaped (it is no longer remotely shut down).
                    tracing::error!(
                        consecutive_errors,
                        "control socket accept failed repeatedly; abandoning the control accept loop \
                         (watcher keeps watching the parent; remote Shutdown is no longer available)"
                    );
                }
            },
        );
    });

    // Spawn the MT-091 FREEZE-POLL thread (§6.13.5). On its OWN thread (NOT the control loop —
    // RISK-011-4: the bounded hung-window probe must never block control), it polls the FreezeDetector
    // every POLL_INTERVAL with the heartbeat read PASSIVELY through the zero-cooperation reader and the
    // hung-window probe, and publishes the typed verdict into the shared FreezeStatus. The reader is
    // shared (Arc) so the poll thread reads the SAME map without re-opening it. It stops when `run` is
    // cleared by the lifecycle.
    let freeze_status = Arc::new(FreezeStatus::default());
    let probe = (options.hung_window_probe_factory)(config.parent_pid);
    let freeze_handle = spawn_freeze_poll(
        Arc::clone(&ring_reader),
        probe,
        Arc::clone(&freeze_status),
        Arc::clone(&run),
        freeze_detect::POLL_INTERVAL,
    );
    tracing::info!(
        poll_interval_ms = freeze_detect::POLL_INTERVAL.as_millis() as u64,
        freeze_threshold_ms = freeze_detect::FREEZE_THRESHOLD.as_millis() as u64,
        idle_heartbeat_cadence_ms = freeze_detect::MT084_IDLE_HEARTBEAT_CADENCE.as_millis() as u64,
        "freeze-detection poll thread armed (§6.13.5 double-signal gate)"
    );

    // MT-092: spawn the CRASH minidumper SERVER thread (§6.13.6). On its OWN dedicated thread it runs the
    // Embark `minidumper::Server`, listening on a DEDICATED minidumper socket (derived from the control
    // socket name so the launcher can pass exactly one base name). When the CLIENT (Handshake) signals a
    // crash with a `CrashContext`, the server writes a MINIDUMP OUT-OF-PROCESS (reading the crashing
    // client's memory cross-process via minidump-writer) to a LOCAL `.dmp` sibling of the ring. The
    // server loop exits after the first dump (one crash per session) or when `run` is cleared. The
    // `captured` latch tells the post-lifecycle code whether a rich minidump was written, so it does not
    // ALSO write a post-mortem record for the same crash. The handler holds the LOCAL dump path; there is
    // NO upload anywhere (§6.13.8 local-only). A crash-socket BIND failure is logged but NOT fatal: the
    // process-handle-wait post-mortem path (the FLOOR) still records a crash, so the watcher degrades to
    // best-effort capture rather than refusing to watch.
    let minidump_path = minidump_path_for(Path::new(&config.ring_path), &config.session_id);
    let crash_handler = CrashServerHandler::new(minidump_path.clone());
    let crash_captured = crash_handler.captured_flag();
    let crash_thread_id = crash_handler.faulting_thread_id_handle();
    let crash_write_error = crash_handler.write_error_handle();
    let crash_socket = crash_socket_path(&config.control_socket);
    // The minidumper `Server::run` `shutdown` flag uses INVERTED polarity from our `run` flag: minidumper
    // STOPS when its flag is `true`, whereas our `run` is `true` while the watcher is ALIVE. Passing `run`
    // directly would make the crash server return IMMEDIATELY at startup (the CrashContext-driven RICH
    // minidump path would silently never fire). So the crash server gets its OWN dedicated flag,
    // `crash_shutdown`, that starts `false` (keep running) and is flipped to `true` at teardown once the
    // lifecycle has ended — the correct minidumper polarity (§6.13.6 / RISK-012-1).
    let crash_shutdown = Arc::new(AtomicBool::new(false));
    let crash_server_shutdown = Arc::clone(&crash_shutdown);
    let crash_handle = match (options.crash_server_factory)(&crash_socket) {
        Ok(mut server) => {
            tracing::info!(
                socket = %crash_socket,
                minidump_path = %minidump_path.display(),
                "crash minidumper server bound (§6.13.6 out-of-process dump writer armed)"
            );
            Some(std::thread::spawn(move || {
                // `minidumper::Server::run` blocks accepting + serving the client until a dump is written
                // (handler returns LoopAction::Exit) or `crash_server_shutdown` is SET (true). A
                // stale-timeout reaps a silent (crashed/exited) client connection so the loop never blocks
                // forever.
                if let Err(err) = server.run(
                    Box::new(crash_handler),
                    &crash_server_shutdown,
                    Some(CRASH_SERVER_STALE_TIMEOUT),
                ) {
                    tracing::warn!(%err, "crash minidumper server loop ended with an error");
                }
            }))
        }
        Err(err) => {
            // NON-FATAL: the FLOOR (post-mortem) path still records a crash. Record + continue.
            tracing::warn!(
                %err,
                socket = %crash_socket,
                "could not bind the crash minidumper socket; the CrashContext-driven minidump path is \
                 unavailable this session — the process-handle-wait post-mortem record (the floor) still \
                 covers a crash (§6.13.6 best-effort)"
            );
            None
        }
    };

    // Fully started: control socket bound, parent watch armed, ring reader open. Take an initial
    // PASSIVE liveness reading through the zero-cooperation reader so the watcher records, at readiness,
    // that it can observe Handshake's published heartbeat from shared memory (the data source the
    // MT-091 freeze probe + the MT-093 survivor store consume). This is a pure seqlock read — no call
    // into / lock against / wait on the writer (§6.13.4). `None` here is not fatal: Handshake may not
    // have published its first heartbeat yet (MT-091 owns the stall/no-heartbeat policy).
    match ring_reader.read_heartbeat() {
        Some(hb) => tracing::info!(
            heartbeat_counter = hb.counter,
            heartbeat_ts_nanos = hb.timestamp_nanos,
            "initial passive liveness reading taken from the diag ring"
        ),
        None => tracing::info!("no heartbeat published yet (Handshake may still be starting)"),
    }

    // Signal readiness (tests assert started-and-staying-alive here) and enter the run loop. The reader
    // is shared into the callback so an in-crate caller / future freeze-probe thread (MT-091) can read
    // liveness without re-opening the map.
    if let Some(cb) = options.on_ready {
        cb();
    }

    // Drive the lifecycle to its terminal reason. This BLOCKS until Shutdown or a recorded abnormal
    // parent death + finalize. The `ring_reader` is held live across this whole window so the freeze
    // probe (MT-091) and survivor store (MT-093) can read Handshake's last-published state even while
    // the writer is frozen.
    let reason = run_lifecycle(&state, &run, options.lifecycle);
    tracing::info!(?reason, "lifecycle ended");

    // Persist the survivor record next to the ring (a sibling JSON file). MT-093 forwards it to the FR;
    // here we durably write the lifecycle facts so the evidence survives even if nothing else runs.
    let record = build_survivor_record(&config.session_id, config.parent_pid, &state, reason);
    if let Err(err) = persist_survivor_record(&config.ring_path, &config.session_id, &record) {
        // A persist failure is logged but does not change the exit class — the lifecycle still ended
        // correctly; the record write is best-effort durable evidence.
        tracing::warn!(%err, "failed to persist survivor record");
    }

    // Surface the freeze evidence the poll thread accumulated (MT-093 will persist it durably; here it is
    // logged so a session that observed a freeze is visible even before MT-093 lands).
    if freeze_status.freeze_ever_confirmed() {
        tracing::warn!(
            latest = ?freeze_status.latest(),
            "a freeze was CONFIRMED during this session (§6.13.5); MT-093 owns the durable survivor capture"
        );
    }

    // MT-092: CRASH RECORD decision (§6.13.6 + §6.13 clean-shutdown rule). Two outcomes produce a crash
    // record; a CLEAN shutdown produces NEITHER (RISK-012-2):
    //
    //   (a) RICH: a `CrashContext` arrived and the minidumper server wrote an out-of-process minidump
    //       (`crash_captured` latched). Write a typed crash record NAMING the local dump file.
    //   (b) FLOOR: the parent died ABNORMALLY (the MT-089 `abnormal_parent_exit` signal) with NO
    //       CrashContext (a hard kill delivers none). Write a best-effort post-mortem crash record (exit
    //       code + last heartbeat/events) — no minidump (impossible post-mortem without the client).
    //
    // A clean Shutdown (`state.parent_exit_abnormal()` is false) writes NO crash record + NO minidump —
    // the §6.13 clean-shutdown rule (AC-012-2). The two paths are mutually exclusive here: if a rich
    // minidump was captured we do NOT also write a floor record for the same crash.
    let last_heartbeat = ring_reader.read_heartbeat();
    let last_events = ring_reader.read_last_events(ring_reader.capacity().min(64));
    if crash_captured.load(Ordering::SeqCst) {
        // (a) RICH: a CrashContext-driven out-of-process minidump was written. Record it.
        let faulting_thread_id = crash_thread_id.load(Ordering::SeqCst);
        let crash = CrashRecord::with_minidump(
            &config.session_id,
            config.parent_pid,
            faulting_thread_id,
            minidump_path.clone(),
            last_heartbeat,
            &last_events,
        );
        match persist_crash_record(Path::new(&config.ring_path), &crash) {
            Ok(path) => tracing::warn!(
                diag_event_code = handshake_diag_ring::DiagEventCode::CrashDetected.as_u16(),
                minidump_path = %minidump_path.display(),
                record_path = %path.display(),
                faulting_thread_id,
                "CRASH CAPTURED — out-of-process minidump written (§6.13.6) + typed crash record persisted"
            ),
            Err(err) => tracing::warn!(%err, "failed to persist the crash record"),
        }
    } else if state.parent_exit_abnormal() {
        // (b) FLOOR: abnormal parent death, no CrashContext — best-effort post-mortem record. Surface a
        // crash-socket write error if one occurred so a missed RICH dump is visible.
        if let Ok(slot) = crash_write_error.lock() {
            if let Some(err) = slot.as_ref() {
                tracing::warn!(error = %err, "the crash minidumper server reported a dump-write error");
            }
        }
        let crash = CrashRecord::post_mortem(
            &config.session_id,
            config.parent_pid,
            state.parent_exit_code(),
            last_heartbeat,
            &last_events,
        );
        match persist_crash_record(Path::new(&config.ring_path), &crash) {
            Ok(path) => tracing::warn!(
                diag_event_code = handshake_diag_ring::DiagEventCode::CrashDetected.as_u16(),
                exit_code = ?state.parent_exit_code(),
                record_path = %path.display(),
                "CRASH RECORDED (post-mortem, no CrashContext — hard kill / abrupt exit); a full minidump \
                 needs the CrashContext path (§6.13.6 floor)"
            ),
            Err(err) => tracing::warn!(%err, "failed to persist the post-mortem crash record"),
        }
    } else {
        // CLEAN shutdown: NO crash record, NO minidump (§6.13 clean-shutdown rule, AC-012-2). The crash
        // server never wrote a dump (no CrashContext arrived) and the exit was not abnormal, so there is
        // nothing to write and nothing to clean up.
        tracing::debug!("clean shutdown — no crash record or minidump written (§6.13)");
    }

    // Tear down: `run` is already false (set by run_lifecycle). The parent-watch thread observes it and
    // returns; the freeze-poll thread observes it on its next sleep boundary; the control thread is woken
    // by `run` going false on its next accept boundary. We join the watch + freeze threads (bounded) and
    // detach the control thread (its blocking accept may outlive us harmlessly — the process is exiting).
    let _ = watch_handle.join();
    let _ = freeze_handle.join();
    // SIGNAL the crash server to stop (its flag is INVERTED vs `run`: minidumper stops when its flag is
    // `true`), then join it (bounded). The server loop re-checks the flag every ~10ms, so it returns
    // promptly. If it already exited after writing a dump, the flag is moot and the join is immediate.
    crash_shutdown.store(true, Ordering::SeqCst);
    if let Some(handle) = crash_handle {
        let _ = handle.join();
    }
    drop(control_handle); // detached; the process exit closes the socket.
    drop(ring_reader); // unmap the diag ring; the watcher no longer needs Handshake's published state.

    Ok(())
}

/// Derive the DEDICATED minidumper crash-socket path from the control-socket base name. The Embark
/// `minidumper` IPC uses its OWN socket, distinct from the interprocess control socket, so the two
/// channels never collide. On EVERY platform (including Windows 10+), minidumper binds an AF_UNIX
/// (`PF_UNIX` / `SOCK_STREAM`) socket whose address is a real FILESYSTEM PATH in `sun_path` — NOT a
/// `\\.\pipe\` named pipe (verified from the minidumper 0.10 source: `ipc/windows.rs` uses `sockaddr_un`
/// with a 108-byte `sun_path`). So the crash socket is a short `.sock` file under the OS temp dir on all
/// platforms. The path must fit `sun_path` (108 bytes), so the base token is truncated to keep it short.
/// The CLIENT derives the SAME path with the SAME rule so they rendezvous.
pub fn crash_socket_path(control_socket: &str) -> String {
    crash_socket_path_for_token(&crash_capture::safe_session_token(control_socket))
}

/// Build the minidumper crash-socket filesystem path for an already-sanitized token. Truncates the token
/// so the full path stays within the AF_UNIX `sun_path` 108-byte limit (the temp-dir prefix + suffix
/// leave well under 108 bytes for a ~40-char token).
fn crash_socket_path_for_token(token: &str) -> String {
    let short: String = token.chars().take(40).collect();
    std::env::temp_dir()
        .join(format!("hsk-crash-{short}.sock"))
        .to_string_lossy()
        .into_owned()
}

/// Why the control accept loop terminated. Returned to the loop's `on_exit` callback so the caller owns
/// the side effects (request a shutdown, record a fault) instead of the loop hard-coding them — which is
/// what makes the loop unit-testable without a real socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLoopExit {
    /// A `Shutdown` control message resolved the connection: begin a clean exit.
    Shutdown,
    /// The shared `run` flag was cleared (the lifecycle ended for another reason); stop accepting.
    RunCleared,
    /// `CONTROL_ERROR_CEILING` consecutive accept/read errors occurred: the listener is presumed
    /// unrecoverable, so the loop STOPS rather than busy-spinning forever.
    ErrorCeiling {
        /// The consecutive-error count that tripped the ceiling.
        consecutive_errors: u32,
    },
}

/// The control accept loop, factored out of `run_watcher` so the error/backoff behavior is unit-testable
/// without a real socket (see the `#[cfg(test)]` `control_loop_*` tests). It repeatedly calls
/// `serve_once` (which accepts one connection and serves it) while `run` is set:
///
/// - `Ok(Shutdown)` => exit via [`ControlLoopExit::Shutdown`].
/// - `Ok(Continue)` => the peer disconnected without a Shutdown; loop and accept again. Resets the
///   consecutive-error counter.
/// - `Err(_)` => an accept/read error must NOT crash the watcher AND must NOT busy-spin (the bug this
///   guards): sleep `backoff` (which also bounds the retry rate and the log rate), re-check `run`, and
///   retry. After `error_ceiling` consecutive errors the loop gives up via
///   [`ControlLoopExit::ErrorCeiling`] instead of spinning forever.
///
/// The `run` flag is re-checked after every error sleep so a clean shutdown still breaks promptly.
fn run_control_loop<S, E>(
    serve_once: &mut S,
    run: &AtomicBool,
    backoff: Duration,
    error_ceiling: u32,
    on_exit: E,
) where
    S: FnMut() -> std::io::Result<ControlOutcome>,
    E: FnOnce(ControlLoopExit),
{
    let mut consecutive_errors: u32 = 0;
    let reason = loop {
        if !run.load(Ordering::SeqCst) {
            break ControlLoopExit::RunCleared;
        }
        match serve_once() {
            Ok(ControlOutcome::Shutdown) => break ControlLoopExit::Shutdown,
            Ok(ControlOutcome::Continue) => {
                // Peer disconnected without a Shutdown; a successful accept clears the error streak.
                consecutive_errors = 0;
                continue;
            }
            Err(err) => {
                consecutive_errors += 1;
                // BOUNDED backoff: never busy-spin on a persistent accept fault (CPU/log hog while
                // Handshake is up — HBR-QUIET). Re-check `run` after sleeping so a clean shutdown breaks.
                tracing::warn!(
                    %err,
                    consecutive_errors,
                    "control connection error; backing off before retry"
                );
                if consecutive_errors >= error_ceiling {
                    break ControlLoopExit::ErrorCeiling { consecutive_errors };
                }
                std::thread::sleep(backoff);
                if !run.load(Ordering::SeqCst) {
                    break ControlLoopExit::RunCleared;
                }
                continue;
            }
        }
    };
    on_exit(reason);
}

/// Write the survivor record as a JSON sibling of the ring file: `<ring_dir>/palmistry-survivor-<session>.json`.
/// A numeric/typed record only (no project content). MT-093 reads + forwards it.
fn persist_survivor_record(
    ring_path: &str,
    session_id: &str,
    record: &lifecycle::SurvivorRecord,
) -> std::io::Result<()> {
    let ring = Path::new(ring_path);
    let dir = ring.parent().unwrap_or_else(|| Path::new("."));
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
    let out = dir.join(format!("palmistry-survivor-{safe}.json"));
    let json = serde_json::to_string_pretty(record).map_err(std::io::Error::other)?;
    std::fs::write(&out, json)?;
    tracing::info!(path = %out.display(), "survivor record persisted");
    Ok(())
}

#[cfg(test)]
mod control_loop_tests {
    //! Regression coverage for the control-thread accept-error path (the perf-lens must-fix): a
    //! persistent accept error must NOT busy-spin the control thread (no sleep => 100% CPU + unbounded
    //! log flood while Handshake is up). These tests drive `run_control_loop` directly with a fake
    //! `serve_once` so the error/backoff/ceiling behavior is provable without a real socket.

    use super::*;
    use std::time::Instant;

    /// A persistent accept error must (a) NOT spin unboundedly — it sleeps the bounded backoff between
    /// retries, and (b) eventually give up at the consecutive-error ceiling instead of looping forever.
    /// Without the fix this loop never sleeps and never breaks, so this test would hang / spin.
    #[test]
    fn persistent_accept_error_backs_off_and_hits_ceiling_instead_of_spinning() {
        let run = AtomicBool::new(true);
        let calls = std::cell::Cell::new(0u32);
        let mut serve_once = || -> std::io::Result<ControlOutcome> {
            calls.set(calls.get() + 1);
            Err(std::io::Error::other("persistent accept fault"))
        };

        let ceiling: u32 = 8;
        let backoff = Duration::from_millis(5);
        let exit = std::cell::Cell::new(None);
        let start = Instant::now();
        run_control_loop(&mut serve_once, &run, backoff, ceiling, |reason| {
            exit.set(Some(reason));
        });
        let elapsed = start.elapsed();

        // It STOPPED (did not spin forever) and reported the ceiling.
        assert_eq!(
            exit.get(),
            Some(ControlLoopExit::ErrorCeiling {
                consecutive_errors: ceiling
            }),
            "a persistent accept error must escalate to the ceiling, not spin forever"
        );
        // It called serve_once exactly `ceiling` times (one per consecutive error), proving a bounded
        // retry count rather than an unbounded busy-spin.
        assert_eq!(calls.get(), ceiling, "exactly one retry per consecutive error up to the ceiling");
        // It actually SLEPT between retries: with (ceiling - 1) backoff sleeps of `backoff` each, the
        // wall-clock must be at least roughly that. This is the anti-busy-spin guard: a no-sleep loop
        // would finish in microseconds. (The last error trips the ceiling BEFORE sleeping.)
        let min_expected = backoff * (ceiling - 1);
        assert!(
            elapsed >= min_expected,
            "the error path must back off (sleep) between retries — elapsed {elapsed:?} < expected \
             >= {min_expected:?}; a busy-spin would return near-instantly"
        );
    }

    /// A clean shutdown (the `run` flag cleared by the lifecycle) must break the loop PROMPTLY even while
    /// accept errors are occurring — the backoff sleep re-checks `run`, so teardown is not blocked.
    #[test]
    fn run_cleared_during_error_backoff_breaks_promptly() {
        let run = Arc::new(AtomicBool::new(true));
        let run_in_loop = Arc::clone(&run);
        // Clear `run` on the 3rd call so the loop observes it after the backoff sleep and breaks.
        let calls = std::cell::Cell::new(0u32);
        let mut serve_once = || -> std::io::Result<ControlOutcome> {
            let n = calls.get() + 1;
            calls.set(n);
            if n >= 3 {
                run_in_loop.store(false, Ordering::SeqCst);
            }
            Err(std::io::Error::other("transient accept fault"))
        };

        let exit = std::cell::Cell::new(None);
        // A high ceiling so the RunCleared path is what ends the loop, not the ceiling.
        run_control_loop(
            &mut serve_once,
            &run,
            Duration::from_millis(1),
            10_000,
            |reason| exit.set(Some(reason)),
        );

        assert_eq!(
            exit.get(),
            Some(ControlLoopExit::RunCleared),
            "a cleared run flag must break the error-backoff loop promptly (clean shutdown)"
        );
        assert!(calls.get() <= 4, "the loop must stop within a retry of the run flag clearing");
    }

    /// A `Shutdown` outcome exits the loop with `Shutdown`, and a `Continue` (peer disconnect) resets the
    /// consecutive-error streak so an intermittent error does not creep toward the ceiling.
    #[test]
    fn shutdown_exits_and_success_resets_error_streak() {
        let run = AtomicBool::new(true);
        // Sequence: Err, Err, Continue (resets streak), Err, Shutdown.
        let step = std::cell::Cell::new(0u32);
        let mut serve_once = || -> std::io::Result<ControlOutcome> {
            let s = step.get();
            step.set(s + 1);
            match s {
                0 | 1 | 3 => Err(std::io::Error::other("blip")),
                2 => Ok(ControlOutcome::Continue),
                _ => Ok(ControlOutcome::Shutdown),
            }
        };

        let exit = std::cell::Cell::new(None);
        // Ceiling of 3: if the Continue did NOT reset the streak, the 2 pre-Continue errors + the 1
        // post-Continue error would trip the ceiling before the Shutdown. The reset is what lets Shutdown
        // win — proving the streak resets on success.
        run_control_loop(
            &mut serve_once,
            &run,
            Duration::from_millis(1),
            3,
            |reason| exit.set(Some(reason)),
        );

        assert_eq!(
            exit.get(),
            Some(ControlLoopExit::Shutdown),
            "a successful Continue must reset the error streak so a later Shutdown still wins"
        );
    }
}
