//! The window loop's events as typed input, the keys and buttons held, and a model's input.

use std::collections::BTreeMap;

use handshake_shell_window::{
    Attribution, ClientPoint, Event, EventTime, InputStamp, KeyEvent, KeyLocation, LogicalKey,
    Origin, PhysicalKey, PointerButton, PressState, TypedText, WindowEvent, WindowKey,
};

use crate::event::{
    Input, InputEvent, KeyInput, KeyPhase, Source, Synthetic, WheelInput, WheelSource,
};
use crate::ime::{CaretPlace, FocusRefusal, ImeRequest, ImeRouter, TextFocus, TextTarget};

/// The most keys held at once that are kept per window; more are passed on and not ended by this crate.
pub const MAX_HELD_KEYS: usize = 128;
/// The most pointer buttons held at once that are kept per window.
pub const MAX_HELD_BUTTONS: usize = 16;

/// Who moves a window's text focus.
#[derive(Clone, Copy, Debug)]
pub enum FocusBy<'a> {
    /// The input that moved it, for example a click or Tab: a model's input counts as the model's.
    Input(&'a InputEvent),
    /// A model, through a steering route.
    Model(Attribution),
    /// The shell itself, for example when a dialog closes.
    Host,
}

impl FocusBy<'_> {
    fn is_model(self) -> bool {
        match self {
            Self::Input(input) => input.is_model(),
            Self::Model(_) => true,
            Self::Host => false,
        }
    }
}

/// A model's input, for the window it acts in. Its origin is the model's, always.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ModelInput {
    /// A key.
    Key {
        /// By position.
        physical: PhysicalKey,
        /// By meaning.
        logical: LogicalKey,
        /// Which of several such keys.
        location: KeyLocation,
        /// Down or up.
        state: PressState,
        /// A repeat of a key the model holds.
        repeat: bool,
    },
    /// Text.
    Text(TypedText),
    /// The pointer moved.
    PointerMoved(ClientPoint),
    /// A pointer button.
    PointerButton {
        /// Which.
        button: PointerButton,
        /// Down or up.
        state: PressState,
        /// Where.
        position: Option<ClientPoint>,
    },
    /// The wheel, in 1/120 detents.
    Wheel {
        /// Horizontal; positive scrolls right.
        x: i32,
        /// Vertical; positive scrolls down.
        y: i32,
        /// Where.
        position: Option<ClientPoint>,
    },
}

/// Why a model's input was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ModelRefusal {
    /// The tracker saw no `Created` of the window, or it is gone.
    UnknownWindow,
    /// The Operator composes in the window: a model's keys and text would land in the Operator's composition.
    OperatorComposing,
}

impl ModelRefusal {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownWindow => "input.model.unknown_window",
            Self::OperatorComposing => "input.model.operator_composing",
        }
    }
}

/// What the tracker counted. Numbers only, for the host's diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputReport {
    /// Inputs with an Operator stamp older than one already seen: a replayed stamp. Passed on as not attributed and
    /// without the stamp.
    pub stale_stamps: u64,
    /// Keys or buttons held beyond [`MAX_HELD_KEYS`] or [`MAX_HELD_BUTTONS`], not kept.
    pub holds_not_kept: u64,
    /// Input method changes with no composition and no target that takes composed text.
    pub ime_dropped: u64,
    /// Input method cursors outside their preedit text, passed on as none.
    pub ime_bad_cursors: u64,
    /// Empty commits left out.
    pub ime_empty_commits: u64,
    /// Compositions this crate ended for a focus change, a loss or a window that went.
    pub compositions_settled: u64,
    /// Focus changes and inputs of a model refused while the Operator composed.
    pub model_refused: u64,
}

/// A key held, and whose press it was.
#[derive(Clone, Debug)]
struct HeldKey {
    physical: PhysicalKey,
    logical: LogicalKey,
    location: KeyLocation,
    origin: Origin,
}

#[derive(Debug, Default)]
struct Held {
    keys: Vec<HeldKey>,
    buttons: Vec<(PointerButton, Origin)>,
}

/// The input of one event loop, as typed events. Feed it every event of the loop from `LoopStarted` on, on the loop's
/// thread, and carry out its [`InputTracker::take_requests`] after each.
#[derive(Debug, Default)]
pub struct InputTracker {
    held: BTreeMap<WindowKey, Held>,
    ime: ImeRouter,
    /// The newest Operator stamp seen.
    newest_operator: Option<EventTime>,
    report: InputReport,
}

impl InputTracker {
    /// A tracker that saw nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// The inputs an event of the loop makes, in order. `now` is the loop's time (`Control::now`), for what this
    /// crate synthesizes. Events that are no input give none, but the tracker reads windows, scales and focus from them.
    pub fn feed(&mut self, now: EventTime, event: &Event) -> Vec<InputEvent> {
        let mut out = Vec::new();
        match event {
            Event::Window { window, event } => self.window_event(*window, event, now, &mut out),
            Event::EventsLost { .. } => {
                let windows: Vec<WindowKey> = self.held.keys().copied().collect();
                for window in windows {
                    self.release(window, Synthetic::EventsLost, now, true, &mut out);
                }
                self.ime.events_lost(now, &mut out);
            }
            _ => {}
        }
        out
    }

    /// Move a window's text focus to a target, or to none; or change what its target takes and where its caret is.
    /// The events it makes end the composition of the target that loses the focus.
    ///
    /// # Errors
    ///
    /// [`FocusRefusal::UnknownWindow`]; [`FocusRefusal::OperatorComposing`] when a model would take the focus or the
    /// composed text away from the Operator's composition.
    pub fn focus_text(
        &mut self,
        window: WindowKey,
        focus: Option<TextFocus>,
        by: FocusBy<'_>,
        now: EventTime,
    ) -> Result<Vec<InputEvent>, FocusRefusal> {
        let mut out = Vec::new();
        self.ime
            .focus(window, focus, by.is_model(), now, &mut out)?;
        Ok(out)
    }

    /// The focused target's caret moved: the candidate window follows.
    ///
    /// # Errors
    ///
    /// [`FocusRefusal::UnknownWindow`], or [`FocusRefusal::NotFocused`] for a target without the focus, whose update
    /// is out of date.
    pub fn move_caret(
        &mut self,
        window: WindowKey,
        target: TextTarget,
        caret: CaretPlace,
    ) -> Result<(), FocusRefusal> {
        self.ime.place(window, target, |place| *place = caret)
    }

    /// The focused target's content scrolled: the candidate window follows the caret.
    ///
    /// # Errors
    ///
    /// As [`InputTracker::move_caret`].
    pub fn scroll_text(
        &mut self,
        window: WindowKey,
        target: TextTarget,
        x: f64,
        y: f64,
    ) -> Result<(), FocusRefusal> {
        self.ime.place(window, target, |place| {
            place.scroll_x = x;
            place.scroll_y = y;
        })
    }

    /// What the host has to ask the windows for, with [`ImeRequest::command`]: the newest of each kind per window.
    pub fn take_requests(&mut self) -> Vec<(WindowKey, ImeRequest)> {
        self.ime.take_requests()
    }

    /// A model's input, stamped as the model's.
    ///
    /// # Errors
    ///
    /// [`ModelRefusal`].
    pub fn model_input(
        &mut self,
        window: WindowKey,
        attribution: Attribution,
        at: EventTime,
        input: ModelInput,
    ) -> Result<InputEvent, ModelRefusal> {
        if !self.ime.knows(window) {
            return Err(ModelRefusal::UnknownWindow);
        }
        let typing = matches!(input, ModelInput::Key { .. } | ModelInput::Text(_));
        if typing && self.ime.operator_composing(window) {
            self.report.model_refused = self.report.model_refused.saturating_add(1);
            return Err(ModelRefusal::OperatorComposing);
        }
        let stamp = InputStamp::model_action(attribution, at);
        let input = match input {
            ModelInput::Key {
                physical,
                logical,
                location,
                state,
                repeat,
            } => Input::Key(KeyInput {
                physical,
                logical,
                location,
                phase: match (state, repeat) {
                    (PressState::Pressed, false) => KeyPhase::Press,
                    (PressState::Pressed, true) => KeyPhase::Repeat,
                    (PressState::Released, _) => KeyPhase::Release,
                },
                focus_sync: false,
            }),
            ModelInput::Text(text) => Input::Text(text),
            ModelInput::PointerMoved(position) => Input::PointerMoved(position),
            ModelInput::PointerButton {
                button,
                state,
                position,
            } => Input::PointerButton {
                button,
                state,
                position,
            },
            ModelInput::Wheel { x, y, position } => Input::Wheel(WheelInput {
                x,
                y,
                source: WheelSource::of(
                    stamp.device(),
                    x % handshake_shell_window::WHEEL_DETENT != 0
                        || y % handshake_shell_window::WHEEL_DETENT != 0,
                ),
                position,
            }),
        };
        Ok(InputEvent::new(window, input, Source::of(&stamp)))
    }

    /// What the tracker counted so far.
    pub fn report(&self) -> InputReport {
        let ime = self.ime.counts;
        InputReport {
            ime_dropped: ime.dropped,
            ime_bad_cursors: ime.bad_cursors,
            ime_empty_commits: ime.empty_commits,
            compositions_settled: ime.settled,
            model_refused: self.report.model_refused.saturating_add(ime.refused),
            ..self.report
        }
    }

    fn window_event(
        &mut self,
        window: WindowKey,
        event: &WindowEvent,
        now: EventTime,
        out: &mut Vec<InputEvent>,
    ) {
        match event {
            WindowEvent::Created { state, .. } => {
                self.ime.created(window, state.scale, state.focused)
            }
            WindowEvent::Resized { scale, .. } => self.ime.scale(window, *scale),
            WindowEvent::ScaleChanged { to, .. } => self.ime.scale(window, *to),
            WindowEvent::FocusChanged { focused } => {
                if !focused {
                    self.release(window, Synthetic::FocusLost, now, false, out);
                }
                self.ime.keyboard(window, *focused, now, out);
            }
            WindowEvent::Destroyed => {
                self.release(window, Synthetic::WindowGone, now, true, out);
                self.held.remove(&window);
                self.ime.gone(window, now, out);
            }
            WindowEvent::Key(key) => self.key(window, key, out),
            WindowEvent::Text(text) => {
                let from = self.source(&text.stamp);
                out.push(InputEvent::new(
                    window,
                    Input::Text(text.text.clone()),
                    from,
                ));
            }
            WindowEvent::ModifiersChanged { modifiers, stamp } => {
                let from = self.source(stamp);
                out.push(InputEvent::new(window, Input::Modifiers(*modifiers), from));
            }
            WindowEvent::PointerMoved(moved) => {
                let from = self.source(&moved.stamp);
                out.push(InputEvent::new(
                    window,
                    Input::PointerMoved(moved.position),
                    from,
                ));
            }
            WindowEvent::PointerEntered(stamp) => {
                let from = self.source(stamp);
                out.push(InputEvent::new(window, Input::PointerEntered, from));
            }
            WindowEvent::PointerLeft(stamp) => {
                let from = self.source(stamp);
                out.push(InputEvent::new(window, Input::PointerLeft, from));
            }
            WindowEvent::PointerButton(button) => {
                let from = self.source(&button.stamp);
                self.hold_button(window, button.button, button.state, from.origin);
                let input = Input::PointerButton {
                    button: button.button,
                    state: button.state,
                    position: button.position,
                };
                out.push(InputEvent::new(window, input, from));
            }
            WindowEvent::Wheel(wheel) => {
                let from = self.source(&wheel.stamp);
                let input = Input::Wheel(WheelInput {
                    x: wheel.x,
                    y: wheel.y,
                    source: WheelSource::of(wheel.stamp.device(), wheel.precise),
                    position: wheel.position,
                });
                out.push(InputEvent::new(window, input, from));
            }
            WindowEvent::Contact(contact) => {
                let from = self.source(&contact.stamp);
                let input = Input::Contact {
                    contact: contact.contact,
                    phase: contact.phase,
                    position: contact.position,
                    pressure: contact.pressure,
                };
                out.push(InputEvent::new(window, input, from));
            }
            WindowEvent::Ime(ime) => {
                let from = self.source(&ime.stamp);
                self.ime.change(window, &ime.change, from, out);
            }
            _ => {}
        }
    }

    /// Where a stamped input came from, as passed on.
    fn source(&mut self, stamp: &InputStamp) -> Source {
        let origin = self.classify(stamp.origin(), stamp.received());
        Source {
            origin,
            at: stamp.received(),
            stamp: (origin == stamp.origin()).then_some(*stamp),
        }
    }

    /// The loop stamps inputs in the order it hands them out, so an Operator stamp older than one already seen is a
    /// copy: it is passed on as not attributed.
    fn classify(&mut self, origin: Origin, at: EventTime) -> Origin {
        if origin != Origin::OperatorDevice {
            return origin;
        }
        match self.newest_operator {
            Some(newest) if at < newest => {
                self.report.stale_stamps = self.report.stale_stamps.saturating_add(1);
                Origin::Unattributed
            }
            _ => {
                self.newest_operator = Some(at);
                origin
            }
        }
    }

    fn key(&mut self, window: WindowKey, key: &KeyEvent, out: &mut Vec<InputEvent>) {
        let from = self.source(&key.stamp);
        let held = self.held.entry(window).or_default();
        let at = held
            .keys
            .iter()
            .position(|held| held.physical == key.physical);
        let phase = match key.state {
            PressState::Pressed => {
                let phase = if at.is_some() || key.repeat {
                    KeyPhase::Repeat
                } else {
                    KeyPhase::Press
                };
                if at.is_none() {
                    if held.keys.len() < MAX_HELD_KEYS {
                        held.keys.push(HeldKey {
                            physical: key.physical,
                            logical: key.logical.clone(),
                            location: key.location,
                            origin: from.origin,
                        });
                    } else {
                        self.report.holds_not_kept = self.report.holds_not_kept.saturating_add(1);
                    }
                }
                phase
            }
            PressState::Released => {
                if let Some(at) = at {
                    held.keys.remove(at);
                }
                KeyPhase::Release
            }
        };
        let input = Input::Key(KeyInput {
            physical: key.physical,
            logical: key.logical.clone(),
            location: key.location,
            phase,
            focus_sync: key.focus_sync,
        });
        out.push(InputEvent::new(window, input, from));
    }

    fn hold_button(
        &mut self,
        window: WindowKey,
        button: PointerButton,
        state: PressState,
        origin: Origin,
    ) {
        let held = self.held.entry(window).or_default();
        let at = held.buttons.iter().position(|(held, _)| *held == button);
        match (state, at) {
            (PressState::Pressed, None) if held.buttons.len() < MAX_HELD_BUTTONS => {
                held.buttons.push((button, origin))
            }
            (PressState::Pressed, None) => {
                self.report.holds_not_kept = self.report.holds_not_kept.saturating_add(1);
            }
            (PressState::Released, Some(at)) => {
                held.buttons.remove(at);
            }
            _ => {}
        }
    }

    /// End what the window holds: its keys, and with `buttons` its pointer buttons. Each release carries the origin of
    /// its press.
    fn release(
        &mut self,
        window: WindowKey,
        cause: Synthetic,
        at: EventTime,
        buttons: bool,
        out: &mut Vec<InputEvent>,
    ) {
        let Some(held) = self.held.get_mut(&window) else {
            return;
        };
        for key in held.keys.drain(..) {
            let input = Input::Key(KeyInput {
                physical: key.physical,
                logical: key.logical,
                location: key.location,
                phase: KeyPhase::Release,
                focus_sync: false,
            });
            out.push(InputEvent::synthesized(
                window, input, key.origin, at, cause,
            ));
        }
        if buttons {
            for (button, origin) in held.buttons.drain(..) {
                let input = Input::PointerButton {
                    button,
                    state: PressState::Released,
                    position: None,
                };
                out.push(InputEvent::synthesized(window, input, origin, at, cause));
            }
        }
    }
}

#[cfg(test)]
#[path = "tracker_tests.rs"]
mod tests;
