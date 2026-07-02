//! WP-KERNEL-012 MT-047 signature help (parameter hints) LIVE proofs (E1 — VS Code parity).
//!
//! These tests drive the REAL signature-help surface and inspect the actual results / live AccessKit
//! tree / rendered pixels — the same data a swarm agent reads out-of-process and an operator sees:
//!
//! - PT-001 `signature_help_request`: a MOCK LSP (an in-memory duplex-pipe transport installed via the
//!   REAL `LspClient::install_test_transport`) returns one `SignatureInformation`; the parsed
//!   `active_parameter` index matches the mock. Drives the EXACT production
//!   `request()` -> framed write -> `read_loop` -> `route_message` path (no parallel reimplementation).
//! - PT-002 `signature_help_graceful`: with no LSP attached, the code-nav fallback yields a signature
//!   from the backend symbol's REAL `display_name` (a BARE identifier — the backend sets
//!   `display_name = symbol.name.clone()`, so against real data the fallback is a bare-name popup with
//!   ZERO parameters); when the backend is also unreachable nothing renders and no panic occurs
//!   (AC-003 / AC-008).
//! - PT-003 `signature_help_popup`: an egui_kittest screenshot proves the active-parameter run renders
//!   EMPHASIZED (distinct color) vs the inactive runs; the PNG is saved to the EXTERNAL artifact root.
//! - PT-004 `signature_help_accesskit`: the live tree contains a `Role::Tooltip` node
//!   `code_editor_signature_help` whose value carries the active signature label (AC-005 / MC-006).
//! - PT-005 `active_parameter_from_commas`: the top-level comma counter ignores nested calls + string /
//!   char literals (AC-007).
//!
//! The screenshot/AccessKit halves are STANDALONE (the popup rendering + AccessKit emission are
//! independent of the backend — the SignatureHelpState is fed synthetically through the panel's public
//! `open_signature_help` API, the same way the MT-008 completion/hover proofs feed their state). Those
//! synthetic states use an LSP-shaped signature with explicit parameter spans BECAUSE an LSP server
//! (when attached) is the only source that carries real parameter labels; they prove the renderer +
//! AccessKit emphasis logic, not the code-nav fallback's parameter content.
//!
//! NEEDS_MANAGED_RESOURCE_PROOF (Spec-Realism Gate sub-rule 2): the code-nav FALLBACK parameter hints
//! are NOT provable against the real Handshake backend, because the code-nav resource carries no
//! parameter data. The backend `symbol_to_json` projection's `display_name` is a BARE identifier
//! (`display_name = symbol.name.clone()`, engine.rs:781; `ExtractedSymbol.name` = "Simple identifier,
//! e.g. `render`", symbols.rs:79-80) and neither `GET /knowledge/code/symbols/:entity_id` nor
//! `GET /knowledge/code/symbols/:entity_id/spans` returns a parameter signature or the source text
//! needed to derive one (the spans endpoint returns only line/byte ranges + a content hash). So the
//! production-realistic fallback is a bare-name popup with ZERO parameters (proven by PT-002 against a
//! captured real-shape symbol). A parameter-bearing code-nav fallback would require a NEW backend route
//! (source-text-by-span or a server-provided signature field) — a backend gap, reported as a typed
//! blocker, never a frontend-faked signature. The fallback PARSING code (`signature_from_code_nav_symbol`
//! splitting a parenthesized `display_name`) is retained as a no-cost forward-compat path for any future
//! server/adapter that does emit a rich label, and is unit-tested as such — but it is NOT asserted to
//! produce parameter hints from the current real backend.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::code_editor::code_nav::{CodeNavClient, CodeSymbolNavProjection};
use handshake_native::code_editor::lsp_client::LspClient;
use handshake_native::code_editor::signature_help::{
    active_parameter_from_commas, SignatureHelpState, SignatureInfo, SignatureSource,
    CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID,
};
use handshake_native::code_editor::CodeEditorPanel;

/// The external artifact root for MT-047 screenshots (CX-212E — NEVER repo-local). The same
/// `../../../../Handshake_Artifacts/handshake-test/<subdir>` pattern the MT-008 proofs use.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Hard guard (CX-212E): NO repo-local `test_output/` or `tests/screenshots/` dir may exist after a
/// screenshot test — artifacts go ONLY to the external Handshake_Artifacts root. The reviewer also runs
/// `git ls-files src/**/*.png` and fails the MT if any artifact is tracked.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        assert!(
            !Path::new(local).exists(),
            "no repo-local {local}/ dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (CX-212E)"
        );
    }
}

/// Build a synthetic two-parameter signature state with the SECOND parameter active (the shape the
/// renderer + AccessKit proofs feed). Uses explicit label spans (the `LabelOffsets` resolution path).
fn synthetic_state_active_param_1() -> SignatureHelpState {
    let sig = SignatureInfo {
        label: "fn add(a: i32, b: i32) -> i32".to_owned(),
        parameters: vec![
            handshake_native::code_editor::signature_help::ParamSpan {
                label: "a: i32".into(),
                range_in_label: Some(7..13),
            },
            handshake_native::code_editor::signature_help::ParamSpan {
                label: "b: i32".into(),
                range_in_label: Some(15..21),
            },
        ],
        documentation: Some("Adds two numbers".to_owned()),
    };
    SignatureHelpState {
        signatures: vec![sig],
        active_signature: 0,
        active_parameter: 1,
        anchor_byte: 0,
        source: SignatureSource::Lsp,
    }
}

// ── PT-001 / AC-001: mock-LSP request -> parsed active_parameter matches the mock ──────────────────

#[test]
fn signature_help_request_parses_active_parameter_from_mock_lsp() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled();
        // The server declared signatureHelpProvider (so the client issues the request — AC-006 gate).
        client.set_signature_help_capability_for_test(&['(', ',']);
        // Install the in-memory duplex transport wired to the REAL request() path; we (the mock server)
        // hold the server side of the pipe.
        let mut server = client.install_test_transport();

        // Fire the request on the client (the same call the editor's trigger spawns).
        let pos = lsp_types::Position { line: 1, character: 25 };
        let client_arc = Arc::new(client);
        let req_client = Arc::clone(&client_arc);
        let request = tokio::spawn(async move {
            req_client.signature_help("file:///mock.rs", pos).await
        });

        // Read the framed request the client emitted over the REAL transport, assert it is the right
        // method, then write the framed response back (the mock server's reply).
        let req = client_arc
            .read_test_request()
            .await
            .expect("the client wrote a framed signatureHelp request");
        assert_eq!(
            req.get("method").and_then(|m| m.as_str()),
            Some("textDocument/signatureHelp"),
            "AC-001: the request method is textDocument/signatureHelp"
        );
        let id = req.get("id").cloned().expect("the request carries an id");

        // The mock server's response: one SignatureInformation, active_parameter = 1.
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "signatures": [{
                    "label": "fn add(a: i32, b: i32) -> i32",
                    "parameters": [
                        { "label": [7, 13] },
                        { "label": "b: i32" }
                    ]
                }],
                "activeSignature": 0,
                "activeParameter": 1
            }
        });
        let frame = LspClient::frame_message_for_test(&response);
        use tokio::io::AsyncWriteExt;
        server.write_all(&frame).await.expect("write response frame");
        server.flush().await.expect("flush");

        // The client's request resolves with the parsed SignatureHelp.
        let help = tokio::time::timeout(std::time::Duration::from_secs(5), request)
            .await
            .expect("AC-001: the signatureHelp request resolved within the timeout")
            .expect("join")
            .expect("AC-001: a Some(SignatureHelp) was parsed from the mock response");

        // Convert via the SAME production path the editor uses, and assert the active parameter index.
        let state = SignatureHelpState::from_lsp(&help, 42).expect("one signature -> Some");
        assert_eq!(
            state.active_parameter, 1,
            "AC-001: the parsed active_parameter index matches the mock (1)"
        );
        assert_eq!(state.source, SignatureSource::Lsp);
        // The active run resolves to the second parameter 'b: i32'.
        let sig = state.active().unwrap();
        let runs =
            handshake_native::code_editor::signature_help::signature_label_runs(sig, state.active_parameter);
        let active: String = runs.iter().filter(|(_, a)| *a).map(|(t, _)| t.clone()).collect();
        assert_eq!(active, "b: i32", "AC-001: the active run is the second parameter");
        println!(
            "PT-001 signature_help_request: method=textDocument/signatureHelp, active_parameter={}, \
             active_run={active:?}",
            state.active_parameter
        );
    });
}

/// AC-006 gate: when the server did NOT declare signatureHelpProvider, the client SKIPS the request
/// entirely (no second transport, no wasted round-trip) and returns None so the caller falls back.
#[test]
fn signature_help_request_skipped_without_server_capability() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = LspClient::disabled(); // no capability set.
        assert!(!client.supports_signature_help(), "no capability declared");
        let pos = lsp_types::Position {
            line: 0,
            character: 0,
        };
        // No transport installed + no capability -> immediate None, no panic (AC-006 / AC-008).
        assert!(
            client.signature_help("file:///x.rs", pos).await.is_none(),
            "AC-006: signature_help is a graceful None without the server capability"
        );
        // The default trigger chars are the contract's '(' and ','.
        assert_eq!(client.signature_help_trigger_chars(), vec!['(', ',']);
    });
}

// ── PT-002 / AC-003: graceful fallback (no LSP -> code-nav signature; backend down -> nothing) ─────

#[test]
fn signature_help_graceful_fallback_and_no_panic() {
    // The fallback LOGIC against the REAL backend shape (AC-003). This fixture is the CAPTURED real
    // `symbol_to_json` projection a code symbol produces: `display_name` is a BARE identifier
    // (`add`) — the backend sets `display_name: symbol.name.clone()`
    // (knowledge_code_index/engine.rs:781) and `ExtractedSymbol.name` is "Simple identifier, e.g.
    // `render`" (knowledge_code_index/symbols.rs:79-80). The code-nav API (`symbol_to_json` /
    // `symbol_spans`) carries NO parameter list or signature text, so the production-realistic
    // fallback is a BARE-NAME popup with ZERO parameters — NOT the paren-bearing synthetic fixture an
    // earlier draft seeded. The richer-param fallback is the NEEDS_MANAGED_RESOURCE_PROOF gap named in
    // the MT lifecycle (see the module header).
    let real_symbol = CodeSymbolNavProjection {
        symbol_entity_id: "ent-add".into(),
        symbol_key: "fn:src/math.rs#add".into(),
        display_name: "add".into(), // bare identifier — the REAL backend shape.
        symbol_kind: "function".into(),
        ..Default::default()
    };
    // active_parameter computed locally from the comma count (the fallback path). Even though the
    // local comma count says "the cursor is on argument 1", the REAL backend symbol exposes no
    // parameters, so the popup degrades to the bare call-target name with no active-parameter run.
    let state = SignatureHelpState::from_code_nav(&real_symbol, 10, 1)
        .expect("AC-003: a real code-nav symbol still yields a (bare-name) fallback signature");
    assert_eq!(state.source, SignatureSource::CodeNavFallback);
    let sig = state.active().unwrap();
    assert_eq!(
        sig.label, "add",
        "AC-003: the real fallback label is the bare call-target name"
    );
    assert!(
        sig.parameters.is_empty(),
        "AC-003: the REAL backend symbol carries no parameter signature (bare display_name) — the \
         fallback shows the call target with zero parameters; the parameter-hint fallback is the \
         NEEDS_MANAGED_RESOURCE_PROOF code-nav gap"
    );
    // The whole label is ONE inactive run — no active-parameter emphasis against real data.
    let runs = handshake_native::code_editor::signature_help::signature_label_runs(
        sig,
        state.active_parameter,
    );
    assert_eq!(
        runs,
        vec![("add".to_owned(), false)],
        "AC-003: against the real bare display_name the popup shows no emphasized parameter run"
    );

    // Backend unreachable: a panel with NO runtime + NO workspace renders nothing and never panics. The
    // trigger is a graceful no-op (no runtime to spawn on), and rendering a closed popup is a no-op.
    let panel = CodeEditorPanel::new("fn caller() { add(1, ) }", "rs");
    assert!(
        !panel.is_signature_help_open(),
        "AC-003: nothing shown before any trigger"
    );
    // A graceful no-op trigger path: no runtime -> the pump clears the request, no spawn, no panic.
    // (Driving a frame exercises the pump; without a runtime it cannot reach the backend.)
    // Build a headless harness to run a frame.
    let panel = Arc::new(panel);
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 200.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    assert!(
        !panel.is_signature_help_open(),
        "AC-003: with no LSP + no backend reachable, nothing renders (and no panic occurred)"
    );

    // A live code-nav client against an unreachable base URL: a lookup errors -> the fallback resolves
    // to no symbol -> nothing renders (proving the unreachable-backend branch degrades gracefully).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let client = CodeNavClient::new("http://127.0.0.1:1"); // nothing listening.
        let result = client.lookup_symbols("ws", "add", 5).await;
        assert!(
            result.is_err(),
            "AC-003: an unreachable backend errors (caller treats as no symbol)"
        );
    });
    println!("PT-002 signature_help_graceful: fallback signature parsed; unreachable backend -> no panic, nothing rendered");
}

// ── PT-005 / AC-007: top-level comma counter ignores nested calls + string/char literals ───────────

#[test]
fn active_parameter_from_commas_ignores_nested_and_literals() {
    // Nested call: the inner comma must not advance the outer active parameter (RISK-001).
    let nested = "(a, inner(x, y), ";
    assert_eq!(
        active_parameter_from_commas(nested, 0, nested.len()),
        2,
        "AC-007: nested-call comma ignored"
    );
    // String + char literals: commas inside them are skipped.
    let strs = "(a, \"x, y\", ";
    assert_eq!(
        active_parameter_from_commas(strs, 0, strs.len()),
        2,
        "AC-007: string comma ignored"
    );
    let chr = "(a, ',', ";
    assert_eq!(
        active_parameter_from_commas(chr, 0, chr.len()),
        2,
        "AC-007: char comma ignored"
    );
    // Generic angle brackets: the comma inside `<K, V>` is skipped.
    let generic = "(a, HashMap<K, V>, ";
    assert_eq!(
        active_parameter_from_commas(generic, 0, generic.len()),
        2,
        "AC-007: generic comma ignored"
    );
    // A plain top-level list counts correctly.
    let plain = "(a, b, c";
    assert_eq!(active_parameter_from_commas(plain, 0, plain.len()), 2);
    println!("PT-005 active_parameter_from_commas: nested/string/char/generic commas all ignored");
}

// ── PT-004 / AC-005: AccessKit Tooltip node carries the active signature label ─────────────────────

#[test]
fn signature_help_accesskit_tooltip_node_present() {
    let panel = Arc::new(CodeEditorPanel::new("fn caller() { add(1, 2) }", "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 300.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    assert!(
        !panel.is_signature_help_open(),
        "signature help starts closed"
    );

    // Open the popup with the synthetic state (the deterministic path; a live trigger delivers the same
    // state off-thread into the same slot).
    panel.open_signature_help(synthetic_state_active_param_1());
    harness.run();
    harness.run(); // settle so the AccessKit node is emitted.
    assert!(
        panel.is_signature_help_open(),
        "AC-005: signature help popup is open"
    );

    let root = harness.root();
    let mut node_role: Option<String> = None;
    let mut node_value: Option<String> = None;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID) {
            node_role = Some(format!("{:?}", ak.role()));
            node_value = ak.value().map(|s| s.to_owned());
            break;
        }
    }
    assert_eq!(
        node_role.as_deref(),
        Some("Tooltip"),
        "AC-005: '{CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID}' must be a Role::Tooltip node"
    );
    let value = node_value.expect("AC-005: the signature-help node carries a value");
    assert!(
        value.contains("fn add(a: i32, b: i32)"),
        "AC-005: the node value carries the active signature label; got {value:?}"
    );
    assert!(
        value.contains("b: i32"),
        "MC-006: the node value names the active parameter so a swarm agent reads the hint; got {value:?}"
    );
    println!("PT-004 signature_help_accesskit: {{\"{CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID}\":\"Tooltip\", value contains the active signature label}}");

    // Closing removes the node from the tree.
    panel.close_signature_help();
    harness.run();
    harness.run();
    let still = harness
        .root()
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID));
    assert!(
        !still,
        "AC-005: the signature-help node is removed after closing"
    );
}

// ── PT-003 / AC-004: screenshot proves the active parameter renders emphasized ─────────────────────

#[test]
fn signature_help_popup_emphasizes_active_parameter_screenshot() {
    let panel = Arc::new(CodeEditorPanel::new("fn caller() { add(1, 2) }", "rs"));
    let panel_ui = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 300.0))
        .wgpu()
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    harness.run();
    // Open the popup with the active parameter = 1 ('b: i32') emphasized.
    panel.open_signature_help(synthetic_state_active_param_1());
    harness.run();
    harness.run(); // settle so the popup paints.
    assert!(
        panel.is_signature_help_open(),
        "AC-004: the popup is open for the screenshot"
    );

    // The renderer emphasizes the active run with the selection stroke color (distinct from the default
    // monospace text). Proof: render the frame, save the PNG to the EXTERNAL artifact root, and confirm
    // the popup drew a non-trivial number of colored pixels matching the emphasis color band.
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            let raw = image.as_raw();
            // The egui dark-theme selection stroke is a light-blue-ish accent; count pixels that are
            // clearly blue-dominant (b high, b > r and b > g) — the emphasized run's color. The bulk of
            // the signature text is near-white monospace (r==g==b high), which this signature excludes.
            let mut emphasis = 0usize;
            let mut i = 0usize;
            while i + 4 <= raw.len() {
                let (r, g, b, a) = (
                    raw[i] as i32,
                    raw[i + 1] as i32,
                    raw[i + 2] as i32,
                    raw[i + 3],
                );
                if a != 0 && b > 130 && b > r + 30 && b > g + 10 {
                    emphasis += 1;
                }
                i += 4;
            }
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-047");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-047-signature-help-popup.png");
            let saved = image.save(&png_path).is_ok();
            println!(
                "PT-003 signature_help_popup screenshot: {w}x{h}, emphasis_pixels={emphasis}, \
                 saved={saved} ({})",
                png_path.display()
            );
            assert!(
                emphasis >= 8,
                "AC-004: the active parameter must render with a distinct emphasis color; got \
                 {emphasis} emphasis-colored pixels (expected the 'b: i32' run to paint accent-blue)"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-047 signature-help screenshot render unavailable (no wgpu \
                 adapter): {e}. The popup-open state + the emphasized-run logic (signature_label_runs) \
                 + the AccessKit Tooltip node prove the active-parameter emphasis; the PNG color check \
                 is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}
