//! WP-KERNEL-012 MT-103 — foreground-safe model navigation.
//!
//! The driver under test must steer the live native shell through the existing MCP/AccessKit
//! action-channel path. It must not call OS foreground or input-injection APIs.

use egui::accesskit;
use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::mcp::{ActionChannel, SessionToken, ERR_ACTION_QUEUE_FULL};
use handshake_native::mcp_navigation::{NavigationError, NavigationSequence, NavigationStep};
use handshake_native::theme::HsTheme;

const THEME_TOGGLE_AUTHOR_ID: &str = "shell.chrome.theme-toggle";
const QUICK_LINK_CHAT_AUTHOR_ID: &str = "quick-links.pane-c.0";
const CHAT_INPUT_AUTHOR_ID: &str = "runtime-chat-input";
const CHAT_SEND_AUTHOR_ID: &str = "runtime-chat-send";
const FOCUS_PANE_AUTHOR_ID: &str = "pane-a";

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }))
}

fn shell_harness<'a>() -> Harness<'a, HandshakeApp> {
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run();
    harness
}

fn harness_node_value(
    harness: &Harness<'_, HandshakeApp>,
    target: accesskit::NodeId,
) -> Option<String> {
    let root = harness.kittest_state().root();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.id() == target {
            return node.value();
        }
        for child in node.children() {
            stack.push(child);
        }
    }
    None
}

fn fresh_author_value(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) -> Option<String> {
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let node_id = accesskit::NodeId(snapshot.find_by_author_id(author_id)?.node_id);
    harness_node_value(harness, node_id)
}

fn focused_author_id(harness: &Harness<'_, HandshakeApp>) -> Option<String> {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().is_focused())
        .and_then(|node| node.accesskit_node().author_id().map(str::to_owned))
}

fn dispatch_step_with_fresh_snapshot(
    sequence: &NavigationSequence,
    index: usize,
    token: &SessionToken,
    channel: &mut ActionChannel,
    harness: &mut Harness<'_, HandshakeApp>,
) -> handshake_native::mcp_navigation::NavigationReceipt {
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let receipt = sequence
        .dispatch_step(index, token, "session-secret", &snapshot, channel)
        .unwrap_or_else(|err| panic!("step {index} dispatches against fresh snapshot: {err:?}"));
    let events = channel.drain_into_events();
    assert!(
        events.iter().all(|event| matches!(
            event,
            egui::Event::AccessKitActionRequest(_) | egui::Event::Text(_)
        )),
        "navigation driver only emits egui AccessKit/Text events, never OS input"
    );
    for event in events {
        harness.event(event);
    }
    harness.run();
    harness.run();
    receipt
}

#[test]
fn foreground_safe_navigation_sequence_drives_live_tree_without_os_input() {
    let token = SessionToken::from_hex("session-secret");
    let mut channel = ActionChannel::new();
    let mut harness = shell_harness();

    let before_theme = harness.state().current_theme();
    assert_eq!(before_theme, HsTheme::Dark, "test starts in dark theme");
    assert_eq!(
        fresh_author_value(&mut harness, CHAT_INPUT_AUTHOR_ID).unwrap_or_default(),
        "",
        "runtime chat input starts empty"
    );

    let sequence = NavigationSequence::new(vec![
        NavigationStep::open_pane(QUICK_LINK_CHAT_AUTHOR_ID, "pane-c"),
        NavigationStep::click(THEME_TOGGLE_AUTHOR_ID),
        NavigationStep::set_value(CHAT_INPUT_AUTHOR_ID, "hello from mt-103"),
        NavigationStep::focus(FOCUS_PANE_AUTHOR_ID),
    ]);

    let open = dispatch_step_with_fresh_snapshot(&sequence, 0, &token, &mut channel, &mut harness);
    assert_eq!(open.target, QUICK_LINK_CHAT_AUTHOR_ID);
    assert_eq!(open.action, "Click");
    assert_eq!(open.expected_pane.as_deref(), Some("pane-c"));
    assert_eq!(
        harness.state().active_pane().map(|pane| pane.as_ref()),
        Some("pane-c"),
        "open-pane step routed focus to the chat pane"
    );

    let click = dispatch_step_with_fresh_snapshot(&sequence, 1, &token, &mut channel, &mut harness);
    assert_eq!(click.target, THEME_TOGGLE_AUTHOR_ID);
    assert_eq!(click.action, "Click");
    assert_eq!(
        harness.state().current_theme(),
        HsTheme::Light,
        "click_widget path changed observable shell state"
    );

    let set_value =
        dispatch_step_with_fresh_snapshot(&sequence, 2, &token, &mut channel, &mut harness);
    assert_eq!(set_value.target, CHAT_INPUT_AUTHOR_ID);
    assert_eq!(set_value.action, "Focus");
    assert_eq!(set_value.text_payload.as_deref(), Some("hello from mt-103"));
    assert_eq!(
        fresh_author_value(&mut harness, CHAT_INPUT_AUTHOR_ID).unwrap_or_default(),
        "hello from mt-103",
        "set_value path changed the live Runtime Chat input value found by author_id"
    );

    let focus = dispatch_step_with_fresh_snapshot(&sequence, 3, &token, &mut channel, &mut harness);
    assert_eq!(focus.target, FOCUS_PANE_AUTHOR_ID);
    assert_eq!(focus.action, "Focus");
    assert_eq!(
        focused_author_id(&harness).as_deref(),
        Some(FOCUS_PANE_AUTHOR_ID),
        "focus step changes the focused live AccessKit node"
    );
}

#[test]
fn foreground_safe_navigation_unknown_author_id_is_typed_error() {
    let token = SessionToken::from_hex("session-secret");
    let mut harness = shell_harness();
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let mut channel = ActionChannel::new();
    let sequence = NavigationSequence::new(vec![NavigationStep::click("missing.author-id")]);

    let err = sequence
        .dispatch_step(0, &token, "session-secret", &snapshot, &mut channel)
        .expect_err("unknown author_id returns typed error");
    assert_eq!(channel.pending(), 0, "failed dispatch queues no action");
    assert!(
        matches!(
            &err,
            NavigationError::Tool {
                target,
                message,
                ..
            } if target == "missing.author-id" && message.contains("no live widget")
        ),
        "unknown author_id reports the missing stable target, got {err:?}"
    );
}

#[test]
fn foreground_safe_navigation_wrong_token_queues_nothing() {
    let token = SessionToken::from_hex("session-secret");
    let mut harness = shell_harness();
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let mut channel = ActionChannel::new();
    let sequence = NavigationSequence::new(vec![NavigationStep::click(THEME_TOGGLE_AUTHOR_ID)]);

    let err = sequence
        .dispatch_step(0, &token, "wrong-token", &snapshot, &mut channel)
        .expect_err("wrong token is rejected");
    assert_eq!(err, NavigationError::Unauthorized);
    assert_eq!(
        channel.pending(),
        0,
        "unauthorized dispatch queues no action"
    );
}

#[test]
fn foreground_safe_navigation_disabled_target_is_typed_error() {
    let token = SessionToken::from_hex("session-secret");
    let mut harness = shell_harness();
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let mut channel = ActionChannel::new();
    let send = snapshot
        .find_by_author_id(CHAT_SEND_AUTHOR_ID)
        .expect("runtime chat send button exists");
    assert!(
        send.disabled,
        "empty Runtime Chat draft keeps send disabled"
    );
    let sequence = NavigationSequence::new(vec![NavigationStep::click(CHAT_SEND_AUTHOR_ID)]);

    let err = sequence
        .dispatch_step(0, &token, "session-secret", &snapshot, &mut channel)
        .expect_err("disabled target returns typed error");
    assert_eq!(channel.pending(), 0, "disabled dispatch queues no action");
    assert!(
        matches!(
            &err,
            NavigationError::Tool {
                target,
                message,
                ..
            } if target == CHAT_SEND_AUTHOR_ID && message.contains("disabled")
        ),
        "disabled target reports the stable author_id, got {err:?}"
    );
}

#[test]
fn foreground_safe_navigation_queue_full_is_typed_error() {
    let token = SessionToken::from_hex("session-secret");
    let mut harness = shell_harness();
    let snapshot = harness.state_mut().capture_mcp_snapshot_for_navigation();
    let mut channel = ActionChannel::with_capacity(1);
    let sequence = NavigationSequence::new(vec![
        NavigationStep::click(THEME_TOGGLE_AUTHOR_ID),
        NavigationStep::set_value(CHAT_INPUT_AUTHOR_ID, "queued after full"),
    ]);

    let first = sequence
        .dispatch_step(0, &token, "session-secret", &snapshot, &mut channel)
        .expect("first action queues and returns a receipt before capacity is exhausted");
    assert_eq!(first.target, THEME_TOGGLE_AUTHOR_ID);
    assert_eq!(channel.pending(), 1, "first action remains queued");

    let err = sequence
        .dispatch_step(1, &token, "session-secret", &snapshot, &mut channel)
        .expect_err("second action hits bounded queue capacity");
    assert_eq!(channel.pending(), 1, "first action remains queued");
    assert!(
        matches!(
            &err,
            NavigationError::Tool {
                target,
                code: ERR_ACTION_QUEUE_FULL,
                message,
                ..
            } if target == CHAT_INPUT_AUTHOR_ID && message.contains("action queue full")
        ),
        "queue full returns the typed MCP queue error, got {err:?}"
    );
}
