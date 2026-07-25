//! FEMS memory-write PROPOSAL proofs — WP-KERNEL-012 MT-064 (cluster E9).
//!
//! This suite proves the editor→FEMS proposal path at unit/widget/client boundaries. The current backend
//! owns the live review-gated proposal route and native-editor Flight Recorder ingestion; managed-PG
//! correlation proofs live in `test_fems_interop_proofs`. The 404 typed-blocker remains covered so an
//! older or capability-restricted backend never triggers a direct-write fallback.
//!
//! Proof map:
//! - PT-001 / AC-001: `build_proposal` sets class + FULL source provenance from a TextRange selection
//!   (covered in the module unit tests; re-asserted here through the public API).
//! - PT-002 / AC-002: `procedural_review_gated` asserts review_gated==true for Procedural and no
//!   editor path sets it false (no direct-commit call site — MC-001/002).
//! - PT-003 / AC-004: `propose_creates_proposal_via_endpoint` submits a proposal to an in-process mock
//!   "proposal endpoint" (200 → ProposalAck) and asserts the FR-EVT-MEM-001 event SHAPE the MT-036
//!   emitter would post. The LIVE PG record + LIVE FR ingestion is the double-gate blocker (recorded).
//! - PT-004 / AC-005: `missing_endpoint_blocker` — a 404 from the mock maps to
//!   `MemoryProposalError::MissingEndpoint` (the typed blocker), with NO commit and NO silent fallback.
//! - PT-005 / AC-007: `propose_dialog_accesskit_nodes_present` dumps the live AccessKit tree and asserts
//!   `fems-propose-dialog` (Dialog), `fems-class-{episodic|semantic|procedural}` (RadioButton), and
//!   `fems-propose-confirm` (Button) present with the correct roles; saves a screenshot to the EXTERNAL
//!   root.
//! - AC-009 (no backend / no SQLite) + MC-001 (no direct-commit call site): `read_no_direct_commit_site`
//!   greps the production source for any direct memory-commit/write route and asserts the only write
//!   path is the proposal POST; `assert_no_local_artifact_dir` guards artifact hygiene (CX-212E).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::NodeT;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
use serde_json::{json, Value};

use handshake_native::event_emitter::{
    EventLedgerTransport, NativeEditorEvent, NativeEditorEventEmitter,
};
use handshake_native::fems::memory_proposal::{
    build_proposal_for_document, content_hash_of_selection, fems_class_author_id, submit_proposal,
    submit_proposal_and_emit, HandshakeCoreClient, MemoryClass, MemoryProposalError,
    ProposalSubmitOutcome, ProposeToMemoryDialog, FEMS_PROPOSE_CANCEL_AUTHOR_ID,
    FEMS_PROPOSE_COMMAND_ID, FEMS_PROPOSE_CONFIRM_AUTHOR_ID, FEMS_PROPOSE_DIALOG_AUTHOR_ID,
    FEMS_PROPOSE_DIALOG_NODE_ID,
};
use handshake_native::interop::{EditorSurfaceKind, SharedSelection};
use handshake_native::theme::HsTheme;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Artifact hygiene (CX-212E / SCREENSHOT RULE): all artifacts go to the EXTERNAL root ONLY.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the SCREENSHOT/TEST-ARTIFACT RULE).
/// Artifacts go to the external `Handshake_Artifacts/handshake-test` root ONLY; a stray `test_output/`
/// OR `tests/screenshots/` is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// In-process mock HTTP server (the PROVEN MT-020/MT-037/MT-063 TcpListener pattern — no new dependency).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The captured request line + body a mock exchange delivers (the reply is configured by the caller).
type MockCapture = (String, String);

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line + body. Returns (base_url, join handle delivering the capture).
fn spawn_mock(
    status_line: &'static str,
    body: Value,
) -> (String, std::thread::JoinHandle<MockCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let (request_line, req_body) = read_one_http_request(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        (request_line, req_body)
    });
    (base_url, handle)
}

/// Read one HTTP request (request line + body) off the stream. A POST has a body delimited by
/// Content-Length, so we read the headers then the declared body length.
fn read_one_http_request(stream: &mut std::net::TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    // Read until the header terminator.
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some(hdr_end) = text.find("\r\n\r\n") {
            // Parse Content-Length and read the remainder of the body.
            let header_text = text[..hdr_end].to_string();
            let content_length = header_text
                .lines()
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    if k.trim().eq_ignore_ascii_case("content-length") {
                        v.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            let body_start = hdr_end + 4;
            while buf.len() < body_start + content_length {
                let n = stream.read(&mut tmp).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }
            let full = String::from_utf8_lossy(&buf).to_string();
            let request_line = full.lines().next().unwrap_or("").to_string();
            let body = full.get(body_start..).unwrap_or("").to_string();
            return (request_line, body);
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let request_line = text.lines().next().unwrap_or("").to_string();
    (request_line, String::new())
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

fn dark() -> handshake_native::theme::HsPalette {
    HsTheme::Dark.palette()
}

fn text_range(pane: &str, start: usize, end: usize, text: &str) -> SharedSelection {
    SharedSelection::TextRange {
        pane_id: Arc::from(pane),
        surface: EditorSurfaceKind::RichText,
        start,
        end,
        text: text.to_owned(),
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-001 / AC-001 — build_proposal sets class + full provenance from a TextRange selection.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn build_proposal_full_provenance() {
    // The PT-001 named proof (matches the `cargo test build_proposal` filter): a TextRange selection
    // yields a proposal with the class set and EVERY MemorySourceProvenance field populated.
    let sel = text_range("pane-rich", 12, 32, "the protagonist Aria");
    let p = build_or_panic(&sel, MemoryClass::Semantic, "WS-1", "actor-3");

    assert_eq!(
        p.class,
        MemoryClass::Semantic,
        "AC-001: class set from the argument"
    );
    assert_eq!(p.content, "the protagonist Aria");
    assert_eq!(
        p.source.document_id, "DOC-TEST",
        "AC-001: document_id from the owning pane's authoritative active tab"
    );
    assert_eq!(p.source.pane_id, "pane-rich", "AC-001: pane_id set");
    assert_eq!(p.source.workspace_id, "WS-1", "AC-001: workspace_id set");
    assert_eq!(p.source.selection_start, 12, "AC-001: selection_start set");
    assert_eq!(
        p.source.selection_end, 32,
        "AC-001: selection_end preserves the authoritative UTF-8 byte offset"
    );
    assert_eq!(
        p.source.content_hash.len(),
        64,
        "AC-001/AC-003: content_hash is a 64-char hex"
    );
    assert!(
        p.source
            .content_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "AC-003: content_hash is lowercase hex"
    );
    // AC-003: the hash equals the loom block hash for identical content (reuses the loom primitive).
    let loom = handshake_native::loom_address::ContentHash::of_content_json(&Value::String(
        "the protagonist Aria".to_owned(),
    ));
    assert_eq!(
        p.source.content_hash,
        loom.as_str(),
        "AC-003: proposal content_hash == loom block hash for identical content (no second scheme)"
    );
    assert_eq!(
        content_hash_of_selection("the protagonist Aria"),
        p.source.content_hash
    );
    println!(
        "PT-001 OK: build_proposal class={:?} doc={} range={}..{} hash={}…",
        p.class,
        p.source.document_id,
        p.source.selection_start,
        p.source.selection_end,
        &p.source.content_hash[..8]
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-002 / AC-002 — Procedural review_gated + no auto-commit path.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn procedural_review_gated() {
    let sel = text_range("pane-code", 0, 9, "step one\n");
    let proc = build_or_panic(&sel, MemoryClass::Procedural, "WS-1", "a");
    assert!(
        proc.is_review_gated(),
        "AC-002: a Procedural-class proposal is review-gated"
    );
    assert!(proc.is_review_gated());

    // review_gated is true for EVERY class — the editor never produces a non-review-gated proposal.
    for class in MemoryClass::ORDER {
        let p = build_or_panic(&sel, class, "WS-1", "a");
        assert!(
            p.is_review_gated(),
            "{:?} proposal must be review_gated (never editor-direct commit)",
            class
        );
    }

    // No path flips it false: opening + switching class in the dialog keeps it true.
    let mut dlg = ProposeToMemoryDialog::open_for_document(&sel, "DOC-TEST", "WS-1", "a")
        .expect("opens over a selection");
    dlg.set_class(MemoryClass::Procedural);
    assert!(
        dlg.proposal.is_review_gated(),
        "AC-002: dialog class switch never sets review_gated false"
    );
    println!("PT-002 OK: review_gated==true for all classes incl. Procedural; no auto-commit path");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-004 / AC-005 — the missing-endpoint typed compatibility/failure path.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn missing_endpoint_blocker() {
    // A 404 (route absent) maps to the typed blocker — NOT a panic, NOT a silent no-op, NOT a commit.
    let (base_url, server) = spawn_mock("HTTP/1.1 404 Not Found", json!({"error": "not found"}));
    let client = HandshakeCoreClient::with_base_url(base_url);
    let sel = text_range("pane-rich", 0, 6, "memory");
    let proposal = build_or_panic(&sel, MemoryClass::Episodic, "WS-1", "a");

    let result = rt().block_on(async { submit_proposal(&proposal, &client).await });
    let _ = server.join();

    match result {
        Err(MemoryProposalError::MissingEndpoint { probed_path }) => {
            assert!(
                probed_path.contains("/workspaces/WS-1/memory/proposals"),
                "AC-005: MissingEndpoint must name the probed proposal path; got '{probed_path}'"
            );
            println!("PT-004 typed blocker OK: MissingEndpoint(probed='{probed_path}'), no commit, no fallback");
        }
        other => panic!("AC-005: a 404 must map to MissingEndpoint, got {other:?}"),
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-003 / AC-004 — submit to the proposal endpoint creates a proposal + the FR-EVT-MEM-001 SHAPE.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A capturing FR transport: records the native-editor events the emitter would post (so the test
/// asserts the FR-EVT-MEM-001 payload SHAPE without a live FR ingestion route — the double-gate).
struct CapturingTransport {
    captured: Arc<std::sync::Mutex<Vec<NativeEditorEvent>>>,
}
impl EventLedgerTransport for CapturingTransport {
    fn build_post_body(&self, event: &NativeEditorEvent) -> Value {
        event.to_native_payload()
    }
    fn post(
        &self,
        event: NativeEditorEvent,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(), handshake_native::event_emitter::EmitError>>
                + Send,
        >,
    > {
        let captured = Arc::clone(&self.captured);
        Box::pin(async move {
            captured.lock().unwrap().push(event);
            Ok(())
        })
    }
}

#[test]
fn propose_creates_proposal_via_endpoint() {
    // A mock endpoint isolates the wire shape; the managed-PG suite separately proves durable correlation.
    let runtime = rt();
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 200 OK",
        json!({"proposal_id": "550e8400-e29b-41d4-a716-446655440007", "status": "pending_review", "created_at": "2026-07-17T00:00:00Z", "flight_recorder_event_id": "550e8400-e29b-41d4-a716-446655440001"}),
    );
    let client = HandshakeCoreClient::with_base_url(base_url);

    // A capturing transport proves the frontend does not emit a duplicate event.
    let captured: Arc<std::sync::Mutex<Vec<NativeEditorEvent>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let emitter = NativeEditorEventEmitter::new(
        "WS-1",
        Arc::new(CapturingTransport {
            captured: captured.clone(),
        }),
        Some(runtime.handle().clone()),
    );

    let sel = text_range("pane-rich", 5, 11, "memory");
    let proposal =
        build_proposal_for_document(&sel, MemoryClass::Procedural, "WS-1", "actor-1", "DOC-42")
            .expect("authoritative text document identity builds");

    let ack = runtime
        .block_on(async { submit_proposal_and_emit(&proposal, &client, &emitter).await })
        .expect("AC-004: a 200 from the proposal endpoint yields a ProposalAck");
    assert!(
        matches!(&ack, ProposalSubmitOutcome::EventPersisted { .. }),
        "a correlated success requires the backend-projected FR event UUID in the ack"
    );
    let (req_line, req_body) = server.join().unwrap();

    // The submit is a POST to the documented proposal route (a WRITE — the only write path).
    assert!(
        req_line.starts_with("POST ") && req_line.contains("/workspaces/WS-1/memory/proposals"),
        "AC-004: submit must POST the documented proposal route; got '{req_line}'"
    );
    // The body is the typed, REVIEW-GATED proposal (never a direct commit).
    let body: Value = serde_json::from_str(&req_body).expect("proposal body is JSON");
    assert_eq!(
        body["review_gated"], true,
        "AC-002/AC-004: the submitted proposal is review-gated"
    );
    assert_eq!(body["class"], "procedural");
    assert_eq!(body["content"], "memory");
    assert_eq!(body["source"]["content_hash"], proposal.source.content_hash);
    assert_eq!(body["source"]["document_id"], "DOC-42");
    assert_eq!(ack.proposal_id, "550e8400-e29b-41d4-a716-446655440007");
    assert_eq!(ack.status, "pending_review");

    // The backend acknowledgement binds the transactionally projected canonical event. The
    // frontend must not enqueue the former duplicate native-editor envelope.
    let events = captured.lock().unwrap();
    assert_eq!(
        events.len(),
        0,
        "AC-008: no duplicate frontend FR event is emitted after the durable backend ack"
    );
    assert_eq!(
        match &ack {
            ProposalSubmitOutcome::EventPersisted { event_id, .. } => event_id.as_str(),
            other => panic!("expected canonical persisted outcome, got {other:?}"),
        },
        "550e8400-e29b-41d4-a716-446655440001"
    );
    println!(
        "PT-003 OK (fixture endpoint): proposal POSTed (review_gated), ack={}, canonical FR-EVT-MEM-001 acknowledged.",
        ack.proposal_id
    );
}

#[test]
fn authoritative_text_document_identity_is_required_by_live_dialog_path() {
    let sel = text_range("pane-rich", 0, 6, "memory");
    let err = ProposeToMemoryDialog::open_for_document(&sel, "", "WS-1", "actor-1")
        .expect_err("a text selection without an active document id must fail closed");
    assert_eq!(
        err,
        MemoryProposalError::MissingDocumentIdentity {
            pane_id: "pane-rich".to_owned()
        }
    );

    let dialog =
        ProposeToMemoryDialog::open_for_document(&sel, "DOC-authoritative", "WS-1", "actor-1")
            .expect("active tab document id is accepted");
    assert_eq!(dialog.proposal.source.document_id, "DOC-authoritative");
    assert_ne!(dialog.proposal.source.document_id, "pane-rich");
}

#[test]
fn backend_owned_proposal_event_is_not_rejected_by_frontend_emitter_scope() {
    let runtime = rt();
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 200 OK",
        json!({"proposal_id": "550e8400-e29b-41d4-a716-44665544000a", "status": "pending_review", "created_at": "2026-07-17T00:00:00Z", "flight_recorder_event_id": "550e8400-e29b-41d4-a716-446655440002"}),
    );
    let client = HandshakeCoreClient::with_base_url(base_url);
    let captured = Arc::new(std::sync::Mutex::new(Vec::<NativeEditorEvent>::new()));
    let emitter = NativeEditorEventEmitter::new(
        "WS-B",
        Arc::new(CapturingTransport {
            captured: captured.clone(),
        }),
        Some(runtime.handle().clone()),
    );
    let sel = text_range("pane-rich", 0, 6, "memory");
    let proposal =
        build_proposal_for_document(&sel, MemoryClass::Semantic, "WS-A", "actor-1", "DOC-A")
            .expect("proposal builds");

    let outcome = runtime
        .block_on(async { submit_proposal_and_emit(&proposal, &client, &emitter).await })
        .expect("the proposal POST itself was accepted");
    let (_request, _body) = server.join().expect("proposal POST captured");
    match outcome {
        ProposalSubmitOutcome::EventPersisted { ack, event_id } => {
            assert_eq!(ack.proposal_id, "550e8400-e29b-41d4-a716-44665544000a");
            assert_eq!(event_id, "550e8400-e29b-41d4-a716-446655440002");
        }
        other => panic!(
            "backend-owned event must be durable independently of the emitter, got {other:?}"
        ),
    }
    assert!(
        captured.lock().unwrap().is_empty(),
        "a workspace-mismatched FR event must never reach transport"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-005 / AC-007 — the dialog + class radios + confirm button AccessKit nodes (+ screenshot).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn propose_dialog_accesskit_nodes_present() {
    let sel = text_range("pane-rich", 0, 6, "memory");
    let dialog = ProposeToMemoryDialog::open_for_document(&sel, "DOC-TEST", "WS-1", "actor-1")
        .expect("opens over a selection");

    let mut dialog = dialog;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 320.0))
        .wgpu()
        .build_ui(move |ui| {
            // Match the mounted product path: the dialog renders inside an egui Window, surrounded by
            // the host tree. This catches anonymous widget-id collisions that a direct isolated call to
            // `show` cannot reproduce.
            egui::Window::new("Propose to Memory")
                .id(egui::Id::new("fems-propose-to-memory-dialog-test"))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    let _ = dialog.show(ui, &dark());
                });
        });
    harness.run();

    let first_radio_node_ids = MemoryClass::ORDER.map(|class| {
        let author = fems_class_author_id(class);
        harness
            .root()
            .children_recursive()
            .find_map(|node| {
                let access = node.accesskit_node();
                (access.author_id() == Some(author.as_str())).then_some(access.id().0)
            })
            .unwrap_or_else(|| panic!("first mounted frame is missing '{author}'"))
    });
    let distinct_radio_node_ids = first_radio_node_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        distinct_radio_node_ids.len(),
        MemoryClass::ORDER.len(),
        "each mounted FEMS class radio must own a distinct AccessKit NodeId"
    );

    harness.run();

    let second_radio_node_ids = MemoryClass::ORDER.map(|class| {
        let author = fems_class_author_id(class);
        harness
            .root()
            .children_recursive()
            .find_map(|node| {
                let access = node.accesskit_node();
                (access.author_id() == Some(author.as_str())).then_some(access.id().0)
            })
            .unwrap_or_else(|| panic!("second mounted frame is missing '{author}'"))
    });
    assert_eq!(
        second_radio_node_ids, first_radio_node_ids,
        "mounted FEMS class radio NodeIds must remain stable across frames"
    );

    let root = harness.root();

    // AC-007: the dialog root (Dialog), the three class radios (RadioButton), and confirm (Button).
    let dialog_role = role_of(&root, FEMS_PROPOSE_DIALOG_AUTHOR_ID);
    assert_eq!(
        dialog_role.as_deref(),
        Some("Dialog"),
        "AC-007: '{FEMS_PROPOSE_DIALOG_AUTHOR_ID}' must be Role::Dialog (got {dialog_role:?})"
    );
    for class in MemoryClass::ORDER {
        let author = fems_class_author_id(class);
        let role = role_of(&root, &author);
        assert_eq!(
            role.as_deref(),
            Some("RadioButton"),
            "AC-007: '{author}' must be Role::RadioButton (got {role:?})"
        );
    }
    let confirm_role = role_of(&root, FEMS_PROPOSE_CONFIRM_AUTHOR_ID);
    assert_eq!(
        confirm_role.as_deref(),
        Some("Button"),
        "AC-007: '{FEMS_PROPOSE_CONFIRM_AUTHOR_ID}' must be Role::Button (got {confirm_role:?})"
    );
    let cancel_role = role_of(&root, FEMS_PROPOSE_CANCEL_AUTHOR_ID);
    assert_eq!(
        cancel_role.as_deref(),
        Some("Button"),
        "AC-007: '{FEMS_PROPOSE_CANCEL_AUTHOR_ID}' must be Role::Button (got {cancel_role:?})"
    );
    // RISK-010 / must-fix #5: the dialog author_id appears EXACTLY ONCE in the live tree (a swarm agent
    // gets a single deterministic match — the prior build emitted a SECOND node with the same author_id).
    let dialog_node_count = root
        .children_recursive()
        .filter(|n| n.accesskit_node().author_id() == Some(FEMS_PROPOSE_DIALOG_AUTHOR_ID))
        .count();
    assert_eq!(
        dialog_node_count, 1,
        "RISK-010: exactly ONE node may carry author_id '{FEMS_PROPOSE_DIALOG_AUTHOR_ID}' \
         (got {dialog_node_count}) — a duplicate breaks deterministic swarm addressing"
    );
    println!("PT-005 AccessKit OK: dialog(Dialog, x1) + 3 class radios(RadioButton) + confirm(Button) present");

    // Screenshot to the EXTERNAL root ONLY (best-effort pixel readback).
    if let Ok(image) = harness.render() {
        let ext_dir = external_artifact_dir("wp-kernel-012-mt-064");
        let _ = std::fs::create_dir_all(&ext_dir);
        let ext_path = ext_dir.join("MT-064-propose-to-memory-dialog.png");
        let saved = image.save(&ext_path).is_ok();
        println!(
            "PT-005 screenshot: {}x{} saved_ext={saved} ({})",
            image.width(),
            image.height(),
            ext_path.display()
        );
    } else {
        println!(
            "PT-005 screenshot: GPU readback unavailable on this host (structural proof stands)"
        );
    }

    assert_no_local_artifact_dir();
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// MC-001 / AC-009 — no direct memory-commit call site; the only write path is the proposal POST.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn read_no_direct_commit_site() {
    // Grep the production module for any direct memory-commit/write route. The ONLY write verb that may
    // appear is the proposal POST (`/memory/proposals`); a `/memory/commit` or a direct memory-write
    // route is forbidden (MC-001, AC-002). PostgreSQL/EventLedger is the only durable authority — there
    // is no SQLite anywhere (AC-009).
    let src = std::fs::read_to_string("src/fems/memory_proposal.rs").expect("read module source");

    // The only POST path string is the proposal route.
    assert!(
        src.contains("/workspaces/{workspace_id}/memory/proposals"),
        "the proposal POST route must be present (the only write path)"
    );
    for forbidden in [
        "/memory/commit",
        "/memory/write",
        "memory/direct",
        "rusqlite",
        "sqlite",
    ] {
        assert!(
            !src.to_lowercase().contains(forbidden),
            "MC-001/AC-009: no direct memory-commit/write or SQLite token may appear ('{forbidden}')"
        );
    }
    // The review-gated invariant is exposed by behavior rather than a brittle source spelling: every
    // proposal class reports true and the module contains no false assignment.
    for class in [
        MemoryClass::Semantic,
        MemoryClass::Episodic,
        MemoryClass::Procedural,
    ] {
        let proposal = build_or_panic(
            &text_range("pane-rich", 0, 4, "test"),
            class,
            "WS-1",
            "actor-review-gate",
        );
        assert!(
            proposal.is_review_gated(),
            "MC-002: every proposal class is review-gated"
        );
    }
    assert!(
        !src.contains("review_gated: false") && !src.contains("review_gated = false"),
        "MC-002/AC-002: review_gated is NEVER set false from the editor"
    );
    println!("MC-001/AC-009 OK: only write path is the proposal POST; no direct-commit, no SQLite; review_gated hard-true");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-007 / MC-010 — the dialog id is REGISTERED in the WP-011 AccessKit id registry (must-fix #3/#4).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn propose_dialog_id_is_in_declared_identities() {
    // The dialog ROOT must be enrolled in `accessibility::registry::DECLARED_IDENTITIES` so the
    // registry's compile-time collision/coverage test covers it (MC-010 de-duplication). Before the
    // hardening the id was emitted ad-hoc and was NOT in the registry, so a future collision against
    // `fems-propose-dialog` would have been silent (RISK-010).
    let entry = handshake_native::accessibility::DECLARED_IDENTITIES
        .iter()
        .find(|d| d.author_id == FEMS_PROPOSE_DIALOG_AUTHOR_ID)
        .unwrap_or_else(|| {
            panic!(
                "AC-007/MC-010: '{FEMS_PROPOSE_DIALOG_AUTHOR_ID}' must be registered in \
                 DECLARED_IDENTITIES so the collision/coverage test covers it"
            )
        });
    assert_eq!(
        entry.node_id, FEMS_PROPOSE_DIALOG_NODE_ID,
        "the registry entry's NodeId must match the module's fixed dialog NodeId (single source of truth)"
    );
    println!(
        "AC-007/MC-010 OK: '{FEMS_PROPOSE_DIALOG_AUTHOR_ID}' (NodeId {FEMS_PROPOSE_DIALOG_NODE_ID}) is registry-declared"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-006 — the palette command has a REAL dispatch arm (must-fix #2): the enabled catalog row is not a
// silent no-op. Source-level proof that `dispatch_palette_action` matches the command id (the absence of
// this arm was exactly how the dead-row slipped through) + that it opens the dialog over the selection.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn palette_command_has_real_dispatch_arm() {
    let app_src = std::fs::read_to_string("src/app.rs").expect("read app.rs");
    // The dispatch arm matches the command id (by the const, not a string literal) and opens the dialog.
    assert!(
        app_src.contains("memory_proposal::FEMS_PROPOSE_COMMAND_ID =>"),
        "AC-006/must-fix #2: dispatch_palette_action must have a match arm for the Propose-to-Memory \
         command id (an enabled catalog row with no dispatch arm is a silent no-op)"
    );
    assert!(
        app_src.contains("ProposeToMemoryDialog::open")
            && app_src.contains("register_propose_to_memory_command"),
        "must-fix #2: the dispatch arm opens the dialog over the live selection AND registers the \
         runtime command (wiring register_propose_to_memory_command to a live call site)"
    );
    assert!(
        app_src.contains("self.drive_propose_to_memory(ctx)"),
        "must-fix #2: the open dialog is rendered each frame (a visible result, not a no-op)"
    );
    assert!(
        app_src.contains("submit_proposal_and_emit")
            && app_src.contains("memory_proposal_submit_cell")
            && !app_src.contains("Proposal endpoint not present in this build"),
        "the confirm arm must dispatch the live proposal+FR workflow off-frame and drain its real result"
    );
    let open_request_fn = app_src
        .split("fn open_memory_proposal_request")
        .nth(1)
        .expect("open_memory_proposal_request source")
        .split("fn invalidate_memory_proposal_state_on_workspace_change")
        .next()
        .expect("proposal-open request body");
    assert!(
        open_request_fn.contains("authoritative_selection_source")
            && open_request_fn.contains("request.selection")
            && open_request_fn.contains("request.emitter")
            && open_request_fn.contains("pending_memory_proposal_emitter")
            && open_request_fn.contains("ProposeToMemoryDialog::open_for_document")
            && open_request_fn.contains("ProposeToMemoryDialog::open_for_document_snapshot"),
        "dialog-open must resolve owning-tab provenance, preserve the dispatch-time emitter, and use the snapshot-aware path for code"
    );
    let confirm_fn = app_src
        .split("fn drive_propose_to_memory")
        .nth(1)
        .expect("drive_propose_to_memory source")
        .split("const CANVAS_Z_FRONT")
        .next()
        .expect("proposal driver body");
    assert!(
        confirm_fn.contains("pending_memory_proposal_emitter.take()")
            && !confirm_fn.contains("bus.event_emitter().cloned()"),
        "confirmation must consume the captured dialog-open emitter, never fetch the current workspace emitter"
    );
    assert!(
        confirm_fn.contains("ProposalSubmitOutcome::EventRejected")
            && confirm_fn.contains("correlated FR event was NOT queued"),
        "the UI must disclose proposal-accepted/event-rejected partial success"
    );
    // Sanity: the command id const is the addressable id a swarm agent dispatches.
    assert_eq!(FEMS_PROPOSE_COMMAND_ID, "fems.propose_to_memory");
    println!("AC-006 OK: real dispatch arm opens the dialog + registers the runtime command (no dead row)");
}

// ── kittest AccessKit dump helpers ────────────────────────────────────────────────────────────────

/// The `Role::{Variant}` debug string of the node with `author_id`, or `None` if absent.
fn role_of(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

// ── build helper (keeps the `build_proposal` PT name as a test fn while still exercising the API) ──
mod helpers {
    use super::*;
    pub fn build_or_panic(
        sel: &SharedSelection,
        class: MemoryClass,
        workspace_id: &str,
        actor_id: &str,
    ) -> handshake_native::fems::memory_proposal::MemoryWriteProposal {
        match sel {
            SharedSelection::TextRange { .. } => {
                handshake_native::fems::memory_proposal::build_proposal_for_document(
                    sel,
                    class,
                    workspace_id,
                    actor_id,
                    "DOC-TEST",
                )
                .expect("build_proposal_for_document must succeed")
            }
            _ => handshake_native::fems::memory_proposal::build_proposal(
                sel,
                class,
                workspace_id,
                actor_id,
            )
            .expect("build_proposal must succeed"),
        }
    }
}
use helpers::build_or_panic;
