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

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, PostQuitMessage, MSG, PM_REMOVE, WM_QUIT,
};
use winit::event_loop::EventLoopProxy;

use super::window::Window;
use super::{clipboard, commands, guard, place, wake::Lane};
use crate::clipboard::{ClipboardError, TextClipboard};
use crate::command::{CreateFailure, Refusal, WindowSpec, WindowState};
use crate::event::{Event, WindowEvent, WindowKey};
use crate::host::{Control, LoopConfig, LoopError, RunReport, ShellHost};
use crate::origin::{Authority, AuthorityLedger, CommandSource, DeviceKind, EventTime, Origin};
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
    /// The number of the newest input stamp.
    serial: Cell<u64>,
    authority: RefCell<AuthorityLedger>,
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
            serial: Cell::new(0),
            authority: RefCell::default(),
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
        // The host's events are input; this lane does not wait for more.
        let lane = Lane::open(EVENT_LANE, service, self.thread, self.epoch, false)
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
        self.report()
    }

    /// What the run was like so far.
    pub(super) fn report(&self) -> RunReport {
        self.update_report(|report| report.activations_refused = guard::refused());
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

    /// A number for a new input stamp. Never 0.
    pub(super) fn next_serial(&self) -> u64 {
        let serial = self.serial.get().saturating_add(1);
        self.serial.set(serial);
        serial
    }

    pub(super) fn authority_of(&self, source: &CommandSource) -> Authority {
        self.authority.borrow().authority(source, self.now())
    }

    pub(super) fn use_up(&self, source: &CommandSource) {
        self.authority.borrow_mut().use_up(source);
    }

    pub(super) fn service(&self) -> HWND {
        self.service.get()
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

    /// The service window got a wake message out of the queue, with the number it was posted with.
    pub(super) fn dispatch_wake(self: &Rc<Self>, id: u32, token: LPARAM) {
        let Some(lane) = self.lane(id) else { return };
        let Some(taken) = lane.take(token) else { return };
        if id == EVENT_LANE {
            self.drain(EVENTS_PER_TURN);
            return;
        }
        // From `finish` on the windows are going or gone, also while a loop inside the host still dispatches
        // (MT-214 W6): no wake reaches the host.
        if !matches!(self.phase.get(), Phase::Running | Phase::Draining) {
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

    /// A window that exists. One whose handle is gone, and whose end the library did not report yet, is none.
    pub(super) fn window_by_key(&self, key: WindowKey) -> Option<Rc<Window>> {
        self.windows
            .borrow()
            .iter()
            .find(|window| window.key == key && place::exists(window.hwnd))
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

    /// Destroy whatever is left, without asking. For the end of the loop. A window a native token still names is only
    /// hidden: a surface must not outlive its window (MT-214 W7).
    pub(super) fn destroy_windows(&self) {
        let windows: Vec<_> = self.windows.borrow().clone();
        for window in windows {
            if window.tokens() > 0 {
                place::hide(window.hwnd);
            } else {
                place::destroy(window.hwnd);
            }
        }
    }

    /// Let go of windows whose last event nobody will translate any more. One a token still names stays until the
    /// thread ends.
    pub(super) fn forget_windows(&self) {
        let windows = std::mem::take(&mut *self.windows.borrow_mut());
        let mut left = 0;
        for window in windows {
            if window.tokens() > 0 && place::exists(window.hwnd) {
                window.leave();
                left += 1;
            } else {
                window.let_go();
            }
        }
        self.update_report(|report| report.windows_left = left);
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

    pub(super) fn counted<T>(&self, outcome: Result<T, Refusal>) -> Result<T, Refusal> {
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

    /// None once `finish` began destroying windows: a token then could outlive its window.
    pub(crate) fn native_window(&self, key: WindowKey) -> Option<NativeWindowToken> {
        if !matches!(self.phase.get(), Phase::Running | Phase::Draining) {
            return None;
        }
        let window = self.window_by_key(key)?;
        Some(NativeWindowToken::new(
            key,
            window.handles?,
            Arc::clone(&window.hold),
        ))
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
            .and_then(|id| Lane::open(id, self.service.get(), self.thread, self.epoch, true))
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
        commands::may_write_clipboard(self, source)?;
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
