//! Canonical Argus request context, live window snapshots, and applied-action receipts.
//!
//! Authentication remains owned by [`crate::mcp::tools::SessionToken`].  The caller-provided
//! `agent_label` is bounded attribution metadata, never an authentication credential.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::accessibility::{UiTreeNode, UiTreeSnapshot};
use crate::mcp::action::UiAction;
use serde::Serialize;

pub const MAIN_WINDOW_ID: &str = "main";
pub const MAX_AGENT_LABEL_BYTES: usize = 64;
pub const ACTION_RECEIPT_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArgusWindowDescriptor {
    pub window_id: String,
    pub viewport_id: String,
    pub title: String,
}

/// One deterministic row returned by `argus.list_windows`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArgusWindowListing {
    pub window_id: String,
    pub viewport_id: String,
    pub title: String,
    pub revision: u64,
    pub snapshot_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArgusWindowSnapshot {
    #[serde(flatten)]
    pub window: ArgusWindowDescriptor,
    pub revision: u64,
    pub snapshot: UiTreeSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgusError {
    UnknownWindow {
        window_id: String,
    },
    SnapshotUnavailable {
        window_id: String,
    },
    StaleRevision {
        window_id: String,
        expected: u64,
        actual: u64,
    },
    DuplicateTarget {
        window_id: String,
        author_id: String,
        count: usize,
    },
    InvalidAgentLabel,
    ReceiptTimeout {
        action_id: String,
    },
    UnknownReceipt {
        action_id: String,
    },
}

impl std::fmt::Display for ArgusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownWindow { window_id } => write!(f, "unknown Argus window '{window_id}'"),
            Self::SnapshotUnavailable { window_id } => {
                write!(f, "Argus window '{window_id}' has no rendered snapshot yet")
            }
            Self::StaleRevision { window_id, expected, actual } => write!(
                f,
                "stale Argus snapshot for '{window_id}': expected revision {expected}, actual {actual}"
            ),
            Self::DuplicateTarget { window_id, author_id, count } => write!(
                f,
                "Argus target '{author_id}' is ambiguous in window '{window_id}' ({count} matches)"
            ),
            Self::InvalidAgentLabel => write!(
                f,
                "agent_label must be 1..={MAX_AGENT_LABEL_BYTES} ASCII graphic bytes"
            ),
            Self::ReceiptTimeout { action_id } => {
                write!(f, "timed out waiting for applied Argus action '{action_id}'")
            }
            Self::UnknownReceipt { action_id } => {
                write!(f, "unknown Argus action receipt '{action_id}'")
            }
        }
    }
}

impl std::error::Error for ArgusError {}

#[derive(Debug, Default)]
struct WindowRegistryState {
    windows: HashMap<String, WindowEntry>,
}

#[derive(Debug, Clone)]
struct WindowEntry {
    descriptor: ArgusWindowDescriptor,
    revision: u64,
    snapshot: Option<UiTreeSnapshot>,
}

#[derive(Debug, Clone, Default)]
pub struct WindowSnapshotRegistry {
    inner: Arc<Mutex<WindowRegistryState>>,
}

impl WindowSnapshotRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, descriptor: ArgusWindowDescriptor) {
        let mut state = lock_unpoisoned(&self.inner);
        state
            .windows
            .entry(descriptor.window_id.clone())
            .and_modify(|entry| entry.descriptor = descriptor.clone())
            .or_insert(WindowEntry {
                descriptor,
                revision: 0,
                snapshot: None,
            });
    }

    pub fn unregister(&self, window_id: &str) {
        lock_unpoisoned(&self.inner).windows.remove(window_id);
    }

    pub fn publish(&self, descriptor: ArgusWindowDescriptor, snapshot: UiTreeSnapshot) -> u64 {
        let mut state = lock_unpoisoned(&self.inner);
        let entry = state
            .windows
            .entry(descriptor.window_id.clone())
            .or_insert(WindowEntry {
                descriptor: descriptor.clone(),
                revision: 0,
                snapshot: None,
            });
        entry.descriptor = descriptor;
        entry.revision = entry.revision.saturating_add(1);
        entry.snapshot = Some(snapshot);
        entry.revision
    }

    pub fn get(&self, window_id: &str) -> Result<ArgusWindowSnapshot, ArgusError> {
        let state = lock_unpoisoned(&self.inner);
        let entry = state
            .windows
            .get(window_id)
            .ok_or_else(|| ArgusError::UnknownWindow {
                window_id: window_id.to_owned(),
            })?;
        let snapshot = entry
            .snapshot
            .clone()
            .ok_or_else(|| ArgusError::SnapshotUnavailable {
                window_id: window_id.to_owned(),
            })?;
        Ok(ArgusWindowSnapshot {
            window: entry.descriptor.clone(),
            revision: entry.revision,
            snapshot,
        })
    }

    pub fn descriptor(&self, window_id: &str) -> Result<ArgusWindowDescriptor, ArgusError> {
        lock_unpoisoned(&self.inner)
            .windows
            .get(window_id)
            .map(|entry| entry.descriptor.clone())
            .ok_or_else(|| ArgusError::UnknownWindow {
                window_id: window_id.to_owned(),
            })
    }

    pub fn descriptor_by_viewport(&self, viewport_id: &str) -> Option<ArgusWindowDescriptor> {
        lock_unpoisoned(&self.inner)
            .windows
            .values()
            .find(|entry| entry.descriptor.viewport_id == viewport_id)
            .map(|entry| entry.descriptor.clone())
    }

    /// Enumerate every registered Argus window in stable `window_id` order.
    ///
    /// A just-created pop-out is deliberately visible before its first rendered
    /// snapshot (`revision == 0`, `snapshot_available == false`) so an external
    /// driver can poll one canonical surface instead of guessing viewport timing.
    pub fn list(&self) -> Vec<ArgusWindowListing> {
        let state = lock_unpoisoned(&self.inner);
        let mut windows = state
            .windows
            .values()
            .map(|entry| ArgusWindowListing {
                window_id: entry.descriptor.window_id.clone(),
                viewport_id: entry.descriptor.viewport_id.clone(),
                title: entry.descriptor.title.clone(),
                revision: entry.revision,
                snapshot_available: entry.snapshot.is_some(),
            })
            .collect::<Vec<_>>();
        windows.sort_by(|left, right| left.window_id.cmp(&right.window_id));
        windows
    }

    pub fn validate_target(
        &self,
        window_id: &str,
        author_id: &str,
        expected_revision: u64,
    ) -> Result<ArgusWindowSnapshot, ArgusError> {
        let window = self.get(window_id)?;
        if window.revision != expected_revision {
            return Err(ArgusError::StaleRevision {
                window_id: window_id.to_owned(),
                expected: expected_revision,
                actual: window.revision,
            });
        }
        let count = count_author_id(&window.snapshot.root, author_id);
        if count > 1 {
            return Err(ArgusError::DuplicateTarget {
                window_id: window_id.to_owned(),
                author_id: author_id.to_owned(),
                count,
            });
        }
        Ok(window)
    }
}

/// Post-pass bridge from egui's exact rendered AccessKit output into the window-keyed Argus
/// registry. `input_hook` remembers which viewport is about to run; `output_hook` is invoked after
/// egui generated that pass's `FullOutput::platform_output.accesskit_update` and before eframe takes
/// it, so no duplicate/offscreen render is involved.
pub struct ArgusOutputPlugin {
    windows: WindowSnapshotRegistry,
    legacy_main: Arc<Mutex<UiTreeSnapshot>>,
    /// Input/output hooks can nest when a root pass renders an immediate pop-out. LIFO pairing keeps
    /// the child FullOutput bound to the child viewport and the later root FullOutput bound to root.
    viewport_stack: Vec<String>,
}

impl ArgusOutputPlugin {
    pub fn new(windows: WindowSnapshotRegistry, legacy_main: Arc<Mutex<UiTreeSnapshot>>) -> Self {
        Self {
            windows,
            legacy_main,
            viewport_stack: Vec::new(),
        }
    }
}

impl egui::Plugin for ArgusOutputPlugin {
    fn debug_name(&self) -> &'static str {
        "argus-output"
    }

    fn input_hook(&mut self, input: &mut egui::RawInput) {
        self.viewport_stack.push(format!("{:?}", input.viewport_id));
    }

    fn output_hook(&mut self, output: &mut egui::FullOutput) {
        let Some(viewport_id) = self.viewport_stack.pop() else {
            return;
        };
        let Some(descriptor) = self.windows.descriptor_by_viewport(&viewport_id) else {
            return;
        };
        let Some(update) = output.platform_output.accesskit_update.as_ref() else {
            return;
        };
        let snapshot = crate::accessibility::collect_ui_tree_snapshot(update);
        self.windows.publish(descriptor.clone(), snapshot.clone());
        if descriptor.window_id == MAIN_WINDOW_ID {
            *lock_unpoisoned(&self.legacy_main) = snapshot;
        }
    }
}

pub fn validate_agent_label(label: &str) -> Result<(), ArgusError> {
    if label.is_empty()
        || label.len() > MAX_AGENT_LABEL_BYTES
        || !label.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ArgusError::InvalidAgentLabel);
    }
    Ok(())
}

fn count_author_id(node: &UiTreeNode, author_id: &str) -> usize {
    usize::from(node.author_id.as_deref() == Some(author_id))
        + node
            .children
            .iter()
            .map(|child| count_author_id(child, author_id))
            .sum::<usize>()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionReceiptStatus {
    Queued,
    Applied,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArgusActionReceipt {
    pub action_id: String,
    pub action: String,
    pub connection_id: String,
    pub agent_label: String,
    pub window_id: String,
    pub author_id: String,
    pub before_revision: u64,
    pub after_revision: Option<u64>,
    pub status: ActionReceiptStatus,
    pub error: Option<String>,
    pub evidence_ref: Option<String>,
    /// Evidence persistence is independent from the action result. A backend
    /// outage must never rewrite an already-observed UI outcome as `failed`.
    pub durability_error: Option<String>,
}

#[derive(Debug, Clone)]
enum ExpectedPostcondition {
    HandlerAcknowledgement,
    SetValue { expected_value: String },
}

#[derive(Debug, Default)]
struct ReceiptState {
    receipts: HashMap<String, ArgusActionReceipt>,
    postconditions: HashMap<String, ExpectedPostcondition>,
    handler_acknowledgements: HashSet<String>,
}

#[derive(Clone)]
struct PendingActionEffect {
    action_id: String,
    tracker: ActionReceiptTracker,
}

type ActionEffectKey = (egui::ViewportId, String);
static PENDING_ACTION_EFFECTS: OnceLock<Mutex<HashMap<ActionEffectKey, PendingActionEffect>>> =
    OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionEffectRegistration {
    Registered,
    Busy,
    AlreadyTerminal,
}

/// Register the exact action which the named target handler must acknowledge.
/// Returns `Busy` while the same viewport/author target already has an
/// outstanding causal handler action, or `AlreadyTerminal` when the receipt timed out/failed
/// before its first drain. The caller retains `Busy` actions and drops
/// `AlreadyTerminal` actions without AccessKit injection.
///
/// Merely draining input or changing/removing a snapshot node cannot touch this
/// registry and therefore cannot manufacture an Applied receipt.
pub fn register_action_effect(
    viewport_id: egui::ViewportId,
    author_id: &str,
    action_id: &str,
    tracker: ActionReceiptTracker,
) -> ActionEffectRegistration {
    // Receipt state and target reservation are one atomic admission decision.
    // Terminal transitions take these locks in the same receipt -> registry
    // order before clearing a reservation, so a pre-drain timeout cannot race
    // this check and install a reservation for an already-terminal action.
    let pending_tracker = tracker.clone();
    let (receipt_lock, _) = &*tracker.inner;
    let receipt_state = lock_unpoisoned(receipt_lock);
    if receipt_state
        .receipts
        .get(action_id)
        .is_none_or(|receipt| receipt.status != ActionReceiptStatus::Queued)
    {
        return ActionEffectRegistration::AlreadyTerminal;
    }
    let mut pending =
        lock_unpoisoned(PENDING_ACTION_EFFECTS.get_or_init(|| Mutex::new(HashMap::new())));
    let key = (viewport_id, author_id.to_owned());
    if pending.contains_key(&key) {
        return ActionEffectRegistration::Busy;
    }
    pending.insert(
        key,
        PendingActionEffect {
            action_id: action_id.to_owned(),
            tracker: pending_tracker,
        },
    );
    ActionEffectRegistration::Registered
}

/// Called by a production target handler only after it has applied its effect.
pub fn acknowledge_action_effect(ctx: &egui::Context, author_id: &str) {
    acknowledge_action_effect_for_viewport(ctx.viewport_id(), author_id);
}

fn acknowledge_action_effect_for_viewport(viewport_id: egui::ViewportId, author_id: &str) {
    // Keep the reservation until the receipt becomes terminal. Releasing it on
    // handler acknowledgement would let a second same-target action enter before
    // the newer rendered snapshot proves the first action's postcondition.
    let pending =
        lock_unpoisoned(PENDING_ACTION_EFFECTS.get_or_init(|| Mutex::new(HashMap::new())))
            .get(&(viewport_id, author_id.to_owned()))
            .cloned();
    if let Some(pending) = pending {
        pending.tracker.acknowledge_effect(&pending.action_id);
    }
}

fn clear_registered_action_effect(action_id: &str) {
    lock_unpoisoned(PENDING_ACTION_EFFECTS.get_or_init(|| Mutex::new(HashMap::new())))
        .retain(|_, pending| pending.action_id != action_id);
}

#[derive(Debug, Clone, Default)]
pub struct ActionReceiptTracker {
    inner: Arc<(Mutex<ReceiptState>, Condvar)>,
}

impl ActionReceiptTracker {
    /// Whether an action receipt is already terminal (or no longer exists). The viewport drain uses
    /// this to discard a mutation whose bounded waiter timed out before the next input pass, so a
    /// timed-out SetValue cannot be injected later as an orphan side effect.
    pub fn is_terminal(&self, action_id: &str) -> bool {
        let (lock, _) = &*self.inner;
        lock_unpoisoned(lock)
            .receipts
            .get(action_id)
            .is_none_or(|receipt| receipt.status != ActionReceiptStatus::Queued)
    }

    /// Read a receipt without waiting or changing its state.
    pub fn get(&self, action_id: &str) -> Option<ArgusActionReceipt> {
        let (lock, _) = &*self.inner;
        lock_unpoisoned(lock).receipts.get(action_id).cloned()
    }

    /// Release a terminal receipt after the session has finished attribution and optional durable
    /// persistence. Queued receipts are never removed: the live waiter/viewport still owns them.
    pub fn release_terminal(&self, action_id: &str) -> bool {
        let (lock, _) = &*self.inner;
        let mut state = lock_unpoisoned(lock);
        let terminal = state
            .receipts
            .get(action_id)
            .is_some_and(|receipt| receipt.status != ActionReceiptStatus::Queued);
        if !terminal {
            return false;
        }
        state.receipts.remove(action_id);
        state.postconditions.remove(action_id);
        state.handler_acknowledgements.remove(action_id);
        drop(state);
        clear_registered_action_effect(action_id);
        true
    }

    pub fn begin(
        &self,
        connection_id: &str,
        agent_label: &str,
        window_id: &str,
        author_id: &str,
        action: &UiAction,
        before_revision: u64,
        snapshot: &UiTreeSnapshot,
    ) -> ArgusActionReceipt {
        let (lock, _) = &*self.inner;
        let mut state = lock_unpoisoned(lock);
        let (action_name, postcondition) = match action {
            UiAction::Click | UiAction::ShowContextMenu => {
                let Some(_) = snapshot.find_by_author_id(author_id) else {
                    return ArgusActionReceipt {
                        action_id: format!("argus-action-{}", uuid::Uuid::now_v7()),
                        action: match action {
                            UiAction::Click => "Click",
                            UiAction::ShowContextMenu => "ShowContextMenu",
                            _ => unreachable!(),
                        }
                        .to_owned(),
                        connection_id: connection_id.to_owned(),
                        agent_label: agent_label.to_owned(),
                        window_id: window_id.to_owned(),
                        author_id: author_id.to_owned(),
                        before_revision,
                        after_revision: None,
                        status: ActionReceiptStatus::Failed,
                        error: Some("click target missing from pre-action snapshot".to_owned()),
                        evidence_ref: None,
                        durability_error: None,
                    };
                };
                (
                    match action {
                        UiAction::Click => "Click",
                        UiAction::ShowContextMenu => "ShowContextMenu",
                        _ => unreachable!(),
                    },
                    ExpectedPostcondition::HandlerAcknowledgement,
                )
            }
            UiAction::SetValue { text } => (
                "SetValue",
                ExpectedPostcondition::SetValue {
                    expected_value: text.clone(),
                },
            ),
            _ => unreachable!("Argus receipts are only created for mutating actions"),
        };
        let receipt = ArgusActionReceipt {
            // Durable EventLedger idempotency keys must remain unique across native process
            // restarts, not merely within one in-memory receipt tracker.
            action_id: format!("argus-action-{}", uuid::Uuid::now_v7()),
            action: action_name.to_owned(),
            connection_id: connection_id.to_owned(),
            agent_label: agent_label.to_owned(),
            window_id: window_id.to_owned(),
            author_id: author_id.to_owned(),
            before_revision,
            after_revision: None,
            status: ActionReceiptStatus::Queued,
            error: None,
            evidence_ref: None,
            durability_error: None,
        };
        state
            .postconditions
            .insert(receipt.action_id.clone(), postcondition);
        state
            .receipts
            .insert(receipt.action_id.clone(), receipt.clone());
        receipt
    }

    pub fn set_evidence_ref(&self, action_id: &str, evidence_ref: String) {
        let (lock, _) = &*self.inner;
        if let Some(receipt) = lock_unpoisoned(lock).receipts.get_mut(action_id) {
            receipt.evidence_ref = Some(evidence_ref);
        }
    }

    pub fn set_durability_error(&self, action_id: &str, error: impl Into<String>) {
        let (lock, _) = &*self.inner;
        if let Some(receipt) = lock_unpoisoned(lock).receipts.get_mut(action_id) {
            receipt.durability_error = Some(error.into());
        }
    }

    pub fn acknowledge_effect(&self, action_id: &str) {
        let (lock, _) = &*self.inner;
        let mut state = lock_unpoisoned(lock);
        if state
            .receipts
            .get(action_id)
            .is_some_and(|receipt| receipt.status == ActionReceiptStatus::Queued)
        {
            state.handler_acknowledgements.insert(action_id.to_owned());
        }
    }

    /// Complete a consumed action only after its action-specific observable
    /// postcondition is present in a newer real UI snapshot.
    pub fn observe_postcondition(
        &self,
        action_id: &str,
        after_revision: u64,
        snapshot: &UiTreeSnapshot,
    ) -> bool {
        let (lock, wake) = &*self.inner;
        let mut state = lock_unpoisoned(lock);
        let Some(receipt) = state.receipts.get(action_id) else {
            return true;
        };
        if receipt.status != ActionReceiptStatus::Queued {
            return true;
        }
        let author_id = receipt.author_id.clone();
        let postcondition = state.postconditions.get(action_id).cloned();
        let outcome = match postcondition {
            Some(ExpectedPostcondition::SetValue { expected_value }) => {
                // Text input mutation is a focus request followed by a text event. The first newer
                // AccessKit snapshot can therefore still expose the pre-edit value; keep observing
                // real rendered frames until the exact value appears or the receipt waiter's bounded
                // timeout marks the action failed. A single intermediate snapshot is not terminal
                // evidence that the input rejected the action.
                if snapshot
                    .find_by_author_id(&author_id)
                    .is_some_and(|node| node.value.as_deref() == Some(expected_value.as_str()))
                {
                    Some(Ok(()))
                } else {
                    None
                }
            }
            Some(ExpectedPostcondition::HandlerAcknowledgement)
                if state.handler_acknowledgements.remove(action_id) =>
            {
                Some(Ok(()))
            }
            Some(ExpectedPostcondition::HandlerAcknowledgement) => Some(Err(
                "target handler did not acknowledge the requested action effect".to_owned(),
            )),
            None => Some(Err(
                "action postcondition evidence is unavailable".to_owned()
            )),
        };
        let Some(outcome) = outcome else {
            return false;
        };
        state.postconditions.remove(action_id);
        state.handler_acknowledgements.remove(action_id);
        if let Some(receipt) = state.receipts.get_mut(action_id) {
            receipt.after_revision = Some(after_revision);
            match outcome {
                Ok(()) => receipt.status = ActionReceiptStatus::Applied,
                Err(error) => {
                    receipt.status = ActionReceiptStatus::Failed;
                    receipt.error = Some(error);
                }
            }
            wake.notify_all();
        }
        drop(state);
        clear_registered_action_effect(action_id);
        true
    }

    pub fn failed(&self, action_id: &str, error: impl Into<String>) {
        self.finish(
            action_id,
            ActionReceiptStatus::Failed,
            None,
            Some(error.into()),
        );
    }

    fn finish(
        &self,
        action_id: &str,
        status: ActionReceiptStatus,
        after_revision: Option<u64>,
        error: Option<String>,
    ) {
        let (lock, wake) = &*self.inner;
        let mut state = lock_unpoisoned(lock);
        let mut transitioned = false;
        if let Some(receipt) = state.receipts.get_mut(action_id) {
            // Applied and failed are terminal. In particular, a late render
            // cannot turn a timeout/failure into a false success.
            if receipt.status != ActionReceiptStatus::Queued {
                return;
            }
            receipt.status = status;
            receipt.after_revision = after_revision;
            receipt.error = error;
            transitioned = true;
            wake.notify_all();
        }
        if !transitioned {
            return;
        }
        state.postconditions.remove(action_id);
        state.handler_acknowledgements.remove(action_id);
        drop(state);
        clear_registered_action_effect(action_id);
    }

    pub fn wait(
        &self,
        action_id: &str,
        timeout: Duration,
    ) -> Result<ArgusActionReceipt, ArgusError> {
        let (lock, wake) = &*self.inner;
        let start = Instant::now();
        let mut state = lock_unpoisoned(lock);
        loop {
            let receipt = state.receipts.get(action_id).cloned().ok_or_else(|| {
                ArgusError::UnknownReceipt {
                    action_id: action_id.to_owned(),
                }
            })?;
            if receipt.status != ActionReceiptStatus::Queued {
                return Ok(receipt);
            }
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                let error = ArgusError::ReceiptTimeout {
                    action_id: action_id.to_owned(),
                };
                let message = error.to_string();
                let receipt = state.receipts.get_mut(action_id).expect("receipt exists");
                receipt.status = ActionReceiptStatus::Failed;
                receipt.error = Some(message);
                state.postconditions.remove(action_id);
                state.handler_acknowledgements.remove(action_id);
                wake.notify_all();
                drop(state);
                clear_registered_action_effect(action_id);
                return Err(error);
            }
            let (next, result) = wake
                .wait_timeout(state, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state = next;
            if result.timed_out() {
                let current = state.receipts.get(action_id).cloned().ok_or_else(|| {
                    ArgusError::UnknownReceipt {
                        action_id: action_id.to_owned(),
                    }
                })?;
                if current.status != ActionReceiptStatus::Queued {
                    return Ok(current);
                }
                let error = ArgusError::ReceiptTimeout {
                    action_id: action_id.to_owned(),
                };
                let message = error.to_string();
                let receipt = state.receipts.get_mut(action_id).expect("receipt exists");
                receipt.status = ActionReceiptStatus::Failed;
                receipt.error = Some(message);
                state.postconditions.remove(action_id);
                state.handler_acknowledgements.remove(action_id);
                wake.notify_all();
                drop(state);
                clear_registered_action_effect(action_id);
                return Err(error);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn pending_postcondition_count(&self) -> usize {
        let (lock, _) = &*self.inner;
        lock_unpoisoned(lock).postconditions.len()
    }
}

#[cfg(test)]
pub(crate) fn registered_action_effect_count() -> usize {
    lock_unpoisoned(PENDING_ACTION_EFFECTS.get_or_init(|| Mutex::new(HashMap::new()))).len()
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::accesskit::{Node, NodeId, Role, Tree, TreeUpdate};
    use egui::Plugin;

    fn tree_with_duplicate(duplicate: bool) -> UiTreeSnapshot {
        let child = UiTreeNode {
            id: "node:2".to_owned(),
            author_id: Some("target".to_owned()),
            node_id: 2,
            role: "Button".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: vec!["Click".to_owned()],
            bounds: None,
            children: Vec::new(),
        };
        let mut children = vec![child.clone()];
        if duplicate {
            children.push(child);
        }
        UiTreeSnapshot {
            root: UiTreeNode {
                id: "node:1".to_owned(),
                author_id: None,
                node_id: 1,
                role: "Window".to_owned(),
                label: None,
                value: None,
                disabled: false,
                actions: Vec::new(),
                bounds: None,
                children,
            },
            captured_at_utc: "0Z".to_owned(),
            widget_count: if duplicate { 3 } else { 2 },
        }
    }

    fn accesskit_output(author_id: &str) -> egui::FullOutput {
        let root_id = NodeId(1);
        let mut root = Node::new(Role::Window);
        root.set_author_id(author_id.to_owned());
        let mut output = egui::FullOutput::default();
        output.platform_output.accesskit_update = Some(TreeUpdate {
            nodes: vec![(root_id, root)],
            tree: Some(Tree::new(root_id)),
            focus: root_id,
        });
        output
    }

    #[test]
    fn output_plugin_publishes_exact_post_pass_tree_and_legacy_main() {
        let windows = WindowSnapshotRegistry::new();
        let descriptor = ArgusWindowDescriptor {
            window_id: MAIN_WINDOW_ID.to_owned(),
            viewport_id: format!("{:?}", egui::ViewportId::ROOT),
            title: "Handshake".to_owned(),
        };
        windows.register(descriptor);
        let legacy = Arc::new(Mutex::new(tree_with_duplicate(false)));
        let mut plugin = ArgusOutputPlugin::new(windows.clone(), legacy.clone());
        let mut input = egui::RawInput::default();
        plugin.input_hook(&mut input);

        let mut output = accesskit_output("actual-root");
        plugin.output_hook(&mut output);

        let published = windows.get(MAIN_WINDOW_ID).expect("published main tree");
        assert_eq!(published.revision, 1);
        assert_eq!(
            published.snapshot.root.author_id.as_deref(),
            Some("actual-root")
        );
        assert_eq!(
            lock_unpoisoned(&legacy).root.author_id.as_deref(),
            Some("actual-root")
        );
        assert!(
            output.platform_output.accesskit_update.is_some(),
            "plugin observes without consuming backend output"
        );
    }

    #[test]
    fn window_listing_is_sorted_and_exposes_unpublished_registration() {
        let windows = WindowSnapshotRegistry::new();
        windows.register(ArgusWindowDescriptor {
            window_id: "popout-pane-b".to_owned(),
            viewport_id: "B".to_owned(),
            title: "Handshake – Problems".to_owned(),
        });
        windows.publish(
            ArgusWindowDescriptor {
                window_id: MAIN_WINDOW_ID.to_owned(),
                viewport_id: "ROOT".to_owned(),
                title: "Handshake".to_owned(),
            },
            tree_with_duplicate(false),
        );

        let listed = windows.list();
        assert_eq!(
            listed
                .iter()
                .map(|window| window.window_id.as_str())
                .collect::<Vec<_>>(),
            vec![MAIN_WINDOW_ID, "popout-pane-b"]
        );
        assert_eq!(listed[0].revision, 1);
        assert!(listed[0].snapshot_available);
        assert_eq!(listed[1].revision, 0);
        assert!(!listed[1].snapshot_available);
    }

    #[test]
    fn output_plugin_pairs_nested_immediate_viewports_lifo() {
        let windows = WindowSnapshotRegistry::new();
        let root_id = egui::ViewportId::ROOT;
        let child_id = egui::ViewportId::from_hash_of("pane-a");
        windows.register(ArgusWindowDescriptor {
            window_id: MAIN_WINDOW_ID.to_owned(),
            viewport_id: format!("{root_id:?}"),
            title: "Handshake".to_owned(),
        });
        windows.register(ArgusWindowDescriptor {
            window_id: "popout-pane-a".to_owned(),
            viewport_id: format!("{child_id:?}"),
            title: "Handshake – Workspace".to_owned(),
        });
        let legacy = Arc::new(Mutex::new(tree_with_duplicate(false)));
        let mut plugin = ArgusOutputPlugin::new(windows.clone(), legacy);

        let mut root_input = egui::RawInput::default();
        root_input.viewport_id = root_id;
        plugin.input_hook(&mut root_input);
        let mut child_input = egui::RawInput::default();
        child_input.viewport_id = child_id;
        plugin.input_hook(&mut child_input);
        plugin.output_hook(&mut accesskit_output("child-tree"));
        plugin.output_hook(&mut accesskit_output("root-tree"));

        assert_eq!(
            windows
                .get("popout-pane-a")
                .unwrap()
                .snapshot
                .root
                .author_id
                .as_deref(),
            Some("child-tree")
        );
        assert_eq!(
            windows
                .get(MAIN_WINDOW_ID)
                .unwrap()
                .snapshot
                .root
                .author_id
                .as_deref(),
            Some("root-tree")
        );
    }

    #[test]
    fn stale_revision_and_duplicate_target_fail_before_action_resolution() {
        let windows = WindowSnapshotRegistry::new();
        let descriptor = ArgusWindowDescriptor {
            window_id: MAIN_WINDOW_ID.to_owned(),
            viewport_id: "ROOT".to_owned(),
            title: "Handshake".to_owned(),
        };
        windows.publish(descriptor.clone(), tree_with_duplicate(false));
        assert!(matches!(
            windows.validate_target(MAIN_WINDOW_ID, "target", 0),
            Err(ArgusError::StaleRevision { actual: 1, .. })
        ));
        windows.publish(descriptor, tree_with_duplicate(true));
        assert!(matches!(
            windows.validate_target(MAIN_WINDOW_ID, "target", 2),
            Err(ArgusError::DuplicateTarget { count: 2, .. })
        ));
    }

    #[test]
    fn click_receipt_rejects_removal_and_arbitrary_fingerprint_drift_without_handler_ack() {
        let tracker = ActionReceiptTracker::default();
        let snapshot = tree_with_duplicate(false);
        let drift_receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            "target",
            &UiAction::Click,
            7,
            &snapshot,
        );
        let mut drifted = snapshot.clone();
        drifted.root.children[0].disabled = true;
        assert!(
            tracker.observe_postcondition(&drift_receipt.action_id, 8, &drifted),
            "missing handler acknowledgement remains an immediate terminal failure"
        );
        assert_eq!(
            tracker
                .wait(&drift_receipt.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Failed
        );
        assert_eq!(tracker.pending_postcondition_count(), 0);

        let removal_receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            "target",
            &UiAction::Click,
            8,
            &snapshot,
        );
        let mut removed = snapshot.clone();
        removed.root.children.clear();
        tracker.observe_postcondition(&removal_receipt.action_id, 9, &removed);
        assert_eq!(
            tracker
                .wait(&removal_receipt.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Failed
        );
    }

    #[test]
    fn click_receipt_applies_only_after_exact_target_handler_ack() {
        let tracker = ActionReceiptTracker::default();
        let mut snapshot = tree_with_duplicate(false);
        let author_id = format!("target-{}", uuid::Uuid::now_v7());
        snapshot.root.children[0].author_id = Some(author_id.clone());
        let receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            &author_id,
            &UiAction::Click,
            7,
            &snapshot,
        );
        register_action_effect(
            egui::ViewportId::ROOT,
            &author_id,
            &receipt.action_id,
            tracker.clone(),
        );
        acknowledge_action_effect_for_viewport(egui::ViewportId::ROOT, &author_id);
        tracker.observe_postcondition(&receipt.action_id, 8, &snapshot);
        assert_eq!(
            tracker
                .wait(&receipt.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn set_value_receipt_waits_across_intermediate_snapshots_for_exact_value() {
        let tracker = ActionReceiptTracker::default();
        let mut before = tree_with_duplicate(false);
        before.root.children[0].role = "TextInput".to_owned();
        before.root.children[0].actions = vec!["Focus".to_owned()];
        before.root.children[0].value = Some(String::new());
        let receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            "target",
            &UiAction::SetValue {
                text: "concurrency".to_owned(),
            },
            7,
            &before,
        );

        assert!(
            !tracker.observe_postcondition(&receipt.action_id, 8, &before),
            "the first newer snapshot is intermediate, not terminal failure evidence"
        );
        assert_eq!(
            tracker.get(&receipt.action_id).unwrap().status,
            ActionReceiptStatus::Queued
        );

        let mut after = before;
        after.root.children[0].value = Some("concurrency".to_owned());
        assert!(tracker.observe_postcondition(&receipt.action_id, 9, &after));
        let applied = tracker
            .wait(&receipt.action_id, Duration::ZERO)
            .expect("exact value makes the receipt terminal");
        assert_eq!(applied.status, ActionReceiptStatus::Applied);
        assert_eq!(applied.after_revision, Some(9));
    }

    #[test]
    fn wait_timeout_is_atomic_and_late_exact_value_cannot_resurrect_receipt() {
        let tracker = ActionReceiptTracker::default();
        let mut snapshot = tree_with_duplicate(false);
        snapshot.root.children[0].role = "TextInput".to_owned();
        snapshot.root.children[0].actions = vec!["Focus".to_owned()];
        snapshot.root.children[0].value = Some("old".to_owned());
        let receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            "target",
            &UiAction::SetValue {
                text: "replacement".to_owned(),
            },
            7,
            &snapshot,
        );

        let timeout = tracker.wait(&receipt.action_id, Duration::ZERO);
        assert!(matches!(timeout, Err(ArgusError::ReceiptTimeout { .. })));
        let timed_out = tracker
            .get(&receipt.action_id)
            .expect("timeout retains its terminal failed receipt for session persistence");
        assert_eq!(timed_out.status, ActionReceiptStatus::Failed);
        assert!(timed_out.error.as_deref().unwrap().contains("timed out"));
        assert_eq!(tracker.pending_postcondition_count(), 0);

        snapshot.root.children[0].value = Some("replacement".to_owned());
        assert!(tracker.observe_postcondition(&receipt.action_id, 8, &snapshot));
        assert_eq!(
            tracker.get(&receipt.action_id).unwrap().status,
            ActionReceiptStatus::Failed,
            "a snapshot after the atomic deadline cannot resurrect the receipt"
        );
    }

    #[test]
    fn terminal_receipt_is_released_only_after_explicit_session_cleanup() {
        let tracker = ActionReceiptTracker::default();
        let snapshot = tree_with_duplicate(false);
        let receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            "target",
            &UiAction::SetValue {
                text: "replacement".to_owned(),
            },
            7,
            &snapshot,
        );
        assert!(!tracker.release_terminal(&receipt.action_id));
        tracker.failed(&receipt.action_id, "terminal release test");
        assert!(tracker.get(&receipt.action_id).is_some());
        assert!(tracker.release_terminal(&receipt.action_id));
        assert!(tracker.get(&receipt.action_id).is_none());
        assert!(!tracker.release_terminal(&receipt.action_id));
    }

    #[test]
    fn show_context_menu_receipt_uses_the_same_exact_handler_ack_gate() {
        let tracker = ActionReceiptTracker::default();
        let mut snapshot = tree_with_duplicate(false);
        let author_id = format!("context-target-{}", uuid::Uuid::now_v7());
        snapshot.root.children[0].author_id = Some(author_id.clone());
        let receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            &author_id,
            &UiAction::ShowContextMenu,
            4,
            &snapshot,
        );
        assert_eq!(receipt.action, "ShowContextMenu");
        assert_eq!(
            register_action_effect(
                egui::ViewportId::ROOT,
                &author_id,
                &receipt.action_id,
                tracker.clone(),
            ),
            ActionEffectRegistration::Registered
        );
        acknowledge_action_effect_for_viewport(egui::ViewportId::ROOT, "unrelated-target");
        tracker.observe_postcondition(&receipt.action_id, 5, &snapshot);
        assert_eq!(
            tracker
                .wait(&receipt.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Failed,
            "an unrelated handler acknowledgement must not apply a context-menu receipt"
        );

        let receipt = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            &author_id,
            &UiAction::ShowContextMenu,
            5,
            &snapshot,
        );
        assert_eq!(
            register_action_effect(
                egui::ViewportId::ROOT,
                &author_id,
                &receipt.action_id,
                tracker.clone(),
            ),
            ActionEffectRegistration::Registered
        );
        acknowledge_action_effect_for_viewport(egui::ViewportId::ROOT, &author_id);
        tracker.observe_postcondition(&receipt.action_id, 6, &snapshot);
        assert_eq!(
            tracker
                .wait(&receipt.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn unrelated_parallel_handler_ack_and_snapshot_change_cannot_complete_other_action() {
        let tracker = ActionReceiptTracker::default();
        let mut snapshot = tree_with_duplicate(false);
        let mut other = snapshot.root.children[0].clone();
        other.author_id = Some("other-target".to_owned());
        other.node_id += 1;
        snapshot.root.children.push(other);
        snapshot.widget_count += 1;
        let target = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            "target",
            &UiAction::Click,
            7,
            &snapshot,
        );
        let unrelated = tracker.begin(
            "connection-2",
            "agent-2",
            MAIN_WINDOW_ID,
            "other-target",
            &UiAction::Click,
            7,
            &snapshot,
        );
        tracker.acknowledge_effect(&unrelated.action_id);
        let mut after = snapshot.clone();
        after.root.children[0].label = Some("unrelated drift".to_owned());
        tracker.observe_postcondition(&target.action_id, 8, &after);
        tracker.observe_postcondition(&unrelated.action_id, 8, &after);
        assert_eq!(
            tracker
                .wait(&target.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Failed
        );
        assert_eq!(
            tracker
                .wait(&unrelated.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Applied
        );
    }

    #[test]
    fn same_target_parallel_clicks_are_serialized_without_action_id_overwrite() {
        let tracker = ActionReceiptTracker::default();
        let mut snapshot = tree_with_duplicate(false);
        let author_id = format!("same-target-{}", uuid::Uuid::now_v7());
        snapshot.root.children[0].author_id = Some(author_id.clone());
        let first = tracker.begin(
            "connection-1",
            "agent-1",
            MAIN_WINDOW_ID,
            &author_id,
            &UiAction::Click,
            7,
            &snapshot,
        );
        let second = tracker.begin(
            "connection-2",
            "agent-2",
            MAIN_WINDOW_ID,
            &author_id,
            &UiAction::Click,
            7,
            &snapshot,
        );

        assert_eq!(
            register_action_effect(
                egui::ViewportId::ROOT,
                &author_id,
                &first.action_id,
                tracker.clone(),
            ),
            ActionEffectRegistration::Registered
        );
        assert_eq!(
            register_action_effect(
                egui::ViewportId::ROOT,
                &author_id,
                &second.action_id,
                tracker.clone(),
            ),
            ActionEffectRegistration::Busy,
            "a same-target parallel click must remain queued, not replace the first action id"
        );
        acknowledge_action_effect_for_viewport(egui::ViewportId::ROOT, &author_id);
        tracker.observe_postcondition(&first.action_id, 8, &snapshot);
        assert_eq!(
            tracker
                .wait(&first.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Applied
        );
        assert_eq!(
            tracker.get(&second.action_id).unwrap().status,
            ActionReceiptStatus::Queued
        );

        assert_eq!(
            register_action_effect(
                egui::ViewportId::ROOT,
                &author_id,
                &second.action_id,
                tracker.clone(),
            ),
            ActionEffectRegistration::Registered
        );
        acknowledge_action_effect_for_viewport(egui::ViewportId::ROOT, &author_id);
        tracker.observe_postcondition(&second.action_id, 9, &snapshot);
        assert_eq!(
            tracker
                .wait(&second.action_id, Duration::ZERO)
                .unwrap()
                .status,
            ActionReceiptStatus::Applied
        );
    }
}
