//! The shell's side of the Palmistry watcher (MT-214 S7, AC214-11; the toolkit-free launcher is MT-215's AC215-11).
//!
//! This part is the adapter: [`PalmistryWatcher`] answers the program's [`ShutdownWatcher`] question over the watcher the
//! launcher started. The launcher (`palmistry_launch`) stays where it is; the shell only asks it to shut the watcher down
//! and reads what it reports.
//!
//! # What the watcher is told
//!
//! Only a settled shutdown reaches [`PalmistryWatcher::acknowledge_shutdown`] (the program asks after the owner reported
//! every claim settled and every window gone, `crate::may_acknowledge`). Anything else never asks, and an adapter that is
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
    control_socket_name, LaunchSession, PalmistryHandle, ShutdownAcknowledgement, ShutdownOutcome,
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
}

impl Degrade {
    /// A stable code, for diagnostics.
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoHostScope => "no_host_scope",
            Self::NoLease => "no_lease",
            Self::NoRing => "no_ring",
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
            process_instance_id: Uuid::now_v7(),
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
}

fn production_host_scope() -> io::Result<String> {
    resolve_embedded_runtime_host_scope().map_err(io::Error::other)
}

impl Seams<'static> {
    /// The product's: the host scope comes from the environment, as for the egui host.
    pub fn production() -> Self {
        Self {
            host_scope: &production_host_scope,
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
