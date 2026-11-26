//! Input as typed events, each with its window, origin and time.

use std::fmt;

use handshake_shell_window::{
    ClientPoint, ContactPhase, DeviceKind, EventTime, InputStamp, KeyLocation, LogicalKey,
    Modifiers, Origin, PhysicalKey, PointerButton, PressState, TypedText, WindowKey,
};

use crate::ime::{ImeEvent, TextTarget};

/// One input: its window, what it was, where it came from and when.
///
/// Only this crate makes one, from an event the window loop stamped ([`crate::InputTracker::feed`]) or from a model's
/// input ([`crate::InputTracker::model_input`]), whose stamp is always a model's. Its fields are private, so no caller
/// can give a model's input another origin, or the Operator's origin to anything.
#[derive(Clone, Debug, PartialEq)]
pub struct InputEvent {
    window: WindowKey,
    input: Input,
    origin: Origin,
    at: EventTime,
    stamp: Option<InputStamp>,
    synthetic: Option<Synthetic>,
}

impl InputEvent {
    pub(crate) fn new(window: WindowKey, input: Input, from: Source) -> Self {
        Self {
            window,
            input,
            origin: from.origin,
            at: from.at,
            stamp: from.stamp,
            synthetic: None,
        }
    }

    /// An input this crate made, for a cause the operating system did not report. Carries the origin of what it ends
    /// and no stamp, so it grants nothing.
    pub(crate) fn synthesized(
        window: WindowKey,
        input: Input,
        origin: Origin,
        at: EventTime,
        cause: Synthetic,
    ) -> Self {
        Self {
            window,
            input,
            origin,
            at,
            stamp: None,
            synthetic: Some(cause),
        }
    }

    /// The window it is for.
    pub fn window(&self) -> WindowKey {
        self.window
    }

    /// What it was.
    pub fn input(&self) -> &Input {
        &self.input
    }

    /// Where it came from: the Operator's device, a model's action, another program, or not said.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// A model's input.
    pub fn is_model(&self) -> bool {
        matches!(self.origin, Origin::ModelAction(_))
    }

    /// When: the loop's time of the input, or of what made this crate synthesize it.
    pub fn at(&self) -> EventTime {
        self.at
    }

    /// The loop's stamp, for a command that answers this input (`CommandSource::from_input`). `None` for an input
    /// this crate synthesized, and for one whose Operator stamp was older than one already seen (a replay).
    pub fn stamp(&self) -> Option<&InputStamp> {
        self.stamp.as_ref()
    }

    /// Why this crate made it, when the operating system did not report it.
    pub fn synthetic(&self) -> Option<Synthetic> {
        self.synthetic
    }

    /// The text target, for an input method event.
    pub fn text_target(&self) -> Option<TextTarget> {
        match &self.input {
            Input::Ime(ime) => Some(ime.target),
            _ => None,
        }
    }
}

/// What an input was.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Input {
    /// A key.
    Key(KeyInput),
    /// Text typed: the text of a key, or of a key after a dead key. Control characters and shortcuts are not text.
    Text(TypedText),
    /// Other modifiers are held.
    Modifiers(Modifiers),
    /// The pointer moved inside the client area.
    PointerMoved(ClientPoint),
    /// The pointer came in.
    PointerEntered,
    /// The pointer left.
    PointerLeft,
    /// A pointer button went down or up.
    PointerButton {
        /// Which.
        button: PointerButton,
        /// Down or up.
        state: PressState,
        /// Where the pointer last was; `None` before its first move or for a release this crate synthesized.
        position: Option<ClientPoint>,
    },
    /// The wheel turned, or a touchpad scrolled.
    Wheel(WheelInput),
    /// A touch or pen contact.
    Contact {
        /// Tells contacts apart while they last.
        contact: u64,
        /// Where in its life.
        phase: ContactPhase,
        /// Where.
        position: ClientPoint,
        /// 0.0 to 1.0, where the device measures it.
        pressure: Option<f64>,
    },
    /// The input method's composition, for one text target.
    Ime(ImeEvent),
}

/// Where a key is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyPhase {
    /// It went down.
    Press,
    /// It is held, and the keyboard repeats it: every press after the first until its release, whether or not the
    /// operating system said so.
    Repeat,
    /// It went up, or this crate ends it (see [`InputEvent::synthetic`]).
    Release,
}

/// A key. Prints its phase only, so a log of events never shows what was typed (MT-214 W5).
#[derive(Clone, PartialEq, Eq)]
pub struct KeyInput {
    /// The key by position.
    pub physical: PhysicalKey,
    /// The key by meaning under the active layout.
    pub logical: LogicalKey,
    /// Which of several such keys.
    pub location: KeyLocation,
    /// Press, repeat or release.
    pub phase: KeyPhase,
    /// The window library reported it because the window gained or lost the focus while it was held, not because it
    /// moved. It types nothing.
    pub focus_sync: bool,
}

impl KeyInput {
    /// A dead key: it types nothing itself, and the key after it types its accent with it, as one [`Input::Text`]
    /// (the layout's `WM_DEADCHAR`). Its repeats and its release are dead too.
    pub fn is_dead(&self) -> bool {
        matches!(self.logical, LogicalKey::Dead(_))
    }
}

impl fmt::Debug for KeyInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyInput")
            .field("phase", &self.phase)
            .field("focus_sync", &self.focus_sync)
            .finish_non_exhaustive()
    }
}

/// What scrolled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WheelSource {
    /// A wheel in whole detents.
    Detents,
    /// A wheel finer than whole detents (a high resolution wheel). Scroll by it directly.
    FineWheel,
    /// A precision touchpad. A pinch arrives as a vertical turn with Ctrl held, as the operating system reports it.
    Touchpad,
}

impl WheelSource {
    /// What a turn came from, by the device the operating system named and whether it was finer than a detent.
    pub fn of(device: DeviceKind, precise: bool) -> Self {
        match (device, precise) {
            (DeviceKind::Touchpad, _) => Self::Touchpad,
            (_, true) => Self::FineWheel,
            (_, false) => Self::Detents,
        }
    }
}

/// The wheel turned, or a touchpad scrolled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelInput {
    /// Horizontal turn in 1/120 detents; positive scrolls right.
    pub x: i32,
    /// Vertical turn in 1/120 detents; positive scrolls down.
    pub y: i32,
    /// What scrolled.
    pub source: WheelSource,
    /// Where the pointer last was; `None` before its first move.
    pub position: Option<ClientPoint>,
}

/// Why this crate made an input the operating system did not report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Synthetic {
    /// The window lost the keyboard: held keys end, and so does a composition.
    FocusLost,
    /// The loop left events out: held keys and buttons end, and so does a composition (`Event::EventsLost`).
    EventsLost,
    /// The window is gone.
    WindowGone,
    /// The host moved the text focus to another target, or to none.
    FocusMoved,
    /// The focused target stopped taking composed text.
    ImeTurnedOff,
    /// A composition began while one was open.
    Replaced,
    /// The input method went on with a composition this crate had ended: it begins again at the focused target.
    Rebound,
}

impl Synthetic {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::FocusLost => "input.synthetic.focus_lost",
            Self::EventsLost => "input.synthetic.events_lost",
            Self::WindowGone => "input.synthetic.window_gone",
            Self::FocusMoved => "input.synthetic.focus_moved",
            Self::ImeTurnedOff => "input.synthetic.ime_turned_off",
            Self::Replaced => "input.synthetic.replaced",
            Self::Rebound => "input.synthetic.rebound",
        }
    }
}

/// Where and when an input came from, as this crate passes it on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Source {
    pub(crate) origin: Origin,
    pub(crate) at: EventTime,
    pub(crate) stamp: Option<InputStamp>,
}

impl Source {
    /// A stamp as it is.
    pub(crate) fn of(stamp: &InputStamp) -> Self {
        Self {
            origin: stamp.origin(),
            at: stamp.received(),
            stamp: Some(*stamp),
        }
    }
}
