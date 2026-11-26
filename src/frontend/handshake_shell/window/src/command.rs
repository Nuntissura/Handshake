//! What the host can ask of a window, and how it is refused.

use std::fmt;

use crate::event::Display;
use crate::geometry::{
    ClientRect, LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, ScaleFactor,
};
use crate::input::CursorShape;
use crate::monitor::MonitorKey;

/// Whom a window is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WindowClass {
    /// For the Operator: a taskbar button, and activation when the Operator asks for it.
    Operator,
    /// For agents and tests: no taskbar button, never active, at the bottom of the Z order whenever it is shown.
    Quiet,
}

/// Where a new window goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Where the operating system puts new windows.
    System,
    /// In the middle of a monitor's work area.
    Centered(MonitorKey),
    /// With its frame's top left corner here.
    At(PhysicalPosition),
    /// On no monitor. For a quiet window that must exist but not be seen.
    OutsideAllMonitors,
}

/// What a new window is like. It is created hidden.
#[derive(Clone, PartialEq)]
pub struct WindowSpec {
    /// Title bar text. May be private: never printed.
    pub title: String,
    /// Client size.
    pub inner: LogicalSize,
    /// The smallest client size the Operator can drag it to.
    pub min_inner: Option<LogicalSize>,
    /// Where.
    pub placement: Placement,
    /// For whom.
    pub class: WindowClass,
}

impl fmt::Debug for WindowSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowSpec")
            .field("inner", &self.inner)
            .field("min_inner", &self.min_inner)
            .field("placement", &self.placement)
            .field("class", &self.class)
            .finish_non_exhaustive()
    }
}

/// A command for an existing window.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum WindowCommand {
    /// Show it. A minimised window comes back as it was before.
    Show {
        /// Also make it the active window. Only the Operator's authority does that.
        activate: bool,
    },
    /// Show it at its normal size, also when it is minimised or maximised.
    Restore {
        /// Also make it the active window. Only the Operator's authority does that.
        activate: bool,
    },
    /// Hide it.
    Hide,
    /// Minimise it. The next window becomes active only when the Operator asked.
    Minimize,
    /// Give the client area this size.
    SetInnerSize(LogicalSize),
    /// Move the frame's top left corner here.
    MoveTo(PhysicalPosition),
    /// Move it to the same place on another monitor. Its logical size stays.
    MoveToMonitor(MonitorKey),
    /// The pointer's shape over the client area.
    SetCursor(CursorShape),
    /// Title bar text. May be private: never printed.
    SetTitle(String),
    /// Let the operating system's input method compose text for the window, or not; off until asked. On while a text
    /// target that takes composed text has the window's focus. It moves no window and no focus.
    AllowIme(bool),
    /// Where the input method's candidate window keeps clear of: the caret, in the client area.
    SetImeArea(ClientRect),
    /// Destroy it. Refused while a [`crate::NativeWindowToken`] exists: a Vulkan surface must not outlive its window.
    Destroy,
}

impl fmt::Debug for WindowCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Show { activate } => write!(f, "Show {{ activate: {activate} }}"),
            Self::Restore { activate } => write!(f, "Restore {{ activate: {activate} }}"),
            Self::Hide => f.write_str("Hide"),
            Self::Minimize => f.write_str("Minimize"),
            Self::SetInnerSize(size) => write!(f, "SetInnerSize({size:?})"),
            Self::MoveTo(to) => write!(f, "MoveTo({to:?})"),
            Self::MoveToMonitor(to) => write!(f, "MoveToMonitor({to:?})"),
            Self::SetCursor(shape) => write!(f, "SetCursor({shape:?})"),
            Self::SetTitle(_) => f.write_str("SetTitle"),
            Self::AllowIme(allowed) => write!(f, "AllowIme({allowed})"),
            Self::SetImeArea(area) => write!(f, "SetImeArea({area:?})"),
            Self::Destroy => f.write_str("Destroy"),
        }
    }
}

/// What a window is like now, read from the operating system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowState {
    /// For whom.
    pub class: WindowClass,
    /// How it is displayed.
    pub display: Display,
    /// It has the keyboard.
    pub focused: bool,
    /// Its frame.
    pub outer: PhysicalRect,
    /// Its client size. Empty while minimised.
    pub inner: PhysicalSize,
    /// Its scale.
    pub scale: ScaleFactor,
    /// The monitor most of it is on.
    pub monitor: Option<MonitorKey>,
    /// How many [`crate::NativeWindowToken`]s exist.
    pub native_tokens: usize,
}

/// Why a command did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Refusal {
    /// No such window, or it is gone.
    UnknownWindow,
    /// No such monitor, or monitors changed since.
    UnknownMonitor,
    /// That frame is not outstanding.
    UnknownFrame,
    /// Too many windows.
    WindowLimit,
    /// The size or position cannot be used.
    InvalidGeometry,
    /// Size and position of a minimised or maximised window belong to the operating system.
    NotAtNormalSize,
    /// A quiet window is never activated.
    QuietWindow,
    /// Only the Operator's own recent input may do this.
    NeedsOperator,
    /// A model may not take away the window the Operator works in.
    WouldMoveFocus,
    /// A model may not move or size the window the Operator works in.
    OperatorsWindow,
    /// The window's surface is still held.
    NativeTokensHeld,
    /// The loop shuts down.
    ShuttingDown,
    /// [`crate::Control::begin_shutdown`] comes first, so the host can end what watches the loop while it runs.
    ShutdownNotBegun,
    /// The operating system refused, with its error number.
    System(u32),
}

impl Refusal {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownWindow => "window.unknown",
            Self::UnknownMonitor => "window.unknown_monitor",
            Self::UnknownFrame => "window.unknown_frame",
            Self::WindowLimit => "window.limit",
            Self::InvalidGeometry => "window.invalid_geometry",
            Self::NotAtNormalSize => "window.not_at_normal_size",
            Self::QuietWindow => "window.quiet",
            Self::NeedsOperator => "window.needs_operator",
            Self::WouldMoveFocus => "window.would_move_focus",
            Self::OperatorsWindow => "window.operators_window",
            Self::NativeTokensHeld => "window.native_tokens_held",
            Self::ShuttingDown => "window.shutting_down",
            Self::ShutdownNotBegun => "window.shutdown_not_begun",
            Self::System(_) => "window.system",
        }
    }
}

/// Why a window could not be created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CreateFailure {
    /// The operating system refused, with its error number.
    System(u32),
    /// The monitor it was to go on is gone.
    UnknownMonitor,
    /// Shutdown began first.
    ShuttingDown,
}

impl CreateFailure {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::System(_) => "window.create.system",
            Self::UnknownMonitor => "window.create.unknown_monitor",
            Self::ShuttingDown => "window.create.shutting_down",
        }
    }
}
