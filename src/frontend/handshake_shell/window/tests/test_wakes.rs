//! Wakes of a real loop: on its thread, in order, when due, through a full queue, and inside loops that run inside the
//! host. One test, because a process has one loop.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use handshake_shell_window::{
    Control, Event, LaneWaker, LoopConfig, LoopError, WakeLane, WakeRefused,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DialogBoxIndirectParamW, DispatchMessageW, EndDialog, FindWindowW, GetMessageW, PostMessageW,
    TranslateMessage, DLGTEMPLATE, MSG, WM_DISPLAYCHANGE, WM_INITDIALOG, WM_NULL, WS_POPUP,
};

use support::{on_private_desktop, run_script, wide, Decoy, Log};

const LIMIT: Duration = Duration::from_secs(60);
const DIALOG_RESULT: isize = 7;

thread_local! {
    static DIALOG: Cell<HWND> = const { Cell::new(0) };
}

unsafe extern "system" fn dialog(hwnd: HWND, message: u32, _: WPARAM, _: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        DIALOG.set(hwnd);
    }
    0
}

/// A dialog without controls, as the memory layout the operating system reads: template, no menu, no class, no title.
#[repr(C, align(4))]
struct EmptyDialog {
    template: DLGTEMPLATE,
    ends: [u16; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Nested {
    No,
    PlainLoop,
    Dialog,
}

struct State {
    thread: ThreadId,
    a: RefCell<Option<WakeLane>>,
    b: RefCell<Option<WakeLane>>,
    waker: RefCell<Option<LaneWaker>>,
    requested_at: Cell<Instant>,
    decoy: RefCell<Option<Decoy>>,
    wakes: Log<u64>,
    events: Log<&'static str>,
    in_event: Cell<u32>,
    nested: Cell<Nested>,
    nested_wakes: Cell<u32>,
    plain_loop_ends: Cell<bool>,
}

impl State {
    fn request(&self, value: u64, delay_ms: u64) {
        self.requested_at.set(Instant::now());
        let lane = self.a.borrow();
        lane.as_ref()
            .unwrap()
            .request(value, Duration::from_millis(delay_ms))
            .unwrap();
    }

    /// Fill this thread's message queue until the operating system takes no more.
    fn fill_queue(&self) {
        let decoy = self.decoy.borrow().as_ref().unwrap().0;
        for _ in 0..200_000 {
            // SAFETY: a message that does nothing, to a window of this thread.
            if unsafe { PostMessageW(decoy, WM_NULL, 0, 0) } == 0 {
                return;
            }
        }
        panic!("the queue never filled, so the refused post is not tested");
    }

    /// Three wakes have to arrive inside each nested loop.
    fn nested_wake(&self) -> bool {
        let count = self.nested_wakes.get() + 1;
        self.nested_wakes.set(count);
        count >= 3
    }

    fn plain_loop(&self) {
        self.nested.set(Nested::PlainLoop);
        self.nested_wakes.set(0);
        self.request(30, 5);
        while !self.plain_loop_ends.get() {
            // SAFETY: an ordinary message loop of this thread.
            unsafe {
                let mut message: MSG = std::mem::zeroed();
                assert!(GetMessageW(&mut message, 0, 0, 0) > 0);
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        self.nested.set(Nested::No);
        assert_eq!(self.nested_wakes.get(), 3);
    }

    fn modal_dialog(&self) {
        self.nested.set(Nested::Dialog);
        self.nested_wakes.set(0);
        self.request(40, 5);
        let empty = EmptyDialog {
            template: DLGTEMPLATE {
                style: WS_POPUP,
                dwExtendedStyle: 0,
                cdit: 0,
                x: 0,
                y: 0,
                cx: 20,
                cy: 20,
            },
            ends: [0; 3],
        };
        // SAFETY: a complete template that outlives the call, which returns when the dialog ends.
        let result =
            unsafe { DialogBoxIndirectParamW(0, ptr::addr_of!(empty).cast(), 0, Some(dialog), 0) };
        self.nested.set(Nested::No);
        assert_eq!(result, DIALOG_RESULT, "ended by the wake");
        assert_eq!(self.nested_wakes.get(), 3);
    }

    fn event(&self, control: &Control, event: Event) {
        assert_eq!(
            self.in_event.replace(1),
            0,
            "an event handler was entered twice"
        );
        match event {
            Event::LoopStarted => {
                assert!(
                    self.wakes.all().is_empty(),
                    "no wake before the loop started"
                );
                *self.decoy.borrow_mut() = Some(Decoy::create());
                let a = control.open_wake_lane().unwrap();
                let b = control.open_wake_lane().unwrap();
                assert!(a.on_loop_thread());
                assert_ne!(a.id(), b.id());
                *self.waker.borrow_mut() = Some(a.waker());
                *self.a.borrow_mut() = Some(a);
                *self.b.borrow_mut() = Some(b);
                self.request(1, 0);
                assert!(self.wakes.all().is_empty(), "never from inside the request");
                self.events.add("started");
            }
            Event::ShutdownBegan => {
                self.plain_loop();
                // An event that comes up now has to wait for this handler.
                let service = wide("hsk_shell_service");
                // SAFETY: the name outlives the call; a window of this thread.
                unsafe {
                    let service = FindWindowW(service.as_ptr(), ptr::null());
                    assert_ne!(service, 0);
                    assert_ne!(PostMessageW(service, WM_DISPLAYCHANGE, 0, 0), 0);
                }
                self.modal_dialog();
                self.events.add("shutdown");
                control.finish(5).unwrap();
            }
            Event::MonitorsChanged => self.events.add("monitors"),
            Event::LoopEnding => {
                self.events.add("ending");
                self.decoy.borrow_mut().take();
            }
            other => panic!("unexpected {other:?}"),
        }
        self.in_event.set(0);
    }

    fn wake(&self, control: &Control, value: u64, inside_host: bool) {
        assert_eq!(std::thread::current().id(), self.thread);
        assert_eq!(inside_host, self.nested.get() != Nested::No, "wake {value}");
        self.wakes.add(value);
        match value {
            1 => {
                self.request(2, 60);
            }
            2 => {
                assert!(
                    self.requested_at.get().elapsed() >= Duration::from_millis(60),
                    "not before it is due"
                );
                let waker = self.waker.borrow().clone().unwrap();
                // The loop is in here meanwhile, so all three are under way at once.
                std::thread::spawn(move || {
                    for value in [10, 11, 12] {
                        waker.wake(value).unwrap();
                    }
                })
                .join()
                .unwrap();
            }
            12 => {
                self.request(13, 30);
                self.a.borrow().as_ref().unwrap().cancel();
                let lane = self.b.borrow();
                lane.as_ref()
                    .unwrap()
                    .request(14, Duration::from_millis(150))
                    .unwrap();
            }
            14 => {
                // The post of the next wake is refused, and so is every other until the loop has caught up.
                self.fill_queue();
                self.request(20, 0);
            }
            20 => {
                assert_eq!(
                    control.finish(1).unwrap_err().code(),
                    "window.shutdown_not_begun"
                );
                control.begin_shutdown().unwrap();
                assert_eq!(
                    control.begin_shutdown().unwrap_err().code(),
                    "window.shutting_down"
                );
                // Dispatched during shutdown, by the loop that the shutdown handler runs.
                let lane = self.b.borrow();
                lane.as_ref().unwrap().request(21, Duration::ZERO).unwrap();
            }
            21 => {}
            30 => {
                if self.nested_wake() {
                    self.plain_loop_ends.set(true);
                } else {
                    self.request(30, 5);
                }
            }
            40 => {
                if self.nested_wake() {
                    // SAFETY: the dialog that is open on this thread.
                    unsafe { EndDialog(DIALOG.get(), DIALOG_RESULT) };
                } else {
                    self.request(40, 5);
                }
            }
            other => panic!("wake {other} was replaced or cancelled"),
        }
    }
}

#[test]
fn a_wake_is_dispatched_by_every_loop_of_its_thread_and_by_nothing_else() {
    on_private_desktop(LIMIT, || {
        let state = Rc::new(State {
            thread: std::thread::current().id(),
            a: RefCell::default(),
            b: RefCell::default(),
            waker: RefCell::default(),
            requested_at: Cell::new(Instant::now()),
            decoy: RefCell::default(),
            wakes: Log::new(),
            events: Log::new(),
            in_event: Cell::new(0),
            nested: Cell::new(Nested::No),
            nested_wakes: Cell::new(0),
            plain_loop_ends: Cell::new(false),
        });
        let (events, wakes) = (Rc::clone(&state), Rc::clone(&state));
        let report = run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            move |control, wake| wakes.wake(control, wake.value(), wake.inside_host_call()),
        )
        .unwrap();

        let seen = state.wakes.all();
        assert_eq!(&seen[..6], &[1, 2, 12, 14, 20, 21], "{seen:?}");
        assert_eq!(state.wakes.count(&30), 3);
        assert_eq!(state.wakes.count(&40), 3);
        let events = state.events.all();
        let own: Vec<_> = events
            .iter()
            .filter(|event| **event != "monitors")
            .collect();
        assert_eq!(own, [&"started", &"shutdown", &"ending"]);
        let waited = events
            .iter()
            .skip_while(|event| **event != "shutdown")
            .any(|event| *event == "monitors");
        assert!(
            waited,
            "the event that came up in the handler arrived after it: {events:?}"
        );
        assert_eq!(report.exit_code, 5);
        assert_eq!(report.wakes, seen.len() as u64);

        // After the loop, from this thread and from another.
        let waker = state.waker.borrow().clone().unwrap();
        assert_eq!(waker.wake(1), Err(WakeRefused::Closed));
        assert_eq!(
            std::thread::spawn(move || waker.wake(1)).join().unwrap(),
            Err(WakeRefused::Closed)
        );
        let lane = state.a.borrow_mut().take().unwrap();
        assert_eq!(lane.request(1, Duration::ZERO), Err(WakeRefused::Closed));
        assert_eq!(
            run_script(LoopConfig::default(), |_, _| {}, |_, _| {}),
            Err(LoopError::AlreadyCreated)
        );
    });
}
