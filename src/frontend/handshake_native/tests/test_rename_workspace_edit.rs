//! WP-KERNEL-012 MT-048 Rename Symbol — WorkspaceEdit apply + LSP transport LIVE proofs (E1 VS Code
//! parity).
//!
//! These tests drive the REAL rename apply + the REAL MT-008 LSP transport:
//!
//! - PT-001 / AC-002 `rename_workspace_edit`: a mocked LSP `WorkspaceEdit` spanning 2 files is applied to
//!   2 open buffers, edit offsets adjusted correctly (DESCENDING-offset apply order verified — RISK-001 —
//!   and an ASCENDING-order regression must FAIL), resulting buffer text exact-matches expected.
//! - AC-007 / PT-006 `rename_request_reuses_mt008_transport`: the `textDocument/rename` request reuses the
//!   EXISTING MT-008 stdio JSON-RPC transport (the SAME `install_test_transport` mock-LSP path the MT-008 /
//!   MT-047 proofs use — no second transport) AND parses BOTH the `changes` map form AND the
//!   `documentChanges` array form of the WorkspaceEdit response to the same FileEditPreview set.
//!
//! Provable WITHOUT a live PostgreSQL / a real language server: the mock LSP is an in-memory duplex pipe
//! installed via the REAL `LspClient::install_test_transport`, and the 2-file apply uses in-memory
//! `TextBuffer`s — exactly the MT proof discipline.

use std::sync::Arc;

use handshake_native::code_editor::lsp_client::LspClient;
use handshake_native::code_editor::rename::{
    apply_preview, apply_text_edits_to_string, FileEditPreview, LspRange, TextEdit,
    WorkspaceEditPreview,
};

// ── PT-001 / AC-002 / RISK-001 / MC-001: 2-file WorkspaceEdit applied, descending-offset correct ──────

#[test]
fn rename_workspace_edit_applies_two_files_descending_offset() {
    // Two files, each with two `foo` occurrences renamed to a LONGER name `frobnicate` (so a length change
    // exposes any offset-shift bug). file_a has occurrences on two lines; file_b has two on one line.
    let file_a = "fn foo() {}\nfn bar() { foo(); }";
    let file_b = "fn baz() { foo(); foo(); }";

    let preview = WorkspaceEditPreview {
        files: vec![
            FileEditPreview {
                uri: "file:///a.rs".into(),
                is_open_buffer: true,
                edits: vec![
                    // `foo` at line 0 col 3..6 (the definition), and line 1 col 11..14 (the call).
                    TextEdit {
                        range: LspRange { start_line: 0, start_char: 3, end_line: 0, end_char: 6 },
                        new_text: "frobnicate".into(),
                    },
                    TextEdit {
                        range: LspRange { start_line: 1, start_char: 11, end_line: 1, end_char: 14 },
                        new_text: "frobnicate".into(),
                    },
                ],
                hunks: vec![],
            },
            FileEditPreview {
                uri: "file:///b.rs".into(),
                is_open_buffer: true,
                edits: vec![
                    // TWO occurrences on the SAME line — the exact case where ascending apply corrupts:
                    // applying the FIRST (col 11..14) lengthens the line, so the SECOND's original col
                    // 18..21 would point at the WRONG text unless we apply descending.
                    TextEdit {
                        range: LspRange { start_line: 0, start_char: 11, end_line: 0, end_char: 14 },
                        new_text: "frobnicate".into(),
                    },
                    TextEdit {
                        range: LspRange { start_line: 0, start_char: 18, end_line: 0, end_char: 21 },
                        new_text: "frobnicate".into(),
                    },
                ],
                hunks: vec![],
            },
        ],
        is_single_file_fallback: false,
    };

    use std::collections::HashMap;
    let mut buffers: HashMap<String, String> = HashMap::new();
    buffers.insert("file:///a.rs".into(), file_a.to_owned());
    buffers.insert("file:///b.rs".into(), file_b.to_owned());
    let read_buffers = buffers.clone();

    let report = apply_preview(
        &preview,
        |uri| read_buffers.get(uri).cloned(),
        |uri, new_text| {
            buffers.insert(uri.to_owned(), new_text.to_owned());
        },
    )
    .expect("AC-002: the 2-file WorkspaceEdit applies cleanly");

    assert_eq!(report.files_changed, vec!["file:///a.rs", "file:///b.rs"]);
    assert_eq!(report.edits_applied, 4, "AC-002: all 4 occurrences applied");
    assert_eq!(
        buffers["file:///a.rs"],
        "fn frobnicate() {}\nfn bar() { frobnicate(); }",
        "AC-002: file a renamed exactly"
    );
    assert_eq!(
        buffers["file:///b.rs"],
        "fn baz() { frobnicate(); frobnicate(); }",
        "AC-002 / RISK-001: file b's two same-line occurrences both renamed (descending-offset apply)"
    );

    // RISK-001 regression: prove ASCENDING-order apply CORRUPTS file_b. Apply edit[0] first (the lower
    // offset), then edit[1] at its ORIGINAL byte range — the naive bug.
    let edits = &preview.files[1].edits;
    let correct = apply_text_edits_to_string(file_b, edits).unwrap();
    assert_eq!(correct, buffers["file:///b.rs"], "the descending apply equals the real result");
    // Naive ascending: resolve both ranges against the ORIGINAL text, apply low-offset first.
    let mut naive = file_b.to_owned();
    // edit[0] = col 11..14 ("foo" -> "frobnicate"): replace bytes 11..14.
    naive.replace_range(11..14, "frobnicate");
    // edit[1] = col 18..21 against the ORIGINAL — but `naive` is now longer, so 18..21 hits the WRONG span.
    // Clamp to avoid a panic (a real ascending bug mangles text rather than panicking).
    let len = naive.len();
    let s = 18.min(len);
    let e = 21.min(len);
    naive.replace_range(s..e, "frobnicate");
    assert_ne!(
        naive, correct,
        "RISK-001 regression: ascending-order apply must NOT equal the correct descending result"
    );
    println!(
        "PT-001 rename_workspace_edit: 2 files, 4 edits applied descending; ascending corrupts (regression proven)\n  a.rs => {:?}\n  b.rs => {:?}",
        buffers["file:///a.rs"], buffers["file:///b.rs"]
    );
}

// ── AC-007 / PT-006: textDocument/rename reuses the MT-008 transport + parses BOTH WorkspaceEdit forms ──

#[test]
fn rename_request_reuses_mt008_transport_and_parses_both_workspace_edit_forms() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        // ── Round 1: the `changes` map form ──
        let changes_response = serde_json::json!({
            "changes": {
                "file:///x.rs": [
                    { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 6 } }, "newText": "renamed" }
                ]
            }
        });
        let edit_changes = run_rename_over_mock_transport(changes_response).await;

        // ── Round 2: the `documentChanges` array form (same logical edit) ──
        let doc_changes_response = serde_json::json!({
            "documentChanges": [
                {
                    "textDocument": { "uri": "file:///x.rs", "version": 1 },
                    "edits": [
                        { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 6 } }, "newText": "renamed" }
                    ]
                }
            ]
        });
        let edit_doc = run_rename_over_mock_transport(doc_changes_response).await;

        // Both responses parse to the SAME FileEditPreview set (AC-007 / PT-006).
        let p_changes = WorkspaceEditPreview::from_lsp(&edit_changes, |_| None);
        let p_doc = WorkspaceEditPreview::from_lsp(&edit_doc, |_| None);
        assert_eq!(p_changes.files.len(), 1, "changes form -> 1 file");
        assert_eq!(p_doc.files.len(), 1, "documentChanges form -> 1 file");
        assert_eq!(p_changes.files[0].uri, p_doc.files[0].uri, "PT-006: same uri from both forms");
        assert_eq!(
            p_changes.files[0].edits, p_doc.files[0].edits,
            "PT-006: the `changes` map and `documentChanges` array forms parse to the SAME edits"
        );
        assert_eq!(
            p_changes.files[0].edits[0],
            TextEdit {
                range: LspRange { start_line: 0, start_char: 3, end_line: 0, end_char: 6 },
                new_text: "renamed".into(),
            }
        );
        println!(
            "PT-006 rename both-forms: changes-map and documentChanges-array parse to the same FileEditPreview ({} edit)",
            p_changes.files[0].edits.len()
        );
    });
}

/// Fire `LspClient::rename` over the REAL in-memory MT-008 transport (`install_test_transport`), assert
/// the request method is `textDocument/rename` (AC-007 — no second transport), write `response_result` as
/// the `result` of the framed response, and return the parsed `lsp_types::WorkspaceEdit`.
async fn run_rename_over_mock_transport(
    response_result: serde_json::Value,
) -> lsp_types::WorkspaceEdit {
    let client = LspClient::disabled();
    let mut server = client.install_test_transport();
    let client_arc = Arc::new(client);

    let req_client = Arc::clone(&client_arc);
    let pos = lsp_types::Position { line: 0, character: 4 };
    let request = tokio::spawn(async move {
        req_client.rename("file:///x.rs", pos, "renamed").await
    });

    // Observe the framed request the client wrote over the REAL transport.
    let req = client_arc
        .read_test_request()
        .await
        .expect("the client wrote a framed rename request");
    assert_eq!(
        req.get("method").and_then(|m| m.as_str()),
        Some("textDocument/rename"),
        "AC-007: the request method is textDocument/rename over the EXISTING MT-008 transport (no 2nd transport)"
    );
    // Assert the params carry newName + the position params (the contract's request shape).
    let params = req.get("params").expect("rename request carries params");
    assert_eq!(
        params.get("newName").and_then(|n| n.as_str()),
        Some("renamed"),
        "AC-007: the rename request carries newName"
    );
    assert!(
        params.get("textDocument").is_some() && params.get("position").is_some(),
        "AC-007: the rename request carries the textDocument + position params"
    );
    let id = req.get("id").cloned().expect("the request carries an id");

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": response_result,
    });
    let frame = LspClient::frame_message_for_test(&response);
    use tokio::io::AsyncWriteExt;
    server.write_all(&frame).await.expect("write response frame");
    server.flush().await.expect("flush");

    let edit = tokio::time::timeout(std::time::Duration::from_secs(5), request)
        .await
        .expect("the rename request resolved within the timeout")
        .expect("join")
        .expect("a WorkspaceEdit was parsed from the mock response");
    edit
}
