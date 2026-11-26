//! MT-214 S3 on a real event loop: the messages the operating system sends a window, posted to a quiet window of the
//! test's own on a private desktop, through winit and the window crate into the tracker.
//!
//! Keys: a key down the system translates, its repeat, and the focus going: one press, one repeat, one release. The
//! input method, allowed for a text target: a composition's start and its emptied preedit (`WM_IME_COMPOSITION` with no
//! flags), the focus going mid-composition (the window completes it first and says so, then one end, and none again
//! when the input method's own end follows); a composition the host moves the text focus away from, which the window
//! is asked to complete before it ends, going on at the new target as one of its own; and nothing composed at all once
//! a target that takes no composed text has the focus (`AllowIme(false)` carried out). A real input method's text, and
//! what it does when asked to complete, need the Operator's keyboard and desktop (HBR-QUIET-004). One loop per
//! process, so this is a target of its own.
//!
//! The window library takes a window as focused only when it is active as well (winit 0.30.13 `window_state.rs`
//! `has_active_focus`: `WM_NCACTIVATE` and `WM_SETFOCUS`), so every focus the test posts follows a `WM_NCACTIVATE` of
//! an active window (MT-214 E6-F1). Posting the message activates nothing: the window stays inactive and not
//! foreground, which `must_stay_inactive` holds.
#![cfg(windows)]
#![allow(unsafe_code)]

#[path = "../../window/tests/support/mod.rs"]
mod support;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use handshake_shell_input::{
    CaretPlace, Composition, FocusBy, Input, InputEvent, InputTracker, KeyPhase, LogicalRect,
    Synthetic, TextFocus, TextTarget,
};
use handshake_shell_window::{
    CommandSource, Control, Event, ImeChange, KeyCode, LogicalSize, LoopConfig, Origin,
    PhysicalPosition, Placement, WakeLane, WindowClass, WindowCommand, WindowEvent, WindowKey,
    WindowSpec,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PostMessageW, WM_IME_COMPOSITION, WM_IME_ENDCOMPOSITION, WM_IME_STARTCOMPOSITION, WM_KEYDOWN,
    WM_KILLFOCUS, WM_NCACTIVATE, WM_SETFOCUS,
};

use support::{must_stay_inactive, on_private_desktop, run_script, stop_watching, Log};

/// The space bar: its virtual key, and the first parameter with its scan code, pressed, and pressed again while held.
const SPACE: WPARAM = 0x20;
const SPACE_DOWN: LPARAM = 0x0039_0001;
const SPACE_REPEAT: LPARAM = 0x4039_0001;

/// The window is active (`WM_NCACTIVATE`, wparam true), which the window library needs before it reports a focus.
const ACTIVE: (u32, WPARAM, LPARAM) = (WM_NCACTIVATE, 1, 0);

const A: TextTarget = TextTarget(1);
const B: TextTarget = TextTarget(2);
const C: TextTarget = TextTarget(3);

fn text_focus(target: TextTarget, accepts_ime: bool) -> Option<TextFocus> {
    Some(TextFocus {
        target,
        accepts_ime,
        caret: CaretPlace {
            viewport: LogicalRect {
                x: 0.0,
                y: 0.0,
                width: 150.0,
                height: 80.0,
            },
            caret: LogicalRect {
                x: 12.0,
                y: 8.0,
                width: 1.0,
                height: 14.0,
            },
            scroll_x: 0.0,
            scroll_y: 0.0,
        },
    })
}

/// Post messages to the test's own window, as the operating system would put them into its queue.
fn post(hwnd: HWND, messages: &[(u32, WPARAM, LPARAM)]) {
    for (message, wparam, lparam) in messages {
        // SAFETY: plain values, to a window of this thread.
        assert_ne!(unsafe { PostMessageW(hwnd, *message, *wparam, *lparam) }, 0);
    }
}

/// An input method event as a line: target, composition, what, and why when synthesized.
fn composition(event: &InputEvent) -> Option<String> {
    let Input::Ime(ime) = event.input() else {
        return None;
    };
    let what = match &ime.change {
        Composition::Enabled => "enabled".to_owned(),
        Composition::Preedit { text, cursor } => format!("preedit {:?} {cursor:?}", text.as_str()),
        Composition::Commit(text) => format!("commit {:?}", text.as_str()),
        Composition::Cancel => "cancel".to_owned(),
        Composition::Disabled => "disabled".to_owned(),
        other => format!("{other:?}"),
    };
    let why = event.synthetic().map(Synthetic::code).unwrap_or_default();
    Some(
        format!(
            "{} #{} {what} {why}",
            ime.target.0,
            ime.composition.number()
        )
        .trim_end()
        .to_owned(),
    )
}

#[derive(Default)]
struct State {
    lane: RefCell<Option<WakeLane>>,
    window: Cell<Option<WindowKey>>,
    hwnd: Cell<HWND>,
    step: Cell<u32>,
    commands: Cell<u64>,
    tracker: RefCell<InputTracker>,
    inputs: RefCell<Vec<InputEvent>>,
    /// The window crate's own input method events, as it reported them.
    window_ime: Log<String>,
    /// The focus the window reported, as it changed.
    window_focus: Log<bool>,
}

impl State {
    fn next(&self) {
        self.step.set(self.step.get() + 1);
        let lane = self.lane.borrow();
        lane.as_ref()
            .unwrap()
            .request(0, Duration::from_millis(30))
            .unwrap();
    }

    fn key(&self) -> WindowKey {
        self.window.get().unwrap()
    }

    /// Carry out what the tracker asks the windows for; every request must be accepted.
    fn carry_out(&self, control: &Control) {
        let requests = self.tracker.borrow_mut().take_requests();
        for (window, request) in requests {
            self.commands.set(self.commands.get() + 1);
            let source = CommandSource::host(self.commands.get());
            control.command(&source, window, request.command()).unwrap();
        }
    }

    fn focus(&self, control: &Control, focus: Option<TextFocus>) {
        let ended = self
            .tracker
            .borrow_mut()
            .focus_text(self.key(), focus, FocusBy::Host, control.now())
            .unwrap();
        self.inputs.borrow_mut().extend(ended);
        self.carry_out(control);
    }

    /// The input method events so far, as lines, taken.
    fn take_compositions(&self) -> Vec<String> {
        let mut inputs = self.inputs.borrow_mut();
        let lines = inputs.iter().filter_map(composition).collect();
        inputs.retain(|input| !matches!(input.input(), Input::Ime(_)));
        lines
    }

    fn event(&self, control: &Control, event: Event) {
        let made = self.tracker.borrow_mut().feed(control.now(), &event);
        self.inputs.borrow_mut().extend(made);
        self.carry_out(control);
        match event {
            Event::LoopStarted => {
                *self.lane.borrow_mut() = Some(control.open_wake_lane().unwrap());
                let spec = WindowSpec {
                    title: "test".to_owned(),
                    inner: LogicalSize::new(200.0, 100.0),
                    min_inner: None,
                    placement: Placement::At(PhysicalPosition::new(30, 30)),
                    class: WindowClass::Quiet,
                };
                let key = control
                    .create_window(&CommandSource::host(1), spec)
                    .unwrap();
                self.window.set(Some(key));
            }
            Event::Window {
                window,
                event: WindowEvent::Created { .. },
            } => {
                let hwnd = control.native_window(window).unwrap().win32().hwnd.get();
                self.hwnd.set(hwnd);
                must_stay_inactive(hwnd);
                self.next();
            }
            Event::Window {
                event: WindowEvent::Ime(ime),
                ..
            } => self.window_ime.add(match ime.change {
                ImeChange::Started => "started".to_owned(),
                ImeChange::Preedit { .. } => "preedit".to_owned(),
                ImeChange::Commit(_) => "commit".to_owned(),
                ImeChange::Ended => "ended".to_owned(),
                ImeChange::Completed => "completed".to_owned(),
                _ => "other".to_owned(),
            }),
            Event::Window {
                event: WindowEvent::FocusChanged { focused },
                ..
            } => self.window_focus.add(focused),
            Event::Window {
                event: WindowEvent::Destroyed,
                ..
            } => {
                stop_watching(self.hwnd.get());
                control.begin_shutdown().unwrap();
            }
            Event::ShutdownBegan => control.finish(0).unwrap(),
            _ => {}
        }
    }

    /// The space bar's key inputs so far: phase, and whether the window library or this crate ended it.
    fn space(&self) -> Vec<(KeyPhase, bool)> {
        self.inputs
            .borrow()
            .iter()
            .filter_map(|input| match input.input() {
                Input::Key(key) if key.physical.code == KeyCode::Space => {
                    Some((key.phase, key.focus_sync || input.synthetic().is_some()))
                }
                _ => None,
            })
            .collect()
    }

    fn texts(&self) -> Vec<String> {
        self.inputs
            .borrow()
            .iter()
            .filter_map(|input| match input.input() {
                Input::Text(text) => Some(text.as_str().to_owned()),
                _ => None,
            })
            .collect()
    }

    fn step(&self, control: &Control) {
        let hwnd = self.hwnd.get();
        match self.step.get() {
            1 => post(
                hwnd,
                &[ACTIVE, (WM_SETFOCUS, 0, 0), (WM_KEYDOWN, SPACE, SPACE_DOWN)],
            ),
            // One key at a time, so the character the system translates it to follows it in the queue.
            2 => post(hwnd, &[(WM_KEYDOWN, SPACE, SPACE_REPEAT)]),
            3 => {
                // E6-F1: without the activation the window library reports no focus, and every step after this one
                // waits for a focus that never comes.
                assert_eq!(
                    self.window_focus.all(),
                    [true],
                    "the window reported its focus"
                );
                assert_eq!(
                    self.space(),
                    [(KeyPhase::Press, false), (KeyPhase::Repeat, false)]
                );
                assert_eq!(
                    self.texts(),
                    [" ", " "],
                    "the translated characters, one per key"
                );
                post(hwnd, &[(WM_KILLFOCUS, 0, 0)]);
            }
            4 => {
                // The window library may release the key itself as the focus goes; either way, once.
                let space = self.space();
                assert_eq!(space.len(), 3, "{space:?}");
                assert_eq!(space[2], (KeyPhase::Release, true), "{space:?}");
                // The key, its text and its release: one source, the test's posts, and never a model's.
                let origins: Vec<Origin> = self
                    .inputs
                    .borrow()
                    .iter()
                    .filter(|input| matches!(input.input(), Input::Key(_) | Input::Text(_)))
                    .map(InputEvent::origin)
                    .collect();
                assert!(
                    origins.iter().all(|origin| *origin == origins[0]),
                    "one source: {origins:?}"
                );
                assert!(!self.inputs.borrow().iter().any(InputEvent::is_model));
                self.inputs.borrow_mut().clear();
                // The input method, for target A.
                self.focus(control, text_focus(A, true));
                post(
                    hwnd,
                    &[
                        ACTIVE,
                        (WM_SETFOCUS, 0, 0),
                        (WM_IME_STARTCOMPOSITION, 0, 0),
                        (WM_IME_COMPOSITION, 0, 0),
                        (WM_KILLFOCUS, 0, 0),
                        (WM_IME_ENDCOMPOSITION, 0, 0),
                    ],
                );
            }
            5 => {
                // The window asked the input method to complete the composition before it reported the loss of its
                // focus (S3-Q1): its report comes before the end, and a quiet input method has no result to give.
                let window = self.window_ime.all();
                let at = |name: &str| window.iter().position(|seen| seen == name);
                assert_eq!(
                    window.iter().filter(|seen| *seen != "completed").cloned().collect::<Vec<_>>(),
                    ["started", "preedit", "ended"],
                    "the window's own events: {window:?}"
                );
                assert!(
                    at("completed") > at("preedit") && at("completed") < at("ended"),
                    "completed before the loss of the keyboard: {window:?}"
                );
                assert_eq!(self.window_focus.all(), [true, false, true, false]);
                assert_eq!(
                    self.take_compositions(),
                    [
                        "1 #1 enabled",
                        "1 #1 preedit \"\" None",
                        "1 #1 disabled input.synthetic.focus_lost"
                    ],
                    "one end at the focus loss, none at the input method's own end"
                );
                post(
                    hwnd,
                    &[ACTIVE, (WM_SETFOCUS, 0, 0), (WM_IME_STARTCOMPOSITION, 0, 0)],
                );
            }
            6 => {
                assert_eq!(self.take_compositions(), ["1 #2 enabled"]);
                // The host moves the text focus: the window is asked to complete the composition first (the tracker's
                // request, carried out), and the composition ends at its own target once that is over.
                self.window_ime.clear();
                self.focus(control, text_focus(B, true));
                assert_eq!(
                    self.take_compositions(),
                    Vec::<String>::new(),
                    "it waits for the completion, and is not cancelled before"
                );
            }
            7 => {
                assert_eq!(self.window_ime.all(), ["completed"], "the window completed it");
                assert_eq!(
                    self.take_compositions(),
                    ["1 #2 disabled input.synthetic.focus_moved"],
                    "ended at its own target when the completion was over"
                );
                post(
                    hwnd,
                    &[(WM_IME_COMPOSITION, 0, 0), (WM_IME_ENDCOMPOSITION, 0, 0)],
                );
            }
            8 => {
                assert_eq!(
                    self.take_compositions(),
                    [
                        "2 #3 enabled input.synthetic.rebound",
                        "2 #3 preedit \"\" None",
                        "2 #3 disabled"
                    ],
                    "what the input method went on with is the new target's"
                );
                self.focus(control, text_focus(C, false));
                self.window_ime.clear();
                post(
                    hwnd,
                    &[
                        (WM_IME_STARTCOMPOSITION, 0, 0),
                        (WM_IME_ENDCOMPOSITION, 0, 0),
                    ],
                );
            }
            9 => {
                assert_eq!(
                    self.window_ime.all(),
                    Vec::<String>::new(),
                    "composition is off for the window"
                );
                assert_eq!(self.take_compositions(), Vec::<String>::new());
                let report = self.tracker.borrow().report();
                assert_eq!(report.compositions_settled, 2, "{report:?}");
                assert_eq!(report.stale_stamps, 0);
                control
                    .command(
                        &CommandSource::host(900),
                        self.key(),
                        WindowCommand::Destroy,
                    )
                    .unwrap();
                return;
            }
            other => panic!("step {other}"),
        }
        self.next();
    }
}

#[test]
fn keys_and_the_input_method_on_a_real_loop_reach_the_focused_target_and_end_once() {
    on_private_desktop(Duration::from_secs(60), || {
        let state = Rc::new(State::default());
        let (events, steps) = (Rc::clone(&state), Rc::clone(&state));
        let report = run_script(
            LoopConfig::default(),
            move |control, event| events.event(control, event),
            move |control, _| steps.step(control),
        )
        .unwrap();
        assert_eq!(state.step.get(), 9, "every step ran");
        assert_eq!(report.quiet_broken, 0);
        assert_eq!(
            report.ime_completions, 2,
            "the loss of the keyboard, and the host's move of the text focus"
        );
        assert_eq!(
            report.refusals, 0,
            "every request of the tracker was carried out"
        );
    });
}
