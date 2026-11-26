//! Input as typed data. Plain values: nothing here reads a device or injects input.

use std::fmt;
use std::path::PathBuf;

use crate::geometry::{ClientPoint, PhysicalPosition};
use crate::keys::{KeyCode, NamedKey};
use crate::origin::InputStamp;

/// The modifiers held. AltGr is not among them: it selects text and arrives in the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    /// Either Shift key.
    pub shift: bool,
    /// Either Ctrl key.
    pub control: bool,
    /// Either Alt key.
    pub alt: bool,
    /// Either Windows key.
    pub logo: bool,
}

impl Modifiers {
    /// No key that turns typing into a command is held.
    pub fn leave_text(self) -> bool {
        !(self.control || self.alt || self.logo)
    }
}

/// Text the Operator typed, or a model sent. Prints as its length only, so a log of events never holds what was typed.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypedText(String);

impl TypedText {
    /// The text, kept out of logs. Grants nothing: who typed it is said by the stamp that comes with it.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for TypedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypedText({} bytes)", self.0.len())
    }
}

/// A key by its position on the keyboard. Prints without the key.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalKey {
    /// Its name, where it has one.
    pub code: KeyCode,
    /// The keyboard's own number for it, with 0xE000 set for an extended key. 0 when unknown.
    pub scancode: u32,
}

impl fmt::Debug for PhysicalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PhysicalKey(..)")
    }
}

/// A key by its meaning under the active keyboard layout. Prints its kind only, without the key.
#[derive(Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LogicalKey {
    /// A key with a name instead of text.
    Named(NamedKey),
    /// The text this key gives with the modifiers held, for matching shortcuts. What was typed arrives as
    /// [`TextEvent`].
    Text(TypedText),
    /// A dead key, with its accent where the layout has one. The text follows with the next key.
    Dead(Option<char>),
    /// A key the layout gives no meaning, with the platform's virtual key code.
    Unidentified(u32),
}

impl fmt::Debug for LogicalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Named(_) => "Named(..)",
            Self::Text(_) => "Text(..)",
            Self::Dead(_) => "Dead(..)",
            Self::Unidentified(_) => "Unidentified(..)",
        })
    }
}

/// Which of several keys with the same meaning it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyLocation {
    /// The only or main one.
    Standard,
    /// The left one.
    Left,
    /// The right one.
    Right,
    /// On the number pad.
    Numpad,
}

/// Down or up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PressState {
    /// Went down.
    Pressed,
    /// Went up.
    Released,
}

/// A key went down or up. Prints without the key, its meaning and its place, so a log of events never shows what was
/// typed (MT-214 W5).
#[derive(Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key by position.
    pub physical: PhysicalKey,
    /// The key by meaning.
    pub logical: LogicalKey,
    /// Which of several such keys.
    pub location: KeyLocation,
    /// Down or up.
    pub state: PressState,
    /// The keyboard is repeating a held key.
    pub repeat: bool,
    /// Reported because the window gained or lost the focus while the key was held, not because it moved. Never
    /// followed by text.
    pub focus_sync: bool,
    /// Where and when from.
    pub stamp: InputStamp,
}

impl fmt::Debug for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyEvent")
            .field("state", &self.state)
            .field("repeat", &self.repeat)
            .field("focus_sync", &self.focus_sync)
            .field("stamp", &self.stamp)
            .finish_non_exhaustive()
    }
}

/// Text was typed. Follows the key that completed it. Control characters and shortcuts are not text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEvent {
    /// The text.
    pub text: TypedText,
    /// Where and when from.
    pub stamp: InputStamp,
}

/// What the operating system's input method did to the window's composition (IMM32, through the window library; MT-214
/// S3). The window keeps none of it; `handshake_shell_input` routes it to the focused text target.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImeChange {
    /// A composition began.
    Started,
    /// The text being composed. Empty text clears it.
    Preedit {
        /// The text.
        text: TypedText,
        /// The input method's cursor, or its selected clause, as byte offsets into `text`; `None` hides the cursor.
        cursor: Option<(usize, usize)>,
    },
    /// Text the composition produced, to be inserted. May be empty.
    Commit(TypedText),
    /// The composition ended.
    Ended,
    /// The window asked the input method to complete its composition ([`crate::WindowCommand::CompleteIme`], or the
    /// window's keyboard going while a composition was open, before [`crate::WindowEvent::FocusChanged`] says so;
    /// MT-214 S3-Q1). What the input method gave for it at once came before this. Not a change of the input method's
    /// own; its stamp is not attributed.
    Completed,
}

/// An input method change, and where and when it came from. The operating system says nothing of where the input
/// method's own messages come from: they take the origin of a key only while that key is dispatched, and are not
/// attributed otherwise; another thread's are refused (MT-214 E6-F4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImeInput {
    /// What changed.
    pub change: ImeChange,
    /// Where and when from.
    pub stamp: InputStamp,
}

/// The pointer moved inside the client area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerMoved {
    /// Where to.
    pub position: ClientPoint,
    /// Where and when from.
    pub stamp: InputStamp,
}

/// A button of a pointing device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PointerButton {
    /// Left, unless the Operator swapped the buttons.
    Primary,
    /// Right, unless the Operator swapped the buttons.
    Secondary,
    /// The wheel.
    Middle,
    /// The back button.
    Back,
    /// The forward button.
    Forward,
    /// Another one, by number.
    Other(u16),
}

/// A button went down or up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerButtonEvent {
    /// Which.
    pub button: PointerButton,
    /// Down or up.
    pub state: PressState,
    /// Where the pointer last was; `None` before its first move.
    pub position: Option<ClientPoint>,
    /// Where and when from.
    pub stamp: InputStamp,
}

/// Wheel units of one full detent.
pub const WHEEL_DETENT: i32 = 120;

/// The wheel turned, or a touchpad scrolled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelEvent {
    /// Horizontal turn in 1/120 detents; positive scrolls right. Whole numbers, so sums do not drift.
    pub x: i32,
    /// Vertical turn in 1/120 detents; positive scrolls down.
    pub y: i32,
    /// Finer than whole detents: a precision touchpad or a high resolution wheel. Scroll by it directly.
    pub precise: bool,
    /// Where the pointer last was; `None` before its first move.
    pub position: Option<ClientPoint>,
    /// Where and when from. [`crate::DeviceKind::Touchpad`] for a precision touchpad. A pinch arrives as a vertical
    /// turn with Ctrl held, as the operating system reports it.
    pub stamp: InputStamp,
}

/// Where a touch or pen contact is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContactPhase {
    /// Touched.
    Started,
    /// Moved.
    Moved,
    /// Lifted.
    Ended,
    /// Taken away by the operating system; undo what it started.
    Cancelled,
}

/// A touch or pen contact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactEvent {
    /// Tells contacts apart while they last.
    pub contact: u64,
    /// Where in its life.
    pub phase: ContactPhase,
    /// Where.
    pub position: ClientPoint,
    /// 0.0 to 1.0, where the device measures it.
    pub pressure: Option<f64>,
    /// Where and when from.
    pub stamp: InputStamp,
}

/// The most paths one drop carries.
pub const MAX_DROPPED_FILES: u32 = 4096;

/// Files dropped on the window. Prints without their paths.
#[derive(Clone, PartialEq)]
pub struct FilesDropped {
    /// Their paths, in the order the operating system gave.
    pub paths: Vec<PathBuf>,
    /// How many more there were than [`MAX_DROPPED_FILES`].
    pub left_out: u32,
    /// Where in the client area, in physical pixels. `None` when dropped on the frame.
    pub position: Option<PhysicalPosition>,
    /// Where and when from.
    pub stamp: InputStamp,
}

impl fmt::Debug for FilesDropped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilesDropped")
            .field("paths", &self.paths.len())
            .field("left_out", &self.left_out)
            .field("position", &self.position)
            .field("stamp", &self.stamp)
            .finish()
    }
}

/// How far one detent of the wheel scrolls, as the Operator set it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WheelSettings {
    /// Lines per vertical detent. `None` means one page.
    pub lines: Option<u32>,
    /// Characters per horizontal detent.
    pub characters: u32,
}

/// What a pointer looks like over the client area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CursorShape {
    /// An arrow.
    #[default]
    Arrow,
    /// A hand, over something to press.
    Hand,
    /// A text caret.
    Text,
    /// Busy, and still usable.
    Progress,
    /// Busy.
    Wait,
    /// A cross for precise work.
    Cross,
    /// Moving something.
    Move,
    /// Not allowed here.
    NotAllowed,
    /// A vertical divider.
    ResizeColumns,
    /// A horizontal divider.
    ResizeRows,
    /// Top left or bottom right corner.
    ResizeFalling,
    /// Top right or bottom left corner.
    ResizeRising,
    /// Question mark.
    Help,
    /// None.
    Hidden,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::WindowEvent;
    use crate::origin::{DeviceKind, EventTime, Origin};

    #[test]
    fn a_key_event_prints_no_key_code_meaning_or_text() {
        let stamp = InputStamp {
            received: EventTime { nanos: 7 },
            os_millis: None,
            origin: Origin::OperatorDevice,
            device: DeviceKind::Keyboard,
            modifiers: Modifiers::default(),
            serial: 9,
        };
        let meanings = [
            LogicalKey::Text(TypedText::new("q")),
            LogicalKey::Dead(Some('^')),
            LogicalKey::Named(NamedKey::Escape),
            LogicalKey::Unidentified(0xE8),
        ];
        for logical in meanings {
            let key = KeyEvent {
                physical: PhysicalKey {
                    code: KeyCode::KeyQ,
                    scancode: 0x10,
                },
                logical,
                location: KeyLocation::Numpad,
                state: PressState::Pressed,
                repeat: false,
                focus_sync: false,
                stamp,
            };
            let event = WindowEvent::Key(key.clone());
            for printed in [format!("{key:?}"), format!("{event:?}"), format!("{key:#?}")] {
                for secret in ["KeyQ", "16", "0x10", "\"q\"", "'q'", "^", "Escape", "232", "Numpad"] {
                    assert!(!printed.contains(secret), "{secret} in {printed}");
                }
                assert!(printed.contains("Pressed"), "the rest still prints: {printed}");
            }
        }
        let position = PhysicalKey {
            code: KeyCode::KeyQ,
            scancode: 0x10,
        };
        assert_eq!(format!("{position:?}"), "PhysicalKey(..)");
    }

    #[test]
    fn an_input_method_change_prints_no_composed_text() {
        let stamp = InputStamp {
            received: EventTime { nanos: 7 },
            os_millis: None,
            origin: Origin::OperatorDevice,
            device: DeviceKind::Keyboard,
            modifiers: Modifiers::default(),
            serial: 9,
        };
        let changes = [
            ImeChange::Preedit {
                text: TypedText::new("geheim"),
                cursor: Some((0, 6)),
            },
            ImeChange::Commit(TypedText::new("geheim")),
        ];
        for change in changes {
            let event = WindowEvent::Ime(ImeInput { change, stamp });
            let printed = format!("{event:?}");
            assert!(!printed.contains("geheim"), "{printed}");
            assert!(printed.contains("6 bytes"), "the length still prints: {printed}");
        }
    }
}
