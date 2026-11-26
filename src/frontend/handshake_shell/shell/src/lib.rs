//! The shell crate of the kernel-owned shell (WP-KERNEL-012 MT-214 AC214-12): composition of the shell's crates and
//! the toolkit-free host ports, with no widget, pane or domain logic of its own.
//!
//! So far it holds one composition, MT-215 slice 2: [`ShellHeartbeat`], the UI heartbeat port bound to the window
//! crate's event loop and the surface crate's present path, so that a frozen UI thread or a stalled present reaches the
//! diagnostics ring that the internal diagnostics and the standalone Palmistry watcher read. The module documentation
//! of [`heartbeat`] is its manual: how a host uses it, what it proves and what reads it.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What to do |
//! | --- | --- | --- |
//! | `ShellHeartbeat::open` refused | the loop has ended, or has no lane left | none: the window crate's refusal |
//! | `StartRefused::NotLoopThread` | started off the loop's thread | start it in a handler |
//! | `WakeOutcome::Withheld` | a watched window wanted a frame for 2 s and none was presented | the renderer's or the GPU's failure; Palmistry suspects, the ring holds a `SlowFrame` `Degraded` event |
//! | `WakeOutcome::Ignored` | a wake of the heartbeat's lane that was late, duplicate or not asked for | nothing |
//! | [`WatchRefused::Full`] | [`MAX_WATCHED_WINDOWS`] are watched | that window's presents are not watched |
//! | `HeartbeatCounters::loop_closed` | the loop ended | none |
//!
//! Windows x64 first; on other systems only the watches compile.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

#[cfg_attr(not(windows), allow(dead_code))]
mod watches;

#[cfg(windows)]
pub mod heartbeat;

#[cfg(windows)]
pub use heartbeat::ShellHeartbeat;
pub use watches::{WatchRefused, MAX_WATCHED_WINDOWS};
