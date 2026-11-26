//! What the event loop tells the host.

use crate::command::{CreateFailure, WindowState};
use crate::frame::{FrameCause, FrameTicket};
use crate::geometry::{PhysicalPosition, PhysicalSize, ScaleFactor};
use crate::input::{
    ContactEvent, FilesDropped, ImeInput, KeyEvent, Modifiers, PointerButtonEvent, PointerMoved,
    TextEvent, WheelEvent,
};
use crate::monitor::MonitorKey;
use crate::origin::InputStamp;

/// One window, for as long as the process lives. Numbers are never used again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowKey(pub(crate) u64);

impl WindowKey {
    /// The key with this number, for a recorded event sequence or a test (MT-214 S3). It names a window only while the
    /// loop has one under it; a command for any other is refused as unknown.
    pub const fn from_number(number: u64) -> Self {
        Self(number)
    }

    /// The number, for diagnostics.
    pub fn number(self) -> u64 {
        self.0
    }
}

/// What the event loop tells the host, in the order it happened.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// The loop runs. Nothing was dispatched before this, so whatever must exist before the first wake is made here.
    LoopStarted,
    /// Something happened to one window.
    Window {
        /// Which.
        window: WindowKey,
        /// What.
        event: WindowEvent,
    },
    /// Monitors, their scales or their work areas changed. Monitor keys from before are no longer valid.
    MonitorsChanged,
    /// The system is about to sleep.
    SystemSuspending,
    /// The system woke up.
    SystemResumed,
    /// The Operator's session ends now. The process may be gone when the handler returns.
    SessionEnding,
    /// Shutdown began. The loop still runs and dispatches wakes, until [`crate::Control::finish`].
    ShutdownBegan,
    /// The last event: every window is gone and no wake follows. End what depends on wakes here.
    LoopEnding,
    /// Events were left out because the host did not take them in time. Held keys and buttons must be read as released.
    EventsLost {
        /// How many.
        count: u64,
    },
}

/// What happened to one window.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WindowEvent {
    /// It exists, hidden. Whatever must be attached before it is first shown is attached here.
    Created {
        /// The command that asked for it.
        command: u64,
        /// What it is like.
        state: WindowState,
    },
    /// It could not be created, and its key stays unused.
    CreateFailed {
        /// The command that asked for it.
        command: u64,
        /// Why.
        failure: CreateFailure,
    },
    /// It is gone. The last event of this window.
    Destroyed,
    /// Somebody asked to close it. The host destroys it, or does not.
    CloseRequested,
    /// The client area has a new size. Not reported while minimised.
    Resized {
        /// The size.
        inner: PhysicalSize,
        /// The scale it is at.
        scale: ScaleFactor,
    },
    /// The frame's top left corner moved. Not reported while minimised.
    Moved {
        /// Where to.
        outer: PhysicalPosition,
    },
    /// It is drawn at another scale now. The logical size stays; [`WindowEvent::Resized`] follows when the physical
    /// size changes with it.
    ScaleChanged {
        /// The scale before.
        from: ScaleFactor,
        /// The scale now.
        to: ScaleFactor,
    },
    /// Most of it is on another monitor now, or on none.
    MonitorChanged {
        /// Which.
        monitor: Option<MonitorKey>,
    },
    /// It was hidden, shown, minimised, maximised or restored.
    DisplayChanged {
        /// What it is now.
        display: Display,
    },
    /// It gained or lost the keyboard.
    FocusChanged {
        /// It has it.
        focused: bool,
    },
    /// The operating system runs a loop of its own for this window. Events and wakes keep coming.
    ModalLoop {
        /// Which loop.
        kind: ModalLoop,
        /// It began; otherwise it ended.
        began: bool,
    },
    /// A frame is due. Draw it, then say what became of it with [`crate::Control::frame_done`]: the next one is due
    /// only after that.
    FrameDue {
        /// Names this frame.
        ticket: FrameTicket,
        /// Why.
        cause: FrameCause,
    },
    /// A key.
    Key(KeyEvent),
    /// Text.
    Text(TextEvent),
    /// Other modifiers are held.
    ModifiersChanged {
        /// Those held now.
        modifiers: Modifiers,
        /// Where and when from.
        stamp: InputStamp,
    },
    /// The pointer moved. Moves the host did not take in time are folded into the newest.
    PointerMoved(PointerMoved),
    /// The pointer came in.
    PointerEntered(InputStamp),
    /// The pointer left.
    PointerLeft(InputStamp),
    /// A pointer button.
    PointerButton(PointerButtonEvent),
    /// The wheel.
    Wheel(WheelEvent),
    /// Touch or pen.
    Contact(ContactEvent),
    /// Files were dropped.
    FilesDropped(FilesDropped),
    /// The operating system's input method changed the composition. Only while [`crate::WindowCommand::AllowIme`] is
    /// on and the window has the keyboard.
    Ime(ImeInput),
    /// A quiet window was activated by something outside this crate. It must not happen; the host records it.
    QuietBroken,
}

/// How a window is displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Display {
    /// Not at all.
    Hidden,
    /// At its own size.
    Normal,
    /// As a taskbar button only.
    Minimized,
    /// Filling the work area.
    Maximized,
}

/// A loop the operating system runs in place of ours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModalLoop {
    /// Moving or sizing by the frame.
    MoveOrSize,
    /// A menu.
    Menu,
}
