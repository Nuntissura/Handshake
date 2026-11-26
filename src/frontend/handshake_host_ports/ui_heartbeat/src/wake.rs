//! Wakes: how the port asks a host's event loop to prove it is being serviced.

use std::time::Duration;

/// One requested wake. Minted by the port, carried by the host's loop, handed back when the loop dispatches it.
///
/// A ticket may cross threads (a timer thread may post it), because holding one publishes nothing: only
/// [`crate::UiHeartbeat::on_wake`] does, and that takes the loop's own proof of a dispatch, not a ticket.
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

    /// Rebuild a ticket from [`WakeTicket::to_raw`], inside [`EventLoopWake::dispatched_ticket`]. A number the port
    /// never requested is ignored on dispatch.
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

/// Why a loop did not take a wake request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeRefused {
    /// The loop has ended; no wake will ever be dispatched. The port stops for good and asks no more (MT-215 F4).
    LoopClosed,
    /// The loop could not take it now (for example a full message queue). The port asks again on a later turn.
    Unavailable,
}

/// The host's event loop, as the port sees it.
///
/// # What an implementation must do
///
/// - Dispatch the ticket **from the event loop, on the loop's own thread**, the way it dispatches any other event,
///   and hand the host a [`EventLoopWake::Dispatched`] for it, which the host passes to
///   [`crate::UiHeartbeat::on_wake`]. Prefer a type that only the loop can make: then no handler can report a
///   dispatch that did not happen (MT-215 HB-7).
/// - **Deliver an accepted wake** (MT-215 F1). Once `request_wake` returned `Ok`, the wake is dispatched unless it is
///   withdrawn, replaced or the loop ends. A post that fails later (a full queue) or that another loop takes out and
///   throws away is posted again until the loop dispatches the wake, at intervals that grow, and no more than a few
///   posts at a time while the loop takes nothing out of its queue. Nothing else guarantees it: an idle loop has no
///   turn on which the port could notice the loss, and a long handler after it would read as a confirmed freeze.
/// - **Hold at most one wake** (MT-215 F4): a request replaces one that has not been dispatched.
/// - Never dispatch inside `request_wake`. The port ignores such a call.
/// - Not block and not allocate per request in `request_wake`.
/// - Release any operating-system timer it holds when it is dropped.
///
/// # On Windows
///
/// - Address the wake to a window, not to the thread: a nested modal loop (window move or resize, menu, native dialog)
///   dispatches window messages, but a message posted without a window reaches only a loop that looks for it.
/// - `WM_TIMER` is delivered only when the queue holds nothing else, so a timer message alone can starve under a
///   flood of other messages. A posted message is first in, first out.
/// - A pool timer that posts once is not enough: `PostMessage` fails when the queue is full, after `request_wake` has
///   returned. Post again until dispatched, as the shell's wake lanes do (`handshake_shell_window::WakeLane`).
pub trait EventLoopWake {
    /// What the loop gives the host when it dispatches a wake: its proof that it did.
    type Dispatched<'a>;

    /// Whether the calling thread is the one that dispatches this loop's events. On Windows: the thread that owns the
    /// window the watcher probes.
    fn on_loop_thread(&self) -> bool;

    /// Have the loop dispatch `ticket` once `delay` has passed, replacing a wake not dispatched yet.
    ///
    /// # Errors
    ///
    /// [`WakeRefused`] when the loop will not dispatch it.
    fn request_wake(&self, ticket: WakeTicket, delay: Duration) -> Result<(), WakeRefused>;

    /// Withdraw wakes that have not been dispatched, where the loop can. A wake that still arrives is ignored.
    fn cancel_wakes(&self);

    /// The ticket `wake` carries, when it is a wake this loop dispatched for the port; `None` for any other (a wake of
    /// another source of the same loop, for example).
    fn dispatched_ticket(&self, wake: &Self::Dispatched<'_>) -> Option<WakeTicket>;
}
