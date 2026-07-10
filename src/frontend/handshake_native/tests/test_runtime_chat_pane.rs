//! WP-KERNEL-012 MT-098: Runtime Chat pane beside the native editor work surface.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::pane_registry::PaneType;
use handshake_native::runtime_chat::{
    ChatRole, ChatSendError, RuntimeChatClient, RuntimeChatPanel, RUNTIME_CHAT_INPUT_AUTHOR_ID,
    RUNTIME_CHAT_PANEL_AUTHOR_ID, RUNTIME_CHAT_SEND_AUTHOR_ID, RUNTIME_CHAT_STATUS_AUTHOR_ID,
};
use handshake_native::theme::HsTheme;

static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }))
}

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn live_author_nodes(
    harness: &Harness<'_, HandshakeApp>,
) -> HashMap<String, (String, Option<String>, bool)> {
    let mut found = HashMap::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.insert(
                author_id.to_owned(),
                (format!("{:?}", ak.role()), ak.label(), ak.is_disabled()),
            );
        }
    }
    found
}

fn rect_for(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> egui::Rect {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("missing node {author_id}"))
        .rect()
}

#[test]
fn production_runtime_chat_send_returns_typed_endpoint_missing_without_assistant_turn() {
    let client = RuntimeChatClient::production();
    assert_eq!(
        client.probed_path(),
        "/api/runtime_chat/messages",
        "RuntimeChatClient must not target Flight Recorder ingestion as a fake chat backend"
    );
    assert_ne!(
        client.probed_path(),
        "/api/flight_recorder/runtime_chat_event",
        "Flight Recorder runtime-chat event ingestion is observability, not chat send/receive"
    );
    let err = client
        .send("hello")
        .expect_err("production chat route is absent in this build");
    assert!(err.is_endpoint_missing());
    assert!(matches!(
        err,
        ChatSendError::EndpointMissing { ref probed_path } if probed_path == "/api/runtime_chat/messages"
    ));
    let empty = client
        .send("   ")
        .expect_err("empty chat input is validation-blocked, not a successful backend send");
    assert!(empty.is_empty_message());

    let mut panel = RuntimeChatPanel::production(HsTheme::Dark.palette());
    panel.set_draft_for_test("   ");
    let empty = panel
        .send_current_message_for_test()
        .expect_err("empty panel send returns a typed validation blocker");
    assert!(empty.is_empty_message());

    panel.set_draft_for_test("hello from test");
    let err = panel
        .send_current_message_for_test()
        .expect_err("panel send surfaces the same typed blocker");
    assert!(err.is_endpoint_missing());
    assert!(
        !panel
            .turns_for_test()
            .iter()
            .any(|turn| turn.role == ChatRole::Assistant),
        "EndpointMissing must not synthesize an assistant response"
    );
    assert!(
        panel
            .last_error_for_test()
            .is_some_and(ChatSendError::is_endpoint_missing),
        "panel stores the visible typed blocker"
    );
}

// ── MT-108 (MT-098 residual): LIVE type-and-click send observes EndpointMissing ────────────────────

#[test]
fn live_type_and_click_send_observes_endpoint_missing_without_assistant_turn() {
    // Drive the REAL widget end to end: seed a typed draft (the nested frameless singleline is not
    // reliably driven by kittest keystroke simulation, so the message text is seeded through the panel's
    // documented test API — the same "type" input the existing send test uses), render so the Send
    // button reflects the non-empty draft, then CLICK the live Send button through the AccessKit tree.
    // Observe that the live send surfaced the typed `EndpointMissing` blocker (probed route is the
    // aligned `/api/runtime_chat/messages`) and fabricated NO assistant reply. The button click + the
    // observed blocker are the live-UI proof (distinct from the `send_current_message_for_test` shortcut).
    let panel = Arc::new(Mutex::new(RuntimeChatPanel::production(HsTheme::Dark.palette())));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 420.0))
        .build_ui(move |ui| {
            panel_ui.lock().unwrap_or_else(|p| p.into_inner()).show(ui);
        });
    harness.run();

    // Precondition: Send is disabled with an empty draft.
    assert!(
        live_author_nodes_ui(&harness)
            .get(RUNTIME_CHAT_SEND_AUTHOR_ID)
            .is_some_and(|disabled| *disabled),
        "Send starts disabled with an empty draft"
    );

    // Seed the typed message, then render so the Send button reflects the non-empty draft.
    panel
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .set_draft_for_test("hello from the live UI");
    harness.run();
    harness.run();

    // A non-empty draft enabled the live Send button (the rendered UI reflects the message text).
    assert!(
        live_author_nodes_ui(&harness)
            .get(RUNTIME_CHAT_SEND_AUTHOR_ID)
            .is_some_and(|disabled| !*disabled),
        "a non-empty draft enabled the Send button"
    );

    // CLICK the live Send button through the AccessKit tree (the genuine live-UI send trigger).
    let send_id = harness
        .root()
        .children_recursive()
        .find_map(|n| {
            let ak = n.accesskit_node();
            (ak.author_id() == Some(RUNTIME_CHAT_SEND_AUTHOR_ID)).then(|| ak.id())
        })
        .expect("runtime-chat-send node present");
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: send_id,
            data: None,
        },
    ));
    harness.run();
    harness.run();

    // The live send observed the typed EndpointMissing blocker, and no assistant turn was fabricated.
    let p = panel.lock().unwrap_or_else(|e| e.into_inner());
    assert!(
        p.last_error_for_test()
            .is_some_and(ChatSendError::is_endpoint_missing),
        "MT-098: the live click-send surfaced the typed EndpointMissing blocker"
    );
    assert!(
        !p.turns_for_test()
            .iter()
            .any(|turn| turn.role == ChatRole::Assistant),
        "MT-098: a missing endpoint must not fabricate an assistant reply"
    );
    println!(
        "MT-108 (MT-098): live click-send observed EndpointMissing (probed {}) with no assistant turn",
        RuntimeChatClient::production().probed_path()
    );
}

/// author_id -> is_disabled for every live node (a lightweight view for the send-enabled assertions).
fn live_author_nodes_ui(harness: &Harness<'_, ()>) -> HashMap<String, bool> {
    let mut found = HashMap::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.insert(author_id.to_owned(), ak.is_disabled());
        }
    }
    found
}

#[test]
fn live_default_tree_contains_runtime_chat_beside_editors_and_screenshot() {
    let _guard = WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), ok_app());
    harness.run_steps(4);

    let pane_types: HashMap<String, PaneType> = {
        let registry = harness.state().pane_registry();
        let guard = registry.lock().expect("registry");
        guard
            .iter()
            .map(|(id, record)| (id.to_string(), record.pane_type.clone()))
            .collect()
    };
    assert_eq!(pane_types.get("pane-a"), Some(&PaneType::CodeSymbol));
    assert_eq!(pane_types.get("pane-b"), Some(&PaneType::LoomWikiPage));
    assert_eq!(pane_types.get("pane-c"), Some(&PaneType::RuntimeChat));
    assert!(
        !pane_types.contains_key("pane-d"),
        "fresh MT-098 default stays minimal: no pane-d"
    );

    let nodes = live_author_nodes(&harness);
    for (author_id, role) in [
        ("pane-c", "Region"),
        ("tabbar-pane-c", "TabList"),
        ("tab-pane-c-0", "Tab"),
        (RUNTIME_CHAT_PANEL_AUTHOR_ID, "Region"),
        (RUNTIME_CHAT_STATUS_AUTHOR_ID, "Status"),
        (RUNTIME_CHAT_INPUT_AUTHOR_ID, "TextInput"),
        (RUNTIME_CHAT_SEND_AUTHOR_ID, "Button"),
    ] {
        let Some((actual_role, _label, _disabled)) = nodes.get(author_id) else {
            panic!(
                "missing live Runtime Chat author_id {author_id}; found {:?}",
                nodes.keys()
            );
        };
        assert_eq!(actual_role, role, "{author_id} role");
    }
    let status_label = nodes
        .get(RUNTIME_CHAT_STATUS_AUTHOR_ID)
        .and_then(|(_role, label, _disabled)| label.as_deref())
        .expect("runtime-chat-status label");
    assert!(
        status_label.contains("EndpointMissing")
            && status_label.contains("/api/runtime_chat/messages"),
        "runtime-chat-status label must expose typed blocker and probed path: {status_label}"
    );
    assert_eq!(
        nodes
            .get(RUNTIME_CHAT_INPUT_AUTHOR_ID)
            .and_then(|(_role, label, _disabled)| label.as_deref()),
        Some("Runtime Chat message"),
        "runtime-chat-input has an explicit model-readable label"
    );
    assert!(
        nodes
            .get(RUNTIME_CHAT_SEND_AUTHOR_ID)
            .is_some_and(|(_role, _label, disabled)| *disabled),
        "runtime-chat-send is disabled until the draft has non-whitespace text"
    );
    assert!(
        !nodes.contains_key("divider-horizontal"),
        "three-column default should not expose a bottom-row divider"
    );

    let pane_a = rect_for(&harness, "pane-a");
    let pane_b = rect_for(&harness, "pane-b");
    let pane_c = rect_for(&harness, "pane-c");
    assert!(
        pane_a.center().x < pane_b.center().x && pane_b.center().x < pane_c.center().x,
        "Runtime Chat must be beside the editors left-to-right: a={pane_a:?}, b={pane_b:?}, c={pane_c:?}"
    );
    for (id, rect) in [("pane-a", pane_a), ("pane-b", pane_b), ("pane-c", pane_c)] {
        assert!(
            rect.width() > 100.0,
            "{id} is not starved horizontally: {rect:?}"
        );
        assert!(
            rect.height() > 300.0,
            "{id} is full-height enough: {rect:?}"
        );
    }

    let image = harness
        .render()
        .expect("wgpu render succeeds for MT-098 Runtime Chat screenshot");
    assert!(
        image.width() > 0 && image.height() > 0,
        "non-empty screenshot"
    );
    let ext_dir = external_artifact_dir("wp-kernel-012-mt-098");
    std::fs::create_dir_all(&ext_dir).expect("create external artifact dir");
    let png_path = ext_dir.join("MT-098-runtime-chat-default.png");
    image
        .save(&png_path)
        .expect("save MT-098 Runtime Chat screenshot");
    println!("MT-098 screenshot: {}", png_path.display());
}
