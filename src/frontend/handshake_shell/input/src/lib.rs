//! Typed input of the kernel-owned shell, with its origin and time, and the input method's composition as one event
//! stream per focused text target (WP-KERNEL-012 MT-214 S3, AC214-5, AC214-6).
//!
//! The window crate (`handshake_shell_window`) turns the operating system's messages into window events with a stamp:
//! where each input came from and when. This crate turns those into [`InputEvent`]s: pointer, wheel and precision
//! touchpad, keyboard (physical and logical key, press, repeat and release, dead keys), text, modifiers, touch and
//! pen, and the input method's composition. It reads no device, injects no input and calls no operating system
//! function: it has no unsafe code and no dependency but the window crate.
//!
//! # How a host uses it
//!
//! 1. Make one [`InputTracker`] per loop. In `ShellHost::on_event`, [`InputTracker::feed`] it every event with
//!    `Control::now`, dispatch the inputs it returns, then carry out [`InputTracker::take_requests`] with
//!    `Control::command` and `ImeRequest::command` (a host `CommandSource`).
//! 2. When a text target gets or loses a window's text focus, [`InputTracker::focus_text`]; when its caret moves or its
//!    content scrolls, [`InputTracker::move_caret`] or [`InputTracker::scroll_text`]. The window is asked to let the
//!    input method compose while a target that takes composed text has the focus, and where to put the candidates.
//! 3. A model's input goes through [`InputTracker::model_input`].
//!
//! # Who an input is from
//!
//! [`InputEvent::origin`] is the stamp's: the Operator's device, a model's action, another program, or not said. An
//! [`InputEvent`] is made only here, and its fields are private, so no caller can give one another origin.
//! [`InputTracker::model_input`] stamps with `InputStamp::model_action`, the only stamp a caller can make: a model's
//! input is never the Operator's. An Operator stamp older than one the tracker already saw is a copy and is passed on
//! as not attributed, without the stamp. What this crate synthesizes ([`InputEvent::synthetic`]) carries the origin of
//! what it ends and no stamp, so it grants nothing.
//!
//! # Keys
//!
//! A key's first press is [`KeyPhase::Press`]; every further press until its release is [`KeyPhase::Repeat`],
//! whether or not the operating system marked it. A dead key types nothing ([`KeyInput::is_dead`]); the text of the
//! key after it, accent included, arrives as one [`Input::Text`] stamped as that key. When the window loses the
//! keyboard, the keys still held are released; when the loop lost events or the window goes, held pointer buttons are
//! released too (`Event::EventsLost`: "held keys and buttons must be read as released").
//!
//! # The input method
//!
//! The window reports what the operating system's input method did (`ImeChange`, from winit's IMM32 events: a
//! composition's start, its preedit, its commits, its end). The tracker routes it to the text target that has the
//! window's text focus as [`ImeEvent`]s ([`Composition::Enabled`], `Preedit` with its cursor range, `Commit`,
//! `Cancel`, `Disabled`), numbered per composition. Composed text reaches only the target where its composition
//! began. A composition that showed preedit text ends in exactly one commit or one cancel, then disabled: also when
//! the target loses the focus (the window loses the keyboard, the host moves the text focus, the target stops taking
//! composed text, events are lost, the window goes), which ends it at once, with a cancel when its preedit was not
//! committed, and nothing of it reaches that target afterwards. What the input method goes on with begins again as a
//! new composition at the target that has the focus then ([`Synthetic::Rebound`]), or is left out when none takes
//! composed text. The candidate area follows the caret on scroll and on a scale change ([`CaretPlace::candidate_area`]). The composition itself is Kumihimo's (MT-198 KG-03): no preedit text, cursor or anchor
//! is kept here. A model may not move the text focus, turn composition off, or type into a window while the
//! Operator composes there.
//!
//! # Failure modes
//!
//! | Seen | Meaning | What to do |
//! | --- | --- | --- |
//! | [`FocusRefusal::UnknownWindow`], [`ModelRefusal::UnknownWindow`] | the tracker was not fed the window's `Created`, or the window is gone | feed every event from `LoopStarted` on |
//! | [`FocusRefusal::NotFocused`] | a caret update of a target that lost the focus | drop it |
//! | [`FocusRefusal::OperatorComposing`], [`ModelRefusal::OperatorComposing`] | a model reached for the Operator's composition | wait for its end, or ask the Operator |
//! | [`InputReport::stale_stamps`] above 0 | an Operator stamp was fed again | find who fed it |
//! | [`InputReport::ime_dropped`] above 0 | the input method composed with no target that takes it | expected after the focus left a composition |
//! | [`InputReport::ime_bad_cursors`] above 0 | the input method's cursor was outside its text | none; the preedit shows without a cursor |

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod event;
mod ime;
mod tracker;

pub use event::{Input, InputEvent, KeyInput, KeyPhase, Synthetic, WheelInput, WheelSource};
pub use ime::{
    CaretPlace, Composition, CompositionId, FocusRefusal, ImeEvent, ImeRequest, LogicalRect,
    TextFocus, TextTarget,
};
pub use tracker::{
    FocusBy, InputReport, InputTracker, ModelInput, ModelRefusal, MAX_HELD_BUTTONS, MAX_HELD_KEYS,
};

/// The window crate's input types this crate's events carry.
pub use handshake_shell_window::{
    Attribution, ClientPoint, ClientRect, ContactPhase, DeviceKind, EventTime, InputStamp, KeyCode,
    KeyLocation, LogicalKey, Modifiers, NamedKey, Origin, PhysicalKey, PointerButton, PressState,
    ScaleFactor, TypedText, WindowKey,
};
