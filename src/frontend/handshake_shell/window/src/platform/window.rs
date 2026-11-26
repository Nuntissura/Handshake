//! One window, and the hook in front of the window library's window procedure.
//!
//! The hook sees every message first. It keeps a quiet window at the bottom of the Z order whoever asks for something
//! else (the activation guard keeps it inactive), notes where an input came from before the library turns it into an
//! event, keeps from the library what would end its loop, tells the Operator's close from anybody else's, and reports
//! what the library does not.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Controls::WM_MOUSELEAVE;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows_sys::Win32::UI::Input::{GetCurrentInputMessageSource, INPUT_MESSAGE_SOURCE};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetMessageTime, GWL_EXSTYLE, MA_NOACTIVATE, STYLESTRUCT, WA_INACTIVE,
    WINDOWPOS, WM_ACTIVATE, WM_CHAR, WM_DEADCHAR, WM_DPICHANGED, WM_DROPFILES, WM_ENTERMENULOOP,
    WM_ENTERSIZEMOVE, WM_EXITMENULOOP, WM_EXITSIZEMOVE, WM_KEYFIRST, WM_KEYLAST,
    WM_MOUSEACTIVATE, WM_MOUSEFIRST, WM_MOUSELAST, WM_NCDESTROY, WM_NCMOUSEMOVE,
    WM_NCXBUTTONDBLCLK, WM_POINTERDOWN, WM_POINTERUP, WM_POINTERUPDATE, WM_STYLECHANGING,
    WM_SYSCHAR, WM_SYSDEADCHAR, WM_TOUCH, WM_WINDOWPOSCHANGED, WM_WINDOWPOSCHANGING,
};
use winit::dpi::LogicalSize as LibrarySize;
use winit::event_loop::ActiveEventLoop;
use winit::platform::windows::WindowAttributesExtWindows;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{WindowAttributes, WindowId};

use super::closing::{self, Closing, Pending};
use super::key_gate::{self, KeyGate, Kept, Passed};
use super::loop_core::{Core, Creation, MessageOrigin};
use super::place::Target;
use super::retrieval::{self, Arrival};
use super::{drops, guard, last_error, monitors, place, translate};
use crate::command::{CreateFailure, Placement, Refusal, WindowClass, WindowState};
use crate::event::{Display, ModalLoop, WindowEvent, WindowKey};
use crate::frame::FrameLedger;
use crate::geometry::{ClientPoint, LogicalSize, PhysicalPosition, PhysicalSize, ScaleFactor};
use crate::input::{CursorShape, Modifiers};
use crate::monitor::{centered, outside_all, MonitorKey};
use crate::origin::{DeviceKind, InputStamp, Origin};
use crate::token::Win32Window;

const CLASS_NAME: &str = "hsk_shell_window";
const HOOK: usize = 1;

/// What the hook does once the window library handled a message.
#[derive(Default)]
struct After {
    key: Option<Passed>,
}

pub(super) struct Window {
    pub(super) key: WindowKey,
    pub(super) id: WindowId,
    pub(super) hwnd: HWND,
    pub(super) handles: Option<Win32Window>,
    pub(super) class: WindowClass,
    /// One reference per native token, and this one.
    pub(super) hold: Arc<()>,
    pub(super) logical: Cell<LogicalSize>,
    pub(super) scale: Cell<ScaleFactor>,
    /// Requests to close it.
    pub(super) closing: Closing,
    pub(super) frames: RefCell<FrameLedger>,
    /// Where the window goes in the Z order while a show is carried out.
    order_during_show: Cell<Option<HWND>>,
    /// The host was told that it exists. Nothing is reported about it before.
    announced: Cell<bool>,
    pub(super) modifiers: Cell<Modifiers>,
    pub(super) pointer: Cell<Option<ClientPoint>>,
    focused: Cell<bool>,
    display: Cell<Display>,
    monitor: Cell<Option<MonitorKey>>,
    size_reported: Cell<(PhysicalSize, ScaleFactor)>,
    position_reported: Cell<PhysicalPosition>,
    /// Where the newest key message came from. Its text messages are posted by the system and no longer say.
    key_origin: Cell<Option<(Origin, DeviceKind)>>,
    /// What keyboard messages the window library sees.
    keys: KeyGate,
    library: RefCell<Option<winit::window::Window>>,
    core: Weak<Core>,
}

impl Window {
    /// Create a window, hidden. Runs inside a callback of the window library. Nothing is activated meanwhile.
    pub(super) fn create(
        core: &Rc<Core>,
        event_loop: &ActiveEventLoop,
        creation: &Creation,
    ) -> Result<Rc<Self>, CreateFailure> {
        guard::still(|| Self::create_hidden(core, event_loop, creation))
    }

    fn create_hidden(
        core: &Rc<Core>,
        event_loop: &ActiveEventLoop,
        creation: &Creation,
    ) -> Result<Rc<Self>, CreateFailure> {
        let spec = &creation.spec;
        let quiet = spec.class == WindowClass::Quiet;
        let size = |size: LogicalSize| LibrarySize::new(size.width, size.height);
        let mut attributes = WindowAttributes::default()
            .with_title(spec.title.as_str())
            .with_inner_size(size(spec.inner))
            .with_visible(false)
            .with_active(false)
            .with_class_name(CLASS_NAME)
            // The library gives every window a taskbar button unless told, again after Explorer restarts (MT-214 W7).
            .with_skip_taskbar(quiet)
            // Its drop target reports one path at a time and no position.
            .with_drag_and_drop(false);
        if let Some(least) = spec.min_inner {
            attributes = attributes.with_min_inner_size(size(least));
        }
        let library = event_loop
            .create_window(attributes)
            .map_err(|_| CreateFailure::System(last_error()))?;
        let handle = match library.window_handle().map(|handle| handle.as_raw()) {
            Ok(RawWindowHandle::Win32(handle)) => handle,
            _ => return Err(CreateFailure::System(0)),
        };
        let hwnd = handle.hwnd.get();
        if quiet && !guard::mark_quiet(hwnd) {
            place::destroy(hwnd);
            return Err(CreateFailure::System(0));
        }
        let window = Rc::new(Self {
            key: creation.key,
            id: library.id(),
            hwnd,
            handles: handle.hinstance.map(|hinstance| Win32Window {
                hwnd: handle.hwnd,
                hinstance,
            }),
            class: spec.class,
            hold: Arc::new(()),
            logical: Cell::new(spec.inner),
            scale: Cell::new(place::scale(hwnd)),
            closing: Closing::default(),
            frames: RefCell::default(),
            order_during_show: Cell::new(None),
            announced: Cell::new(false),
            modifiers: Cell::default(),
            pointer: Cell::new(None),
            focused: Cell::new(false),
            display: Cell::new(Display::Hidden),
            monitor: Cell::new(None),
            size_reported: Cell::new((PhysicalSize::default(), ScaleFactor::ONE)),
            position_reported: Cell::default(),
            key_origin: Cell::new(None),
            keys: KeyGate::default(),
            library: RefCell::new(Some(library)),
            core: Rc::downgrade(core),
        });
        let prepared = window.hook().and_then(|()| window.prepare(spec.placement));
        if let Err(failure) = prepared {
            place::destroy(hwnd);
            return Err(failure);
        }
        Ok(window)
    }

    fn prepare(&self, placement: Placement) -> Result<(), CreateFailure> {
        let refused = |refusal| match refusal {
            Refusal::System(error) => CreateFailure::System(error),
            _ => CreateFailure::System(0),
        };
        place::apply_class(&self.target()).map_err(refused)?;
        if self.class == WindowClass::Quiet {
            // Its frame is another one now.
            place::resize(self.hwnd, self.logical.get().to_physical(self.scale.get()))
                .map_err(refused)?;
        }
        let outer = place::outer(self.hwnd).size;
        let position = match placement {
            Placement::System => return Ok(()),
            Placement::At(position) => position,
            Placement::OutsideAllMonitors => outside_all(&monitors::all(), outer),
            Placement::Centered(key) => {
                let monitors = monitors::all();
                let monitor = monitors
                    .iter()
                    .find(|monitor| monitor.key == key)
                    .ok_or(CreateFailure::UnknownMonitor)?;
                // The size it will have there.
                let there = crate::monitor::size_on(outer, self.scale.get(), monitor.scale);
                centered(there, monitor.work_area)
            }
        };
        place::move_to(self.hwnd, position).map_err(refused)?;
        let scale = place::scale(self.hwnd);
        if self.scale.replace(scale) != scale {
            place::resize(self.hwnd, self.logical.get().to_physical(scale)).map_err(refused)?;
        }
        Ok(())
    }

    /// Put the hook in front of the window procedure. It holds a reference, given back with the last message.
    fn hook(self: &Rc<Self>) -> Result<(), CreateFailure> {
        let reference = Rc::into_raw(Rc::clone(self));
        // SAFETY: the window is of this thread, and the reference outlives the hook.
        if unsafe { SetWindowSubclass(self.hwnd, Some(hook), HOOK, reference as usize) } == 0 {
            // SAFETY: nothing else took the reference.
            drop(unsafe { Rc::from_raw(reference) });
            return Err(CreateFailure::System(last_error()));
        }
        Ok(())
    }

    pub(super) fn target(&self) -> Target<'_> {
        Target {
            hwnd: self.hwnd,
            class: self.class,
            order_during_show: &self.order_during_show,
        }
    }

    /// The window is gone: let go of the library's handle to it.
    pub(super) fn let_go(&self) {
        let library = self.library.borrow_mut().take();
        drop(library);
    }

    /// The loop is over and a native token still names the window: it stays, hidden, until the thread ends, and the
    /// library is not asked to destroy it.
    pub(super) fn leave(&self) {
        if let Some(library) = self.library.borrow_mut().take() {
            std::mem::forget(library);
        }
    }

    pub(super) fn tokens(&self) -> usize {
        Arc::strong_count(&self.hold).saturating_sub(1)
    }

    pub(super) fn state(&self) -> WindowState {
        WindowState {
            class: self.class,
            display: place::display(self.hwnd),
            focused: self.focused.get(),
            outer: place::outer(self.hwnd),
            inner: self.inner(),
            scale: self.scale.get(),
            monitor: monitors::of_window(self.hwnd),
            native_tokens: self.tokens(),
        }
    }

    /// Empty while minimised, whatever the operating system keeps for it.
    pub(super) fn inner(&self) -> PhysicalSize {
        match place::display(self.hwnd) {
            Display::Minimized => PhysicalSize::default(),
            _ => place::inner(self.hwnd),
        }
    }

    pub(super) fn can_show(&self) -> bool {
        let shown = matches!(
            place::display(self.hwnd),
            Display::Normal | Display::Maximized
        );
        shown && !place::inner(self.hwnd).is_empty()
    }

    pub(super) fn ask_for_paint(&self) {
        if let Some(library) = self.library.borrow().as_ref() {
            library.request_redraw();
        }
    }

    pub(super) fn set_title(&self, title: &str) {
        if let Some(library) = self.library.borrow().as_ref() {
            library.set_title(title);
        }
    }

    pub(super) fn set_cursor(&self, shape: CursorShape) {
        if let Some(library) = self.library.borrow().as_ref() {
            let icon = translate::cursor_icon(shape);
            library.set_cursor_visible(icon.is_some());
            if let Some(icon) = icon {
                library.set_cursor(icon);
            }
        }
    }

    pub(super) fn set_focused(&self, focused: bool) -> bool {
        self.focused.replace(focused) != focused
    }

    /// The stamp of the message that is being handled, with the loop's number for it.
    pub(super) fn stamp(&self, core: &Core) -> InputStamp {
        let message = core.message();
        InputStamp {
            received: core.now(),
            os_millis: message.os_millis,
            origin: message.origin,
            device: message.device,
            modifiers: self.modifiers.get(),
            serial: core.next_serial(),
        }
    }

    /// What the window is like when the host is told that it exists. Changes are reported from here on.
    pub(super) fn announce(&self) -> WindowState {
        let state = self.state();
        self.display.set(state.display);
        self.monitor.set(state.monitor);
        self.size_reported.set((state.inner, state.scale));
        self.position_reported.set(state.outer.origin);
        self.announced.set(true);
        state
    }

    /// Report the size, when it changed. As it is now and not as an event said: the library holds events back while a
    /// window is created, and they arrive when the sizes they carry are gone.
    pub(super) fn report_size(&self, core: &Core) {
        let display = place::display(self.hwnd);
        if display == Display::Minimized {
            return;
        }
        let (inner, scale) = (place::inner(self.hwnd), self.scale.get());
        if display != Display::Maximized && self.logical.get().to_physical(scale) != inner {
            // The Operator or the system chose another one.
            self.logical.set(inner.to_logical(scale));
        }
        if self.size_reported.replace((inner, scale)) != (inner, scale) {
            core.push_window(self.key, WindowEvent::Resized { inner, scale });
        }
    }

    /// Report the position, when it changed.
    pub(super) fn report_position(&self, core: &Core) {
        if place::display(self.hwnd) == Display::Minimized {
            return;
        }
        let outer = place::outer(self.hwnd).origin;
        if self.position_reported.replace(outer) != outer {
            core.push_window(self.key, WindowEvent::Moved { outer });
        }
    }

    /// Report how the window is displayed and on which monitor, when that changed.
    fn report_display(&self, core: &Core) {
        if !self.announced.get() {
            return;
        }
        let display = place::display(self.hwnd);
        if self.display.replace(display) != display {
            core.push_window(self.key, WindowEvent::DisplayChanged { display });
            if matches!(display, Display::Normal | Display::Maximized)
                && self.frames.borrow().wants_paint()
            {
                self.ask_for_paint();
            }
        }
        if display == Display::Minimized {
            return;
        }
        let monitor = monitors::of_window(self.hwnd);
        if self.monitor.replace(monitor) != monitor {
            core.push_window(self.key, WindowEvent::MonitorChanged { monitor });
        }
    }

    /// A quiet window told it is active is broken only when it really is: refused activations still send
    /// `WM_ACTIVATE` on the way back from minimised (MT-214 evaluation probe).
    fn check_quiet(&self, core: &Core, wparam: WPARAM) {
        // SAFETY: no arguments, read only.
        let active = unsafe { GetActiveWindow() } == self.hwnd;
        if self.class == WindowClass::Quiet && wparam as u32 & 0xFFFF != WA_INACTIVE && active {
            core.update_report(|report| report.quiet_broken = report.quiet_broken.saturating_add(1));
            core.push_window(self.key, WindowEvent::QuietBroken);
        }
    }

    /// Before the window library sees a message: what it says of a request to close the window (`closing.rs`). Whatever
    /// the hook then does with the message, [`Window::close_after`] follows.
    fn close_before(
        &self,
        core: &Core,
        message: u32,
        wparam: WPARAM,
        arrival: Arrival,
    ) -> Option<Pending> {
        let operators_input =
            arrival == Arrival::Queue && core.message().origin == Origin::OperatorDevice;
        self.closing.before(core.now(), message, wparam, arrival, operators_input, || {
            closing::queued_close(self.hwnd)
        })
    }

    fn close_after(&self, core: &Core, pending: Pending) {
        self.closing
            .after(core.now(), pending, || closing::queued_close(self.hwnd));
    }

    /// Before the window library sees a message. An error answers it in the library's place.
    fn before(
        &self,
        core: &Core,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        arrival: Arrival,
    ) -> Result<After, LRESULT> {
        match message {
            WM_WINDOWPOSCHANGING => {
                // SAFETY: the message carries a writable WINDOWPOS.
                place::keep_quiet(&self.target(), unsafe { &mut *(lparam as *mut WINDOWPOS) });
            }
            WM_STYLECHANGING if wparam as i32 == GWL_EXSTYLE => {
                // SAFETY: the message carries a writable STYLESTRUCT.
                let style = unsafe { &mut *(lparam as *mut STYLESTRUCT) };
                style.styleNew = place::class_style(self.class, style.styleNew);
            }
            WM_MOUSEACTIVATE if self.class == WindowClass::Quiet => return Err(MA_NOACTIVATE as LRESULT),
            WM_ENTERSIZEMOVE | WM_EXITSIZEMOVE => {
                self.modal_loop(core, ModalLoop::MoveOrSize, message == WM_ENTERSIZEMOVE);
            }
            WM_ENTERMENULOOP | WM_EXITMENULOOP => {
                self.modal_loop(core, ModalLoop::Menu, message == WM_ENTERMENULOOP);
            }
            WM_DROPFILES => {
                let (dropped, position) = drops::take(wparam);
                core.push_window(
                    self.key,
                    WindowEvent::FilesDropped(dropped.into_event(position, self.stamp(core))),
                );
                return Err(0);
            }
            _ => {
                return Ok(After {
                    key: self.gate_keys(core, message, wparam, arrival)?,
                })
            }
        }
        Ok(After::default())
    }

    /// Once the window library handled the message `before` let through.
    fn after(&self, after: After) {
        if let Some(passed) = after.key {
            self.keys.done(passed);
        }
    }

    /// Keep from the window library what would end its loop or start a key nobody typed (`key_gate.rs`).
    fn gate_keys(
        &self,
        core: &Core,
        message: u32,
        wparam: WPARAM,
        arrival: Arrival,
    ) -> Result<Option<Passed>, LRESULT> {
        let sent = arrival == Arrival::OtherThread;
        let checked = self.keys.check(message, wparam, sent, || {
            key_gate::next_key_message(self.hwnd)
        });
        checked.map_err(|kept| {
            if kept == Kept::Sent {
                core.update_report(|report| {
                    report.sent_keys_refused = report.sent_keys_refused.saturating_add(1);
                });
            }
            0
        })
    }

    fn modal_loop(&self, core: &Core, kind: ModalLoop, began: bool) {
        core.push_window(self.key, WindowEvent::ModalLoop { kind, began });
    }

    /// Where an input message came from; nothing for any other message.
    fn origin_of(&self, message: u32) -> MessageOrigin {
        let text = matches!(message, WM_CHAR | WM_DEADCHAR | WM_SYSCHAR | WM_SYSDEADCHAR);
        let keyboard = (WM_KEYFIRST..=WM_KEYLAST).contains(&message);
        let pointer = (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&message)
            || (WM_NCMOUSEMOVE..=WM_NCXBUTTONDBLCLK).contains(&message)
            || matches!(
                message,
                WM_MOUSELEAVE | WM_POINTERDOWN | WM_POINTERUP | WM_POINTERUPDATE | WM_TOUCH
            );
        if !keyboard && !pointer && message != WM_DROPFILES {
            return MessageOrigin::NONE;
        }
        let mut source = INPUT_MESSAGE_SOURCE {
            deviceType: 0,
            originId: 0,
        };
        // SAFETY: the structure outlives the calls, which only read.
        let (known, time) = unsafe {
            (
                GetCurrentInputMessageSource(&mut source) != 0,
                GetMessageTime(),
            )
        };
        let mut from = if known {
            translate::message_origin(source.originId, source.deviceType)
        } else {
            (Origin::Unattributed, DeviceKind::Unknown)
        };
        if text {
            if let (Origin::Unattributed, Some(key)) = (from.0, self.key_origin.get()) {
                from = key;
            }
        } else if keyboard {
            self.key_origin.set(Some(from));
        }
        MessageOrigin {
            os_millis: Some(time as u32),
            origin: from.0,
            device: from.1,
        }
    }
}

unsafe extern "system" fn hook(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    reference: usize,
) -> LRESULT {
    // SAFETY: `Window::hook` left a reference here, and it is given back below, after the last message. This one keeps
    // the window alive until the procedure returns.
    let window = unsafe {
        let window = reference as *const Window;
        Rc::increment_strong_count(window);
        Rc::from_raw(window)
    };
    let Some(core) = window.core.upgrade() else {
        // The loop is over and this window was left for a token (`Window::leave`): the library is not entered again.
        if message == WM_NCDESTROY {
            // SAFETY: as below.
            unsafe { release(hwnd, reference) };
        }
        // SAFETY: the message as it came.
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    };
    let arrival = retrieval::arrival(hwnd, message, wparam, lparam);
    // Nested messages have origins of their own; this one's is put back when they are done.
    let outer = core.set_message(window.origin_of(message));
    let close = core
        .guarded(|| window.close_before(&core, message, wparam, arrival))
        .flatten();
    let before = core.guarded(|| window.before(&core, message, wparam, lparam, arrival));
    let (answer, after) = match before {
        Some(Ok(after)) => (None, after),
        Some(Err(result)) => (Some(result), After::default()),
        None => (None, After::default()),
    };
    let result = match answer {
        Some(result) => result,
        // The library may restore, and so activate, a window whose scale changes (MT-214 W3).
        // SAFETY: the message as it came.
        None if message == WM_DPICHANGED => {
            guard::still(|| unsafe { DefSubclassProc(hwnd, message, wparam, lparam) })
        }
        // SAFETY: the message as it came.
        None => unsafe { DefSubclassProc(hwnd, message, wparam, lparam) },
    };
    window.after(after);
    if let Some(pending) = close {
        core.guarded(|| window.close_after(&core, pending));
    }
    match message {
        WM_WINDOWPOSCHANGED => {
            core.guarded(|| window.report_display(&core));
        }
        WM_ACTIVATE => {
            core.guarded(|| window.check_quiet(&core, wparam));
        }
        _ => {}
    }
    core.set_message(outer);
    if message == WM_NCDESTROY {
        guard::forget(hwnd);
        // SAFETY: the hook that `Window::hook` installed, and the reference it left.
        unsafe { release(hwnd, reference) };
    }
    result
}

/// Remove the hook and give back the reference it held.
///
/// # Safety
///
/// `reference` is the one `Window::hook` installed for `hwnd`, given back once.
unsafe fn release(hwnd: HWND, reference: usize) {
    // SAFETY: as the caller says.
    unsafe {
        RemoveWindowSubclass(hwnd, Some(hook), HOOK);
        drop(Rc::from_raw(reference as *const Window));
    }
}
