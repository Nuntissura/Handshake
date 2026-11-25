//! Native windows and the event loop of the kernel-owned shell (WP-KERNEL-012 MT-214 S1).
//!
//! Typed commands in, typed events out, with no window-library, graphics or toolkit type in between. Nothing is drawn
//! here, and nothing moves the Operator's focus, cursor or windows.
//!
//! # How a host uses it
//!
//! 1. Implement [`ShellHost`] and call [`run`] on the thread that will own the windows.
//! 2. On [`Event::LoopStarted`] make what must exist before the first wake (the diagnostics recorder), then open a
//!    [`WakeLane`] for whatever proves the loop is alive. No wake is dispatched before.
//! 3. [`Control::create_window`] creates a hidden window. On [`WindowEvent::Created`] attach what must be there before
//!    it is first shown (surface, accessibility), then [`WindowCommand::Show`] it.
//! 4. Draw on [`WindowEvent::FrameDue`] and answer with [`Control::frame_done`]. Ask with [`Control::request_frame`].
//! 5. To exit: [`Control::begin_shutdown`], end what watches the loop while it still runs, drop every
//!    [`NativeWindowToken`], then [`Control::finish`].
//!
//! # Who may do what
//!
//! A command carries a [`CommandSource`]. The Operator's authority comes from the [`InputStamp`] of an input that the
//! operating system reported as hardware, for [`OPERATOR_AUTHORITY_LIFETIME`], or once from the Operator's launch of
//! the process. Only it activates a window or replaces the clipboard. A model's command never does, and cannot hide,
//! minimise or destroy the window the Operator works in.
//!
//! A [`WindowClass::Quiet`] window is never activated: not by a command, not by a click, and not by a style that
//! something else sets. It is at the bottom of the Z order whenever it is shown.
//!
//! # Wakes
//!
//! A wake is a message posted to a window of the loop, so the loops that the operating system runs for moving, sizing,
//! menus and dialogs dispatch it as well. [`ShellHost::on_wake`] gets a [`DispatchedWake`], which only the loop can
//! make. An accepted wake is posted until it is dispatched.
//!
//! What a dispatched wake does not prove: that anything was drawn, or that events reach the host. [`FrameProgress`]
//! measures the first; the second stops while the host is inside a loop it opened itself.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What to do |
//! | --- | --- | --- |
//! | [`LoopError::AlreadyCreated`] | a second loop in one process | one loop per process |
//! | [`Refusal::NeedsOperator`], [`Refusal::QuietWindow`] | activation without the Operator | show without activating |
//! | [`Refusal::WouldMoveFocus`] | a model reached for the Operator's window | leave it, or ask the Operator |
//! | [`Refusal::NativeTokensHeld`] | a surface may still exist | release it, drop its token, ask again |
//! | [`Refusal::ShutdownNotBegun`] | finish without shutdown | end the watcher first |
//! | [`WindowEvent::QuietBroken`] | something outside this crate activated a quiet window | record it; it is a defect |
//! | [`Event::EventsLost`] | the host did not take events | read held keys and buttons as released |
//! | [`FrameProgress::waiting`] grows | the loop lives and presents nothing | the renderer's or the GPU's failure |
//!
//! # The window library
//!
//! winit 0.30.13, private to `platform` (MT-214 S1), with Win32 beside it for what winit does not do or does not do
//! quietly. README.md lists the evidence for each.
//!
//! On other platforms only the data types exist so far.

#![deny(missing_docs)]
#![deny(private_bounds, private_interfaces)]
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

mod clipboard;
mod command;
mod event;
#[cfg_attr(not(windows), allow(dead_code))]
mod frame;
mod geometry;
mod input;
mod keys;
mod monitor;
#[cfg_attr(not(windows), allow(dead_code))]
mod origin;
#[cfg_attr(not(windows), allow(dead_code))]
mod policy;
#[cfg_attr(not(windows), allow(dead_code))]
mod queue;
#[cfg_attr(not(windows), allow(dead_code))]
mod wake_slot;

#[cfg(windows)]
mod host;
#[cfg(windows)]
mod platform;
#[cfg(windows)]
mod token;
#[cfg(windows)]
mod wake;

pub use clipboard::{ClipboardError, TextClipboard, MAX_CLIPBOARD_BYTES};
pub use command::{
    CreateFailure, Placement, Refusal, WindowClass, WindowCommand, WindowSpec, WindowState,
};
pub use event::{Display, Event, ModalLoop, WindowEvent, WindowKey};
pub use frame::{FrameCause, FrameOutcome, FrameProgress, FrameTicket};
pub use geometry::{
    ClientPoint, LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, ScaleFactor, BASE_DPI,
};
pub use input::{
    ContactEvent, ContactPhase, CursorShape, FilesDropped, KeyEvent, KeyLocation, LogicalKey,
    Modifiers, PhysicalKey, PointerButton, PointerButtonEvent, PointerMoved, PressState, TextEvent,
    TypedText, WheelEvent, WheelSettings, MAX_DROPPED_FILES, WHEEL_DETENT,
};
pub use keys::{KeyCode, NamedKey};
pub use monitor::{carried, centered, clamped, outside_all, size_on, Monitor, MonitorKey};
pub use origin::{
    Attribution, CommandSource, DeviceKind, EventTime, InputStamp, Origin,
    LAUNCH_AUTHORITY_LIFETIME, OPERATOR_AUTHORITY_LIFETIME,
};

#[cfg(windows)]
pub use host::{run, Control, LoopConfig, LoopError, RunReport, ShellHost};
#[cfg(windows)]
pub use token::{NativeWindowToken, Win32Window};
#[cfg(windows)]
pub use wake::{DispatchedWake, LaneId, LaneWaker, WakeLane, WakeRefused};
