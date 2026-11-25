//! A real window crosses scales: the operating system's own message for it, sent to a window of this process on a
//! desktop that receives no input. Last, while it is hidden and maximised: the window library must not restore it,
//! which would show and activate it (MT-214 W3).
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_window::{
    CommandSource, Control, Display, Event, FrameOutcome, LogicalSize, LoopConfig,
    PhysicalPosition, Placement, ScaleFactor, WakeLane, WindowClass, WindowCommand, WindowEvent,
    WindowKey, WindowSpec,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, IsWindowVisible, IsZoomed, SendMessageW, ShowWindow, SW_SHOWMAXIMIZED,
    WM_DPICHANGED,
};

use support::{must_stay_inactive, on_private_desktop, run_script, stop_watching, Log};

/// A size that does not survive rounding at each scale.
const INNER: LogicalSize = LogicalSize::new(267.0, 151.0);

struct State {
    lane: RefCell<Option<WakeLane>>,
    window: Cell<Option<WindowKey>>,
    hwnd: Cell<HWND>,
    /// Scales still to cross, the last one first.
    scales: RefCell<Vec<u32>>,
    changes: Log<(u32, u32)>,
    sizes: Log<(u32, u32, u32)>,
    frames: Cell<u32>,
    /// The scales were crossed; the window is maximised and hidden now.
    maximised: Cell<bool>,
}

impl State {
    fn next(&self) {
        let lane = self.lane.borrow();
        lane.as_ref()
            .unwrap()
            .request(1, Duration::from_millis(15))
            .unwrap();
    }

    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                *self.lane.borrow_mut() = Some(control.open_wake_lane().unwrap());
                let spec = WindowSpec {
                    title: "test".to_owned(),
                    inner: INNER,
                    min_inner: None,
                    placement: Placement::At(PhysicalPosition::new(30, 30)),
                    class: WindowClass::Quiet,
                };
                let key = control
                    .create_window(&CommandSource::host(1), spec)
                    .unwrap();
                self.window.set(Some(key));
            }
            Event::Window { window, event } => match event {
                WindowEvent::Created { state, .. } => {
                    let token = control.native_window(window).unwrap();
                    self.hwnd.set(token.win32().hwnd.get());
                    must_stay_inactive(self.hwnd.get());
                    let own = state.scale.dpi();
                    // Back and forth, and home at the end.
                    let mut scales = vec![own];
                    for round in 0..12 {
                        scales.push([120, 144, 168, 192, 96][round % 5]);
                    }
                    scales.dedup();
                    if scales.last() == Some(&own) {
                        scales.pop();
                    }
                    *self.scales.borrow_mut() = scales;
                    control
                        .command(
                            &CommandSource::host(2),
                            window,
                            WindowCommand::Show { activate: false },
                        )
                        .unwrap();
                    control.request_frame(window).unwrap();
                    self.next();
                }
                WindowEvent::ScaleChanged { from, to } if !self.maximised.get() => {
                    self.changes.add((from.dpi(), to.dpi()))
                }
                WindowEvent::Resized { inner, scale } if !self.maximised.get() => {
                    self.sizes.add((scale.dpi(), inner.width, inner.height))
                }
                WindowEvent::FrameDue { ticket, .. } => {
                    self.frames.set(self.frames.get() + 1);
                    control
                        .frame_done(window, ticket, FrameOutcome::Presented)
                        .unwrap();
                }
                WindowEvent::Destroyed => {
                    stop_watching(self.hwnd.get());
                    control.begin_shutdown().unwrap()
                }
                _ => {}
            },
            Event::ShutdownBegan => control.finish(0).unwrap(),
            _ => {}
        }
    }

    fn send_scale(&self, dpi: u32) {
        let mut area = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: the rectangle outlives both calls; the message is the operating system's, to a window of this thread.
        unsafe {
            GetWindowRect(self.hwnd.get(), &mut area);
            let both = ((dpi as usize) << 16) | dpi as usize;
            SendMessageW(
                self.hwnd.get(),
                WM_DPICHANGED,
                both,
                &area as *const RECT as LPARAM,
            );
        }
    }

    /// Maximised (the guard keeps it inactive), hidden, then a scale change: it stays hidden and maximised.
    fn hidden_and_maximised(&self, control: &Control) {
        let window = self.window.get().unwrap();
        let hwnd = self.hwnd.get();
        self.maximised.set(true);
        // SAFETY: plain values, for a window of this thread on the private desktop.
        unsafe { ShowWindow(hwnd, SW_SHOWMAXIMIZED) };
        control
            .command(&CommandSource::host(4), window, WindowCommand::Hide)
            .unwrap();
        let state = control.window_state(window).unwrap();
        assert_eq!(state.display, Display::Hidden);
        let dpi = state.scale.dpi();
        // SAFETY: a plain value, read only.
        assert_ne!(unsafe { IsZoomed(hwnd) }, 0, "maximised while hidden");
        self.send_scale(if dpi == 144 { 120 } else { 144 });
        // SAFETY: plain values, read only.
        unsafe {
            assert_eq!(IsWindowVisible(hwnd), 0, "still hidden");
            assert_ne!(
                IsZoomed(hwnd),
                0,
                "still maximised: the library did not restore it with SW_RESTORE"
            );
            assert_ne!(GetActiveWindow(), hwnd);
        }
    }

    fn cross(&self, control: &Control) {
        let window = self.window.get().unwrap();
        let Some(dpi) = self.scales.borrow_mut().pop() else {
            if !self.maximised.get() {
                let progress = control.frame_progress(window).unwrap();
                assert!(progress.presented >= 1, "a frame that was asked for is due");
                self.hidden_and_maximised(control);
                self.next();
                return;
            }
            control
                .command(&CommandSource::host(3), window, WindowCommand::Destroy)
                .unwrap();
            return;
        };
        self.send_scale(dpi);
        let state = control.window_state(window).unwrap();
        let scale = ScaleFactor::from_dpi(dpi).unwrap();
        assert_eq!(state.scale, scale);
        assert_eq!(
            state.inner,
            INNER.to_physical(scale),
            "the logical size stays, at every scale"
        );
        self.next();
    }
}

#[test]
fn crossing_scales_keeps_the_logical_size_exactly() {
    on_private_desktop(Duration::from_secs(60), || {
        let state = Rc::new(State {
            lane: RefCell::default(),
            window: Cell::new(None),
            hwnd: Cell::new(0),
            scales: RefCell::default(),
            changes: Log::new(),
            sizes: Log::new(),
            frames: Cell::new(0),
            maximised: Cell::new(false),
        });
        let (events, wakes) = (Rc::clone(&state), Rc::clone(&state));
        run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            move |control, _| wakes.cross(control),
        )
        .unwrap();

        let changes = state.changes.all();
        assert!(changes.len() >= 10, "{changes:?}");
        assert!(changes.iter().all(|(from, to)| from != to));
        assert!(
            changes.windows(2).all(|pair| pair[0].1 == pair[1].0),
            "each starts where the last one ended"
        );
        for (dpi, width, height) in state.sizes.all() {
            let scale = ScaleFactor::from_dpi(dpi).unwrap();
            let size = INNER.to_physical(scale);
            assert_eq!((width, height), (size.width, size.height), "at {dpi} dpi");
        }
        assert!(state.frames.get() >= 1);
    });
}
