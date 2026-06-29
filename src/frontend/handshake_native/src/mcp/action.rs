//! The action channel: resolve a model's `author_id`-addressed request to a stable AccessKit
//! `NodeId` and build the `accesskit::ActionRequest` (plus any text payload) the egui frame loop
//! dispatches.
//!
//! ## Resolution path (author_id -> NodeId)
//!
//! A model addresses a widget by its stable kebab-case `author_id` (the MT-025 convention). The
//! mapping from `author_id` to the live AccessKit `NodeId` already exists in the MT-026 snapshot
//! ([`UiTreeSnapshot`]), which every node — including its `node_id` and `author_id` — is projected
//! into. [`resolve_target`] looks the target up in a snapshot taken from the current frame's live
//! tree, so the channel never needs a second, drift-prone id map: the SAME tree the model READ is the
//! tree it STEERS. A request for an unknown `author_id`, or for a disabled widget, is rejected with a
//! typed [`ActionError`] rather than silently dropped (red-team: never steer a control the model
//! cannot see / must not touch).
//!
//! ## Why a bounded in-process queue with burst limiting
//!
//! [`ActionChannel`] is a bounded FIFO (capacity [`DEFAULT_ACTION_CAPACITY`]) of pending
//! `accesskit::ActionRequest`s the egui frame loop drains each frame. Bounding it implements the
//! contract's back-pressure control (queue full -> typed `ActionError::QueueFull`, mapped by the tool
//! layer to JSON-RPC `-32002`), and the per-drain burst cap [`MAX_ACTIONS_PER_BURST`] implements the
//! red-team "action flood" control (a buggy/adversarial caller cannot saturate one frame). This is the
//! in-process analog of the contract's `tokio::sync::mpsc` bounded channel; a future transport MT can
//! feed this same queue from a socket/pipe without changing the steering semantics.

use std::collections::VecDeque;

use egui::accesskit;

use crate::accessibility::UiTreeSnapshot;

/// Default bound on the number of queued, not-yet-dispatched actions. Matches the contract's
/// `mpsc` capacity of 64: large enough for normal multi-step steering, small enough that a flood is
/// rejected promptly rather than buffering unboundedly.
pub const DEFAULT_ACTION_CAPACITY: usize = 64;

/// The maximum number of actions a single [`ActionChannel::drain_into_events`] call will emit in one
/// frame. Implements the red-team "action flood" control: even a full queue cannot push more than this
/// many actions into a single egui frame, so one frame's input is always bounded.
pub const MAX_ACTIONS_PER_BURST: usize = 16;

/// A model-facing UI action, addressed by a widget's stable `author_id`. This is the typed core the
/// JSON-RPC tool layer parses request params into; keeping it a closed enum (rather than a stringly
/// `op` field threaded through the dispatch) makes an invalid action impossible to represent past the
/// parse boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    /// Activate the widget (egui `Action::Click` — buttons, toggles, tabs).
    Click,
    /// Move keyboard focus to the widget (egui `Action::Focus`).
    Focus,
    /// Set a text widget's value. egui 0.33 has no `SetValue` action for text inputs (see the module
    /// docs); this is accepted only for `TextInput` nodes and resolves to a Focus action plus
    /// select-all and `egui::Event::Text` events after the field is focused. The `text` is carried
    /// here so the caller has a single typed action to dispatch.
    SetValue { text: String },
    /// Scroll the widget (or its scroll container) into view (egui `Action::ScrollIntoView`).
    Scroll,
    /// Select the widget (focus is egui's selection primitive for list/tree rows).
    Select,
}

impl UiAction {
    /// The AccessKit `Action` this UI action dispatches. `SetValue` dispatches `Focus` (then the
    /// caller feeds text — see the module docs); `Select` maps to `Focus` (egui's row-selection
    /// primitive).
    pub fn accesskit_action(&self) -> accesskit::Action {
        match self {
            UiAction::Click => accesskit::Action::Click,
            UiAction::Focus | UiAction::SetValue { .. } | UiAction::Select => {
                accesskit::Action::Focus
            }
            UiAction::Scroll => accesskit::Action::ScrollIntoView,
        }
    }

    /// The text payload to feed as `egui::Event::Text` AFTER the action's request is dispatched, for
    /// actions that carry one (`SetValue`). `None` for actions with no text payload.
    pub fn text_payload(&self) -> Option<&str> {
        match self {
            UiAction::SetValue { text } => Some(text.as_str()),
            _ => None,
        }
    }
}

/// A typed failure from the action channel. Each variant maps to a specific JSON-RPC error in the
/// tool layer, so a model gets an actionable reason rather than a generic failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// No live node carries the requested `author_id` (the model addressed a widget that is not on
    /// screen this frame).
    UnknownTarget { author_id: String },
    /// The target exists but is disabled; steering a disabled control is rejected (red-team: never
    /// drive a control the model must not touch).
    DisabledTarget { author_id: String },
    /// The target exists but does not support the requested action (e.g. `Click` on a static label).
    UnsupportedAction { author_id: String, action: String },
    /// The bounded queue is full; the caller should retry after the frame loop drains it (back-pressure).
    QueueFull,
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionError::UnknownTarget { author_id } => {
                write!(f, "no live widget with author_id '{author_id}'")
            }
            ActionError::DisabledTarget { author_id } => {
                write!(f, "widget '{author_id}' is disabled and cannot be steered")
            }
            ActionError::UnsupportedAction { author_id, action } => {
                write!(f, "widget '{author_id}' does not support action '{action}'")
            }
            ActionError::QueueFull => write!(f, "action queue full"),
        }
    }
}

impl std::error::Error for ActionError {}

/// The result of resolving + enqueuing an action: the dispatched `ActionRequest` and any text payload
/// the frame loop must feed after it. Returned so a caller (and the live test) can assert exactly what
/// was dispatched.
#[derive(Debug, Clone)]
pub struct ActionOutcome {
    /// The AccessKit request enqueued for the frame loop.
    pub request: accesskit::ActionRequest,
    /// Text to feed as `egui::Event::Text` after the request (Some only for `SetValue`).
    pub text_payload: Option<String>,
}

/// Look up the live `NodeId` for a stable `author_id` in a current-frame snapshot, validating the
/// widget is present, enabled, and supports the requested action.
///
/// Returns the resolved `NodeId` on success, or the specific [`ActionError`] explaining why the
/// target cannot be steered. Resolution reads the SAME snapshot the model used to choose the target,
/// so there is no second id map to drift.
pub fn resolve_target(
    snapshot: &UiTreeSnapshot,
    author_id: &str,
    action: &UiAction,
) -> Result<accesskit::NodeId, ActionError> {
    let node = snapshot
        .find_by_author_id(author_id)
        .ok_or_else(|| ActionError::UnknownTarget {
            author_id: author_id.to_owned(),
        })?;

    if node.disabled {
        return Err(ActionError::DisabledTarget {
            author_id: author_id.to_owned(),
        });
    }

    if matches!(action, UiAction::SetValue { .. }) && node.role != "TextInput" {
        return Err(ActionError::UnsupportedAction {
            author_id: author_id.to_owned(),
            action: "SetValue".to_owned(),
        });
    }

    // The action must be supported by the live node. `SetValue`/`Select` dispatch `Focus`, so we check
    // for the egui-real action the snapshot reports (a TextInput surfaces `Focus`, MT-026-proven).
    let needed = format!("{:?}", action.accesskit_action());
    if !node.actions.iter().any(|a| a == &needed) {
        return Err(ActionError::UnsupportedAction {
            author_id: author_id.to_owned(),
            action: needed,
        });
    }

    Ok(accesskit::NodeId(node.node_id))
}

/// Build the `accesskit::ActionRequest` (plus any text payload) for a resolved target + action. The
/// request targets the STABLE `NodeId`, so it survives frame re-layout and process restarts — exactly
/// what out-of-process steering needs.
pub fn build_action_request(target: accesskit::NodeId, action: &UiAction) -> ActionOutcome {
    ActionOutcome {
        request: accesskit::ActionRequest {
            action: action.accesskit_action(),
            target,
            data: None,
        },
        text_payload: action.text_payload().map(|t| t.to_owned()),
    }
}

/// A bounded, in-process FIFO of pending AccessKit action requests the egui frame loop drains each
/// frame. This is the in-process analog of the contract's bounded `tokio::sync::mpsc` channel: the
/// MCP tool layer pushes resolved actions in; the `eframe::App::update` loop drains them out and feeds
/// them to egui. Bounding + per-drain burst limiting implement the back-pressure and flood controls.
#[derive(Debug, Default)]
pub struct ActionChannel {
    queue: VecDeque<ActionOutcome>,
    capacity: usize,
}

impl ActionChannel {
    /// A channel with the default capacity ([`DEFAULT_ACTION_CAPACITY`]).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_ACTION_CAPACITY)
    }

    /// A channel with an explicit capacity (used by tests to force the queue-full path deterministically).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    /// Number of pending (not-yet-drained) actions.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// True when the queue is at capacity (the next [`Self::enqueue`] would be rejected).
    pub fn is_full(&self) -> bool {
        self.queue.len() >= self.capacity
    }

    /// Resolve + enqueue an action addressed by `author_id` against the given current-frame snapshot.
    ///
    /// Returns the enqueued [`ActionOutcome`] (the dispatched request + any text payload) so the
    /// caller can report exactly what was queued, or an [`ActionError`] when the target cannot be
    /// resolved or the queue is full. Resolution happens BEFORE the capacity check so an unknown /
    /// disabled / unsupported target is reported as such even when the queue is also full (the more
    /// actionable error wins).
    pub fn enqueue(
        &mut self,
        snapshot: &UiTreeSnapshot,
        author_id: &str,
        action: UiAction,
    ) -> Result<ActionOutcome, ActionError> {
        let target = resolve_target(snapshot, author_id, &action)?;
        if self.is_full() {
            return Err(ActionError::QueueFull);
        }
        let outcome = build_action_request(target, &action);
        self.queue.push_back(outcome.clone());
        Ok(outcome)
    }

    /// Drain up to [`MAX_ACTIONS_PER_BURST`] pending actions into a list of `egui::Event`s the frame
    /// loop feeds to egui this frame. For each drained action: the `AccessKitActionRequest` event,
    /// followed (for `SetValue`) by a select-all key event and the `Text` event so the focused field
    /// replaces its existing contents.
    ///
    /// Returns the events in dispatch order. The frame loop calls this at the start of
    /// `eframe::App::update` (or a test feeds the events to the kittest harness). The burst cap bounds
    /// one frame's injected input regardless of how full the queue is (red-team: action flood).
    pub fn drain_into_events(&mut self) -> Vec<egui::Event> {
        let mut events = Vec::new();
        let take = self.queue.len().min(MAX_ACTIONS_PER_BURST);
        for _ in 0..take {
            let Some(outcome) = self.queue.pop_front() else {
                break;
            };
            events.push(egui::Event::AccessKitActionRequest(outcome.request));
            if let Some(text) = outcome.text_payload {
                events.push(select_all_key_event());
                events.push(egui::Event::Text(text));
            }
        }
        events
    }
}

fn select_all_key_event() -> egui::Event {
    egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{UiNodeBounds, UiTreeNode, UiTreeSnapshot};

    /// Build a tiny snapshot with a clickable button, a focusable text input, and a disabled button —
    /// enough to exercise every resolution branch on a controlled input (no real shell render needed).
    fn fixture_snapshot() -> UiTreeSnapshot {
        let button = UiTreeNode {
            id: "btn".to_owned(),
            author_id: Some("btn".to_owned()),
            node_id: 10,
            role: "Button".to_owned(),
            label: Some("Go".to_owned()),
            value: None,
            disabled: false,
            actions: vec!["Click".to_owned(), "Focus".to_owned()],
            bounds: Some(UiNodeBounds {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            }),
            children: Vec::new(),
        };
        let input = UiTreeNode {
            id: "field".to_owned(),
            author_id: Some("field".to_owned()),
            node_id: 11,
            role: "TextInput".to_owned(),
            label: None,
            value: Some(String::new()),
            disabled: false,
            actions: vec!["Click".to_owned(), "Focus".to_owned()],
            bounds: None,
            children: Vec::new(),
        };
        let disabled = UiTreeNode {
            id: "off".to_owned(),
            author_id: Some("off".to_owned()),
            node_id: 12,
            role: "Button".to_owned(),
            label: None,
            value: None,
            disabled: true,
            actions: vec!["Click".to_owned()],
            bounds: None,
            children: Vec::new(),
        };
        let root = UiTreeNode {
            id: "node:1".to_owned(),
            author_id: None,
            node_id: 1,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children: vec![button, input, disabled],
        };
        UiTreeSnapshot {
            root,
            captured_at_utc: "0.000000000Z".to_owned(),
            widget_count: 4,
        }
    }

    #[test]
    fn resolves_click_to_stable_node_id() {
        let snap = fixture_snapshot();
        let id = resolve_target(&snap, "btn", &UiAction::Click).expect("btn resolves");
        assert_eq!(id, accesskit::NodeId(10));
    }

    #[test]
    fn unknown_target_is_rejected() {
        let snap = fixture_snapshot();
        let err = resolve_target(&snap, "nope", &UiAction::Click).unwrap_err();
        assert_eq!(
            err,
            ActionError::UnknownTarget {
                author_id: "nope".to_owned()
            }
        );
    }

    #[test]
    fn disabled_target_is_rejected() {
        let snap = fixture_snapshot();
        let err = resolve_target(&snap, "off", &UiAction::Click).unwrap_err();
        assert_eq!(
            err,
            ActionError::DisabledTarget {
                author_id: "off".to_owned()
            }
        );
    }

    #[test]
    fn unsupported_action_is_rejected() {
        let snap = fixture_snapshot();
        // The text input surfaces Focus but not Click... actually it has Click here; use a click on a
        // node that only supports Focus: synthesize by asking Click on a node lacking it.
        let mut snap = snap;
        // Strip Click from the input so a Click request is unsupported.
        if let Some(input) = snap.root.children.get_mut(1) {
            input.actions = vec!["Focus".to_owned()];
        }
        let err = resolve_target(&snap, "field", &UiAction::Click).unwrap_err();
        assert_eq!(
            err,
            ActionError::UnsupportedAction {
                author_id: "field".to_owned(),
                action: "Click".to_owned()
            }
        );
    }

    #[test]
    fn set_value_resolves_to_focus_and_carries_text() {
        let snap = fixture_snapshot();
        let action = UiAction::SetValue {
            text: "hello swarm".to_owned(),
        };
        // The input supports Focus, so SetValue (which dispatches Focus) resolves.
        let id = resolve_target(&snap, "field", &action).expect("field resolves via Focus");
        assert_eq!(id, accesskit::NodeId(11));
        let outcome = build_action_request(id, &action);
        assert_eq!(outcome.request.action, accesskit::Action::Focus);
        assert_eq!(outcome.text_payload.as_deref(), Some("hello swarm"));
    }

    #[test]
    fn set_value_rejects_non_text_targets_even_when_focusable() {
        let snap = fixture_snapshot();
        let err = resolve_target(
            &snap,
            "btn",
            &UiAction::SetValue {
                text: "not a text input".to_owned(),
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            ActionError::UnsupportedAction {
                author_id: "btn".to_owned(),
                action: "SetValue".to_owned()
            }
        );
    }

    #[test]
    fn queue_is_bounded_and_reports_full() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::with_capacity(2);
        assert!(chan.enqueue(&snap, "btn", UiAction::Click).is_ok());
        assert!(chan.enqueue(&snap, "btn", UiAction::Click).is_ok());
        assert!(chan.is_full());
        let err = chan.enqueue(&snap, "btn", UiAction::Click).unwrap_err();
        assert_eq!(err, ActionError::QueueFull);
        assert_eq!(chan.pending(), 2);
    }

    #[test]
    fn drain_emits_focus_then_select_all_then_text_and_respects_burst_cap() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        chan.enqueue(
            &snap,
            "field",
            UiAction::SetValue {
                text: "abc".to_owned(),
            },
        )
        .expect("enqueue set_value");
        let events = chan.drain_into_events();
        // Focus the field, select all existing content, then type the replacement text.
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], egui::Event::AccessKitActionRequest(_)));
        assert!(
            matches!(
                &events[1],
                egui::Event::Key {
                    key: egui::Key::A,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command && modifiers.ctrl
            ),
            "set_value must select existing text before typing replacement text"
        );
        assert!(matches!(&events[2], egui::Event::Text(t) if t == "abc"));
        assert_eq!(chan.pending(), 0, "drained");

        // Burst cap: enqueue more than MAX_ACTIONS_PER_BURST clicks; one drain takes at most the cap.
        let mut chan = ActionChannel::new();
        for _ in 0..(MAX_ACTIONS_PER_BURST + 5) {
            chan.enqueue(&snap, "btn", UiAction::Click)
                .expect("enqueue click");
        }
        let drained = chan.drain_into_events();
        assert_eq!(
            drained.len(),
            MAX_ACTIONS_PER_BURST,
            "one drain bounded by burst cap"
        );
        assert_eq!(
            chan.pending(),
            5,
            "remainder stays queued for the next frame"
        );
    }
}
