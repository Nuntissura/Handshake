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
use std::mem::ManuallyDrop;

use palmistry_launch::{PalmistryHandle, ShutdownAcknowledgement, ShutdownOutcome};

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
            if matches!(done.acknowledgement, ShutdownAcknowledgement::SignedConfirmed) && done.success() =>
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
