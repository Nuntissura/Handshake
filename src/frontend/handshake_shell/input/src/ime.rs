//! The input method's composition as one event stream per focused text target (AC214-6).
//!
//! The window reports what the operating system's input method did ([`ImeChange`], from winit's IMM32 events). This
//! routes it to the text target that has the window's text focus, numbers each composition, and ends every
//! composition once: a composition that showed preedit text ends in exactly one [`Composition::Commit`] or one
//! [`Composition::Cancel`], then [`Composition::Disabled`]. When the target loses the focus (the window loses the
//! keyboard, the host moves the text focus, the target stops taking composed text, events are lost or the window goes)
//! an open composition ends at once, with a cancel if its preedit was not committed, and nothing of it reaches that
//! target afterwards. What the input method goes on with begins again, as a new composition, at the target that has
//! the focus then (Synthetic::Rebound), or is left out when none takes composed text.
//!
//! What is composed is Kumihimo's (MT-198 KG-03): this keeps no preedit text, cursor or anchor, only which
//! composition is open for which target, whose it is, and whether its preedit was not committed yet.

use std::collections::BTreeMap;
use std::ops::Range;

use handshake_shell_window::{
    ClientRect, EventTime, ImeChange, Origin, ScaleFactor, TypedText, WindowCommand, WindowKey,
};

use crate::event::{Input, InputEvent, Source, Synthetic};

/// The host's number for a text target of a window: an editor, a field. Only compared here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextTarget(pub u64);

/// One composition, numbered by the tracker. Numbers are never used again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CompositionId(u64);

impl CompositionId {
    /// The number, for diagnostics.
    pub fn number(self) -> u64 {
        self.0
    }
}

/// What happened to a composition.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Composition {
    /// It began. Preedit and commits follow at the caret.
    Enabled,
    /// The text being composed; empty clears it. The cursor, or the selected clause, as a byte range of the text.
    /// Shown over the document, never put into it.
    Preedit {
        /// The text.
        text: TypedText,
        /// The cursor or clause, on character boundaries of `text`; `None` hides it.
        cursor: Option<Range<usize>>,
    },
    /// Insert this text once at the composition's anchor. Never empty.
    Commit(TypedText),
    /// Drop the preedit text; nothing is inserted.
    Cancel,
    /// It ended. No event of it follows.
    Disabled,
}

/// An event of one composition, for one text target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImeEvent {
    /// The target it is for.
    pub target: TextTarget,
    /// Which composition.
    pub composition: CompositionId,
    /// What happened.
    pub change: Composition,
}

/// A rectangle in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width, at least 0.
    pub width: f64,
    /// Height, at least 0.
    pub height: f64,
}

impl LogicalRect {
    fn is_usable(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|value| value.is_finite())
            && self.width >= 0.0
            && self.height >= 0.0
    }
}

/// Where a focused text target shows its caret, for the input method's candidate window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaretPlace {
    /// The target's visible area, from the client area's top left.
    pub viewport: LogicalRect,
    /// The caret, or the composed text's line, in the target's content.
    pub caret: LogicalRect,
    /// How far the content is scrolled right: content x is shown at `viewport.x + x - scroll_x`.
    pub scroll_x: f64,
    /// How far the content is scrolled down: content y is shown at `viewport.y + y - scroll_y`.
    pub scroll_y: f64,
}

impl CaretPlace {
    /// Where the candidate window keeps clear of, in physical client pixels at `scale`: the caret where it is shown,
    /// kept inside the viewport when it is scrolled out of it, so the candidates stay beside the target. Edges are
    /// rounded outward and the rectangle is at least one pixel each way. `None` when the place is not usable.
    pub fn candidate_area(&self, scale: ScaleFactor) -> Option<ClientRect> {
        let (view, caret) = (self.viewport, self.caret);
        if !view.is_usable()
            || !caret.is_usable()
            || !self.scroll_x.is_finite()
            || !self.scroll_y.is_finite()
        {
            return None;
        }
        let width = caret.width.min(view.width);
        let height = caret.height.min(view.height);
        // A non-negative span added to the near edge, so the far bound is never below it.
        let x = (view.x + caret.x - self.scroll_x).clamp(view.x, view.x + (view.width - width));
        let y = (view.y + caret.y - self.scroll_y).clamp(view.y, view.y + (view.height - height));
        let limit = |value: f64| value.clamp(f64::from(i32::MIN), f64::from(i32::MAX));
        let left = limit(scale.to_physical(x).floor());
        let top = limit(scale.to_physical(y).floor());
        let right = limit(scale.to_physical(x + width).ceil()).max(left + 1.0);
        let bottom = limit(scale.to_physical(y + height).ceil()).max(top + 1.0);
        Some(ClientRect {
            x: left as i32,
            y: top as i32,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        })
    }
}

/// The text target that has a window's text focus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextFocus {
    /// Which.
    pub target: TextTarget,
    /// It takes composed text. Off for a password field or a target that takes keys only.
    pub accepts_ime: bool,
    /// Where its caret is.
    pub caret: CaretPlace,
}

/// What the host asks the window for, so the operating system's input method follows the text focus. Carry it out
/// with [`ImeRequest::command`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImeRequest {
    /// Let the input method compose for the window, or not.
    Allow(bool),
    /// Where its candidate window keeps clear of.
    Area(ClientRect),
}

impl ImeRequest {
    /// The window command that carries it out. It moves no window and no focus.
    pub fn command(self) -> WindowCommand {
        match self {
            Self::Allow(allowed) => WindowCommand::AllowIme(allowed),
            Self::Area(area) => WindowCommand::SetImeArea(area),
        }
    }
}

/// Why a text focus change or caret update did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FocusRefusal {
    /// The tracker saw no `Created` of the window, or it is gone.
    UnknownWindow,
    /// The target does not have the window's text focus.
    NotFocused,
    /// The Operator composes in the window, and a model may not take that away (GUI-DESKTOP-041).
    OperatorComposing,
}

impl FocusRefusal {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownWindow => "input.unknown_window",
            Self::NotFocused => "input.not_focused",
            Self::OperatorComposing => "input.operator_composing",
        }
    }
}

/// Counts of the input method stream, for diagnostics. Numbers only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ImeCounts {
    pub(crate) dropped: u64,
    pub(crate) bad_cursors: u64,
    pub(crate) empty_commits: u64,
    pub(crate) settled: u64,
    pub(crate) refused: u64,
}

/// An open composition: which, for which target, whose, and whether preedit text was shown that no commit took. Not
/// its text: the composition itself is Kumihimo's.
#[derive(Clone, Copy, Debug)]
struct Open {
    id: CompositionId,
    target: TextTarget,
    origin: Origin,
    pending: bool,
}

/// The text focus of one window and its open composition.
#[derive(Debug)]
struct WindowText {
    scale: ScaleFactor,
    keyboard: bool,
    focus: Option<TextFocus>,
    open: Option<Open>,
    /// What the window was last asked for.
    allowed: bool,
    area: Option<ClientRect>,
    /// Requests the host did not take yet: the newest of each kind only.
    want_allow: Option<bool>,
    want_area: Option<ClientRect>,
}

impl WindowText {
    fn new(scale: ScaleFactor, keyboard: bool) -> Self {
        Self {
            scale,
            keyboard,
            focus: None,
            open: None,
            allowed: false,
            area: None,
            want_allow: None,
            want_area: None,
        }
    }

    /// Ask for what the focus needs now. `again`: the area even when it did not change, for a composition that begins.
    fn refresh(&mut self, again: bool) {
        let allow = self.focus.is_some_and(|focus| focus.accepts_ime);
        if allow != self.allowed {
            self.allowed = allow;
            self.want_allow = Some(allow);
        }
        let area = self
            .focus
            .filter(|focus| focus.accepts_ime)
            .and_then(|focus| focus.caret.candidate_area(self.scale));
        match area {
            Some(area) if again || self.area != Some(area) => {
                self.area = Some(area);
                self.want_area = Some(area);
            }
            Some(_) => {}
            // Asked again once composing is allowed again.
            None => self.area = None,
        }
    }
}

/// Emit `change` of `open` into `out`.
fn emit(
    out: &mut Vec<InputEvent>,
    window: WindowKey,
    open: &Open,
    change: Composition,
    from: Source,
    synthetic: Option<Synthetic>,
) {
    let event = ImeEvent {
        target: open.target,
        composition: open.id,
        change,
    };
    out.push(match synthetic {
        None => InputEvent::new(window, Input::Ime(event), from),
        Some(cause) => {
            InputEvent::synthesized(window, Input::Ime(event), from.origin, from.at, cause)
        }
    });
}

/// The routing of the input method's changes, per window.
#[derive(Debug, Default)]
pub(crate) struct ImeRouter {
    windows: BTreeMap<WindowKey, WindowText>,
    next: u64,
    pub(crate) counts: ImeCounts,
}

impl ImeRouter {
    pub(crate) fn knows(&self, window: WindowKey) -> bool {
        self.windows.contains_key(&window)
    }

    /// The text target with the window's text focus.
    pub(crate) fn focused(&self, window: WindowKey) -> Option<TextTarget> {
        self.windows
            .get(&window)
            .and_then(|text| text.focus)
            .map(|focus| focus.target)
    }

    /// An Operator's composition is open in the window.
    pub(crate) fn operator_composing(&self, window: WindowKey) -> bool {
        self.windows
            .get(&window)
            .and_then(|text| text.open)
            .is_some_and(|open| open.origin == Origin::OperatorDevice)
    }

    pub(crate) fn created(&mut self, window: WindowKey, scale: ScaleFactor, keyboard: bool) {
        self.windows
            .insert(window, WindowText::new(scale, keyboard));
    }

    pub(crate) fn scale(&mut self, window: WindowKey, scale: ScaleFactor) {
        if let Some(text) = self.windows.get_mut(&window) {
            if text.scale != scale {
                text.scale = scale;
                text.refresh(false);
            }
        }
    }

    pub(crate) fn keyboard(
        &mut self,
        window: WindowKey,
        has: bool,
        at: EventTime,
        out: &mut Vec<InputEvent>,
    ) {
        if let Some(text) = self.windows.get_mut(&window) {
            text.keyboard = has;
        }
        if !has {
            self.settle(window, Synthetic::FocusLost, at, out);
        }
    }

    pub(crate) fn gone(&mut self, window: WindowKey, at: EventTime, out: &mut Vec<InputEvent>) {
        self.settle(window, Synthetic::WindowGone, at, out);
        self.windows.remove(&window);
    }

    pub(crate) fn events_lost(&mut self, at: EventTime, out: &mut Vec<InputEvent>) {
        let windows: Vec<WindowKey> = self.windows.keys().copied().collect();
        for window in windows {
            self.settle(window, Synthetic::EventsLost, at, out);
        }
    }

    /// End the window's open composition now: a cancel when its preedit was not committed, then disabled.
    fn settle(
        &mut self,
        window: WindowKey,
        cause: Synthetic,
        at: EventTime,
        out: &mut Vec<InputEvent>,
    ) {
        let Some(open) = self
            .windows
            .get_mut(&window)
            .and_then(|text| text.open.take())
        else {
            return;
        };
        let from = Source {
            origin: open.origin,
            at,
            stamp: None,
        };
        if open.pending {
            emit(out, window, &open, Composition::Cancel, from, Some(cause));
        }
        emit(out, window, &open, Composition::Disabled, from, Some(cause));
        self.counts.settled = self.counts.settled.saturating_add(1);
    }

    /// Open a composition at the focused target, if it takes composed text and the window has the keyboard.
    fn begin(&mut self, window: WindowKey, origin: Origin) -> Option<Open> {
        let text = self.windows.get_mut(&window)?;
        let focus = text
            .focus
            .filter(|focus| focus.accepts_ime && text.keyboard)?;
        self.next = self.next.saturating_add(1);
        let open = Open {
            id: CompositionId(self.next),
            target: focus.target,
            origin,
            pending: false,
        };
        text.open = Some(open);
        text.refresh(true);
        Some(open)
    }

    /// The open composition, or one begun again at the focused target (Synthetic::Rebound).
    fn open_or_rebind(
        &mut self,
        window: WindowKey,
        from: Source,
        out: &mut Vec<InputEvent>,
    ) -> Option<Open> {
        if let Some(open) = self.windows.get(&window)?.open {
            return Some(open);
        }
        let open = self.begin(window, from.origin)?;
        emit(
            out,
            window,
            &open,
            Composition::Enabled,
            from,
            Some(Synthetic::Rebound),
        );
        Some(open)
    }

    fn set_open(&mut self, window: WindowKey, open: Option<Open>) {
        if let Some(text) = self.windows.get_mut(&window) {
            text.open = open;
        }
    }

    fn dropped(&mut self) {
        self.counts.dropped = self.counts.dropped.saturating_add(1);
    }

    /// What the operating system's input method did.
    pub(crate) fn change(
        &mut self,
        window: WindowKey,
        change: &ImeChange,
        from: Source,
        out: &mut Vec<InputEvent>,
    ) {
        if !self.knows(window) {
            self.dropped();
            return;
        }
        match change {
            ImeChange::Started => {
                self.settle(window, Synthetic::Replaced, from.at, out);
                match self.begin(window, from.origin) {
                    Some(open) => emit(out, window, &open, Composition::Enabled, from, None),
                    None => self.dropped(),
                }
            }
            ImeChange::Preedit { text, cursor } => {
                let Some(mut open) = self.open_or_rebind(window, from, out) else {
                    self.dropped();
                    return;
                };
                let cursor = match cursor_range(text.as_str(), *cursor) {
                    Ok(cursor) => cursor,
                    Err(()) => {
                        self.counts.bad_cursors = self.counts.bad_cursors.saturating_add(1);
                        None
                    }
                };
                if !text.as_str().is_empty() {
                    open.pending = true;
                    self.set_open(window, Some(open));
                }
                let change = Composition::Preedit {
                    text: text.clone(),
                    cursor,
                };
                emit(out, window, &open, change, from, None);
            }
            ImeChange::Commit(text) => {
                // The window library commits the empty result string when a composition ends without one.
                if text.as_str().is_empty() {
                    self.counts.empty_commits = self.counts.empty_commits.saturating_add(1);
                    return;
                }
                let Some(mut open) = self.open_or_rebind(window, from, out) else {
                    self.dropped();
                    return;
                };
                open.pending = false;
                self.set_open(window, Some(open));
                emit(
                    out,
                    window,
                    &open,
                    Composition::Commit(text.clone()),
                    from,
                    None,
                );
            }
            ImeChange::Ended => {
                // Nothing open: the input method ends one this crate already ended, or none.
                let Some(open) = self
                    .windows
                    .get_mut(&window)
                    .and_then(|text| text.open.take())
                else {
                    return;
                };
                if open.pending {
                    // A composition cleared without a result string was cancelled (WM_IME_COMPOSITION, Remarks).
                    emit(out, window, &open, Composition::Cancel, from, None);
                }
                emit(out, window, &open, Composition::Disabled, from, None);
            }
            _ => self.dropped(),
        }
    }

    /// The host moved the window's text focus, or changed what its target takes. `model`: a model asked.
    pub(crate) fn focus(
        &mut self,
        window: WindowKey,
        focus: Option<TextFocus>,
        model: bool,
        at: EventTime,
        out: &mut Vec<InputEvent>,
    ) -> Result<(), FocusRefusal> {
        let text = self
            .windows
            .get(&window)
            .ok_or(FocusRefusal::UnknownWindow)?;
        let target = |focus: Option<TextFocus>| focus.map(|focus| focus.target);
        let moves = target(text.focus) != target(focus);
        let turns_off =
            !moves && text.open.is_some() && !focus.is_some_and(|focus| focus.accepts_ime);
        if model && (moves || turns_off) && self.operator_composing(window) {
            self.counts.refused = self.counts.refused.saturating_add(1);
            return Err(FocusRefusal::OperatorComposing);
        }
        if moves {
            self.settle(window, Synthetic::FocusMoved, at, out);
        } else if turns_off {
            self.settle(window, Synthetic::ImeTurnedOff, at, out);
        }
        if let Some(text) = self.windows.get_mut(&window) {
            text.focus = focus;
            text.refresh(false);
        }
        Ok(())
    }

    /// The focused target's caret moved, or its content scrolled.
    pub(crate) fn place(
        &mut self,
        window: WindowKey,
        target: TextTarget,
        update: impl FnOnce(&mut CaretPlace),
    ) -> Result<(), FocusRefusal> {
        let text = self
            .windows
            .get_mut(&window)
            .ok_or(FocusRefusal::UnknownWindow)?;
        let focus = text
            .focus
            .as_mut()
            .filter(|focus| focus.target == target)
            .ok_or(FocusRefusal::NotFocused)?;
        update(&mut focus.caret);
        text.refresh(false);
        Ok(())
    }

    /// What the host has to ask the windows for, allowing before placing.
    pub(crate) fn take_requests(&mut self) -> Vec<(WindowKey, ImeRequest)> {
        let mut requests = Vec::new();
        for (window, text) in &mut self.windows {
            if let Some(allowed) = text.want_allow.take() {
                requests.push((*window, ImeRequest::Allow(allowed)));
            }
            if let Some(area) = text.want_area.take() {
                requests.push((*window, ImeRequest::Area(area)));
            }
        }
        requests
    }
}

/// The input method's cursor as a range of its text: `Ok(None)` when it has none, `Err` when it is not on character
/// boundaries of the text.
fn cursor_range(text: &str, cursor: Option<(usize, usize)>) -> Result<Option<Range<usize>>, ()> {
    let Some((start, end)) = cursor else {
        return Ok(None);
    };
    let fits = start <= end
        && end <= text.len()
        && text.is_char_boundary(start)
        && text.is_char_boundary(end);
    if fits {
        Ok(Some(start..end))
    } else {
        Err(())
    }
}

#[cfg(test)]
#[path = "ime_tests.rs"]
mod tests;
