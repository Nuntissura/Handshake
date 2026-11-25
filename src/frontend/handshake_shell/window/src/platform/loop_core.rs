//! The state of one loop, on its thread.
//!
//! The window library's callbacks only put events into the queue. The host is called from the service window's
//! procedure, so a loop that the host or the operating system runs inside another one keeps dispatching.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, PostQuitMessage, MSG, PM_REMOVE, WM_QUIT,
};
use winit::event_loop::EventLoopProxy;

use super::window::Window;
use super::{clipboard, monitors, place, wake::Lane};
use crate::clipboard::{ClipboardError, TextClipboard};
use crate::command::{CreateFailure, Refusal, WindowCommand, WindowSpec, WindowState};
use crate::event::{Event, WindowEvent, WindowKey};
use crate::frame::{FrameOutcome, FrameProgress, FrameTicket};
use crate::host::{Control, LoopConfig, LoopError, RunReport, ShellHost};
use crate::input::WheelSettings;
use crate::monitor::{carried, Monitor};
use crate::origin::{CommandSource, DeviceKind, EventTime, Origin};
use crate::policy;
use crate::queue::EventQueue;
use crate::token::NativeWindowToken;
use crate::wake::{DispatchedWake, LaneId, WakeLane, WakeRefused};
use crate::wake_slot::nanos;

/// The most windows that exist or are being created at once.
const MAX_WINDOWS: usize = 32;

/// The most lanes one loop opens. Their numbers are not used again.
const MAX_LANES: usize = 64;

/// The lane that hands events to the host.
const EVENT_LANE: u32 = 0;

/// The most events handed over in one go, so that wakes get their turn.
const EVENTS_PER_TURN: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Running,
    /// Shutdown began; the host ends what watches the loop.
    Draining,
    /// Windows are being destroyed.
    Finishing,
    /// The host was told that the loop ends, or panicked.
    Ended,
}

/// Where the message that is being handled came from.
#[derive(Clone, Copy)]
pub(super) struct MessageOrigin {
    pub(super) os_millis: Option<u32>,
    pub(super) origin: Origin,
    pub(super) device: DeviceKind,
}

impl MessageOrigin {
    pub(super) const NONE: Self = Self {
        os_millis: None,
        origin: Origin::Unattributed,
        device: DeviceKind::Unknown,
    };
}

/// A window that was asked for.
pub(super) struct Creation {
    pub(super) key: WindowKey,
    pub(super) command: u64,
    pub(super) spec: WindowSpec,
}

pub(crate) struct Core {
    epoch: Instant,
    thread: u32,
    host: RefCell<Option<Rc<dyn ShellHost>>>,
    service: Cell<HWND>,
    library: RefCell<Option<EventLoopProxy<()>>>,
    windows: RefCell<Vec<Rc<Window>>>,
    creations: RefCell<VecDeque<Creation>>,
    events: RefCell<EventQueue>,
    drain_requested: Cell<bool>,
    host_busy: Cell<bool>,
    phase: Cell<Phase>,
    next_window: Cell<u64>,
    lanes: RefCell<Vec<Option<Lane>>>,
    message: Cell<MessageOrigin>,
    clipboard: RefCell<Option<Box<dyn TextClipboard>>>,
    launched_by_operator: Cell<bool>,
    report: Cell<RunReport>,
    panic: RefCell<Option<Box<dyn Any + Send>>>,
}

impl Core {
    pub(super) fn new(
        config: LoopConfig,
        host: Rc<dyn ShellHost>,
        library: EventLoopProxy<()>,
    ) -> Rc<Self> {
        Rc::new(Self {
            epoch: Instant::now(),
            // SAFETY: no arguments.
            thread: unsafe { GetCurrentThreadId() },
            host: RefCell::new(Some(host)),
            service: Cell::new(0),
            library: RefCell::new(Some(library)),
            windows: RefCell::default(),
            creations: RefCell::default(),
            events: RefCell::default(),
            drain_requested: Cell::new(false),
            host_busy: Cell::new(false),
            phase: Cell::new(Phase::Running),
            next_window: Cell::new(1),
            lanes: RefCell::default(),
            message: Cell::new(MessageOrigin::NONE),
            clipboard: RefCell::new(config.clipboard),
            launched_by_operator: Cell::new(config.launched_by_operator),
            report: Cell::default(),
            panic: RefCell::default(),
        })
    }

    /// The service window exists: wakes and the clipboard work from here on.
    pub(super) fn attach_service(&self, service: HWND) -> Result<(), LoopError> {
        self.service.set(service);
        self.clipboard
            .borrow_mut()
            .get_or_insert_with(|| Box::new(clipboard::SystemClipboard::new(service)));
        let lane = Lane::open(EVENT_LANE, service, self.thread, self.epoch)
            .ok_or_else(|| LoopError::System(super::last_error()))?;
        self.lanes.borrow_mut().push(Some(lane));
        Ok(())
    }

    /// The loop is over. Wakes are refused from here on, by any thread.
    pub(super) fn detach(&self) -> RunReport {
        let lanes = std::mem::take(&mut *self.lanes.borrow_mut());
        for lane in lanes.iter().flatten() {
            lane.close();
        }
        self.phase.set(Phase::Ended);
        self.library.borrow_mut().take();
        self.host.borrow_mut().take();
        self.service.set(0);
        self.report.get()
    }

    // Time and messages.

    pub(crate) fn now(&self) -> EventTime {
        EventTime {
            nanos: nanos(self.epoch.elapsed()),
        }
    }

    pub(super) fn message(&self) -> MessageOrigin {
        self.message.get()
    }

    pub(super) fn set_message(&self, origin: MessageOrigin) -> MessageOrigin {
        self.message.replace(origin)
    }

    // Events to the host.

    pub(super) fn push(&self, event: Event) {
        self.events.borrow_mut().push(event);
        self.request_drain();
    }

    pub(super) fn push_window(&self, window: WindowKey, event: WindowEvent) {
        self.push(Event::Window { window, event });
    }

    fn request_drain(&self) {
        if self.drain_requested.replace(true) {
            return;
        }
        let lane = self.lane(EVENT_LANE);
        let requested = lane.is_some_and(|lane| lane.request(0, Duration::ZERO).is_ok());
        self.drain_requested.set(requested);
    }

    fn lane(&self, id: u32) -> Option<Lane> {
        self.lanes.borrow().get(id as usize).cloned().flatten()
    }

    /// Hand queued events to the host. Does nothing inside a call of the host's own; that call's turn goes on when it
    /// returns.
    pub(super) fn drain(self: &Rc<Self>, most: usize) {
        self.drain_requested.set(false);
        if self.host_busy.get() {
            return;
        }
        let Some(host) = self.host.borrow().clone() else {
            return;
        };
        let control = Control::new(Rc::clone(self));
        self.host_busy.set(true);
        for _ in 0..most {
            if self.phase.get() == Phase::Ended {
                break;
            }
            let next = self.events.borrow_mut().pop();
            let Some(event) = next else { break };
            let last = event == Event::LoopEnding;
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| host.on_event(&control, event)))
            {
                self.panicked(payload);
            }
            if last {
                self.phase.set(Phase::Ended);
            }
        }
        self.host_busy.set(false);
        if !self.events.borrow().is_empty() && self.phase.get() != Phase::Ended {
            self.request_drain();
        }
        self.prompt_library();
    }

    /// The window library creates windows and ends its loop only inside one of its own callbacks: have it make one.
    fn prompt_library(&self) {
        if self.creations.borrow().is_empty() && !self.has_ended() {
            return;
        }
        if let Some(library) = self.library.borrow().as_ref() {
            // It ignores a full message queue. The loop's own turn then comes back to it.
            let _ = library.send_event(());
        }
    }

    /// The service window got a wake message out of the queue.
    pub(super) fn dispatch_wake(self: &Rc<Self>, id: u32) {
        let Some(lane) = self.lane(id) else { return };
        let Some(taken) = lane.take() else { return };
        if id == EVENT_LANE {
            self.drain(EVENTS_PER_TURN);
            return;
        }
        if self.phase.get() == Phase::Ended {
            return;
        }
        let Some(host) = self.host.borrow().clone() else {
            return;
        };
        let late = Duration::from_nanos(taken.late);
        self.update_report(|report| {
            report.wakes = report.wakes.saturating_add(1);
            report.longest_wake_delay = report.longest_wake_delay.max(late);
        });
        let control = Control::new(Rc::clone(self));
        let wake = DispatchedWake::new(LaneId(id), taken.value, late, self.host_busy.get());
        host.on_wake(&control, &wake);
    }

    /// Run something a window procedure must not unwind out of.
    pub(super) fn guarded<T>(&self, work: impl FnOnce() -> T) -> Option<T> {
        match catch_unwind(AssertUnwindSafe(work)) {
            Ok(value) => Some(value),
            Err(payload) => {
                self.panicked(payload);
                None
            }
        }
    }

    fn panicked(&self, payload: Box<dyn Any + Send>) {
        self.panic.borrow_mut().get_or_insert(payload);
        self.phase.set(Phase::Ended);
        // Ends a dialog or menu that the panicking call was inside of; the library's own loop takes no notice.
        // SAFETY: a plain value.
        unsafe { PostQuitMessage(1) };
        self.prompt_library();
    }

    /// The panic that ended the loop. The thread's next loop is not ended by it as well.
    pub(super) fn take_panic(&self) -> Option<Box<dyn Any + Send>> {
        let payload = self.panic.borrow_mut().take()?;
        // SAFETY: the message outlives the call.
        unsafe {
            let mut message: MSG = std::mem::zeroed();
            PeekMessageW(&mut message, 0, WM_QUIT, WM_QUIT, PM_REMOVE);
        }
        Some(payload)
    }

    pub(super) fn update_report(&self, change: impl FnOnce(&mut RunReport)) {
        let mut report = self.report.get();
        change(&mut report);
        self.report.set(report);
    }

    // Windows.

    pub(super) fn window_by_key(&self, key: WindowKey) -> Option<Rc<Window>> {
        self.windows
            .borrow()
            .iter()
            .find(|window| window.key == key)
            .cloned()
    }

    pub(super) fn window_by_id(&self, id: winit::window::WindowId) -> Option<Rc<Window>> {
        self.windows
            .borrow()
            .iter()
            .find(|window| window.id == id)
            .cloned()
    }

    pub(super) fn next_creation(&self) -> Option<Creation> {
        self.creations.borrow_mut().pop_front()
    }

    pub(super) fn window_created(&self, window: Rc<Window>, command: u64) {
        let key = window.key;
        let state = window.announce();
        self.windows.borrow_mut().push(window);
        self.update_report(|report| {
            report.windows_created = report.windows_created.saturating_add(1)
        });
        self.push_window(key, WindowEvent::Created { command, state });
    }

    pub(super) fn creation_failed(&self, key: WindowKey, command: u64, failure: CreateFailure) {
        self.push_window(key, WindowEvent::CreateFailed { command, failure });
    }

    pub(super) fn window_destroyed(&self, key: WindowKey) {
        let gone = {
            let mut windows = self.windows.borrow_mut();
            windows
                .iter()
                .position(|window| window.key == key)
                .map(|at| windows.remove(at))
        };
        let Some(gone) = gone else { return };
        gone.let_go();
        self.update_report(|report| {
            report.windows_destroyed = report.windows_destroyed.saturating_add(1)
        });
        self.push_window(key, WindowEvent::Destroyed);
        self.end_when_empty();
    }

    /// Destroy whatever is left, without asking. For the end of the loop.
    pub(super) fn destroy_windows(&self) {
        let windows: Vec<_> = self.windows.borrow().clone();
        for window in windows {
            place::destroy(window.hwnd);
        }
    }

    /// Let go of windows whose last event nobody will translate any more.
    pub(super) fn forget_windows(&self) {
        let windows = std::mem::take(&mut *self.windows.borrow_mut());
        for window in windows {
            window.let_go();
        }
    }

    fn end_when_empty(&self) {
        if self.phase.get() == Phase::Finishing && self.windows.borrow().is_empty() {
            self.push(Event::LoopEnding);
        }
    }

    /// Nothing is left to do but to end the window library's loop.
    pub(super) fn has_ended(&self) -> bool {
        self.phase.get() == Phase::Ended
    }

    // What `Control` offers.

    pub(crate) fn create_window(
        &self,
        source: &CommandSource,
        spec: WindowSpec,
    ) -> Result<WindowKey, Refusal> {
        let outcome = self.accept_window(source, spec);
        self.counted(outcome)
    }

    fn accept_window(
        &self,
        source: &CommandSource,
        spec: WindowSpec,
    ) -> Result<WindowKey, Refusal> {
        if self.phase.get() != Phase::Running {
            return Err(Refusal::ShuttingDown);
        }
        if !spec.inner.is_usable() || spec.min_inner.is_some_and(|least| !least.is_usable()) {
            return Err(Refusal::InvalidGeometry);
        }
        if self.windows.borrow().len() + self.creations.borrow().len() >= MAX_WINDOWS {
            return Err(Refusal::WindowLimit);
        }
        let key = WindowKey(self.next_window.get());
        self.next_window.set(key.0 + 1);
        self.creations.borrow_mut().push_back(Creation {
            key,
            command: source.id(),
            spec,
        });
        self.prompt_library();
        Ok(key)
    }

    pub(crate) fn command(
        &self,
        source: &CommandSource,
        window: WindowKey,
        command: WindowCommand,
    ) -> Result<(), Refusal> {
        let outcome = self.carry_out(source, window, command);
        self.counted(outcome)
    }

    fn carry_out(
        &self,
        source: &CommandSource,
        key: WindowKey,
        command: WindowCommand,
    ) -> Result<(), Refusal> {
        let window = self.window_by_key(key).ok_or(Refusal::UnknownWindow)?;
        let authority = source.authority(self.now());
        let foreground = place::foreground(window.hwnd);
        match command {
            WindowCommand::Show { activate } => {
                let plan = policy::plan_show(window.class, activate, authority, foreground)?;
                place::show(&window, plan, false)
            }
            WindowCommand::Restore { activate } => {
                let plan = policy::plan_show(window.class, activate, authority, foreground)?;
                place::show(&window, plan, true)
            }
            WindowCommand::Hide => {
                policy::may_remove(authority, foreground, window.close_requested.get())?;
                place::hide(window.hwnd);
                Ok(())
            }
            WindowCommand::Minimize => {
                let plan = policy::plan_minimize(window.class, authority, foreground)?;
                place::minimize(window.hwnd, plan);
                Ok(())
            }
            WindowCommand::SetInnerSize(size) => {
                if !size.is_usable() {
                    return Err(Refusal::InvalidGeometry);
                }
                place::at_normal_size(window.hwnd)?;
                let wanted = size.to_physical(window.scale.get());
                place::resize(window.hwnd, wanted)?;
                if place::inner(window.hwnd) == wanted {
                    // Exactly what was asked for, and not what its whole pixels come back to.
                    window.logical.set(size);
                }
                Ok(())
            }
            WindowCommand::MoveTo(position) => {
                place::at_normal_size(window.hwnd)?;
                place::move_to(window.hwnd, position)
            }
            WindowCommand::MoveToMonitor(target) => {
                place::at_normal_size(window.hwnd)?;
                let monitors = monitors::all();
                let find = |key| monitors.iter().find(|monitor| monitor.key == key);
                let to = find(target).ok_or(Refusal::UnknownMonitor)?;
                let outer = place::outer(window.hwnd);
                let position = match monitors::of_window(window.hwnd).and_then(find) {
                    Some(from) => carried(outer, from, to),
                    None => crate::monitor::centered(outer.size, to.work_area),
                };
                place::move_to(window.hwnd, position)
            }
            WindowCommand::SetCursor(shape) => {
                window.set_cursor(shape);
                Ok(())
            }
            WindowCommand::SetTitle(title) => {
                window.set_title(&title);
                Ok(())
            }
            WindowCommand::Destroy => {
                policy::may_remove(authority, foreground, window.close_requested.get())?;
                if window.tokens() > 0 {
                    return Err(Refusal::NativeTokensHeld);
                }
                if place::destroy(window.hwnd) {
                    Ok(())
                } else {
                    Err(Refusal::System(super::last_error()))
                }
            }
        }
    }

    fn counted<T>(&self, outcome: Result<T, Refusal>) -> Result<T, Refusal> {
        if outcome.is_err() {
            self.update_report(|report| report.refusals = report.refusals.saturating_add(1));
        }
        outcome
    }

    pub(crate) fn window_state(&self, key: WindowKey) -> Option<WindowState> {
        self.window_by_key(key).map(|window| window.state())
    }

    pub(crate) fn window_keys(&self) -> Vec<WindowKey> {
        self.windows
            .borrow()
            .iter()
            .map(|window| window.key)
            .collect()
    }

    pub(crate) fn native_window(&self, key: WindowKey) -> Option<NativeWindowToken> {
        let window = self.window_by_key(key)?;
        Some(NativeWindowToken::new(
            key,
            window.handles?,
            Arc::clone(&window.hold),
        ))
    }

    pub(crate) fn monitors(&self) -> Vec<Monitor> {
        monitors::all()
    }

    pub(crate) fn wheel_settings(&self) -> WheelSettings {
        monitors::wheel_settings()
    }

    pub(crate) fn request_frame(&self, key: WindowKey) -> Result<(), Refusal> {
        let window = self.window_by_key(key).ok_or(Refusal::UnknownWindow)?;
        if window.frames.borrow_mut().request(self.now().nanos) {
            window.ask_for_paint();
        }
        Ok(())
    }

    pub(crate) fn frame_done(
        &self,
        key: WindowKey,
        ticket: FrameTicket,
        outcome: FrameOutcome,
    ) -> Result<(), Refusal> {
        let window = self.window_by_key(key).ok_or(Refusal::UnknownWindow)?;
        let again = window
            .frames
            .borrow_mut()
            .answer(ticket, outcome)
            .ok_or(Refusal::UnknownFrame)?;
        if again {
            window.ask_for_paint();
        }
        Ok(())
    }

    pub(crate) fn frame_progress(&self, key: WindowKey) -> Option<FrameProgress> {
        let window = self.window_by_key(key)?;
        let progress = window
            .frames
            .borrow()
            .progress(self.now().nanos, window.can_show());
        Some(progress)
    }

    pub(crate) fn open_lane(self: &Rc<Self>) -> Result<WakeLane, WakeRefused> {
        if self.phase.get() == Phase::Ended || self.service.get() == 0 {
            return Err(WakeRefused::Closed);
        }
        let id = self.lanes.borrow().len();
        if id >= MAX_LANES {
            return Err(WakeRefused::LaneLimit);
        }
        let lane = u32::try_from(id)
            .ok()
            .and_then(|id| Lane::open(id, self.service.get(), self.thread, self.epoch))
            .ok_or(WakeRefused::LaneLimit)?;
        self.lanes.borrow_mut().push(Some(lane.clone()));
        Ok(WakeLane::new(lane, Rc::downgrade(self)))
    }

    pub(crate) fn forget_lane(&self, id: LaneId) {
        let forgotten = self
            .lanes
            .borrow_mut()
            .get_mut(id.0 as usize)
            .and_then(Option::take);
        // Dropped outside the borrow: closing its timer waits for a callback that is running.
        drop(forgotten);
    }

    pub(crate) fn read_clipboard(&self) -> Result<Option<String>, ClipboardError> {
        self.with_clipboard(|clipboard| clipboard.read())
    }

    pub(crate) fn write_clipboard(
        &self,
        source: &CommandSource,
        text: &str,
    ) -> Result<(), ClipboardError> {
        let foreground = place::foreground(self.service.get());
        if policy::may_write_clipboard(source.authority(self.now()), foreground).is_err() {
            return Err(ClipboardError::NeedsOperator);
        }
        self.with_clipboard(|clipboard| clipboard.write(text))
    }

    pub(crate) fn withdraw_clipboard(&self) -> Result<bool, ClipboardError> {
        self.with_clipboard(|clipboard| clipboard.withdraw())
    }

    fn with_clipboard<T>(
        &self,
        work: impl FnOnce(&mut dyn TextClipboard) -> Result<T, ClipboardError>,
    ) -> Result<T, ClipboardError> {
        match self.clipboard.try_borrow_mut() {
            Ok(mut clipboard) => match &mut *clipboard {
                Some(clipboard) => work(&mut **clipboard),
                None => Err(ClipboardError::Busy),
            },
            // Asked again from inside its own call.
            Err(_) => Err(ClipboardError::Busy),
        }
    }

    pub(crate) fn take_launch_source(&self, id: u64) -> Option<CommandSource> {
        self.launched_by_operator
            .replace(false)
            .then(|| CommandSource::operator_launch(EventTime { nanos: 0 }, id))
    }

    pub(crate) fn begin_shutdown(&self) -> Result<(), Refusal> {
        if self.phase.get() != Phase::Running {
            return Err(Refusal::ShuttingDown);
        }
        self.phase.set(Phase::Draining);
        while let Some(creation) = self.next_creation() {
            self.creation_failed(creation.key, creation.command, CreateFailure::ShuttingDown);
        }
        self.push(Event::ShutdownBegan);
        Ok(())
    }

    pub(crate) fn finish(&self, exit_code: i32) -> Result<(), Refusal> {
        match self.phase.get() {
            Phase::Running => return Err(Refusal::ShutdownNotBegun),
            Phase::Finishing | Phase::Ended => return Err(Refusal::ShuttingDown),
            Phase::Draining => {}
        }
        if self
            .windows
            .borrow()
            .iter()
            .any(|window| window.tokens() > 0)
        {
            return Err(Refusal::NativeTokensHeld);
        }
        self.update_report(|report| report.exit_code = exit_code);
        self.phase.set(Phase::Finishing);
        if self.windows.borrow().is_empty() {
            self.end_when_empty();
        } else {
            self.destroy_windows();
        }
        Ok(())
    }
}
