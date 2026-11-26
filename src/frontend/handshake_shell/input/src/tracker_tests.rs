//! The window loop's events as typed input: mapping, origin, keys held, dead keys and repeats, and a model's input.
//! Window events here carry a model's stamp, the only one a caller can make; the Operator's origin is checked where the
//! tracker decides on it.

use super::*;
use handshake_shell_window::{
    ContactEvent, ContactPhase, DeviceKind, Display, ImeChange, ImeInput, KeyCode, Modifiers,
    NamedKey, PhysicalPosition, PhysicalRect, PhysicalSize, PointerButtonEvent, PointerMoved,
    ScaleFactor, TextEvent, WheelEvent, WindowClass, WindowCommand, WindowState,
};

use crate::ime::{Composition, LogicalRect};

const W: WindowKey = WindowKey::from_number(1);
const V: WindowKey = WindowKey::from_number(2);
const MODEL: Attribution = Attribution {
    actor: 7,
    session: 8,
    correlation: 9,
};

fn at(nanos: u64) -> EventTime {
    EventTime::from_nanos(nanos)
}

fn stamp(nanos: u64) -> InputStamp {
    InputStamp::model_action(MODEL, at(nanos))
}

fn window(window: WindowKey, event: WindowEvent) -> Event {
    Event::Window { window, event }
}

fn created(key: WindowKey, focused: bool) -> Event {
    let state = WindowState {
        class: WindowClass::Quiet,
        display: Display::Hidden,
        focused,
        outer: PhysicalRect::new(0, 0, 200, 100),
        inner: PhysicalSize::new(200, 100),
        scale: ScaleFactor::ONE,
        monitor: None,
        native_tokens: 0,
    };
    window(key, WindowEvent::Created { command: 1, state })
}

fn physical(code: KeyCode, scancode: u32) -> PhysicalKey {
    PhysicalKey { code, scancode }
}

fn key(code: KeyCode, logical: LogicalKey, state: PressState, repeat: bool, nanos: u64) -> Event {
    window(
        W,
        WindowEvent::Key(KeyEvent {
            physical: physical(code, 0x10),
            logical,
            location: KeyLocation::Standard,
            state,
            repeat,
            focus_sync: false,
            stamp: stamp(nanos),
        }),
    )
}

fn text(typed: &str, nanos: u64) -> Event {
    window(
        W,
        WindowEvent::Text(TextEvent {
            text: TypedText::new(typed),
            stamp: stamp(nanos),
        }),
    )
}

fn feed_all(tracker: &mut InputTracker, events: &[Event]) -> Vec<InputEvent> {
    let mut out = Vec::new();
    for (number, event) in events.iter().enumerate() {
        out.extend(tracker.feed(at(1000 + number as u64), event));
    }
    out
}

/// A key's phase and whether it is dead, or the text, one per input.
fn keys(events: &[InputEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| match event.input() {
            Input::Key(key) => format!(
                "{:?}{}{}",
                key.phase,
                if key.is_dead() { " dead" } else { "" },
                event
                    .synthetic()
                    .map(|cause| format!(" {}", cause.code()))
                    .unwrap_or_default()
            ),
            Input::Text(text) => format!("text {}", text.as_str()),
            other => format!("{other:?}"),
        })
        .collect()
}

/// AC214-5: every input of the window arrives as one typed input with its stamp's origin, time and stamp.
#[test]
fn every_input_of_the_window_is_one_typed_input_with_its_stamp() {
    let point = ClientPoint { x: 3.5, y: 4.25 };
    let given = [
        (
            WindowEvent::Text(TextEvent {
                text: TypedText::new("a"),
                stamp: stamp(1),
            }),
            "Text",
        ),
        (
            WindowEvent::ModifiersChanged {
                modifiers: Modifiers {
                    shift: true,
                    ..Modifiers::default()
                },
                stamp: stamp(2),
            },
            "Modifiers",
        ),
        (
            WindowEvent::PointerMoved(PointerMoved {
                position: point,
                stamp: stamp(3),
            }),
            "PointerMoved",
        ),
        (WindowEvent::PointerEntered(stamp(4)), "PointerEntered"),
        (WindowEvent::PointerLeft(stamp(5)), "PointerLeft"),
        (
            WindowEvent::PointerButton(PointerButtonEvent {
                button: PointerButton::Secondary,
                state: PressState::Pressed,
                position: Some(point),
                stamp: stamp(6),
            }),
            "PointerButton",
        ),
        (
            WindowEvent::Wheel(WheelEvent {
                x: 0,
                y: 120,
                precise: false,
                position: Some(point),
                stamp: stamp(7),
            }),
            "Wheel",
        ),
        (
            WindowEvent::Contact(ContactEvent {
                contact: 4,
                phase: ContactPhase::Moved,
                position: point,
                pressure: Some(0.5),
                stamp: stamp(8),
            }),
            "Contact",
        ),
        (
            WindowEvent::Key(KeyEvent {
                physical: physical(KeyCode::KeyA, 0x1E),
                logical: LogicalKey::Text(TypedText::new("a")),
                location: KeyLocation::Standard,
                state: PressState::Pressed,
                repeat: false,
                focus_sync: false,
                stamp: stamp(9),
            }),
            "Key",
        ),
    ];
    let mut tracker = InputTracker::new();
    for (event, kind) in given {
        let sent = match &event {
            WindowEvent::Text(e) => e.stamp,
            WindowEvent::ModifiersChanged { stamp, .. } => *stamp,
            WindowEvent::PointerMoved(e) => e.stamp,
            WindowEvent::PointerEntered(s) | WindowEvent::PointerLeft(s) => *s,
            WindowEvent::PointerButton(e) => e.stamp,
            WindowEvent::Wheel(e) => e.stamp,
            WindowEvent::Contact(e) => e.stamp,
            WindowEvent::Key(e) => e.stamp,
            other => panic!("{other:?}"),
        };
        let out = tracker.feed(at(99), &window(W, event));
        assert_eq!(out.len(), 1, "{kind}: {out:?}");
        let input = &out[0];
        assert!(
            format!("{:?}", input.input()).starts_with(kind),
            "{kind}: {input:?}"
        );
        assert_eq!(input.window(), W);
        assert_eq!(input.origin(), Origin::ModelAction(MODEL), "{kind}");
        assert_eq!(
            input.at(),
            sent.received(),
            "{kind}: the input's time, not the feed's"
        );
        assert_eq!(input.stamp(), Some(&sent), "{kind}");
        assert_eq!(input.synthetic(), None, "{kind}");
    }
    match tracker.feed(
        at(99),
        &window(
            W,
            WindowEvent::PointerButton(PointerButtonEvent {
                button: PointerButton::Primary,
                state: PressState::Released,
                position: Some(point),
                stamp: stamp(10),
            }),
        ),
    )[0]
    .input()
    {
        Input::PointerButton {
            button,
            state,
            position,
        } => {
            assert_eq!(
                (*button, *state, *position),
                (PointerButton::Primary, PressState::Released, Some(point))
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn what_is_no_input_gives_none() {
    let mut tracker = InputTracker::new();
    let quiet = [
        created(W, false),
        window(
            W,
            WindowEvent::Moved {
                outer: PhysicalPosition::new(1, 2),
            },
        ),
        window(W, WindowEvent::CloseRequested),
        window(W, WindowEvent::QuietBroken),
        window(
            W,
            WindowEvent::DisplayChanged {
                display: Display::Normal,
            },
        ),
        window(W, WindowEvent::MonitorChanged { monitor: None }),
        window(W, WindowEvent::FocusChanged { focused: true }),
        window(
            W,
            WindowEvent::Resized {
                inner: PhysicalSize::new(10, 10),
                scale: ScaleFactor::ONE,
            },
        ),
        Event::LoopStarted,
        Event::MonitorsChanged,
        Event::ShutdownBegan,
    ];
    assert!(feed_all(&mut tracker, &quiet).is_empty());
}

#[test]
fn a_wheel_says_what_scrolled() {
    assert_eq!(
        WheelSource::of(DeviceKind::Touchpad, false),
        WheelSource::Touchpad
    );
    assert_eq!(
        WheelSource::of(DeviceKind::Touchpad, true),
        WheelSource::Touchpad
    );
    assert_eq!(
        WheelSource::of(DeviceKind::Mouse, true),
        WheelSource::FineWheel
    );
    assert_eq!(
        WheelSource::of(DeviceKind::Mouse, false),
        WheelSource::Detents
    );
    assert_eq!(
        WheelSource::of(DeviceKind::Unknown, false),
        WheelSource::Detents
    );
}

/// The tracker's rule for the Operator's origin: the loop hands inputs out in the order it stamped them, so an older
/// Operator stamp is a copy.
#[test]
fn an_operator_stamp_older_than_one_seen_is_not_the_operators() {
    let mut tracker = InputTracker::new();
    assert_eq!(
        tracker.classify(Origin::OperatorDevice, at(10)),
        Origin::OperatorDevice
    );
    assert_eq!(
        tracker.classify(Origin::OperatorDevice, at(10)),
        Origin::OperatorDevice,
        "a key and its text share one"
    );
    assert_eq!(
        tracker.classify(Origin::OperatorDevice, at(20)),
        Origin::OperatorDevice
    );
    assert_eq!(
        tracker.classify(Origin::OperatorDevice, at(15)),
        Origin::Unattributed,
        "a copy"
    );
    assert_eq!(tracker.report().stale_stamps, 1);
    assert_eq!(
        tracker.classify(Origin::ModelAction(MODEL), at(1)),
        Origin::ModelAction(MODEL)
    );
    assert_eq!(
        tracker.classify(Origin::OtherProgram, at(1)),
        Origin::OtherProgram
    );
    assert_eq!(
        tracker.report().stale_stamps,
        1,
        "only the Operator's origin is checked"
    );
}

/// AC214-5: a model's input is the model's, whatever it is.
#[test]
fn a_models_input_is_always_the_models() {
    let mut tracker = InputTracker::new();
    assert_eq!(
        tracker.model_input(W, MODEL, at(1), ModelInput::Text(TypedText::new("x"))),
        Err(ModelRefusal::UnknownWindow)
    );
    tracker.feed(at(0), &created(W, true));
    let inputs = [
        ModelInput::Key {
            physical: physical(KeyCode::KeyA, 0x1E),
            logical: LogicalKey::Text(TypedText::new("a")),
            location: KeyLocation::Standard,
            state: PressState::Pressed,
            repeat: false,
        },
        ModelInput::Text(TypedText::new("a")),
        ModelInput::PointerMoved(ClientPoint { x: 1.0, y: 2.0 }),
        ModelInput::PointerButton {
            button: PointerButton::Primary,
            state: PressState::Pressed,
            position: None,
        },
        ModelInput::Wheel {
            x: 0,
            y: 30,
            position: None,
        },
    ];
    for input in inputs {
        let made = tracker.model_input(W, MODEL, at(5), input).unwrap();
        assert_eq!(made.origin(), Origin::ModelAction(MODEL));
        assert!(made.is_model());
        assert_ne!(made.origin(), Origin::OperatorDevice);
        assert_eq!(
            made.stamp().map(InputStamp::origin),
            Some(Origin::ModelAction(MODEL))
        );
        assert_eq!(made.at(), at(5));
        assert_eq!(made.synthetic(), None);
    }
    let wheel = tracker
        .model_input(
            W,
            MODEL,
            at(5),
            ModelInput::Wheel {
                x: 0,
                y: 30,
                position: None,
            },
        )
        .unwrap();
    assert!(matches!(
        wheel.input(),
        Input::Wheel(WheelInput {
            source: WheelSource::FineWheel,
            ..
        })
    ));
    let key = |state, repeat| ModelInput::Key {
        physical: physical(KeyCode::KeyB, 0x30),
        logical: LogicalKey::Text(TypedText::new("b")),
        location: KeyLocation::Standard,
        state,
        repeat,
    };
    let phases: Vec<KeyPhase> = [
        (PressState::Pressed, false),
        (PressState::Pressed, true),
        (PressState::Released, false),
    ]
    .into_iter()
    .map(|(state, repeat)| {
        match tracker
            .model_input(W, MODEL, at(6), key(state, repeat))
            .unwrap()
            .input()
        {
            Input::Key(key) => key.phase,
            other => panic!("{other:?}"),
        }
    })
    .collect();
    assert_eq!(
        phases,
        [KeyPhase::Press, KeyPhase::Repeat, KeyPhase::Release]
    );
}

/// GUI-DESKTOP-041: while the Operator composes, a model's keys and text are refused; its pointer is not.
#[test]
fn a_model_cannot_type_into_the_operators_composition() {
    let mut tracker = InputTracker::new();
    tracker.feed(at(0), &created(W, true));
    let focus = TextFocus {
        target: crate::TextTarget(1),
        accepts_ime: true,
        caret: crate::CaretPlace::default(),
    };
    tracker
        .focus_text(W, Some(focus), FocusBy::Host, at(1))
        .unwrap();
    let operator = Source {
        origin: Origin::OperatorDevice,
        at: at(2),
        stamp: None,
    };
    let mut out = Vec::new();
    tracker
        .ime
        .change(W, &ImeChange::Started, operator, &mut out);
    assert_eq!(
        tracker.model_input(W, MODEL, at(3), ModelInput::Text(TypedText::new("x"))),
        Err(ModelRefusal::OperatorComposing)
    );
    let typed = ModelInput::Key {
        physical: physical(KeyCode::KeyA, 0x1E),
        logical: LogicalKey::Text(TypedText::new("a")),
        location: KeyLocation::Standard,
        state: PressState::Pressed,
        repeat: false,
    };
    assert_eq!(
        tracker.model_input(W, MODEL, at(3), typed),
        Err(ModelRefusal::OperatorComposing)
    );
    assert!(tracker
        .model_input(
            W,
            MODEL,
            at(3),
            ModelInput::PointerMoved(ClientPoint::default())
        )
        .is_ok());
    assert_eq!(
        tracker.focus_text(W, None, FocusBy::Model(MODEL), at(4)),
        Err(FocusRefusal::OperatorComposing)
    );
    let models_click = tracker
        .model_input(
            W,
            MODEL,
            at(5),
            ModelInput::PointerButton {
                button: PointerButton::Primary,
                state: PressState::Pressed,
                position: None,
            },
        )
        .unwrap();
    assert_eq!(
        tracker.focus_text(W, None, FocusBy::Input(&models_click), at(5)),
        Err(FocusRefusal::OperatorComposing),
        "a model's click counts as the model's"
    );
    assert_eq!(tracker.report().model_refused, 4);
    // The shell may: the window is asked to complete the composition, which ends when that is over (MT-214 S3-Q1).
    let ended = tracker.focus_text(W, None, FocusBy::Host, at(6)).unwrap();
    assert!(ended.is_empty(), "it waits for the completion");
    assert!(tracker
        .take_requests()
        .iter()
        .any(|(_, request)| *request == ImeRequest::Complete));
    let mut ended = Vec::new();
    tracker
        .ime
        .change(W, &ImeChange::Completed, operator, &mut ended);
    assert!(matches!(
        ended.last().map(InputEvent::input),
        Some(Input::Ime(ime)) if ime.change == Composition::Disabled
    ));
    assert!(tracker
        .model_input(W, MODEL, at(7), ModelInput::Text(TypedText::new("x")))
        .is_ok());
}

/// MT-214 E7-F7 (mutation M-B1): another program, through the accessibility tree, is no better placed than a model: it
/// may not take the text focus or the composed text from the Operator's composition. Fails if `FocusBy::OtherProgram`
/// stops yielding, and if the refusal outlives the composition.
#[test]
fn another_program_cannot_take_the_focus_from_the_operators_composition() {
    let mut tracker = InputTracker::new();
    tracker.feed(at(0), &created(W, true));
    let focus = |target| {
        Some(TextFocus {
            target: crate::TextTarget(target),
            accepts_ime: true,
            caret: crate::CaretPlace::default(),
        })
    };
    tracker
        .focus_text(W, focus(1), FocusBy::Host, at(1))
        .unwrap();
    let operator = Source {
        origin: Origin::OperatorDevice,
        at: at(2),
        stamp: None,
    };
    let mut out = Vec::new();
    tracker
        .ime
        .change(W, &ImeChange::Started, operator, &mut out);
    assert_eq!(
        tracker.focus_text(W, focus(2), FocusBy::OtherProgram, at(3)),
        Err(FocusRefusal::OperatorComposing),
        "to another target"
    );
    assert_eq!(
        tracker.focus_text(W, None, FocusBy::OtherProgram, at(3)),
        Err(FocusRefusal::OperatorComposing),
        "to none"
    );
    assert_eq!(tracker.text_focus(W), Some(crate::TextTarget(1)), "the focus stayed");
    assert_eq!(tracker.report().model_refused, 2, "counted as refused");
    // The Operator's composition over, another program may move the focus.
    tracker.focus_text(W, None, FocusBy::Host, at(4)).unwrap();
    let mut ended = Vec::new();
    tracker
        .ime
        .change(W, &ImeChange::Completed, operator, &mut ended);
    let moved = tracker
        .focus_text(W, focus(2), FocusBy::OtherProgram, at(5))
        .unwrap();
    assert!(moved.is_empty());
    assert_eq!(tracker.text_focus(W), Some(crate::TextTarget(2)));
}

/// Every press after the first until the release is a repeat, with or without the operating system's mark.
#[test]
fn a_key_is_pressed_once_then_repeats_until_released() {
    let mut tracker = InputTracker::new();
    let a = || LogicalKey::Text(TypedText::new("a"));
    let out = feed_all(
        &mut tracker,
        &[
            key(KeyCode::KeyA, a(), PressState::Pressed, false, 1),
            text("a", 1),
            key(KeyCode::KeyA, a(), PressState::Pressed, true, 2),
            text("a", 2),
            key(KeyCode::KeyA, a(), PressState::Pressed, false, 3),
            key(KeyCode::KeyA, a(), PressState::Released, false, 4),
            key(KeyCode::KeyA, a(), PressState::Pressed, true, 5),
            key(KeyCode::KeyA, a(), PressState::Released, false, 6),
            key(KeyCode::KeyA, a(), PressState::Released, false, 7),
        ],
    );
    assert_eq!(
        keys(&out),
        [
            "Press", "text a", "Repeat", "text a", "Repeat", "Release",
            "Repeat", "Release", "Release"
        ],
        "a mark without a press before is a repeat of a press the window did not see; a second release passes"
    );
}

/// Dead keys as winit 0.30.13 reports them on Windows (`keyboard.rs`: the dead key's `WM_DEADCHAR` gives a key with no
/// text; the next key's `WM_CHAR`s give its text with the accent, or the accent and the character).
#[test]
fn a_dead_key_types_nothing_and_the_next_key_types_its_accent_once() {
    let dead = || LogicalKey::Dead(Some('^'));
    for (next, typed) in [
        (LogicalKey::Text(TypedText::new("e")), "ê"),
        (LogicalKey::Named(NamedKey::Space), "^"),
        (LogicalKey::Text(TypedText::new("x")), "^x"),
    ] {
        let mut tracker = InputTracker::new();
        let out = feed_all(
            &mut tracker,
            &[
                key(KeyCode::BracketLeft, dead(), PressState::Pressed, false, 1),
                key(KeyCode::BracketLeft, dead(), PressState::Released, false, 2),
                key(KeyCode::KeyE, next.clone(), PressState::Pressed, false, 3),
                text(typed, 3),
                key(KeyCode::KeyE, next, PressState::Released, false, 4),
            ],
        );
        assert_eq!(
            keys(&out),
            [
                "Press dead",
                "Release dead",
                "Press",
                format!("text {typed}").as_str(),
                "Release"
            ]
        );
        let text_time = out
            .iter()
            .find(|event| matches!(event.input(), Input::Text(_)))
            .unwrap()
            .at();
        assert_eq!(text_time, at(3), "the text is the completing key's");
    }
    // Held, a dead key repeats as a dead key.
    let mut tracker = InputTracker::new();
    let out = feed_all(
        &mut tracker,
        &[
            key(KeyCode::BracketLeft, dead(), PressState::Pressed, false, 1),
            key(KeyCode::BracketLeft, dead(), PressState::Pressed, true, 2),
            key(KeyCode::BracketLeft, dead(), PressState::Released, false, 3),
        ],
    );
    assert_eq!(keys(&out), ["Press dead", "Repeat dead", "Release dead"]);
}

/// When the window loses the keyboard, what is held ends once, with the origin of its press; what the window library
/// released itself is not released again.
#[test]
fn keys_held_when_the_keyboard_goes_are_released_once_with_their_origin() {
    let mut tracker = InputTracker::new();
    tracker.feed(at(0), &created(W, true));
    let a = || LogicalKey::Text(TypedText::new("a"));
    let mut synced = KeyEvent {
        physical: physical(KeyCode::ShiftLeft, 0x2A),
        logical: LogicalKey::Named(NamedKey::Shift),
        location: KeyLocation::Left,
        state: PressState::Pressed,
        repeat: false,
        focus_sync: false,
        stamp: stamp(1),
    };
    let mut out = feed_all(
        &mut tracker,
        &[
            window(W, WindowEvent::Key(synced.clone())),
            key(
                KeyCode::BracketLeft,
                LogicalKey::Dead(Some('^')),
                PressState::Pressed,
                false,
                2,
            ),
            key(KeyCode::KeyA, a(), PressState::Pressed, false, 3),
            window(
                W,
                WindowEvent::PointerButton(PointerButtonEvent {
                    button: PointerButton::Primary,
                    state: PressState::Pressed,
                    position: None,
                    stamp: stamp(4),
                }),
            ),
        ],
    );
    // The library's own release of Shift as the focus goes, then the focus.
    synced.state = PressState::Released;
    synced.focus_sync = true;
    out.clear();
    out.extend(tracker.feed(at(50), &window(W, WindowEvent::Key(synced))));
    out.extend(tracker.feed(
        at(51),
        &window(W, WindowEvent::FocusChanged { focused: false }),
    ));
    assert_eq!(
        keys(&out),
        [
            "Release",
            "Release dead input.synthetic.focus_lost",
            "Release input.synthetic.focus_lost"
        ]
    );
    for released in &out[1..] {
        assert_eq!(
            released.origin(),
            Origin::ModelAction(MODEL),
            "the press's origin"
        );
        assert_eq!(released.stamp(), None);
        assert_eq!(released.at(), at(51));
    }
    assert!(
        tracker
            .feed(
                at(52),
                &window(W, WindowEvent::FocusChanged { focused: false })
            )
            .is_empty(),
        "nothing is held twice"
    );
    // The button stays held: the keyboard's focus is not the pointer's.
    let lost = tracker.feed(at(60), &Event::EventsLost { count: 3 });
    assert!(
        matches!(
            lost.as_slice(),
            [event] if matches!(event.input(), Input::PointerButton { state: PressState::Released, .. })
                && event.synthetic() == Some(Synthetic::EventsLost)
        ),
        "{lost:?}"
    );
}

#[test]
fn lost_events_and_a_window_gone_release_keys_and_buttons_of_every_window() {
    let mut tracker = InputTracker::new();
    let b = |window_key| {
        window(
            window_key,
            WindowEvent::PointerButton(PointerButtonEvent {
                button: PointerButton::Middle,
                state: PressState::Pressed,
                position: None,
                stamp: stamp(1),
            }),
        )
    };
    feed_all(
        &mut tracker,
        &[
            created(W, true),
            created(V, false),
            key(
                KeyCode::KeyQ,
                LogicalKey::Text(TypedText::new("q")),
                PressState::Pressed,
                false,
                1,
            ),
            b(W),
            b(V),
        ],
    );
    let lost = tracker.feed(at(9), &Event::EventsLost { count: 1 });
    let windows: Vec<WindowKey> = lost.iter().map(InputEvent::window).collect();
    assert_eq!(windows, [W, W, V], "W's key and button, then V's button");
    assert!(lost
        .iter()
        .all(|event| event.synthetic() == Some(Synthetic::EventsLost)));
    feed_all(&mut tracker, &[b(V)]);
    let gone = tracker.feed(at(10), &window(V, WindowEvent::Destroyed));
    assert_eq!(gone.len(), 1);
    assert_eq!(gone[0].synthetic(), Some(Synthetic::WindowGone));
    assert_eq!(
        tracker.model_input(
            V,
            MODEL,
            at(11),
            ModelInput::PointerMoved(ClientPoint::default())
        ),
        Err(ModelRefusal::UnknownWindow)
    );
}

#[test]
fn holds_beyond_the_limit_are_passed_on_and_counted() {
    let mut tracker = InputTracker::new();
    for number in 0..=MAX_HELD_KEYS as u32 {
        let event = window(
            W,
            WindowEvent::Key(KeyEvent {
                physical: physical(KeyCode::Unidentified, number + 1),
                logical: LogicalKey::Unidentified(number),
                location: KeyLocation::Standard,
                state: PressState::Pressed,
                repeat: false,
                focus_sync: false,
                stamp: stamp(u64::from(number)),
            }),
        );
        assert_eq!(tracker.feed(at(1), &event).len(), 1);
    }
    assert_eq!(tracker.report().holds_not_kept, 1);
    let released = tracker.feed(
        at(2),
        &window(W, WindowEvent::FocusChanged { focused: false }),
    );
    assert_eq!(released.len(), MAX_HELD_KEYS);
}

/// The input method's events through the tracker: routed to the focused target with the stamp's origin, and ended
/// when the window loses the keyboard; the window is asked to compose and where.
#[test]
fn input_method_events_reach_the_focused_target_through_the_tracker() {
    let mut tracker = InputTracker::new();
    tracker.feed(at(0), &created(W, true));
    let caret = crate::CaretPlace {
        viewport: LogicalRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        },
        caret: LogicalRect {
            x: 5.0,
            y: 6.0,
            width: 1.0,
            height: 12.0,
        },
        scroll_x: 0.0,
        scroll_y: 0.0,
    };
    let focus = TextFocus {
        target: crate::TextTarget(3),
        accepts_ime: true,
        caret,
    };
    assert!(tracker
        .focus_text(W, Some(focus), FocusBy::Host, at(1))
        .unwrap()
        .is_empty());
    let requests: Vec<WindowCommand> = tracker
        .take_requests()
        .into_iter()
        .map(|(_, request)| request.command())
        .collect();
    assert_eq!(
        requests,
        [
            WindowCommand::AllowIme(true),
            WindowCommand::SetImeArea(handshake_shell_window::ClientRect {
                x: 5,
                y: 6,
                width: 1,
                height: 12
            })
        ]
    );
    let ime = |change, nanos| {
        window(
            W,
            WindowEvent::Ime(ImeInput {
                change,
                stamp: stamp(nanos),
            }),
        )
    };
    let out = feed_all(
        &mut tracker,
        &[
            ime(ImeChange::Started, 2),
            ime(
                ImeChange::Preedit {
                    text: TypedText::new("か"),
                    cursor: Some((3, 3)),
                },
                3,
            ),
            window(W, WindowEvent::FocusChanged { focused: false }),
            // The preedit no result took: the input method's own end is a cancel (MT-214 S3-Q1).
            ime(ImeChange::Ended, 4),
        ],
    );
    let changes: Vec<(Composition, Option<Synthetic>, Origin)> = out
        .iter()
        .map(|event| match event.input() {
            Input::Ime(ime) => {
                assert_eq!(ime.target, crate::TextTarget(3));
                (ime.change.clone(), event.synthetic(), event.origin())
            }
            other => panic!("{other:?}"),
        })
        .collect();
    let model = Origin::ModelAction(MODEL);
    assert_eq!(
        changes,
        [
            (Composition::Enabled, None, model),
            (
                Composition::Preedit {
                    text: TypedText::new("か"),
                    cursor: Some(3..3)
                },
                None,
                model
            ),
            (Composition::Cancel, Some(Synthetic::FocusLost), model),
            (Composition::Disabled, Some(Synthetic::FocusLost), model),
        ]
    );
    assert_eq!(out[1].stamp().map(InputStamp::received), Some(at(3)));
    assert_eq!(out[1].text_target(), Some(crate::TextTarget(3)));
    assert_eq!(tracker.report().compositions_settled, 1);
    assert_eq!(
        tracker.move_caret(W, crate::TextTarget(4), caret),
        Err(FocusRefusal::NotFocused)
    );
    assert_eq!(
        tracker.scroll_text(W, crate::TextTarget(3), 0.0, 3.0),
        Ok(())
    );
    assert_eq!(
        tracker.take_requests(),
        [(
            W,
            ImeRequest::Area(handshake_shell_window::ClientRect {
                x: 5,
                y: 3,
                width: 1,
                height: 12
            })
        )]
    );
}

#[test]
fn a_key_input_prints_no_key() {
    let key = KeyInput {
        physical: physical(KeyCode::KeyQ, 0x10),
        logical: LogicalKey::Text(TypedText::new("q")),
        location: KeyLocation::Numpad,
        phase: KeyPhase::Press,
        focus_sync: false,
    };
    let printed = format!("{key:?} {:?}", Input::Key(key.clone()));
    for secret in ["KeyQ", "16", "\"q\"", "Numpad"] {
        assert!(!printed.contains(secret), "{secret} in {printed}");
    }
    assert!(printed.contains("Press"));
}
