//! A real window of the Operator's class on a desktop that receives no input: who may activate it and who may replace
//! the clipboard. It is also the control for the quiet window's test: the probe used there does see an activation.
#![cfg(windows)]
#![allow(unsafe_code)]

mod support;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_window::{
    Attribution, ClipboardError, CommandSource, Control, Event, InputStamp, LogicalSize,
    LoopConfig, PhysicalPosition, Placement, Refusal, TextClipboard, WakeLane, WindowClass,
    WindowCommand, WindowEvent, WindowKey, WindowSpec,
};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;

use support::{on_private_desktop, run_script, Decoy};

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

struct State {
    decoy: RefCell<Option<Decoy>>,
    lane: RefCell<Option<WakeLane>>,
    window: Cell<Option<WindowKey>>,
    hwnd: Cell<HWND>,
}

impl State {
    fn event(&self, control: &Control, event: Event) {
        match event {
            Event::LoopStarted => {
                *self.decoy.borrow_mut() = Some(Decoy::create());
                *self.lane.borrow_mut() = Some(control.open_wake_lane().unwrap());
                let spec = WindowSpec {
                    title: "test".to_owned(),
                    inner: LogicalSize::new(300.0, 200.0),
                    min_inner: None,
                    placement: Placement::At(PhysicalPosition::new(40, 40)),
                    class: WindowClass::Operator,
                };
                let key = control
                    .create_window(&CommandSource::host(1), spec)
                    .unwrap();
                self.window.set(Some(key));
            }
            Event::Window {
                event: WindowEvent::Created { .. },
                ..
            } => {
                let token = control.native_window(self.window.get().unwrap()).unwrap();
                self.hwnd.set(token.win32().hwnd.get());
                self.created(control);
                let lane = self.lane.borrow();
                lane.as_ref()
                    .unwrap()
                    .request(1, Duration::from_millis(50))
                    .unwrap();
            }
            Event::Window {
                event: WindowEvent::FrameDue { ticket, .. },
                ..
            } => {
                let outcome = handshake_shell_window::FrameOutcome::Unchanged;
                control
                    .frame_done(self.window.get().unwrap(), ticket, outcome)
                    .unwrap();
            }
            Event::ShutdownBegan => control.finish(0).unwrap(),
            Event::LoopEnding => {
                self.decoy.borrow_mut().take();
            }
            _ => {}
        }
    }

    fn created(&self, control: &Control) {
        let window = self.window.get().unwrap();
        let host = CommandSource::host(2);
        let model = CommandSource::model(MODEL, 3);
        let model_input = InputStamp::model_action(MODEL, control.now());
        let answer = CommandSource::from_input(&model_input, 4);
        let activate = || WindowCommand::Show { activate: true };
        for source in [&host, &model, &answer] {
            assert_eq!(
                control.command(source, window, activate()),
                Err(Refusal::NeedsOperator)
            );
            let restore = WindowCommand::Restore { activate: true };
            assert_eq!(
                control.command(source, window, restore),
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
            .command(&host, window, WindowCommand::Show { activate: false })
            .unwrap();
        assert_ne!(active_window(), self.hwnd.get());
        assert!(self
            .decoy
            .borrow()
            .as_ref()
            .unwrap()
            .is_above(self.hwnd.get()));

        // The Operator's launch may, once.
        let launch = control.take_launch_source(6).unwrap();
        assert!(control.take_launch_source(7).is_none());
        control
            .write_clipboard(&launch, "from the operator")
            .unwrap();
        assert_eq!(
            control.read_clipboard(),
            Ok(Some("from the operator".to_owned()))
        );
        assert_eq!(control.withdraw_clipboard(), Ok(true));
        assert_eq!(control.withdraw_clipboard(), Ok(false));
        control.command(&launch, window, activate()).unwrap();
        assert_eq!(
            active_window(),
            self.hwnd.get(),
            "the Operator's show activates"
        );
        assert!(!self
            .decoy
            .borrow()
            .as_ref()
            .unwrap()
            .is_above(self.hwnd.get()));
    }
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
        let state = Rc::new(State {
            decoy: RefCell::default(),
            lane: RefCell::default(),
            window: Cell::new(None),
            hwnd: Cell::new(0),
        });
        let events = Rc::clone(&state);
        let config = LoopConfig {
            launched_by_operator: true,
            clipboard: Some(Box::new(pad.clone())),
        };
        let report = run_script(
            config,
            move |control, event| events.event(control, event),
            move |control, _| control.begin_shutdown().unwrap(),
        )
        .unwrap();
        assert_eq!((report.windows_created, report.windows_destroyed), (1, 1));
        assert_eq!(report.refusals, 6, "each refused command is counted");
        assert_eq!(*pad.0.borrow(), None);
    });
}
