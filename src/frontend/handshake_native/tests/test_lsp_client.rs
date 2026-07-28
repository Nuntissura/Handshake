//! MT-008 LSP client proofs (WP-KERNEL-012 E1 code editor). The focused
//! `real_language_server_end_to_end_runtime_proof` test launches a real language server; the other
//! tests remain standalone and deterministic.
//!
//! AC-004 / PT-004 (`cargo test -p handshake-native lsp_client_graceful`): with NO language server
//! configured, EVERY LSP method (`initialize`, `did_open`, `did_change`, `completion`, `hover`,
//! `goto_definition`, `references`) returns empty/None without panicking (graceful degradation).
//!
//! AC-008 / PT-007: an LSP `textDocument/publishDiagnostics` NOTIFICATION (no `id`) is received over
//! the stdio transport and ROUTED to the diagnostics channel, then mapped to a gutter marker. This
//! drives the SAME production reader loop (`LspClient::spawn_reader_for_test` runs the real
//! `transport::read_loop` + `route_message`) against an in-memory pipe carrying a real
//! `Content-Length`-framed publishDiagnostics frame — proving the production notification-routing path,
//! not a parallel reimplementation. A MOCK "language server" here is the in-memory pipe writer that
//! emits one error diagnostic frame (the MT impl-note minimal stdio mock, without spawning a real OS
//! process so the test is deterministic + fast + focus-safe).

use std::sync::Arc;

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use handshake_native::code_editor::gutter::{DiagnosticSeverity, GutterMarker, GutterMarkerKind};
#[cfg(windows)]
use handshake_native::code_editor::lsp_client::{
    lsp_focus_safe_creation_flags_for_test, LSP_CREATE_NO_WINDOW_FLAG,
};
use handshake_native::code_editor::lsp_client::{
    published_diagnostics_from_lsp, LspClient, LspServerConfig, MAX_LSP_CONTENT_BYTES,
    MAX_LSP_HEADER_BYTES, MAX_LSP_HEADER_LINE_BYTES, REQUEST_TIMEOUT,
};
use handshake_native::code_editor::{CodeEditorPanel, CompletionItem, CompletionKind, HoverState};
use serde_json::Value;
use tokio::io::AsyncWriteExt;

async fn read_mock_request(client: &LspClient) -> Value {
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        client.read_test_request(),
    )
    .await
    .expect("mock LSP request arrived before timeout")
    .expect("mock LSP request frame")
}

async fn write_mock_response(
    server_write: &mut tokio::io::DuplexStream,
    request: &Value,
    result: Value,
) {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "result": result,
    });
    server_write
        .write_all(&LspClient::frame_message_for_test(&response))
        .await
        .expect("write mock LSP response");
    server_write.flush().await.expect("flush mock LSP response");
}

async fn write_mock_error(server_write: &mut tokio::io::DuplexStream, request: &Value) {
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request["id"].clone(),
        "error": { "code": -32603, "message": "test error" },
    });
    server_write
        .write_all(&LspClient::frame_message_for_test(&response))
        .await
        .expect("write mock LSP error response");
    server_write.flush().await.expect("flush mock LSP error");
}

fn run_panel_frames(panel: Arc<CodeEditorPanel>, until: impl Fn() -> bool) {
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    for _ in 0..40 {
        harness.run();
        if until() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn press_panel_key(harness: &mut Harness, key: egui::Key, modifiers: egui::Modifiers) {
    harness.event(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
}

#[test]
fn panel_live_completion_explicit_empty_prefix_and_automatic_debounce() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("completion input runtime");
    let panel = Arc::new(CodeEditorPanel::new("", "rs"));
    panel.set_file_path("completion-input.rs");
    panel.set_runtime(rt.handle().clone());
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();

    panel.mark_edit_now();
    press_panel_key(
        &mut harness,
        egui::Key::Space,
        egui::Modifiers {
            ctrl: true,
            ..Default::default()
        },
    );
    harness.run();
    let explicit = rt.block_on(read_mock_request(&client));
    assert_eq!(explicit["method"], "textDocument/completion");
    assert_eq!(explicit["params"]["position"]["character"], 0);
    rt.block_on(write_mock_response(
        &mut server_write,
        &explicit,
        serde_json::json!([]),
    ));

    panel.set_text("x");
    panel.set_single_cursor(1);
    press_panel_key(
        &mut harness,
        egui::Key::Space,
        egui::Modifiers {
            ctrl: true,
            ..Default::default()
        },
    );
    harness.run();
    let one_character = rt.block_on(read_mock_request(&client));
    assert_eq!(one_character["method"], "textDocument/completion");
    assert_eq!(one_character["params"]["position"]["character"], 1);
    rt.block_on(write_mock_response(
        &mut server_write,
        &one_character,
        serde_json::json!([]),
    ));

    panel.set_text("");
    panel.set_single_cursor(0);
    harness.event(egui::Event::Text(".".to_owned()));
    harness.run();
    assert!(
        panel.completion_request_armed_for_test(),
        "automatic trigger remains armed during debounce"
    );
    harness.event(egui::Event::Text("a".to_owned()));
    harness.run();
    assert_eq!(panel.buffer().to_string(), ".a");
    assert!(
        panel.completion_request_armed_for_test(),
        "ordinary continuation typing re-anchors rather than cancels the trigger request"
    );
    std::thread::sleep(std::time::Duration::from_millis(220));
    harness.run();
    assert!(!panel.completion_request_armed_for_test());
    let automatic = rt.block_on(read_mock_request(&client));
    assert_eq!(automatic["method"], "textDocument/completion");
    assert_eq!(automatic["params"]["position"]["character"], 2);
    rt.block_on(write_mock_response(
        &mut server_write,
        &automatic,
        serde_json::json!([]),
    ));
}

#[test]
fn panel_lsp_definition_without_workspace_handles_same_and_cross_file_targets() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("definition runtime");
    let panel = Arc::new(CodeEditorPanel::new("call", "rs"));
    panel.set_file_path("definition-source.rs");
    panel.set_single_cursor(2);
    panel.set_runtime(rt.handle().clone());
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();

    press_panel_key(&mut harness, egui::Key::F12, egui::Modifiers::default());
    harness.run();
    let same_request = rt.block_on(read_mock_request(&client));
    let source_uri = same_request["params"]["textDocument"]["uri"]
        .as_str()
        .expect("source uri")
        .to_owned();
    rt.block_on(write_mock_response(
        &mut server_write,
        &same_request,
        serde_json::json!({
            "uri": source_uri,
            "range": {"start":{"line":0,"character":1},"end":{"line":0,"character":3}}
        }),
    ));
    for _ in 0..20 {
        harness.run();
        if panel.last_definition_target().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        panel
            .last_definition_target()
            .unwrap()
            .range
            .start
            .character,
        1
    );

    panel.set_single_cursor(2);
    press_panel_key(&mut harness, egui::Key::F12, egui::Modifiers::default());
    harness.run();
    let cross_request = rt.block_on(read_mock_request(&client));
    let cross_uri =
        lsp_types::Url::from_file_path(std::env::current_dir().unwrap().join("cross-target.rs"))
            .unwrap()
            .to_string();
    rt.block_on(write_mock_response(
        &mut server_write,
        &cross_request,
        serde_json::json!({
            "uri": cross_uri,
            "range": {"start":{"line":7,"character":4},"end":{"line":7,"character":8}}
        }),
    ));
    for _ in 0..20 {
        harness.run();
        if panel.pending_cross_file_jump().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let pending = panel
        .pending_cross_file_jump()
        .expect("cross-file definition parked for host");
    assert_eq!(pending.position.line, 7);
    assert_eq!(pending.position.column, 4);
    assert_eq!(panel.last_definition_target().unwrap().uri, cross_uri);
}

#[test]
fn panel_lsp_definition_reversed_delivery_keeps_newest_identity() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("stale definition runtime");
    let panel = Arc::new(CodeEditorPanel::new("old new", "rs"));
    panel.set_file_path("stale-definition.rs");
    panel.set_runtime(rt.handle().clone());
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    panel.set_single_cursor(2);
    press_panel_key(&mut harness, egui::Key::F12, egui::Modifiers::default());
    harness.run();
    let old_request = rt.block_on(read_mock_request(&client));
    panel.set_single_cursor(6);
    press_panel_key(&mut harness, egui::Key::F12, egui::Modifiers::default());
    harness.run();
    let new_request = rt.block_on(read_mock_request(&client));
    let uri = new_request["params"]["textDocument"]["uri"].clone();
    rt.block_on(async {
        write_mock_response(
            &mut server_write,
            &new_request,
            serde_json::json!({
                "uri":uri,
                "range":{"start":{"line":1,"character":0},"end":{"line":1,"character":1}}
            }),
        )
        .await;
        write_mock_response(
            &mut server_write,
            &old_request,
            serde_json::json!({
                "uri":uri,
                "range":{"start":{"line":9,"character":0},"end":{"line":9,"character":1}}
            }),
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    });
    for _ in 0..20 {
        harness.run();
        if panel.last_definition_target().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(panel.last_definition_target().unwrap().range.start.line, 1);
}

#[test]
fn panel_lsp_references_without_workspace_are_retained_and_rendered() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("references runtime");
    let panel = Arc::new(CodeEditorPanel::new("call", "rs"));
    panel.set_file_path("references.rs");
    panel.set_single_cursor(2);
    panel.set_runtime(rt.handle().clone());
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    press_panel_key(
        &mut harness,
        egui::Key::F12,
        egui::Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    harness.run();
    let request = rt.block_on(read_mock_request(&client));
    assert_eq!(request["method"], "textDocument/references");
    let uri = request["params"]["textDocument"]["uri"].clone();
    rt.block_on(write_mock_response(
        &mut server_write,
        &request,
        serde_json::json!([
            {"uri":uri,"range":{"start":{"line":1,"character":2},"end":{"line":1,"character":6}}},
            {"uri":"file:///other.rs","range":{"start":{"line":3,"character":0},"end":{"line":3,"character":4}}}
        ]),
    ));
    for _ in 0..20 {
        harness.run();
        if panel.last_lsp_references().len() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let refs = panel.last_lsp_references();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[1].range.start.line, 3);
    harness.run();
    let reference_node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("code_editor_reference_1"))
        .expect("reference row has a stable AccessKit id")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: reference_node,
            data: None,
        },
    ));
    harness.run_steps(3);
    let pending = panel
        .pending_cross_file_jump()
        .expect("reference click parks cross-file target");
    assert_eq!(pending.position.line, 3);
    assert_eq!(panel.references_overlay_len(), 0);
}

#[test]
fn panel_empty_lsp_references_close_existing_overlay() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("empty references runtime");
    let panel = Arc::new(CodeEditorPanel::new("call", "rs"));
    panel.set_file_path("empty-references.rs");
    panel.set_single_cursor(2);
    panel.set_runtime(rt.handle().clone());
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 300.0))
        .build_ui(move |ui| panel_ui.show(ui));
    harness.run();
    press_panel_key(
        &mut harness,
        egui::Key::F12,
        egui::Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    harness.run();
    let request = rt.block_on(read_mock_request(&client));
    let uri = request["params"]["textDocument"]["uri"].clone();
    rt.block_on(write_mock_response(
        &mut server_write,
        &request,
        serde_json::json!([{
            "uri":uri,
            "range":{"start":{"line":1,"character":0},"end":{"line":1,"character":4}}
        }]),
    ));
    for _ in 0..20 {
        harness.run();
        if panel.references_overlay_len() == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let close_node = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some("code_editor_references_close"))
        .expect("references overlay exposes a stable close action")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: close_node,
            data: None,
        },
    ));
    harness.run_steps(3);
    assert_eq!(panel.references_overlay_len(), 0);

    press_panel_key(
        &mut harness,
        egui::Key::F12,
        egui::Modifiers {
            shift: true,
            ..Default::default()
        },
    );
    harness.run();
    let empty_request = rt.block_on(read_mock_request(&client));
    rt.block_on(write_mock_response(
        &mut server_write,
        &empty_request,
        serde_json::json!([]),
    ));
    run_panel_frames(Arc::clone(&panel), || false);
    assert_eq!(panel.references_overlay_len(), 0);
    assert!(panel.last_lsp_references().is_empty());
}

#[test]
fn panel_lsp_hover_retains_resolvable_same_file_definition_link() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("hover definition runtime");
    let panel = Arc::new(CodeEditorPanel::new("call", "rs"));
    panel.set_file_path("hover-definition.rs");
    panel.set_single_cursor(2);
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    panel.trigger_hover(rt.handle(), "call");
    let hover_request = rt.block_on(read_mock_request(&client));
    rt.block_on(write_mock_response(
        &mut server_write,
        &hover_request,
        serde_json::json!({"contents":{"kind":"markdown","value":"hover docs"}}),
    ));
    let definition_request = rt.block_on(read_mock_request(&client));
    assert_eq!(definition_request["method"], "textDocument/definition");
    let uri = definition_request["params"]["textDocument"]["uri"].clone();
    rt.block_on(write_mock_response(
        &mut server_write,
        &definition_request,
        serde_json::json!({
            "uri":uri,
            "range":{"start":{"line":5,"character":0},"end":{"line":5,"character":4}}
        }),
    ));
    run_panel_frames(Arc::clone(&panel), || {
        panel
            .hover_state()
            .and_then(|hover| hover.definition_target)
            .map(|target| target.range.start.line)
            == Some(5)
    });
    assert_eq!(
        panel
            .hover_state()
            .unwrap()
            .definition_target
            .unwrap()
            .range
            .start
            .line,
        5
    );
}

#[test]
fn lsp_transport_eof_marks_dead_and_releases_in_flight_request() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("transport eof runtime");
    rt.block_on(async {
        let client = Arc::new(LspClient::disabled());
        let server_write = client.install_test_transport();
        let requester = Arc::clone(&client);
        let request_task = tokio::spawn(async move {
            requester
                .completion("file:///eof.rs", lsp_types::Position::new(0, 0))
                .await
        });
        let _request = read_mock_request(&client).await;
        drop(server_write);
        let result = tokio::time::timeout(std::time::Duration::from_millis(500), request_task)
            .await
            .expect("EOF clears pending request promptly")
            .expect("request task did not panic");
        assert!(result.is_empty());
        assert!(!client.is_running(), "EOF marks the transport dead");
    });
}

#[test]
fn lsp_non_reading_transport_bounds_lock_write_and_recovers_after_replacement() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("non-reading transport runtime");
    rt.block_on(async {
        let client = Arc::new(LspClient::disabled());
        let blocked_server_write = client.install_test_transport();
        // Hold stdin longer than the request budget without changing transport liveness. This proves
        // request() applies its own deadline while acquiring the mutex, independent of notify().
        let holder_client = Arc::clone(&client);
        let holder = tokio::spawn(async move {
            holder_client
                .hold_stdin_lock_for_test(REQUEST_TIMEOUT + std::time::Duration::from_secs(2))
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !client.stdin_locked_for_test() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("test writer acquired the request transport's stdin mutex");
        let requester = Arc::clone(&client);
        let request_task = tokio::spawn(async move {
            requester
                .completion("file:///blocked.rs", lsp_types::Position::new(0, 0))
                .await
        });

        // Separately keep a mock server alive but never drain its request reader. A notification
        // larger than the 64-KiB pipe wedges in framed write/flush while holding its stdin mutex.
        let notify_client = Arc::new(LspClient::disabled());
        let notify_server_write = notify_client.install_test_transport();
        let notifier = Arc::clone(&notify_client);
        let notify_task = tokio::spawn(async move {
            notifier
                .did_change("file:///blocked.rs", 2, &"x".repeat(256 * 1024))
                .await;
        });
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !notify_client.stdin_locked_for_test() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("oversized notification acquired the stdin mutex");
        let bounded = REQUEST_TIMEOUT + std::time::Duration::from_secs(1);
        let (request_result, notify_result) = tokio::join!(
            tokio::time::timeout(bounded, request_task),
            tokio::time::timeout(bounded, notify_task),
        );
        let result = request_result
            .expect("request deadline includes waiting for the stdin mutex")
            .expect("blocked request task did not panic");
        assert!(result.is_empty());
        notify_result
            .expect("notification lock/write is bounded")
            .expect("blocked notification task did not panic");
        holder.abort();
        let _ = holder.await;
        assert!(
            !client.is_running(),
            "a request lock timeout poisons the transport for host restart"
        );
        assert!(
            !notify_client.is_running(),
            "a notification write timeout poisons the transport for host restart"
        );
        drop(blocked_server_write);
        drop(notify_server_write);

        // Replace the dead generation with an uninitialized transport and drive the real serialized
        // initialize path. This is the deterministic in-memory equivalent of the host restart attach.
        let mut recovered_server_write = client.install_uninitialized_test_transport();
        let recovering = Arc::clone(&client);
        let initialize_task =
            tokio::spawn(async move { recovering.initialize_test_transport().await });
        let initialize = read_mock_request(&client).await;
        assert_eq!(initialize["method"], "initialize");
        write_mock_response(
            &mut recovered_server_write,
            &initialize,
            serde_json::json!({"capabilities":{}}),
        )
        .await;
        assert!(initialize_task.await.expect("restart initialize task"));
        let initialized = read_mock_request(&client).await;
        assert_eq!(initialized["method"], "initialized");
        assert!(client.is_running(), "replacement transport is healthy");

        let recovered = Arc::clone(&client);
        let completion_task = tokio::spawn(async move {
            recovered
                .completion("file:///recovered.rs", lsp_types::Position::new(0, 0))
                .await
        });
        let completion = read_mock_request(&client).await;
        write_mock_response(
            &mut recovered_server_write,
            &completion,
            serde_json::json!([{"label":"recovered"}]),
        )
        .await;
        let items = completion_task.await.expect("recovered completion task");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "recovered");
    });
}

#[test]
fn lsp_document_feature_request_releases_sync_guard_and_discards_stale_response() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("document feature guard runtime");
    rt.block_on(async {
        let client = Arc::new(LspClient::disabled());
        let mut server_write = client.install_test_transport();
        let uri_a = "file:///guard-a.rs";
        let uri_b = "file:///guard-b.rs";

        let generation_a = client.reserve_document_sync_generation();
        client
            .sync_document_generation(generation_a, uri_a, "rust", 1, "a", true)
            .await;
        let open_a = read_mock_request(&client).await;
        assert_eq!(open_a["method"], "textDocument/didOpen");

        let feature_client = Arc::clone(&client);
        let feature_task = tokio::spawn(async move {
            feature_client
                .completion_after_sync(uri_a, lsp_types::Position::new(0, 0))
                .await
        });
        let stale_completion = read_mock_request(&client).await;
        assert_eq!(stale_completion["method"], "textDocument/completion");

        // Leave the completion response pending and open another document. The other URI must not
        // block, close A, or invalidate A: one shared client supports simultaneous editor tabs.
        let generation_b = client.reserve_document_sync_generation();
        let sync_client = Arc::clone(&client);
        let sync_task = tokio::spawn(async move {
            sync_client
                .sync_document_generation(generation_b, uri_b, "rust", 1, "b", true)
                .await;
        });
        tokio::time::timeout(std::time::Duration::from_millis(500), sync_task)
            .await
            .expect("document sync write guard is not blocked by an in-flight feature request")
            .expect("document sync task did not panic");
        let open_b = read_mock_request(&client).await;
        assert_eq!(open_b["method"], "textDocument/didOpen");
        assert_eq!(
            client.open_document_uris_for_test().await,
            vec![uri_a.to_owned(), uri_b.to_owned()]
        );

        // Explicitly closing A invalidates only A's in-flight feature result and leaves B open.
        let close_generation = client.reserve_document_sync_generation();
        client
            .close_document_generation(close_generation, uri_a)
            .await;
        let close_a = read_mock_request(&client).await;
        assert_eq!(close_a["method"], "textDocument/didClose");
        assert_eq!(close_a["params"]["textDocument"]["uri"], uri_a);

        write_mock_response(
            &mut server_write,
            &stale_completion,
            serde_json::json!([{"label":"stale"}]),
        )
        .await;
        assert!(
            feature_task
                .await
                .expect("stale completion task did not panic")
                .is_empty(),
            "post-response generation check discards the previous document's result"
        );
        assert_eq!(
            client.open_document_uris_for_test().await,
            vec![uri_b.to_owned()]
        );
    });
}

#[test]
fn lsp_concurrent_initialize_serializes_handshake_before_ordinary_state() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("concurrent initialize runtime");
    rt.block_on(async {
        let client = Arc::new(LspClient::disabled());
        let mut server_write = client.install_uninitialized_test_transport();
        assert!(!client.is_running(), "raw transport is not handshake-ready");
        let first_client = Arc::clone(&client);
        let second_client = Arc::clone(&client);
        let first = tokio::spawn(async move { first_client.initialize_test_transport().await });
        let second = tokio::spawn(async move { second_client.initialize_test_transport().await });

        let initialize = read_mock_request(&client).await;
        assert_eq!(initialize["method"], "initialize");
        assert!(client.is_initializing());
        assert!(!client.is_running());
        write_mock_response(
            &mut server_write,
            &initialize,
            serde_json::json!({"capabilities":{}}),
        )
        .await;
        assert!(first.await.expect("first initialize task"));
        assert!(second.await.expect("second initialize task"));
        assert!(client.is_running());

        let initialized = read_mock_request(&client).await;
        assert_eq!(initialized["method"], "initialized");
        assert!(
            initialized.get("id").is_none(),
            "only the single initialized notification follows the one initialize request"
        );
    });
}

#[test]
fn lsp_json_rpc_error_is_not_a_success_or_transport_failure() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("JSON-RPC error runtime");
    rt.block_on(async {
        let client = Arc::new(LspClient::disabled());
        let mut server_write = client.install_uninitialized_test_transport();
        let initializing = Arc::clone(&client);
        let initialize_task =
            tokio::spawn(async move { initializing.initialize_test_transport().await });
        let initialize = read_mock_request(&client).await;
        write_mock_error(&mut server_write, &initialize).await;
        assert!(!initialize_task.await.expect("initialize task"));
        assert!(
            !client.is_running(),
            "initialize error cannot mark the client ready"
        );

        let client = Arc::new(LspClient::disabled());
        let mut server_write = client.install_test_transport();
        let requester = Arc::clone(&client);
        let completion_task = tokio::spawn(async move {
            requester
                .completion("file:///error.rs", lsp_types::Position::new(0, 0))
                .await
        });
        let completion = read_mock_request(&client).await;
        write_mock_error(&mut server_write, &completion).await;
        assert!(completion_task.await.expect("completion task").is_empty());
        assert!(
            client.is_running(),
            "ordinary JSON-RPC method errors do not poison a healthy transport"
        );

        let requester = Arc::clone(&client);
        let hover_task = tokio::spawn(async move {
            requester
                .hover("file:///error.rs", lsp_types::Position::new(0, 0))
                .await
        });
        let hover = read_mock_request(&client).await;
        write_mock_response(
            &mut server_write,
            &hover,
            serde_json::json!({"contents":{"kind":"markdown","value":"recovered"}}),
        )
        .await;
        assert_eq!(
            hover_task.await.expect("hover task").expect("hover").value,
            "recovered"
        );
    });
}

#[test]
fn lsp_reader_rejects_oversized_headers_and_content_length_before_body_allocation() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("framing limits runtime");
    rt.block_on(async {
        async fn reader_finishes_for(bytes: Vec<u8>) {
            let client = LspClient::disabled();
            let capacity = bytes.len().saturating_add(1024);
            let (client_read, mut server_write) = tokio::io::duplex(capacity);
            let reader = client.spawn_reader_task_for_test(client_read);
            server_write
                .write_all(&bytes)
                .await
                .expect("write malformed frame");
            drop(server_write);
            tokio::time::timeout(std::time::Duration::from_secs(1), reader)
                .await
                .expect("bounded reader rejects malformed frame promptly")
                .expect("reader task does not panic");
        }

        reader_finishes_for(vec![b'x'; MAX_LSP_HEADER_LINE_BYTES + 1]).await;

        let mut total_header = Vec::new();
        while total_header.len() <= MAX_LSP_HEADER_BYTES {
            total_header.extend_from_slice(b"X-Test: ");
            total_header.extend(std::iter::repeat_n(b'x', 4096));
            total_header.extend_from_slice(b"\r\n");
        }
        reader_finishes_for(total_header).await;

        reader_finishes_for(
            format!("Content-Length: {}\r\n\r\n", MAX_LSP_CONTENT_BYTES + 1).into_bytes(),
        )
        .await;
    });
}

#[test]
fn lsp_per_document_generations_keep_multiple_uris_open_and_close_independently() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("document generation runtime");
    rt.block_on(async {
        let client = Arc::new(LspClient::disabled());
        let _server_write = client.install_test_transport();
        let uri_a = "file:///a.rs";
        let uri_b = "file:///b.rs";

        let generation_a = client.reserve_document_sync_generation();
        client
            .sync_document_generation(generation_a, uri_a, "rust", 1, "a", true)
            .await;
        let open_a = read_mock_request(&client).await;
        assert_eq!(open_a["method"], "textDocument/didOpen");
        assert_eq!(open_a["params"]["textDocument"]["uri"], uri_a);

        let generation_b = client.reserve_document_sync_generation();
        client
            .sync_document_generation(generation_b, uri_b, "rust", 3, "b", true)
            .await;
        let open_b = read_mock_request(&client).await;
        assert_eq!(open_b["method"], "textDocument/didOpen");
        assert_eq!(open_b["params"]["textDocument"]["uri"], uri_b);
        assert_eq!(open_b["params"]["textDocument"]["version"], 3);

        let older_a = client.reserve_document_sync_generation();
        let newer_a = client.reserve_document_sync_generation();
        client
            .sync_document_generation(newer_a, uri_a, "rust", 5, "new-a", false)
            .await;
        let change_a = read_mock_request(&client).await;
        assert_eq!(change_a["method"], "textDocument/didChange");
        assert_eq!(change_a["params"]["textDocument"]["uri"], uri_a);
        assert_eq!(change_a["params"]["textDocument"]["version"], 5);

        // This older task starts after the newer A generation and is rejected without changing A.
        client
            .sync_document_generation(older_a, uri_a, "rust", 4, "old-a", false)
            .await;
        assert_eq!(
            client.open_document_uris_for_test().await,
            vec![uri_a.to_owned(), uri_b.to_owned()]
        );

        client.close_document(uri_a).await;
        let close_a = read_mock_request(&client).await;
        assert_eq!(close_a["method"], "textDocument/didClose");
        assert_eq!(
            client.open_document_uris_for_test().await,
            vec![uri_b.to_owned()]
        );
    });
}

#[test]
fn lsp_final_arc_drop_on_tokio_worker_uses_bounded_shutdown() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("drop supervisor runtime");
    rt.block_on(async {
        let client = Arc::new(LspClient::disabled());
        let _server_write = client.install_test_transport();
        let drop_task = tokio::spawn(async move { drop(client) });
        tokio::time::timeout(std::time::Duration::from_secs(1), drop_task)
            .await
            .expect("worker-thread final drop is bounded")
            .expect("worker-thread final drop does not panic");
    });
}

/// AC-004: a client built with NO server config is not configured + not running.
#[test]
fn lsp_client_graceful_unconfigured_is_not_running() {
    let client = LspClient::disabled();
    assert!(
        !client.is_configured(),
        "AC-004: disabled client reports not configured"
    );
    assert!(
        !client.is_running(),
        "AC-004: disabled client has no spawned process"
    );

    // A config with a non-empty command IS configured (but still not spawned until did_open).
    let configured = LspClient::new(LspServerConfig::command("rust-analyzer"));
    assert!(configured.is_configured());
    assert!(
        !configured.is_running(),
        "configured but not spawned until did_open"
    );
}

/// HBR-QUIET / RISK-001: on Windows, LSP subprocesses must be created without a console window so model
/// work never steals operator focus. The production spawn path reads this named option before
/// `Command::spawn`.
#[test]
#[cfg(windows)]
fn lsp_spawn_option_uses_create_no_window() {
    assert_eq!(LSP_CREATE_NO_WINDOW_FLAG, 0x0800_0000);
    assert_eq!(
        lsp_focus_safe_creation_flags_for_test(),
        LSP_CREATE_NO_WINDOW_FLAG,
        "MT-008: production LSP spawn uses the named CREATE_NO_WINDOW option"
    );
}

/// AC-004 / PT-004: with no server, every method degrades gracefully (empty/None, no panic). Runs the
/// REAL async methods on a current-thread runtime — the same path the editor calls.
#[test]
fn lsp_client_graceful_all_methods_return_empty_without_server() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled();
        // initialize returns false (no server), no panic.
        assert!(
            !client.initialize(None).await,
            "AC-004: initialize false without server"
        );
        // did_open / did_change are graceful no-ops (no panic, no spawn).
        client
            .did_open("file:///x.rs", "rust", "fn main() {}")
            .await;
        client.did_change("file:///x.rs", 2, "fn main() {}").await;
        assert!(
            !client.is_running(),
            "AC-004: no process spawned for a disabled client"
        );

        let pos = lsp_types::Position {
            line: 0,
            character: 0,
        };
        assert!(
            client.completion("file:///x.rs", pos).await.is_empty(),
            "AC-004: completion empty without server"
        );
        assert!(
            client.hover("file:///x.rs", pos).await.is_none(),
            "AC-004: hover None without server"
        );
        assert!(
            client.goto_definition("file:///x.rs", pos).await.is_none(),
            "AC-004: goto_definition None without server"
        );
        assert!(
            client.references("file:///x.rs", pos).await.is_empty(),
            "AC-004: references empty without server"
        );
        println!("PT-004 lsp_client_graceful: all methods returned empty/None without a server");
    });
}

/// MT-008 contract proof: the live panel asks the attached LSP even with no workspace/CodeNav fallback
/// available, and a late response for the older prefix/caret cannot overwrite the newer completion.
#[test]
fn panel_completion_lsp_primary_reversed_responses_keep_newest_request() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("completion generation runtime");
    let panel = Arc::new(CodeEditorPanel::new("old new", "rs"));
    panel.set_file_path("generation.rs");
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _runtime_guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));

    panel.set_single_cursor(2);
    panel.trigger_completion(rt.handle(), "old");
    let old_request = rt.block_on(read_mock_request(&client));
    panel.set_single_cursor(6);
    panel.trigger_completion(rt.handle(), "new");
    let new_request = rt.block_on(read_mock_request(&client));

    rt.block_on(async {
        assert_eq!(old_request["method"], "textDocument/completion");
        assert_eq!(new_request["method"], "textDocument/completion");
        assert_eq!(old_request["params"]["position"]["character"], 2);
        assert_eq!(new_request["params"]["position"]["character"], 6);
        write_mock_response(
            &mut server_write,
            &new_request,
            serde_json::json!([{
                "label": "new_item",
                "insertText": "new_item",
                "kind": 3,
                "detail": "new LSP completion"
            }]),
        )
        .await;
        write_mock_response(
            &mut server_write,
            &old_request,
            serde_json::json!([{
                "label": "old_item",
                "insertText": "old_item",
                "kind": 3,
                "detail": "stale LSP completion"
            }]),
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });

    run_panel_frames(Arc::clone(&panel), || panel.completion_state().is_some());
    let completion = panel
        .completion_state()
        .expect("newest LSP completion opens the popup");
    assert_eq!(completion.items.len(), 1);
    assert_eq!(completion.items[0].label, "new_item");
    assert_eq!(completion.items[0].detail, "new LSP completion");
}

/// MT-008 contract proof: hover uses the real LSP request path first and rejects an older hover that
/// arrives after the request for the newer word/caret.
#[test]
fn panel_hover_lsp_primary_reversed_responses_keep_newest_request() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("hover generation runtime");
    let panel = Arc::new(CodeEditorPanel::new("old new", "rs"));
    panel.set_file_path("generation.rs");
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _runtime_guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));

    panel.set_single_cursor(2);
    panel.trigger_hover(rt.handle(), "old");
    let old_request = rt.block_on(read_mock_request(&client));
    panel.set_single_cursor(6);
    panel.trigger_hover(rt.handle(), "new");
    let new_request = rt.block_on(read_mock_request(&client));

    rt.block_on(async {
        assert_eq!(old_request["method"], "textDocument/hover");
        assert_eq!(new_request["method"], "textDocument/hover");
        assert_eq!(old_request["params"]["position"]["character"], 2);
        assert_eq!(new_request["params"]["position"]["character"], 6);
        write_mock_response(
            &mut server_write,
            &new_request,
            serde_json::json!({
                "contents": { "kind": "markdown", "value": "new hover" }
            }),
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });

    // Drain the newer result first. The delivery slot is now empty; the late old response must still
    // be rejected by the live generation gate rather than reopening/replacing the tooltip.
    run_panel_frames(Arc::clone(&panel), || panel.hover_state().is_some());
    assert_eq!(
        panel.hover_state().expect("new hover delivered").markdown,
        "new hover"
    );

    rt.block_on(async {
        write_mock_response(
            &mut server_write,
            &old_request,
            serde_json::json!({
                "contents": { "kind": "markdown", "value": "stale hover" }
            }),
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });

    run_panel_frames(Arc::clone(&panel), || false);
    let hover = panel.hover_state().expect("newest LSP hover opens tooltip");
    assert_eq!(hover.display_name, "new");
    assert_eq!(hover.markdown, "new hover");
}

/// A response is stale even without a newer request when the backing buffer version changes while the
/// LSP request is in flight.
#[test]
fn panel_completion_rejects_response_for_replaced_buffer() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("buffer identity runtime");
    let panel = Arc::new(CodeEditorPanel::new("old", "rs"));
    panel.set_file_path("buffer.rs");
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _runtime_guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    panel.set_single_cursor(2);
    panel.trigger_completion(rt.handle(), "old");

    rt.block_on(async {
        let request = read_mock_request(&client).await;
        panel.set_text("changed");
        write_mock_response(
            &mut server_write,
            &request,
            serde_json::json!([{ "label": "stale_buffer_item" }]),
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    run_panel_frames(Arc::clone(&panel), || false);
    assert!(
        panel.completion_state().is_none(),
        "completion for an older buffer version must be discarded"
    );
}

/// A response is stale even without a newer request when the panel has switched to another document.
#[test]
fn panel_hover_rejects_response_for_replaced_document() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("document identity runtime");
    let panel = Arc::new(CodeEditorPanel::new("old", "rs"));
    panel.set_file_path("first.rs");
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _runtime_guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));
    panel.set_single_cursor(2);
    panel.trigger_hover(rt.handle(), "old");

    rt.block_on(async {
        let request = read_mock_request(&client).await;
        panel.load_file("second.rs");
        write_mock_response(
            &mut server_write,
            &request,
            serde_json::json!({
                "contents": { "kind": "markdown", "value": "stale document hover" }
            }),
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    run_panel_frames(Arc::clone(&panel), || false);
    assert!(
        panel.hover_state().is_none(),
        "hover for the previous document URI must be discarded"
    );
}

#[test]
fn panel_lsp_position_counts_utf16_units_before_astral_character_caret() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("UTF-16 position runtime");
    let panel = Arc::new(CodeEditorPanel::new("😀 ad", "rs"));
    panel.set_file_path("utf16.rs");
    panel.set_single_cursor("😀 ad".len());
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _runtime_guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));

    panel.trigger_completion(rt.handle(), "ad");
    let request = rt.block_on(read_mock_request(&client));
    assert_eq!(request["method"], "textDocument/completion");
    assert_eq!(request["params"]["position"]["line"], 0);
    assert_eq!(
        request["params"]["position"]["character"], 5,
        "astral emoji counts as two UTF-16 units, followed by space + `ad`"
    );
    rt.block_on(write_mock_response(
        &mut server_write,
        &request,
        serde_json::json!([]),
    ));
}

#[test]
fn trigger_short_circuits_close_existing_completion_and_hover_immediately() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("overlay invalidation runtime");
    let panel = CodeEditorPanel::new("add", "rs");
    panel.open_completion(vec![CompletionItem {
        label: "stale".into(),
        insert_text: "stale".into(),
        kind: CompletionKind::Function,
        detail: "old completion".into(),
        documentation: String::new(),
        symbol_entity_id: String::new(),
    }]);
    panel.trigger_completion(rt.handle(), "x");
    assert!(
        !panel.is_completion_open(),
        "a short-circuited request closes the old completion popup"
    );

    panel.open_hover(HoverState {
        markdown: "old hover".into(),
        display_name: "old".into(),
        anchor: egui::pos2(10.0, 10.0),
        definition_target: None,
    });
    panel.trigger_hover(rt.handle(), "");
    assert!(
        !panel.is_hover_open(),
        "a short-circuited request closes the old hover tooltip"
    );
}

#[test]
fn current_empty_lsp_hover_delivery_closes_reopened_stale_tooltip() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("empty hover runtime");
    let panel = Arc::new(CodeEditorPanel::new("add", "rs"));
    panel.set_file_path("empty_hover.rs");
    panel.set_single_cursor(2);
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _runtime_guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));

    panel.trigger_hover(rt.handle(), "add");
    let request = rt.block_on(read_mock_request(&client));
    panel.open_hover(HoverState {
        markdown: "reopened stale hover".into(),
        display_name: "stale".into(),
        anchor: egui::pos2(10.0, 10.0),
        definition_target: None,
    });
    rt.block_on(async {
        write_mock_response(&mut server_write, &request, serde_json::Value::Null).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });
    run_panel_frames(Arc::clone(&panel), || !panel.is_hover_open());
    assert!(
        !panel.is_hover_open(),
        "a current empty response closes any stale tooltip reopened while the request was in flight"
    );
}

#[test]
fn explicit_dismiss_rejects_in_flight_completion_and_hover_deliveries() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("dismiss generation runtime");
    let panel = Arc::new(CodeEditorPanel::new("add", "rs"));
    panel.set_file_path("dismiss.rs");
    panel.set_single_cursor(2);
    let client = Arc::new(LspClient::disabled());
    let mut server_write = {
        let _guard = rt.enter();
        client.install_test_transport()
    };
    panel.set_lsp_client(Arc::clone(&client));

    panel.trigger_completion(rt.handle(), "add");
    let completion = rt.block_on(read_mock_request(&client));
    panel.close_completion();
    rt.block_on(write_mock_response(
        &mut server_write,
        &completion,
        serde_json::json!([{"label":"must_not_reopen"}]),
    ));
    run_panel_frames(Arc::clone(&panel), || false);
    assert!(!panel.is_completion_open());

    panel.trigger_hover(rt.handle(), "add");
    let hover = rt.block_on(read_mock_request(&client));
    panel.close_hover();
    rt.block_on(write_mock_response(
        &mut server_write,
        &hover,
        serde_json::json!({"contents":{"kind":"markdown","value":"must not reopen"}}),
    ));
    run_panel_frames(Arc::clone(&panel), || false);
    assert!(!panel.is_hover_open());
}

/// AC-008 / PT-007: a `publishDiagnostics` notification framed exactly as a real LSP server sends it is
/// received over the stdio transport and routed to the diagnostics channel, then mapped to a 0-based
/// gutter line + severity. The MOCK server is the in-memory pipe writer emitting one error diagnostic.
#[test]
fn lsp_publish_diagnostics_notification_is_routed_to_channel() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled();
        let mut diagnostics_rx = client
            .take_diagnostics_receiver()
            .expect("diagnostics receiver available before reader starts");

        // An in-memory duplex pipe stands in for the server's stdout: the test (the "mock server")
        // writes a publishDiagnostics frame; the client's REAL reader loop reads it.
        let (client_read, mut mock_write) = tokio::io::duplex(8192);
        client.spawn_reader_for_test(client_read);

        // One ERROR diagnostic on line 5 (0-based 4 in LSP coordinates), as a real server would send.
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///mock.rs",
                "diagnostics": [{
                    "range": {
                        "start": { "line": 4, "character": 0 },
                        "end": { "line": 4, "character": 7 }
                    },
                    "severity": 1,
                    "message": "expected `;`, found `}`"
                }]
            }
        });
        let frame = LspClient::frame_message_for_test(&notification);
        use tokio::io::AsyncWriteExt;
        mock_write.write_all(&frame).await.expect("write frame");
        mock_write.flush().await.expect("flush");

        // The reader routes it to the diagnostics channel (bounded wait so a failure does not hang).
        let published =
            tokio::time::timeout(std::time::Duration::from_secs(3), diagnostics_rx.recv())
                .await
                .expect("AC-008: publishDiagnostics routed within the timeout")
                .expect("AC-008: diagnostics channel delivered a notification");

        assert_eq!(published.uri, "file:///mock.rs");
        assert_eq!(
            published.diagnostics.len(),
            1,
            "AC-008: one diagnostic received"
        );
        assert_eq!(
            published.diagnostics[0].line, 4,
            "AC-008: LSP range.start.line (0-based) maps to the gutter line"
        );
        assert_eq!(
            published.diagnostics[0].severity, 1,
            "AC-008: error severity preserved"
        );
        assert!(published.diagnostics[0].message.contains("expected"));
        println!(
            "PT-007 lsp publishDiagnostics routed: uri={} line={} sev={} msg={:?}",
            published.uri,
            published.diagnostics[0].line,
            published.diagnostics[0].severity,
            published.diagnostics[0].message
        );
    });
}

/// AC-008: a malformed (non-JSON) stdout line BEFORE a valid frame is SKIPPED, never panicked on
/// (RISK-003), and the following valid publishDiagnostics frame is still routed.
#[test]
fn lsp_reader_skips_malformed_lines_then_routes_valid_frame() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled();
        let mut diagnostics_rx = client.take_diagnostics_receiver().expect("receiver");
        let (client_read, mut mock_write) = tokio::io::duplex(8192);
        client.spawn_reader_for_test(client_read);

        use tokio::io::AsyncWriteExt;
        // A stray non-header debug print (no Content-Length) — RISK-003: must be skipped, not panic.
        mock_write
            .write_all(b"this is a stray server debug line with no header\r\n\r\n")
            .await
            .expect("write garbage");
        // Then a malformed framed body (declares a length but the body is not JSON).
        mock_write
            .write_all(b"Content-Length: 11\r\n\r\nNOT-JSON!!!")
            .await
            .expect("write malformed body");
        // Then a VALID publishDiagnostics frame.
        let valid = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///ok.rs",
                "diagnostics": [{
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } },
                    "severity": 2,
                    "message": "unused"
                }]
            }
        });
        mock_write
            .write_all(&LspClient::frame_message_for_test(&valid))
            .await
            .expect("write valid");
        mock_write.flush().await.expect("flush");

        let published = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            diagnostics_rx.recv(),
        )
        .await
        .expect("RISK-003: reader survived malformed input and routed the valid frame")
        .expect("valid frame delivered");
        assert_eq!(published.uri, "file:///ok.rs");
        assert_eq!(published.diagnostics[0].severity, 2);
        println!("RISK-003: malformed lines skipped; valid frame still routed");
    });
}

/// AC-008: the client notification channel is not enough by itself. This drives the same framed
/// `publishDiagnostics` reader into a mounted `CodeEditorPanel` and proves `drain_lsp_diagnostics`
/// maps it onto the editor gutter marker store that the live UI renders.
#[test]
fn lsp_publish_diagnostics_notification_reaches_panel_gutter() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let panel = CodeEditorPanel::new("fn main() {}\n", "rs");
        panel.set_file_path("mock.rs");
        let client = Arc::new(LspClient::disabled());
        let (client_read, mut mock_write) = tokio::io::duplex(8192);
        client.spawn_reader_for_test(client_read);
        panel.set_lsp_client(Arc::clone(&client));

        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                // Equivalent to the panel's `file:///mock.rs`, but deliberately percent-encoded so
                // raw string equality would drop a valid diagnostic notification.
                "uri": "file:///mock%2Ers",
                "diagnostics": [{
                    "range": {
                        "start": { "line": 1, "character": 0 },
                        "end": { "line": 1, "character": 5 }
                    },
                    "severity": 2,
                    "message": "unused variable"
                }]
            }
        });
        use tokio::io::AsyncWriteExt;
        mock_write
            .write_all(&LspClient::frame_message_for_test(&notification))
            .await
            .expect("write publishDiagnostics frame");
        mock_write.flush().await.expect("flush publishDiagnostics");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let count = panel
            .drain_lsp_diagnostics()
            .expect("AC-008: panel drained the publishDiagnostics notification");
        assert_eq!(count, 1, "AC-008: one gutter marker pushed");
        let markers = panel.diagnostic_markers();
        assert_eq!(
            markers.len(),
            1,
            "AC-008: panel gutter now stores the marker"
        );
        assert_eq!(markers[0].line, 1, "AC-008: LSP line maps to gutter line");
        assert_eq!(markers[0].message, "unused variable");
        assert_eq!(
            markers[0].kind,
            GutterMarkerKind::Diagnostic(DiagnosticSeverity::Warning),
            "AC-008: severity 2 maps to a warning gutter marker"
        );
        println!(
            "AC-008 panel gutter proof: publishDiagnostics routed to CodeEditorPanel marker store"
        );
    });
}

#[test]
fn lsp_diagnostics_fan_out_to_every_panel_and_reject_stale_document_versions() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("diagnostic fan-out runtime");
    rt.block_on(async {
        let panel_a = CodeEditorPanel::new("fn a() {}\n", "rs");
        let panel_b = CodeEditorPanel::new("fn a() {}\n", "rs");
        panel_a.set_file_path("shared.rs");
        panel_b.set_file_path("shared.rs");
        let client = Arc::new(LspClient::disabled());
        let (client_read, mut mock_write) = tokio::io::duplex(16 * 1024);
        client.spawn_reader_for_test(client_read);
        panel_a.set_lsp_client(Arc::clone(&client));
        panel_b.set_lsp_client(Arc::clone(&client));

        async fn publish(writer: &mut tokio::io::DuplexStream, version: i64, message: &str) {
            let notification = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": {
                    "uri": "file:///shared.rs",
                    "version": version,
                    "diagnostics": [{
                        "range": {
                            "start": { "line": 0, "character": 0 },
                            "end": { "line": 0, "character": 2 }
                        },
                        "severity": 2,
                        "message": message
                    }]
                }
            });
            writer
                .write_all(&LspClient::frame_message_for_test(&notification))
                .await
                .expect("write diagnostic notification");
            writer.flush().await.expect("flush diagnostic notification");
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        publish(&mut mock_write, 1, "initial").await;
        assert_eq!(panel_a.drain_lsp_diagnostics(), Some(1));
        assert_eq!(panel_b.drain_lsp_diagnostics(), Some(1));
        assert_eq!(panel_a.diagnostic_markers()[0].message, "initial");
        assert_eq!(panel_b.diagnostic_markers()[0].message, "initial");

        panel_a.set_text("fn a() { let newer = 1; }\n");
        assert_eq!(panel_a.buffer_version_for_test(), 2);
        publish(&mut mock_write, 1, "stale for edited panel").await;
        assert_eq!(
            panel_a.drain_lsp_diagnostics(),
            None,
            "diagnostics older than the panel buffer version are discarded"
        );
        assert_eq!(panel_a.diagnostic_markers()[0].message, "initial");
        assert_eq!(
            panel_b.drain_lsp_diagnostics(),
            Some(1),
            "the same version remains current for the unedited subscriber"
        );

        publish(&mut mock_write, 2, "current").await;
        assert_eq!(panel_a.drain_lsp_diagnostics(), Some(1));
        assert_eq!(panel_b.drain_lsp_diagnostics(), Some(1));
        assert_eq!(panel_a.diagnostic_markers()[0].message, "current");
        assert_eq!(panel_b.diagnostic_markers()[0].message, "current");
    });
}

/// AC-008 hardening: a language server may publish diagnostics for multiple files through one client.
/// The panel must not let a different document's notification replace this file's gutter state.
#[test]
fn lsp_publish_diagnostics_ignores_non_matching_document_uri() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let panel = CodeEditorPanel::new("fn main() {}\n", "rs");
        panel.set_file_path("mock.rs");
        panel.push_diagnostics(vec![GutterMarker::diagnostic(
            0,
            DiagnosticSeverity::Error,
            "existing marker",
        )]);
        let client = Arc::new(LspClient::disabled());
        let (client_read, mut mock_write) = tokio::io::duplex(8192);
        client.spawn_reader_for_test(client_read);
        panel.set_lsp_client(Arc::clone(&client));

        let wrong_document = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///other.rs",
                "diagnostics": [{
                    "range": {
                        "start": { "line": 1, "character": 0 },
                        "end": { "line": 1, "character": 5 }
                    },
                    "severity": 2,
                    "message": "wrong file warning"
                }]
            }
        });
        use tokio::io::AsyncWriteExt;
        mock_write
            .write_all(&LspClient::frame_message_for_test(&wrong_document))
            .await
            .expect("write non-matching publishDiagnostics frame");
        mock_write
            .flush()
            .await
            .expect("flush non-matching publishDiagnostics");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        assert_eq!(
            panel.drain_lsp_diagnostics(),
            None,
            "AC-008 hardening: non-matching URI is drained but not applied"
        );
        let markers = panel.diagnostic_markers();
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].line, 0);
        assert_eq!(markers[0].message, "existing marker");
        assert_eq!(
            markers[0].kind,
            GutterMarkerKind::Diagnostic(DiagnosticSeverity::Error)
        );
    });
}

/// Sanity: the LSP->editor diagnostic mapping (`published_diagnostics_from_lsp`) is the same function
/// the channel feeds, so a direct call mirrors what the gutter receives (AC-008 mapping).
#[test]
fn lsp_diagnostics_map_to_zero_based_lines() {
    use lsp_types::{
        Diagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Url,
    };
    let params = PublishDiagnosticsParams {
        uri: Url::parse("file:///z.rs").unwrap(),
        version: Some(7),
        diagnostics: vec![Diagnostic {
            range: Range {
                start: Position {
                    line: 7,
                    character: 1,
                },
                end: Position {
                    line: 7,
                    character: 4,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            message: "w".to_owned(),
            ..Default::default()
        }],
    };
    let mapped = published_diagnostics_from_lsp(params);
    assert_eq!(mapped.version, Some(7));
    assert_eq!(mapped.diagnostics[0].line, 7);
    assert_eq!(mapped.diagnostics[0].severity, 2);
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-008 REMEDIATION: server DISCOVERY (typed absent-state, un-gated) + the GATED
// real-process spawn/initialize/Drop-no-zombie proof against a canned stdio LSP subprocess.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use handshake_native::code_editor::lsp_client::{discover_lsp_server_in, LspServerDiscovery};

/// MT-008 REMEDIATION (un-gated): the HONEST typed absent-state. A host with no server on PATH, a
/// missing PATH variable, and a language with no known canonical server all resolve to the typed
/// [`LspServerDiscovery::Absent`] carrying WHAT was probed — never a fake `Found` and never a panic.
/// This is the exact state the live shell surfaces (`LspAttachState::Absent`) when rust-analyzer is
/// absent on the host.
#[test]
fn lsp_discovery_typed_absent_state() {
    // Empty PATH: the canonical rust server is probed but not found.
    let got = discover_lsp_server_in("rust", Some(std::ffi::OsString::new()));
    assert_eq!(
        got,
        LspServerDiscovery::Absent {
            language_id: "rust".to_owned(),
            probed_command: "rust-analyzer".to_owned(),
        },
        "empty PATH -> typed Absent naming the probed canonical command"
    );
    assert!(!got.is_found());

    // No PATH variable at all: same typed absent-state (no panic, no fabricated config).
    let got = discover_lsp_server_in("rust", None);
    assert_eq!(
        got,
        LspServerDiscovery::Absent {
            language_id: "rust".to_owned(),
            probed_command: "rust-analyzer".to_owned(),
        },
    );

    // A language this build knows no canonical server for: Absent with an EMPTY probed_command (the
    // honest "nothing was even probed" disclosure).
    let got = discover_lsp_server_in("cobol", Some(std::env::var_os("PATH").unwrap_or_default()));
    assert_eq!(
        got,
        LspServerDiscovery::Absent {
            language_id: "cobol".to_owned(),
            probed_command: String::new(),
        },
    );
}

/// MT-008 REMEDIATION (un-gated): the Found branch, proven deterministically against a temp dir
/// placed on the probe PATH containing a `rust-analyzer` executable file. The discovery must resolve
/// the ABSOLUTE launch path (so a later PATH change cannot redirect the lazy first spawn).
#[test]
fn lsp_discovery_finds_server_in_path_dir() {
    let dir = std::env::temp_dir().join(format!("hs-lsp-discovery-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create probe dir");
    let exe_name = if cfg!(windows) {
        "rust-analyzer.exe"
    } else {
        "rust-analyzer"
    };
    let exe = dir.join(exe_name);
    std::fs::write(&exe, b"stub-not-executed").expect("write probe stub");

    let got = discover_lsp_server_in("rust", Some(dir.clone().into_os_string()));
    match got {
        LspServerDiscovery::Found(config) => {
            let resolved = std::path::Path::new(&config.command);
            assert!(
                resolved.is_absolute(),
                "discovery resolves the ABSOLUTE executable path, got {resolved:?}"
            );
            assert!(
                config.command.ends_with(exe_name),
                "resolved command names the probed executable: {}",
                config.command
            );
            assert!(config.args.is_empty(), "no default args for rust-analyzer");
        }
        other => panic!("expected Found for an on-PATH executable, got {other:?}"),
    }

    let _ = std::fs::remove_file(&exe);
    let _ = std::fs::remove_dir(&dir);
}

/// Whether an OS process with `pid` currently exists (the no-zombie probe for the gated test).
fn process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
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

fn terminate_process(pid: u32) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

async fn matching_diagnostics(
    receiver: &mut tokio::sync::broadcast::Receiver<
        handshake_native::code_editor::lsp_client::PublishedDiagnostics,
    >,
    uri: &str,
    expect_empty: bool,
) -> handshake_native::code_editor::lsp_client::PublishedDiagnostics {
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let published = receiver
                .recv()
                .await
                .expect("diagnostics channel remains open");
            if published.uri == uri && published.diagnostics.is_empty() == expect_empty {
                return published;
            }
        }
    })
    .await
    .expect("real language server published matching diagnostics")
}

/// MT-008 V2 authoritative real-runtime proof. This is intentionally NOT ignored and does not use
/// the in-process mock transport: it launches clangd, indexes a generated two-file C++ workspace,
/// and drives the production stdio client through didOpen/didChange plus every MT-008 feature.
/// `HANDSHAKE_REAL_LSP_SERVER` may override the executable; otherwise `clangd` must be on PATH.
#[test]
fn real_language_server_end_to_end_runtime_proof() {
    struct WorkspaceCleanup(std::path::PathBuf);
    impl Drop for WorkspaceCleanup {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.0) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    panic!(
                        "remove generated real-LSP workspace {}: {error}",
                        self.0.display()
                    );
                }
            }
        }
    }

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let workspace = std::env::temp_dir().join(format!(
        "handshake-mt008-real-lsp-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&workspace).expect("create real-LSP workspace");
    let _workspace_cleanup = WorkspaceCleanup(workspace.clone());

    let header_path = workspace.join("math_ops.h");
    let source_path = workspace.join("main.cpp");
    let header =
        "/// Add two integers.\ninline int add_numbers(int lhs, int rhs) { return lhs + rhs; }\n";
    let opened_source = concat!(
        "#include \"math_ops.h\"\n",
        "\n",
        "int main() {\n",
        "    int result = add_numbers(20, 22);\n",
        "    int repeat = add_numbers(result, 1);\n",
        "    int broken = \"not an int\";\n",
        "    int candidate = add_\n",
        "    return result + repeat;\n",
        "}\n",
    );
    let changed_source = concat!(
        "#include \"math_ops.h\"\n",
        "\n",
        "int main() {\n",
        "    int result = add_numbers(20, 22);\n",
        "    int repeat = add_numbers(result, 1);\n",
        "    int broken = 0;\n",
        "    int candidate = add_numbers(1, 2);\n",
        "    return result + repeat + broken + candidate;\n",
        "}\n",
    );
    std::fs::write(&header_path, header).expect("write indexed header");
    std::fs::write(&source_path, opened_source).expect("write source file");
    std::fs::write(workspace.join("compile_flags.txt"), "-std=c++17\n-I.\n")
        .expect("write clangd compile flags");

    let canonical_source_path =
        std::fs::canonicalize(&source_path).expect("canonicalize generated source path");
    let canonical_header_path =
        std::fs::canonicalize(&header_path).expect("canonicalize generated header path");
    let source_uri = lsp_types::Url::from_file_path(&canonical_source_path)
        .expect("source path converts to file URI")
        .to_string();
    let header_uri = lsp_types::Url::from_file_path(&canonical_header_path)
        .expect("header path converts to file URI")
        .to_string();
    let server = std::env::var("HANDSHAKE_REAL_LSP_SERVER").unwrap_or_else(|_| "clangd".to_owned());
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("real-LSP runtime");
    let client = LspClient::new(LspServerConfig {
        command: server.clone(),
        args: vec![
            "--background-index=false".to_owned(),
            "--clang-tidy=false".to_owned(),
            "--header-insertion=never".to_owned(),
            "--log=error".to_owned(),
        ],
    });
    let mut diagnostics = client.subscribe_diagnostics();

    rt.block_on(async {
        client.did_open(&source_uri, "cpp", opened_source).await;
        assert!(
            client.is_running(),
            "{server:?} must launch and initialize as a real language-server process"
        );
        let pid = client
            .spawned_process_id_for_test()
            .expect("production transport exposes the real server pid");
        assert!(
            process_alive(pid),
            "real language-server pid {pid} is alive"
        );

        let completion = client
            .completion(
                &source_uri,
                lsp_types::Position {
                    line: 6,
                    character: 24,
                },
            )
            .await;
        assert!(
            completion
                .iter()
                .any(|item| item.label.contains("add_numbers")),
            "real indexed completion contains add_numbers; got {completion:?}"
        );

        let symbol_position = lsp_types::Position {
            line: 3,
            character: 20,
        };
        let hover = client
            .hover(&source_uri, symbol_position)
            .await
            .expect("real server returns hover for add_numbers");
        assert!(
            hover.value.contains("add_numbers"),
            "real hover describes add_numbers; got {:?}",
            hover.value
        );

        let definition = client
            .goto_definition(&source_uri, symbol_position)
            .await
            .expect("real server resolves cross-file definition");
        assert_eq!(definition.uri.to_string(), header_uri);
        assert_eq!(definition.range.start.line, 1);

        let references = client.references(&source_uri, symbol_position).await;
        assert!(
            references.len() >= 2,
            "real index returns the two populated call sites; got {references:?}"
        );
        for expected_line in [3, 4] {
            assert!(
                references.iter().any(|location| {
                    location.uri.to_string() == source_uri
                        && location.range.start.line == expected_line
                }),
                "real references include call-site line {expected_line}; got {references:?}"
            );
        }

        let broken = matching_diagnostics(&mut diagnostics, &source_uri, false).await;
        assert!(
            broken
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.line == 5),
            "real diagnostics identify the invalid string-to-int assignment; got {broken:?}"
        );

        client.did_change(&source_uri, 2, changed_source).await;
        let fixed = matching_diagnostics(&mut diagnostics, &source_uri, true).await;
        assert!(
            fixed.version.is_none() || fixed.version == Some(2),
            "cleared diagnostics correspond to didChange version 2; got {fixed:?}"
        );
    });

    let pid = client
        .spawned_process_id_for_test()
        .expect("real server remains attached before host shutdown");
    client.shutdown_for_host();
    assert!(
        wait_for_process_exit(pid),
        "host shutdown reaps real language-server pid {pid} without a zombie"
    );

    // A second real process proves the production crash/error path: after abrupt server loss, the
    // reader marks the transport dead and a feature request degrades promptly instead of hanging.
    let failed_client = LspClient::new(LspServerConfig {
        command: server,
        args: vec![
            "--background-index=false".to_owned(),
            "--log=error".to_owned(),
        ],
    });
    let failed_pid = rt.block_on(async {
        failed_client
            .did_open(&source_uri, "cpp", changed_source)
            .await;
        assert!(failed_client.is_running(), "second real server initializes");
        failed_client
            .spawned_process_id_for_test()
            .expect("second real server pid")
    });
    assert!(
        terminate_process(failed_pid),
        "test can terminate its exact child server pid {failed_pid}"
    );
    assert!(
        wait_for_process_exit(failed_pid),
        "abruptly terminated server pid {failed_pid} exits"
    );
    let after_crash = rt
        .block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(2),
                failed_client.hover(
                    &source_uri,
                    lsp_types::Position {
                        line: 3,
                        character: 20,
                    },
                ),
            )
            .await
        })
        .expect("post-crash feature request returns within two seconds");
    assert!(after_crash.is_none(), "post-crash hover degrades to None");
    drop(failed_client);
    drop(rt);
}
