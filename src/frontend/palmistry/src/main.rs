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

use cli::PalmistryConfig;
use control::{ControlOutcome, ControlServer};
use lifecycle::{
    build_survivor_record, run_lifecycle, spawn_parent_watch, LifecycleConfig, LifecycleState,
    ParentWatch,
};

/// Startup-error exit code (refused partial start / could not bind control socket).
const EXIT_CONFIG_ERROR: i32 = 2;

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
/// behavior; the integration tests override `parent_watch_factory` to inject the real Windows watch
/// against a test-spawned dummy parent (they use the default), and override `ready_signal` so the test
/// can observe when the watcher is fully started + staying alive.
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
        while control_run.load(Ordering::SeqCst) {
            let mut on_msg = |msg: &control::ControlMessage| {
                tracing::debug!(?msg, "control message");
                if matches!(msg, control::ControlMessage::Shutdown) {
                    control_state.request_shutdown();
                }
            };
            match server.serve_connection(&mut on_msg) {
                Ok(ControlOutcome::Shutdown) => {
                    control_state.request_shutdown();
                    break;
                }
                Ok(ControlOutcome::Continue) => {
                    // Peer disconnected without a Shutdown; loop to accept the next connection unless the
                    // lifecycle has since ended.
                    continue;
                }
                Err(err) => {
                    // An accept/read error must NOT crash the watcher; log + retry while still running.
                    tracing::warn!(%err, "control connection error; continuing to watch");
                    if !control_run.load(Ordering::SeqCst) {
                        break;
                    }
                    continue;
                }
            }
        }
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
