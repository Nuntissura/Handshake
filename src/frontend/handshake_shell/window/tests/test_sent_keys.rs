//! MT-214 E3-F5: keys that a loop takes out of the queue and dispatches while it runs inside a message another thread
//! sent reach the window library and are not counted as sent; keys another thread sends are refused and counted. The
//! loop here is the test's own, filtered to the window's keyboard messages, as a dialog's or the system's move loop
//! that the shell (taskbar "Move") starts with a sent `WM_SYSCOMMAND`. A process has one loop, so this is a target of
//! its own.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_window::{
    CommandSource, Control, Event, KeyCode, LogicalSize, LoopConfig, Placement, PressState,
    WakeLane, WindowClass, WindowEvent, WindowKey, WindowSpec,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, PeekMessageW,
    PostMessageW, RegisterClassW, MSG, PM_REMOVE, WM_APP, WM_CHAR, WM_KEYDOWN, WM_KEYFIRST,
    WM_KEYLAST, WM_KEYUP, WNDCLASSW, WS_POPUP,
};

use support::{
    must_stay_inactive, on_private_desktop, run_script, send_from_another_thread, stop_watching,
    wide, Log,
};

/// Sent to the test's window from another thread: run a loop inside it.
const RUN_NESTED: u32 = WM_APP + 7;

/// The key A: virtual key, and the first parameter with its scan code, pressed and released.
const KEY_A: WPARAM = 0x41;
const A_DOWN: LPARAM = 0x001E_0001;
const A_UP: LPARAM = 0xC01E_0001_u32 as LPARAM;

thread_local! {
    /// The shell window whose keyboard messages the nested loop takes out.
    static SHELL: Cell<HWND> = const { Cell::new(0) };
    static NESTED_RAN: Cell<bool> = const { Cell::new(false) };
}

unsafe extern "system" fn nested(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message != RUN_NESTED {
        // SAFETY: the message as it came.
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }
    NESTED_RAN.set(true);
    // SAFETY: a message structure of this function; only the shell window's keyboard messages are taken out, each
    // dispatched as a modal loop does.
    unsafe {
        let mut taken: MSG = std::mem::zeroed();
        while PeekMessageW(&mut taken, SHELL.get(), WM_KEYFIRST, WM_KEYLAST, PM_REMOVE) != 0 {
            DispatchMessageW(&taken);
        }
    }
    0
}

/// A hidden window of the test's, on the loop's thread, whose procedure runs the nested loop.
fn nested_window() -> HWND {
    let class = wide("hsk_test_nested_loop");
    // SAFETY: the class name outlives the calls; a hidden window of this thread on the private desktop.
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let mut description: WNDCLASSW = std::mem::zeroed();
        description.lpfnWndProc = Some(nested);
        description.hInstance = instance;
        description.lpszClassName = class.as_ptr();
        RegisterClassW(&description);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            ptr::null(),
            WS_POPUP,
            0,
            0,
            10,
            10,
            0,
            0,
            instance,
            ptr::null(),
        );
        assert_ne!(hwnd, 0);
        hwnd
    }
}

#[derive(Default)]
struct State {
    lane: RefCell<Option<WakeLane>>,
    window: Cell<Option<WindowKey>>,
    nested: Cell<HWND>,
    step: Cell<u32>,
    keys: Log<(KeyCode, bool)>,
    texts: Log<String>,
}

impl State {
    fn next(&self) {
        self.step.set(self.step.get() + 1);
        let lane = self.lane.borrow();
        lane.as_ref().unwrap().request(0, Duration::from_millis(20)).unwrap();
    }

    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                *self.lane.borrow_mut() = Some(control.open_wake_lane().unwrap());
                self.nested.set(nested_window());
                let spec = WindowSpec {
                    title: "test".to_owned(),
                    inner: LogicalSize::new(200.0, 100.0),
                    min_inner: None,
                    placement: Placement::At(handshake_shell_window::PhysicalPosition::new(30, 30)),
                    class: WindowClass::Quiet,
                };
                let key = control.create_window(&CommandSource::host(1), spec).unwrap();
                self.window.set(Some(key));
            }
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => {
                let hwnd = control.native_window(window).unwrap().win32().hwnd.get();
                SHELL.set(hwnd);
                must_stay_inactive(hwnd);
                self.next();
            }
            Event::Window {
                event: WindowEvent::Key(key),
                ..
            } => self.keys.add((key.physical.code, key.state == PressState::Pressed)),
            Event::Window {
                event: WindowEvent::Text(text),
                ..
            } => self.texts.add(text.text.as_str().to_owned()),
            Event::Window {
                event: WindowEvent::Destroyed,
                ..
            } => {
                stop_watching(SHELL.get());
                control.begin_shutdown().unwrap();
            }
            Event::ShutdownBegan => control.finish(0).unwrap(),
            _ => {}
        }
    }

    fn step(&self, control: &Control) {
        let shell = SHELL.get();
        match self.step.get() {
            1 => {
                // The Operator's key A, as the input system posts it: down, its character, up. Then another thread
                // sends the message that runs a loop, and that loop takes the keys out.
                // SAFETY: plain values, to a window of this thread.
                unsafe {
                    for (message, wparam, lparam) in
                        [(WM_KEYDOWN, KEY_A, A_DOWN), (WM_CHAR, 'a' as usize, A_DOWN), (WM_KEYUP, KEY_A, A_UP)]
                    {
                        assert_ne!(PostMessageW(shell, message, wparam, lparam), 0);
                    }
                }
                let sent = control.report().sent_keys_refused;
                send_from_another_thread(self.nested.get(), vec![(RUN_NESTED, 0, 0)]);
                assert!(NESTED_RAN.get(), "the loop ran inside the sent message");
                assert_eq!(control.report().sent_keys_refused, sent, "taken out of the queue: not sent");
                // Keys another thread sends: refused and counted.
                let keys = vec![(WM_KEYDOWN, KEY_A, A_DOWN), (WM_KEYUP, KEY_A, A_UP)];
                send_from_another_thread(shell, keys);
                assert_eq!(control.report().sent_keys_refused, sent + 2);
            }
            2 => {
                // The events of the dispatched keys came; none of the sent ones.
                assert_eq!(
                    self.keys.all(),
                    [(KeyCode::KeyA, true), (KeyCode::KeyA, false)],
                    "the key down and up the loop dispatched, and nothing else"
                );
                assert_eq!(self.texts.all(), ["a"]);
                let window = self.window.get().unwrap();
                let destroy = handshake_shell_window::WindowCommand::Destroy;
                control.command(&CommandSource::host(2), window, destroy).unwrap();
                // SAFETY: the test's own window, of this thread.
                unsafe { DestroyWindow(self.nested.get()) };
                return;
            }
            other => panic!("step {other}"),
        }
        self.next();
    }
}

#[test]
fn keys_a_loop_inside_a_sent_message_takes_out_are_not_refused_as_sent() {
    on_private_desktop(Duration::from_secs(60), || {
        let state = Rc::new(State::default());
        let (events, steps) = (Rc::clone(&state), Rc::clone(&state));
        let report = run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            move |control, _| steps.step(control),
        )
        .unwrap();
        assert_eq!(state.step.get(), 2, "every step ran");
        assert_eq!(report.sent_keys_refused, 2);
        assert_eq!(report.quiet_broken, 0);
    });
}
