//! MT-008 LSP client proofs (WP-KERNEL-012 E1 code editor). These run STANDALONE — no backend, no
//! real language server — so they are part of the default `cargo test` run.
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

use handshake_native::code_editor::lsp_client::{
    published_diagnostics_from_lsp, LspClient, LspServerConfig,
};

/// AC-004: a client built with NO server config is not configured + not running.
#[test]
fn lsp_client_graceful_unconfigured_is_not_running() {
    let client = LspClient::disabled();
    assert!(!client.is_configured(), "AC-004: disabled client reports not configured");
    assert!(!client.is_running(), "AC-004: disabled client has no spawned process");

    // A config with a non-empty command IS configured (but still not spawned until did_open).
    let configured = LspClient::new(LspServerConfig::command("rust-analyzer"));
    assert!(configured.is_configured());
    assert!(!configured.is_running(), "configured but not spawned until did_open");
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
        assert!(!client.initialize(None).await, "AC-004: initialize false without server");
        // did_open / did_change are graceful no-ops (no panic, no spawn).
        client.did_open("file:///x.rs", "rust", "fn main() {}").await;
        client.did_change("file:///x.rs", 2, "fn main() {}").await;
        assert!(!client.is_running(), "AC-004: no process spawned for a disabled client");

        let pos = lsp_types::Position { line: 0, character: 0 };
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
        let published = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            diagnostics_rx.recv(),
        )
        .await
        .expect("AC-008: publishDiagnostics routed within the timeout")
        .expect("AC-008: diagnostics channel delivered a notification");

        assert_eq!(published.uri, "file:///mock.rs");
        assert_eq!(published.diagnostics.len(), 1, "AC-008: one diagnostic received");
        assert_eq!(
            published.diagnostics[0].line, 4,
            "AC-008: LSP range.start.line (0-based) maps to the gutter line"
        );
        assert_eq!(published.diagnostics[0].severity, 1, "AC-008: error severity preserved");
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

/// Sanity: the LSP->editor diagnostic mapping (`published_diagnostics_from_lsp`) is the same function
/// the channel feeds, so a direct call mirrors what the gutter receives (AC-008 mapping).
#[test]
fn lsp_diagnostics_map_to_zero_based_lines() {
    use lsp_types::{Diagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Url};
    let params = PublishDiagnosticsParams {
        uri: Url::parse("file:///z.rs").unwrap(),
        version: None,
        diagnostics: vec![Diagnostic {
            range: Range {
                start: Position { line: 7, character: 1 },
                end: Position { line: 7, character: 4 },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            message: "w".to_owned(),
            ..Default::default()
        }],
    };
    let mapped = published_diagnostics_from_lsp(params);
    assert_eq!(mapped.diagnostics[0].line, 7);
    assert_eq!(mapped.diagnostics[0].severity, 2);
}
