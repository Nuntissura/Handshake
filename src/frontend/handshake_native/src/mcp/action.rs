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

use std::collections::{HashSet, VecDeque};

use egui::accesskit;

use crate::accessibility::UiTreeSnapshot;
use crate::mcp::argus::{ActionReceiptTracker, ArgusActionReceipt, MAIN_WINDOW_ID};

/// Default bound on the number of queued, not-yet-dispatched actions. Matches the contract's
/// `mpsc` capacity of 64: large enough for normal multi-step steering, small enough that a flood is
/// rejected promptly rather than buffering unboundedly.
pub const DEFAULT_ACTION_CAPACITY: usize = 64;

/// The maximum number of actions a single [`ActionChannel::drain_into_events`] call will emit in one
/// frame. Implements the red-team "action flood" control: even a full queue cannot push more than this
/// many actions into a single egui frame, so one frame's input is always bounded.
///
/// This is the HARD ceiling. The operator may lower the live budget below it (never above) through
/// Settings > Swarm ([`ActionChannel::set_burst_limit`]); the compiled-in ceiling still applies, so a
/// settings value can only ever TIGHTEN the flood control, never widen it.
pub const MAX_ACTIONS_PER_BURST: usize = 16;

/// The smallest admission budget the operator may configure: one swarm action per frame (fully
/// serialized concurrent-agent admission). Zero is not offered — it would wedge every agent.
pub const MIN_ACTIONS_PER_BURST: usize = 1;

/// The discrete per-frame swarm admission budgets Settings offers, ascending. Kept a small closed set
/// (rather than a free-form number) so the control is a ComboBox an out-of-process agent can enumerate
/// and drive deterministically, and so every offered value is inside `1..=MAX_ACTIONS_PER_BURST`.
pub const SWARM_ADMISSION_BUDGET_OPTIONS: [usize; 4] = [1, 4, 8, MAX_ACTIONS_PER_BURST];

/// Clamp an arbitrary (persisted or operator-supplied) admission budget into the legal
/// `MIN_ACTIONS_PER_BURST..=MAX_ACTIONS_PER_BURST` range. A foreign/garbage persisted value can
/// therefore never widen the flood control or wedge the queue.
pub fn clamp_admission_budget(value: usize) -> usize {
    value.clamp(MIN_ACTIONS_PER_BURST, MAX_ACTIONS_PER_BURST)
}

/// A model-facing UI action, addressed by a widget's stable `author_id`. This is the typed core the
/// JSON-RPC tool layer parses request params into; keeping it a closed enum (rather than a stringly
/// `op` field threaded through the dispatch) makes an invalid action impossible to represent past the
/// parse boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    /// Activate the widget (egui `Action::Click` — buttons, toggles, tabs).
    Click,
    /// Ask the target to reveal its context menu without synthesizing pointer
    /// coordinates or stealing OS focus.
    ShowContextMenu,
    /// Move keyboard focus to the widget (egui `Action::Focus`).
    Focus,
    /// Replace a text widget's value. egui 0.33 has no `SetValue` action for text inputs (see the
    /// module docs); this resolves to a Focus action plus deterministic in-app select-all, clear, and
    /// optional replacement-text events. No OS keyboard input is synthesized.
    SetValue { text: String },
    /// Scroll the widget (or its scroll container) into view (egui `Action::ScrollIntoView`).
    Scroll,
    /// Select the widget (focus is egui's selection primitive for list/tree rows).
    Select,
}

impl UiAction {
    /// The AccessKit `Action` this UI action dispatches. `SetValue` dispatches `Focus` before the
    /// channel feeds logical replacement events (see the module docs); `Select` maps to `Focus`.
    pub fn accesskit_action(&self) -> accesskit::Action {
        match self {
            UiAction::Click => accesskit::Action::Click,
            UiAction::ShowContextMenu => accesskit::Action::ShowContextMenu,
            UiAction::Focus | UiAction::SetValue { .. } | UiAction::Select => {
                accesskit::Action::Focus
            }
            UiAction::Scroll => accesskit::Action::ScrollIntoView,
        }
    }

    /// The exact replacement payload the channel applies after focus + select-all + clear.
    /// `Some("")` means clear; `None` means the action carries no text replacement.
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

#[derive(Debug, Clone)]
struct QueuedAction {
    outcome: ActionOutcome,
    action_id: Option<String>,
    author_id: String,
    action: UiAction,
    window_id: String,
}

#[derive(Debug, Clone)]
pub struct DrainedArgusAction {
    pub action_id: String,
    pub author_id: String,
    pub target_node_id: u64,
    pub action: UiAction,
}

/// Events consumed by one concrete viewport in `raw_input_hook`, plus the action ids that must only
/// be acknowledged after that viewport publishes a newer rendered snapshot.
#[derive(Debug, Default)]
pub struct DrainedActionBatch {
    pub events: Vec<egui::Event>,
    pub action_ids: Vec<String>,
    pub attributed_actions: Vec<DrainedArgusAction>,
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
    queue: VecDeque<QueuedAction>,
    capacity: usize,
    receipts: ActionReceiptTracker,
    /// Operator-configured per-frame swarm admission budget (Settings > Swarm). `None` means "use the
    /// compiled-in [`MAX_ACTIONS_PER_BURST`] ceiling" — which is exactly the pre-MT-021 behaviour, so
    /// `Default`/`new()` are unchanged for every existing caller. `Some(n)` is always clamped into
    /// `MIN_ACTIONS_PER_BURST..=MAX_ACTIONS_PER_BURST` by [`Self::set_burst_limit`], so a configured
    /// value can only TIGHTEN the flood control.
    burst_limit: Option<usize>,
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
            receipts: ActionReceiptTracker::default(),
            burst_limit: None,
        }
    }

    /// Set the live per-frame swarm admission budget (WP-1 MT-021, Settings > Swarm). The value is
    /// clamped into `MIN_ACTIONS_PER_BURST..=MAX_ACTIONS_PER_BURST`, so this can only lower the number
    /// of queued swarm actions one frame admits — never raise it above the compiled-in flood ceiling.
    ///
    /// This is the REAL runtime effect of the Settings control: the very next
    /// [`Self::drain_for_window`] / [`Self::drain_for_viewport`] admits at most this many actions, and
    /// the rest stay queued for later frames. The shell holds this channel behind the same
    /// `Arc<Mutex<_>>` the live MCP/Argus server enqueues into, so the change applies to the running
    /// transport without a rebind.
    pub fn set_burst_limit(&mut self, limit: usize) {
        self.burst_limit = Some(clamp_admission_budget(limit));
    }

    /// The live per-frame swarm admission budget actually applied by the drain path.
    pub fn burst_limit(&self) -> usize {
        self.burst_limit.unwrap_or(MAX_ACTIONS_PER_BURST)
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
        self.queue.push_back(QueuedAction {
            outcome: outcome.clone(),
            action_id: None,
            author_id: author_id.to_owned(),
            action,
            window_id: MAIN_WINDOW_ID.to_owned(),
        });
        Ok(outcome)
    }

    /// Resolve and enqueue an attributed Argus mutation. The returned receipt remains `queued`
    /// until the live viewport drains this action and publishes a newer snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn enqueue_argus(
        &mut self,
        snapshot: &UiTreeSnapshot,
        window_id: &str,
        author_id: &str,
        action: UiAction,
        connection_id: &str,
        agent_label: &str,
        before_revision: u64,
    ) -> Result<(ActionOutcome, ArgusActionReceipt), ActionError> {
        let target = resolve_target(snapshot, author_id, &action)?;
        if self.is_full() {
            return Err(ActionError::QueueFull);
        }
        let outcome = build_action_request(target, &action);
        let receipt = self.receipts.begin(
            connection_id,
            agent_label,
            window_id,
            author_id,
            &action,
            before_revision,
            snapshot,
        );
        self.queue.push_back(QueuedAction {
            outcome: outcome.clone(),
            action_id: Some(receipt.action_id.clone()),
            author_id: author_id.to_owned(),
            action,
            window_id: window_id.to_owned(),
        });
        Ok((outcome, receipt))
    }

    pub fn receipt_tracker(&self) -> ActionReceiptTracker {
        self.receipts.clone()
    }

    /// Remove one queued action by its durable receipt id. The bounded RPC timeout calls this before
    /// releasing the terminal receipt so a mutation addressed to a viewport that has already closed
    /// cannot occupy queue capacity or drive an idle-root repaint loop forever.
    pub fn discard_action(&mut self, action_id: &str) -> bool {
        let before = self.queue.len();
        self.queue
            .retain(|queued| queued.action_id.as_deref() != Some(action_id));
        self.queue.len() != before
    }

    /// Drain up to the live admission budget ([`Self::burst_limit`], default [`MAX_ACTIONS_PER_BURST`],
    /// lowerable from Settings > Swarm) pending actions into a list of `egui::Event`s the frame
    /// loop feeds to egui this frame. For each drained action: the `AccessKitActionRequest` event,
    /// followed (for `SetValue`) by the `Text` event so the focused field receives the characters.
    ///
    /// Returns the events in dispatch order. The frame loop calls this at the start of
    /// `eframe::App::update` (or a test feeds the events to the kittest harness). The burst cap bounds
    /// one frame's injected input regardless of how full the queue is (red-team: action flood).
    pub fn drain_into_events(&mut self) -> Vec<egui::Event> {
        self.drain_for_window(MAIN_WINDOW_ID).events
    }

    /// Drain only actions addressed to `window_id`; actions for other live viewports remain queued.
    pub fn drain_for_window(&mut self, window_id: &str) -> DrainedActionBatch {
        self.drain_for_window_inner(window_id, None)
    }

    /// Production viewport drain. Causal handler actions sharing one viewport +
    /// author target are serialized through the prior action's terminal receipt,
    /// because egui's boolean responses cannot attribute two same-target events
    /// from one frame to two distinct action ids.
    pub fn drain_for_viewport(
        &mut self,
        window_id: &str,
        viewport_id: egui::ViewportId,
    ) -> DrainedActionBatch {
        self.drain_for_window_inner(window_id, Some(viewport_id))
    }

    fn drain_for_window_inner(
        &mut self,
        window_id: &str,
        viewport_id: Option<egui::ViewportId>,
    ) -> DrainedActionBatch {
        let mut batch = DrainedActionBatch::default();
        let mut retained = VecDeque::with_capacity(self.queue.len());
        let mut terminal_dropped_actions = HashSet::new();
        let mut taken = 0usize;
        // WP-1 MT-021: the live, operator-configurable admission budget (defaults to the compiled-in
        // MAX_ACTIONS_PER_BURST ceiling). Read once per drain so a mid-drain settings change cannot
        // produce a torn budget within one frame.
        let burst_limit = self.burst_limit();
        while let Some(queued) = self.queue.pop_front() {
            if let Some(action_id) = queued.action_id.as_deref() {
                if self.receipts.is_terminal(action_id) {
                    // Terminal cleanup is window-independent. A closed pop-out will never drain its
                    // own window again, but any live viewport pass must still reclaim its queue slot.
                    if queued.window_id == window_id {
                        if let Some(viewport_id) = viewport_id {
                            if matches!(&queued.action, UiAction::Click | UiAction::ShowContextMenu)
                            {
                                terminal_dropped_actions
                                    .insert((viewport_id, queued.author_id.clone()));
                            }
                        }
                    }
                    continue;
                }
            }
            if queued.window_id != window_id || taken >= burst_limit {
                retained.push_back(queued);
                continue;
            }
            if let (Some(viewport_id), Some(action_id)) = (viewport_id, queued.action_id.as_deref())
            {
                let action_key = (viewport_id, queued.author_id.clone());
                if matches!(&queued.action, UiAction::Click | UiAction::ShowContextMenu)
                    && terminal_dropped_actions.contains(&action_key)
                {
                    retained.push_back(queued);
                    continue;
                }
                if matches!(&queued.action, UiAction::Click | UiAction::ShowContextMenu) {
                    match crate::mcp::argus::register_action_effect(
                        viewport_id,
                        &queued.author_id,
                        action_id,
                        self.receipts.clone(),
                    ) {
                        crate::mcp::argus::ActionEffectRegistration::Registered => {}
                        crate::mcp::argus::ActionEffectRegistration::Busy => {
                            retained.push_back(queued);
                            continue;
                        }
                        crate::mcp::argus::ActionEffectRegistration::AlreadyTerminal => {
                            // Drop terminal A without injecting it. Keep later
                            // same-target B for a subsequent drain so this batch
                            // proves that no event or reservation survived A.
                            terminal_dropped_actions.insert(action_key);
                            continue;
                        }
                    }
                }
            }
            taken += 1;
            let target_node_id = queued.outcome.request.target.0;
            batch
                .events
                .push(egui::Event::AccessKitActionRequest(queued.outcome.request));
            if let Some(text) = queued.outcome.text_payload {
                // `set_value` is replacement, not cursor insertion. Egui 0.33 exposes Focus for a
                // TextInput but no AccessKit SetValue, so drive the equivalent logical edit entirely
                // through this frame's raw egui input: focus, select all, clear, then insert the exact
                // requested value. Clearing first makes an empty value a real clear operation and
                // prevents a non-empty field from becoming `old + requested`.
                batch.events.push(egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                });
                batch.events.push(egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                });
                batch.events.push(egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
                batch.events.push(egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
                if !text.is_empty() {
                    batch.events.push(egui::Event::Text(text));
                }
            }
            if let Some(action_id) = queued.action_id {
                batch.action_ids.push(action_id.clone());
                batch.attributed_actions.push(DrainedArgusAction {
                    action_id,
                    author_id: queued.author_id,
                    target_node_id,
                    action: queued.action,
                });
            }
        }
        self.queue = retained;
        batch
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
    fn drain_emits_focus_select_all_clear_then_replacement_and_respects_burst_cap() {
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
        assert_eq!(events.len(), 6);
        assert!(matches!(events[0], egui::Event::AccessKitActionRequest(_)));
        assert!(matches!(
            events[1],
            egui::Event::Key {
                key: egui::Key::A,
                pressed: true,
                modifiers: egui::Modifiers::COMMAND,
                ..
            }
        ));
        assert!(matches!(
            events[2],
            egui::Event::Key {
                key: egui::Key::A,
                pressed: false,
                modifiers: egui::Modifiers::COMMAND,
                ..
            }
        ));
        assert!(matches!(
            events[3],
            egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
                ..
            }
        ));
        assert!(matches!(
            events[4],
            egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
                ..
            }
        ));
        assert!(matches!(&events[5], egui::Event::Text(t) if t == "abc"));
        assert_eq!(chan.pending(), 0, "drained");

        let mut clear = ActionChannel::new();
        clear
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: String::new(),
                },
            )
            .expect("enqueue clear");
        let clear_events = clear.drain_into_events();
        assert_eq!(clear_events.len(), 5, "empty replacement emits no Text");
        assert!(matches!(
            clear_events[4],
            egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: false,
                ..
            }
        ));

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

    /// WP-1 MT-021 (AC-3): the operator-configurable per-frame swarm admission budget is a REAL drain
    /// bound, not a display value. Lowering it makes the very next drain admit fewer actions and leave
    /// the rest queued; it can never be raised above the compiled-in flood ceiling, and a garbage
    /// persisted value is clamped rather than obeyed.
    #[test]
    fn configured_admission_budget_bounds_the_next_drain_and_cannot_widen_the_ceiling() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        assert_eq!(
            chan.burst_limit(),
            MAX_ACTIONS_PER_BURST,
            "default budget is the compiled-in ceiling (unchanged pre-MT-021 behaviour)"
        );

        for _ in 0..(MAX_ACTIONS_PER_BURST + 5) {
            chan.enqueue(&snap, "btn", UiAction::Click)
                .expect("enqueue click");
        }
        // Serialize concurrent-agent admission to one action per frame.
        chan.set_burst_limit(1);
        assert_eq!(chan.burst_limit(), 1);
        assert_eq!(
            chan.drain_into_events().len(),
            1,
            "the configured budget bounds the very next drain"
        );
        assert_eq!(
            chan.pending(),
            MAX_ACTIONS_PER_BURST + 4,
            "everything above the budget stays queued for later frames"
        );

        // Raising past the ceiling is clamped DOWN (a settings value can only tighten the control).
        chan.set_burst_limit(usize::MAX);
        assert_eq!(chan.burst_limit(), MAX_ACTIONS_PER_BURST);
        assert_eq!(
            chan.drain_into_events().len(),
            MAX_ACTIONS_PER_BURST,
            "clamped budget still bounded by the compiled-in flood ceiling"
        );

        // Zero would wedge every agent; it is clamped UP to the serialized minimum.
        chan.set_burst_limit(0);
        assert_eq!(chan.burst_limit(), MIN_ACTIONS_PER_BURST);
        assert_eq!(clamp_admission_budget(0), MIN_ACTIONS_PER_BURST);
        assert_eq!(clamp_admission_budget(9999), MAX_ACTIONS_PER_BURST);

        // Every offered Settings option is inside the legal band and clamps to itself.
        for option in SWARM_ADMISSION_BUDGET_OPTIONS {
            assert_eq!(clamp_admission_budget(option), option, "option {option}");
        }
    }

    #[test]
    fn viewport_drain_serializes_same_target_parallel_click_receipts() {
        let mut snap = fixture_snapshot();
        let author_id = format!("parallel-btn-{}", uuid::Uuid::now_v7());
        snap.root.children[0].author_id = Some(author_id.clone());
        let mut chan = ActionChannel::new();
        let (_, first) = chan
            .enqueue_argus(
                &snap,
                MAIN_WINDOW_ID,
                &author_id,
                UiAction::Click,
                "connection-1",
                "agent-1",
                7,
            )
            .expect("enqueue first same-target click");
        let (_, second) = chan
            .enqueue_argus(
                &snap,
                MAIN_WINDOW_ID,
                &author_id,
                UiAction::Click,
                "connection-2",
                "agent-2",
                7,
            )
            .expect("enqueue second same-target click");
        let tracker = chan.receipt_tracker();

        let first_batch = chan.drain_for_viewport(MAIN_WINDOW_ID, egui::ViewportId::ROOT);
        assert_eq!(first_batch.action_ids, vec![first.action_id.clone()]);
        assert_eq!(chan.pending(), 1, "second click stays queued");
        let ctx = egui::Context::default();
        crate::mcp::argus::acknowledge_action_effect(&ctx, &author_id);
        tracker.observe_postcondition(&first.action_id, 8, &snap);

        let second_batch = chan.drain_for_viewport(MAIN_WINDOW_ID, egui::ViewportId::ROOT);
        assert_eq!(second_batch.action_ids, vec![second.action_id.clone()]);
        assert_eq!(chan.pending(), 0);
        crate::mcp::argus::acknowledge_action_effect(&ctx, &author_id);
        tracker.observe_postcondition(&second.action_id, 9, &snap);
        assert_eq!(
            tracker
                .wait(&first.action_id, std::time::Duration::ZERO)
                .unwrap()
                .status,
            crate::mcp::argus::ActionReceiptStatus::Applied
        );
        assert_eq!(
            tracker
                .wait(&second.action_id, std::time::Duration::ZERO)
                .unwrap()
                .status,
            crate::mcp::argus::ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn terminal_before_first_drain_is_dropped_and_cannot_fence_same_target_successor() {
        let mut snap = fixture_snapshot();
        let author_id = format!("terminal-before-drain-{}", uuid::Uuid::now_v7());
        snap.root.children[0].author_id = Some(author_id.clone());
        let mut chan = ActionChannel::new();
        let (_, first) = chan
            .enqueue_argus(
                &snap,
                MAIN_WINDOW_ID,
                &author_id,
                UiAction::Click,
                "connection-1",
                "agent-1",
                7,
            )
            .expect("enqueue action A");
        let (_, second) = chan
            .enqueue_argus(
                &snap,
                MAIN_WINDOW_ID,
                &author_id,
                UiAction::Click,
                "connection-2",
                "agent-2",
                7,
            )
            .expect("enqueue action B");
        let tracker = chan.receipt_tracker();
        tracker.failed(
            &first.action_id,
            "receipt timed out before first viewport drain",
        );

        let terminal_batch = chan.drain_for_viewport(MAIN_WINDOW_ID, egui::ViewportId::ROOT);
        assert!(terminal_batch.events.is_empty());
        assert!(terminal_batch.action_ids.is_empty());
        assert_eq!(chan.pending(), 1, "only successor B remains queued");

        let successor_batch = chan.drain_for_viewport(MAIN_WINDOW_ID, egui::ViewportId::ROOT);
        assert_eq!(successor_batch.action_ids, vec![second.action_id.clone()]);
        assert_eq!(successor_batch.events.len(), 1);
        let ctx = egui::Context::default();
        crate::mcp::argus::acknowledge_action_effect(&ctx, &author_id);
        tracker.observe_postcondition(&second.action_id, 8, &snap);
        assert_eq!(
            tracker
                .wait(&second.action_id, std::time::Duration::ZERO)
                .unwrap()
                .status,
            crate::mcp::argus::ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn terminal_set_value_is_dropped_even_by_legacy_window_drain() {
        let snap = fixture_snapshot();
        let mut channel = ActionChannel::new();
        let (_, receipt) = channel
            .enqueue_argus(
                &snap,
                MAIN_WINDOW_ID,
                "field",
                UiAction::SetValue {
                    text: "must-not-arrive".to_owned(),
                },
                "connection-1",
                "agent-1",
                7,
            )
            .expect("enqueue SetValue");
        channel
            .receipt_tracker()
            .failed(&receipt.action_id, "receipt timed out before drain");

        let batch = channel.drain_for_window(MAIN_WINDOW_ID);
        assert!(batch.events.is_empty());
        assert!(batch.action_ids.is_empty());
        assert_eq!(channel.pending(), 0);
    }
}
