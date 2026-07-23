//! The action channel: resolve a model's `author_id`-addressed request to a stable AccessKit
//! `NodeId` and build the target-specific `accesskit::ActionRequest` the egui frame loop
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
use std::time::{Duration, Instant};

use egui::accesskit;
use serde::Serialize;

use crate::accessibility::UiTreeSnapshot;

/// Default bound on the number of queued, not-yet-dispatched actions. Matches the contract's
/// `mpsc` capacity of 64: large enough for normal multi-step steering, small enough that a flood is
/// rejected promptly rather than buffering unboundedly.
pub const DEFAULT_ACTION_CAPACITY: usize = 64;

/// The maximum number of actions a single [`ActionChannel::drain_into_events`] call will emit in one
/// frame. Implements the red-team "action flood" control: even a full queue cannot push more than this
/// many actions into a single egui frame, so one frame's input is always bounded.
pub const MAX_ACTIONS_PER_BURST: usize = 16;

/// Maximum ownership interval for a queued/dispatched target mutation. A lost frame acknowledgement
/// must not leave that target permanently busy.
const ACTION_LEASE_TIMEOUT: Duration = Duration::from_secs(5);

/// A model-facing UI action, addressed by a widget's stable `author_id`. This is the typed core the
/// JSON-RPC tool layer parses request params into; keeping it a closed enum (rather than a stringly
/// `op` field threaded through the dispatch) makes an invalid action impossible to represent past the
/// parse boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    /// Activate the widget (egui `Action::Click` — buttons, toggles, tabs).
    Click,
    /// Activate the widget with a JSON payload carried as `ActionData::Value`. This is how
    /// parameterized KnowledgeActionRegistry controls such as `collection.sort` and
    /// `collection.kanban-move` are driven through the same `click_widget` transport.
    ClickWithPayload { payload: String },
    /// Move keyboard focus to the widget (egui `Action::Focus`).
    Focus,
    /// Replace the whole widget value through one target-specific AccessKit `SetValue` request.
    /// String-valued widgets consume `ActionData::Value`; numeric widgets consume
    /// `ActionData::NumericValue`. No global keyboard/text events are injected.
    SetValue { text: String },
    /// Set a value through a native AccessKit SetValue request on surfaces that consume it directly.
    NativeSetValue { text: String },
    /// Replace the current text selection through the native AccessKit request.
    ReplaceSelectedText { text: String },
    /// Scroll the widget (or its scroll container) into view (egui `Action::ScrollIntoView`).
    Scroll,
    /// Select the widget (focus is egui's selection primitive for list/tree rows).
    Select,
}

impl UiAction {
    /// The AccessKit `Action` this UI action dispatches. `Select` maps to `Focus` (egui's
    /// row-selection primitive).
    pub fn accesskit_action(&self) -> accesskit::Action {
        match self {
            UiAction::Click | UiAction::ClickWithPayload { .. } => accesskit::Action::Click,
            UiAction::Focus | UiAction::Select => accesskit::Action::Focus,
            UiAction::SetValue { .. } | UiAction::NativeSetValue { .. } => {
                accesskit::Action::SetValue
            }
            UiAction::ReplaceSelectedText { .. } => accesskit::Action::ReplaceSelectedText,
            UiAction::Scroll => accesskit::Action::ScrollIntoView,
        }
    }

    /// AccessKit action data carried by payload-capable actions.
    pub fn accesskit_data(&self) -> Option<accesskit::ActionData> {
        match self {
            UiAction::ClickWithPayload { payload }
            | UiAction::SetValue { text: payload }
            | UiAction::NativeSetValue { text: payload }
            | UiAction::ReplaceSelectedText { text: payload } => Some(
                accesskit::ActionData::Value(payload.clone().into_boxed_str()),
            ),
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
    /// A numeric widget was addressed with a value that cannot be parsed as a finite number.
    InvalidNumericValue { author_id: String, value: String },
    /// The target has a domain-specific value contract and the supplied value violates it.
    InvalidValue {
        author_id: String,
        value: String,
        reason: String,
    },
    /// A prior mutation for this target is still awaiting post-render observation.
    TargetBusy { author_id: String },
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
            ActionError::InvalidNumericValue { author_id, value } => {
                write!(
                    f,
                    "widget '{author_id}' requires a finite numeric value, got '{value}'"
                )
            }
            ActionError::InvalidValue {
                author_id,
                value,
                reason,
            } => write!(f, "widget '{author_id}' rejected value '{value}': {reason}"),
            ActionError::TargetBusy { author_id } => write!(
                f,
                "widget '{author_id}' already has an unacknowledged mutation"
            ),
            ActionError::QueueFull => write!(f, "action queue full"),
        }
    }
}

impl std::error::Error for ActionError {}

/// The result of resolving + enqueuing one target-specific AccessKit action request.
#[derive(Debug, Clone)]
pub struct ActionOutcome {
    /// The AccessKit request enqueued for the frame loop.
    pub request: accesskit::ActionRequest,
    /// Stable id for the queued/dispatched/terminal receipt.
    pub receipt_id: u64,
}

/// Observable lifecycle of one model mutation. `Applied` is reserved for actions with an
/// action-specific causal observation. A visible requested SetValue alone is not causal proof and
/// therefore terminates as [`ActionReceiptStatus::Indeterminate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionReceiptStatus {
    Queued,
    Dispatched,
    Applied,
    /// The request was dispatched but the post-render tree exposes no action-specific predicate that
    /// can prove the requested effect occurred. This is terminal and deliberately never means success.
    Indeterminate,
    Rejected,
}

/// Bounded diagnostic/caller receipt returned by `argus.inspect` under `action_receipts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionReceipt {
    pub receipt_id: u64,
    pub target: String,
    pub expected_node_id: u64,
    pub expected_role: String,
    pub expected_action: String,
    pub expected_generation: String,
    pub status: ActionReceiptStatus,
    pub observed_value: Option<String>,
    pub rejection: Option<String>,
}

fn resolve_node<'a>(
    snapshot: &'a UiTreeSnapshot,
    author_id: &str,
    action: &UiAction,
) -> Result<&'a crate::accessibility::UiTreeNode, ActionError> {
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

    let needed = format!("{:?}", action.accesskit_action());
    if !node.actions.iter().any(|a| a == &needed) {
        return Err(ActionError::UnsupportedAction {
            author_id: author_id.to_owned(),
            action: needed,
        });
    }

    Ok(node)
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
    let node = resolve_node(snapshot, author_id, action)?;
    Ok(accesskit::NodeId(node.node_id))
}

/// Build the `accesskit::ActionRequest` for a resolved target + action. The
/// request targets the live snapshot's exact `NodeId`; callers take a fresh snapshot before steering,
/// so re-layout or a process restart cannot reuse a stale author-id mapping.
pub fn build_action_request(target: accesskit::NodeId, action: &UiAction) -> ActionOutcome {
    ActionOutcome {
        request: accesskit::ActionRequest {
            action: action.accesskit_action(),
            target,
            data: action.accesskit_data(),
        },
        receipt_id: 0,
    }
}

/// Read the last string-valued native `SetValue` request addressed to `widget_id` in this frame.
///
/// Callers attach `Action::SetValue` to the real widget node, then apply this value directly to that
/// widget's backing state. Requests for an unmounted/different node are never observed here, which is
/// the target-safety property the old global text injection could not provide.
pub fn accesskit_string_set_value(ui: &egui::Ui, widget_id: egui::Id) -> Option<String> {
    let mut replacement = None;
    ui.input(|input| {
        for request in input.accesskit_action_requests(widget_id, accesskit::Action::SetValue) {
            if let Some(accesskit::ActionData::Value(value)) = &request.data {
                replacement = Some(value.to_string());
            }
        }
    });
    replacement
}

fn build_action_request_for_node(
    node: &crate::accessibility::UiTreeNode,
    author_id: &str,
    action: &UiAction,
) -> Result<ActionOutcome, ActionError> {
    let mut outcome = build_action_request(accesskit::NodeId(node.node_id), action);
    if let UiAction::SetValue { text } = action {
        outcome.request.data = if matches!(node.role.as_str(), "SpinButton" | "Slider") {
            let number = text
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| ActionError::InvalidNumericValue {
                    author_id: author_id.to_owned(),
                    value: text.clone(),
                })?;
            Some(accesskit::ActionData::NumericValue(number))
        } else {
            Some(accesskit::ActionData::Value(text.clone().into_boxed_str()))
        };
    }
    Ok(outcome)
}

/// A bounded, in-process FIFO of pending AccessKit action requests the egui frame loop drains each
/// frame. This is the in-process analog of the contract's bounded `tokio::sync::mpsc` channel: the
/// MCP tool layer pushes resolved actions in; the `eframe::App::update` loop drains them out and feeds
/// them to egui. Bounding + per-drain burst limiting implement the back-pressure and flood controls.
#[derive(Debug)]
pub struct ActionChannel {
    queue: VecDeque<PendingAction>,
    in_flight: Vec<PendingAction>,
    receipts: VecDeque<ActionReceipt>,
    capacity: usize,
    next_receipt_id: u64,
}

#[derive(Debug, Clone)]
struct PendingAction {
    outcome: ActionOutcome,
    author_id: String,
    expected_role: String,
    expected_action: String,
    expected_generation: String,
    enqueued_value: Option<String>,
    enqueued_at: Instant,
    action: UiAction,
}

impl Default for ActionChannel {
    fn default() -> Self {
        Self::new()
    }
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
            in_flight: Vec::new(),
            receipts: VecDeque::new(),
            capacity: capacity.max(1),
            next_receipt_id: 1,
        }
    }

    /// Number of pending (not-yet-drained) actions.
    pub fn pending(&self) -> usize {
        self.queue.len() + self.in_flight.len()
    }

    /// True when the queue is at capacity (the next [`Self::enqueue`] would be rejected).
    pub fn is_full(&self) -> bool {
        self.pending() >= self.capacity
    }

    /// Human/OS mutation input already present in the frame wins over model steering. Reject every
    /// not-yet-dispatched action so a stale snapshot cannot authorize a model write after a same-frame
    /// keyboard, paste, or pointer mutation.
    pub fn reject_queued_for_operator_input(&mut self) {
        while let Some(pending) = self.queue.pop_front() {
            self.update_receipt(
                pending.outcome.receipt_id,
                ActionReceiptStatus::Rejected,
                None,
                Some("operator input took priority in the dispatch frame".to_owned()),
            );
        }
    }

    /// Resolve + enqueue an action addressed by `author_id` against the given current-frame snapshot.
    ///
    /// Returns the enqueued [`ActionOutcome`] so the caller can report exactly what was queued, or an
    /// [`ActionError`] when the target cannot be
    /// resolved or the queue is full. Resolution happens BEFORE the capacity check so an unknown /
    /// disabled / unsupported target is reported as such even when the queue is also full (the more
    /// actionable error wins).
    pub fn enqueue(
        &mut self,
        snapshot: &UiTreeSnapshot,
        author_id: &str,
        action: UiAction,
    ) -> Result<ActionOutcome, ActionError> {
        self.expire_stale_actions();
        let node = resolve_node(snapshot, author_id, &action)?;
        validate_target_value(author_id, node, &action)?;
        if self
            .queue
            .iter()
            .chain(self.in_flight.iter())
            .any(|pending| pending.author_id == author_id)
        {
            return Err(ActionError::TargetBusy {
                author_id: author_id.to_owned(),
            });
        }
        if self.is_full() {
            return Err(ActionError::QueueFull);
        }
        let mut outcome = build_action_request_for_node(node, author_id, &action)?;
        let receipt_id = self.next_receipt_id;
        self.next_receipt_id = self.next_receipt_id.wrapping_add(1).max(1);
        outcome.receipt_id = receipt_id;
        let expected_action = format!("{:?}", action.accesskit_action());
        self.receipts.push_back(ActionReceipt {
            receipt_id,
            target: author_id.to_owned(),
            expected_node_id: node.node_id,
            expected_role: node.role.clone(),
            expected_action: expected_action.clone(),
            expected_generation: snapshot.captured_at_utc.clone(),
            status: ActionReceiptStatus::Queued,
            observed_value: None,
            rejection: None,
        });
        self.trim_receipts();
        self.queue.push_back(PendingAction {
            outcome: outcome.clone(),
            author_id: author_id.to_owned(),
            expected_role: node.role.clone(),
            expected_action,
            expected_generation: snapshot.captured_at_utc.clone(),
            enqueued_value: node.value.clone(),
            enqueued_at: Instant::now(),
            action,
        });
        Ok(outcome)
    }

    /// Drain up to [`MAX_ACTIONS_PER_BURST`] pending actions into target-specific AccessKit events.
    /// A set-value event carries its replacement in the request data, so concurrent values never share
    /// the global keyboard/text stream and a widget that disappeared before this frame simply consumes
    /// nothing.
    ///
    /// Returns the events in dispatch order. The frame loop calls this at the start of
    /// `eframe::App::update` (or a test feeds the events to the kittest harness). The burst cap bounds
    /// one frame's injected input regardless of how full the queue is (red-team: action flood).
    pub fn drain_revalidated_into_events(
        &mut self,
        fresh_snapshot: &UiTreeSnapshot,
    ) -> Vec<egui::Event> {
        self.expire_stale_actions();
        let mut events = Vec::new();
        let take = self.queue.len().min(MAX_ACTIONS_PER_BURST);
        for _ in 0..take {
            let Some(mut pending) = self.queue.pop_front() else {
                break;
            };
            if let Err(reason) = revalidate_pending(&pending, fresh_snapshot) {
                self.update_receipt(
                    pending.outcome.receipt_id,
                    ActionReceiptStatus::Rejected,
                    None,
                    Some(reason),
                );
                continue;
            }
            // A timestamp/generation refresh is normal between frame drains. The fresh tree above is
            // authoritative for dispatch after identity, role, enabled state, and action capability
            // all revalidate. Record that actual dispatch generation for diagnostics.
            pending
                .expected_generation
                .clone_from(&fresh_snapshot.captured_at_utc);
            self.update_receipt_generation(
                pending.outcome.receipt_id,
                &fresh_snapshot.captured_at_utc,
            );
            self.update_receipt(
                pending.outcome.receipt_id,
                ActionReceiptStatus::Dispatched,
                None,
                None,
            );
            events.push(egui::Event::AccessKitActionRequest(
                pending.outcome.request.clone(),
            ));
            self.in_flight.push(pending);
        }
        events
    }

    /// Legacy in-process test seam. Production uses [`Self::drain_revalidated_into_events`]. This
    /// retains source compatibility for focused widget tests that construct their own already-current
    /// snapshot and immediately inject the event.
    #[doc(hidden)]
    pub fn drain_into_events(&mut self) -> Vec<egui::Event> {
        let mut events = Vec::new();
        let take = self.queue.len().min(MAX_ACTIONS_PER_BURST);
        for _ in 0..take {
            let Some(pending) = self.queue.pop_front() else {
                break;
            };
            self.update_receipt(
                pending.outcome.receipt_id,
                ActionReceiptStatus::Indeterminate,
                None,
                Some(
                    "legacy dispatch seam has no post-render observation; effect is unverified"
                        .to_owned(),
                ),
            );
            events.push(egui::Event::AccessKitActionRequest(pending.outcome.request));
        }
        events
    }

    /// Complete dispatched transactions from a fresh post-render tree. A visible requested SetValue on
    /// the same target is recorded as terminal `Indeterminate` because the tree has no causal mutation
    /// token; target/action drift is rejected.
    pub fn acknowledge_after_render(&mut self, fresh_snapshot: &UiTreeSnapshot) {
        self.expire_stale_actions();
        let in_flight = std::mem::take(&mut self.in_flight);
        for pending in in_flight {
            let node = fresh_snapshot.find_by_author_id(&pending.author_id);
            match (&pending.action, node) {
                (UiAction::SetValue { text }, Some(node))
                    if post_render_target_identity_matches(&pending, node)
                        && !observed_value_matches(
                            &pending.author_id,
                            &pending.enqueued_value,
                            text,
                        )
                        && observed_value_matches(&pending.author_id, &node.value, text) =>
                {
                    self.update_receipt(
                        pending.outcome.receipt_id,
                        ActionReceiptStatus::Indeterminate,
                        node.value.clone(),
                        Some(
                            "requested value is visible, but the snapshot carries no causal mutation token; concurrent or replacement writes cannot be attributed to Argus"
                                .to_owned(),
                        ),
                    );
                }
                (UiAction::SetValue { text }, Some(node)) => self.update_receipt(
                    pending.outcome.receipt_id,
                    if post_render_target_identity_matches(&pending, node)
                        && observed_value_matches(
                            &pending.author_id,
                            &pending.enqueued_value,
                            text,
                        )
                        && observed_value_matches(&pending.author_id, &node.value, text)
                    {
                        ActionReceiptStatus::Indeterminate
                    } else {
                        ActionReceiptStatus::Rejected
                    },
                    node.value.clone(),
                    Some(if observed_value_matches(
                        &pending.author_id,
                        &pending.enqueued_value,
                        text,
                    ) {
                        format!(
                            "requested value '{text}' was already present before dispatch; mutation cannot be proven"
                        )
                    } else {
                        format!("post-render value did not acknowledge requested value '{text}'")
                    }),
                ),
                (UiAction::SetValue { .. }, None) => self.update_receipt(
                    pending.outcome.receipt_id,
                    ActionReceiptStatus::Rejected,
                    None,
                    Some("target disappeared before mutation acknowledgement".to_owned()),
                ),
                (
                    UiAction::NativeSetValue { text } | UiAction::ReplaceSelectedText { text },
                    Some(node),
                ) if post_render_target_identity_matches(&pending, node)
                    && pending.enqueued_value.as_deref() != Some(text.as_str())
                    && node.value.as_deref() == Some(text.as_str()) =>
                {
                    self.update_receipt(
                        pending.outcome.receipt_id,
                        ActionReceiptStatus::Indeterminate,
                        node.value.clone(),
                        Some(
                            "requested value is visible, but the snapshot carries no causal mutation token; concurrent or replacement writes cannot be attributed to Argus"
                                .to_owned(),
                        ),
                    );
                }
                (
                    UiAction::NativeSetValue { text } | UiAction::ReplaceSelectedText { text },
                    Some(node),
                ) => self.update_receipt(
                    pending.outcome.receipt_id,
                    if post_render_target_identity_matches(&pending, node)
                        && pending.enqueued_value.as_deref() == Some(text.as_str())
                        && node.value.as_deref() == Some(text.as_str())
                    {
                        ActionReceiptStatus::Indeterminate
                    } else {
                        ActionReceiptStatus::Rejected
                    },
                    node.value.clone(),
                    Some(if pending.enqueued_value.as_deref() == Some(text.as_str()) {
                        format!(
                            "requested value '{text}' was already present before dispatch; mutation cannot be proven"
                        )
                    } else {
                        format!(
                            "post-render value did not exactly acknowledge requested value '{text}'"
                        )
                    }),
                ),
                (UiAction::NativeSetValue { .. } | UiAction::ReplaceSelectedText { .. }, None) => {
                    self.update_receipt(
                        pending.outcome.receipt_id,
                        ActionReceiptStatus::Rejected,
                        None,
                        Some("target disappeared before mutation acknowledgement".to_owned()),
                    )
                }
                (UiAction::Click | UiAction::ClickWithPayload { .. }, Some(node)) => {
                    let reason = if post_render_target_identity_matches(&pending, node) {
                        "click was dispatched, but this target exposes no action-specific completion predicate"
                    } else {
                        "click target identity changed before acknowledgement"
                    };
                    self.update_receipt(
                        pending.outcome.receipt_id,
                        ActionReceiptStatus::Indeterminate,
                        node.value.clone(),
                        Some(reason.to_owned()),
                    );
                }
                (UiAction::Click | UiAction::ClickWithPayload { .. }, None) => self.update_receipt(
                    pending.outcome.receipt_id,
                    ActionReceiptStatus::Indeterminate,
                    None,
                    Some("click target disappeared before its effect could be observed".to_owned()),
                ),
                (UiAction::Focus | UiAction::Select | UiAction::Scroll, Some(node)) => {
                    self.update_receipt(
                        pending.outcome.receipt_id,
                        ActionReceiptStatus::Indeterminate,
                        node.value.clone(),
                        Some(
                            "no explicit focus, selection, or scroll predicate is exposed for post-render acknowledgement"
                                .to_owned(),
                        ),
                    );
                }
                (UiAction::Focus | UiAction::Select | UiAction::Scroll, None) => self
                    .update_receipt(
                        pending.outcome.receipt_id,
                        ActionReceiptStatus::Indeterminate,
                        None,
                        Some("target disappeared before its effect could be observed".to_owned()),
                    ),
            }
        }
    }

    pub fn receipts(&mut self) -> Vec<ActionReceipt> {
        self.expire_stale_actions();
        self.receipts.iter().cloned().collect()
    }

    fn update_receipt(
        &mut self,
        receipt_id: u64,
        status: ActionReceiptStatus,
        observed_value: Option<String>,
        rejection: Option<String>,
    ) {
        if let Some(receipt) = self
            .receipts
            .iter_mut()
            .find(|receipt| receipt.receipt_id == receipt_id)
        {
            receipt.status = status;
            receipt.observed_value = observed_value;
            receipt.rejection = rejection;
        }
    }

    fn update_receipt_generation(&mut self, receipt_id: u64, generation: &str) {
        if let Some(receipt) = self
            .receipts
            .iter_mut()
            .find(|receipt| receipt.receipt_id == receipt_id)
        {
            receipt.expected_generation = generation.to_owned();
        }
    }

    fn trim_receipts(&mut self) {
        const MAX_RECEIPTS: usize = 256;
        while self.receipts.len() > MAX_RECEIPTS {
            if self.receipts.front().is_some_and(|receipt| {
                matches!(
                    receipt.status,
                    ActionReceiptStatus::Queued | ActionReceiptStatus::Dispatched
                )
            }) {
                break;
            }
            self.receipts.pop_front();
        }
    }

    fn expire_stale_actions(&mut self) {
        let now = Instant::now();
        let mut expired = Vec::new();
        self.queue.retain(|pending| {
            let timed_out =
                now.saturating_duration_since(pending.enqueued_at) >= ACTION_LEASE_TIMEOUT;
            if timed_out {
                expired.push((pending.outcome.receipt_id, pending.enqueued_value.clone()));
            }
            !timed_out
        });
        self.in_flight.retain(|pending| {
            let timed_out =
                now.saturating_duration_since(pending.enqueued_at) >= ACTION_LEASE_TIMEOUT;
            if timed_out {
                expired.push((pending.outcome.receipt_id, pending.enqueued_value.clone()));
            }
            !timed_out
        });
        for (receipt_id, _enqueued_value) in expired {
            self.update_receipt(
                receipt_id,
                ActionReceiptStatus::Indeterminate,
                None,
                Some("action acknowledgement deadline elapsed; target lease released".to_owned()),
            );
        }
    }
}

fn post_render_target_identity_matches(
    pending: &PendingAction,
    node: &crate::accessibility::UiTreeNode,
) -> bool {
    node.node_id == pending.outcome.request.target.0
        && node.role == pending.expected_role
        && !node.disabled
        && node
            .actions
            .iter()
            .any(|action| action == &pending.expected_action)
}

fn revalidate_pending(pending: &PendingAction, snapshot: &UiTreeSnapshot) -> Result<(), String> {
    let node = snapshot
        .find_by_author_id(&pending.author_id)
        .ok_or_else(|| "target disappeared before dispatch".to_owned())?;
    if node.node_id != pending.outcome.request.target.0 {
        return Err("target NodeId changed before dispatch".to_owned());
    }
    if node.role != pending.expected_role {
        return Err("target role changed before dispatch".to_owned());
    }
    if node.disabled {
        return Err("target became disabled before dispatch".to_owned());
    }
    if !node
        .actions
        .iter()
        .any(|action| action == &pending.expected_action)
    {
        return Err("target no longer supports the requested action".to_owned());
    }
    if node.value != pending.enqueued_value {
        return Err("target value changed before dispatch".to_owned());
    }
    Ok(())
}

fn validate_target_value(
    author_id: &str,
    node: &crate::accessibility::UiTreeNode,
    action: &UiAction,
) -> Result<(), ActionError> {
    let UiAction::SetValue { text } = action else {
        return Ok(());
    };
    let invalid = |reason: &str| ActionError::InvalidValue {
        author_id: author_id.to_owned(),
        value: text.clone(),
        reason: reason.to_owned(),
    };
    let lower = text.trim().to_ascii_lowercase();
    match author_id {
        "settings-editor-font-size" => validate_number(text, 6.0, 48.0, false)
            .map_err(|_| invalid("expected a finite number in 6..=48"))?,
        "settings-editor-tab-size" => validate_number(text, 1.0, 16.0, true)
            .map_err(|_| invalid("expected an integer in 1..=16"))?,
        "settings-editor-wrap-column" => validate_number(text, 20.0, 400.0, true)
            .map_err(|_| invalid("expected an integer in 20..=400"))?,
        "settings-editor-line-height" => validate_number(text, 1.0, 2.0, false)
            .map_err(|_| invalid("expected a finite number in 1..=2"))?,
        "settings-editor-word-wrap"
            if !matches!(
                lower.as_str(),
                "off" | "on" | "viewport" | "on (viewport)" | "bounded" | "bounded column"
            ) =>
        {
            return Err(invalid("expected off, on, or bounded"))
        }
        "settings-editor-render-whitespace"
            if !matches!(lower.as_str(), "none" | "boundary" | "all") =>
        {
            return Err(invalid("expected none, boundary, or all"))
        }
        "settings-syntax-palette-mode"
            if !matches!(lower.as_str(), "muted" | "standard" | "custom") =>
        {
            return Err(invalid("expected muted, standard, or custom"))
        }
        _ if author_id.starts_with("settings-keybind-row-") => {
            crate::code_editor::keymap_settings::KeymapSettings::chord_from_str(text)
                .map_err(|error| invalid(&format!("expected a valid key chord: {error}")))?;
        }
        _ if author_id.starts_with("settings-syntax-swatch-") && !valid_srgba(text) => {
            return Err(invalid(
                "expected #RRGGBB, #RRGGBBAA, or a JSON [r,g,b,a] byte array",
            ))
        }
        _ if matches!(node.role.as_str(), "SpinButton" | "Slider") => {
            text.parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| ActionError::InvalidNumericValue {
                    author_id: author_id.to_owned(),
                    value: text.clone(),
                })?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_number(text: &str, min: f64, max: f64, integer: bool) -> Result<(), ()> {
    let value = text.parse::<f64>().map_err(|_| ())?;
    if !value.is_finite() || value < min || value > max || (integer && value.fract() != 0.0) {
        return Err(());
    }
    Ok(())
}

fn valid_srgba(text: &str) -> bool {
    parse_srgba_channels(text).is_some()
}

fn parse_srgba_channels(text: &str) -> Option<[u8; 4]> {
    let trimmed = text.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if !matches!(hex.len(), 6 | 8)
            || !hex.is_ascii()
            || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        let byte = |start| u8::from_str_radix(hex.get(start..start + 2)?, 16).ok();
        return Some([
            byte(0)?,
            byte(2)?,
            byte(4)?,
            if hex.len() == 8 { byte(6)? } else { 255 },
        ]);
    }
    serde_json::from_str::<[u8; 4]>(trimmed).ok()
}

fn observed_value_matches(author_id: &str, observed: &Option<String>, requested: &str) -> bool {
    let Some(observed) = observed.as_deref() else {
        return false;
    };
    let requested_lower = requested.trim().to_ascii_lowercase();
    match author_id {
        "settings-editor-word-wrap" => match requested_lower.as_str() {
            "off" => observed == "Off",
            "on" | "viewport" | "on (viewport)" => observed == "On (viewport)",
            "bounded" | "bounded column" => observed.starts_with("Bounded"),
            _ => false,
        },
        "settings-editor-render-whitespace" | "settings-syntax-palette-mode" => {
            observed.eq_ignore_ascii_case(&requested_lower)
        }
        _ if author_id.starts_with("settings-syntax-swatch-") => {
            parse_srgba_channels(observed) == parse_srgba_channels(requested)
        }
        _ if matches!(
            author_id,
            "settings-editor-font-size"
                | "settings-editor-tab-size"
                | "settings-editor-wrap-column"
                | "settings-editor-line-height"
        ) =>
        {
            let observed_number = observed
                .split_whitespace()
                .next()
                .and_then(|value| value.trim_end_matches(['×']).parse::<f64>().ok());
            let requested_number = requested.parse::<f64>().ok();
            matches!((observed_number, requested_number), (Some(a), Some(b)) if (a - b).abs() < 0.0001)
        }
        _ => observed == requested,
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
            actions: vec![
                "Click".to_owned(),
                "Focus".to_owned(),
                "SetValue".to_owned(),
            ],
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
            actions: vec![
                "Click".to_owned(),
                "Focus".to_owned(),
                "SetValue".to_owned(),
                "ReplaceSelectedText".to_owned(),
            ],
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
    fn set_value_resolves_to_targeted_native_replacement() {
        let snap = fixture_snapshot();
        let action = UiAction::SetValue {
            text: "hello swarm".to_owned(),
        };
        let id = resolve_target(&snap, "field", &action).expect("field resolves via SetValue");
        assert_eq!(id, accesskit::NodeId(11));
        let outcome = build_action_request(id, &action);
        assert_eq!(outcome.request.action, accesskit::Action::SetValue);
        assert!(matches!(
            outcome.request.data,
            Some(accesskit::ActionData::Value(ref value)) if value.as_ref() == "hello swarm"
        ));
    }

    #[test]
    fn queue_is_bounded_and_reports_full() {
        let mut snap = fixture_snapshot();
        let mut second = snap.root.children[0].clone();
        second.id = "btn2".to_owned();
        second.author_id = Some("btn2".to_owned());
        second.node_id = 13;
        snap.root.children.push(second);
        let mut chan = ActionChannel::with_capacity(2);
        assert!(chan.enqueue(&snap, "btn", UiAction::Click).is_ok());
        assert!(chan.enqueue(&snap, "btn2", UiAction::Click).is_ok());
        assert!(chan.is_full());
        let err = chan.enqueue(&snap, "field", UiAction::Click).unwrap_err();
        assert_eq!(err, ActionError::QueueFull);
        assert_eq!(chan.pending(), 2);
    }

    #[test]
    fn drain_emits_one_targeted_event_per_action_and_respects_burst_cap() {
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
        assert_eq!(events.len(), 1);
        let egui::Event::AccessKitActionRequest(request) = &events[0] else {
            panic!("set value must remain one targeted AccessKit request");
        };
        assert_eq!(request.target, accesskit::NodeId(11));
        assert_eq!(request.action, accesskit::Action::SetValue);
        assert!(matches!(
            request.data.as_ref(),
            Some(accesskit::ActionData::Value(value)) if value.as_ref() == "abc"
        ));
        assert_eq!(chan.pending(), 0, "drained");

        // Burst cap: enqueue more than MAX_ACTIONS_PER_BURST clicks; one drain takes at most the cap.
        let mut chan = ActionChannel::new();
        let mut burst_snapshot = snap.clone();
        for index in 0..(MAX_ACTIONS_PER_BURST + 5) {
            let mut node = burst_snapshot.root.children[0].clone();
            let author_id = format!("burst-{index}");
            node.id.clone_from(&author_id);
            node.author_id = Some(author_id.clone());
            node.node_id = 100 + index as u64;
            burst_snapshot.root.children.push(node);
            chan.enqueue(&burst_snapshot, &author_id, UiAction::Click)
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

    #[test]
    fn revalidation_accepts_generation_refresh_but_rejects_node_identity_drift() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "after".into(),
                },
            )
            .expect("queue field");
        let mut refreshed = snap.clone();
        refreshed.captured_at_utc = "1.000000000Z".to_owned();
        assert_eq!(
            chan.drain_revalidated_into_events(&refreshed).len(),
            1,
            "frame timestamp refresh alone is not target drift"
        );
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Dispatched);
        assert_eq!(receipt.expected_generation, refreshed.captured_at_utc);

        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "after".into(),
                },
            )
            .expect("queue field for identity drift");
        let mut changed = refreshed;
        changed.root.children[1].node_id = 99;
        assert!(chan.drain_revalidated_into_events(&changed).is_empty());
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Rejected);
        assert!(receipt.rejection.unwrap().contains("NodeId changed"));

        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "after".into(),
                },
            )
            .expect("queue field for value drift");
        let mut changed = snap.clone();
        changed.captured_at_utc = "2.000000000Z".to_owned();
        changed.root.children[1].value = Some("concurrent-writer".to_owned());
        assert!(chan.drain_revalidated_into_events(&changed).is_empty());
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Rejected);
        assert!(receipt.rejection.unwrap().contains("value changed"));
    }

    #[test]
    fn operator_input_priority_rejects_queued_model_mutation_before_dispatch() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "model-write".to_owned(),
                },
            )
            .expect("queue model mutation");
        chan.reject_queued_for_operator_input();
        assert!(chan.drain_revalidated_into_events(&snap).is_empty());
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .expect("operator-priority receipt");
        assert_eq!(receipt.status, ActionReceiptStatus::Rejected);
        assert!(receipt
            .rejection
            .as_deref()
            .is_some_and(|reason| reason.contains("operator input took priority")));
    }

    #[test]
    fn receipt_projection_expires_an_overdue_action_without_another_dispatch_cycle() {
        let snap = fixture_snapshot();
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snap, "btn", UiAction::Click)
            .expect("queue click");
        channel.queue.front_mut().unwrap().enqueued_at =
            Instant::now() - ACTION_LEASE_TIMEOUT - Duration::from_millis(1);

        let receipt = channel
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert!(receipt
            .rejection
            .as_deref()
            .is_some_and(|reason| reason.contains("deadline elapsed")));
        assert!(channel.queue.is_empty());
    }

    #[test]
    fn queued_remainder_survives_snapshot_refresh_and_drains_on_second_frame() {
        let mut first_frame = fixture_snapshot();
        let action_count = MAX_ACTIONS_PER_BURST + 5;
        for index in 0..action_count {
            let mut node = first_frame.root.children[0].clone();
            let author_id = format!("two-frame-burst-{index}");
            node.id.clone_from(&author_id);
            node.author_id = Some(author_id);
            node.node_id = 200 + index as u64;
            first_frame.root.children.push(node);
        }
        first_frame.widget_count = first_frame.root.children.len() + 1;

        let mut channel = ActionChannel::new();
        let mut receipt_ids = Vec::new();
        for index in 0..action_count {
            let outcome = channel
                .enqueue(
                    &first_frame,
                    &format!("two-frame-burst-{index}"),
                    UiAction::Click,
                )
                .expect("queue distinct burst target");
            receipt_ids.push(outcome.receipt_id);
        }

        assert_eq!(
            channel.drain_revalidated_into_events(&first_frame).len(),
            MAX_ACTIONS_PER_BURST
        );
        let mut second_frame = first_frame.clone();
        second_frame.captured_at_utc = "1.000000000Z".to_owned();
        channel.acknowledge_after_render(&second_frame);
        assert_eq!(
            channel.pending(),
            5,
            "only the burst remainder stays queued"
        );

        assert_eq!(
            channel.drain_revalidated_into_events(&second_frame).len(),
            5,
            "unchanged targets queued behind the first-frame cap dispatch next frame"
        );
        for receipt_id in receipt_ids.into_iter().skip(MAX_ACTIONS_PER_BURST) {
            let receipt = channel
                .receipts()
                .into_iter()
                .find(|receipt| receipt.receipt_id == receipt_id)
                .expect("remainder receipt");
            assert_eq!(receipt.status, ActionReceiptStatus::Dispatched);
            assert_eq!(receipt.expected_generation, second_frame.captured_at_utc);
            assert!(receipt.rejection.is_none());
        }
    }

    #[test]
    fn same_target_serializes_until_observed_acknowledgement() {
        let mut snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let first = chan
            .enqueue(&snap, "field", UiAction::SetValue { text: "one".into() })
            .unwrap();
        assert!(matches!(
            chan.enqueue(&snap, "field", UiAction::SetValue { text: "two".into() }),
            Err(ActionError::TargetBusy { .. })
        ));
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        snap.root.children[1].value = Some("one".to_owned());
        chan.acknowledge_after_render(&snap);
        assert_eq!(
            chan.receipts()
                .into_iter()
                .find(|receipt| receipt.receipt_id == first.receipt_id)
                .unwrap()
                .status,
            ActionReceiptStatus::Indeterminate
        );
        assert!(chan
            .enqueue(&snap, "field", UiAction::SetValue { text: "two".into() })
            .is_ok());
    }

    #[test]
    fn no_op_click_is_indeterminate_and_releases_target_lease() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan.enqueue(&snap, "btn", UiAction::Click).unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        chan.acknowledge_after_render(&snap);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert_eq!(chan.pending(), 0);
        assert!(chan.enqueue(&snap, "btn", UiAction::Click).is_ok());
    }

    #[test]
    fn disappeared_click_target_is_never_reported_applied() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan.enqueue(&snap, "btn", UiAction::Click).unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        let mut observed = snap.clone();
        observed
            .root
            .children
            .retain(|node| node.author_id.as_deref() != Some("btn"));
        chan.acknowledge_after_render(&observed);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert_ne!(receipt.status, ActionReceiptStatus::Applied);
        assert_eq!(chan.pending(), 0);
    }

    #[test]
    fn generic_click_change_is_still_indeterminate_without_action_specific_predicate() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan.enqueue(&snap, "btn", UiAction::Click).unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        let mut observed = snap.clone();
        observed.root.children[0].value = Some("activated".to_owned());
        chan.acknowledge_after_render(&observed);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert!(receipt
            .rejection
            .as_deref()
            .is_some_and(|reason| reason.contains("no action-specific completion predicate")));
    }

    #[test]
    fn already_equal_set_value_is_indeterminate_not_false_applied() {
        let mut snap = fixture_snapshot();
        snap.root.children[1].value = Some("already".to_owned());
        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "already".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        chan.acknowledge_after_render(&snap);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert_ne!(receipt.status, ActionReceiptStatus::Applied);
    }

    #[test]
    fn semantically_equal_normalized_set_value_is_indeterminate() {
        let mut snap = fixture_snapshot();
        snap.root.children[1].id = "settings-editor-word-wrap".to_owned();
        snap.root.children[1].author_id = Some("settings-editor-word-wrap".to_owned());
        snap.root.children[1].value = Some("On (viewport)".to_owned());
        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "settings-editor-word-wrap",
                UiAction::SetValue {
                    text: "on".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        chan.acknowledge_after_render(&snap);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert_ne!(receipt.status, ActionReceiptStatus::Applied);
    }

    #[test]
    fn reused_node_id_with_changed_role_cannot_acknowledge_set_value() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "after".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        let mut observed = snap.clone();
        observed.root.children[1].role = "Slider".to_owned();
        observed.root.children[1].value = Some("after".to_owned());
        chan.acknowledge_after_render(&observed);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Rejected);
        assert_ne!(receipt.status, ActionReceiptStatus::Applied);
    }

    #[test]
    fn same_role_same_node_replacement_reaching_value_is_indeterminate() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "after".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);

        // A replacement control can reuse author_id, NodeId, role and supported actions while a
        // concurrent writer happens to publish the requested value. Snapshot identity/value alone
        // cannot causally attribute that mutation to Argus.
        let mut replacement = snap.clone();
        replacement.root.children[1].value = Some("after".to_owned());
        chan.acknowledge_after_render(&replacement);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert!(receipt
            .rejection
            .as_deref()
            .is_some_and(|reason| reason.contains("no causal mutation token")));
    }

    #[test]
    fn expired_in_flight_action_releases_target_lease() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let first = chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "first".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
        chan.in_flight[0].enqueued_at =
            Instant::now() - ACTION_LEASE_TIMEOUT - Duration::from_secs(1);
        assert!(chan
            .enqueue(
                &snap,
                "field",
                UiAction::SetValue {
                    text: "second".to_owned(),
                },
            )
            .is_ok());
        let expired = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == first.receipt_id)
            .unwrap();
        assert_eq!(expired.status, ActionReceiptStatus::Indeterminate);
        assert!(expired
            .rejection
            .as_deref()
            .is_some_and(|reason| reason.contains("deadline elapsed")));
    }

    #[test]
    fn legacy_drain_never_claims_applied_without_post_render_observation() {
        let snap = fixture_snapshot();
        let mut chan = ActionChannel::new();
        let outcome = chan.enqueue(&snap, "btn", UiAction::Click).unwrap();
        assert_eq!(chan.drain_into_events().len(), 1);
        let receipt = chan
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == outcome.receipt_id)
            .unwrap();
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert_eq!(chan.pending(), 0);
    }

    #[test]
    fn native_and_replace_value_require_exact_post_render_readback() {
        for action in [
            UiAction::NativeSetValue {
                text: "expected".to_owned(),
            },
            UiAction::ReplaceSelectedText {
                text: "expected".to_owned(),
            },
        ] {
            let snap = fixture_snapshot();
            let mut chan = ActionChannel::new();
            let outcome = chan.enqueue(&snap, "field", action).unwrap();
            assert_eq!(chan.drain_revalidated_into_events(&snap).len(), 1);
            let mut observed = snap.clone();
            observed.root.children[1].value = Some("not-expected".to_owned());
            chan.acknowledge_after_render(&observed);
            let receipt = chan
                .receipts()
                .into_iter()
                .find(|receipt| receipt.receipt_id == outcome.receipt_id)
                .unwrap();
            assert_eq!(receipt.status, ActionReceiptStatus::Rejected);
            assert_eq!(chan.pending(), 0);
        }
    }

    #[test]
    fn simultaneous_same_target_mutations_admit_exactly_one_until_acknowledged() {
        use std::sync::{Arc, Barrier, Mutex};

        let snapshot = Arc::new(fixture_snapshot());
        let channel = Arc::new(Mutex::new(ActionChannel::new()));
        let barrier = Arc::new(Barrier::new(3));
        let workers = ["one", "two"].map(|text| {
            let snapshot = Arc::clone(&snapshot);
            let channel = Arc::clone(&channel);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                channel
                    .lock()
                    .unwrap()
                    .enqueue(
                        &snapshot,
                        "field",
                        UiAction::SetValue {
                            text: text.to_owned(),
                        },
                    )
                    .map(|_| ())
            })
        });
        barrier.wait();
        let results = workers.map(|worker| worker.join().expect("mutation worker"));
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(ActionError::TargetBusy { .. })))
                .count(),
            1
        );
        assert_eq!(channel.lock().unwrap().pending(), 1);
    }

    #[test]
    fn settings_domains_reject_invalid_and_utf8_swatch_without_panicking() {
        let mut snap = fixture_snapshot();
        let node = &mut snap.root.children[1];
        node.id = "settings-editor-word-wrap".to_owned();
        node.author_id = Some("settings-editor-word-wrap".to_owned());
        let mut chan = ActionChannel::new();
        assert!(matches!(
            chan.enqueue(
                &snap,
                "settings-editor-word-wrap",
                UiAction::SetValue {
                    text: "diagonal".into()
                }
            ),
            Err(ActionError::InvalidValue { .. })
        ));
        snap.root.children[1].id = "settings-syntax-swatch-keyword".to_owned();
        snap.root.children[1].author_id = Some("settings-syntax-swatch-keyword".to_owned());
        assert!(matches!(
            chan.enqueue(
                &snap,
                "settings-syntax-swatch-keyword",
                UiAction::SetValue {
                    text: "#aéabc".into()
                }
            ),
            Err(ActionError::InvalidValue { .. })
        ));
        snap.root.children[1].id = "settings-keybind-row-open_find".to_owned();
        snap.root.children[1].author_id = Some("settings-keybind-row-open_find".to_owned());
        assert!(matches!(
            chan.enqueue(
                &snap,
                "settings-keybind-row-open_find",
                UiAction::SetValue {
                    text: "Ctrl+DefinitelyNotAKey".into()
                }
            ),
            Err(ActionError::InvalidValue { .. })
        ));
    }
}
