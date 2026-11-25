//! A real quiet window, on a desktop that receives no input: through every command, and against whoever asks the
//! operating system for something else.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_window::{
    Attribution, CommandSource, Control, Display, Event, LogicalSize, LoopConfig,
    NativeWindowToken, PhysicalPosition, Placement, Refusal, WakeLane, WindowClass, WindowCommand,
    WindowEvent, WindowKey, WindowSpec, WindowState,
};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, IsWindow, IsWindowVisible, PostMessageW, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, GWL_EXSTYLE, HWND_TOP, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, WM_CLOSE, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use support::{on_private_desktop, run_script, Decoy, Log};

const INNER: LogicalSize = LogicalSize::new(400.0, 300.0);
const MODEL: Attribution = Attribution {
    actor: 1,
    session: 2,
    correlation: 3,
};

struct State {
    decoy: RefCell<Option<Decoy>>,
    lane: RefCell<Option<WakeLane>>,
    window: Cell<Option<WindowKey>>,
    token: RefCell<Option<NativeWindowToken>>,
    hwnd: Cell<HWND>,
    step: Cell<u32>,
    displays: Log<Display>,
    sizes: Log<(u32, u32)>,
    events: Log<&'static str>,
}

impl State {
    fn window(&self) -> WindowKey {
        self.window.get().unwrap()
    }

    /// The next step runs once the events of this one were handed over.
    fn next(&self) {
        self.step.set(self.step.get() + 1);
        let lane = self.lane.borrow();
        lane.as_ref()
            .unwrap()
            .request(0, Duration::from_millis(40))
            .unwrap();
    }

    fn assert_quiet(&self) {
        self.assert_not_active();
        let decoy = self.decoy.borrow();
        assert!(
            decoy.as_ref().unwrap().is_above(self.hwnd.get()),
            "step {}: above the other window",
            self.step.get()
        );
        // SAFETY: plain values, read only.
        let style = unsafe { GetWindowLongPtrW(self.hwnd.get(), GWL_EXSTYLE) } as u32;
        assert_eq!(style & WS_EX_APPWINDOW, 0);
        assert_eq!(style & WS_EX_NOACTIVATE, WS_EX_NOACTIVATE);
        assert_eq!(style & WS_EX_TOOLWINDOW, WS_EX_TOOLWINDOW);
    }

    fn run(&self, control: &Control, command: WindowCommand) -> Result<(), Refusal> {
        control.command(&CommandSource::host(9), self.window(), command)
    }

    fn state(&self, control: &Control) -> WindowState {
        control.window_state(self.window()).unwrap()
    }

    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                *self.decoy.borrow_mut() = Some(Decoy::create());
                *self.lane.borrow_mut() = Some(control.open_wake_lane().unwrap());
                let spec = WindowSpec {
                    title: "test".to_owned(),
                    inner: INNER,
                    min_inner: None,
                    placement: Placement::At(PhysicalPosition::new(120, 140)),
                    class: WindowClass::Quiet,
                };
                let key = control
                    .create_window(&CommandSource::model(MODEL, 77), spec)
                    .unwrap();
                assert!(control.window_state(key).is_none(), "not there yet");
                assert_eq!(
                    control.command(&CommandSource::host(1), key, WindowCommand::Hide),
                    Err(Refusal::UnknownWindow)
                );
                self.window.set(Some(key));
            }
            Event::Window { window, event } => {
                assert_eq!(window, self.window());
                self.window_event(control, event);
            }
            Event::ShutdownBegan => control.finish(0).unwrap(),
            Event::LoopEnding => {
                self.decoy.borrow_mut().take();
            }
            _ => {}
        }
    }

    fn window_event(&self, control: &Control, event: WindowEvent) {
        match event {
            WindowEvent::Created { command, state } => {
                assert_eq!(command, 77);
                assert_eq!(state.display, Display::Hidden);
                assert_eq!(state.class, WindowClass::Quiet);
                assert!(!state.focused);
                assert_eq!(state.inner, INNER.to_physical(state.scale));
                assert_eq!(state.outer.origin, PhysicalPosition::new(120, 140));
                assert_eq!(state.native_tokens, 0);
                let token = control.native_window(self.window()).unwrap();
                self.hwnd.set(token.win32().hwnd.get());
                *self.token.borrow_mut() = Some(token);
                assert_eq!(self.state(control).native_tokens, 1);
                // SAFETY: plain values, read only.
                assert_eq!(unsafe { IsWindowVisible(self.hwnd.get()) }, 0);
                self.events.add("created");
                self.next();
            }
            WindowEvent::DisplayChanged { display } => self.displays.add(display),
            WindowEvent::Resized { inner, .. } => self.sizes.add((inner.width, inner.height)),
            WindowEvent::CloseRequested => {
                self.events.add("close");
                self.next();
            }
            WindowEvent::Destroyed => {
                self.events.add("destroyed");
                // SAFETY: a plain value, read only.
                assert_eq!(unsafe { IsWindow(self.hwnd.get()) }, 0);
                assert!(control.windows().is_empty());
                control.begin_shutdown().unwrap();
            }
            WindowEvent::FocusChanged { focused } => {
                assert!(!focused, "a quiet window got the keyboard")
            }
            WindowEvent::QuietBroken => panic!("a quiet window was activated"),
            WindowEvent::FrameDue { ticket, .. } => {
                let outcome = handshake_shell_window::FrameOutcome::Unchanged;
                control.frame_done(self.window(), ticket, outcome).unwrap();
            }
            _ => {}
        }
    }

    fn step(&self, control: &Control) {
        match self.step.get() {
            1 => {
                // Nobody activates it, not even the Operator who launched the process.
                let launch = control.take_launch_source(5).unwrap();
                let activate = WindowCommand::Show { activate: true };
                assert_eq!(
                    control.command(&launch, self.window(), activate.clone()),
                    Err(Refusal::QuietWindow)
                );
                assert_eq!(self.run(control, activate), Err(Refusal::QuietWindow));
                self.assert_hidden();
                self.run(control, WindowCommand::Show { activate: false })
                    .unwrap();
                assert_eq!(self.state(control).display, Display::Normal);
                self.assert_quiet();
            }
            2 => {
                // Something else in the process asks the operating system directly.
                // SAFETY: plain values, for a window of this thread on the private desktop.
                unsafe {
                    SetWindowLongPtrW(self.hwnd.get(), GWL_EXSTYLE, WS_EX_APPWINDOW as isize);
                    SetWindowPos(
                        self.hwnd.get(),
                        HWND_TOP,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE,
                    );
                    ShowWindow(self.hwnd.get(), SW_SHOW);
                }
                self.assert_quiet();
            }
            3 => {
                let wider = LogicalSize::new(500.0, 350.0);
                self.run(control, WindowCommand::SetInnerSize(wider))
                    .unwrap();
                let state = self.state(control);
                assert_eq!(state.inner, wider.to_physical(state.scale));
                self.run(
                    control,
                    WindowCommand::MoveTo(PhysicalPosition::new(70, 80)),
                )
                .unwrap();
                assert_eq!(
                    self.state(control).outer.origin,
                    PhysicalPosition::new(70, 80)
                );
                let nothing = LogicalSize::new(0.0, 10.0);
                assert_eq!(
                    self.run(control, WindowCommand::SetInnerSize(nothing)),
                    Err(Refusal::InvalidGeometry)
                );
                self.assert_quiet();
            }
            4 => {
                self.run(control, WindowCommand::Minimize).unwrap();
                let state = self.state(control);
                assert_eq!(state.display, Display::Minimized);
                assert!(state.inner.is_empty());
                assert_eq!(
                    self.run(control, WindowCommand::SetInnerSize(INNER)),
                    Err(Refusal::NotAtNormalSize)
                );
                assert_eq!(
                    self.run(control, WindowCommand::MoveTo(PhysicalPosition::new(1, 1))),
                    Err(Refusal::NotAtNormalSize)
                );
                self.assert_not_active();
            }
            5 => {
                self.run(control, WindowCommand::Restore { activate: false })
                    .unwrap();
                let state = self.state(control);
                assert_eq!(state.display, Display::Normal);
                assert_eq!(
                    state.inner,
                    LogicalSize::new(500.0, 350.0).to_physical(state.scale)
                );
                assert_eq!(state.outer.origin, PhysicalPosition::new(70, 80));
                self.assert_quiet();
            }
            6 => {
                self.run(control, WindowCommand::Hide).unwrap();
                assert_eq!(self.state(control).display, Display::Hidden);
                self.run(control, WindowCommand::Show { activate: false })
                    .unwrap();
                self.assert_quiet();
            }
            7 => {
                // SAFETY: plain values, to a window of this thread.
                assert_ne!(unsafe { PostMessageW(self.hwnd.get(), WM_CLOSE, 0, 0) }, 0);
                return;
            }
            8 => {
                // Asking does not destroy it, and a surface that may exist keeps it.
                // SAFETY: a plain value, read only.
                assert_ne!(unsafe { IsWindow(self.hwnd.get()) }, 0);
                assert_eq!(
                    self.run(control, WindowCommand::Destroy),
                    Err(Refusal::NativeTokensHeld)
                );
                let token = self.token.borrow_mut().take().unwrap();
                let second = token.clone();
                assert_eq!(second.tokens(), 2);
                drop(token);
                assert_eq!(
                    self.run(control, WindowCommand::Destroy),
                    Err(Refusal::NativeTokensHeld)
                );
                drop(second);
                self.run(control, WindowCommand::Destroy).unwrap();
                assert_eq!(
                    self.run(control, WindowCommand::Destroy),
                    Err(Refusal::UnknownWindow),
                    "gone at once"
                );
                return;
            }
            other => panic!("step {other}"),
        }
        self.next();
    }

    /// The active window of this thread, on its own desktop.
    fn assert_not_active(&self) {
        // SAFETY: no arguments.
        let active = unsafe { GetActiveWindow() };
        assert_ne!(active, self.hwnd.get(), "step {}", self.step.get());
    }

    fn assert_hidden(&self) {
        // SAFETY: plain values, read only.
        assert_eq!(unsafe { IsWindowVisible(self.hwnd.get()) }, 0);
    }
}

#[test]
fn a_quiet_window_is_never_active_and_never_above_another_window() {
    on_private_desktop(Duration::from_secs(60), || {
        let state = Rc::new(State {
            decoy: RefCell::default(),
            lane: RefCell::default(),
            window: Cell::new(None),
            token: RefCell::default(),
            hwnd: Cell::new(0),
            step: Cell::new(0),
            displays: Log::new(),
            sizes: Log::new(),
            events: Log::new(),
        });
        let (events, steps) = (Rc::clone(&state), Rc::clone(&state));
        let config = LoopConfig {
            launched_by_operator: true,
            ..LoopConfig::default()
        };
        let report = run_script(
            config,
            move |control, event| events.event(control, event),
            move |control, _| steps.step(control),
        )
        .unwrap();

        assert_eq!(state.events.all(), ["created", "close", "destroyed"]);
        let displays = state.displays.all();
        let commanded = [
            Display::Normal,
            Display::Minimized,
            Display::Normal,
            Display::Hidden,
            Display::Normal,
        ];
        assert_eq!(displays.get(..5), Some(&commanded[..]), "{displays:?}");
        // Destroying a shown window hides it first.
        assert!(displays
            .iter()
            .skip(5)
            .all(|display| *display == Display::Hidden));
        assert!(
            !state.sizes.all().contains(&(0, 0)),
            "minimising is not a size"
        );
        assert_eq!((report.windows_created, report.windows_destroyed), (1, 1));
        assert_eq!(report.quiet_broken, 0);
        // SAFETY: a plain value, read only.
        assert_eq!(unsafe { IsWindow(state.hwnd.get()) }, 0);
    });
}
