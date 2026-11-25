//! The UI-thread heartbeat port (WP-KERNEL-012 MT-215 S1).
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
//! blocked, deadlocked or stuck in a handler, and the loop (the outer one or a nested modal one) was dispatching.
//!
//! No other thread can supply that proof through this port. [`UiHeartbeat`] is neither `Send` nor `Sync`, so another
//! thread cannot publish through it, and [`UiHeartbeat::start`] refuses a thread that does not dispatch the loop. Wakes
//! that another thread posts publish nothing until the UI thread dispatches one, and then one beat per wake the port
//! asked for, not per post. Input, redraws and other events publish nothing either.
//!
//! # What a beat does not prove
//!
//! - that the loop was prompt: gaps shorter than the watcher's threshold are the watcher's to judge, and
//!   [`HeartbeatCounters`] records them;
//! - that anything was drawn or presented, or that the renderer or the GPU is healthy;
//! - that input reaches the application: a nested modal loop dispatches wakes while the rest of the host waits;
//! - that other threads or the backend are alive;
//! - which process this is. That stays bound by the ring path, the version 2 ring's launch nonce and the process id
//!   and birth time the watcher was launched with; a beat carries none of them;
//! - that the host kept its side: a host that calls [`UiHeartbeat::on_wake`] from a handler which did not come out of
//!   the loop, or writes the ring's heartbeat slot past this port, forges it. Blocking the real loop shows that.
//!
//! # How a host uses it
//!
//! 1. Implement [`EventLoopWake`] over the loop. Create [`UiHeartbeat`] on the UI thread with a [`ProcessClock`], that
//!    loop and the sink: the diagnostics recorder ([`RecorderHeartbeat`]), or a ring writer the host owns.
//! 2. Call [`UiHeartbeat::start`] as soon as the loop can take a wake. A new ring reads as counter 0, so the watcher's
//!    threshold already runs from its first poll.
//! 3. When the loop dispatches a wake, call [`UiHeartbeat::on_wake`] first, before the work of that turn. For any other
//!    event call [`UiHeartbeat::on_loop_turn`], which recovers lost wakes and publishes nothing.
//! 4. On the operating system's resume notification, call [`UiHeartbeat::on_system_resume`].
//! 5. On exit, end the watcher while the loop still runs, or have its acknowledgement within its threshold of the last
//!    beat; then [`UiHeartbeat::stop`] (or [`UiHeartbeat::into_sink`]), and drop the ring writer last.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What happens |
//! | --- | --- | --- |
//! | [`StartRefused::NotLoopThread`] | started from the wrong thread | nothing is published; start it on the UI thread |
//! | [`WakeRefused`] | the loop did not take a wake | the beat goes stale until a loop turn gets one taken |
//! | [`IgnoredWake`] | a late, duplicate or unknown wake | nothing is published |
//! | [`HeartbeatCounters::overdue_beats`] | the loop was late but came back | recorded; the watcher applies its own threshold |
//!
//! A wake that was lost is replaced by the next loop turn. A loop that is idle and has lost its wake stays stale: the
//! watcher then suspects a freeze but does not confirm one, because the window still answers.
//!
//! Frames, GUI toolkits, windows and graphics are no part of this crate's interface or closure.

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
mod port;
mod sink;
mod wake;

pub use clock::{MonotonicClock, ProcessClock};
pub use handshake_diag_ring::Heartbeat;
pub use port::{HeartbeatCounters, IgnoredWake, StartRefused, UiHeartbeat, WakeOutcome};
pub use sink::{HeartbeatSink, RecorderHeartbeat};
pub use wake::{EventLoopWake, WakeRefused, WakeTicket};

use std::time::Duration;

/// The longest time between two beats of a loop that is being serviced. The watcher's idle cadence.
pub const MAX_BEAT_GAP: Duration = Duration::from_millis(250);

/// How long after a beat the next wake is due. What is left of [`MAX_BEAT_GAP`] covers timer granularity and the time
/// the loop needs to get to it.
pub const WAKE_DELAY: Duration = Duration::from_millis(200);

/// How long past due a wake may be before a loop turn asks for a replacement.
pub const LOST_WAKE_GRACE: Duration = Duration::from_millis(500);

/// How long after a refused wake request a loop turn asks again.
pub const REFUSED_WAKE_RETRY: Duration = Duration::from_millis(50);

const _: () = assert!(WAKE_DELAY.as_millis() < MAX_BEAT_GAP.as_millis());
const _: () = assert!(REFUSED_WAKE_RETRY.as_millis() < LOST_WAKE_GRACE.as_millis());
