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
use serde::{Deserialize, Serialize};

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

/// Reserved, opt-in action-specific acknowledgement carried in an AccessKit node's raw `value`.
/// Generic controls never receive this schema and therefore retain conservative `Indeterminate`
/// click receipts.
const CLICK_COMPLETION_SCHEMA: &str = "handshake.click-completion/v1";
const MAX_CLICK_COMPLETION_TOKEN_BYTES: usize = 4096;
const MAX_CLICK_COMPLETION_EFFECT_BYTES: usize = 128;
const MAX_CLICK_COMPLETION_CONTEXT_BYTES: usize = 512;
const MAX_CLICK_COMPLETION_AUTHOR_BYTES: usize = 256;
const MAX_CLICK_COMPLETION_SEMANTIC_BYTES: usize = 2048;
const MAX_CLICK_COMPLETION_ERROR_BYTES: usize = 1024;
const MAX_CLICK_COMPLETION_DETAIL_BYTES: usize = 2048;

/// Opt-in causal acknowledgement for a SetValue target. A sibling Status node carries this token
/// because the target's own value must remain the operator-visible text. Generic SetValue controls
/// without this observer retain the conservative Indeterminate behavior below.
const SET_VALUE_COMPLETION_SCHEMA: &str = "handshake.set-value-completion/v1";
const SET_VALUE_COMPLETION_SUFFIX: &str = ".set-value-completion";
const MAX_SET_VALUE_COMPLETION_TOKEN_BYTES: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SetValueCompletionToken {
    schema: String,
    target: String,
    context: String,
    generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    applied_value: Option<String>,
}

impl SetValueCompletionToken {
    fn valid(&self) -> bool {
        self.schema == SET_VALUE_COMPLETION_SCHEMA
            && valid_click_token_field(&self.target, MAX_CLICK_COMPLETION_AUTHOR_BYTES)
            && valid_click_token_field(&self.context, MAX_CLICK_COMPLETION_CONTEXT_BYTES)
            && self.applied_value.as_deref().map_or(true, |value| {
                value.len() <= MAX_SET_VALUE_COMPLETION_TOKEN_BYTES
                    && !value.chars().any(char::is_control)
            })
    }
}

pub(crate) fn set_value_completion_author_id(target: &str) -> String {
    format!("{target}{SET_VALUE_COMPLETION_SUFFIX}")
}

pub(crate) fn serialize_set_value_completion(
    target: &str,
    context: &str,
    generation: u64,
    applied_value: Option<&str>,
) -> Option<String> {
    let token = SetValueCompletionToken {
        schema: SET_VALUE_COMPLETION_SCHEMA.to_owned(),
        target: target.to_owned(),
        context: context.to_owned(),
        generation,
        applied_value: applied_value.map(str::to_owned),
    };
    if !token.valid() {
        return None;
    }
    let encoded = serde_json::to_string(&token).ok()?;
    (encoded.len() <= MAX_SET_VALUE_COMPLETION_TOKEN_BYTES).then_some(encoded)
}

fn parse_set_value_completion(raw: &str) -> Option<SetValueCompletionToken> {
    if raw.len() > MAX_SET_VALUE_COMPLETION_TOKEN_BYTES {
        return None;
    }
    let token: SetValueCompletionToken = serde_json::from_str(raw).ok()?;
    token.valid().then_some(token)
}

/// State transition exposed by an opt-in click-completion token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ClickCompletionState {
    Ready,
    Pending,
    Applied,
    /// The exact observer-backed action reached a typed terminal failure. This is deliberately
    /// distinct from `Indeterminate`: the observer causally owns the failure and binds it to the
    /// same target/context/generation/semantic tuple declared before dispatch.
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ClickCompletionMode {
    SameTarget,
    Observer,
}

/// One deliberately closed token shape for both supported acknowledgement modes. Optional fields
/// are validated as an exact mode/state-specific combination after serde rejects unknown fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClickCompletionToken {
    schema: String,
    mode: ClickCompletionMode,
    effect: String,
    context: String,
    generation: u64,
    state: ClickCompletionState,
    #[serde(skip_serializing_if = "Option::is_none")]
    observer_author_id: Option<String>,
    /// Observer declarations default to transient-target semantics. A deliberately persistent row
    /// opts in explicitly and is then required to retain its exact dispatch-bound identity and raw
    /// declaration token through acknowledgement.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    persistent_target: bool,
    /// Retry-style controls may disappear on recovery or remain on a typed terminal failure. When
    /// they remain, they are held to the same exact identity/declaration-advance checks as persistent
    /// targets; absence is also permitted. This is opt-in and mutually exclusive with persistence.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    flexible_target: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending_target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal_detail: Option<String>,
}

impl ClickCompletionToken {
    fn valid_common(&self) -> bool {
        self.schema == CLICK_COMPLETION_SCHEMA
            && valid_click_token_field(&self.effect, MAX_CLICK_COMPLETION_EFFECT_BYTES)
            && valid_click_token_field(&self.context, MAX_CLICK_COMPLETION_CONTEXT_BYTES)
            && self.observer_author_id.as_deref().map_or(true, |value| {
                valid_click_token_field(value, MAX_CLICK_COMPLETION_AUTHOR_BYTES)
            })
            && self.pending_target.as_deref().map_or(true, |value| {
                valid_click_token_field(value, MAX_CLICK_COMPLETION_AUTHOR_BYTES)
            })
            && self.semantic_value.as_deref().map_or(true, |value| {
                valid_click_token_field(value, MAX_CLICK_COMPLETION_SEMANTIC_BYTES)
            })
            && self.terminal_error.as_deref().map_or(true, |value| {
                valid_click_token_field(value, MAX_CLICK_COMPLETION_ERROR_BYTES)
            })
            && self.terminal_detail.as_deref().map_or(true, |value| {
                valid_click_token_field(value, MAX_CLICK_COMPLETION_DETAIL_BYTES)
            })
    }

    fn valid_same_target(&self) -> bool {
        self.valid_common()
            && self.mode == ClickCompletionMode::SameTarget
            && self.observer_author_id.is_none()
            && !self.persistent_target
            && !self.flexible_target
            && self.pending_target.is_none()
            && self.semantic_value.is_none()
            && self.terminal_error.is_none()
            && self.terminal_detail.is_none()
            && self.state != ClickCompletionState::Failed
    }

    fn valid_observer_target_declaration(&self) -> bool {
        self.valid_common()
            && self.mode == ClickCompletionMode::Observer
            && self.state == ClickCompletionState::Ready
            && self.observer_author_id.is_some()
            && !(self.persistent_target && self.flexible_target)
            && self.pending_target.is_none()
            && self.semantic_value.is_some()
            && self.terminal_error.is_none()
            && self.terminal_detail.is_none()
    }

    fn valid_observer_state(&self) -> bool {
        self.valid_common()
            && self.mode == ClickCompletionMode::Observer
            && self.observer_author_id.is_none()
            && !self.persistent_target
            && !self.flexible_target
            && match self.state {
                ClickCompletionState::Ready => {
                    self.pending_target.is_none()
                        && self.semantic_value.is_none()
                        && self.terminal_error.is_none()
                        && self.terminal_detail.is_none()
                }
                ClickCompletionState::Pending => {
                    self.pending_target.is_some()
                        && self.semantic_value.is_some()
                        && self.terminal_error.is_none()
                        && self.terminal_detail.is_none()
                }
                ClickCompletionState::Applied => {
                    self.pending_target.is_some()
                        && self.semantic_value.is_some()
                        && self.terminal_error.is_none()
                }
                ClickCompletionState::Failed => {
                    self.pending_target.is_some()
                        && self.semantic_value.is_some()
                        && self.terminal_error.is_some()
                }
            }
    }
}

fn valid_click_token_field(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

fn serialize_click_completion_token(token: &ClickCompletionToken) -> Option<String> {
    if !token.valid_common() {
        return None;
    }
    let encoded = serde_json::to_string(token).ok()?;
    (encoded.len() <= MAX_CLICK_COMPLETION_TOKEN_BYTES).then_some(encoded)
}

fn parse_click_completion_token(raw: &str) -> Option<ClickCompletionToken> {
    if raw.len() > MAX_CLICK_COMPLETION_TOKEN_BYTES {
        return None;
    }
    let token: ClickCompletionToken = serde_json::from_str(raw).ok()?;
    token.valid_common().then_some(token)
}

/// Serialize the value for a same-target acknowledgement. The same author-id/node/context/effect
/// must transition a settled Ready/Applied generation to Pending/Applied at exactly generation + 1
/// after a plain Click.
pub(crate) fn serialize_same_target_click_completion(
    effect: &str,
    context: &str,
    generation: u64,
    state: ClickCompletionState,
) -> Option<String> {
    let token = ClickCompletionToken {
        schema: CLICK_COMPLETION_SCHEMA.to_owned(),
        mode: ClickCompletionMode::SameTarget,
        effect: effect.to_owned(),
        context: context.to_owned(),
        generation,
        state,
        observer_author_id: None,
        persistent_target: false,
        flexible_target: false,
        pending_target: None,
        semantic_value: None,
        terminal_error: None,
        terminal_detail: None,
    };
    token
        .valid_same_target()
        .then(|| serialize_click_completion_token(&token))
        .flatten()
}

/// Serialize an observer-mode target declaration. `generation` is the exact stable non-Pending
/// generation of the durable observer named by `observer_author_id`; `semantic_value` identifies
/// this target's effect so a shared observer cannot acknowledge the wrong completion row. A durable
/// terminal baseline can therefore stay observable while the next action declares its successor.
pub(crate) fn serialize_observer_click_target(
    effect: &str,
    context: &str,
    generation: u64,
    observer_author_id: &str,
    semantic_value: &str,
) -> Option<String> {
    let token = ClickCompletionToken {
        schema: CLICK_COMPLETION_SCHEMA.to_owned(),
        mode: ClickCompletionMode::Observer,
        effect: effect.to_owned(),
        context: context.to_owned(),
        generation,
        state: ClickCompletionState::Ready,
        observer_author_id: Some(observer_author_id.to_owned()),
        persistent_target: false,
        flexible_target: false,
        pending_target: None,
        semantic_value: Some(semantic_value.to_owned()),
        terminal_error: None,
        terminal_detail: None,
    };
    token
        .valid_observer_target_declaration()
        .then(|| serialize_click_completion_token(&token))
        .flatten()
}

/// Serialize an observer-mode declaration for a control that intentionally remains mounted after
/// activation. A persistent acknowledgement is stricter than the transient form: the target must
/// retain its exact node id, role, action capability, and raw declaration value while the observer
/// makes the action-specific generation transition.
pub(crate) fn serialize_persistent_observer_click_target(
    effect: &str,
    context: &str,
    generation: u64,
    observer_author_id: &str,
    semantic_value: &str,
) -> Option<String> {
    let token = ClickCompletionToken {
        schema: CLICK_COMPLETION_SCHEMA.to_owned(),
        mode: ClickCompletionMode::Observer,
        effect: effect.to_owned(),
        context: context.to_owned(),
        generation,
        state: ClickCompletionState::Ready,
        observer_author_id: Some(observer_author_id.to_owned()),
        persistent_target: true,
        flexible_target: false,
        pending_target: None,
        semantic_value: Some(semantic_value.to_owned()),
        terminal_error: None,
        terminal_detail: None,
    };
    token
        .valid_observer_target_declaration()
        .then(|| serialize_click_completion_token(&token))
        .flatten()
}

/// Serialize an observer-mode declaration for a Retry-style control whose success removes the
/// target while a terminal failure leaves the exact control mounted. A present post-target is held
/// to the full persistent identity/declaration transition; an absent target is allowed.
pub(crate) fn serialize_flexible_observer_click_target(
    effect: &str,
    context: &str,
    generation: u64,
    observer_author_id: &str,
    semantic_value: &str,
) -> Option<String> {
    let token = ClickCompletionToken {
        schema: CLICK_COMPLETION_SCHEMA.to_owned(),
        mode: ClickCompletionMode::Observer,
        effect: effect.to_owned(),
        context: context.to_owned(),
        generation,
        state: ClickCompletionState::Ready,
        observer_author_id: Some(observer_author_id.to_owned()),
        persistent_target: false,
        flexible_target: true,
        pending_target: None,
        semantic_value: Some(semantic_value.to_owned()),
        terminal_error: None,
        terminal_detail: None,
    };
    token
        .valid_observer_target_declaration()
        .then(|| serialize_click_completion_token(&token))
        .flatten()
}

/// Serialize a durable observer's state. Ready observers carry no pending target/value; Pending and
/// Applied observers carry the exact clicked author-id and semantic value at `generation + 1`.
pub(crate) fn serialize_observer_click_state(
    effect: &str,
    context: &str,
    generation: u64,
    state: ClickCompletionState,
    pending_target: Option<&str>,
    semantic_value: Option<&str>,
) -> Option<String> {
    let token = ClickCompletionToken {
        schema: CLICK_COMPLETION_SCHEMA.to_owned(),
        mode: ClickCompletionMode::Observer,
        effect: effect.to_owned(),
        context: context.to_owned(),
        generation,
        state,
        observer_author_id: None,
        persistent_target: false,
        flexible_target: false,
        pending_target: pending_target.map(str::to_owned),
        semantic_value: semantic_value.map(str::to_owned),
        terminal_error: None,
        terminal_detail: None,
    };
    token
        .valid_observer_state()
        .then(|| serialize_click_completion_token(&token))
        .flatten()
}

/// Serialize a successful observer terminal state with bounded action-specific proof detail. The
/// pre-click `semantic_value` remains unchanged and is still the causal binding; `terminal_detail`
/// can add persisted/readback identity that was unknowable before dispatch.
pub(crate) fn serialize_observer_click_applied(
    effect: &str,
    context: &str,
    generation: u64,
    pending_target: &str,
    semantic_value: &str,
    terminal_detail: &str,
) -> Option<String> {
    let token = ClickCompletionToken {
        schema: CLICK_COMPLETION_SCHEMA.to_owned(),
        mode: ClickCompletionMode::Observer,
        effect: effect.to_owned(),
        context: context.to_owned(),
        generation,
        state: ClickCompletionState::Applied,
        observer_author_id: None,
        persistent_target: false,
        flexible_target: false,
        pending_target: Some(pending_target.to_owned()),
        semantic_value: Some(semantic_value.to_owned()),
        terminal_error: None,
        terminal_detail: Some(terminal_detail.to_owned()),
    };
    token
        .valid_observer_state()
        .then(|| serialize_click_completion_token(&token))
        .flatten()
}

/// Serialize a terminal failure for an exact observer-backed click. The target declaration and the
/// pending observer state already fixed the target/context/generation/semantic tuple; this function
/// adds only a bounded typed error to that same tuple. ActionChannel publishes it as terminal
/// [`ActionReceiptStatus::Rejected`], never as a transport-wide/global failure.
pub(crate) fn serialize_observer_click_failure(
    effect: &str,
    context: &str,
    generation: u64,
    pending_target: &str,
    semantic_value: &str,
    terminal_error: &str,
    terminal_detail: Option<&str>,
) -> Option<String> {
    let token = ClickCompletionToken {
        schema: CLICK_COMPLETION_SCHEMA.to_owned(),
        mode: ClickCompletionMode::Observer,
        effect: effect.to_owned(),
        context: context.to_owned(),
        generation,
        state: ClickCompletionState::Failed,
        observer_author_id: None,
        persistent_target: false,
        flexible_target: false,
        pending_target: Some(pending_target.to_owned()),
        semantic_value: Some(semantic_value.to_owned()),
        terminal_error: Some(terminal_error.to_owned()),
        terminal_detail: terminal_detail.map(str::to_owned),
    };
    token
        .valid_observer_state()
        .then(|| serialize_click_completion_token(&token))
        .flatten()
}

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
    click_completion: Option<PendingClickCompletion>,
    set_value_completion: Option<PendingSetValueCompletion>,
}

#[derive(Debug, Clone)]
struct PendingSetValueCompletion {
    baseline: SetValueCompletionToken,
    observer_author_id: String,
    observer_node_id: u64,
    observer_role: String,
    observer_raw_baseline: String,
    dispatch_validated: bool,
}

#[derive(Debug, Clone)]
enum PendingClickCompletion {
    SameTarget {
        baseline: ClickCompletionToken,
    },
    Observer {
        declaration: ClickCompletionToken,
        observer_author_id: String,
        observer_node_id: u64,
        observer_role: String,
        observer_raw_baseline: String,
        dispatch_validated: bool,
    },
}

impl PendingAction {
    fn leases_author_id(&self, author_id: &str) -> bool {
        self.author_id == author_id
            || self
                .set_value_completion
                .as_ref()
                .is_some_and(|completion| completion.observer_author_id == author_id)
            || matches!(
                &self.click_completion,
                Some(PendingClickCompletion::Observer {
                    observer_author_id,
                    ..
                }) if observer_author_id == author_id
            )
    }
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

    /// Read-only attribution seam for app-owned completion observers. Returns an author id only when
    /// exactly one currently in-flight plain Click has reached Dispatched. Operator input has no
    /// channel row and concurrent model clicks are deliberately ambiguous, so both return `None`.
    pub(crate) fn unique_dispatched_click_author_id(&self) -> Option<String> {
        let mut matches = self.in_flight.iter().filter(|pending| {
            matches!(pending.action, UiAction::Click)
                && self.receipts.iter().any(|receipt| {
                    receipt.receipt_id == pending.outcome.receipt_id
                        && receipt.status == ActionReceiptStatus::Dispatched
                })
        });
        let author_id = matches.next()?.author_id.clone();
        matches.next().is_none().then_some(author_id)
    }

    /// Payload-aware counterpart used by app-owned action-specific completion observers. The
    /// returned payload is `None` for a plain click and preserves the exact closed payload string for
    /// `ClickWithPayload`. As with the legacy click-only seam, concurrent activations are deliberately
    /// ambiguous and therefore return `None`.
    pub(crate) fn unique_dispatched_activation(
        &self,
    ) -> Option<(String, Option<String>, Option<String>)> {
        let mut matches = self.in_flight.iter().filter(|pending| {
            is_click_activation(&pending.action)
                && self.receipts.iter().any(|receipt| {
                    receipt.receipt_id == pending.outcome.receipt_id
                        && receipt.status == ActionReceiptStatus::Dispatched
                })
        });
        let pending = matches.next()?;
        let activation = (
            pending.author_id.clone(),
            match &pending.action {
                UiAction::Click => None,
                UiAction::ClickWithPayload { payload } => Some(payload.clone()),
                _ => return None,
            },
            match &pending.click_completion {
                Some(PendingClickCompletion::Observer { declaration, .. }) => {
                    declaration.semantic_value.clone()
                }
                _ => None,
            },
        );
        matches.next().is_none().then_some(activation)
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
        let click_completion = pending_click_completion(snapshot, node, &action);
        let set_value_completion = pending_set_value_completion(snapshot, author_id, node, &action);
        let observer_author_id = match &click_completion {
            Some(PendingClickCompletion::Observer {
                observer_author_id, ..
            }) => Some(observer_author_id.as_str()),
            _ => None,
        };
        let set_value_observer_author_id = set_value_completion
            .as_ref()
            .map(|completion| completion.observer_author_id.as_str());
        let conflicting_lease =
            self.queue
                .iter()
                .chain(self.in_flight.iter())
                .find_map(|pending| {
                    if pending.leases_author_id(author_id) {
                        Some(author_id)
                    } else {
                        observer_author_id
                            .or(set_value_observer_author_id)
                            .filter(|observer| pending.leases_author_id(observer))
                    }
                });
        if let Some(conflicting_author_id) = conflicting_lease {
            return Err(ActionError::TargetBusy {
                author_id: conflicting_author_id.to_owned(),
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
            click_completion,
            set_value_completion,
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
            if let Err(reason) = revalidate_pending(&mut pending, fresh_snapshot) {
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
            if matches!(pending.action, UiAction::SetValue { .. })
                && pending.set_value_completion.is_some()
            {
                match acknowledge_set_value_completion(&pending, fresh_snapshot) {
                    SetValueCompletionAcknowledgement::StillPending { observed_value } => {
                        self.update_receipt(
                            pending.outcome.receipt_id,
                            ActionReceiptStatus::Dispatched,
                            observed_value,
                            None,
                        );
                        self.in_flight.push(pending);
                    }
                    SetValueCompletionAcknowledgement::Terminal {
                        status,
                        observed_value,
                        rejection,
                    } => self.update_receipt(
                        pending.outcome.receipt_id,
                        status,
                        observed_value,
                        rejection,
                    ),
                }
                continue;
            }
            if is_click_activation(&pending.action) && pending.click_completion.is_some() {
                match acknowledge_click_completion(&pending, fresh_snapshot) {
                    ClickCompletionAcknowledgement::Terminal {
                        status,
                        observed_value,
                        rejection,
                    } => self.update_receipt(
                        pending.outcome.receipt_id,
                        status,
                        observed_value,
                        rejection,
                    ),
                    ClickCompletionAcknowledgement::StillPending { observed_value } => {
                        self.update_receipt(
                            pending.outcome.receipt_id,
                            ActionReceiptStatus::Dispatched,
                            observed_value,
                            None,
                        );
                        self.in_flight.push(pending);
                    }
                }
                continue;
            }
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

enum ClickCompletionAcknowledgement {
    StillPending {
        observed_value: Option<String>,
    },
    Terminal {
        status: ActionReceiptStatus,
        observed_value: Option<String>,
        rejection: Option<String>,
    },
}

fn acknowledge_click_completion(
    pending: &PendingAction,
    snapshot: &UiTreeSnapshot,
) -> ClickCompletionAcknowledgement {
    let indeterminate =
        |observed_value: Option<String>, reason: &str| ClickCompletionAcknowledgement::Terminal {
            status: ActionReceiptStatus::Indeterminate,
            observed_value,
            rejection: Some(reason.to_owned()),
        };
    match pending
        .click_completion
        .as_ref()
        .expect("caller checks click-completion presence")
    {
        PendingClickCompletion::SameTarget { baseline } => {
            let Some(node) = snapshot.find_by_author_id(&pending.author_id) else {
                return indeterminate(
                    None,
                    "same-target completion target disappeared before acknowledgement",
                );
            };
            let observed = node.value.clone();
            if !post_render_target_identity_matches(pending, node) {
                return indeterminate(
                    observed,
                    "same-target completion identity or action capability drifted",
                );
            }
            let Some(applied) = node.value.as_deref().and_then(parse_click_completion_token) else {
                return indeterminate(observed, "same-target completion token is malformed");
            };
            let generation_matches = baseline
                .generation
                .checked_add(1)
                .is_some_and(|expected| applied.generation == expected);
            if baseline.state != ClickCompletionState::Pending
                && applied.valid_same_target()
                && applied.effect == baseline.effect
                && applied.context == baseline.context
                && generation_matches
            {
                match applied.state {
                    ClickCompletionState::Pending => ClickCompletionAcknowledgement::StillPending {
                        observed_value: observed,
                    },
                    ClickCompletionState::Applied => ClickCompletionAcknowledgement::Terminal {
                        status: ActionReceiptStatus::Applied,
                        observed_value: observed,
                        rejection: None,
                    },
                    ClickCompletionState::Failed => indeterminate(
                        observed,
                        "same-target completion cannot publish an observer failure",
                    ),
                    ClickCompletionState::Ready => indeterminate(
                        observed,
                        "same-target completion remained ready after dispatch",
                    ),
                }
            } else {
                indeterminate(
                    observed,
                    "same-target completion did not make the exact ready-to-applied generation transition",
                )
            }
        }
        PendingClickCompletion::Observer {
            declaration,
            observer_author_id,
            observer_node_id,
            observer_role,
            dispatch_validated,
            ..
        } => {
            if !dispatch_validated {
                return indeterminate(None, "observer identity/context was not valid at dispatch");
            }
            let Some(observer) = snapshot.find_unique_by_author_id(observer_author_id) else {
                return indeterminate(None, "observer disappeared before acknowledgement");
            };
            let observed = observer.value.clone();
            if observer.node_id != *observer_node_id || observer.role != *observer_role {
                return indeterminate(observed, "observer identity drifted before acknowledgement");
            }
            let Some(applied) = observer
                .value
                .as_deref()
                .and_then(parse_click_completion_token)
            else {
                return indeterminate(observed, "observer completion token is malformed");
            };
            let post_target = snapshot.find_unique_by_author_id(&pending.author_id);
            if declaration.persistent_target
                || (declaration.flexible_target && post_target.is_some())
            {
                let Some(post_target) = post_target else {
                    return indeterminate(
                        None,
                        "observer target disappeared before acknowledgement",
                    );
                };
                let pending_disabled_identity = declaration.persistent_target
                    && applied.state == ClickCompletionState::Pending
                    && post_target.node_id == pending.outcome.request.target.0
                    && post_target.role == pending.expected_role
                    && post_target.disabled;
                if !pending_disabled_identity
                    && !post_render_target_identity_matches(pending, post_target)
                {
                    return indeterminate(
                        post_target.value.clone(),
                        "persistent observer target identity or action capability drifted",
                    );
                }
                let post_declaration = post_target
                    .value
                    .as_deref()
                    .and_then(parse_click_completion_token);
                let declaration_advanced = post_declaration.as_ref().is_some_and(|post| {
                    post.valid_observer_target_declaration()
                        && post.persistent_target == declaration.persistent_target
                        && post.flexible_target == declaration.flexible_target
                        && post.effect == declaration.effect
                        && post.context == declaration.context
                        && post.observer_author_id == declaration.observer_author_id
                        && post.semantic_value == declaration.semantic_value
                        && declaration
                            .generation
                            .checked_add(1)
                            .is_some_and(|generation| post.generation == generation)
                });
                if !declaration_advanced {
                    return indeterminate(
                        post_target.value.clone(),
                        "persistent observer target declaration did not make the exact generation transition",
                    );
                }
            } else if !declaration.flexible_target && post_target.is_some() {
                return indeterminate(
                    None,
                    "observer completion requires the transient click target to disappear",
                );
            }
            let generation_matches = declaration
                .generation
                .checked_add(1)
                .is_some_and(|expected| applied.generation == expected);
            if applied.valid_observer_state()
                && applied.effect == declaration.effect
                && applied.context == declaration.context
                && generation_matches
                && applied.pending_target.as_deref() == Some(pending.author_id.as_str())
                && applied.semantic_value == declaration.semantic_value
            {
                if declaration.flexible_target
                    && ((applied.state == ClickCompletionState::Applied && post_target.is_some())
                        || (applied.state == ClickCompletionState::Failed && post_target.is_none()))
                {
                    return indeterminate(
                        observed,
                        "flexible observer terminal state does not match Retry target presence",
                    );
                }
                match applied.state {
                    ClickCompletionState::Pending => ClickCompletionAcknowledgement::StillPending {
                        observed_value: observed,
                    },
                    ClickCompletionState::Applied => ClickCompletionAcknowledgement::Terminal {
                        status: ActionReceiptStatus::Applied,
                        observed_value: observed,
                        rejection: None,
                    },
                    ClickCompletionState::Failed => ClickCompletionAcknowledgement::Terminal {
                        status: ActionReceiptStatus::Rejected,
                        observed_value: observed,
                        rejection: applied.terminal_error.clone(),
                    },
                    ClickCompletionState::Ready => indeterminate(
                        observed,
                        "observer completion remained ready after dispatch",
                    ),
                }
            } else {
                indeterminate(
                    observed,
                    "observer completion did not match the exact effect/context/target/value generation transition",
                )
            }
        }
    }
}

fn pending_click_completion(
    snapshot: &UiTreeSnapshot,
    target: &crate::accessibility::UiTreeNode,
    action: &UiAction,
) -> Option<PendingClickCompletion> {
    if !is_click_activation(action) {
        return None;
    }
    let token = parse_click_completion_token(target.value.as_deref()?)?;
    if token.valid_same_target() && token.state != ClickCompletionState::Pending {
        return Some(PendingClickCompletion::SameTarget { baseline: token });
    }
    if !token.valid_observer_target_declaration() {
        return None;
    }
    let observer_author_id = token.observer_author_id.clone()?;
    let observer = snapshot.find_by_author_id(&observer_author_id)?;
    let observer_raw_baseline = observer.value.clone()?;
    let observer_token = parse_click_completion_token(&observer_raw_baseline)?;
    if !observer_token.valid_observer_state()
        || observer_token.state == ClickCompletionState::Pending
        || observer_token.effect != token.effect
        || observer_token.context != token.context
        || observer_token.generation != token.generation
    {
        return None;
    }
    Some(PendingClickCompletion::Observer {
        declaration: token,
        observer_author_id,
        observer_node_id: observer.node_id,
        observer_role: observer.role.clone(),
        observer_raw_baseline,
        dispatch_validated: false,
    })
}

fn pending_set_value_completion(
    snapshot: &UiTreeSnapshot,
    target_author_id: &str,
    target: &crate::accessibility::UiTreeNode,
    action: &UiAction,
) -> Option<PendingSetValueCompletion> {
    let UiAction::SetValue { text } = action else {
        return None;
    };
    if observed_value_matches(target_author_id, &target.value, text) {
        return None;
    }
    let observer_author_id = set_value_completion_author_id(target_author_id);
    let observer = snapshot.find_unique_by_author_id(&observer_author_id)?;
    let observer_raw_baseline = observer.value.clone()?;
    let baseline = parse_set_value_completion(&observer_raw_baseline)?;
    if baseline.target != target_author_id {
        return None;
    }
    Some(PendingSetValueCompletion {
        baseline,
        observer_author_id,
        observer_node_id: observer.node_id,
        observer_role: observer.role.clone(),
        observer_raw_baseline,
        dispatch_validated: false,
    })
}

enum SetValueCompletionAcknowledgement {
    StillPending {
        observed_value: Option<String>,
    },
    Terminal {
        status: ActionReceiptStatus,
        observed_value: Option<String>,
        rejection: Option<String>,
    },
}

fn acknowledge_set_value_completion(
    pending: &PendingAction,
    snapshot: &UiTreeSnapshot,
) -> SetValueCompletionAcknowledgement {
    let completion = pending
        .set_value_completion
        .as_ref()
        .expect("caller checks SetValue completion presence");
    let UiAction::SetValue { text } = &pending.action else {
        unreachable!("SetValue completion is registered only for SetValue")
    };
    let terminal =
        |status, observed_value, rejection: &str| SetValueCompletionAcknowledgement::Terminal {
            status,
            observed_value,
            rejection: Some(rejection.to_owned()),
        };
    if !completion.dispatch_validated {
        return terminal(
            ActionReceiptStatus::Indeterminate,
            None,
            "SetValue completion observer changed before dispatch",
        );
    }
    let Some(target) = snapshot.find_unique_by_author_id(&pending.author_id) else {
        return terminal(
            ActionReceiptStatus::Rejected,
            None,
            "SetValue target disappeared or became ambiguous before acknowledgement",
        );
    };
    if !post_render_target_identity_matches(pending, target) {
        return terminal(
            ActionReceiptStatus::Rejected,
            target.value.clone(),
            "SetValue target identity or action capability drifted",
        );
    }
    let Some(observer) = snapshot.find_unique_by_author_id(&completion.observer_author_id) else {
        return terminal(
            ActionReceiptStatus::Indeterminate,
            target.value.clone(),
            "SetValue completion observer disappeared or became ambiguous",
        );
    };
    if observer.node_id != completion.observer_node_id || observer.role != completion.observer_role
    {
        return terminal(
            ActionReceiptStatus::Indeterminate,
            target.value.clone(),
            "SetValue completion observer identity drifted",
        );
    }
    let Some(observed_completion) = observer
        .value
        .as_deref()
        .and_then(parse_set_value_completion)
    else {
        return terminal(
            ActionReceiptStatus::Indeterminate,
            target.value.clone(),
            "SetValue completion observer token is malformed",
        );
    };
    if observed_completion == completion.baseline {
        return SetValueCompletionAcknowledgement::StillPending {
            observed_value: target.value.clone(),
        };
    }
    let generation_matches = completion
        .baseline
        .generation
        .checked_add(1)
        .is_some_and(|generation| observed_completion.generation == generation);
    if observed_completion.target == pending.author_id
        && observed_completion.context == completion.baseline.context
        && generation_matches
        && observed_completion.applied_value.as_deref() == Some(text.as_str())
        && observed_value_matches(&pending.author_id, &target.value, text)
    {
        return SetValueCompletionAcknowledgement::Terminal {
            status: ActionReceiptStatus::Applied,
            observed_value: target.value.clone(),
            rejection: None,
        };
    }
    terminal(
        ActionReceiptStatus::Indeterminate,
        target.value.clone(),
        "SetValue completion did not make the exact target/context/generation/value transition",
    )
}

fn is_click_activation(action: &UiAction) -> bool {
    matches!(action, UiAction::Click | UiAction::ClickWithPayload { .. })
}

fn observer_dispatch_identity_matches(
    snapshot: &UiTreeSnapshot,
    declaration: &ClickCompletionToken,
    observer_author_id: &str,
    observer_node_id: u64,
    observer_role: &str,
    observer_raw_baseline: &str,
) -> bool {
    let Some(observer) = snapshot.find_by_author_id(observer_author_id) else {
        return false;
    };
    if observer.node_id != observer_node_id
        || observer.role != observer_role
        || observer.value.as_deref() != Some(observer_raw_baseline)
    {
        return false;
    }
    let Some(token) = observer
        .value
        .as_deref()
        .and_then(parse_click_completion_token)
    else {
        return false;
    };
    token.valid_observer_state()
        && token.state != ClickCompletionState::Pending
        && token.effect == declaration.effect
        && token.context == declaration.context
        && token.generation == declaration.generation
}

fn revalidate_pending(
    pending: &mut PendingAction,
    snapshot: &UiTreeSnapshot,
) -> Result<(), String> {
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
    if let Some(PendingClickCompletion::Observer {
        declaration,
        observer_author_id,
        observer_node_id,
        observer_role,
        observer_raw_baseline,
        dispatch_validated,
    }) = &mut pending.click_completion
    {
        *dispatch_validated = observer_dispatch_identity_matches(
            snapshot,
            declaration,
            observer_author_id,
            *observer_node_id,
            observer_role,
            observer_raw_baseline,
        );
    }
    if let Some(completion) = &mut pending.set_value_completion {
        let observer = snapshot.find_unique_by_author_id(&completion.observer_author_id);
        completion.dispatch_validated = observer.is_some_and(|observer| {
            observer.node_id == completion.observer_node_id
                && observer.role == completion.observer_role
                && observer.value.as_deref() == Some(completion.observer_raw_baseline.as_str())
                && observer
                    .value
                    .as_deref()
                    .and_then(parse_set_value_completion)
                    .as_ref()
                    == Some(&completion.baseline)
        });
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

    fn clickable_node(author_id: &str, node_id: u64, value: Option<String>) -> UiTreeNode {
        UiTreeNode {
            id: author_id.to_owned(),
            author_id: Some(author_id.to_owned()),
            node_id,
            role: "Button".to_owned(),
            label: Some(author_id.to_owned()),
            value,
            disabled: false,
            actions: vec!["Click".to_owned(), "Focus".to_owned()],
            bounds: None,
            children: Vec::new(),
        }
    }

    fn observer_node(author_id: &str, node_id: u64, value: String) -> UiTreeNode {
        UiTreeNode {
            id: author_id.to_owned(),
            author_id: Some(author_id.to_owned()),
            node_id,
            role: "Document".to_owned(),
            label: Some("completion observer".to_owned()),
            value: Some(value),
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children: Vec::new(),
        }
    }

    fn top_level_node_mut<'a>(
        snapshot: &'a mut UiTreeSnapshot,
        author_id: &str,
    ) -> &'a mut UiTreeNode {
        snapshot
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(author_id))
            .expect("top-level fixture node exists")
    }

    fn terminal_receipt(channel: &mut ActionChannel, receipt_id: u64) -> ActionReceipt {
        channel
            .receipts()
            .into_iter()
            .find(|receipt| receipt.receipt_id == receipt_id)
            .expect("receipt exists")
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
    fn same_target_click_completion_requires_exact_ready_to_applied_transition() {
        let mut snapshot = fixture_snapshot();
        snapshot.root.children[0].value = serialize_same_target_click_completion(
            "image-modal-open",
            "workspace-a/image-1",
            7,
            ClickCompletionState::Ready,
        );
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, "btn", UiAction::Click)
            .expect("queue tokenized click");
        assert_eq!(channel.drain_revalidated_into_events(&snapshot).len(), 1);

        let mut applied = snapshot.clone();
        applied.root.children[0].value = serialize_same_target_click_completion(
            "image-modal-open",
            "workspace-a/image-1",
            8,
            ClickCompletionState::Applied,
        );
        channel.acknowledge_after_render(&applied);
        let receipt = terminal_receipt(&mut channel, outcome.receipt_id);
        assert_eq!(receipt.status, ActionReceiptStatus::Applied);
        assert!(receipt.rejection.is_none());

        let repeated = channel
            .enqueue(&applied, "btn", UiAction::Click)
            .expect("a settled Applied token is a valid repeated-click baseline");
        channel.drain_revalidated_into_events(&applied);
        let mut applied_again = applied;
        applied_again.root.children[0].value = serialize_same_target_click_completion(
            "image-modal-open",
            "workspace-a/image-1",
            9,
            ClickCompletionState::Applied,
        );
        channel.acknowledge_after_render(&applied_again);
        assert_eq!(
            terminal_receipt(&mut channel, repeated.receipt_id).status,
            ActionReceiptStatus::Applied,
            "Applied(N) -> Applied(N+1) supports persistent controls"
        );
    }

    #[test]
    fn same_target_pending_remains_dispatched_until_applied_or_timeout() {
        let mut snapshot = fixture_snapshot();
        snapshot.root.children[0].value = serialize_same_target_click_completion(
            "graph.relayout",
            "workspace-a/graph-main",
            11,
            ClickCompletionState::Ready,
        );
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, "btn", UiAction::Click)
            .expect("queue multi-frame click");
        channel.drain_revalidated_into_events(&snapshot);

        let mut pending = snapshot.clone();
        pending.root.children[0].value = serialize_same_target_click_completion(
            "graph.relayout",
            "workspace-a/graph-main",
            12,
            ClickCompletionState::Pending,
        );
        channel.acknowledge_after_render(&pending);
        channel.acknowledge_after_render(&pending);
        let receipt = terminal_receipt(&mut channel, outcome.receipt_id);
        assert_eq!(receipt.status, ActionReceiptStatus::Dispatched);
        assert_eq!(channel.in_flight.len(), 1);

        let pending_for_timeout = pending.clone();
        let mut applied = pending;
        applied.root.children[0].value = serialize_same_target_click_completion(
            "graph.relayout",
            "workspace-a/graph-main",
            12,
            ClickCompletionState::Applied,
        );
        channel.acknowledge_after_render(&applied);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Applied
        );
        assert!(channel.in_flight.is_empty());

        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, "btn", UiAction::Click)
            .expect("queue expiring multi-frame click");
        channel.drain_revalidated_into_events(&snapshot);
        channel.acknowledge_after_render(&pending_for_timeout);
        channel.in_flight[0].enqueued_at =
            Instant::now() - ACTION_LEASE_TIMEOUT - Duration::from_millis(1);
        let receipt = terminal_receipt(&mut channel, outcome.receipt_id);
        assert_eq!(receipt.status, ActionReceiptStatus::Indeterminate);
        assert!(receipt
            .rejection
            .as_deref()
            .is_some_and(|reason| reason.contains("deadline elapsed")));
    }

    #[test]
    fn same_target_token_drift_fails_closed_and_payload_click_can_apply() {
        let mut baseline = fixture_snapshot();
        baseline.root.children[0].value = serialize_same_target_click_completion(
            "image-modal-open",
            "workspace-a/image-1",
            3,
            ClickCompletionState::Ready,
        );
        for (label, post_value) in [
            (
                "generation jump",
                serialize_same_target_click_completion(
                    "image-modal-open",
                    "workspace-a/image-1",
                    5,
                    ClickCompletionState::Applied,
                ),
            ),
            (
                "context drift",
                serialize_same_target_click_completion(
                    "image-modal-open",
                    "workspace-b/image-1",
                    4,
                    ClickCompletionState::Applied,
                ),
            ),
            (
                "effect drift",
                serialize_same_target_click_completion(
                    "different-effect",
                    "workspace-a/image-1",
                    4,
                    ClickCompletionState::Applied,
                ),
            ),
            ("malformed", Some("{not-json".to_owned())),
        ] {
            let mut channel = ActionChannel::new();
            let outcome = channel
                .enqueue(&baseline, "btn", UiAction::Click)
                .expect("queue tokenized click");
            channel.drain_revalidated_into_events(&baseline);
            let mut post = baseline.clone();
            post.root.children[0].value = post_value;
            channel.acknowledge_after_render(&post);
            assert_eq!(
                terminal_receipt(&mut channel, outcome.receipt_id).status,
                ActionReceiptStatus::Indeterminate,
                "{label}"
            );
        }

        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(
                &baseline,
                "btn",
                UiAction::ClickWithPayload {
                    payload: "ignored".to_owned(),
                },
            )
            .expect("payload click remains dispatchable");
        channel.drain_revalidated_into_events(&baseline);
        let mut post = baseline;
        post.root.children[0].value = serialize_same_target_click_completion(
            "image-modal-open",
            "workspace-a/image-1",
            4,
            ClickCompletionState::Applied,
        );
        channel.acknowledge_after_render(&post);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Applied,
            "ClickWithPayload uses the same exact ready-to-applied completion transition"
        );
    }

    #[test]
    fn observer_click_completion_binds_target_observer_value_and_shared_lease() {
        let context = "file:///workspace/main.cpp@2";
        let effect = "code-completion-accept";
        let observer_author = "code-editor.completion-observer";
        let semantic = "add_numbers|add_numbers(int,int)";
        let observer_ready = serialize_observer_click_state(
            effect,
            context,
            20,
            ClickCompletionState::Ready,
            None,
            None,
        )
        .unwrap();
        let target_value =
            serialize_observer_click_target(effect, context, 20, observer_author, semantic)
                .unwrap();
        let mut snapshot = fixture_snapshot();
        snapshot.root.children.push(clickable_node(
            "completion-item-a",
            30,
            Some(target_value.clone()),
        ));
        snapshot.root.children.push(clickable_node(
            "completion-item-b",
            31,
            Some(
                serialize_observer_click_target(
                    effect,
                    context,
                    20,
                    observer_author,
                    "subtract_numbers|subtract_numbers(int,int)",
                )
                .unwrap(),
            ),
        ));
        snapshot
            .root
            .children
            .push(observer_node(observer_author, 40, observer_ready));

        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, "completion-item-a", UiAction::Click)
            .expect("queue observer-backed target");
        assert_eq!(
            channel
                .enqueue(&snapshot, "completion-item-b", UiAction::Click)
                .unwrap_err(),
            ActionError::TargetBusy {
                author_id: observer_author.to_owned()
            }
        );
        channel.drain_revalidated_into_events(&snapshot);

        let mut pending = snapshot.clone();
        pending
            .root
            .children
            .retain(|node| node.author_id.as_deref() != Some("completion-item-a"));
        let observer = pending
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer_author))
            .unwrap();
        observer.value = serialize_observer_click_state(
            effect,
            context,
            21,
            ClickCompletionState::Pending,
            Some("completion-item-a"),
            Some(semantic),
        );
        channel.acknowledge_after_render(&pending);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Dispatched
        );

        let observer = pending
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer_author))
            .unwrap();
        observer.value = serialize_observer_click_applied(
            effect,
            context,
            21,
            "completion-item-a",
            semantic,
            "{\"result\":\"accepted\"}",
        );
        channel.acknowledge_after_render(&pending);
        let receipt = terminal_receipt(&mut channel, outcome.receipt_id);
        assert_eq!(receipt.status, ActionReceiptStatus::Applied);
        assert!(receipt
            .observed_value
            .as_deref()
            .is_some_and(|value| value.contains("terminal_detail")));
    }

    #[test]
    fn unique_dispatched_click_attribution_excludes_operator_and_concurrent_actions() {
        let mut snapshot = fixture_snapshot();
        snapshot
            .root
            .children
            .push(clickable_node("other-button", 99, None));

        let operator_only = ActionChannel::new();
        assert_eq!(
            operator_only.unique_dispatched_click_author_id(),
            None,
            "operator UI activity has no model action-channel attribution"
        );

        let mut one = ActionChannel::new();
        one.enqueue(&snapshot, "btn", UiAction::Click).unwrap();
        one.drain_revalidated_into_events(&snapshot);
        assert_eq!(
            one.unique_dispatched_click_author_id().as_deref(),
            Some("btn")
        );

        let mut concurrent = ActionChannel::new();
        concurrent
            .enqueue(&snapshot, "btn", UiAction::Click)
            .unwrap();
        concurrent
            .enqueue(&snapshot, "other-button", UiAction::Click)
            .unwrap();
        concurrent.drain_revalidated_into_events(&snapshot);
        assert_eq!(
            concurrent.unique_dispatched_click_author_id(),
            None,
            "two concurrent dispatched clicks are ambiguous and cannot advance an app observer"
        );
    }

    #[test]
    fn unique_dispatched_activation_preserves_exact_payload_and_plain_click_behavior() {
        let mut snapshot = fixture_snapshot();
        snapshot
            .root
            .children
            .push(clickable_node("other-button", 99, None));

        let mut payload = ActionChannel::new();
        payload
            .enqueue(
                &snapshot,
                "btn",
                UiAction::ClickWithPayload {
                    payload: r#"{"kind":"slash_command","command_id":"code-ref"}"#.to_owned(),
                },
            )
            .unwrap();
        payload.drain_revalidated_into_events(&snapshot);
        assert_eq!(
            payload.unique_dispatched_activation(),
            Some((
                "btn".to_owned(),
                Some(r#"{"kind":"slash_command","command_id":"code-ref"}"#.to_owned()),
                None,
            ))
        );
        assert_eq!(
            payload.unique_dispatched_click_author_id(),
            None,
            "the MT-033 plain-click attribution seam remains payload-blind"
        );

        let mut plain = ActionChannel::new();
        plain.enqueue(&snapshot, "btn", UiAction::Click).unwrap();
        plain.drain_revalidated_into_events(&snapshot);
        assert_eq!(
            plain.unique_dispatched_activation(),
            Some(("btn".to_owned(), None, None))
        );
        assert_eq!(
            plain.unique_dispatched_click_author_id().as_deref(),
            Some("btn")
        );

        let mut concurrent = ActionChannel::new();
        concurrent
            .enqueue(&snapshot, "btn", UiAction::Click)
            .unwrap();
        concurrent
            .enqueue(
                &snapshot,
                "other-button",
                UiAction::ClickWithPayload {
                    payload: "{}".to_owned(),
                },
            )
            .unwrap();
        concurrent.drain_revalidated_into_events(&snapshot);
        assert_eq!(concurrent.unique_dispatched_activation(), None);
    }

    #[test]
    fn persistent_observer_requires_exact_stable_target_and_exact_observer_transition() {
        #[derive(Clone, Copy)]
        enum Drift {
            None,
            TargetDisappears,
            Node,
            Role,
            Action,
            RawDeclaration,
            Semantic,
            Generation,
        }

        let effect = "atelier-hslink-insert";
        let context = "workspace-a/doc-a";
        let observer_author = "mt033.argus-action-completion";
        let target_author = "atelier-item-item-a";
        let semantic = "item-a|media|doc-a|revision-4|hash-before";

        for drift in [
            Drift::None,
            Drift::TargetDisappears,
            Drift::Node,
            Drift::Role,
            Drift::Action,
            Drift::RawDeclaration,
            Drift::Semantic,
            Drift::Generation,
        ] {
            let declaration = serialize_persistent_observer_click_target(
                effect,
                context,
                4,
                observer_author,
                semantic,
            )
            .unwrap();
            let ready = serialize_observer_click_state(
                effect,
                context,
                4,
                ClickCompletionState::Ready,
                None,
                None,
            )
            .unwrap();
            let mut snapshot = fixture_snapshot();
            snapshot.root.children.push(clickable_node(
                target_author,
                300,
                Some(declaration.clone()),
            ));
            snapshot
                .root
                .children
                .push(observer_node(observer_author, 301, ready));

            let mut channel = ActionChannel::new();
            let outcome = channel
                .enqueue(&snapshot, target_author, UiAction::Click)
                .expect("queue persistent observer target");
            channel.drain_revalidated_into_events(&snapshot);
            let mut post = snapshot;
            if matches!(drift, Drift::TargetDisappears) {
                post.root
                    .children
                    .retain(|node| node.author_id.as_deref() != Some(target_author));
            } else {
                let target = post
                    .root
                    .children
                    .iter_mut()
                    .find(|node| node.author_id.as_deref() == Some(target_author))
                    .unwrap();
                target.value = serialize_persistent_observer_click_target(
                    effect,
                    context,
                    5,
                    observer_author,
                    semantic,
                );
                match drift {
                    Drift::Node => target.node_id += 1,
                    Drift::Role => target.role = "ListItem".to_owned(),
                    Drift::Action => target.actions.clear(),
                    Drift::RawDeclaration => {
                        target.value = serialize_persistent_observer_click_target(
                            effect,
                            context,
                            5,
                            observer_author,
                            "item-b|media|doc-a|revision-4|hash-before",
                        )
                    }
                    _ => {}
                }
            }
            let post_semantic = if matches!(drift, Drift::Semantic) {
                "item-b|media|doc-a|revision-4|hash-before"
            } else {
                semantic
            };
            let post_generation = if matches!(drift, Drift::Generation) {
                6
            } else {
                5
            };
            post.root
                .children
                .iter_mut()
                .find(|node| node.author_id.as_deref() == Some(observer_author))
                .unwrap()
                .value = serialize_observer_click_applied(
                effect,
                context,
                post_generation,
                target_author,
                post_semantic,
                "{\"ref_kind\":\"media\",\"ref_value\":\"item-a\",\"revision\":5}",
            );
            channel.acknowledge_after_render(&post);
            let status = terminal_receipt(&mut channel, outcome.receipt_id).status;
            assert_eq!(
                status,
                if matches!(drift, Drift::None) {
                    ActionReceiptStatus::Applied
                } else {
                    ActionReceiptStatus::Indeterminate
                }
            );
        }

        // The pre-existing transient declaration keeps the disappearance rule; a mounted target does
        // not silently inherit persistent semantics from the new opt-in mode.
        let declaration =
            serialize_observer_click_target(effect, context, 4, observer_author, semantic).unwrap();
        let ready = serialize_observer_click_state(
            effect,
            context,
            4,
            ClickCompletionState::Ready,
            None,
            None,
        )
        .unwrap();
        let mut snapshot = fixture_snapshot();
        snapshot
            .root
            .children
            .push(clickable_node(target_author, 310, Some(declaration)));
        snapshot
            .root
            .children
            .push(observer_node(observer_author, 311, ready));
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, target_author, UiAction::Click)
            .unwrap();
        channel.drain_revalidated_into_events(&snapshot);
        snapshot
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer_author))
            .unwrap()
            .value = serialize_observer_click_applied(
            effect,
            context,
            5,
            target_author,
            semantic,
            "{\"result\":\"applied\"}",
        );
        channel.acknowledge_after_render(&snapshot);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate
        );
    }

    #[test]
    fn flexible_observer_accepts_absent_target_with_exact_applied_observer() {
        let (effect, context, observer, target, semantic) = (
            "graph-open-node",
            "workspace-a/graph-global",
            "mt042.graph-open-completion",
            "graph.retry",
            "retry|workspace-a|generation-8",
        );
        let mut snapshot = fixture_snapshot();
        snapshot.root.children.push(clickable_node(
            target,
            400,
            serialize_flexible_observer_click_target(effect, context, 8, observer, semantic),
        ));
        snapshot.root.children.push(observer_node(
            observer,
            401,
            serialize_observer_click_state(
                effect,
                context,
                8,
                ClickCompletionState::Ready,
                None,
                None,
            )
            .unwrap(),
        ));
        let mut channel = ActionChannel::new();
        let outcome = channel.enqueue(&snapshot, target, UiAction::Click).unwrap();
        channel.drain_revalidated_into_events(&snapshot);
        snapshot
            .root
            .children
            .retain(|node| node.author_id.as_deref() != Some(target));
        snapshot
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer))
            .unwrap()
            .value = serialize_observer_click_applied(
            effect,
            context,
            9,
            target,
            semantic,
            "{\"graph_recovered\":true}",
        );
        channel.acknowledge_after_render(&snapshot);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn flexible_observer_accepts_exact_advanced_target_with_typed_failure() {
        let (effect, context, observer, target, semantic) = (
            "graph-open-node",
            "workspace-a/graph-global",
            "mt042.graph-open-completion",
            "graph.retry",
            "retry|workspace-a|generation-8",
        );
        let mut snapshot = fixture_snapshot();
        snapshot.root.children.push(clickable_node(
            target,
            410,
            serialize_flexible_observer_click_target(effect, context, 8, observer, semantic),
        ));
        snapshot.root.children.push(observer_node(
            observer,
            411,
            serialize_observer_click_state(
                effect,
                context,
                8,
                ClickCompletionState::Ready,
                None,
                None,
            )
            .unwrap(),
        ));
        let mut channel = ActionChannel::new();
        let outcome = channel.enqueue(&snapshot, target, UiAction::Click).unwrap();
        channel.drain_revalidated_into_events(&snapshot);
        top_level_node_mut(&mut snapshot, target).value =
            serialize_flexible_observer_click_target(effect, context, 9, observer, semantic);
        top_level_node_mut(&mut snapshot, observer).value = serialize_observer_click_failure(
            effect,
            context,
            9,
            target,
            semantic,
            "graph_retry_transport: database unavailable",
            Some("{\"recovery_request_generation\":9}"),
        );
        channel.acknowledge_after_render(&snapshot);
        let receipt = terminal_receipt(&mut channel, outcome.receipt_id);
        assert_eq!(receipt.status, ActionReceiptStatus::Rejected);
        assert_eq!(
            receipt.rejection.as_deref(),
            Some("graph_retry_transport: database unavailable")
        );
    }

    #[test]
    fn flexible_observer_rejects_terminal_state_target_presence_counterfactuals() {
        let (effect, context, observer, target, semantic) = (
            "graph-open-node",
            "workspace-a/graph-global",
            "mt042.graph-open-completion",
            "graph.retry",
            "retry|workspace-a|generation-8",
        );
        for (target_present, terminal_state) in [
            (true, ClickCompletionState::Applied),
            (false, ClickCompletionState::Failed),
        ] {
            let mut snapshot = fixture_snapshot();
            snapshot.root.children.push(clickable_node(
                target,
                415,
                serialize_flexible_observer_click_target(effect, context, 8, observer, semantic),
            ));
            snapshot.root.children.push(observer_node(
                observer,
                416,
                serialize_observer_click_state(
                    effect,
                    context,
                    8,
                    ClickCompletionState::Ready,
                    None,
                    None,
                )
                .unwrap(),
            ));
            let mut channel = ActionChannel::new();
            let outcome = channel.enqueue(&snapshot, target, UiAction::Click).unwrap();
            channel.drain_revalidated_into_events(&snapshot);
            if target_present {
                top_level_node_mut(&mut snapshot, target).value =
                    serialize_flexible_observer_click_target(
                        effect, context, 9, observer, semantic,
                    );
            } else {
                snapshot
                    .root
                    .children
                    .retain(|node| node.author_id.as_deref() != Some(target));
            }
            top_level_node_mut(&mut snapshot, observer).value = match terminal_state {
                ClickCompletionState::Applied => serialize_observer_click_applied(
                    effect,
                    context,
                    9,
                    target,
                    semantic,
                    "{\"graph_recovered\":true}",
                ),
                ClickCompletionState::Failed => serialize_observer_click_failure(
                    effect,
                    context,
                    9,
                    target,
                    semantic,
                    "graph_retry_transport: database unavailable",
                    Some("{\"recovery_request_generation\":9}"),
                ),
                _ => unreachable!(),
            };
            channel.acknowledge_after_render(&snapshot);
            assert_eq!(
                terminal_receipt(&mut channel, outcome.receipt_id).status,
                ActionReceiptStatus::Indeterminate,
                "target_present={target_present} terminal_state={terminal_state:?}"
            );
        }
    }

    #[test]
    fn flexible_observer_present_target_drift_matrix_fails_closed() {
        #[derive(Clone, Copy)]
        enum Drift {
            Node,
            Role,
            Action,
            Declaration,
        }
        let (effect, context, observer, target, semantic) = (
            "graph-open-node",
            "workspace-a/graph-global",
            "mt042.graph-open-completion",
            "graph.retry",
            "retry|workspace-a|generation-8",
        );
        for drift in [Drift::Node, Drift::Role, Drift::Action, Drift::Declaration] {
            let mut snapshot = fixture_snapshot();
            snapshot.root.children.push(clickable_node(
                target,
                420,
                serialize_flexible_observer_click_target(effect, context, 8, observer, semantic),
            ));
            snapshot.root.children.push(observer_node(
                observer,
                421,
                serialize_observer_click_state(
                    effect,
                    context,
                    8,
                    ClickCompletionState::Ready,
                    None,
                    None,
                )
                .unwrap(),
            ));
            let mut channel = ActionChannel::new();
            let outcome = channel.enqueue(&snapshot, target, UiAction::Click).unwrap();
            channel.drain_revalidated_into_events(&snapshot);
            let post_target = top_level_node_mut(&mut snapshot, target);
            post_target.value =
                serialize_flexible_observer_click_target(effect, context, 9, observer, semantic);
            match drift {
                Drift::Node => post_target.node_id += 1,
                Drift::Role => post_target.role = "ListItem".to_owned(),
                Drift::Action => post_target.actions.clear(),
                Drift::Declaration => {
                    post_target.value = serialize_flexible_observer_click_target(
                        effect,
                        context,
                        9,
                        observer,
                        "retry|workspace-b|generation-8",
                    )
                }
            }
            top_level_node_mut(&mut snapshot, observer).value = serialize_observer_click_applied(
                effect,
                context,
                9,
                target,
                semantic,
                "{\"graph_recovered\":true}",
            );
            channel.acknowledge_after_render(&snapshot);
            assert_eq!(
                terminal_receipt(&mut channel, outcome.receipt_id).status,
                ActionReceiptStatus::Indeterminate
            );
        }
    }

    #[test]
    fn flexible_observer_present_target_requires_declaration_generation_advance() {
        let (effect, context, observer, target, semantic) = (
            "graph-open-node",
            "workspace-a/graph-global",
            "mt042.graph-open-completion",
            "graph.retry",
            "retry|workspace-a|generation-8",
        );
        let mut snapshot = fixture_snapshot();
        snapshot.root.children.push(clickable_node(
            target,
            430,
            serialize_flexible_observer_click_target(effect, context, 8, observer, semantic),
        ));
        snapshot.root.children.push(observer_node(
            observer,
            431,
            serialize_observer_click_state(
                effect,
                context,
                8,
                ClickCompletionState::Ready,
                None,
                None,
            )
            .unwrap(),
        ));
        let mut channel = ActionChannel::new();
        let outcome = channel.enqueue(&snapshot, target, UiAction::Click).unwrap();
        channel.drain_revalidated_into_events(&snapshot);
        top_level_node_mut(&mut snapshot, observer).value = serialize_observer_click_applied(
            effect,
            context,
            9,
            target,
            semantic,
            "{\"graph_recovered\":true}",
        );
        channel.acknowledge_after_render(&snapshot);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate
        );
    }

    #[test]
    fn flexible_observer_identity_mismatch_is_indeterminate() {
        let (effect, context, observer, target, semantic) = (
            "graph-open-node",
            "workspace-a/graph-global",
            "mt042.graph-open-completion",
            "graph.retry",
            "retry|workspace-a|generation-8",
        );
        let mut snapshot = fixture_snapshot();
        snapshot.root.children.push(clickable_node(
            target,
            440,
            serialize_flexible_observer_click_target(effect, context, 8, observer, semantic),
        ));
        snapshot.root.children.push(observer_node(
            observer,
            441,
            serialize_observer_click_state(
                effect,
                context,
                8,
                ClickCompletionState::Ready,
                None,
                None,
            )
            .unwrap(),
        ));
        let mut channel = ActionChannel::new();
        let outcome = channel.enqueue(&snapshot, target, UiAction::Click).unwrap();
        channel.drain_revalidated_into_events(&snapshot);
        top_level_node_mut(&mut snapshot, target).value =
            serialize_flexible_observer_click_target(effect, context, 9, observer, semantic);
        let post_observer = top_level_node_mut(&mut snapshot, observer);
        post_observer.node_id += 1;
        post_observer.value = serialize_observer_click_applied(
            effect,
            context,
            9,
            target,
            semantic,
            "{\"graph_recovered\":true}",
        );
        channel.acknowledge_after_render(&snapshot);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate
        );
    }

    #[test]
    fn observer_target_modes_remain_mutually_exclusive_regressions() {
        let (effect, context, observer, semantic) = ("effect", "context", "observer", "semantic");
        let transient = serialize_observer_click_target(effect, context, 1, observer, semantic)
            .and_then(|value| parse_click_completion_token(&value))
            .unwrap();
        let persistent =
            serialize_persistent_observer_click_target(effect, context, 1, observer, semantic)
                .and_then(|value| parse_click_completion_token(&value))
                .unwrap();
        let flexible =
            serialize_flexible_observer_click_target(effect, context, 1, observer, semantic)
                .and_then(|value| parse_click_completion_token(&value))
                .unwrap();
        assert!(!transient.persistent_target && !transient.flexible_target);
        assert!(persistent.persistent_target && !persistent.flexible_target);
        assert!(!flexible.persistent_target && flexible.flexible_target);
        let mut illegal = flexible;
        illegal.persistent_target = true;
        assert!(!illegal.valid_observer_target_declaration());
    }

    fn assert_terminal_observer_baseline_advances(baseline: String) {
        let effect = "wiki-overlay-action";
        let context = "workspace-a/projection-a/pane-4/source-revision-a";
        let observer_author = "wiki.action-status.projection-a";
        let target_author = "wiki.cancel.projection-a";
        let semantic = "cancel|draft-8|source-revision-a";
        let declaration =
            serialize_observer_click_target(effect, context, 20, observer_author, semantic)
                .unwrap();
        let mut snapshot = fixture_snapshot();
        snapshot
            .root
            .children
            .push(clickable_node(target_author, 70, Some(declaration)));
        snapshot
            .root
            .children
            .push(observer_node(observer_author, 71, baseline));

        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, target_author, UiAction::Click)
            .expect("queue next action from durable terminal baseline");
        channel.drain_revalidated_into_events(&snapshot);

        let mut applied = snapshot;
        applied
            .root
            .children
            .retain(|node| node.author_id.as_deref() != Some(target_author));
        applied
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer_author))
            .unwrap()
            .value = serialize_observer_click_applied(
            effect,
            context,
            21,
            target_author,
            semantic,
            "{\"action\":\"cancel\",\"action_generation\":21}",
        );
        channel.acknowledge_after_render(&applied);

        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn observer_applied_baseline_can_advance_to_next_exact_applied_generation() {
        let baseline = serialize_observer_click_applied(
            "wiki-overlay-action",
            "workspace-a/projection-a/pane-4/source-revision-a",
            20,
            "wiki.edit.projection-a",
            "edit|draft-8|source-revision-a",
            "{\"action\":\"edit\",\"action_generation\":20}",
        )
        .unwrap();
        assert_terminal_observer_baseline_advances(baseline);
    }

    #[test]
    fn observer_failed_baseline_can_advance_to_next_exact_applied_generation() {
        let baseline = serialize_observer_click_failure(
            "wiki-overlay-action",
            "workspace-a/projection-a/pane-4/source-revision-a",
            20,
            "wiki.save.projection-a",
            "save|draft-8|source-revision-a",
            "wiki_save_transport: unavailable",
            Some("{\"action\":\"save\",\"action_generation\":20}"),
        )
        .unwrap();
        assert_terminal_observer_baseline_advances(baseline);
    }

    #[test]
    fn unchanged_durable_terminal_cannot_acknowledge_the_next_action() {
        let effect = "wiki-overlay-action";
        let context = "workspace-a/projection-a/pane-4/source-revision-a";
        let observer_author = "wiki.action-status.projection-a";
        let target_author = "wiki.cancel.projection-a";
        let semantic = "cancel|draft-8|source-revision-a";
        let baseline = serialize_observer_click_applied(
            effect,
            context,
            20,
            "wiki.edit.projection-a",
            "edit|draft-8|source-revision-a",
            "{\"action\":\"edit\",\"action_generation\":20}",
        )
        .unwrap();
        let declaration =
            serialize_observer_click_target(effect, context, 20, observer_author, semantic)
                .unwrap();
        let mut snapshot = fixture_snapshot();
        snapshot
            .root
            .children
            .push(clickable_node(target_author, 70, Some(declaration)));
        snapshot
            .root
            .children
            .push(observer_node(observer_author, 71, baseline));

        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, target_author, UiAction::Click)
            .unwrap();
        channel.drain_revalidated_into_events(&snapshot);
        snapshot
            .root
            .children
            .retain(|node| node.author_id.as_deref() != Some(target_author));
        channel.acknowledge_after_render(&snapshot);

        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate
        );
    }

    #[test]
    fn pending_observer_baseline_is_rejected_for_a_new_action_registration() {
        let effect = "wiki-overlay-action";
        let context = "workspace-a/projection-a/pane-4/source-revision-a";
        let observer_author = "wiki.action-status.projection-a";
        let target_author = "wiki.cancel.projection-a";
        let semantic = "cancel|draft-8|source-revision-a";
        let declaration =
            serialize_observer_click_target(effect, context, 20, observer_author, semantic)
                .unwrap();
        let pending = serialize_observer_click_state(
            effect,
            context,
            20,
            ClickCompletionState::Pending,
            Some("wiki.save.projection-a"),
            Some("save|draft-8|source-revision-a"),
        )
        .unwrap();
        let mut snapshot = fixture_snapshot();
        snapshot
            .root
            .children
            .push(clickable_node(target_author, 70, Some(declaration)));
        snapshot
            .root
            .children
            .push(observer_node(observer_author, 71, pending));
        let target = snapshot.find_by_author_id(target_author).unwrap();

        assert!(pending_click_completion(&snapshot, target, &UiAction::Click).is_none());
    }

    #[test]
    fn observer_failed_completion_is_a_causally_bound_rejected_receipt() {
        let effect = "wiki-overlay-action";
        let context = "workspace-a/projection-a/pane-4/source-revision-a";
        let observer_author = "wiki.action-status.projection-a";
        let target_author = "wiki.save.projection-a";
        let semantic = "save|draft-7|source-revision-a";
        let ready = serialize_observer_click_state(
            effect,
            context,
            12,
            ClickCompletionState::Ready,
            None,
            None,
        )
        .unwrap();
        let declaration =
            serialize_observer_click_target(effect, context, 12, observer_author, semantic)
                .unwrap();
        let mut snapshot = fixture_snapshot();
        snapshot
            .root
            .children
            .push(clickable_node(target_author, 70, Some(declaration)));
        snapshot
            .root
            .children
            .push(observer_node(observer_author, 71, ready));

        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, target_author, UiAction::Click)
            .expect("queue observer-backed wiki save");
        channel.drain_revalidated_into_events(&snapshot);

        let mut failed = snapshot;
        failed
            .root
            .children
            .retain(|node| node.author_id.as_deref() != Some(target_author));
        failed
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer_author))
            .unwrap()
            .value = serialize_observer_click_failure(
            effect,
            context,
            13,
            target_author,
            semantic,
            "wiki_save_transport: database unavailable",
            Some("{\"action\":\"save\",\"action_generation\":13}"),
        );
        channel.acknowledge_after_render(&failed);

        let receipt = terminal_receipt(&mut channel, outcome.receipt_id);
        assert_eq!(receipt.status, ActionReceiptStatus::Rejected);
        assert_eq!(
            receipt.rejection.as_deref(),
            Some("wiki_save_transport: database unavailable")
        );
        assert_eq!(
            receipt.observed_value,
            failed
                .root
                .children
                .iter()
                .find(|node| node.author_id.as_deref() == Some(observer_author))
                .unwrap()
                .value
        );
    }

    #[test]
    fn observer_failure_cannot_reject_a_different_or_stale_request() {
        let effect = "wiki-overlay-action";
        let context = "workspace-a/projection-a/pane-4/source-revision-a";
        let observer_author = "wiki.action-status.projection-a";
        let target_author = "wiki.save.projection-a";
        let semantic = "save|draft-7|source-revision-a";
        let ready = serialize_observer_click_state(
            effect,
            context,
            20,
            ClickCompletionState::Ready,
            None,
            None,
        )
        .unwrap();
        let declaration =
            serialize_observer_click_target(effect, context, 20, observer_author, semantic)
                .unwrap();
        let base_snapshot = || {
            let mut snapshot = fixture_snapshot();
            snapshot.root.children.push(clickable_node(
                target_author,
                80,
                Some(declaration.clone()),
            ));
            snapshot
                .root
                .children
                .push(observer_node(observer_author, 81, ready.clone()));
            snapshot
        };

        for (label, failed_context, generation, failed_target, failed_semantic) in [
            (
                "workspace/context",
                "workspace-b/projection-a/pane-4/source-revision-a",
                21,
                target_author,
                semantic,
            ),
            ("generation", context, 20, target_author, semantic),
            ("target", context, 21, "wiki.cancel.projection-a", semantic),
            (
                "semantic",
                context,
                21,
                target_author,
                "save|different-draft|source-revision-a",
            ),
        ] {
            let snapshot = base_snapshot();
            let mut channel = ActionChannel::new();
            let outcome = channel
                .enqueue(&snapshot, target_author, UiAction::Click)
                .unwrap();
            channel.drain_revalidated_into_events(&snapshot);
            let mut failed = snapshot;
            failed
                .root
                .children
                .retain(|node| node.author_id.as_deref() != Some(target_author));
            failed
                .root
                .children
                .iter_mut()
                .find(|node| node.author_id.as_deref() == Some(observer_author))
                .unwrap()
                .value = serialize_observer_click_failure(
                effect,
                failed_context,
                generation,
                failed_target,
                failed_semantic,
                "wiki_save_conflict: stale source",
                Some("{\"action\":\"save\",\"action_generation\":21}"),
            );
            channel.acknowledge_after_render(&failed);
            assert_eq!(
                terminal_receipt(&mut channel, outcome.receipt_id).status,
                ActionReceiptStatus::Indeterminate,
                "{label} drift cannot reject the pending request"
            );
        }
    }

    #[test]
    fn observer_completion_identity_context_generation_and_semantic_drift_are_indeterminate() {
        let effect = "code-completion-accept";
        let context = "file:///workspace/main.cpp@2";
        let observer_author = "code-editor.completion-observer";
        let semantic = "add_numbers|add_numbers(int,int)";
        let ready = serialize_observer_click_state(
            effect,
            context,
            9,
            ClickCompletionState::Ready,
            None,
            None,
        )
        .unwrap();
        let declaration =
            serialize_observer_click_target(effect, context, 9, observer_author, semantic).unwrap();
        let base_snapshot = || {
            let mut snapshot = fixture_snapshot();
            snapshot.root.children.push(clickable_node(
                "completion-item",
                50,
                Some(declaration.clone()),
            ));
            snapshot
                .root
                .children
                .push(observer_node(observer_author, 60, ready.clone()));
            snapshot
        };

        for (label, post_effect, post_context, post_generation, post_target, post_semantic) in [
            ("effect", "wrong", context, 10, "completion-item", semantic),
            (
                "context",
                effect,
                "file:///other.cpp@2",
                10,
                "completion-item",
                semantic,
            ),
            (
                "generation",
                effect,
                context,
                11,
                "completion-item",
                semantic,
            ),
            ("target", effect, context, 10, "other-item", semantic),
            (
                "semantic",
                effect,
                context,
                10,
                "completion-item",
                "different",
            ),
        ] {
            let snapshot = base_snapshot();
            let mut channel = ActionChannel::new();
            let outcome = channel
                .enqueue(&snapshot, "completion-item", UiAction::Click)
                .unwrap();
            channel.drain_revalidated_into_events(&snapshot);
            let mut post = snapshot;
            post.root
                .children
                .retain(|node| node.author_id.as_deref() != Some("completion-item"));
            let observer = post
                .root
                .children
                .iter_mut()
                .find(|node| node.author_id.as_deref() == Some(observer_author))
                .unwrap();
            observer.value = serialize_observer_click_state(
                post_effect,
                post_context,
                post_generation,
                ClickCompletionState::Applied,
                Some(post_target),
                Some(post_semantic),
            );
            channel.acknowledge_after_render(&post);
            assert_eq!(
                terminal_receipt(&mut channel, outcome.receipt_id).status,
                ActionReceiptStatus::Indeterminate,
                "{label} drift"
            );
        }

        let snapshot = base_snapshot();
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, "completion-item", UiAction::Click)
            .unwrap();
        let mut dispatch = snapshot.clone();
        dispatch
            .root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer_author))
            .unwrap()
            .role = "Group".to_owned();
        channel.drain_revalidated_into_events(&dispatch);
        let mut post = dispatch;
        post.root
            .children
            .retain(|node| node.author_id.as_deref() != Some("completion-item"));
        post.root
            .children
            .iter_mut()
            .find(|node| node.author_id.as_deref() == Some(observer_author))
            .unwrap()
            .value = serialize_observer_click_state(
            effect,
            context,
            10,
            ClickCompletionState::Applied,
            Some("completion-item"),
            Some(semantic),
        );
        channel.acknowledge_after_render(&post);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate,
            "observer dispatch identity drift must fail closed"
        );
    }

    #[test]
    fn reserved_token_parser_is_bounded_and_rejects_unknown_fields_without_claiming_applied() {
        let valid = serialize_same_target_click_completion(
            "image-modal-open",
            "workspace-a/image-1",
            1,
            ClickCompletionState::Ready,
        )
        .unwrap();
        let mut unknown: serde_json::Value = serde_json::from_str(&valid).unwrap();
        unknown["surprise"] = serde_json::json!(true);
        let unknown = unknown.to_string();
        assert!(parse_click_completion_token(&unknown).is_none());
        assert!(
            parse_click_completion_token(&"x".repeat(MAX_CLICK_COMPLETION_TOKEN_BYTES + 1))
                .is_none()
        );
        assert!(serialize_same_target_click_completion(
            "",
            "workspace-a/image-1",
            1,
            ClickCompletionState::Ready
        )
        .is_none());
        assert!(serialize_observer_click_state(
            "code-completion-accept",
            "context",
            1,
            ClickCompletionState::Pending,
            None,
            None,
        )
        .is_none());
        assert!(serialize_observer_click_failure(
            "wiki-overlay-action",
            "context",
            2,
            "wiki.save.projection-a",
            "save|draft-1",
            &"x".repeat(MAX_CLICK_COMPLETION_ERROR_BYTES + 1),
            None,
        )
        .is_none());
        assert!(serialize_observer_click_failure(
            "wiki-overlay-action",
            "context",
            2,
            "wiki.save.projection-a",
            "save|draft-1",
            "typed\ncontrol",
            None,
        )
        .is_none());
        assert!(serialize_observer_click_applied(
            "wiki-overlay-action",
            "context",
            2,
            "wiki.save.projection-a",
            "save|draft-1",
            &"x".repeat(MAX_CLICK_COMPLETION_DETAIL_BYTES + 1),
        )
        .is_none());
        let normal_observer = serialize_observer_click_state(
            "code-completion-accept",
            "context",
            2,
            ClickCompletionState::Applied,
            Some("completion-item"),
            Some("semantic"),
        )
        .unwrap();
        assert!(
            !normal_observer.contains("terminal_error"),
            "Ready/Pending/Applied serialization remains backward-compatible"
        );
        assert!(
            !normal_observer.contains("terminal_detail"),
            "legacy observer state cannot leak a prior terminal detail"
        );
        let mut ready_with_stale_detail: serde_json::Value = serde_json::from_str(
            &serialize_observer_click_state(
                "code-completion-accept",
                "context",
                3,
                ClickCompletionState::Ready,
                None,
                None,
            )
            .unwrap(),
        )
        .unwrap();
        ready_with_stale_detail["terminal_detail"] = serde_json::json!("stale-secret");
        let parsed = parse_click_completion_token(&ready_with_stale_detail.to_string()).unwrap();
        assert!(
            !parsed.valid_observer_state(),
            "Ready cannot carry stale terminal detail into a new request"
        );

        let mut snapshot = fixture_snapshot();
        snapshot.root.children[0].value = Some(unknown);
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(&snapshot, "btn", UiAction::Click)
            .expect("a malformed opt-in token does not block the underlying generic click");
        channel.drain_revalidated_into_events(&snapshot);
        channel.acknowledge_after_render(&snapshot);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate,
            "malformed token never upgrades a generic click"
        );
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
    fn set_value_completion_requires_exact_context_generation_and_value_transition() {
        let observer_author = set_value_completion_author_id("field");
        let baseline_value =
            serialize_set_value_completion("field", "workspace-a/find", 7, None).unwrap();
        let mut baseline = fixture_snapshot();
        baseline
            .root
            .children
            .push(observer_node(&observer_author, 90, baseline_value.clone()));
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(
                &baseline,
                "field",
                UiAction::SetValue {
                    text: "needle".to_owned(),
                },
            )
            .unwrap();
        assert_eq!(channel.drain_revalidated_into_events(&baseline).len(), 1);

        let mut applied = baseline;
        top_level_node_mut(&mut applied, "field").value = Some("needle".to_owned());
        top_level_node_mut(&mut applied, &observer_author).value =
            serialize_set_value_completion("field", "workspace-a/find", 8, Some("needle"));
        channel.acknowledge_after_render(&applied);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn set_value_completion_fails_closed_for_same_value_and_token_drift() {
        let observer_author = set_value_completion_author_id("field");
        let baseline_token =
            serialize_set_value_completion("field", "workspace-a/find", 3, None).unwrap();

        let mut same = fixture_snapshot();
        top_level_node_mut(&mut same, "field").value = Some("already".to_owned());
        same.root
            .children
            .push(observer_node(&observer_author, 91, baseline_token.clone()));
        let mut same_channel = ActionChannel::new();
        let same_outcome = same_channel
            .enqueue(
                &same,
                "field",
                UiAction::SetValue {
                    text: "already".to_owned(),
                },
            )
            .unwrap();
        same_channel.drain_revalidated_into_events(&same);
        let mut same_post = same;
        top_level_node_mut(&mut same_post, &observer_author).value =
            serialize_set_value_completion("field", "workspace-a/find", 4, Some("already"));
        same_channel.acknowledge_after_render(&same_post);
        assert_eq!(
            terminal_receipt(&mut same_channel, same_outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate,
            "already-present values never opt into causal completion"
        );

        for (label, context, generation, value) in [
            ("wrong context", "workspace-b/find", 4, "needle"),
            ("generation jump", "workspace-a/find", 5, "needle"),
            ("wrong value", "workspace-a/find", 4, "replacement"),
        ] {
            let mut baseline = fixture_snapshot();
            baseline.root.children.push(observer_node(
                &observer_author,
                92,
                baseline_token.clone(),
            ));
            let mut channel = ActionChannel::new();
            let outcome = channel
                .enqueue(
                    &baseline,
                    "field",
                    UiAction::SetValue {
                        text: "needle".to_owned(),
                    },
                )
                .unwrap();
            channel.drain_revalidated_into_events(&baseline);
            let mut post = baseline;
            top_level_node_mut(&mut post, "field").value = Some("needle".to_owned());
            top_level_node_mut(&mut post, &observer_author).value =
                serialize_set_value_completion("field", context, generation, Some(value));
            channel.acknowledge_after_render(&post);
            assert_eq!(
                terminal_receipt(&mut channel, outcome.receipt_id).status,
                ActionReceiptStatus::Indeterminate,
                "{label}"
            );
        }

        let mut baseline = fixture_snapshot();
        baseline
            .root
            .children
            .push(observer_node(&observer_author, 93, baseline_token));
        let mut channel = ActionChannel::new();
        let outcome = channel
            .enqueue(
                &baseline,
                "field",
                UiAction::SetValue {
                    text: "needle".to_owned(),
                },
            )
            .unwrap();
        channel.drain_revalidated_into_events(&baseline);
        let mut post = baseline;
        top_level_node_mut(&mut post, "field").value = Some("needle".to_owned());
        post.root.children.push(observer_node(
            &observer_author,
            94,
            serialize_set_value_completion("field", "workspace-a/find", 4, Some("needle")).unwrap(),
        ));
        channel.acknowledge_after_render(&post);
        assert_eq!(
            terminal_receipt(&mut channel, outcome.receipt_id).status,
            ActionReceiptStatus::Indeterminate,
            "an ambiguous completion observer cannot acknowledge SetValue"
        );
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
