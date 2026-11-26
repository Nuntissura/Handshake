//! Requests through the very action handler a platform adapter gets ([`crate::Accessibility::sink`]), and a model's
//! through the steering route, against trees of two windows. No operating system is involved.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use accesskit::{
    Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, Node, NodeId, Point,
    Rect, Role, TextPosition, TextSelection,
};
use handshake_shell_input::{CaretPlace, FocusBy, InputTracker, TextFocus, TextTarget};
use handshake_shell_window::{
    Attribution, Display, Event, EventTime, Origin, PhysicalRect, PhysicalSize, ScaleFactor,
    WindowClass, WindowEvent, WindowKey, WindowState,
};

use crate::{
    text_run, AccessTarget, Accessibility, ActionRefusal, KeyPosition, KeySelection, NodeKey,
    RequestData, SemanticFrame, SemanticNode, MAX_PENDING_REQUESTS,
};

const A: WindowKey = WindowKey::from_number(1);
const B: WindowKey = WindowKey::from_number(2);
const BUTTON: NodeKey = NodeKey(5);
const FIELD: NodeKey = NodeKey(6);
const RUN: NodeKey = NodeKey(7);
const TARGET: TextTarget = TextTarget(40);
const MODEL: Attribution = Attribution {
    actor: 3,
    session: 4,
    correlation: 5,
};

fn now() -> EventTime {
    EventTime::from_nanos(9_000_000_000)
}

/// A button and a text field with one run, the field showing the input crate's text target.
fn frame(label: &str) -> SemanticFrame {
    let mut button = Node::new(Role::Button);
    button.set_label(label);
    button.set_author_id("save-button");
    button.add_action(Action::Click);
    button.add_action(Action::Focus);
    let mut field = Node::new(Role::TextInput);
    for action in [Action::Focus, Action::SetValue, Action::SetTextSelection, Action::ScrollToPoint] {
        field.add_action(action);
    }
    let mut field = SemanticNode::new(FIELD, field);
    field.children = vec![RUN];
    field.text_target = Some(TARGET);
    let run = text_run("secret text", Rect::new(0.0, 0.0, 80.0, 16.0), None).unwrap();
    SemanticFrame {
        name: Some("window".to_owned()),
        top: vec![BUTTON, FIELD],
        nodes: vec![SemanticNode::new(BUTTON, button), field, SemanticNode::new(RUN, run)],
        focus: None,
    }
}

fn two_windows() -> Accessibility {
    let mut hub = Accessibility::new();
    assert!(hub.open(A, ScaleFactor::ONE));
    assert!(hub.open(B, ScaleFactor::from_dpi(144).unwrap()));
    assert!(!hub.open(A, ScaleFactor::ONE), "one tree per window");
    hub.update(A, frame("save")).unwrap();
    hub.update(B, frame("save")).unwrap();
    hub
}

fn ask(hub: &Accessibility, window: WindowKey, action: Action, target: NodeId, data: Option<ActionData>) {
    hub.sink(window).unwrap().do_action(ActionRequest {
        action,
        target,
        data,
    });
}

/// Mutations caught: one id map for every window (both windows number the button 2), a request routed by its id alone,
/// an origin other than another program's.
#[test]
fn requests_of_the_adapter_come_back_typed_with_their_window_and_key() {
    let mut hub = two_windows();
    let (in_a, in_b) = (hub.node_id(A, BUTTON).unwrap(), hub.node_id(B, BUTTON).unwrap());
    assert_eq!(in_a, in_b, "each window numbers its own nodes");
    ask(&hub, A, Action::Click, in_a, None);
    ask(&hub, B, Action::Focus, in_b, None);
    ask(&hub, A, Action::Focus, NodeId(1), None);
    let requests = hub.take_requests(now());
    let seen: Vec<(WindowKey, AccessTarget, Action, Origin)> = requests
        .iter()
        .map(|request| (request.window(), request.target(), request.action(), request.origin()))
        .collect();
    assert_eq!(
        seen,
        [
            (A, AccessTarget::Node(BUTTON), Action::Click, Origin::OtherProgram),
            (B, AccessTarget::Node(BUTTON), Action::Focus, Origin::OtherProgram),
            (A, AccessTarget::Window, Action::Focus, Origin::OtherProgram),
        ]
    );
    for request in &requests {
        assert!(request.at() <= now() && !request.is_model());
        assert_eq!(request.data(), None);
    }
    assert!(hub.take_requests(now()).is_empty(), "taken once");
    assert_eq!(hub.report().requests, 3);
}

/// Mutations caught: a request for a node that went, an undeclared action, or a closed window reaching the host; an
/// unbounded queue.
#[test]
fn stale_unsupported_and_excess_requests_are_dropped_and_counted() {
    let mut hub = two_windows();
    let old = hub.node_id(A, BUTTON).unwrap();
    let mut without = frame("save");
    without.top.retain(|key| *key != BUTTON);
    without.nodes.retain(|node| node.key != BUTTON);
    hub.update(A, without).unwrap();
    hub.update(A, frame("save")).unwrap();
    assert_ne!(hub.node_id(A, BUTTON), Some(old), "the button came back with a new id");
    let field = hub.node_id(A, FIELD).unwrap();
    let kept_sink = hub.sink(B).unwrap();
    ask(&hub, A, Action::Click, old, None);
    ask(&hub, A, Action::Click, NodeId(999), None);
    ask(&hub, A, Action::Expand, field, None);
    ask(&hub, A, Action::Click, NodeId(1), None);
    assert!(hub.close(B));
    let mut kept_sink = kept_sink;
    kept_sink.do_action(ActionRequest {
        action: Action::Click,
        target: NodeId(2),
        data: None,
    });
    assert!(hub.take_requests(now()).is_empty());
    let report = hub.report();
    assert_eq!((report.requests_stale, report.requests_unsupported), (3, 2), "{report:?}");
    let notified = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&notified);
    hub.set_notifier(move || {
        count.fetch_add(1, Ordering::Relaxed);
    });
    let button = hub.node_id(A, BUTTON).unwrap();
    for _ in 0..MAX_PENDING_REQUESTS + 44 {
        ask(&hub, A, Action::Click, button, None);
    }
    assert_eq!(notified.load(Ordering::Relaxed), MAX_PENDING_REQUESTS, "only queued requests notify");
    assert_eq!(hub.take_requests(now()).len(), MAX_PENDING_REQUESTS);
    assert_eq!(hub.report().requests_dropped, 44);
}

/// Mutations caught: a text selection passed on with the adapter's ids, a point left in physical pixels, a value
/// printed.
#[test]
fn what_comes_with_a_request_is_in_keys_and_logical_pixels() {
    let mut hub = two_windows();
    let (field, run) = (hub.node_id(B, FIELD).unwrap(), hub.node_id(B, RUN).unwrap());
    let at = |character_index| TextPosition {
        node: run,
        character_index,
    };
    ask(&hub, B, Action::ScrollToPoint, field, Some(ActionData::ScrollToPoint(Point::new(150.0, 30.0))));
    ask(&hub, B, Action::SetTextSelection, field, Some(ActionData::SetTextSelection(TextSelection { anchor: at(1), focus: at(4) })));
    ask(&hub, B, Action::SetValue, field, Some(ActionData::Value("secret value".into())));
    ask(&hub, B, Action::SetTextSelection, field, Some(ActionData::SetTextSelection(TextSelection { anchor: at(0), focus: TextPosition { node: NodeId(888), character_index: 0 } })));
    let requests = hub.take_requests(now());
    let data: Vec<RequestData> = requests.iter().filter_map(|request| request.data().cloned()).collect();
    let end = |character_index| KeyPosition {
        node: RUN,
        character_index,
    };
    assert_eq!(
        data,
        [
            RequestData::ScrollToPoint(Point::new(100.0, 20.0)),
            RequestData::SetTextSelection(KeySelection { anchor: end(1), focus: end(4) }),
            RequestData::Value("secret value".to_owned()),
        ],
        "the window is at 1.5"
    );
    assert_eq!(hub.report().requests_stale, 1, "a selection in a run that is not there");
    let printed = format!("{requests:?}");
    assert!(!printed.contains("secret"), "{printed}");
}

/// Mutations caught: a model's request with any other origin, a model reaching an undeclared action or a node that
/// is not there.
#[test]
fn a_models_action_is_the_models_and_checked_like_any_other() {
    let mut hub = two_windows();
    let click = hub
        .model_action(A, MODEL, now(), AccessTarget::Node(BUTTON), Action::Click, None)
        .unwrap();
    assert_eq!(click.origin(), Origin::ModelAction(MODEL));
    assert!(click.is_model() && click.focus_by().is_none());
    let focus = hub
        .model_action(A, MODEL, now(), AccessTarget::Node(FIELD), Action::Focus, None)
        .unwrap();
    assert!(matches!(focus.focus_by(), Some(FocusBy::Model(attribution)) if attribution == MODEL));
    assert_eq!(focus.text_target(), Some(TARGET));
    assert!(hub.model_action(A, MODEL, now(), AccessTarget::Window, Action::Focus, None).is_ok());
    let selection = KeySelection {
        anchor: KeyPosition { node: FIELD, character_index: 0 },
        focus: KeyPosition { node: FIELD, character_index: 0 },
    };
    let refused = [
        (WindowKey::from_number(9), AccessTarget::Window, Action::Focus, None),
        (A, AccessTarget::Node(NodeKey(99)), Action::Click, None),
        (A, AccessTarget::Node(BUTTON), Action::Expand, None),
        (A, AccessTarget::Window, Action::Click, None),
        (A, AccessTarget::Node(FIELD), Action::SetTextSelection, Some(RequestData::SetTextSelection(selection))),
    ]
    .into_iter()
    .map(|(window, target, action, data)| hub.model_action(window, MODEL, now(), target, action, data).map(|_| ()))
    .collect::<Vec<_>>();
    assert_eq!(
        refused,
        [
            Err(ActionRefusal::UnknownWindow),
            Err(ActionRefusal::UnknownNode),
            Err(ActionRefusal::Unsupported),
            Err(ActionRefusal::Unsupported),
            Err(ActionRefusal::BadData),
        ]
    );
    assert_eq!(hub.report().model_refused, 5);
}

/// The input crate's focus model both ways: a focus request moves the text focus as another program, and the text
/// focus puts the tree's focus on the node that shows it. Mutation caught: a focus request with the host's or the
/// Operator's say.
#[test]
fn a_focus_request_moves_the_text_focus_and_the_tree_follows_it() {
    let mut hub = two_windows();
    let mut tracker = InputTracker::new();
    let created = Event::Window {
        window: A,
        event: WindowEvent::Created {
            command: 1,
            state: WindowState {
                class: WindowClass::Quiet,
                display: Display::Hidden,
                focused: false,
                outer: PhysicalRect::default(),
                inner: PhysicalSize::new(200, 100),
                scale: ScaleFactor::ONE,
                monitor: None,
                native_tokens: 0,
            },
        },
    };
    let _ = tracker.feed(now(), &created);
    hub.feed(&created);
    ask(&hub, A, Action::Focus, hub.node_id(A, FIELD).unwrap(), None);
    let request = hub.take_requests(now()).pop().unwrap();
    assert_eq!(request.text_target(), Some(TARGET));
    let by = request.focus_by().unwrap();
    assert!(matches!(by, FocusBy::OtherProgram));
    let focus = TextFocus {
        target: request.text_target().unwrap(),
        accepts_ime: true,
        caret: CaretPlace::default(),
    };
    tracker.focus_text(A, Some(focus), by, now()).unwrap();
    assert_eq!(tracker.text_focus(A), Some(TARGET));
    let mut next = frame("save");
    next.focus_on_text(tracker.text_focus(A));
    hub.update(A, next).unwrap();
    assert_eq!(hub.snapshot(A).unwrap().focus, hub.node_id(A, FIELD).unwrap());
}

#[test]
fn an_inspector_reads_the_whole_tree_and_finds_nodes_by_author_id() {
    let mut hub = two_windows();
    let snapshot = hub.snapshot(A).unwrap();
    assert!(snapshot.tree.is_some(), "whole, with the tree's root");
    assert_eq!(snapshot.nodes.len(), 4, "the window, the button, the field and its run");
    assert_eq!(hub.find_author(A, "save-button"), Some(BUTTON));
    assert_eq!(hub.find_author(A, "nothing"), None);
    // The adapter's activation handler gives the same whole tree.
    assert_eq!(hub.source(A).unwrap().request_initial_tree(), Some(snapshot));
    assert_eq!(hub.report().initial_trees, 1);
    // The window's scale and size follow the loop; a window that went is forgotten.
    hub.feed(&Event::Window {
        window: A,
        event: WindowEvent::ScaleChanged {
            from: ScaleFactor::ONE,
            to: ScaleFactor::from_dpi(192).unwrap(),
        },
    });
    let root = hub.snapshot(A).unwrap().nodes[0].1.clone();
    assert_eq!(root.transform().map(|transform| transform.as_coeffs()[0]), Some(2.0));
    hub.feed(&Event::Window {
        window: A,
        event: WindowEvent::Destroyed,
    });
    assert_eq!(hub.windows(), [B]);
}
