//! The watcher's acknowledgement of an explicit shutdown (MT-214 AC214-11).
//!
//! Palmistry's launcher, with its control socket and signed `Shutdown` message, lives in the egui host
//! (`handshake_native/src/diagnostics/palmistry_launch.rs`), which this shell may not depend on. The shell therefore
//! holds a seam: the program asks a [`ShutdownWatcher`] to acknowledge only after the owner reported every claim
//! settled and every window gone, never before. No toolkit-free launcher exists yet, so the binaries pass
//! [`NoWatcher`], which says so.

/// What the watcher answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WatcherAck {
    /// The watcher acknowledged the shutdown.
    Acknowledged,
    /// No watcher is connected to this process; nothing was asked.
    NotConnected,
    /// The watcher was asked and did not acknowledge within its bound.
    NotAcknowledged,
}

impl WatcherAck {
    /// A stable name, for diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Acknowledged => "acknowledged",
            Self::NotConnected => "not_connected",
            Self::NotAcknowledged => "not_acknowledged",
        }
    }
}

/// Acknowledges an explicit shutdown to the standalone watcher.
pub trait ShutdownWatcher {
    /// Tell the watcher this process closed cleanly, and wait for its answer within the watcher's own bound. Called
    /// once, after the shutdown settled.
    fn acknowledge_shutdown(&self) -> WatcherAck;
}

/// No watcher is connected: the launcher port does not exist yet outside the egui host.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoWatcher;

impl ShutdownWatcher for NoWatcher {
    fn acknowledge_shutdown(&self) -> WatcherAck {
        WatcherAck::NotConnected
    }
}
