//! Real windows of the Operator's class on a desktop that receives no input: who may activate them, move, size or
//! restore the one the Operator works in, and replace the clipboard; where a window shown without activation goes;
//! what a minimise hands on, and where the keyboard goes when the window the Operator works in is closed (MT-214 W1,
//! W4, R2-N1, R2-N5). It is also the control for the quiet window's test: the probe used there does see an activation.
//!
//! The desktop has no foreground window, so the active window of the loop's thread stands for it: the window the
//! launch activated, and later a dialog that window owns.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_window::{
    Attribution, ClipboardError, CommandSource, Control, Display, Event, InputStamp, LogicalSize,
    LoopConfig, PhysicalPosition, Placement, Refusal, TextClipboard, WakeLane, WindowClass,
    WindowCommand, WindowEvent, WindowKey, WindowSpec,
};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetWindow, PostMessageW, SetWindowPos, ShowWindow,
    GW_HWNDNEXT, HWND_TOP, SHOW_WINDOW_CMD, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, WM_CLOSE, WS_POPUP,
};

use support::{on_private_desktop, run_script, wide, Decoy};

const MODEL: Attribution = Attribution {
    actor: 1,
    session: 2,
    correlation: 3,
};

/// A clipboard of the test's own. The Operator's is never touched.
#[derive(Clone, Default)]
struct Pad(Rc<RefCell<Option<String>>>);

impl TextClipboard for Pad {
    fn read(&mut self) -> Result<Option<String>, ClipboardError> {
        Ok(self.0.borrow().clone())
    }

    fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.0.borrow_mut() = Some(text.to_owned());
        Ok(())
    }

    fn withdraw(&mut self) -> Result<bool, ClipboardError> {
        Ok(self.0.borrow_mut().take().is_some())
    }
}

#[derive(Default)]
struct State {
    decoy: RefCell<Option<Decoy>>,
    lane: RefCell<Option<WakeLane>>,
    /// The first window and the second.
    keys: RefCell<Vec<WindowKey>>,
    hwnds: RefCell<Vec<HWND>>,
    dialog: Cell<HWND>,
    step: Cell<u32>,
    closes: Cell<u32>,
}

fn spec(at: i32) -> WindowSpec {
    WindowSpec {
        title: "test".to_owned(),
        inner: LogicalSize::new(300.0, 200.0),
        min_inner: None,
        placement: Placement::At(PhysicalPosition::new(at, at)),
        class: WindowClass::Operator,
    }
}

impl State {
    fn key(&self, index: usize) -> WindowKey {
        self.keys.borrow()[index]
    }

    fn hwnd(&self, index: usize) -> HWND {
        self.hwnds.borrow()[index]
    }

    fn next(&self) {
        self.step.set(self.step.get() + 1);
        let lane = self.lane.borrow();
        lane.as_ref()
            .unwrap()
            .request(0, Duration::from_millis(30))
            .unwrap();
    }

    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                *self.decoy.borrow_mut() = Some(Decoy::create());
                *self.lane.borrow_mut() = Some(control.open_wake_lane().unwrap());
                for at in [40, 200] {
                    let key = control
                        .create_window(&CommandSource::host(1), spec(at))
                        .unwrap();
                    self.keys.borrow_mut().push(key);
                }
            }
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => {
                let token = control.native_window(window).unwrap();
                let at = self.keys.borrow().iter().position(|key| *key == window);
                assert_eq!(at, Some(self.hwnds.borrow().len()), "created in order");
                self.hwnds.borrow_mut().push(token.win32().hwnd.get());
                if self.hwnds.borrow().len() == 2 {
                    self.next();
                }
            }
            Event::Window {
                window,
                event: WindowEvent::CloseRequested,
            } => {
                assert_eq!(window, self.key(0));
                self.closes.set(self.closes.get() + 1);
                if self.closes.get() == 1 {
                    self.closed(control);
                } else {
                    self.closed_again(control);
                }
            }
            Event::Window {
                window,
                event: WindowEvent::FrameDue { ticket, .. },
            } => {
                let outcome = handshake_shell_window::FrameOutcome::Unchanged;
                control.frame_done(window, ticket, outcome).unwrap();
            }
            Event::ShutdownBegan => control.finish(0).unwrap(),
            Event::LoopEnding => {
                self.decoy.borrow_mut().take();
            }
            _ => {}
        }
    }

    fn step(&self, control: &Control) {
        match self.step.get() {
            1 => self.who_activates(control),
            2 => self.what_a_model_may_not_do(control),
            3 => self.owners_and_the_place_behind(control),
            4 => {
                // The close request (`closed`) arrives first; then a minimise of the Operator's window hands the
                // activation to no other window of ours.
                assert_eq!(self.closes.get(), 1);
                let (first, second) = (self.key(0), self.key(1));
                control
                    .command(&CommandSource::host(30), first, WindowCommand::Minimize)
                    .unwrap();
                assert_eq!(control.window_state(first).unwrap().display, Display::Minimized);
                assert_ne!(active_window(), self.hwnd(1), "not handed to the other window");
                assert!(!control.window_state(second).unwrap().focused);
            }
            5 => self.what_a_model_may_not_restore(control),
            6 => {
                // The first window is shown again by the shell, and the Operator clicks into it (a direct call
                // activates it, as in step 3). Then it is asked to close: `closed_again`.
                let host = CommandSource::host(50);
                let show = WindowCommand::Show { activate: false };
                control.command(&host, self.key(0), show).unwrap();
                // SAFETY: plain values, for a window of this thread on the private desktop.
                unsafe {
                    SetWindowPos(self.hwnd(0), HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW)
                };
                assert_eq!(active_window(), self.hwnd(0));
                // SAFETY: plain values, to a window of this thread.
                assert_ne!(unsafe { PostMessageW(self.hwnd(0), WM_CLOSE, 0, 0) }, 0);
                return;
            }
            other => panic!("step {other}"),
        }
        self.next();
    }

    fn who_activates(&self, control: &Control) {
        let (first, second) = (self.key(0), self.key(1));
        let host = CommandSource::host(2);
        let model = CommandSource::model(MODEL, 3);
        let model_input = InputStamp::model_action(MODEL, control.now());
        let answer = CommandSource::from_input(&model_input, 4);
        let activate = || WindowCommand::Show { activate: true };
        for source in [&host, &model, &answer] {
            assert_eq!(
                control.command(source, first, activate()),
                Err(Refusal::NeedsOperator)
            );
            let restore = WindowCommand::Restore { activate: true };
            assert_eq!(
                control.command(source, first, restore),
                Err(Refusal::NeedsOperator)
            );
            assert_eq!(
                control.write_clipboard(source, "x"),
                Err(ClipboardError::NeedsOperator)
            );
        }
        assert_eq!(control.read_clipboard(), Ok(None));

        // Shown without activation: not active, and not above the other window.
        control
            .command(&host, first, WindowCommand::Show { activate: false })
            .unwrap();
        assert_ne!(active_window(), self.hwnd(0));
        assert!(self.decoy.borrow().as_ref().unwrap().is_above(self.hwnd(0)));

        // The Operator's launch activates one window, once, and does not replace the clipboard.
        let launch = control.take_launch_source(6).unwrap();
        assert!(control.take_launch_source(7).is_none());
        assert_eq!(
            control.write_clipboard(&launch, "from the launch"),
            Err(ClipboardError::NeedsOperator)
        );
        control.command(&launch, first, activate()).unwrap();
        assert_eq!(
            active_window(),
            self.hwnd(0),
            "the Operator's show activates"
        );
        assert!(!self.decoy.borrow().as_ref().unwrap().is_above(self.hwnd(0)));
        assert_eq!(
            control.command(&launch, first, activate()),
            Err(Refusal::NeedsOperator),
            "used up"
        );
        assert_eq!(
            control.command(&launch, second, activate()),
            Err(Refusal::NeedsOperator),
            "and not for another window"
        );
        assert_eq!(control.withdraw_clipboard(), Ok(false));
    }

    /// The first window is active, so it is the one the Operator works in.
    fn what_a_model_may_not_do(&self, control: &Control) {
        let first = self.key(0);
        assert_eq!(active_window(), self.hwnd(0));
        let model = || CommandSource::model(MODEL, 10);
        let refused = [
            (WindowCommand::Hide, Refusal::WouldMoveFocus),
            (WindowCommand::Minimize, Refusal::WouldMoveFocus),
            (WindowCommand::Destroy, Refusal::WouldMoveFocus),
            (
                WindowCommand::MoveTo(PhysicalPosition::new(5, 5)),
                Refusal::OperatorsWindow,
            ),
            (
                WindowCommand::SetInnerSize(LogicalSize::new(100.0, 100.0)),
                Refusal::OperatorsWindow,
            ),
        ];
        let before = control.window_state(first).unwrap();
        for (command, refusal) in refused {
            assert_eq!(control.command(&model(), first, command.clone()), Err(refusal), "{command:?}");
        }
        let after = control.window_state(first).unwrap();
        assert_eq!((after.outer, after.display), (before.outer, Display::Normal));
        control
            .command(&model(), first, WindowCommand::SetTitle("renamed".to_owned()))
            .unwrap();
        // The shell itself may move it; that activates nothing.
        let to = PhysicalPosition::new(60, 60);
        control
            .command(&CommandSource::host(11), first, WindowCommand::MoveTo(to))
            .unwrap();
        assert_eq!(control.window_state(first).unwrap().outer.origin, to);
        assert_eq!(active_window(), self.hwnd(0));
    }

    /// A dialog the first window owns is active: the Operator works in the first window still. The second window,
    /// shown without activation, goes right below the first, below the whole owner chain.
    fn owners_and_the_place_behind(&self, control: &Control) {
        let (first, second) = (self.key(0), self.key(1));
        let class = wide("STATIC");
        // SAFETY: a class of the system's, owned by a window of this thread; every pointer outlives the call.
        let dialog = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                ptr::null(),
                WS_POPUP,
                300,
                40,
                80,
                60,
                self.hwnd(0),
                0,
                0,
                ptr::null(),
            )
        };
        assert_ne!(dialog, 0);
        self.dialog.set(dialog);
        // SAFETY: plain values, for a window of this thread; activates it, as the Operator's click would.
        unsafe {
            SetWindowPos(dialog, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW)
        };
        assert_eq!(active_window(), dialog);
        let model = CommandSource::model(MODEL, 20);
        assert_eq!(
            control.command(&model, first, WindowCommand::MoveTo(PhysicalPosition::new(1, 1))),
            Err(Refusal::OperatorsWindow),
            "the owner of the active dialog"
        );
        control
            .command(&CommandSource::host(21), second, WindowCommand::Show { activate: false })
            .unwrap();
        // SAFETY: plain values, read only.
        let below_first = unsafe { GetWindow(self.hwnd(0), GW_HWNDNEXT) };
        assert_eq!(below_first, self.hwnd(1), "right below the owner, not between it and its dialog");
        assert_eq!(active_window(), dialog, "shown without activation");
        // SAFETY: plain values, to a window of this thread.
        assert_ne!(unsafe { PostMessageW(self.hwnd(0), WM_CLOSE, 0, 0) }, 0);
    }

    /// Somebody asked to close the first window: a model may remove it, once.
    fn closed(&self, control: &Control) {
        let first = self.key(0);
        assert_eq!(active_window(), self.dialog.get());
        let model = || CommandSource::model(MODEL, 25);
        control.command(&model(), first, WindowCommand::Hide).unwrap();
        assert_eq!(control.window_state(first).unwrap().display, Display::Hidden);
        assert_eq!(
            control.command(&model(), first, WindowCommand::Destroy),
            Err(Refusal::WouldMoveFocus),
            "the request was used"
        );
        control
            .command(&CommandSource::host(26), first, WindowCommand::Show { activate: false })
            .unwrap();
    }

    /// MT-214 R2-N1: the Operator maximises the second window, then minimises it, and works in it all along (the
    /// direct calls activate it). A model may neither restore it nor show it back from minimised; the shell may.
    fn what_a_model_may_not_restore(&self, control: &Control) {
        let second = self.key(1);
        let display = || control.window_state(second).unwrap().display;
        // SAFETY: the test's own window, of this thread.
        assert_ne!(unsafe { DestroyWindow(self.dialog.get()) }, 0);
        show_directly(self.hwnd(1), SW_SHOWMAXIMIZED);
        assert_eq!(active_window(), self.hwnd(1));
        let model = || CommandSource::model(MODEL, 40);
        let restore = WindowCommand::Restore { activate: false };
        let show = WindowCommand::Show { activate: false };
        assert_eq!(
            control.command(&model(), second, restore.clone()),
            Err(Refusal::OperatorsWindow),
            "not un-maximised"
        );
        assert_eq!(display(), Display::Maximized);
        control.command(&model(), second, show.clone()).unwrap();
        assert_eq!(display(), Display::Maximized, "a show that keeps the size");
        show_directly(self.hwnd(1), SW_SHOWMINIMIZED);
        assert_eq!(active_window(), self.hwnd(1));
        for command in [show, restore.clone()] {
            assert_eq!(
                control.command(&model(), second, command.clone()),
                Err(Refusal::OperatorsWindow),
                "{command:?}"
            );
        }
        assert_eq!(display(), Display::Minimized);
        control
            .command(&CommandSource::host(41), second, restore)
            .unwrap();
        assert_ne!(display(), Display::Minimized);
    }

    /// MT-214 R2-N5: asked to close, the window the Operator works in is destroyed by a model on that request, and the
    /// keyboard goes to the other Operator window (ACTIVATION_AFTER_REMOVAL), not nowhere.
    fn closed_again(&self, control: &Control) {
        assert_eq!(active_window(), self.hwnd(0));
        control
            .command(&CommandSource::model(MODEL, 51), self.key(0), WindowCommand::Destroy)
            .unwrap();
        assert_eq!(active_window(), self.hwnd(1));
        control.begin_shutdown().unwrap();
    }
}

fn show_directly(hwnd: HWND, command: SHOW_WINDOW_CMD) {
    // SAFETY: plain values, for a window of this thread on the private desktop.
    unsafe { ShowWindow(hwnd, command) };
}

/// The active window of this thread, on its own desktop.
fn active_window() -> HWND {
    // SAFETY: no arguments.
    unsafe { GetActiveWindow() }
}

#[test]
fn only_the_operators_authority_activates_a_window_or_replaces_the_clipboard() {
    on_private_desktop(Duration::from_secs(60), || {
        let pad = Pad::default();
        let state = Rc::new(State::default());
        let (events, wakes) = (Rc::clone(&state), Rc::clone(&state));
        let config = LoopConfig {
            launched_by_operator: true,
            clipboard: Some(Box::new(pad.clone())),
        };
        let report = run_script(
            config,
            move |control, event| events.event(control, event),
            move |control, _| wakes.step(control),
        )
        .unwrap();
        assert_eq!(state.step.get(), 6, "every step ran");
        assert_eq!(state.closes.get(), 2);
        assert_eq!((report.windows_created, report.windows_destroyed), (2, 2));
        // 6 without the Operator, 2 after the launch was used, 5 of a model, 1 through the owner, 1 after the close,
        // 3 restores of a model.
        assert_eq!(report.refusals, 18, "each refused command is counted");
        assert_eq!(report.quiet_broken, 0);
        assert_eq!(*pad.0.borrow(), None);
    });
}
