//! The actions assistive tools and models ask for, back to the host as typed requests with their origin.

use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use accesskit::{
    Action, ActionData, ActionHandler, ActionRequest, Point, ScrollHint, ScrollUnit,
};
use handshake_shell_input::{FocusBy, TextTarget};
use handshake_shell_window::{EventTime, Origin, WindowKey};

use crate::frame::{KeySelection, NodeKey};
use crate::tree::WindowTree;

/// The most requests of assistive tools that wait for the host; more are dropped and counted
/// ([`crate::AccessReport::requests_dropped`]).
pub const MAX_PENDING_REQUESTS: usize = 256;

/// What a request is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessTarget {
    /// The window itself. Its only action is [`Action::Focus`], which activates nothing.
    Window,
    /// A node of the host's frame.
    Node(NodeKey),
}

/// What comes with a request, with ids turned into keys and points into logical pixels.
#[derive(Clone, PartialEq)]
#[non_exhaustive]
pub enum RequestData {
    /// The custom action's number.
    CustomAction(i32),
    /// The value to set. May be private: never printed.
    Value(String),
    /// The number to set.
    NumericValue(f64),
    /// How far to scroll.
    ScrollUnit(ScrollUnit),
    /// Where the node should end up when scrolled into view.
    ScrollHint(ScrollHint),
    /// Where to scroll to, in logical pixels from the client area's top left.
    ScrollToPoint(Point),
    /// The scroll offset, in the target's own logical coordinates.
    SetScrollOffset(Point),
    /// The selection or caret to set.
    SetTextSelection(KeySelection),
}

impl fmt::Debug for RequestData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CustomAction(id) => write!(f, "CustomAction({id})"),
            Self::Value(_) => f.write_str("Value(..)"),
            Self::NumericValue(value) => write!(f, "NumericValue({value})"),
            Self::ScrollUnit(unit) => write!(f, "ScrollUnit({unit:?})"),
            Self::ScrollHint(hint) => write!(f, "ScrollHint({hint:?})"),
            Self::ScrollToPoint(point) => write!(f, "ScrollToPoint({point:?})"),
            Self::SetScrollOffset(point) => write!(f, "SetScrollOffset({point:?})"),
            Self::SetTextSelection(selection) => write!(f, "SetTextSelection({selection:?})"),
        }
    }
}

/// An action somebody asked for, for the host to carry out or not.
///
/// Only this crate makes one: from an assistive tool through the platform adapter, with [`Origin::OtherProgram`]
/// ([`crate::Accessibility::take_requests`]), or from a model's steering route, with [`Origin::ModelAction`]
/// ([`crate::Accessibility::model_action`]). Never with the Operator's origin: a request through the accessibility
/// tree carries no authority to activate a window or replace the clipboard.
#[derive(Clone, Debug, PartialEq)]
pub struct AccessRequest {
    window: WindowKey,
    target: AccessTarget,
    action: Action,
    data: Option<RequestData>,
    origin: Origin,
    at: EventTime,
    text_target: Option<TextTarget>,
}

impl AccessRequest {
    /// The window.
    pub fn window(&self) -> WindowKey {
        self.window
    }

    /// What it is for.
    pub fn target(&self) -> AccessTarget {
        self.target
    }

    /// What to do.
    pub fn action(&self) -> Action {
        self.action
    }

    /// What comes with it.
    pub fn data(&self) -> Option<&RequestData> {
        self.data.as_ref()
    }

    /// Who asked: another program (an assistive tool or automation) or a model.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// When, on the clock of the window loop's events.
    pub fn at(&self) -> EventTime {
        self.at
    }

    /// The input crate's text target its node shows, when it shows one.
    pub fn text_target(&self) -> Option<TextTarget> {
        self.text_target
    }

    /// A model asked.
    pub fn is_model(&self) -> bool {
        matches!(self.origin, Origin::ModelAction(_))
    }

    /// For a focus request, who moves the text focus, for `InputTracker::focus_text` with [`Self::text_target`]: the
    /// model, or another program. Neither takes it from the Operator's composition.
    pub fn focus_by(&self) -> Option<FocusBy<'static>> {
        (self.action == Action::Focus).then_some(match self.origin {
            Origin::ModelAction(attribution) => FocusBy::Model(attribution),
            _ => FocusBy::OtherProgram,
        })
    }
}

/// Why a model's action was not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ActionRefusal {
    /// No tree for the window.
    UnknownWindow,
    /// No such node in the window's tree now.
    UnknownNode,
    /// The node does not declare the action.
    Unsupported,
    /// The data names something else than text runs of the window.
    BadData,
}

impl ActionRefusal {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownWindow => "a11y.action.unknown_window",
            Self::UnknownNode => "a11y.action.unknown_node",
            Self::Unsupported => "a11y.action.unsupported",
            Self::BadData => "a11y.action.bad_data",
        }
    }
}

/// A request as the adapter gave it, waiting for the host.
pub(crate) struct Pending {
    pub(crate) window: WindowKey,
    pub(crate) request: ActionRequest,
    pub(crate) received: Instant,
}

type Notify = Arc<dyn Fn() + Send + Sync>;

/// Requests of every window, from whichever thread the adapter calls on.
#[derive(Default)]
pub(crate) struct RequestQueue {
    pending: Mutex<VecDeque<Pending>>,
    dropped: AtomicU64,
    notify: Mutex<Option<Notify>>,
}

impl RequestQueue {
    fn push(&self, window: WindowKey, request: ActionRequest) {
        {
            let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
            if pending.len() >= MAX_PENDING_REQUESTS {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                return;
            }
            pending.push_back(Pending {
                window,
                request,
                received: Instant::now(),
            });
        }
        let notify = self
            .notify
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(notify) = notify {
            notify();
        }
    }

    pub(crate) fn take(&self) -> VecDeque<Pending> {
        std::mem::take(&mut *self.pending.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Forget what a window's assistive tools asked for: it closed, or was destroyed, and a window of the same key may
    /// come after with other nodes under the ids it numbers again (MT-214 E7-F5).
    pub(crate) fn purge(&self, window: WindowKey) {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain(|pending| pending.window != window);
    }

    /// Forget every request.
    pub(crate) fn purge_all(&self) {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub(crate) fn set_notify(&self, notify: Notify) {
        *self.notify.lock().unwrap_or_else(PoisonError::into_inner) = Some(notify);
    }
}

/// The action handler of one window, for a platform adapter. Any thread may call it; it queues and returns, and the
/// host turns what it queued into [`AccessRequest`]s on the loop's thread.
pub struct ActionSink {
    window: WindowKey,
    queue: Arc<RequestQueue>,
}

impl ActionSink {
    pub(crate) fn new(window: WindowKey, queue: Arc<RequestQueue>) -> Self {
        Self { window, queue }
    }
}

impl ActionHandler for ActionSink {
    fn do_action(&mut self, request: ActionRequest) {
        self.queue.push(self.window, request);
    }
}

impl fmt::Debug for ActionSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ActionSink(window {})", self.window.number())
    }
}

/// Why a queued request did not become one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unresolved {
    /// Its window or node is gone, or a node its data names.
    Stale,
    /// Its node does not declare the action.
    Unsupported,
}

/// A request of the adapter, against the window's tree now.
pub(crate) fn resolve(
    tree: &WindowTree,
    window: WindowKey,
    request: ActionRequest,
    origin: Origin,
    at: EventTime,
) -> Result<AccessRequest, Unresolved> {
    let target = tree.target_of(request.target).ok_or(Unresolved::Stale)?;
    let node = tree.node(request.target).ok_or(Unresolved::Stale)?;
    if !node.supports_action(request.action) {
        return Err(Unresolved::Unsupported);
    }
    let data = match request.data {
        Some(data) => Some(convert(tree, data).ok_or(Unresolved::Stale)?),
        None => None,
    };
    Ok(made(tree, window, target, request.action, data, origin, at))
}

pub(crate) fn made(
    tree: &WindowTree,
    window: WindowKey,
    target: AccessTarget,
    action: Action,
    data: Option<RequestData>,
    origin: Origin,
    at: EventTime,
) -> AccessRequest {
    let text_target = match target {
        AccessTarget::Node(key) => tree.text_target(key),
        AccessTarget::Window => None,
    };
    AccessRequest {
        window,
        target,
        action,
        data,
        origin,
        at,
        text_target,
    }
}

/// The adapter's data with keys for ids and logical pixels for the window's physical ones.
fn convert(tree: &WindowTree, data: ActionData) -> Option<RequestData> {
    Some(match data {
        ActionData::CustomAction(id) => RequestData::CustomAction(id),
        ActionData::Value(value) => RequestData::Value(value.into()),
        ActionData::NumericValue(value) => RequestData::NumericValue(value),
        ActionData::ScrollUnit(unit) => RequestData::ScrollUnit(unit),
        ActionData::ScrollHint(hint) => RequestData::ScrollHint(hint),
        // Platform-native coordinates of the window (accesskit 0.21.1, ActionData::ScrollToPoint).
        ActionData::ScrollToPoint(point) => {
            let scale = tree.scale();
            RequestData::ScrollToPoint(Point::new(
                scale.to_logical(point.x),
                scale.to_logical(point.y),
            ))
        }
        // In the target's coordinates, which are the host's own below the window's scale.
        ActionData::SetScrollOffset(point) => RequestData::SetScrollOffset(point),
        ActionData::SetTextSelection(selection) => {
            RequestData::SetTextSelection(tree.key_selection(selection)?)
        }
    })
}
