//! Frames the host wants and has not presented (MT-215 HB-4).
//!
//! A dispatched wake proves the loop runs; it cannot tell a loop that wants frames and never presents one. So a host
//! that presents gives the port what it knows about its frames, and once one has waited [`crate::PRESENT_STALL`] the
//! port records a typed stall event in the ring, and another when the stall ends. Its beats go on meanwhile (AC215-6):
//! the heartbeat says whether the UI thread is serviced, the stall event whether it presents, and a watcher reads the
//! two apart. A stale heartbeat from a window that does not answer is a frozen UI thread; a fresh heartbeat beside a
//! stall event is a stalled present.

use std::time::Duration;

/// The host's frames, as the heartbeat sees them. Read on the UI thread at every dispatched wake of the port, so it
/// must not wait, allocate or panic.
pub trait FrameWait {
    /// How long the frame that has waited longest has been wanted without being presented. `None` when no frame is
    /// wanted, or none can be shown (hidden, minimised, no size).
    fn longest_wait(&self) -> Option<Duration>;
}

/// A host whose frames the port does not watch. Its beats depend on dispatched wakes alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoFrames;

impl FrameWait for NoFrames {
    fn longest_wait(&self) -> Option<Duration> {
        None
    }
}
