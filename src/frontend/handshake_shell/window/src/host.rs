//! The host, and what it drives the loop with.

use std::rc::Rc;
use std::time::Duration;

use crate::clipboard::{ClipboardError, TextClipboard};
use crate::command::{Refusal, WindowCommand, WindowSpec, WindowState};
use crate::event::{Event, WindowKey};
use crate::frame::{FrameOutcome, FrameProgress, FrameTicket};
use crate::input::WheelSettings;
use crate::monitor::Monitor;
use crate::origin::{CommandSource, EventTime};
use crate::platform::Core;
use crate::token::NativeWindowToken;
use crate::wake::{DispatchedWake, WakeLane, WakeRefused};

/// The host of the loop. Both methods run on the loop's thread.
///
/// They take `&self` because a wake can arrive while an event is being handled: an event handler that opens a dialog
/// or a menu runs a loop of the operating system's, and that loop dispatches wakes. Events wait until the handler
/// returns, so `on_event` is never entered twice.
pub trait ShellHost {
    /// Something happened.
    fn on_event(&self, control: &Control, event: Event);

    /// The loop dispatched a wake. Also called inside modal loops and during shutdown. Do little here.
    fn on_wake(&self, control: &Control, wake: &DispatchedWake<'_>);
}

/// How to run the loop.
#[derive(Default)]
pub struct LoopConfig {
    /// The Operator started this process to work in it. The host may then activate one window, once, shortly after
    /// the start ([`Control::take_launch_source`]); it does not replace the clipboard. An agent, a test and an automatic
    /// start leave this off.
    pub launched_by_operator: bool,
    /// Where clipboard text is kept instead of the operating system's clipboard.
    pub clipboard: Option<Box<dyn TextClipboard>>,
}

/// Why the loop did not run.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoopError {
    /// This process already created its loop.
    AlreadyCreated,
    /// The operating system refused, with its error number.
    System(u32),
    /// The window library failed. A short reason without content.
    Library(&'static str),
}

impl LoopError {
    /// A stable name, for diagnostics.
    pub fn code(&self) -> &'static str {
        match self {
            Self::AlreadyCreated => "loop.already_created",
            Self::System(_) => "loop.system",
            Self::Library(_) => "loop.library",
        }
    }
}

/// What a run of the loop was like. Mechanical numbers only, for the host's diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunReport {
    /// What [`Control::finish`] was given.
    pub exit_code: i32,
    /// Windows created.
    pub windows_created: u64,
    /// Windows destroyed. The same number, unless something outside this crate interfered.
    pub windows_destroyed: u64,
    /// Wakes handed to the host.
    pub wakes: u64,
    /// The longest a wake was dispatched after it was due.
    pub longest_wake_delay: Duration,
    /// Commands refused.
    pub refusals: u64,
    /// Times a quiet window was activated by something outside this crate. Must be 0.
    pub quiet_broken: u64,
    /// Activations the guard of the loop's thread refused: of a quiet window, or of any window while this crate
    /// carried out a change that does not activate.
    pub activations_refused: u64,
    /// Windows left hidden at the end because a native token still named them; they go when the thread ends.
    pub windows_left: u64,
    /// Wake messages another thread sent, refused: a wake comes out of the loop's own queue only.
    pub sent_wakes_refused: u64,
    /// Keyboard messages another thread sent, refused: input comes through the queue, and the window library is not
    /// entered with a key while it looks at the queue for one (MT-214 R2-N2).
    pub sent_keys_refused: u64,
}

/// The loop, as the host's handlers use it.
#[derive(Clone)]
pub struct Control {
    core: Rc<Core>,
}

impl Control {
    pub(crate) fn new(core: Rc<Core>) -> Self {
        Self { core }
    }

    /// The time on the clock of the events.
    pub fn now(&self) -> EventTime {
        self.core.now()
    }

    /// The numbers of the run so far; [`run`] returns them at the end.
    pub fn report(&self) -> RunReport {
        self.core.report()
    }

    /// Ask for a window. It is created hidden; [`crate::WindowEvent::Created`] or
    /// [`crate::WindowEvent::CreateFailed`] follows.
    ///
    /// # Errors
    ///
    /// [`Refusal`].
    pub fn create_window(
        &self,
        source: &CommandSource,
        spec: WindowSpec,
    ) -> Result<WindowKey, Refusal> {
        self.core.create_window(source, spec)
    }

    /// Carry out a command, now.
    ///
    /// # Errors
    ///
    /// [`Refusal`].
    pub fn command(
        &self,
        source: &CommandSource,
        window: WindowKey,
        command: WindowCommand,
    ) -> Result<(), Refusal> {
        self.core.command(source, window, command)
    }

    /// What a window is like now.
    pub fn window_state(&self, window: WindowKey) -> Option<WindowState> {
        self.core.window_state(window)
    }

    /// The windows that exist.
    pub fn windows(&self) -> Vec<WindowKey> {
        self.core.window_keys()
    }

    /// A token for the GPU owner. Once one was given out, the window is destroyed only after all are dropped.
    pub fn native_window(&self, window: WindowKey) -> Option<NativeWindowToken> {
        self.core.native_window(window)
    }

    /// The monitors.
    pub fn monitors(&self) -> Vec<Monitor> {
        self.core.monitors()
    }

    /// How far a wheel detent scrolls.
    pub fn wheel_settings(&self) -> WheelSettings {
        self.core.wheel_settings()
    }

    /// Ask for a frame. [`crate::WindowEvent::FrameDue`] follows when the window can show one.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownWindow`].
    pub fn request_frame(&self, window: WindowKey) -> Result<(), Refusal> {
        self.core.request_frame(window)
    }

    /// Say what became of a due frame.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownWindow`], or [`Refusal::UnknownFrame`] when it is not outstanding.
    pub fn frame_done(
        &self,
        window: WindowKey,
        ticket: FrameTicket,
        outcome: FrameOutcome,
    ) -> Result<(), Refusal> {
        self.core.frame_done(window, ticket, outcome)
    }

    /// The frames of a window so far.
    pub fn frame_progress(&self, window: WindowKey) -> Option<FrameProgress> {
        self.core.frame_progress(window)
    }

    /// Open a lane for wakes.
    ///
    /// # Errors
    ///
    /// [`WakeRefused`].
    pub fn open_wake_lane(&self) -> Result<WakeLane, WakeRefused> {
        self.core.open_lane()
    }

    /// The text on the clipboard.
    ///
    /// # Errors
    ///
    /// [`ClipboardError`].
    pub fn read_clipboard(&self) -> Result<Option<String>, ClipboardError> {
        self.core.read_clipboard()
    }

    /// Put text on the clipboard. Only the Operator's authority does.
    ///
    /// # Errors
    ///
    /// [`ClipboardError`].
    pub fn write_clipboard(
        &self,
        source: &CommandSource,
        text: &str,
    ) -> Result<(), ClipboardError> {
        self.core.write_clipboard(source, text)
    }

    /// Take back what this process last put on the clipboard, if it is still there; for example when an account logs
    /// out. Returns whether it was.
    ///
    /// # Errors
    ///
    /// [`ClipboardError`].
    pub fn withdraw_clipboard(&self) -> Result<bool, ClipboardError> {
        self.core.withdraw_clipboard()
    }

    /// The authority of the Operator who launched this process, for the host's first window: given once, and only
    /// with [`LoopConfig::launched_by_operator`]; it activates one window, once, within
    /// [`crate::LAUNCH_AUTHORITY_LIFETIME`].
    pub fn take_launch_source(&self, id: u64) -> Option<CommandSource> {
        self.core.take_launch_source(id)
    }

    /// Begin to shut down. The loop keeps running, so that what watches it can be ended while it still answers
    /// (MT-215 HB-3). No window can be created from here on.
    ///
    /// # Errors
    ///
    /// [`Refusal::ShuttingDown`] when shutdown has begun.
    pub fn begin_shutdown(&self) -> Result<(), Refusal> {
        self.core.begin_shutdown()
    }

    /// Destroy every window, then end the loop with this code. [`Event::LoopEnding`] is the last event. No wake reaches
    /// the host from here on, also not from a loop the host runs, and no native token is given out.
    ///
    /// # Errors
    ///
    /// [`Refusal::ShutdownNotBegun`], or [`Refusal::NativeTokensHeld`] while a surface may still exist.
    pub fn finish(&self, exit_code: i32) -> Result<(), Refusal> {
        self.core.finish(exit_code)
    }
}

/// Run the loop on this thread until [`Control::finish`]. One loop per process.
///
/// A panic of the host ends the loop: its windows are destroyed, the host is not called again, and the panic continues
/// from here. Panic hooks and the operating system's exception handlers are not touched.
///
/// # Errors
///
/// [`LoopError`].
pub fn run(config: LoopConfig, host: Rc<dyn ShellHost>) -> Result<RunReport, LoopError> {
    crate::platform::run(config, host)
}
