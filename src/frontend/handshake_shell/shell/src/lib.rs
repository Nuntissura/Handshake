//! The shell crate of the kernel-owned shell (WP-KERNEL-012 MT-214 AC214-12): composition of the shell's crates and
//! the toolkit-free host ports, with no widget, pane or domain logic of its own.
//!
//! It holds two compositions. MT-214 S5, [`program`]: the five crates as one process, with the pre-account startup gate
//! ([`authority`]: the product opens the device only under `device.gpu.shell` and the kernel issuer, and refuses with a
//! typed outcome without them), the heartbeat, the accessibility hub, the input tracker, and a shutdown that drains,
//! settles claims and acknowledges the watcher only when the owner reported everything settled; [`outcome`] gives every
//! end a typed kind and exit code. The binary is `handshake_shell` (`src/main.rs`); `examples/shell_program.rs` is the
//! test binary for a manual run. MT-214 S6 adds [`windows`] (the root window, detached panes and the detached Settings
//! window on one device and one owner ledger, AC214-8) and [`unavailable`] (the typed, readable state when no device is
//! available, AC214-9; a lost device is recovered by the presenter). And MT-215 slice 2: [`ShellHeartbeat`], the UI heartbeat port bound to the window
//! crate's event loop and the surface crate's present path, so that a frozen UI thread or a stalled present reaches the
//! diagnostics ring that the internal diagnostics and the standalone Palmistry watcher read. The module documentation
//! of [`heartbeat`] is its manual: how a host uses it, what it proves and what reads it.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What to do |
//! | --- | --- | --- |
//! | `OpenRefused::AlreadyOpen` | a heartbeat of this loop lives already; one publisher per ring | keep the first |
//! | `OpenRefused::Lane` | the loop has ended, or has no lane left | none: the window crate's refusal |
//! | `StartRefused::NotLoopThread` | started off the loop's thread | start it in a handler |
//! | a stale heartbeat | the UI thread is frozen, blocked or stuck in a handler | Palmistry confirms a freeze once the window does not answer |
//! | `ShellHeartbeat::present_stalled`, a `SlowFrame` `Degraded` event in the ring | a watched window wanted a frame for 2 s and none was presented, while the heartbeat stays fresh | the renderer's or the GPU's failure; one `SlowFrame` `Recovered` event follows when frames present again |
//! | `WakeOutcome::Ignored` | a wake of the heartbeat's lane that was late, duplicate or not asked for | nothing |
//! | [`WatchRefused::Full`] | [`MAX_WATCHED_WINDOWS`] are watched | that window's presents are not watched |
//! | `HeartbeatCounters::loop_closed` | the loop ended | none |
//!
//! Windows x64 first; on other systems only the watches compile.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod authority;
pub mod outcome;
#[cfg(windows)]
pub mod palmistry;
pub mod shutdown;
pub mod unavailable;
pub mod windows;
#[cfg_attr(not(windows), allow(dead_code))]
mod watches;

#[cfg(windows)]
pub mod heartbeat;
#[cfg(windows)]
pub mod program;

pub use authority::{
    resolve_product, run_product, DeviceAuthority, IssuedAuthority, StartupRefusal, TestBinary, CAPABILITY, MISSING,
};
#[cfg(windows)]
pub use heartbeat::{OpenRefused, ShellHeartbeat};
pub use outcome::{conclude, may_acknowledge, shutdown_settled, ExitKind};
#[cfg(windows)]
pub use program::{run_program, EndAfter, ProgramConfig, ProgramOutcome, ProgramReport};
#[cfg(windows)]
pub use palmistry::{watcher_ack, OwnedWatcher, PalmistryWatcher};
pub use shutdown::{NoWatcher, ShutdownWatcher, WatcherAck};
pub use unavailable::{unavailable_after, Cause, UnavailableState, When, HEADLINE};
pub use windows::{Action, DetachedSpec, KindCounts, WindowKind};
pub use watches::{WatchRefused, MAX_WATCHED_WINDOWS};
