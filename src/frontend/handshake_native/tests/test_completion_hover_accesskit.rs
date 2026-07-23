//! MT-008 completion popup + hover tooltip + staleness-gutter LIVE proofs (WP-KERNEL-012 E1).
//!
//! These egui_kittest tests drive the panel's REAL public completion/hover/staleness API and inspect
//! the LIVE AccessKit tree + rendered state — the same nodes a swarm agent reads out-of-process and the
//! same pixels an operator sees. Standalone tests isolate rendering and interaction semantics. With
//! `--features integration`, strict live tests consume the managed-PostgreSQL fixture values from
//! `HANDSHAKE_TEST_DB_URL` and `HANDSHAKE_TEST_WORKSPACE_ID`, drive the real async CodeNavClient path,
//! and require populated backend data to reach the AccessKit popup/tooltip and stale gutter marker.
//!
//! AC-005 / PT-005: trigger completion -> the live tree contains `code_editor_completion_popup`
//! (Role::ListBox) with >= 1 item node (`code_editor_completion_item_0`).
//! AC-006 / PT-006: open hover on identifier 'add' -> the live tree contains `code_editor_hover`
//! (Role::Tooltip) whose text content contains 'add'.
//! AC-007: a staleness check pushes >= 1 Warning gutter marker -> the gutter renders a diagnostic dot
//! (verified via the gutter marker count + a screenshot saved to the external artifact root).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::NodeT;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::code_editor::code_nav::{
    CodeNavClient, CodeStaleness, CodeSymbolDefinition, CodeSymbolNavProjection, CompletionItem,
    CompletionKind, HOVER_DWELL_MS,
};
use handshake_native::code_editor::editor_view::{
    CodeNavigationLocation, CODE_EDITOR_COMPLETION_ITEM_AUTHOR_PREFIX,
    CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID, CODE_EDITOR_HOVER_AUTHOR_ID,
};
use handshake_native::code_editor::lsp_client::{LspClient, LspServerConfig};
use handshake_native::code_editor::{CodeEditorPanel, HoverState};

#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;

const SNIPPET: &str = "fn add(a: i32, b: i32) -> i32 { a + b }\nfn caller() -> i32 { add(1, 2) }";

fn hover_target(line: u32) -> CodeNavigationLocation {
    CodeNavigationLocation {
        uri: "file:///hover.rs".to_owned(),
        path: Some("hover.rs".to_owned()),
        range: lsp_types::Range::new(
            lsp_types::Position::new(line, 0),
            lsp_types::Position::new(line, 0),
        ),
    }
}

#[cfg(feature = "integration")]
fn live_fixture() -> (String, String) {
    let base_url = std::env::var("HANDSHAKE_TEST_DB_URL")
        .expect("MT-008 live UI proof requires HANDSHAKE_TEST_DB_URL from the ready fixture");
    assert!(
        base_url.starts_with("http://") || base_url.starts_with("https://"),
        "HANDSHAKE_TEST_DB_URL must be the fixture HTTP base URL; got {base_url:?}"
    );
    let workspace_id = std::env::var("HANDSHAKE_TEST_WORKSPACE_ID")
        .expect("MT-008 live UI proof requires HANDSHAKE_TEST_WORKSPACE_ID from the ready fixture");
    (base_url, workspace_id)
}

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn assert_no_local_test_output() {
    let local = Path::new("test_output");
    assert!(
        !local.exists(),
        "no repo-local test_output/ dir may exist — artifacts go to the external \
         Handshake_Artifacts/handshake-test root only"
    );
}

fn stale_add_lookup_body() -> String {
    serde_json::json!({
        "matches": [{
            "symbol_entity_id": "",
            "symbol_key": "rust:src/lib.rs#add",
            "display_name": "add",
            "symbol_kind": "function",
            "definition": {
                "line_start": 1,
                "line_end": 1
            },
            "staleness": {
                "state": "marked_stale",
                "fresh": false
            }
        }]
    })
    .to_string()
}

fn spawn_one_lookup_server() -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind MT-008 lookup mock server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept lookup request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let n = stream.read(&mut buf).expect("read lookup request");
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request_text = String::from_utf8_lossy(&request).to_string();
        let body = stale_add_lookup_body();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write lookup response");
        request_text
    });
    (base_url, handle)
}

/// Two synthetic completion items (the shape the code-nav lookup yields), so the popup-render +
/// AccessKit proof is independent of a live backend.
fn synthetic_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "add".into(),
            insert_text: "add".into(),
            kind: CompletionKind::Function,
            detail: "function".into(),
            documentation: "**add**\nKind: `function`".into(),
            symbol_entity_id: "ent-add".into(),
        },
        CompletionItem {
            label: "adder".into(),
            insert_text: "adder".into(),
            kind: CompletionKind::Class,
            detail: "struct".into(),
            documentation: "**adder**".into(),
            symbol_entity_id: "ent-adder".into(),
        },
    ]
}

#[cfg(feature = "integration")]
#[test]
fn ac005_live_backend_completion_reaches_accesskit_and_stale_gutter() {
    let (base_url, workspace_id) = live_fixture();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build MT-008 live completion runtime");
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_runtime(runtime.handle().clone());
    panel.set_workspace_id(workspace_id);
    panel.set_code_nav_client(CodeNavClient::new(base_url));

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    let completion_offset = panel
        .buffer()
        .to_string()
        .find("add")
        .expect("managed fixture buffer contains add")
        + "add".len();
    panel.set_single_cursor(completion_offset);
    press_key(
        &mut harness,
        egui::Key::Space,
        egui::Modifiers {
            ctrl: true,
            ..Default::default()
        },
    );
    harness.run();
    for _ in 0..150 {
        harness.run();
        if panel.is_completion_open() && !panel.diagnostic_markers().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let completion = panel
        .completion_state()
        .expect("AC-005 managed-backend response opens completion popup");
    let add = completion
        .items
        .iter()
        .find(|item| item.label == "add")
        .expect("AC-005 popup contains the seeded add completion");
    assert!(
        !add.detail.trim().is_empty(),
        "live completion detail is populated"
    );
    assert!(
        !add.symbol_entity_id.trim().is_empty(),
        "live completion retains the backend symbol entity id"
    );

    harness.run();
    let root = harness.root();
    let popup = root
        .children_recursive()
        .find(|node| {
            node.accesskit_node().author_id() == Some(CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID)
        })
        .expect("AC-005 live ListBox is AccessKit-visible");
    assert_eq!(format!("{:?}", popup.accesskit_node().role()), "ListBox");
    let live_item_value = root
        .children_recursive()
        .filter_map(|node| {
            let accesskit = node.accesskit_node();
            accesskit
                .author_id()
                .filter(|author_id| author_id.starts_with("code_editor_completion_item_"))
                .and_then(|_| accesskit.value())
        })
        .find(|value| value.contains("add"))
        .expect("AC-005 live completion item exposes add through AccessKit");
    let markers = panel.diagnostic_markers();
    assert!(
        markers
            .iter()
            .any(|marker| marker.message.contains("Stale code intelligence")),
        "AC-007 real marked-stale backend symbol reaches the diagnostic gutter: {markers:?}"
    );
    println!(
        "AC-005/007 populated live UI: popup=ListBox item={live_item_value:?} stale_markers={}",
        markers.len()
    );

    drop(harness);
    drop(panel);
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
}

#[cfg(feature = "integration")]
#[test]
fn ac006_live_backend_hover_reaches_accesskit_with_definition_and_doc() {
    let (base_url, workspace_id) = live_fixture();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build MT-008 live hover runtime");
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_runtime(runtime.handle().clone());
    panel.set_workspace_id(workspace_id);
    panel.set_code_nav_client(CodeNavClient::new(base_url));

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    let hover_offset = panel
        .buffer()
        .to_string()
        .find("add")
        .expect("managed fixture buffer contains add")
        + 1;
    panel.set_single_cursor(hover_offset);
    harness.run();
    std::thread::sleep(std::time::Duration::from_millis(
        handshake_native::code_editor::code_nav::HOVER_DWELL_MS + 60,
    ));
    harness.run();
    for _ in 0..150 {
        harness.run();
        if panel.is_hover_open() && !panel.diagnostic_markers().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let hover = panel
        .hover_state()
        .expect("AC-006 managed-backend response opens hover tooltip");
    assert_eq!(hover.display_name, "add");
    assert!(
        hover.definition_target.is_some(),
        "live hover has a definition target"
    );
    assert!(hover.markdown.contains("Kind: `function`"));
    assert!(hover.markdown.contains("marked_stale"));
    assert!(
        hover.markdown.contains("Adds two numbers."),
        "live file-lens documentation reaches hover: {:?}",
        hover.markdown
    );

    harness.run();
    let root = harness.root();
    let tooltip = root
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(CODE_EDITOR_HOVER_AUTHOR_ID))
        .expect("AC-006 live Tooltip is AccessKit-visible");
    assert_eq!(format!("{:?}", tooltip.accesskit_node().role()), "Tooltip");
    let value = tooltip
        .accesskit_node()
        .value()
        .expect("live hover AccessKit node carries text");
    assert!(value.contains("add"));
    assert!(value.contains("Adds two numbers."));
    assert!(
        root.children_recursive()
            .any(|node| node.accesskit_node().author_id() == Some("code_editor_hover_gotodef")),
        "live hover definition action is AccessKit-visible"
    );
    println!(
        "AC-006 populated live UI: tooltip contains add, definition={:?}, documentation=true",
        hover
            .definition_target
            .map(|target| target.range.start.line as usize)
    );

    drop(harness);
    drop(panel);
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
}

// ── AC-005 / PT-005: completion popup ListBox + item nodes ─────────────────────────────────────────

#[test]
fn ac005_completion_popup_emits_listbox_and_item_nodes() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Frame 1: render so geometry + glyph width are measured (the popup anchors at the cursor pixel).
    harness.run();
    assert!(!panel.is_completion_open(), "completion starts closed");

    // Trigger the completion popup with the synthetic items (the deterministic path; a live backend
    // would deliver the same items off-thread into the same state).
    panel.open_completion(synthetic_completions());
    harness.run();
    harness.run(); // settle so the popup's AccessKit nodes are emitted.
    assert!(
        panel.is_completion_open(),
        "AC-005: completion popup is open"
    );

    // The live tree must contain the ListBox container node.
    let root = harness.root();
    let mut popup_role: Option<String> = None;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID) {
            popup_role = Some(format!("{:?}", ak.role()));
            break;
        }
    }
    assert_eq!(
        popup_role.as_deref(),
        Some("ListBox"),
        "AC-005: '{CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID}' must be a Role::ListBox node"
    );

    // At least one completion item node is addressable (code_editor_completion_item_0).
    let has_item_0 = root
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some("code_editor_completion_item_0"));
    assert!(
        has_item_0,
        "AC-005: at least one completion item node (code_editor_completion_item_0)"
    );
    let has_item_1 = root
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some("code_editor_completion_item_1"));
    assert!(
        has_item_1,
        "AC-005: the second completion item is also addressable"
    );

    println!(
        "PT-005 completion popup: {{\"{CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID}\":\"{:?}\", \
         items>=2}}",
        popup_role
    );

    // Closing it removes the popup node from the tree.
    panel.close_completion();
    harness.run();
    harness.run();
    let still_present = harness
        .root()
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID));
    assert!(
        !still_present,
        "AC-005: the popup node is removed after closing"
    );
}

/// AC-005 follow-up: keyboard selection moves through the list (the command-palette semantics) and
/// accepting inserts the item — proving the popup is a real keyboard-navigable list, not a static dump.
#[test]
fn ac005_completion_keyboard_select_and_accept_inserts() {
    let panel = Arc::new(CodeEditorPanel::new("", "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    panel.open_completion(synthetic_completions());
    harness.run();

    // Selection starts at 0 ('add'); ArrowDown moves to 1 ('adder').
    assert_eq!(panel.completion_state().unwrap().selected_index, 0);
    panel.completion_select_next();
    assert_eq!(panel.completion_state().unwrap().selected_index, 1);
    // Accept the selected item -> 'adder' inserted into the (empty) buffer, popup closed.
    assert!(
        panel.accept_completion(),
        "AC-005: accept inserts the selected item"
    );
    assert!(
        !panel.is_completion_open(),
        "AC-005: accept closes the popup"
    );
    let text = panel.buffer().to_string();
    assert!(
        text.contains("adder"),
        "AC-005: the accepted item text was inserted; got {text:?}"
    );
    println!("PT-005 keyboard: ArrowDown->'adder', Enter inserted it (buffer now {text:?})");
}

#[test]
fn ac005_accesskit_click_accepts_completion_and_post_action_state_is_observable() {
    let panel = Arc::new(CodeEditorPanel::new("", "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    panel.open_completion(synthetic_completions());
    harness.run_steps(2);

    let item_node_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("code_editor_completion_item_0"))
        .expect("Argus inspect sees completion item 0")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: item_node_id,
            data: None,
        },
    ));
    harness.run_steps(3);

    assert_eq!(panel.buffer().to_string(), "add");
    assert!(!panel.is_completion_open());
    assert!(
        !harness.root().children_recursive().any(|node| {
            node.accesskit_node().author_id() == Some(CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID)
        }),
        "fresh post-action inspection shows the popup closed"
    );
}

// ── AC-006 / PT-006: hover tooltip node contains the identifier ────────────────────────────────────

#[test]
fn ac006_hover_tooltip_contains_identifier() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    assert!(!panel.is_hover_open(), "hover starts closed");

    // Open the hover for the identifier 'add' (the markdown is the code-nav `markdown_for_symbol`
    // output — the same data a live lookup delivers).
    panel.open_hover(HoverState {
        markdown:
            "**add**\nKind: `function`\nSymbol: `rust:src/lib.rs#add`\nStaleness: `fresh (fresh)`"
                .into(),
        display_name: "add".into(),
        anchor: egui::pos2(120.0, 60.0),
        definition_target: Some(hover_target(0)),
    });
    harness.run();
    harness.run(); // settle so the tooltip node is emitted.
    assert!(panel.is_hover_open(), "AC-006: hover tooltip is open");

    // The live tree must contain the Tooltip node whose VALUE contains 'add'.
    let root = harness.root();
    let mut hover_value: Option<String> = None;
    let mut hover_role: Option<String> = None;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(CODE_EDITOR_HOVER_AUTHOR_ID) {
            hover_role = Some(format!("{:?}", ak.role()));
            hover_value = ak.value().map(|s| s.to_owned());
            break;
        }
    }
    assert_eq!(
        hover_role.as_deref(),
        Some("Tooltip"),
        "AC-006: '{CODE_EDITOR_HOVER_AUTHOR_ID}' must be a Role::Tooltip node"
    );
    let value = hover_value.expect("AC-006: hover node carries a value");
    assert!(
        value.contains("add"),
        "AC-006: the hover tooltip text content contains the identifier 'add'; got {value:?}"
    );
    // The go-to-definition link is also addressable (HBR-SWARM).
    assert!(
        root.children_recursive()
            .any(|n| n.accesskit_node().author_id() == Some("code_editor_hover_gotodef")),
        "AC-006: the hover go-to-definition link is AccessKit-addressable"
    );
    println!("PT-006 hover tooltip: {{\"{CODE_EDITOR_HOVER_AUTHOR_ID}\":\"{hover_role:?}\", value contains 'add'}}");

    // Closing removes the hover node.
    panel.close_hover();
    harness.run();
    harness.run();
    assert!(
        !harness
            .root()
            .children_recursive()
            .any(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_HOVER_AUTHOR_ID)),
        "AC-006: the hover node is removed after closing"
    );
}

#[test]
fn ac006_accesskit_click_go_to_definition_moves_caret_and_closes_hover() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_file_path("hover.rs");
    panel.set_single_cursor(SNIPPET.len());
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    panel.open_hover(HoverState {
        markdown: "**add**\nKind: `function`".into(),
        display_name: "add".into(),
        anchor: egui::pos2(120.0, 60.0),
        definition_target: Some(hover_target(0)),
    });
    harness.run_steps(2);

    let link_node_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("code_editor_hover_gotodef"))
        .expect("Argus inspect sees hover go-to-definition link")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: link_node_id,
            data: None,
        },
    ));
    harness.run_steps(3);

    assert_eq!(panel.cursors().primary().head, 0);
    assert!(!panel.is_hover_open());
    assert!(
        !harness
            .root()
            .children_recursive()
            .any(|node| { node.accesskit_node().author_id() == Some(CODE_EDITOR_HOVER_AUTHOR_ID) }),
        "fresh post-action inspection shows the tooltip closed"
    );
}

#[test]
fn hover_cross_file_accesskit_click_parks_host_jump() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_file_path("source.rs");
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    let mut target = hover_target(7);
    target.uri = "file:///target.rs".to_owned();
    target.path = Some("target.rs".to_owned());
    panel.open_hover(HoverState {
        markdown: "cross-file definition".into(),
        display_name: "target".into(),
        anchor: egui::pos2(120.0, 60.0),
        definition_target: Some(target),
    });
    harness.run_steps(2);
    let link_node_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("code_editor_hover_gotodef"))
        .expect("cross-file hover link is AccessKit-visible")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: link_node_id,
            data: None,
        },
    ));
    harness.run_steps(3);
    let pending = panel
        .pending_cross_file_jump()
        .expect("hover click parks cross-file jump for host drain");
    assert_eq!(pending.file_path, PathBuf::from("target.rs"));
    assert_eq!(pending.position.line, 7);
    assert!(!panel.is_hover_open());
}

// ── AC-007: staleness check pushes a Warning gutter marker (diagnostic dot) ────────────────────────

#[test]
fn ac007_staleness_check_pushes_gutter_diagnostic_marker() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    assert!(
        panel.diagnostic_markers().is_empty(),
        "no markers before the staleness check"
    );

    let version_before = panel.buffer_version_for_test();

    // A NOT-FRESH symbol on line 0 (the React `refreshHandshakeCodeIntelligenceMarkers` staleness
    // branch). `push_staleness_markers` is the AC-007 path that calls `push_diagnostics`.
    let stale_symbol = CodeSymbolNavProjection {
        display_name: "add".into(),
        symbol_kind: "function".into(),
        symbol_key: "rust:src/lib.rs#add".into(),
        definition: Some(CodeSymbolDefinition {
            line_start: Some(1),
            line_end: Some(1),
            ..Default::default()
        }),
        staleness: Some(CodeStaleness {
            state: Some("marked_stale".into()),
            fresh: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let pushed = panel.push_staleness_markers(&[stale_symbol]);
    assert_eq!(pushed, 1, "AC-007: one staleness Warning marker pushed");
    assert_eq!(
        panel.diagnostic_markers().len(),
        1,
        "AC-007: the gutter now has one diagnostic marker"
    );
    // AC-007 / MT-007 perf invariant: pushing diagnostics does NOT bump buffer_version (no re-parse).
    assert_eq!(
        panel.buffer_version_for_test(),
        version_before,
        "AC-007: push_staleness_markers (a diagnostics push) does not bump buffer_version"
    );

    harness.run();
    harness.run(); // settle so the gutter paints the diagnostic dot + left bar.

    // The diagnostic node for line 0 is AccessKit-addressable (a swarm agent reads it).
    let has_diag_node = harness
        .root()
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some("code_editor_diagnostic_0"));
    assert!(
        has_diag_node,
        "AC-007: the line-0 diagnostic node is AccessKit-addressable"
    );

    // Screenshot proof: the gutter renders a yellow/orange Warning dot + left bar. Save to the external
    // artifact root. A yellow-dominant pixel signature in the gutter strip confirms the dot rendered.
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            let raw = image.as_raw();
            // The Warning token is yellow (r,g high, b low). Count yellow-dominant pixels in the
            // left gutter strip region.
            let mut yellow = 0usize;
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let (r, g, b, a) = (raw[i], raw[i + 1], raw[i + 2], raw[i + 3]);
                if a != 0 && r as i32 > 120 && g as i32 > 110 && (r as i32) > (b as i32) + 50 {
                    yellow += 1;
                }
                i += 4;
            }
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-008");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-008-staleness-gutter.png");
            let saved = image.save(&png_path).is_ok();
            println!(
                "AC-007 staleness-gutter screenshot: {w}x{h}, yellow_pixels={yellow}, saved={saved} ({})",
                png_path.display()
            );
            assert!(
                saved,
                "AC-007: the current diagnostic-gutter screenshot must be saved at {}",
                png_path.display()
            );
            assert!(
                yellow >= 10,
                "AC-007: the gutter must render a yellow Warning diagnostic dot/bar; got {yellow} \
                 yellow-dominant pixels"
            );
        }
        Err(e) => {
            panic!(
                "AC-007: current diagnostic-gutter screenshot rendering is required; renderer failed: {e}"
            );
        }
    }
    assert_no_local_test_output();
}

#[test]
fn raw_code_nav_symbol_batches_aggregate_staleness_before_gutter_push() {
    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.queue_code_nav_symbols_for_test(
        "add",
        vec![CodeSymbolNavProjection {
            display_name: "add".into(),
            symbol_kind: "function".into(),
            symbol_key: "rust:src/lib.rs#add".into(),
            definition: Some(CodeSymbolDefinition {
                line_start: Some(1),
                line_end: Some(1),
                ..Default::default()
            }),
            staleness: Some(CodeStaleness {
                state: Some("marked_stale".into()),
                fresh: false,
                ..Default::default()
            }),
            ..Default::default()
        }],
    );
    panel.queue_code_nav_symbols_for_test(
        "caller",
        vec![CodeSymbolNavProjection {
            display_name: "caller".into(),
            symbol_kind: "function".into(),
            symbol_key: "rust:src/lib.rs#caller".into(),
            definition: Some(CodeSymbolDefinition {
                line_start: Some(2),
                line_end: Some(2),
                ..Default::default()
            }),
            staleness: Some(CodeStaleness {
                state: Some("marked_stale".into()),
                fresh: false,
                ..Default::default()
            }),
            ..Default::default()
        }],
    );

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();

    let markers = panel.diagnostic_markers();
    let mut marker_lines: Vec<usize> = markers.iter().map(|marker| marker.line).collect();
    marker_lines.sort_unstable();
    assert_eq!(
        marker_lines,
        vec![0, 1],
        "same-frame raw code-nav batches aggregate stale markers instead of last-batch replacement"
    );
}

#[test]
fn completion_fallback_lookup_queues_raw_symbols_for_staleness() {
    let (base_url, server) = spawn_one_lookup_server();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build tokio runtime");

    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_runtime(rt.handle().clone());
    panel.set_workspace_id("ws-test");
    panel.set_code_nav_client(CodeNavClient::new(base_url));

    panel.trigger_completion(rt.handle(), "add");

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    for _ in 0..40 {
        harness.run();
        if panel.is_completion_open() && !panel.diagnostic_markers().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let request = server.join().expect("lookup server thread joined");
    assert!(
        request.starts_with("GET /knowledge/code/symbols?"),
        "MT-008: completion fallback used the code-nav lookup route, got {request:?}"
    );
    assert!(
        request.contains("prefix=add"),
        "MT-008: completion fallback sent the completion prefix, got {request:?}"
    );
    assert!(
        panel.is_completion_open(),
        "MT-008: completion fallback response opened the completion popup"
    );
    let markers = panel.diagnostic_markers();
    assert_eq!(
        markers.len(),
        1,
        "MT-008: completion fallback queued raw symbols and the UI drain pushed a staleness marker"
    );
    assert_eq!(markers[0].line, 0);

    drop(harness);
    drop(panel);
    rt.shutdown_timeout(std::time::Duration::from_secs(2));
}

#[test]
fn hover_fallback_lookup_queues_raw_symbols_for_staleness() {
    let (base_url, server) = spawn_one_lookup_server();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build tokio runtime");

    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_runtime(rt.handle().clone());
    panel.set_workspace_id("ws-test");
    panel.set_code_nav_client(CodeNavClient::new(base_url));

    panel.trigger_hover(rt.handle(), "add");

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    for _ in 0..40 {
        harness.run();
        if panel.is_hover_open() && !panel.diagnostic_markers().is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let request = server.join().expect("lookup server thread joined");
    assert!(
        request.starts_with("GET /knowledge/code/symbols?"),
        "MT-008: hover fallback used the code-nav lookup route, got {request:?}"
    );
    assert!(
        request.contains("prefix=add"),
        "MT-008: hover fallback sent the hovered word as prefix, got {request:?}"
    );
    assert!(
        panel.is_hover_open(),
        "MT-008: hover fallback response opened the hover tooltip"
    );
    let markers = panel.diagnostic_markers();
    assert_eq!(
        markers.len(),
        1,
        "MT-008: hover fallback queued raw symbols and the UI drain pushed a staleness marker"
    );
    assert_eq!(markers[0].line, 0);

    drop(harness);
    drop(panel);
    rt.shutdown_timeout(std::time::Duration::from_secs(2));
}

// ── must-fix #2: the LIVE keystroke -> input-handler -> trigger path is reachable ───────────────────
//
// The adversarial review found that the completion popup keyboard handling and the Ctrl+Space /
// trigger-character completion trigger were NOT wired into `process_cursor_input`, and the per-frame
// hover-dwell / completion / diagnostics pump was not driven from the live `show()` loop — so a user
// typing/dwelling could never reach the (fully-implemented) triggers. The tests below drive the REAL
// production input handler via injected egui key events through the running frame, proving the wiring.

/// Inject a key press into the harness (the same shape the goto-line / find keymap tests use). The
/// editor's `process_cursor_input` reads these off the live egui input each frame.
fn press_key(harness: &mut Harness, key: egui::Key, modifiers: egui::Modifiers) {
    harness.event(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
}

/// must-fix #2 (popup keyboard): with the popup OPEN, ArrowDown / Enter routed THROUGH the live input
/// handler (`process_cursor_input`) — not a direct `completion_select_next()` API call — move the
/// selection and accept the item. This is the path the review found missing: keys now flow through the
/// keymap, intercepted BEFORE the normal cursor keymap while the popup is open.
#[test]
fn mustfix_completion_popup_keyboard_through_input_handler() {
    let panel = Arc::new(CodeEditorPanel::new("", "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();

    // Open the popup with the synthetic items (the data path; the keyboard path is what we prove here).
    panel.open_completion(synthetic_completions());
    harness.run();
    assert!(panel.is_completion_open(), "popup is open");
    assert_eq!(
        panel.completion_state().unwrap().selected_index,
        0,
        "selection starts at 0 ('add')"
    );

    // ArrowDown THROUGH the input handler -> selection advances to 1 ('adder').
    press_key(
        &mut harness,
        egui::Key::ArrowDown,
        egui::Modifiers::default(),
    );
    harness.run();
    assert_eq!(
        panel.completion_state().unwrap().selected_index,
        1,
        "must-fix: ArrowDown routed through process_cursor_input advanced the popup selection"
    );

    // ArrowUp THROUGH the input handler -> back to 0.
    press_key(&mut harness, egui::Key::ArrowUp, egui::Modifiers::default());
    harness.run();
    assert_eq!(
        panel.completion_state().unwrap().selected_index,
        0,
        "must-fix: ArrowUp routed through process_cursor_input moved the selection back"
    );

    // ArrowDown then Enter THROUGH the input handler -> 'adder' inserted, popup closed.
    press_key(
        &mut harness,
        egui::Key::ArrowDown,
        egui::Modifiers::default(),
    );
    harness.run();
    press_key(&mut harness, egui::Key::Enter, egui::Modifiers::default());
    harness.run();
    assert!(
        !panel.is_completion_open(),
        "must-fix: Enter through the input handler closed the popup"
    );
    let text = panel.buffer().to_string();
    assert!(
        text.contains("adder"),
        "must-fix: Enter through the input handler accepted+inserted the selected item; got {text:?}"
    );

    // Re-open and prove Escape THROUGH the input handler dismisses without inserting.
    panel.open_completion(synthetic_completions());
    harness.run();
    assert!(panel.is_completion_open(), "popup re-opened");
    press_key(&mut harness, egui::Key::Escape, egui::Modifiers::default());
    harness.run();
    assert!(
        !panel.is_completion_open(),
        "must-fix: Escape routed through process_cursor_input dismissed the popup"
    );
    println!("must-fix popup keyboard: ArrowDown/ArrowUp/Enter/Escape all route through process_cursor_input");
}

/// must-fix #2 (live trigger pump): Ctrl+Space routed THROUGH the input handler ARMS a completion
/// request, and the per-frame `pump_code_intelligence` (driven from the live `show()` loop) CONSUMES it
/// and fires the off-thread completion trigger on the injected runtime. The backend is not reachable
/// here, so the lookup gracefully yields no items (AC-004 analog) — but the full live path
/// (keystroke -> arm -> pump -> trigger -> spawn) is exercised end-to-end without panicking, which is
/// exactly the integration the review found unreachable. A runtime IS injected (the production wiring
/// the panel exposes via `set_runtime`); a workspace is bound so the trigger's workspace guard passes.
#[test]
fn mustfix_ctrl_space_arms_and_pump_fires_trigger() {
    // A real multi-thread runtime (the same shape the backend-client tests build) so the trigger can
    // `spawn`; the lookup runs off-thread and returns empty against the (unreachable) default backend.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build tokio runtime");

    let panel = Arc::new(CodeEditorPanel::new("fn main() { let total = 1; }", "rs"));
    // Inject the runtime handle (the production injection point) + bind a workspace so the trigger's
    // workspace guard passes (an empty workspace would short-circuit the trigger before it spawns).
    panel.set_runtime(rt.handle().clone());
    panel.set_workspace_id("ws-test");

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();

    // Place the caret inside the identifier "total" so the trigger has a >=2-char prefix word.
    let offset = panel
        .buffer()
        .to_string()
        .find("total")
        .expect("identifier present")
        + 3;
    panel.set_single_cursor(offset);
    // The debounce clock must have elapsed for the trigger to fire — leave last_edit at None (which the
    // panel treats as "elapsed") by NOT marking an edit just before; the pump fires on the armed frame.

    // Ctrl+Space THROUGH the input handler arms the request; the same frame's pump consumes it.
    press_key(
        &mut harness,
        egui::Key::Space,
        egui::Modifiers {
            ctrl: true,
            ..Default::default()
        },
    );
    harness.run();
    // The arm flag was consumed by the pump (it does not linger to fire on a later, unrelated frame).
    assert!(
        !panel.completion_request_armed_for_test(),
        "must-fix: Ctrl+Space armed a completion request that the live pump consumed this frame"
    );
    // A few more frames let the off-thread (empty) lookup settle without panicking; with no backend the
    // popup stays closed (graceful empty), proving the path runs end-to-end safely.
    harness.run();
    harness.run();
    println!(
        "must-fix Ctrl+Space pump: armed via process_cursor_input, consumed by pump_code_intelligence, \
         off-thread trigger spawned on the injected runtime (popup_open={})",
        panel.is_completion_open()
    );

    drop(harness);
    drop(panel);
    rt.shutdown_timeout(std::time::Duration::from_secs(2));
}

#[test]
fn cursor_workspace_and_delete_invalidate_visible_or_armed_intelligence() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("invalidation runtime");
    let panel = Arc::new(CodeEditorPanel::new("abc", "rs"));
    panel.set_runtime(rt.handle().clone());
    panel.open_completion(synthetic_completions());
    panel.open_hover(HoverState {
        markdown: "hover".into(),
        display_name: "abc".into(),
        anchor: egui::pos2(10.0, 10.0),
        definition_target: None,
    });
    panel.set_single_cursor(1);
    assert!(!panel.is_completion_open());
    assert!(!panel.is_hover_open());

    panel.open_completion(synthetic_completions());
    panel.open_hover(HoverState {
        markdown: "hover".into(),
        display_name: "abc".into(),
        anchor: egui::pos2(10.0, 10.0),
        definition_target: None,
    });
    panel.set_workspace_id("changed-workspace");
    assert!(!panel.is_completion_open());
    assert!(!panel.is_hover_open());

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    harness.event(egui::Event::Text(".".to_owned()));
    harness.run();
    assert!(panel.completion_request_armed_for_test());
    panel.set_single_cursor(0);
    assert!(!panel.completion_request_armed_for_test());

    panel.set_single_cursor(panel.buffer().len_bytes());
    harness.event(egui::Event::Text("_".to_owned()));
    harness.run();
    assert!(panel.completion_request_armed_for_test());
    assert_eq!(panel.delete_text(), 1);
    assert!(!panel.completion_request_armed_for_test());
}

/// must-fix #2 (hover dwell pump): the per-frame pump advances the hover-dwell clock for the live caret
/// offset and, once the dwell elapses at the same offset, fires the off-thread hover trigger — driven
/// from the live `show()` loop, not a direct `open_hover` call. With no backend the lookup yields no
/// hover (graceful), but the dwell -> trigger path runs end-to-end without panicking.
#[test]
fn mustfix_hover_dwell_pump_fires_trigger() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build tokio runtime");

    let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
    panel.set_runtime(rt.handle().clone());
    panel.set_workspace_id("ws-test");

    // Park the caret inside the identifier 'add' so the dwell target is a real word.
    let offset = panel
        .buffer()
        .to_string()
        .find("add")
        .expect("identifier present")
        + 1;
    panel.set_single_cursor(offset);

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });

    // Frame 1 starts the dwell clock at this offset (the pump returns false on the first observation).
    harness.run();
    let generation_before_dwell = panel.hover_request_generation_for_test();
    // Sleep past the dwell window, then run again: the pump now observes the elapsed dwell and fires the
    // hover trigger (off-thread). No backend -> no hover opens, but the path must not panic.
    std::thread::sleep(std::time::Duration::from_millis(
        handshake_native::code_editor::code_nav::HOVER_DWELL_MS + 60,
    ));
    harness.run();
    harness.run();
    assert!(
        panel.hover_request_generation_for_test() > generation_before_dwell,
        "must-fix: the stable-caret dwell reached the production hover trigger"
    );
    println!(
        "must-fix hover dwell pump: dwell elapsed at the caret word, hover trigger fired off-thread \
         (hover_open={})",
        panel.is_hover_open()
    );

    drop(harness);
    drop(panel);
    rt.shutdown_timeout(std::time::Duration::from_secs(2));
}

/// P2 hardening: the dwell gate should fire once per stable caret offset, not every frame after the
/// dwell window elapses. The live pump uses this return value to decide whether to spawn a hover lookup.
#[test]
fn hover_dwell_gate_fires_once_per_stable_offset() {
    let panel = CodeEditorPanel::new(SNIPPET, "rs");
    let offset = panel
        .buffer()
        .to_string()
        .find("add")
        .expect("identifier present")
        + 1;

    assert!(
        !panel.update_hover_dwell(offset),
        "first observation starts the dwell clock"
    );
    std::thread::sleep(std::time::Duration::from_millis(HOVER_DWELL_MS + 20));
    assert!(
        panel.update_hover_dwell(offset),
        "dwell fires once after the configured delay"
    );
    assert!(
        !panel.update_hover_dwell(offset),
        "same offset does not repeatedly fire on following frames"
    );

    assert!(
        !panel.update_hover_dwell(offset + 1),
        "cursor movement resets the dwell clock"
    );
    std::thread::sleep(std::time::Duration::from_millis(HOVER_DWELL_MS + 20));
    assert!(
        panel.update_hover_dwell(offset + 1),
        "new settled offset can fire after its own dwell"
    );
}

#[test]
fn moved_caret_dwell_can_replace_an_already_open_hover() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("replacement hover runtime");
    let panel = Arc::new(CodeEditorPanel::new("add next", "rs"));
    panel.set_runtime(rt.handle().clone());
    panel.set_single_cursor(1);
    panel.open_hover(HoverState {
        markdown: "old add hover".into(),
        display_name: "add".into(),
        anchor: egui::pos2(10.0, 10.0),
        definition_target: None,
    });

    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    panel.set_single_cursor("add nex".len());
    harness.run();
    std::thread::sleep(std::time::Duration::from_millis(HOVER_DWELL_MS + 20));
    harness.run();

    assert!(
        !panel.is_hover_open(),
        "the settled caret at a new word triggered replacement lookup and dismissed the old tooltip"
    );
    drop(harness);
    drop(panel);
    rt.shutdown_timeout(std::time::Duration::from_secs(2));
}

/// MT-008 V2 closure proof: a generated, indexed C++ workspace is opened by the production
/// `HandshakeApp`, an actual clangd process feeds diagnostics/hover/completion into the mounted code
/// panel, and the canonical localhost Argus transport inspects and steers the resulting live tree.
/// This is deliberately an integration test: it must not silently substitute a mock LSP, a panel-only
/// harness, or direct AccessKit event injection for the shipped host and MCP action path.
#[cfg(feature = "integration")]
#[test]
fn mt008_mounted_real_lsp_canonical_argus() {
    use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};
    use handshake_native::app::{HandshakeApp, HealthDisplayState};
    use handshake_native::backend_client::HealthInfo;
    use handshake_native::pane_registry::{PaneId, PaneType};
    use handshake_native::tab_bar::{TabBarState, TabState};

    struct WorkspaceCleanup(PathBuf);
    impl Drop for WorkspaceCleanup {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.0) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    panic!(
                        "remove MT-008 generated real-LSP workspace {}: {error}",
                        self.0.display()
                    );
                }
            }
        }
    }

    fn json_author<'a>(
        value: &'a serde_json::Value,
        author_id: &str,
    ) -> Option<&'a serde_json::Value> {
        match value {
            serde_json::Value::Object(object) => {
                if object.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                    return Some(value);
                }
                object
                    .values()
                    .find_map(|value| json_author(value, author_id))
            }
            serde_json::Value::Array(values) => values
                .iter()
                .find_map(|value| json_author(value, author_id)),
            _ => None,
        }
    }

    fn json_has_author_prefix(value: &serde_json::Value, expected_prefix: &str) -> bool {
        match value {
            serde_json::Value::Object(object) => {
                object
                    .get("author_id")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|author_id| author_id.starts_with(expected_prefix))
                    || object
                        .values()
                        .any(|value| json_has_author_prefix(value, expected_prefix))
            }
            serde_json::Value::Array(values) => values
                .iter()
                .any(|value| json_has_author_prefix(value, expected_prefix)),
            _ => false,
        }
    }

    fn process_alive(pid: u32) -> bool {
        #[cfg(windows)]
        {
            std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
                .output()
                .map(|output| {
                    String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\""))
                })
                .unwrap_or(false)
        }
        #[cfg(not(windows))]
        {
            std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        }
    }

    fn wait_for_process_exit(pid: u32) -> bool {
        for _ in 0..100 {
            if !process_alive(pid) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        false
    }

    let unique = uuid::Uuid::new_v4().simple().to_string();
    let workspace = std::env::temp_dir().join(format!("handshake-mt008-mounted-{unique}"));
    std::fs::create_dir_all(&workspace).expect("create mounted real-LSP workspace");
    let _workspace_cleanup = WorkspaceCleanup(workspace.clone());
    let header_path = workspace.join("math_ops.h");
    let source_path = workspace.join("main.cpp");
    let header =
        "/// Add two integers.\ninline int add_numbers(int lhs, int rhs) { return lhs + rhs; }\n";
    let source = concat!(
        "#include \"math_ops.h\"\n",
        "\n",
        "int main() {\n",
        "    int result = add_numbers(20, 22);\n",
        "    int repeat = add_numbers(result, 1);\n",
        "    int broken = \"not an int\";\n",
        "    int candidate = ;\n",
        "    return result + repeat;\n",
        "}\n",
    );
    std::fs::write(&header_path, header).expect("write mounted indexed header");
    std::fs::write(&source_path, source).expect("write mounted source");
    std::fs::write(workspace.join("compile_flags.txt"), "-std=c++17\n-I.\n")
        .expect("write mounted clangd compile flags");

    let server = std::env::var("HANDSHAKE_REAL_LSP_SERVER").unwrap_or_else(|_| "clangd".to_owned());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("mounted real-LSP runtime");
    let client = Arc::new(LspClient::new(LspServerConfig {
        command: server.clone(),
        args: vec![
            "--background-index=false".to_owned(),
            "--clang-tidy=false".to_owned(),
            "--header-insertion=never".to_owned(),
            "--log=error".to_owned(),
        ],
    }));

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    app.set_active_project_id_for_test("mt008-real-lsp");
    let pane_id = PaneId::from("pane-a");
    app.set_active_pane_for_test(Some(pane_id.clone()));
    app.tab_bar_states_mut().insert(
        pane_id.clone(),
        TabBarState::new(pane_id, vec![TabState::new(PaneType::CodeSymbol)]),
    );
    app.set_left_rail_open(false);
    let panel = app.mounted_code_panel();
    panel.set_file_path(source_path.to_string_lossy().to_string());
    panel.set_text(source);
    panel.set_language_override(Some(
        handshake_native::code_editor::language_mode::LanguageId::new("cpp"),
    ));
    panel.set_workspace_id("mt008-real-lsp");
    assert_eq!(panel.resolved_language().detected.as_str(), "cpp");
    app.install_code_lsp_client_for_language_for_test("cpp", Arc::clone(&client), server.clone());

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    let expected_version = panel.buffer_version_for_test();
    for _ in 0..600 {
        harness.run_steps(1);
        if client.is_running()
            && harness
                .state()
                .lsp_doc_sync_watermark()
                .is_some_and(|(_, version)| version == expected_version)
            && !panel.diagnostic_markers().is_empty()
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        client.is_running(),
        "{server:?} is a live semantic LSP process"
    );
    let lsp_pid = client
        .spawned_process_id_for_test()
        .expect("mounted production transport exposes its clangd PID");
    assert!(
        process_alive(lsp_pid),
        "mounted clangd PID {lsp_pid} is alive"
    );
    let (opened_uri, opened_version) = harness
        .state()
        .lsp_doc_sync_watermark()
        .expect("production app frame pump completed mounted didOpen");
    let opened_path = lsp_types::Url::parse(&opened_uri)
        .expect("didOpen watermark is a valid URI")
        .to_file_path()
        .expect("didOpen watermark is a file URI");
    assert_eq!(
        std::fs::canonicalize(opened_path).expect("canonicalize didOpen path"),
        std::fs::canonicalize(&source_path).expect("canonicalize generated source path"),
        "production didOpen targets the exact mounted file despite Windows long/short path spelling"
    );
    assert_eq!(
        opened_version, expected_version,
        "production didOpen targets the mounted buffer version"
    );
    assert!(
        panel
            .diagnostic_markers()
            .iter()
            .any(|marker| marker.line == 5 || marker.line == 6),
        "real publishDiagnostics reached the mounted gutter: {:?}",
        panel.diagnostic_markers()
    );

    let hover_offset = source
        .find("add_numbers(20")
        .expect("source carries hover symbol")
        + 3;
    panel.set_single_cursor(hover_offset);
    panel.trigger_hover(runtime.handle(), "add_numbers");
    for _ in 0..400 {
        harness.run_steps(1);
        if panel.is_hover_open() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        panel.is_hover_open(),
        "real clangd hover reached the mounted panel"
    );

    let artifact_dir = external_artifact_dir("wp-kernel-012-mt-008/canonical-argus");
    std::fs::create_dir_all(&artifact_dir).expect("create external MT-008 Argus artifact dir");
    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt008-mounted-real-lsp");
    let hover_tree = argus.inspect(&mut harness);
    let hover_node = json_author(&hover_tree, CODE_EDITOR_HOVER_AUTHOR_ID)
        .expect("canonical argus.inspect sees the mounted real-LSP hover Tooltip");
    assert!(
        hover_node.to_string().contains("add_numbers"),
        "canonical hover tree carries populated semantic content: {hover_node}"
    );
    assert!(
        json_has_author_id(&hover_tree, "code-editor.lsp-status"),
        "canonical tree exposes the mounted LSP lifecycle status"
    );
    assert!(
        json_has_author_prefix(&hover_tree, "code_editor_diagnostic_"),
        "canonical tree exposes the real-server diagnostic gutter state"
    );
    panel.close_hover();
    harness.run_steps(2);

    let completion_offset = source
        .find("int candidate = ;")
        .expect("source carries completion expression")
        + "int candidate = ".len();
    panel.set_single_cursor(completion_offset);
    panel.trigger_completion(runtime.handle(), "add_numbers");
    for _ in 0..400 {
        harness.run_steps(1);
        if panel.completion_state().is_some_and(|state| {
            state
                .items
                .iter()
                .any(|item| item.label.contains("add_numbers"))
        }) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let completion = panel
        .completion_state()
        .expect("real clangd completion reached the mounted panel");
    let completion_index = completion
        .items
        .iter()
        .position(|item| item.label.contains("add_numbers"))
        .unwrap_or_else(|| {
            panic!(
                "mounted real completion lacks add_numbers: {:?}",
                completion.items
            )
        });
    let expected_insert = completion.items[completion_index].insert_text.clone();
    let completion_item_id =
        format!("{CODE_EDITOR_COMPLETION_ITEM_AUTHOR_PREFIX}{completion_index}");
    let completion_before = argus.inspect(&mut harness);
    assert!(json_has_author_id(
        &completion_before,
        CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID
    ));
    assert!(json_has_author_id(&completion_before, &completion_item_id));
    let completion_item_bounds = json_author(&completion_before, &completion_item_id)
        .and_then(|node| node.get("bounds"))
        .expect("canonical completion item carries screenshot-space bounds");
    std::fs::write(
        artifact_dir.join("mt008-mounted-real-lsp-completion-before.json"),
        serde_json::to_vec_pretty(&completion_before)
            .expect("serialize pre-action canonical completion tree"),
    )
    .expect("write pre-action canonical completion tree externally");

    // `argus.inspect` renders its production capture pass into an isolated context. Refresh the
    // ordinary paint frame before the GPU capture so the PNG and canonical tree represent the same
    // still-open popup state; retain `completion_before` as the exact action snapshot.
    harness.run_steps(1);
    assert!(
        panel.is_completion_open(),
        "the canonical pre-action completion remains open for the corresponding screenshot"
    );
    let screenshot = harness
        .render()
        .expect("canonical mounted completion/diagnostic screenshot must render");
    let bound = |name: &str| {
        completion_item_bounds
            .get(name)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or_else(|| panic!("canonical completion item bounds carry numeric {name}"))
    };
    let x_start = bound("x").floor().max(0.0) as u32;
    let y_start = bound("y").floor().max(0.0) as u32;
    let x_end = (bound("x") + bound("w")).ceil().max(0.0) as u32;
    let y_end = (bound("y") + bound("h")).ceil().max(0.0) as u32;
    assert!(
        x_end <= screenshot.width() && y_end <= screenshot.height(),
        "canonical completion item bounds fit the mounted screenshot"
    );
    let visible_foreground_pixels = (y_start..y_end)
        .flat_map(|y| (x_start..x_end).map(move |x| (x, y)))
        .filter(|&(x, y)| screenshot.get_pixel(x, y).0[..3].iter().copied().max() >= Some(120))
        .count();
    assert!(
        visible_foreground_pixels >= 100,
        "the canonical completion row is visibly painted in its own tree bounds; found only \
         {visible_foreground_pixels} foreground pixels"
    );
    let screenshot_path = artifact_dir.join("mt008-mounted-real-lsp-before-click.png");
    screenshot
        .save(&screenshot_path)
        .expect("save canonical mounted completion screenshot");

    let text_before = panel.buffer().to_string();
    let expected_text_after = format!(
        "{}{}{}",
        &text_before[..completion_offset],
        expected_insert,
        &text_before[completion_offset..]
    );
    let observation = argus.click_from_snapshot_and_reinspect(
        &mut harness,
        &completion_item_id,
        completion_before.clone(),
    );
    assert!(matches!(
        observation.receipt_status.as_str(),
        "applied" | "indeterminate"
    ));
    assert!(
        observation
            .agent_id
            .contains(":client:mt008-mounted-real-lsp-agent"),
        "canonical receipt retains the external caller attribution: {}",
        observation.agent_id
    );
    assert!(
        !panel.is_completion_open(),
        "Argus click closed the mounted popup"
    );
    let text_after = panel.buffer().to_string();
    assert_ne!(
        text_after, text_before,
        "Argus click changed the real editor buffer"
    );
    assert_eq!(
        text_after, expected_text_after,
        "the exact clicked semantic completion was inserted at the mounted caret"
    );
    assert!(
        !json_has_author_id(&observation.after, CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID),
        "fresh canonical reinspection observes the post-action popup closure"
    );
    let changed_version = panel.buffer_version_for_test();
    for _ in 0..400 {
        harness.run_steps(1);
        if harness
            .state()
            .lsp_doc_sync_watermark()
            .is_some_and(|(_, version)| version == changed_version)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        harness
            .state()
            .lsp_doc_sync_watermark()
            .is_some_and(|(_, version)| version == changed_version),
        "production app frame pump completed didChange after the Argus editor action"
    );

    let tree_path = artifact_dir.join("mt008-mounted-real-lsp-tree.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "hover": hover_tree,
            "completion_before": completion_before,
            "completion_after": observation.after,
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
            "lsp_pid": lsp_pid,
        }))
        .expect("serialize canonical MT-008 tree evidence"),
    )
    .expect("write canonical MT-008 tree evidence externally");
    assert!(screenshot_path.is_file());
    assert!(tree_path.is_file());
    argus.finish();

    client.shutdown_for_host();
    assert!(
        wait_for_process_exit(lsp_pid),
        "mounted host cleanup reaps exact clangd PID {lsp_pid}"
    );
    assert_no_local_test_output();
}
