//! The shell's side of the Palmistry watcher (MT-214 S7, AC214-11; the toolkit-free launcher is MT-215's AC215-11).
//!
//! This part is the adapter: [`PalmistryWatcher`] answers the program's [`ShutdownWatcher`] question over the watcher the
//! launcher started. The launcher (`palmistry_launch`) stays where it is; the shell only asks it to shut the watcher down
//! and reads what it reports.
//!
//! # What the watcher is told
//!
//! Only a settled shutdown of a run with no typed failure reaches [`PalmistryWatcher::acknowledge_shutdown`] (the program
//! asks after the owner reported every claim settled and every window gone and no refused window, refused heartbeat or
//! failed loop ended the run, `crate::may_acknowledge`). Anything else never asks, and an adapter that is
//! dropped without having been asked does not tell the watcher either: the launcher's handle sends a clean Shutdown when it
//! is dropped, which for an unsettled end, a failed loop or a panic unwinding through the program would say "closed
//! cleanly" about a process that did not. So the adapter holds the watcher in a [`ManuallyDrop`] and lets go of it without
//! dropping it. The watcher then sees its parent end without a Shutdown and records that end (the post-mortem record of
//! Palmistry's lifecycle). What leaks with it is the child process handle and the pinned image file handle, for the
//! few moments until this process ends.
//!
//! # What the answer means
//!
//! [`watcher_ack`] is the one mapping from the launcher's [`ShutdownOutcome`] to [`WatcherAck`]: only a clean owned exit
//! whose signed Shutdown was confirmed and whose status is a success is [`WatcherAck::Acknowledged`]. A watcher whose signed
//! handshake was never confirmed is shut down and reaped like any other, and is reported [`WatcherAck::NotConnected`]: no
//! confirmed watcher was there to acknowledge, and the watcher is supplementary (a missing one never fails the close).
//! The call can block, bounded by the launcher's own shutdown wait (5 s plus 1 s for its backstop), on the loop's thread
//! at `LoopEnding`, after the heartbeat stopped.

use std::cell::RefCell;
use std::io;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};

use handshake_diag_ring::ring::DiagRingWriterV2;
use handshake_diag_ring::schema::DiagEventV2;
use handshake_diag_ring::{
    default_backing_path, DiagEvent, DiagEventCode, DiagPhase, DiagSeverity, DEFAULT_CAPACITY,
};
use handshake_runtime_instance::{
    acquire_embedded_runtime_instance_lease, resolve_embedded_runtime_host_scope,
    EmbeddedRuntimeInstanceLease,
};
use palmistry_launch::{
    control_socket_name, launch_palmistry_signed_at, resolve_palmistry_exe, LaunchSession,
    PalmistryHandle, ShutdownAcknowledgement, ShutdownOutcome,
};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::shutdown::{ShutdownWatcher, WatcherAck};

/// A watcher the shell owns and can shut down once. [`PalmistryHandle`] is the product's; tests give a fake. Not a
/// contract of the shell: it exists so the adapter can be proven without starting a process.
pub trait OwnedWatcher {
    /// Tell the watcher this process closed cleanly and wait, boundedly, for its exit. Consumes the watcher.
    fn shut_down(self: Box<Self>) -> ShutdownOutcome;
}

impl OwnedWatcher for PalmistryHandle {
    fn shut_down(self: Box<Self>) -> ShutdownOutcome {
        (*self).shutdown()
    }
}

/// What the launcher's report of a shutdown means to the program. Fails closed: anything but a confirmed, successful,
/// owned exit is [`WatcherAck::NotAcknowledged`].
pub fn watcher_ack(outcome: &ShutdownOutcome) -> WatcherAck {
    match outcome {
        ShutdownOutcome::ExitedCleanly(done)
            if matches!(
                done.acknowledgement,
                ShutdownAcknowledgement::SignedConfirmed
            ) && done.success() =>
        {
            WatcherAck::Acknowledged
        }
        _ => WatcherAck::NotAcknowledged,
    }
}

/// The program's [`ShutdownWatcher`] over the watcher the launcher started, or over none.
pub struct PalmistryWatcher {
    /// Never dropped: see the module documentation. Taken once, by the one question.
    owned: RefCell<Option<ManuallyDrop<Box<dyn OwnedWatcher>>>>,
    /// Whether the signed handshake was confirmed. An unconfirmed watcher is shut down and reported not connected.
    confirmed: bool,
}

impl PalmistryWatcher {
    /// No watcher: the launch was not made or degraded. The answer is [`WatcherAck::NotConnected`].
    pub fn none() -> Self {
        Self {
            owned: RefCell::new(None),
            confirmed: false,
        }
    }

    /// Over the launcher's handle. Whether the handshake was confirmed is the handle's own report.
    pub fn from_handle(handle: PalmistryHandle) -> Self {
        let confirmed = handle.handshake_acked();
        Self::over(Box::new(handle), confirmed)
    }

    /// Over any owned watcher, with the confirmation it reports. For tests with a fake; the product uses
    /// [`PalmistryWatcher::from_handle`].
    pub fn over(watcher: Box<dyn OwnedWatcher>, confirmed: bool) -> Self {
        Self {
            owned: RefCell::new(Some(ManuallyDrop::new(watcher))),
            confirmed,
        }
    }

    /// Whether a watcher is held and not yet asked.
    pub fn holds_watcher(&self) -> bool {
        self.owned.borrow().is_some()
    }

    /// Whether the watcher's signed handshake was confirmed (the handle's own report, taken at construction).
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }
}

impl ShutdownWatcher for PalmistryWatcher {
    fn acknowledge_shutdown(&self) -> WatcherAck {
        let Some(watcher) = self.owned.borrow_mut().take() else {
            return WatcherAck::NotConnected;
        };
        let outcome = ManuallyDrop::into_inner(watcher).shut_down();
        if self.confirmed {
            watcher_ack(&outcome)
        } else {
            WatcherAck::NotConnected
        }
    }
}

/// Why there is no watcher. Each is a stable code for the state line and the ring marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Degrade {
    /// `HANDSHAKE_HOST_SCOPE_ID` is not set (or is not usable): the signed launch needs the lease it names.
    NoHostScope,
    /// The loopback lease could not be taken.
    NoLease,
    /// The version 2 ring could not be made.
    NoRing,
    /// The watcher's exe could not be resolved (no override file, no sibling `palmistry.exe`).
    ExeNotFound,
    /// The signed launch failed before a handle existed: the exe could not be started, or an input was refused.
    SpawnFailed,
}

impl Degrade {
    /// A stable code, for diagnostics.
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoHostScope => "no_host_scope",
            Self::NoLease => "no_lease",
            Self::NoRing => "no_ring",
            Self::ExeNotFound => "exe_not_found",
            Self::SpawnFailed => "spawn_failed",
        }
    }
}

/// Everything the signed launch is made of, built once and read by both the ring and the launch, so the two cannot
/// disagree: the launcher's own session record (session id and ring path, with no marker sink: the ring writer belongs to
/// the heartbeat and a function pointer cannot reach it), the control socket, the process instance, the one launch nonce,
/// the signing seed and the launch-local transport key.
pub struct LaunchInputs {
    /// The launcher's own session record: `session_id` is the session uuid, the lease's instance id and the ring's owner.
    pub session: LaunchSession,
    /// The watcher's control socket, derived from the session id by the launcher's rule.
    pub control_socket: String,
    /// The process instance id: not the session id (the signed launch refuses equal ids).
    pub process_instance_id: Uuid,
    /// The ring's launch nonce: the value the ring is made with and the watcher is told with `--ring-nonce`.
    pub launch_nonce: [u8; 16],
    seed: Zeroizing<[u8; 32]>,
    /// A launch-local, non-zero transport key. No secret half is held: the shell never adopts the watcher (MT-199 does).
    pub transport_public_key: [u8; 32],
}

fn non_zero<const N: usize>() -> [u8; N] {
    loop {
        let bytes = rand::random::<[u8; N]>();
        if bytes != [0; N] {
            return bytes;
        }
    }
}

/// A process instance id that is not `session`: the next one `next` makes that differs. Two `Uuid::now_v7()` calls are
/// distinct in practice, and the signed launch refuses equal ids, so the shell never hands the launcher a pair it would
/// refuse (MT-214 S7 F5).
///
/// Bounded (MT-214 E9-F1 F8): after [`DISTINCT_TRIES`] equal answers it returns the session id with its low bit flipped,
/// which is never equal to it, instead of asking for ever.
fn distinct_from(session: Uuid, mut next: impl FnMut() -> Uuid) -> Uuid {
    for _ in 0..DISTINCT_TRIES {
        let id = next();
        if id != session {
            return id;
        }
    }
    Uuid::from_u128(session.as_u128() ^ 1)
}

/// How many times [`distinct_from`] asks before it derives an id itself.
const DISTINCT_TRIES: u32 = 8;

impl LaunchInputs {
    fn new(session: Uuid, ring_path: PathBuf) -> Self {
        let session_id = session.to_string();
        Self {
            control_socket: control_socket_name(&session_id),
            session: LaunchSession {
                session_id,
                ring_path,
                marker_sink: None,
            },
            process_instance_id: distinct_from(session, Uuid::now_v7),
            launch_nonce: non_zero::<16>(),
            seed: Zeroizing::new(non_zero::<32>()),
            transport_public_key: non_zero::<32>(),
        }
    }

    /// The signing seed, as the launcher takes it: a copy that is wiped when it is dropped.
    pub fn signing_seed(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(self.seed.to_vec())
    }
}

/// The path `--diag-ring` names in `arguments` (the program's arguments after its name), or `None` without the flag. Two
/// shapes name it: `--diag-ring path` and `--diag-ring=path`. A flag with no value, an empty one, or another flag in its
/// place is an error, never a path: `--diag-ring --seconds` must not make a ring file named `--seconds` (MT-214 S7 F4).
/// The flag given more than once, in either shape, is an error too: which ring the watcher reads must not depend on
/// which occurrence is read first (MT-214 E9-F1 F7).
///
/// # Errors
///
/// A message for the usage line when `--diag-ring` has no path, or is given more than once.
pub fn diag_ring_argument(arguments: &[String]) -> Result<Option<&Path>, &'static str> {
    const FLAG: &str = "--diag-ring";
    let named = |argument: &String| argument == FLAG || argument.starts_with("--diag-ring=");
    let Some(at) = arguments.iter().position(named) else {
        return Ok(None);
    };
    if arguments[at + 1..].iter().any(named) {
        return Err("--diag-ring given more than once");
    }
    let value = match arguments[at].strip_prefix("--diag-ring=") {
        Some(value) => Some(value),
        None => arguments.get(at + 1).map(String::as_str),
    };
    match value {
        Some(value) if !value.is_empty() && !value.starts_with("--") => Ok(Some(Path::new(value))),
        _ => Err("--diag-ring needs a path"),
    }
}

/// A fresh, non-zero launch nonce for a ring.
pub fn fresh_nonce() -> [u8; 16] {
    non_zero::<16>()
}

/// The one place the shell makes a diagnostics ring: the launcher's version 2 ring, with the launch nonce in its header.
pub fn make_ring(path: &Path, nonce: [u8; 16]) -> io::Result<DiagRingWriterV2> {
    DiagRingWriterV2::create(path, DEFAULT_CAPACITY, nonce)
}

/// What the session reads from outside. Tests give their own; the product uses [`Seams::production`].
pub struct Seams<'a> {
    /// The host scope id the lease is bound to (`HANDSHAKE_HOST_SCOPE_ID` in the product).
    pub host_scope: &'a dyn Fn() -> io::Result<String>,
    /// The watcher's exe (`resolve_palmistry_exe` in the product). A test gives a path that does not exist, so the
    /// launcher's error path is run and no process is started.
    pub resolve_exe: &'a dyn Fn() -> io::Result<PathBuf>,
}

fn production_host_scope() -> io::Result<String> {
    resolve_embedded_runtime_host_scope().map_err(io::Error::other)
}

impl Seams<'static> {
    /// The product's: the host scope comes from the environment, as for the egui host.
    pub fn production() -> Self {
        Self {
            host_scope: &production_host_scope,
            resolve_exe: &resolve_palmistry_exe,
        }
    }
}

/// The first half of a start: the ids, the lease and the ring. It holds them; nothing is launched.
pub struct Prepared {
    /// The launch inputs: one set.
    pub inputs: LaunchInputs,
    /// The lease, bound to the session id. Keep it in a named binding until the program has returned.
    pub lease: Option<EmbeddedRuntimeInstanceLease>,
    /// The ring, made with the inputs' nonce, if it could be made.
    pub ring: Option<DiagRingWriterV2>,
    /// Why the launch cannot go on, if it cannot.
    pub degrade: Option<Degrade>,
}

fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
        })
}

/// Records the Palmistry handshake in the ring: `Start` when confirmed (the counter is this process id), `Degraded`
/// otherwise. Returns whether the ring took it.
fn mark(ring: Option<&DiagRingWriterV2>, confirmed: bool) -> bool {
    let Some(ring) = ring else {
        return false;
    };
    let (phase, severity, counter) = if confirmed {
        (
            DiagPhase::Start,
            DiagSeverity::Info,
            u64::from(std::process::id()),
        )
    } else {
        (DiagPhase::Degraded, DiagSeverity::Warn, 0)
    };
    let marker = DiagEvent::generic(
        DiagEventCode::PalmistryHandshake,
        phase,
        severity,
        0,
        0,
        counter,
        0,
        0,
        now_nanos(),
    );
    DiagEventV2::ordinary(marker).is_some_and(|event| ring.write(event))
}

/// Takes the lease, then makes the ring, for one session. A refusal at either step is a [`Degrade`], not an error: the
/// watcher is supplementary and the program runs without it. A degraded start that has a ring marks it.
///
/// `ring_path` is the ring's file (`--diag-ring`); without it the launcher's default path for the session is used, so
/// that a launched watcher always has a ring to read.
pub fn prepare(ring_path: Option<&Path>, seams: &Seams<'_>) -> Prepared {
    let session = Uuid::now_v7();
    let ring_path = ring_path.map_or_else(
        || default_backing_path(&session.to_string()),
        Path::to_path_buf,
    );
    let inputs = LaunchInputs::new(session, ring_path);
    let mut degrade = None;
    let lease = match (seams.host_scope)() {
        Err(_) => {
            degrade = Some(Degrade::NoHostScope);
            None
        }
        Ok(host_scope) => match acquire_embedded_runtime_instance_lease(session, host_scope) {
            Ok(lease) => Some(lease),
            Err(_) => {
                degrade = Some(Degrade::NoLease);
                None
            }
        },
    };
    let ring = match make_ring(&inputs.session.ring_path, inputs.launch_nonce) {
        Ok(ring) => Some(ring),
        Err(_) => {
            degrade.get_or_insert(Degrade::NoRing);
            None
        }
    };
    if degrade.is_some() {
        mark(ring.as_ref(), false);
    }
    Prepared {
        inputs,
        lease,
        ring,
        degrade,
    }
}

/// How the start ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchState {
    /// The watcher runs and its signed handshake was confirmed.
    Launched,
    /// The watcher runs and is owned, but its signed handshake was not confirmed: it is shut down and reaped at the end like
    /// any other, and reported as not connected.
    HandshakeUnconfirmed,
    /// No watcher: the reason.
    Degraded(Degrade),
}

impl LaunchState {
    /// A stable name, for the state line.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Launched => "launched",
            Self::HandshakeUnconfirmed => "handshake_unconfirmed",
            Self::Degraded(_) => "degraded",
        }
    }

    /// A stable reason code, for the state line.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Launched => "confirmed",
            Self::HandshakeUnconfirmed => "handshake_unconfirmed",
            Self::Degraded(reason) => reason.code(),
        }
    }
}

/// The state of a launch that returned a handle: confirmed or not, as the handle reports it.
pub const fn classify(confirmed: bool) -> LaunchState {
    if confirmed {
        LaunchState::Launched
    } else {
        LaunchState::HandshakeUnconfirmed
    }
}

/// A whole start: what the program is given and what the caller keeps until the program has returned.
pub struct Started {
    /// The ring, for the program's heartbeat.
    pub ring: Option<DiagRingWriterV2>,
    /// The program's [`ShutdownWatcher`]: over the launched watcher, or over none.
    pub watcher: PalmistryWatcher,
    /// The lease. Keep it in a named binding that lives until `run_program` has returned.
    pub lease: Option<EmbeddedRuntimeInstanceLease>,
    /// How the start ended.
    pub state: LaunchState,
    /// The kind of the error that ended a resolve or a launch, if one did.
    pub failure: Option<io::ErrorKind>,
    /// The launch inputs the ring and the launch were made from.
    pub inputs: LaunchInputs,
}

/// Starts the session: the lease, the ring, the watcher's exe and the signed launch, in that order. Any refusal leaves no
/// watcher, marks the ring `Degraded` and goes on: the program runs without a watcher.
pub fn start(ring_path: Option<&Path>) -> Started {
    start_with(ring_path, &Seams::production())
}

/// [`start`] with its outside reads given.
pub fn start_with(ring_path: Option<&Path>, seams: &Seams<'_>) -> Started {
    let prepared = prepare(ring_path, seams);
    let Prepared {
        inputs,
        lease,
        ring,
        degrade,
    } = prepared;
    let mut watcher = PalmistryWatcher::none();
    let mut failure = None;
    let state = match (degrade, lease.as_ref()) {
        (Some(reason), _) => LaunchState::Degraded(reason),
        (None, None) => LaunchState::Degraded(Degrade::NoLease),
        (None, Some(held)) => match (seams.resolve_exe)() {
            Err(error) => {
                failure = Some(error.kind());
                LaunchState::Degraded(Degrade::ExeNotFound)
            }
            Ok(exe) => match launch_palmistry_signed_at(
                &exe,
                &inputs.session,
                &inputs.session.ring_path,
                &inputs.control_socket,
                held,
                inputs.process_instance_id,
                inputs.launch_nonce,
                inputs.signing_seed(),
                inputs.transport_public_key,
            ) {
                Err(error) => {
                    failure = Some(error.kind());
                    LaunchState::Degraded(Degrade::SpawnFailed)
                }
                Ok(handle) => {
                    watcher = PalmistryWatcher::from_handle(handle);
                    classify(watcher.confirmed())
                }
            },
        },
    };
    if degrade.is_none() {
        mark(ring.as_ref(), state == LaunchState::Launched);
    }
    Started {
        ring,
        watcher,
        lease,
        state,
        failure,
        inputs,
    }
}

impl Started {
    /// The one state line the product binary and the example print after the start, before the program runs. It names the
    /// outcome and says what the watcher does not cover: it is watched from the first heartbeat on (a watcher with no
    /// heartbeat yet reads the process as healthy), so the device open and the window creation before the loop starts are
    /// not watched; and an ending that never reaches the acknowledgement (no device, a refused window, a failed loop, a
    /// refused heartbeat, an unsettled close, a panic) leaves the watcher untold, so it records the end as unexpected.
    pub fn state_line(&self) -> String {
        format!(
            "HSK_SHELL_PALMISTRY state={} reason={} session={} watched_from=first_heartbeat untold_on=device_unavailable,no_window,loop_failed,heartbeat_refused,shutdown_unsettled,panic",
            self.state.name(),
            self.state.reason(),
            self.inputs.session.session_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_owned()).collect()
    }

    #[test]
    fn diag_ring_takes_a_path_and_refuses_a_missing_one() {
        assert_eq!(diag_ring_argument(&arguments(&[])), Ok(None));
        assert_eq!(diag_ring_argument(&arguments(&["--seconds", "5"])), Ok(None));
        assert_eq!(
            diag_ring_argument(&arguments(&["--diag-ring", "ring.bin"])),
            Ok(Some(Path::new("ring.bin")))
        );
        assert_eq!(
            diag_ring_argument(&arguments(&["--seconds", "5", "--diag-ring", "r.bin", "--other"])),
            Ok(Some(Path::new("r.bin")))
        );
        let needs = Err("--diag-ring needs a path");
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring"])), needs);
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring", "--seconds", "5"])), needs);
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring", ""])), needs);
        assert_eq!(diag_ring_argument(&arguments(&["--seconds", "5", "--diag-ring"])), needs);
    }

    #[test]
    fn diag_ring_takes_the_equals_shape_and_refuses_a_repeated_flag_in_any_shape() {
        let needs = Err("--diag-ring needs a path");
        let twice = Err("--diag-ring given more than once");
        assert_eq!(
            diag_ring_argument(&arguments(&["--diag-ring=ring.bin"])),
            Ok(Some(Path::new("ring.bin")))
        );
        assert_eq!(
            diag_ring_argument(&arguments(&["--seconds", "5", "--diag-ring=a=b.bin", "--other"])),
            Ok(Some(Path::new("a=b.bin")))
        );
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring="])), needs);
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring=--seconds"])), needs);
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring", "a.bin", "--diag-ring", "b.bin"])), twice);
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring", "a.bin", "--diag-ring=b.bin"])), twice);
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring=a.bin", "--diag-ring", "b.bin"])), twice);
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring=a.bin", "--diag-ring=b.bin"])), twice);
        // A repeated flag without a value is a repeat, not a path.
        assert_eq!(diag_ring_argument(&arguments(&["--diag-ring", "--diag-ring"])), twice);
    }

    #[test]
    fn the_process_id_is_never_the_session_id() {
        let session = Uuid::now_v7();
        let other = Uuid::now_v7();
        let mut asked = 0;
        let id = distinct_from(session, || {
            asked += 1;
            if asked < 3 {
                session
            } else {
                other
            }
        });
        assert_eq!((id, asked), (other, 3), "an equal id is asked for again");
        // A source that only ever answers the session id ends after the bound, with an id that differs.
        let mut asked = 0;
        let derived = distinct_from(session, || {
            asked += 1;
            session
        });
        assert_eq!(asked, DISTINCT_TRIES);
        assert_ne!(derived, session);
        let inputs = LaunchInputs::new(session, PathBuf::from("ring.bin"));
        assert_ne!(inputs.process_instance_id, session);
        assert_ne!(inputs.process_instance_id.to_string(), inputs.session.session_id);
    }
}
