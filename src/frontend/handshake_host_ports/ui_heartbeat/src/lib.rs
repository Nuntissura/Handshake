//! The UI-thread heartbeat port (WP-KERNEL-012 MT-215 S1, S2).
//!
//! The standalone Palmistry watcher decides that Handshake froze when the heartbeat counter in the diagnostics ring
//! stops changing for its threshold and the window does not answer the operating system (Master Spec 6.13.5). A
//! host that draws only what changed draws nothing while idle, so its heartbeat cannot come from frames. This crate
//! takes it from the event loop instead: the port asks the loop for a wake, and publishes one beat each time the loop
//! dispatches that wake on the UI thread.
//!
//! # What a beat proves
//!
//! When the counter has changed, then since the previous value the event loop of the UI thread took a wake out of its
//! own queue and ran the host's handler for it on that thread. So at that moment the thread was running, it was not
//! blocked, deadlocked or stuck in a handler, and the loop (the outer one or a nested modal one) was dispatching. With
//! a host that reports its frames ([`FrameWait`]), also: no frame had been wanted for [`PRESENT_STALL`] without a
//! present. While one has, the port withholds its beats (MT-215 HB-4), so a loop that lives and shows nothing looks
//! stale from a window that still answers, which the watcher reports as suspected, never as a confirmed freeze; the
//! stall is recorded once as a typed event when it begins and once when it ends ([`HeartbeatEvent`]).
//!
//! No other thread can supply that proof through this port. [`UiHeartbeat`] is neither `Send` nor `Sync`, so another
//! thread cannot publish through it, and [`UiHeartbeat::start`] refuses a thread that does not dispatch the loop.
//! [`UiHeartbeat::on_wake`] takes the loop's own proof of a dispatch, not a ticket ([`EventLoopWake::Dispatched`]):
//! with a loop whose proof only the loop can make, no handler can report a dispatch that did not happen (MT-215 HB-7).
//! Wakes that another thread posts publish nothing until the UI thread dispatches one, and then one beat per wake the
//! port asked for, not per post. Input, redraws and other events publish nothing either.
//!
//! # What a beat does not prove
//!
//! - that the loop was prompt: gaps shorter than the watcher's threshold are the watcher's to judge, and
//!   [`HeartbeatCounters`] records them;
//! - that the renderer or the GPU is healthy, beyond the frames the host reports;
//! - that input reaches the application: a nested modal loop dispatches wakes while the rest of the host waits;
//! - that other threads or the backend are alive;
//! - which process this is. That stays bound by the ring path, the version 2 ring's launch nonce and the process id
//!   and birth time the watcher was launched with; a beat carries none of them;
//! - that the host kept its side: a host whose loop hands out a proof anybody can make (a bare ticket), or that writes
//!   the ring's heartbeat slot past this port, forges it. Blocking the real loop shows that.
//!
//! # How a host uses it
//!
//! 1. Implement [`EventLoopWake`] over the loop, keeping its contract: an accepted wake is delivered, posted again
//!    until dispatched (MT-215 F1), and a newer request replaces one not dispatched (F4). The kernel shell's adapter
//!    does this over its wake lanes (`handshake_shell::heartbeat`).
//! 2. Create [`UiHeartbeat`] on the UI thread with a [`ProcessClock`], that loop, the sink and, for a host that
//!    presents, its frames ([`UiHeartbeat::with_frames`]). The sink is the diagnostics recorder ([`RecorderHeartbeat`],
//!    which writes nothing until its ring writer is installed, MT-215 F2), or a ring writer the host gives the port
//!    for good: one publisher per ring.
//! 3. Call [`UiHeartbeat::start`] as soon as the loop can take a wake. A new ring reads as counter 0, so the watcher's
//!    threshold already runs from its first poll.
//! 4. When the loop dispatches a wake, call [`UiHeartbeat::on_wake`] first, before the work of that turn. For any other
//!    event call [`UiHeartbeat::on_loop_turn`], which recovers refused wakes and publishes nothing.
//! 5. On the operating system's resume notification, call [`UiHeartbeat::on_system_resume`].
//! 6. On exit, end the watcher while the loop still runs, or have its acknowledgement within its threshold of the last
//!    beat; then [`UiHeartbeat::stop`] (or [`UiHeartbeat::into_sink`]), and drop the ring writer last.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What happens |
//! | --- | --- | --- |
//! | [`StartRefused::NotLoopThread`] | started from the wrong thread | nothing is published; start it on the UI thread |
//! | [`WakeRefused::Unavailable`] | the loop did not take a wake | the beat goes stale until a loop turn gets one taken |
//! | [`WakeRefused::LoopClosed`] | the loop has ended | the port stops; [`HeartbeatCounters::loop_closed`] |
//! | [`IgnoredWake`] | a late, duplicate, foreign or unknown wake | nothing is published |
//! | [`WakeOutcome::Withheld`] | frames wanted and none presented for [`PRESENT_STALL`] | no beat; one stall event |
//! | [`HeartbeatCounters::held_back_wakes`] | [`MAX_OUTSTANDING_WAKES`] outstanding | no more replacements; the loop broke its contract or takes nothing out |
//! | [`HeartbeatCounters::overdue_beats`] | the loop was late but came back | recorded; the watcher applies its own threshold |
//!
//! A loop that is idle and lost its wake (one that broke its contract) stays stale: the watcher then suspects a freeze
//! but does not confirm one, because the window still answers.
//!
//! GUI toolkits, windows and graphics are no part of this crate's interface or closure.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod clock;
mod events;
mod frames;
mod port;
mod sink;
mod wake;

pub use clock::{MonotonicClock, ProcessClock};
pub use events::HeartbeatEvent;
pub use frames::{FrameWait, NoFrames};
pub use handshake_diag_ring::Heartbeat;
pub use port::{HeartbeatCounters, IgnoredWake, StartRefused, UiHeartbeat, WakeOutcome};
pub use sink::{HeartbeatSink, RecorderHeartbeat};
pub use wake::{EventLoopWake, WakeRefused, WakeTicket};

use std::time::Duration;

/// The longest time between two beats of a loop that is being serviced. The watcher's idle cadence.
pub const MAX_BEAT_GAP: Duration = Duration::from_millis(250);

/// How long after a beat the next wake is due. What is left of [`MAX_BEAT_GAP`] covers [`TIMER_GRANULARITY`] and a
/// handler of up to [`HANDLER_ALLOWANCE`] that runs when the wake comes due (MT-215 F3).
pub const WAKE_DELAY: Duration = Duration::from_millis(100);

/// The coarsest timer resolution a wake's delay may be rounded up by: Windows' default clock interrupt, 15.625 ms.
pub const TIMER_GRANULARITY: Duration = Duration::from_millis(16);

/// How long a handler that runs when the next wake comes due may take, and the beat still keeps [`MAX_BEAT_GAP`].
/// A longer one makes that beat overdue ([`HeartbeatCounters::overdue_beats`]).
pub const HANDLER_ALLOWANCE: Duration = Duration::from_millis(125);

/// How long past due a wake may be before a loop turn asks for a replacement.
pub const LOST_WAKE_GRACE: Duration = Duration::from_millis(500);

/// How long after a refused wake request a loop turn asks again.
pub const REFUSED_WAKE_RETRY: Duration = Duration::from_millis(50);

/// The most wakes the port leaves outstanding before it asks for no replacement (MT-215 F4). A loop that keeps its
/// contract holds one.
pub const MAX_OUTSTANDING_WAKES: u64 = 4;

/// How long a wanted frame may wait without a present before beats are withheld (MT-215 HB-4). Below the watcher's
/// 5 s threshold, so a present stall is suspected within about 7 s of the first frame that was not presented.
pub const PRESENT_STALL: Duration = Duration::from_secs(2);

const _: () = assert!(
    WAKE_DELAY
        .as_millis()
        .saturating_add(TIMER_GRANULARITY.as_millis())
        .saturating_add(HANDLER_ALLOWANCE.as_millis())
        <= MAX_BEAT_GAP.as_millis()
);
const _: () = assert!(REFUSED_WAKE_RETRY.as_millis() < LOST_WAKE_GRACE.as_millis());
const _: () = assert!(MAX_OUTSTANDING_WAKES >= 2);
const _: () = assert!(PRESENT_STALL.as_millis() > MAX_BEAT_GAP.as_millis().saturating_mul(4));
