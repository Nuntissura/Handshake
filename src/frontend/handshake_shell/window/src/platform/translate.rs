//! The window library's events as this crate's. Nothing of the library gets past this file.

use std::rc::Rc;

use windows_sys::Win32::UI::Input::{
    IMDT_KEYBOARD, IMDT_MOUSE, IMDT_PEN, IMDT_TOUCH, IMDT_TOUCHPAD, IMO_HARDWARE, IMO_INJECTED,
    INPUT_MESSAGE_DEVICE_TYPE, INPUT_MESSAGE_ORIGIN_ID,
};
use winit::event::{
    ElementState, Force, Ime as LibraryIme, MouseButton, MouseScrollDelta, Touch, TouchPhase,
    WindowEvent as Library,
};
use winit::keyboard::{
    Key, KeyCode as LibraryCode, KeyLocation as LibraryLocation, ModifiersState,
    NamedKey as LibraryNamed, NativeKey, PhysicalKey as LibraryPhysical,
};
use winit::platform::scancode::PhysicalKeyExtScancode;
use winit::window::CursorIcon;

use super::loop_core::Core;
use super::place;
use super::window::Window;
use crate::event::WindowEvent;
use crate::geometry::{ClientPoint, ScaleFactor};
use crate::input::{
    ContactEvent, ContactPhase, CursorShape, ImeChange, ImeInput, KeyEvent, KeyLocation, LogicalKey,
    Modifiers, PhysicalKey, PointerButton, PointerButtonEvent, PointerMoved, PressState, TextEvent,
    TypedText, WheelEvent, WHEEL_DETENT,
};
use crate::keys::{with_key_codes, with_named_keys, KeyCode, NamedKey};
use crate::origin::{DeviceKind, InputStamp, Origin};

/// An event of one window. Only puts events into the queue, so the library is never entered again from here.
pub(super) fn window_event(core: &Rc<Core>, window: &Rc<Window>, event: Library) {
    let push = |event| core.push_window(window.key, event);
    match event {
        Library::Resized(_) => window.report_size(core),
        Library::Moved(_) => window.report_position(core),
        Library::ScaleFactorChanged {
            scale_factor,
            mut inner_size_writer,
        } => {
            let Some(to) = ScaleFactor::from_factor(scale_factor) else {
                return;
            };
            let from = window.scale.replace(to);
            // Not for a minimised or maximised window, also while hidden: a size asked for there makes the library
            // restore it with SW_RESTORE, which shows and activates it, and then hide it again (MT-214 W3).
            if place::is_normal_size(window.hwnd) {
                // From the logical size and not from the size so far, so crossing monitors back and forth never drifts.
                let size = window.logical.get().to_physical(to);
                let size = winit::dpi::PhysicalSize::new(size.width, size.height);
                // Fails only once the library has taken the size.
                let _ = inner_size_writer.request_inner_size(size);
            }
            if from != to {
                push(WindowEvent::ScaleChanged { from, to });
            }
        }
        // The request itself was opened when the close message came (`window.rs`).
        Library::CloseRequested => push(WindowEvent::CloseRequested),
        Library::Destroyed => core.window_destroyed(window.key),
        Library::Focused(focused) => {
            if window.set_focused(focused) {
                push(WindowEvent::FocusChanged { focused });
            }
        }
        Library::RedrawRequested => {
            // A window that shows nothing keeps the frame it wants until it does.
            if window.can_show() {
                let due = window.frames.borrow_mut().paint(core.now().nanos);
                if let Some((ticket, cause)) = due {
                    push(WindowEvent::FrameDue { ticket, cause });
                }
            }
        }
        Library::ModifiersChanged(changed) => {
            let modifiers = modifiers(changed.state());
            if window.modifiers.replace(modifiers) != modifiers {
                let stamp = window.stamp(core);
                push(WindowEvent::ModifiersChanged { modifiers, stamp });
            }
        }
        Library::KeyboardInput {
            event,
            is_synthetic,
            ..
        } => {
            let stamp = window.stamp(core);
            push(WindowEvent::Key(KeyEvent {
                physical: physical_key(event.physical_key),
                logical: logical_key(&event.logical_key),
                location: location(event.location),
                state: press_state(event.state),
                repeat: event.repeat,
                focus_sync: is_synthetic,
                stamp,
            }));
            let pressed = event.state == ElementState::Pressed && !is_synthetic;
            went_on(window, pressed, stamp);
            if let Some(text) = typed_text(event.text.as_deref(), pressed, stamp.modifiers()) {
                push(WindowEvent::Text(TextEvent { text, stamp }));
            }
        }
        Library::CursorMoved { position, .. } => {
            let position = ClientPoint {
                x: position.x,
                y: position.y,
            };
            window.pointer.set(Some(position));
            let stamp = window.stamp(core);
            push(WindowEvent::PointerMoved(PointerMoved { position, stamp }));
        }
        Library::CursorEntered { .. } => push(WindowEvent::PointerEntered(window.stamp(core))),
        Library::CursorLeft { .. } => push(WindowEvent::PointerLeft(window.stamp(core))),
        Library::MouseInput { state, button, .. } => {
            let stamp = window.stamp(core);
            went_on(window, state == ElementState::Pressed, stamp);
            push(WindowEvent::PointerButton(PointerButtonEvent {
                button: pointer_button(button),
                state: press_state(state),
                position: window.pointer.get(),
                stamp,
            }))
        }
        Library::MouseWheel { delta, .. } => {
            let stamp = window.stamp(core);
            let (x, y) = wheel_units(delta);
            push(WindowEvent::Wheel(WheelEvent {
                x,
                y,
                precise: is_precise(stamp.device(), x, y),
                position: window.pointer.get(),
                stamp,
            }));
        }
        Library::Touch(touch) => push(WindowEvent::Contact(contact(&touch, window, core))),
        // Only while `AllowIme` is on: the library keeps composition off until asked (MT-214 S3).
        Library::Ime(ime) => push(WindowEvent::Ime(ImeInput {
            change: ime_change(ime),
            stamp: window.stamp(core),
        })),
        // The accessibility tree is a later slice. Themes, device paths and gestures of other platforms are not used.
        _ => {}
    }
}

/// The library's input method event as a change. On Windows the library reports a composition's start and end as
/// enabled and disabled (`WM_IME_STARTCOMPOSITION`, `WM_IME_ENDCOMPOSITION`), its cursor in bytes.
fn ime_change(ime: LibraryIme) -> ImeChange {
    match ime {
        LibraryIme::Enabled => ImeChange::Started,
        LibraryIme::Preedit(text, cursor) => ImeChange::Preedit {
            text: TypedText::new(text),
            cursor,
        },
        LibraryIme::Commit(text) => ImeChange::Commit(TypedText::new(text)),
        LibraryIme::Disabled => ImeChange::Ended,
    }
}

/// The Operator pressed a key or a button in the window after asking to close it: the request is over (MT-214 W4).
fn went_on(window: &Window, pressed: bool, stamp: InputStamp) {
    if pressed && stamp.origin() == Origin::OperatorDevice {
        window.closing.end();
    }
}

/// Where the operating system says an input message came from.
pub(super) fn message_origin(
    origin: INPUT_MESSAGE_ORIGIN_ID,
    device: INPUT_MESSAGE_DEVICE_TYPE,
) -> (Origin, DeviceKind) {
    let origin = match origin {
        IMO_HARDWARE => Origin::OperatorDevice,
        IMO_INJECTED => Origin::OtherProgram,
        _ => Origin::Unattributed,
    };
    let device = match device {
        IMDT_KEYBOARD => DeviceKind::Keyboard,
        IMDT_MOUSE => DeviceKind::Mouse,
        IMDT_TOUCH => DeviceKind::Touch,
        IMDT_PEN => DeviceKind::Pen,
        IMDT_TOUCHPAD => DeviceKind::Touchpad,
        _ => DeviceKind::Unknown,
    };
    (origin, device)
}

/// The library's cursor for a shape; `None` hides the pointer.
pub(super) fn cursor_icon(shape: CursorShape) -> Option<CursorIcon> {
    Some(match shape {
        CursorShape::Arrow => CursorIcon::Default,
        CursorShape::Hand => CursorIcon::Pointer,
        CursorShape::Text => CursorIcon::Text,
        CursorShape::Progress => CursorIcon::Progress,
        CursorShape::Wait => CursorIcon::Wait,
        CursorShape::Cross => CursorIcon::Crosshair,
        CursorShape::Move => CursorIcon::Move,
        CursorShape::NotAllowed => CursorIcon::NotAllowed,
        CursorShape::ResizeColumns => CursorIcon::ColResize,
        CursorShape::ResizeRows => CursorIcon::RowResize,
        CursorShape::ResizeFalling => CursorIcon::NwseResize,
        CursorShape::ResizeRising => CursorIcon::NeswResize,
        CursorShape::Help => CursorIcon::Help,
        CursorShape::Hidden => return None,
    })
}

fn physical_key(key: LibraryPhysical) -> PhysicalKey {
    PhysicalKey {
        code: match key {
            LibraryPhysical::Code(code) => key_code(code),
            LibraryPhysical::Unidentified(_) => KeyCode::Unidentified,
        },
        scancode: key.to_scancode().unwrap_or(0),
    }
}

macro_rules! map_key_codes {
    ($($name:ident),* $(,)?) => {
        fn key_code(code: LibraryCode) -> KeyCode {
            match code {
                $(LibraryCode::$name => KeyCode::$name,)*
                _ => KeyCode::Unidentified,
            }
        }
    };
}

macro_rules! map_named_keys {
    ($($name:ident),* $(,)?) => {
        fn named_key(key: LibraryNamed) -> Option<NamedKey> {
            match key {
                $(LibraryNamed::$name => Some(NamedKey::$name),)*
                _ => None,
            }
        }
    };
}

with_key_codes!(map_key_codes);
with_named_keys!(map_named_keys);

fn logical_key(key: &Key) -> LogicalKey {
    match key {
        Key::Named(key) => named_key(*key).map_or(LogicalKey::Unidentified(0), LogicalKey::Named),
        Key::Character(text) => LogicalKey::Text(TypedText::new(text.as_str())),
        Key::Dead(accent) => LogicalKey::Dead(*accent),
        Key::Unidentified(NativeKey::Windows(virtual_key)) => {
            LogicalKey::Unidentified(u32::from(*virtual_key))
        }
        Key::Unidentified(_) => LogicalKey::Unidentified(0),
    }
}

fn location(location: LibraryLocation) -> KeyLocation {
    match location {
        LibraryLocation::Standard => KeyLocation::Standard,
        LibraryLocation::Left => KeyLocation::Left,
        LibraryLocation::Right => KeyLocation::Right,
        LibraryLocation::Numpad => KeyLocation::Numpad,
    }
}

fn press_state(state: ElementState) -> PressState {
    match state {
        ElementState::Pressed => PressState::Pressed,
        ElementState::Released => PressState::Released,
    }
}

/// The text a key typed, if it typed any: not on release, not with a shortcut, and no control characters (Enter, Tab
/// and Backspace are keys).
fn typed_text(text: Option<&str>, pressed: bool, modifiers: Modifiers) -> Option<TypedText> {
    let text = text.filter(|_| pressed && modifiers.leave_text())?;
    let usable = !text.is_empty() && !text.chars().any(char::is_control);
    usable.then(|| TypedText::new(text))
}

fn modifiers(state: ModifiersState) -> Modifiers {
    Modifiers {
        shift: state.shift_key(),
        control: state.control_key(),
        alt: state.alt_key(),
        logo: state.super_key(),
    }
}

fn pointer_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Back,
        MouseButton::Forward => PointerButton::Forward,
        MouseButton::Other(number) => PointerButton::Other(number),
    }
}

/// The library divides the operating system's whole numbers by a detent; this gives them back. Its directions are
/// those in which the content moves; ours those in which the view scrolls over it.
fn wheel_units(delta: MouseScrollDelta) -> (i32, i32) {
    let units = |detents: f64| (-detents * f64::from(WHEEL_DETENT)).round() as i32;
    match delta {
        MouseScrollDelta::LineDelta(x, y) => (units(f64::from(x)), units(f64::from(y))),
        // Not sent on Windows.
        MouseScrollDelta::PixelDelta(by) => (
            units(by.x / f64::from(WHEEL_DETENT)),
            units(by.y / f64::from(WHEEL_DETENT)),
        ),
    }
}

fn is_precise(device: DeviceKind, x: i32, y: i32) -> bool {
    device == DeviceKind::Touchpad || x % WHEEL_DETENT != 0 || y % WHEEL_DETENT != 0
}

fn contact(touch: &Touch, window: &Window, core: &Core) -> ContactEvent {
    ContactEvent {
        contact: touch.id,
        phase: match touch.phase {
            TouchPhase::Started => ContactPhase::Started,
            TouchPhase::Moved => ContactPhase::Moved,
            TouchPhase::Ended => ContactPhase::Ended,
            TouchPhase::Cancelled => ContactPhase::Cancelled,
        },
        position: ClientPoint {
            x: touch.location.x,
            y: touch.location.y,
        },
        pressure: touch.force.map(|force| match force {
            Force::Normalized(pressure) => pressure,
            Force::Calibrated { .. } => force.normalized(),
        }),
        stamp: window.stamp(core),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::{NativeKeyCode, SmolStr};

    #[test]
    fn every_position_a_keyboard_reports_has_its_name_here() {
        // Every scan code, through the library's own table (not this crate's list): each position it names is named
        // here with the library's word for it, and keeps its scan code.
        let mut named = 0;
        for scancode in (0..=0xFF).chain(0xE000..=0xE0FF) {
            let library = LibraryPhysical::from_scancode(scancode);
            let LibraryPhysical::Code(code) = library else {
                continue;
            };
            let ours = physical_key(library);
            assert_ne!(ours.code, KeyCode::Unidentified, "{scancode:#x} {code:?}");
            assert_eq!(ours.code.name(), format!("{code:?}"), "{scancode:#x}");
            assert_eq!(Some(ours.scancode), LibraryPhysical::Code(code).to_scancode());
            named += 1;
        }
        assert!(named >= 140, "the sweep reads the table: {named}");
        assert_eq!(key_code(LibraryCode::F30), KeyCode::Unidentified);
    }

    #[test]
    fn every_meaning_the_windows_layouts_give_has_its_name_here() {
        // What winit 0.30.13 makes of a Windows virtual key (`keyboard_layout.rs`), written out, not taken from this
        // crate's list: a meaning missing here would arrive as an unidentified key.
        use LibraryNamed::*;
        let given = [
            Accept, Alt, AltGraph, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Attn, AudioVolumeDown,
            AudioVolumeMute, AudioVolumeUp, Backspace, BrowserBack, BrowserFavorites, BrowserForward,
            BrowserHome, BrowserRefresh, BrowserSearch, BrowserStop, CapsLock, Clear, ContextMenu,
            Control, Convert, Copy, CrSel, Delete, End, Enter, EraseEof, Escape, ExSel, Execute, F1,
            F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
            F21, F22, F23, F24, FinalMode, HangulMode, HanjaMode, Hankaku, Help, Home, Insert,
            JunjaMode, KanaMode, KanjiMode, Katakana, LaunchApplication1, LaunchApplication2,
            LaunchMail, LaunchMediaPlayer, MediaPlayPause, MediaStop, MediaTrackNext,
            MediaTrackPrevious, ModeChange, NonConvert, NumLock, PageDown, PageUp, Pause, Play, Print,
            PrintScreen, Process, Romaji, ScrollLock, Select, Shift, Space, Standby, Super, Tab,
            Zenkaku, ZoomToggle,
        ];
        for key in given {
            let ours = named_key(key).unwrap_or_else(|| panic!("{key:?} has no name here"));
            assert_eq!(ours.name(), format!("{key:?}"));
        }
        assert_eq!(named_key(TVPower), None);
    }

    #[test]
    fn no_two_names_differ_only_in_case() {
        // Names are words in settings and shortcuts, where case is not read.
        let folded = |names: Vec<&str>| {
            let count = names.len();
            let mut names: Vec<String> = names.iter().map(|name| name.to_lowercase()).collect();
            names.sort_unstable();
            names.dedup();
            (count, names.len())
        };
        let codes = KeyCode::NAMED.iter().map(|code| code.name());
        let (count, distinct) = folded(codes.chain(["Unidentified"]).collect());
        assert_eq!(count, distinct);
        let (count, distinct) = folded(NamedKey::ALL.iter().map(|key| key.name()).collect());
        assert_eq!(count, distinct);
        assert_eq!(folded(vec!["KeyA", "Keya"]), (2, 1), "the check sees a clash");
    }

    #[test]
    fn a_physical_key_carries_the_scan_code_also_without_a_name() {
        let a = physical_key(LibraryPhysical::Code(LibraryCode::KeyA));
        assert_eq!((a.code, a.scancode), (KeyCode::KeyA, 0x1E));
        let right = physical_key(LibraryPhysical::Code(LibraryCode::ControlRight));
        assert_eq!(right.scancode, 0xE01D, "an extended key");
        let odd = physical_key(LibraryPhysical::Unidentified(NativeKeyCode::Windows(0x59)));
        assert_eq!((odd.code, odd.scancode), (KeyCode::Unidentified, 0x59));
    }

    #[test]
    fn a_key_is_a_name_a_text_a_dead_key_or_a_number() {
        assert_eq!(
            logical_key(&Key::Named(LibraryNamed::Escape)),
            LogicalKey::Named(NamedKey::Escape)
        );
        let text = logical_key(&Key::Character(SmolStr::new("é")));
        assert!(matches!(&text, LogicalKey::Text(text) if text.as_str() == "é"));
        assert_eq!(
            logical_key(&Key::Dead(Some('^'))),
            LogicalKey::Dead(Some('^'))
        );
        assert_eq!(
            logical_key(&Key::Unidentified(NativeKey::Windows(0xE8))),
            LogicalKey::Unidentified(0xE8)
        );
    }

    #[test]
    fn every_input_method_event_of_the_library_is_one_change_with_its_text_and_cursor() {
        assert_eq!(ime_change(LibraryIme::Enabled), ImeChange::Started);
        assert_eq!(ime_change(LibraryIme::Disabled), ImeChange::Ended);
        assert_eq!(
            ime_change(LibraryIme::Preedit("かな".to_owned(), Some((3, 6)))),
            ImeChange::Preedit {
                text: TypedText::new("かな"),
                cursor: Some((3, 6)),
            }
        );
        assert_eq!(
            ime_change(LibraryIme::Preedit(String::new(), None)),
            ImeChange::Preedit {
                text: TypedText::new(""),
                cursor: None,
            },
            "the library's clearing preedit before a commit"
        );
        assert_eq!(
            ime_change(LibraryIme::Commit("仮名".to_owned())),
            ImeChange::Commit(TypedText::new("仮名"))
        );
    }

    #[test]
    fn what_was_typed_never_prints() {
        let secret = logical_key(&Key::Character(SmolStr::new("s")));
        assert!(!format!("{secret:?}").contains("\"s\""));
        let text = typed_text(Some("geheim"), true, Modifiers::default()).unwrap();
        assert!(!format!("{text:?}").contains("geheim"));
        assert_eq!(text.as_str(), "geheim");
    }

    #[test]
    fn only_typing_is_text() {
        let none = Modifiers::default();
        let with = |change: fn(&mut Modifiers)| {
            let mut modifiers = none;
            change(&mut modifiers);
            modifiers
        };
        let typed = |text, pressed, modifiers| {
            typed_text(text, pressed, modifiers).map(|text| text.as_str().to_owned())
        };
        assert_eq!(typed(Some("a"), true, none).as_deref(), Some("a"));
        assert_eq!(
            typed(Some("A"), true, with(|m| m.shift = true)).as_deref(),
            Some("A")
        );
        // A completed dead key, and a character outside the basic plane.
        assert_eq!(typed(Some("ê"), true, none).as_deref(), Some("ê"));
        assert_eq!(typed(Some("😀"), true, none).as_deref(), Some("😀"));
        for control in ["\r", "\t", "\u{8}", "\u{1b}", "\u{7f}", "\u{1}", ""] {
            assert_eq!(typed(Some(control), true, none), None, "{control:?}");
        }
        assert_eq!(
            typed(Some("a"), false, none),
            None,
            "a release or a focus sync types nothing"
        );
        assert_eq!(typed(None, true, none), None);
        for shortcut in [
            with(|m| m.control = true),
            with(|m| m.alt = true),
            with(|m| m.logo = true),
        ] {
            assert_eq!(typed(Some("a"), true, shortcut), None);
        }
    }

    #[test]
    fn wheel_units_are_the_operating_systems_whole_numbers() {
        // Every turn Windows can report, through the library's f32 and back.
        for raw in i16::MIN..=i16::MAX {
            let detents = f32::from(raw) / 120.0;
            let (x, y) = wheel_units(MouseScrollDelta::LineDelta(detents, detents));
            assert_eq!((x, y), (-i32::from(raw), -i32::from(raw)));
        }
    }

    #[test]
    fn a_turn_of_whole_detents_is_precise_only_from_a_touchpad() {
        assert!(!is_precise(DeviceKind::Mouse, 0, 120));
        assert!(!is_precise(DeviceKind::Unknown, -240, 0));
        assert!(
            is_precise(DeviceKind::Mouse, 0, 30),
            "a high resolution wheel"
        );
        assert!(is_precise(DeviceKind::Touchpad, 0, 120));
    }

    #[test]
    fn only_hardware_counts_as_the_operator() {
        assert_eq!(
            message_origin(IMO_HARDWARE, IMDT_TOUCHPAD),
            (Origin::OperatorDevice, DeviceKind::Touchpad)
        );
        assert_eq!(
            message_origin(IMO_INJECTED, IMDT_KEYBOARD),
            (Origin::OtherProgram, DeviceKind::Keyboard)
        );
        for origin in [0, 4, 77] {
            assert_eq!(message_origin(origin, 0).0, Origin::Unattributed);
        }
    }

    #[test]
    fn modifiers_and_buttons_are_mapped_one_to_one() {
        let all = modifiers(ModifiersState::all());
        assert!(all.shift && all.control && all.alt && all.logo);
        assert_eq!(modifiers(ModifiersState::empty()), Modifiers::default());
        assert_eq!(
            modifiers(ModifiersState::SUPER),
            Modifiers {
                logo: true,
                ..Modifiers::default()
            }
        );
        assert_eq!(pointer_button(MouseButton::Back), PointerButton::Back);
        assert_eq!(
            pointer_button(MouseButton::Other(9)),
            PointerButton::Other(9)
        );
        assert_eq!(location(LibraryLocation::Numpad), KeyLocation::Numpad);
        assert_eq!(press_state(ElementState::Released), PressState::Released);
    }
}
