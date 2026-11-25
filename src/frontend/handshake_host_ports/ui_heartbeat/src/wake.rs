//! Wakes: how the port asks a host's event loop to prove it is being serviced.

use std::time::Duration;

/// One requested wake. Minted by the port, carried by the host's loop, handed back when the loop dispatches it.
///
/// A ticket may cross threads (a timer thread may post it), because holding one publishes nothing: only
/// [`crate::UiHeartbeat::on_wake`] does, and that cannot leave the UI thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WakeTicket(u64);

impl WakeTicket {
    pub(crate) fn new(number: u64) -> Self {
        Self(number)
    }

    pub(crate) fn number(self) -> u64 {
        self.0
    }

    /// The ticket as a plain integer, for a loop whose messages carry integers.
    pub fn to_raw(self) -> u64 {
        self.0
    }

    /// Rebuild a ticket from [`WakeTicket::to_raw`]. A number the port never requested is ignored on dispatch.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// Why a loop did not take a wake request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeRefused {
    /// The loop has ended; no wake will ever be dispatched.
    LoopClosed,
    /// The loop could not take it now (for example a full message queue). The port asks again on a later turn.
    Unavailable,
}

/// The host's event loop, as the port sees it.
///
/// # What an implementation must do
///
/// - Dispatch the ticket **from the event loop, on the loop's own thread**, the way it dispatches any other event,
///   and then call [`crate::UiHeartbeat::on_wake`] with it. Calling `on_wake` from a handler that did not come out of
///   the loop would report a liveness that was not observed.
/// - Never dispatch inside `request_wake`. The port ignores such a call.
/// - Not block and not allocate per request in `request_wake`.
/// - Release any operating-system timer it holds when it is dropped.
///
/// # On Windows
///
/// - Address the wake to a window, not to the thread: a nested modal loop (window move or resize, menu, native dialog)
///   dispatches window messages, but a message posted without a window reaches only a loop that looks for it.
/// - `WM_TIMER` is delivered only when the queue holds nothing else, so a timer message alone can starve under a
///   flood of other messages. A posted message is first in, first out. A pool timer that posts one is a valid wake:
///   the posting thread still cannot publish.
/// - `PostMessage` fails when the queue is full: report [`WakeRefused::Unavailable`].
pub trait EventLoopWake {
    /// Whether the calling thread is the one that dispatches this loop's events. On Windows: the thread that owns the
    /// window the watcher probes.
    fn on_loop_thread(&self) -> bool;

    /// Have the loop dispatch `ticket` once `delay` has passed.
    ///
    /// # Errors
    ///
    /// [`WakeRefused`] when the loop will not dispatch it.
    fn request_wake(&self, ticket: WakeTicket, delay: Duration) -> Result<(), WakeRefused>;

    /// Withdraw wakes that have not been dispatched, where the loop can. A wake that still arrives is ignored.
    fn cancel_wakes(&self);
}
