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
mod lifecycle;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cli::PalmistryConfig;
use control::{ControlOutcome, ControlServer};
use lifecycle::{
    build_survivor_record, run_lifecycle, spawn_parent_watch, LifecycleConfig, LifecycleState,
    ParentWatch,
};

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
    /// Lifecycle timing. Default = production timings.
    pub lifecycle: LifecycleConfig,
    /// Optional callback invoked once the watcher is fully started (control socket bound, parent watch
    /// armed) and about to enter its run loop. Used by tests to assert "started + staying alive".
    pub on_ready: Option<Box<dyn FnOnce() + Send>>,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            parent_watch_factory: Box::new(default_parent_watch),
            lifecycle: LifecycleConfig::default(),
            on_ready: None,
        }
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

/// Run the watcher to lifecycle completion. The library-shaped entrypoint (tested directly): it binds
/// the control socket, arms the parent watch, spawns the control thread + parent-watch thread, runs the
/// lifecycle, and persists the survivor record next to the ring. Returns `Err` only for a STARTUP
/// failure (bad ring path / control bind) — a normal lifecycle end returns `Ok(())`.
pub fn run_watcher(config: PalmistryConfig, options: RunOptions) -> std::io::Result<()> {
    // Validate the ring path is at least openable as a ring BEFORE committing to run, so a misconfigured
    // ring is a clear startup error rather than a silently broken watcher (the passive ring READ loop is
    // MT-090; here we only confirm the path resolves to a valid ring header).
    validate_ring_path(&config.ring_path)?;

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

    // Fully started: control socket bound, parent watch armed. Signal readiness (tests assert
    // started-and-staying-alive here) and enter the run loop.
    if let Some(cb) = options.on_ready {
        cb();
    }

    // Drive the lifecycle to its terminal reason. This BLOCKS until Shutdown or a recorded abnormal
    // parent death + finalize.
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

    // Tear down: `run` is already false (set by run_lifecycle). The parent-watch thread observes it and
    // returns; the control thread is woken by `run` going false on its next accept boundary. We join the
    // watch thread (bounded) and detach the control thread (its blocking accept may outlive us harmlessly
    // — the process is exiting).
    let _ = watch_handle.join();
    drop(control_handle); // detached; the process exit closes the socket.

    Ok(())
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

/// Confirm `ring_path` resolves to a valid MT-081 ring (header validates). Reusing the ring crate's own
/// `DiagRingReader::open` is the honest validation — it checks magic/version/record_size/capacity and
/// refuses a foreign/garbage map. We immediately drop the reader; the passive read loop is MT-090.
fn validate_ring_path(ring_path: &str) -> std::io::Result<()> {
    let path = Path::new(ring_path);
    match handshake_diag_ring::DiagRingReader::open(path) {
        Ok(_reader) => Ok(()),
        Err(err) => Err(std::io::Error::new(
            err.kind(),
            format!("ring path '{ring_path}' is not a valid diagnostic ring: {err}"),
        )),
    }
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
