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
//! operating system reported as hardware, for [`OPERATOR_AUTHORITY_LIFETIME`] and for one command: the first command
//! that needs it uses it up, for that input and every earlier one, so a copied stamp grants nothing again. The
//! Operator's launch of the process activates one window, once. Only that authority activates a window, hands the
//! activation on with a minimise, or replaces the clipboard. A model's command never does, and cannot move, size,
//! restore, hide, minimise or destroy the window the Operator works in, nor show it back from minimised, nor do any of
//! that to its owner when the Operator works in a window it owns. A request to close that window lets a model remove
//! it once, while the request is open. When a removal the Operator approved leaves the keyboard nowhere, it goes to
//! the removed window's owner, else to the topmost other Operator window, and never away from another program.
//!
//! A [`WindowClass::Quiet`] window is never activated: not by a command, not by a click, not by a style that something
//! else sets, and not by a direct call of the operating system's: a CBT hook of the loop's thread refuses it, and every
//! activation of any of the loop's windows while this crate carries out a change that does not activate. It is at the
//! bottom of the Z order whenever it is shown, and has no taskbar button.
//!
//! # Wakes
//!
//! A wake is a message posted to a window of the loop, so the loops that the operating system runs for moving, sizing,
//! menus and dialogs dispatch it as well. [`ShellHost::on_wake`] gets a [`DispatchedWake`], which only the loop can
//! make: a wake message that was sent, or posted by anybody but the lane, dispatches nothing. An accepted wake is
//! posted again until it is dispatched, also when other loops throw posts away, at intervals that grow to 8 s; after
//! four posts nobody answered only once the loop shows that it runs, so a hung loop's queue holds at most four posts
//! per lane. A refused post is tried again at intervals that grow from 10 ms to 8 s. From [`Control::finish`] on no
//! wake reaches the host.
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
//! | [`Refusal::OperatorsWindow`] | a model tried to move, size or restore the Operator's window | leave it, or ask the Operator |
//! | [`ClipboardError::CannotKeep`] | the clipboard holds only what a failed write could not put back | nothing was written; try again later |
//! | [`RunReport::sent_keys_refused`], [`RunReport::sent_wakes_refused`] above 0 | another thread sent input or a wake | find the sender |
//! | [`RunReport::activations_refused`] above 0 | something asked to activate a window that must not be | expected in tests that try; in the product, find the caller |
//! | [`Refusal::NativeTokensHeld`] | a surface may still exist | release it, drop its token, ask again |
//! | [`Refusal::ShutdownNotBegun`] | finish without shutdown | end the watcher first |
//! | [`WindowEvent::QuietBroken`] | something outside this crate activated a quiet window | record it; it is a defect |
//! | [`Event::EventsLost`] | the host did not take events; it comes where the loss was | read held keys and buttons as released |
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
