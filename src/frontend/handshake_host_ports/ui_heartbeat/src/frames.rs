//! Frames the host wants and has not presented (MT-215 HB-4).
//!
//! A dispatched wake proves the loop runs; it cannot tell a loop that wants frames and never presents one. So a host
//! that presents gives the port what it knows about its frames, and while one has waited [`crate::PRESENT_STALL`] the
//! port withholds its beats: the watcher sees a stale heartbeat from a window that still answers, which it reports as
//! suspected, not as a confirmed freeze. The stall is recorded once as a typed event, and once more when it ends.

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
