//! Every window's tree, its adapter, and the requests that come back.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use accesskit::{Action, ActivationHandler, NodeId, TreeUpdate};
use handshake_shell_window::{
    Attribution, Display, Event, EventTime, Origin, ScaleFactor, WindowEvent, WindowKey,
};

use crate::frame::{FrameRefusal, NodeKey, SemanticFrame};
use crate::request::{
    made, resolve, AccessRequest, AccessTarget, ActionRefusal, ActionSink, RequestData,
    RequestQueue, Unresolved,
};
use crate::tree::{WindowTree, ROOT};

/// What the crate counted. Numbers only, for the host's diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AccessReport {
    /// Frames taken.
    pub frames: u64,
    /// Frames refused ([`FrameRefusal`]).
    pub frames_refused: u64,
    /// Ids the host had set in a node itself, cleared.
    pub ids_cleared: u64,
    /// Updates an active adapter took.
    pub updates_sent: u64,
    /// Whole trees given to an adapter that an assistive tool activated.
    pub initial_trees: u64,
    /// Activations while the tree was being changed: the adapter shows a placeholder, and the next update is whole.
    pub placeholders: u64,
    /// Requests of assistive tools handed to the host.
    pub requests: u64,
    /// Requests for a window, node or text run that is gone.
    pub requests_stale: u64,
    /// Requests for an action the node does not declare.
    pub requests_unsupported: u64,
    /// Requests beyond [`crate::MAX_PENDING_REQUESTS`], dropped.
    pub requests_dropped: u64,
    /// Model actions refused ([`ActionRefusal`]).
    pub model_refused: u64,
    /// Accessibility events the operating system refused to raise.
    pub raise_failed: u64,
}

/// Why a window's tree was not published to the platform.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AttachRefusal {
    /// No such window, or it is gone.
    UnknownWindow,
    /// It has an adapter already.
    AlreadyAttached,
    /// It was shown already, and may be hidden again: the adapter must take the window's procedure before it is first
    /// shown (accesskit_windows 0.29.2). Known from the events fed ([`Accessibility::feed`]) and the window's visibility.
    AlreadyShown,
    /// Not the thread of the window: only the loop's thread may attach.
    WrongThread,
    /// The adapter failed; it panicked inside, and nothing of it stays attached.
    Adapter,
}

impl AttachRefusal {
    /// A stable name, for diagnostics.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownWindow => "a11y.attach.unknown_window",
            Self::AlreadyAttached => "a11y.attach.already_attached",
            Self::AlreadyShown => "a11y.attach.already_shown",
            Self::WrongThread => "a11y.attach.wrong_thread",
            Self::Adapter => "a11y.attach.adapter",
        }
    }
}

#[derive(Debug, Default)]
struct SourceCounts {
    initial: Cell<u64>,
    placeholders: Cell<u64>,
}

/// The activation handler of one window, for a platform adapter: the whole tree as it is now. Called on the loop's
/// thread, inside the window's `WM_GETOBJECT` on Windows; it calls nothing of the host.
pub struct TreeSource {
    tree: Rc<RefCell<WindowTree>>,
    needs_full: Rc<Cell<bool>>,
    counts: Rc<SourceCounts>,
}

impl ActivationHandler for TreeSource {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        if let Ok(tree) = self.tree.try_borrow() {
            self.counts.initial.set(self.counts.initial.get() + 1);
            return Some(tree.full());
        }
        // Being changed, so the adapter shows a placeholder; the next update must be the whole tree (accesskit_windows
        // 0.29.2, `update_if_active`).
        self.needs_full.set(true);
        self.counts.placeholders.set(self.counts.placeholders.get() + 1);
        None
    }
}

impl std::fmt::Debug for TreeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TreeSource")
    }
}

/// What an active adapter is given next: the whole tree when an activation got a placeholder since, so the adapter
/// has a tree to apply the changes to (accesskit_windows 0.29.2, `update_if_active`; a partial update after a
/// placeholder panics in the consumer's `Tree::new`), otherwise `update`.
#[cfg_attr(not(windows), allow(dead_code))]
fn next_update(needs_full: &Cell<bool>, tree: &RefCell<WindowTree>, update: TreeUpdate) -> TreeUpdate {
    if needs_full.replace(false) {
        tree.borrow().full()
    } else {
        update
    }
}

struct Window {
    tree: Rc<RefCell<WindowTree>>,
    needs_full: Rc<Cell<bool>>,
    #[cfg(windows)]
    attached: Option<crate::platform::Attached>,
}

impl Window {
    fn new(scale: ScaleFactor) -> Self {
        Self {
            tree: Rc::new(RefCell::new(WindowTree::new(scale))),
            needs_full: Rc::new(Cell::new(false)),
            #[cfg(windows)]
            attached: None,
        }
    }
}

/// The accessibility of one event loop: one tree per window, the platform adapter of each attached window, and the
/// requests that come back. Lives on the loop's thread.
pub struct Accessibility {
    windows: BTreeMap<WindowKey, Window>,
    /// The windows the loop reported displayed at some time, whether they have a tree or not (MT-214 E7-F6).
    shown: BTreeSet<WindowKey>,
    queue: Arc<RequestQueue>,
    sources: Rc<SourceCounts>,
    report: AccessReport,
}

impl Default for Accessibility {
    fn default() -> Self {
        Self::new()
    }
}

impl Accessibility {
    /// No window yet.
    pub fn new() -> Self {
        Self {
            windows: BTreeMap::new(),
            shown: BTreeSet::new(),
            queue: Arc::default(),
            sources: Rc::default(),
            report: AccessReport::default(),
        }
    }

    /// Called from whichever thread queued a request of an assistive tool, so the host takes it: for example
    /// `move || { let _ = waker.wake(value); }` with the window crate's `LaneWaker`. It must not wait.
    pub fn set_notifier(&mut self, notify: impl Fn() + Send + Sync + 'static) {
        self.queue.set_notify(Arc::new(notify));
    }

    /// A tree for a window without a platform adapter: for inspection, a headless run, or an adapter of another
    /// platform ([`Self::source`], [`Self::sink`]). `false` when the window has one.
    pub fn open(&mut self, window: WindowKey, scale: ScaleFactor) -> bool {
        if self.windows.contains_key(&window) {
            return false;
        }
        self.windows.insert(window, Window::new(scale));
        true
    }

    /// Forget a window: its adapter lets go of the window and of its native token, so it can be destroyed. `false`
    /// when there was nothing.
    pub fn close(&mut self, window: WindowKey) -> bool {
        // The adapter lets go first; what its tools asked for goes with the window, so a window of the same key later
        // never gets a request meant for the nodes this one had (MT-214 E7-F5).
        let removed = self.windows.remove(&window);
        let had = removed.is_some();
        drop(removed);
        self.queue.purge(window);
        had
    }

    /// Forget every window, before `Control::finish`.
    pub fn close_all(&mut self) {
        self.windows.clear();
        self.queue.purge_all();
    }

    /// The window was displayed at some time since the loop created it, as far as the events fed say.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn was_shown(&self, window: WindowKey) -> bool {
        self.shown.contains(&window)
    }

    /// The windows with a tree.
    pub fn windows(&self) -> Vec<WindowKey> {
        self.windows.keys().copied().collect()
    }

    /// Follow the loop: the scale and client size of each window, and windows that went. Feed every event.
    pub fn feed(&mut self, event: &Event) {
        match event {
            Event::Window { window, event } => match event {
                WindowEvent::Created { state, .. } => {
                    self.shown.remove(window);
                    if state.display != Display::Hidden {
                        self.shown.insert(*window);
                    }
                }
                WindowEvent::DisplayChanged { display } if *display != Display::Hidden => {
                    self.shown.insert(*window);
                }
                WindowEvent::ScaleChanged { to, .. } => {
                    self.change(*window, |tree| tree.set_scale(*to));
                }
                WindowEvent::Resized { inner, scale } => {
                    self.change(*window, |tree| tree.set_scale(*scale));
                    self.change(*window, |tree| tree.set_client(inner.to_logical(*scale)));
                }
                WindowEvent::Destroyed => {
                    self.windows.remove(window);
                    self.shown.remove(window);
                    self.queue.purge(*window);
                }
                _ => {}
            },
            Event::LoopEnding => {
                self.windows.clear();
                self.shown.clear();
                self.queue.purge_all();
            }
            _ => {}
        }
    }

    fn change(&mut self, window: WindowKey, change: impl FnOnce(&mut WindowTree) -> Option<TreeUpdate>) {
        let update = match self.windows.get(&window) {
            Some(entry) => change(&mut entry.tree.borrow_mut()),
            None => None,
        };
        if let Some(update) = update {
            self.push(window, update);
        }
    }

    /// The window's semantic tree now. What changed reaches an active adapter; ids stay with their keys.
    ///
    /// # Errors
    ///
    /// [`FrameRefusal`]; the window keeps its tree.
    pub fn update(&mut self, window: WindowKey, frame: SemanticFrame) -> Result<(), FrameRefusal> {
        let applied = match self.windows.get(&window) {
            Some(entry) => entry.tree.borrow_mut().apply(frame),
            None => Err(FrameRefusal::UnknownWindow),
        };
        match applied {
            Ok((update, cleared)) => {
                self.report.frames += 1;
                self.report.ids_cleared += cleared;
                if let Some(update) = update {
                    self.push(window, update);
                }
                Ok(())
            }
            Err(refusal) => {
                self.report.frames_refused += 1;
                Err(refusal)
            }
        }
    }

    /// Hand an update to the window's adapter, if it has one, with no borrow of the tree held: raising its events may
    /// bring a nested `WM_GETOBJECT` (accesskit_windows 0.29.2, `QueuedEvents::raise`).
    #[cfg(windows)]
    fn push(&mut self, window: WindowKey, update: TreeUpdate) {
        let Some(entry) = self.windows.get_mut(&window) else {
            return;
        };
        let Some(attached) = entry.attached.as_mut() else {
            return;
        };
        let (tree, needs_full) = (Rc::clone(&entry.tree), Rc::clone(&entry.needs_full));
        let mut taken = false;
        let raised = attached.update(|| {
            taken = true;
            next_update(&needs_full, &tree, update)
        });
        if taken {
            self.report.updates_sent += 1;
        }
        if !raised {
            self.report.raise_failed += 1;
        }
    }

    #[cfg(not(windows))]
    fn push(&mut self, _window: WindowKey, _update: TreeUpdate) {}

    /// What assistive tools asked for since the last call, as typed requests with [`Origin::OtherProgram`], against the
    /// trees as they are now. `now` is the loop's time (`Control::now`); each request's time is when it was queued.
    pub fn take_requests(&mut self, now: EventTime) -> Vec<AccessRequest> {
        let taken = Instant::now();
        let mut requests = Vec::new();
        for pending in self.queue.take() {
            let age = taken.saturating_duration_since(pending.received).as_nanos();
            let at = EventTime::from_nanos(
                now.nanos()
                    .saturating_sub(u64::try_from(age).unwrap_or(u64::MAX)),
            );
            let resolved = match self.windows.get(&pending.window) {
                Some(entry) => resolve(
                    &entry.tree.borrow(),
                    pending.window,
                    pending.request,
                    Origin::OtherProgram,
                    at,
                ),
                None => Err(Unresolved::Stale),
            };
            match resolved {
                Ok(request) => {
                    self.report.requests += 1;
                    requests.push(request);
                }
                Err(Unresolved::Stale) => self.report.requests_stale += 1,
                Err(Unresolved::Unsupported) => self.report.requests_unsupported += 1,
            }
        }
        requests
    }

    /// A model's action through the steering route, as a request with its attribution: the same request an assistive
    /// tool's would be, never the Operator's. Nothing of the operating system is involved.
    ///
    /// # Errors
    ///
    /// [`ActionRefusal`].
    pub fn model_action(
        &mut self,
        window: WindowKey,
        attribution: Attribution,
        at: EventTime,
        target: AccessTarget,
        action: Action,
        data: Option<RequestData>,
    ) -> Result<AccessRequest, ActionRefusal> {
        let made = self.model_request(window, attribution, at, target, action, data);
        if made.is_err() {
            self.report.model_refused += 1;
        }
        made
    }

    fn model_request(
        &self,
        window: WindowKey,
        attribution: Attribution,
        at: EventTime,
        target: AccessTarget,
        action: Action,
        data: Option<RequestData>,
    ) -> Result<AccessRequest, ActionRefusal> {
        let entry = self.windows.get(&window).ok_or(ActionRefusal::UnknownWindow)?;
        let tree = entry.tree.borrow();
        let id = match target {
            AccessTarget::Window => ROOT,
            AccessTarget::Node(key) => tree.id_of(key).ok_or(ActionRefusal::UnknownNode)?,
        };
        let node = tree.node(id).ok_or(ActionRefusal::UnknownNode)?;
        if !node.supports_action(action) {
            return Err(ActionRefusal::Unsupported);
        }
        if let Some(RequestData::SetTextSelection(selection)) = &data {
            if !tree.is_text_run(selection.anchor.node) || !tree.is_text_run(selection.focus.node) {
                return Err(ActionRefusal::BadData);
            }
        }
        let origin = Origin::ModelAction(attribution);
        Ok(made(&tree, window, target, action, data, origin, at))
    }

    /// The window's whole tree as AccessKit has it, for an inspector such as Argus: read in this process, so no
    /// message reaches the window and nothing of it can be activated.
    pub fn snapshot(&self, window: WindowKey) -> Option<TreeUpdate> {
        self.windows
            .get(&window)
            .map(|entry| entry.tree.borrow().full())
    }

    /// The node whose `author_id` this is, for a model that addresses nodes by it.
    pub fn find_author(&self, window: WindowKey, author_id: &str) -> Option<NodeKey> {
        self.windows
            .get(&window)
            .and_then(|entry| entry.tree.borrow().find_author(author_id))
    }

    /// The AccessKit id of a key now; the window itself is `NodeId(1)`.
    pub fn node_id(&self, window: WindowKey, key: NodeKey) -> Option<NodeId> {
        self.windows
            .get(&window)
            .and_then(|entry| entry.tree.borrow().id_of(key))
    }

    /// The activation handler of a window, for a platform adapter.
    pub fn source(&self, window: WindowKey) -> Option<TreeSource> {
        self.windows.get(&window).map(|entry| TreeSource {
            tree: Rc::clone(&entry.tree),
            needs_full: Rc::clone(&entry.needs_full),
            counts: Rc::clone(&self.sources),
        })
    }

    /// The action handler of a window, for a platform adapter.
    pub fn sink(&self, window: WindowKey) -> Option<ActionSink> {
        self.windows
            .contains_key(&window)
            .then(|| ActionSink::new(window, Arc::clone(&self.queue)))
    }

    /// What was counted so far.
    pub fn report(&self) -> AccessReport {
        AccessReport {
            initial_trees: self.sources.initial.get(),
            placeholders: self.sources.placeholders.get(),
            requests_dropped: self.queue.dropped(),
            ..self.report
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MT-214 E7-F7, mutation M-B2 caught: the update after a placeholder not being the whole tree (the adapter's
    /// consumer would panic on a partial one), or the whole tree given again after that.
    #[test]
    fn the_update_after_a_placeholder_is_whole_once() {
        let mut hub = Accessibility::new();
        let window = WindowKey::from_number(1);
        assert!(hub.open(window, ScaleFactor::ONE));
        let mut source = hub.source(window).unwrap();
        let entry = &hub.windows[&window];
        let partial = TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            focus: ROOT,
        };
        assert_eq!(
            next_update(&entry.needs_full, &entry.tree, partial.clone()),
            partial,
            "no placeholder, the update as it is"
        );
        let held = entry.tree.borrow_mut();
        assert_eq!(source.request_initial_tree(), None);
        drop(held);
        let whole = next_update(&entry.needs_full, &entry.tree, partial.clone());
        assert!(whole.tree.is_some() && whole.nodes.len() == 1, "{whole:?}");
        assert_eq!(whole, entry.tree.borrow().full());
        assert_eq!(
            next_update(&entry.needs_full, &entry.tree, partial.clone()),
            partial,
            "whole once"
        );
    }

    /// MT-214 E7-F5, mutations caught: requests queued before a window closed, was destroyed, or the loop ended
    /// reaching a window of the same key after, whose ids are numbered again.
    #[test]
    fn requests_of_a_window_that_went_do_not_reach_the_next_window_of_its_key() {
        use accesskit::{ActionHandler, ActionRequest};
        let window = WindowKey::from_number(1);
        let other = WindowKey::from_number(2);
        let ask = |hub: &Accessibility, window: WindowKey| {
            hub.sink(window).unwrap().do_action(ActionRequest {
                action: Action::Focus,
                target: ROOT,
                data: None,
            });
        };
        let now = EventTime::from_nanos(1_000);
        for end in 0..3 {
            let mut hub = Accessibility::new();
            assert!(hub.open(window, ScaleFactor::ONE) && hub.open(other, ScaleFactor::ONE));
            ask(&hub, window);
            ask(&hub, other);
            match end {
                0 => assert!(hub.close(window)),
                1 => hub.feed(&Event::Window {
                    window,
                    event: WindowEvent::Destroyed,
                }),
                _ => hub.feed(&Event::LoopEnding),
            }
            assert!(hub.open(window, ScaleFactor::ONE), "the key comes back, with ids numbered again");
            let kept: Vec<WindowKey> = hub.take_requests(now).iter().map(|r| r.window()).collect();
            let expected: &[WindowKey] = if end == 2 { &[] } else { &[other] };
            assert_eq!(kept, expected, "end {end}");
        }
        let mut hub = Accessibility::new();
        assert!(hub.open(window, ScaleFactor::ONE) && hub.open(other, ScaleFactor::ONE));
        ask(&hub, window);
        ask(&hub, other);
        hub.close_all();
        assert!(hub.open(window, ScaleFactor::ONE));
        assert!(hub.take_requests(now).is_empty(), "close_all");
    }

    /// MT-214 E7-F6, mutation caught: only the window's current display read, so a window shown and hidden again
    /// counted as never shown.
    #[test]
    fn a_window_displayed_once_is_shown_for_good() {
        let mut hub = Accessibility::new();
        let window = WindowKey::from_number(1);
        let display = |display| Event::Window {
            window,
            event: WindowEvent::DisplayChanged { display },
        };
        assert!(!hub.was_shown(window));
        hub.feed(&display(Display::Hidden));
        assert!(!hub.was_shown(window), "hidden is not shown");
        hub.feed(&display(Display::Normal));
        hub.feed(&display(Display::Hidden));
        assert!(hub.was_shown(window), "shown, then hidden again");
        hub.feed(&Event::Window {
            window,
            event: WindowEvent::Destroyed,
        });
        assert!(!hub.was_shown(window), "a window that went is forgotten");
        hub.feed(&display(Display::Minimized));
        assert!(hub.was_shown(window));
        hub.feed(&Event::LoopEnding);
        assert!(!hub.was_shown(window));
    }

    /// Mutation caught: an activation while the tree is borrowed panicking inside the window's procedure, or the next
    /// update not whole after a placeholder.
    #[test]
    fn an_activation_while_a_frame_is_taken_gets_a_placeholder_and_the_next_update_is_whole() {
        let mut hub = Accessibility::new();
        let window = WindowKey::from_number(1);
        assert!(hub.open(window, ScaleFactor::ONE));
        let mut source = hub.source(window).unwrap();
        let entry = &hub.windows[&window];
        let held = entry.tree.borrow_mut();
        assert_eq!(source.request_initial_tree(), None);
        drop(held);
        assert!(entry.needs_full.get(), "the next update is whole");
        let whole = source.request_initial_tree().unwrap();
        assert!(whole.tree.is_some() && whole.nodes.len() == 1);
        let report = hub.report();
        assert_eq!((report.placeholders, report.initial_trees), (1, 1));
    }
}

#[cfg(windows)]
mod attach {
    use handshake_shell_window::{Control, WindowKey};

    use super::{Accessibility, AttachRefusal, Window};
    use crate::platform::Attached;

    impl Accessibility {
        /// Publish a window's tree to UI Automation. Call on `WindowEvent::Created`, before the window is first shown:
        /// the adapter takes the window's procedure then (accesskit_windows 0.29.2 `SubclassingAdapter`). It keeps a
        /// native token, so the window is destroyed only after [`Accessibility::close`]. An assistive tool that reads
        /// the tree sends the window `WM_GETOBJECT` and nothing that activates it.
        ///
        /// # Errors
        ///
        /// [`AttachRefusal`].
        pub fn attach(&mut self, control: &Control, window: WindowKey) -> Result<(), AttachRefusal> {
            if self.windows.get(&window).is_some_and(|entry| entry.attached.is_some()) {
                return Err(AttachRefusal::AlreadyAttached);
            }
            // Shown at some time, as the events fed say, though hidden again now (MT-214 E7-F6); the platform
            // checks the window's visibility itself.
            if self.was_shown(window) {
                return Err(AttachRefusal::AlreadyShown);
            }
            let (Some(token), Some(state)) = (control.native_window(window), control.window_state(window)) else {
                return Err(AttachRefusal::UnknownWindow);
            };
            let opened = self.open(window, state.scale);
            let (source, sink) = match (self.source(window), self.sink(window)) {
                (Some(source), Some(sink)) => (source, sink),
                _ => return Err(AttachRefusal::UnknownWindow),
            };
            let entry: &mut Window = self.windows.get_mut(&window).ok_or(AttachRefusal::UnknownWindow)?;
            {
                let mut tree = entry.tree.borrow_mut();
                let _ = tree.set_scale(state.scale);
                if !state.inner.is_empty() {
                    let _ = tree.set_client(state.inner.to_logical(state.scale));
                }
            }
            match Attached::new(token, source, sink) {
                Ok(attached) => {
                    entry.attached = Some(attached);
                    Ok(())
                }
                Err(refusal) => {
                    if opened {
                        self.windows.remove(&window);
                    }
                    Err(refusal)
                }
            }
        }

        /// The window's tree reaches UI Automation.
        pub fn is_attached(&self, window: WindowKey) -> bool {
            self.windows
                .get(&window)
                .is_some_and(|entry| entry.attached.is_some())
        }
    }
}
