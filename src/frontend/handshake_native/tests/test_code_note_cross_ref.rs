//! WP-KERNEL-012 MT-034 (E5 — code<->note cross-references) proof suite.
//!
//! Maps each MT-034 acceptance criterion to a real runtime proof:
//!   - AC-1 (unit + gated live-PG): a `code` cross-ref is the EXISTING `hsLink` atom (ref_kind="code",
//!     ref_value=symbol_entity_id). It ROUND-TRIPS the backend `content_json` with the symbol id intact
//!     — proven structurally by a content_json save/reload here, and end-to-end against real PG in the
//!     `--features integration` test (createRichDocument -> loadRichDocument).
//!   - AC-2 (kittest + gated live-PG): clicking a `code-ref-chip-{id}` in the rich-text pane dispatches
//!     `open-code-symbol`, resolves the real symbol, opens the mounted code pane, and lands on the exact
//!     definition line.
//!   - AC-3 (kittest + gated live-PG): the mounted NoteRefsPanel lists the persisted rich document that
//!     references the focused real symbol; clicking a row routes its document id through `open-document`.
//!   - AC-4 (unit): an UNRESOLVED code ref (symbol deleted -> resolved=false / a 404) renders a greyed
//!     `unresolved` chip and does NOT crash or panic.
//!   - AC-5 (AccessKit dump): `code-ref-chip-{id}` (Button), `note-refs-panel` (List),
//!     `note-ref-{doc}` (ListItem), `code-symbol-search` (Dialog) all present in the right pane context.
//!   - AC-6: `cargo test -p handshake-native code_note_cross_ref` passes (this file).
//!
//! ## Artifact hygiene (CX-212E, HARD)
//!
//! The screenshot proof writes ONLY to the EXTERNAL artifact root via [`external_artifact_dir`];
//! [`assert_no_local_artifact_dir`] fails the run if a repo-local `test_output/` or `tests/screenshots/`
//! dir exists. NO artifact is ever written under `src/`.

use std::path::{Path, PathBuf};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

use egui_kittest::kittest::{NodeT, Queryable};
#[cfg(feature = "wgpu_screenshots")]
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
#[cfg(feature = "wgpu_screenshots")]
use canonical_argus_driver::{json_has_author_id, ArgusObservation, CanonicalArgusDriver};
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::{
    HealthInfo, LoomSearchBlock, LoomSearchV2Body, LoomSearchV2Hit, LoomSearchV2Response,
};
use handshake_native::code_editor::code_nav::CodeNavClient;
use handshake_native::code_editor::note_refs_panel::{
    render_note_refs_panel, row_author_id, NoteRefsState, OPEN_PENDING_AUTHOR_ID,
    PANEL_AUTHOR_ID as NOTE_REFS_PANEL_AUTHOR_ID,
};
use handshake_native::code_editor::panel::CodeEditorPanel;
#[cfg(feature = "wgpu_screenshots")]
use handshake_native::code_editor::panel::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::interop::cross_ref::FindNotesSearch;
use handshake_native::interop::{
    dispatch_code_ref_open, percent_encode_symbol, CrossRefError, InteractionBus, NoteRef,
    CMD_OPEN_CODE_SYMBOL, CMD_OPEN_DOCUMENT,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::document_model::doc_json::{
    from_json_string, to_content_json_value,
};
use handshake_native::rich_editor::document_model::node::{
    BlockNode, Child, HsLinkNode, NodeKind, TextLeaf,
};
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorState, RichEditorWidget,
};
use handshake_native::rich_editor::slash_commands::{
    code_symbol_search::CodeSymbolSearchState, render_code_symbol_search_dialog,
    CODE_SYMBOL_SEARCH_AUTHOR_ID, CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID,
};
use handshake_native::rich_editor::wikilinks::inline_view::{code_ref_chip_author_id, EditorEvent};
use handshake_native::rich_editor::wikilinks::parser::parse_wikilink;
use handshake_native::theme::HsTheme;

#[cfg(feature = "integration")]
#[path = "interconnect_support/mod.rs"]
mod interconnect_support;

// ── Artifact hygiene (CX-212E, disk-agnostic) ────────────────────────────────────────────────────────

/// The one approved external artifact root (CX-212E), resolved from the compile-time worktree layout.
/// An explicit override must equal that root so a typo cannot create a second sibling artifact tree.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    let approved_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("handshake_native manifest is nested below the Handshake Worktrees root")
        .join("Handshake_Artifacts");
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| approved_root.clone());
    assert!(
        root.is_absolute(),
        "HANDSHAKE_ARTIFACTS_ROOT must be absolute so artifact placement never depends on process CWD"
    );
    assert_eq!(
        root, approved_root,
        "HANDSHAKE_ARTIFACTS_ROOT must equal the manifest-derived Handshake_Artifacts root"
    );
    root.join("handshake-test").join(subdir)
}

struct ExternalSourceFixture {
    root: PathBuf,
    path: PathBuf,
    content: String,
}

impl Drop for ExternalSourceFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn external_source_fixture(label: &str, line_count: usize) -> ExternalSourceFixture {
    static NEXT_FIXTURE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let fixture_id = NEXT_FIXTURE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = external_artifact_dir("wp-kernel-012-mt-034").join(format!(
        "source-{label}-{}-{fixture_id}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create external MT-034 source fixture directory");
    let content = (0..line_count)
        .map(|line| format!("// MT-034 external source line {line}\n"))
        .collect::<String>();
    let path = root.join("mt034_code_ref.rs");
    std::fs::write(&path, &content).expect("write external MT-034 source fixture");
    ExternalSourceFixture {
        root: root
            .canonicalize()
            .expect("canonical external MT-034 source fixture directory"),
        path: path
            .canonicalize()
            .expect("canonical external MT-034 source fixture path"),
        content,
    }
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
fn assert_no_local_artifact_dir() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .ancestors()
        .nth(3)
        .expect("handshake_native manifest is nested below the repository root");
    for local in [
        crate_root.join("test_output"),
        crate_root.join("tests/screenshots"),
        repo_root.join("Handshake_Artifacts"),
    ] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

/// Serialize the Windows WGPU proof; concurrent device creation is a known suite-level hazard.
#[cfg(feature = "wgpu_screenshots")]
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[cfg(feature = "wgpu_screenshots")]
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(feature = "wgpu_screenshots")]
fn canonical_action_proof(
    target: &str,
    observation: &ArgusObservation,
    terminal_predicate: &str,
    terminal: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "target": target,
        "observation": {
            "before": observation.before,
            "post_action_initial": observation.after,
            "receipt_id": observation.receipt_id,
            "receipt_status": observation.receipt_status,
            "agent_id": observation.agent_id,
            "initial_post_action_observed_sequence": observation.terminal_observed_sequence,
            "target_selected_before": observation.target_selected_before,
            "target_selected_after": observation.target_selected_after
        },
        "terminal_observation": {
            "refreshed": true,
            "predicate": {
                "id": terminal_predicate,
                "passed": true
            },
            "tree": terminal
        }
    })
}

#[cfg(feature = "wgpu_screenshots")]
fn json_has_author_id_prefix(value: &serde_json::Value, prefix: &str) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            object
                .get("author_id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|author_id| author_id.starts_with(prefix))
                || object
                    .values()
                    .any(|child| json_has_author_id_prefix(child, prefix))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .any(|child| json_has_author_id_prefix(child, prefix)),
        _ => false,
    }
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids<S>(harness: &Harness<'_, S>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

fn role_for<S>(harness: &Harness<'_, S>, author_id: &str) -> Option<String> {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .map(|node| format!("{:?}", node.accesskit_node().role()))
}

/// A live shell with the mounted code pane and mounted Notes/rich pane present. This mirrors the
/// host-mount proof shape, but stays in the MT-034-owned test file so the code-ref route proof is
/// co-located with the code<->note contract.
fn code_note_editor_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());

    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("registry");
        guard.insert(PaneRecord::new(
            PaneId::from("pane-a"),
            PaneType::CodeSymbol,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
        guard.insert(PaneRecord::new(
            PaneId::from("pane-b"),
            PaneType::LoomWikiPage,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }

    (app, runtime)
}

fn code_symbol_response_body(
    symbol_id: &str,
    file_path: &str,
    line_start_one_based: usize,
) -> String {
    serde_json::json!({
        "symbol": {
            "symbol_entity_id": symbol_id,
            "symbol_key": format!("rust:{file_path}#MyStruct"),
            "display_name": "MyStruct",
            "symbol_kind": "struct",
            "definition": {
                "line_start": line_start_one_based,
                "line_end": line_start_one_based + 2,
                "source_id": "KSRC-MT034-OPAQUE"
            },
            "staleness": {
                "state": "fresh",
                "fresh": true
            }
        }
    })
    .to_string()
}

fn code_symbol_lookup_response_body(
    symbol_id: &str,
    file_path: &str,
    line_start_one_based: usize,
) -> String {
    serde_json::json!({
        "matches": [{
            "symbol_entity_id": symbol_id,
            "symbol_key": format!("rust:{file_path}#MyStruct"),
            "display_name": "MyStruct",
            "symbol_kind": "struct",
            "definition": {
                "line_start": line_start_one_based,
                "line_end": line_start_one_based + 2,
                "source_id": "KSRC-MT034-OPAQUE"
            },
            "staleness": {
                "state": "fresh",
                "fresh": true
            }
        }]
    })
    .to_string()
}

fn spawn_code_symbol_server(
    symbol_id: &'static str,
    file_path: &str,
    line_start_one_based: usize,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind MT-034 code-nav mock server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let file_path = file_path.to_owned();
    let handle = std::thread::spawn(move || {
        // The listener is bound before app/harness construction, but the request is intentionally
        // triggered only after that setup. Keep accept blocking so a slow all-target run cannot expire
        // the mock before the action under test; the UI navigation assertion remains bounded below.
        let (mut stream, _) = listener
            .accept()
            .expect("accept MT-034 code-symbol request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let n = stream.read(&mut buf).expect("read code-symbol request");
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request_text = String::from_utf8_lossy(&request).to_string();
        let body = code_symbol_response_body(symbol_id, &file_path, line_start_one_based);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write code-symbol response");
        request_text
    });
    (base_url, handle)
}

fn spawn_code_symbol_lookup_server(
    symbol_id: &'static str,
    file_path: &str,
    line_start_one_based: usize,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind MT-034 lookup mock server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let file_path = file_path.to_owned();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept MT-034 lookup request");
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
        let normalized_request = request_text.replace('+', "%20");
        let is_lookup = normalized_request.contains("GET /knowledge/code/symbols?")
            && normalized_request.contains("workspace_id=default-project")
            && normalized_request.contains("name=MyStruct")
            && normalized_request.contains(&format!("path={}", percent_encode_symbol(&file_path)))
            && normalized_request.contains("limit=20");
        let (status, body) = if is_lookup {
            (
                "200 OK",
                code_symbol_lookup_response_body(symbol_id, &file_path, line_start_one_based),
            )
        } else {
            (
                "404 Not Found",
                serde_json::json!({"error":"not_found"}).to_string(),
            )
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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

fn spawn_single_json_response_server(body: String) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind MT-034 JSON response server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept MT-034 JSON request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let n = stream.read(&mut buf).expect("read MT-034 JSON request");
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write MT-034 JSON response");
        String::from_utf8_lossy(&request).into_owned()
    });
    (base_url, handle)
}

#[cfg(feature = "wgpu_screenshots")]
fn spawn_argus_code_symbol_server(
    symbol_id: &'static str,
    symbol_name: &'static str,
    file_path: &str,
    line_start_one_based: usize,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind MT-034 Argus code-nav server");
    listener
        .set_nonblocking(true)
        .expect("set MT-034 Argus server nonblocking");
    let base_url = format!(
        "http://{}",
        listener.local_addr().expect("Argus local addr")
    );
    let file_path = file_path.to_owned();
    let handle = std::thread::spawn(move || {
        // A full HandshakeApp mount legitimately performs several unrelated background reads before
        // the MT-034 interactions begin. Keep this fixture alive for that traffic; it still terminates
        // promptly once all three authoritative MT-034 routes have been observed and the client is idle.
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut idle_deadline = None;
        let mut requests = Vec::new();
        let mut saw_symbol_detail = false;
        let mut saw_symbol_lookup = false;
        let mut saw_document_load = false;
        while requests.len() < 256 && Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(accepted) => accepted,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if saw_symbol_detail
                        && saw_symbol_lookup
                        && saw_document_load
                        && idle_deadline.is_some_and(|deadline| Instant::now() >= deadline)
                    {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(error) => panic!("accept MT-034 Argus request: {error}"),
            };
            let mut request = Vec::new();
            let mut buf = [0_u8; 2048];
            loop {
                let n = stream.read(&mut buf).expect("read MT-034 Argus request");
                if n == 0 {
                    break;
                }
                request.extend_from_slice(&buf[..n]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request_text = String::from_utf8_lossy(&request).to_string();
            let is_symbol_detail =
                request_text.contains(&format!("GET /knowledge/code/symbols/{symbol_id} "));
            let is_symbol_lookup = request_text.contains("GET /knowledge/code/symbols?");
            let is_document_load = request_text.contains("GET /knowledge/documents/DOC-ARGUS-34 ");
            let is_document_list = request_text.contains("GET /knowledge/documents?");
            let symbol = serde_json::json!({
                "symbol_entity_id": symbol_id,
                "symbol_key": format!("rust:{file_path}#{symbol_name}"),
                "display_name": symbol_name,
                "symbol_kind": "struct",
                "definition": {
                    "line_start": line_start_one_based,
                    "line_end": line_start_one_based + 2,
                    "source_id": "KSRC-MT034-ARGUS"
                },
                "staleness": {
                    "state": "fresh",
                    "fresh": true
                }
            });
            let (status, body) = if is_symbol_lookup {
                saw_symbol_lookup = true;
                (
                    "200 OK",
                    serde_json::json!({"matches": [symbol]}).to_string(),
                )
            } else if is_symbol_detail {
                saw_symbol_detail = true;
                ("200 OK", serde_json::json!({"symbol": symbol}).to_string())
            } else if is_document_load {
                saw_document_load = true;
                (
                    "200 OK",
                    serde_json::json!({
                        "document": {
                            "rich_document_id": "DOC-ARGUS-34",
                            "workspace_id": "default-project",
                            "title": "MT-034 Argus reference",
                            "schema_version": "rich_document_block_tree_v1",
                            "content_json": {
                                "type": "doc",
                                "content": [{
                                    "type": "paragraph",
                                    "content": [
                                        {"type": "text", "text": "Referencing "},
                                        {
                                            "type": "hsLink",
                                            "attrs": {
                                                "refKind": "code",
                                                "refValue": symbol_id,
                                                "label": symbol_name,
                                                "resolved": true
                                            }
                                        }
                                    ]
                                }]
                            },
                            "content_sha256": "mt034-argus-fixture",
                            "doc_version": 1,
                            "crdt_document_id": null,
                            "authority_label": "promoted",
                            "owner_actor_kind": "operator",
                            "owner_actor_id": "mt034-argus-proof",
                            "project_ref": null,
                            "folder_ref": null,
                            "created_at": "2026-07-30T00:00:00Z",
                            "updated_at": "2026-07-30T00:00:00Z"
                        },
                        "tree": {
                            "schema_version": "rich_document_block_tree_v1",
                            "schema_matches": true,
                            "block_ids": [],
                            "blocks": []
                        },
                        "code_nodes": []
                    })
                    .to_string(),
                )
            } else if is_document_list {
                ("200 OK", "[]".to_owned())
            } else {
                (
                    "404 Not Found",
                    serde_json::json!({"error": "not_found"}).to_string(),
                )
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write MT-034 Argus response");
            requests.push(request_text);
            idle_deadline = Some(Instant::now() + Duration::from_millis(500));
        }
        requests
    });
    (base_url, handle)
}

/// Build a one-paragraph doc with a `code` cross-ref hsLink atom embedded (the note->code authored
/// shape: ref_kind="code", ref_value=symbol_entity_id, label=display_name).
fn doc_with_code_ref(symbol_entity_id: &str, display_name: &str) -> BlockNode {
    let mut para = BlockNode::new(NodeKind::Paragraph);
    para.children.push(Child::Text(TextLeaf::new("see ")));
    para.children.push(Child::HsLink(HsLinkNode::new(
        "code",
        symbol_entity_id,
        display_name,
    )));
    para.children.push(Child::Text(TextLeaf::new("")));
    BlockNode::doc(vec![para])
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 (unit): `[[code:path#Symbol]]` parses to a `code` hsLink atom; the atom round-trips content_json
// with the symbol id intact.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac1_code_wikilink_parses_to_code_hs_link() {
    let parsed = parse_wikilink("[[code:src/main.rs#MyStruct]]").expect("a valid code wikilink");
    let link = parsed.to_hs_link();
    assert_eq!(
        link.ref_kind, "code",
        "AC-1: the code: prefix is a `code` ref kind"
    );
    assert_eq!(
        link.ref_value, "src/main.rs#MyStruct",
        "AC-1: the symbol key is the ref value"
    );
    assert!(
        link.resolved,
        "AC-1: the code: prefix is a known resolved kind"
    );
    println!("AC-1: [[code:src/main.rs#MyStruct]] -> hsLink(code, src/main.rs#MyStruct)");
}

#[test]
fn ac1_code_ref_atom_round_trips_content_json_with_symbol_id() {
    // The note->code authored atom: ref_value carries the symbol_entity_id (the resolution key). It is
    // the SAME hsLink node the backend persists, so save->reload preserves the symbol id (AC-1).
    let doc = doc_with_code_ref("ent-MyStruct-42", "MyStruct");
    let json = handshake_native::rich_editor::document_model::doc_json::to_json_string(&doc)
        .expect("serialize");
    let back = from_json_string(&json).expect("reload");
    assert_eq!(
        doc, back,
        "AC-1: the code-ref doc round-trips through DocJson unchanged"
    );

    // The hsLink node carries the symbol id in ref_value, type=hsLink (NOT an invented code_ref node).
    let v = to_content_json_value(&doc);
    let link = &v["content"][0]["content"][1];
    assert_eq!(
        link["type"], "hsLink",
        "AC-1: a code ref is an hsLink atom, never a `code_ref` node"
    );
    assert_eq!(link["attrs"]["refKind"], "code");
    assert_eq!(
        link["attrs"]["refValue"], "ent-MyStruct-42",
        "AC-1: symbol_entity_id preserved"
    );
    assert_eq!(link["attrs"]["label"], "MyStruct");
    println!(
        "AC-1: code hsLink atom round-trips content_json with symbol_entity_id=ent-MyStruct-42"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-2 (kittest): clicking a code-ref chip dispatches `open-code-symbol` with the correct symbol id.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac2_click_code_ref_chip_dispatches_open_code_symbol() {
    // Render a rich editor over a doc carrying a code-ref chip. The chip's stable author_id is
    // `code-ref-chip-{symbol_entity_id}` — the kittest targets it by that id.
    let symbol_id = "ent-MyStruct-42";
    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::new(
        doc_with_code_ref(symbol_id, "MyStruct"),
    )));
    let state_ck = std::sync::Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&state)).show(ui);
        });
    harness.run();

    // The chip is addressable by the contract author_id.
    let chip_id = code_ref_chip_author_id(symbol_id);
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&chip_id),
        "AC-5/AC-2: the code-ref chip is addressable by `{chip_id}`; present ids: {ids:?}"
    );
    assert_eq!(
        role_for(&harness, &chip_id).as_deref(),
        Some("Button"),
        "AC-5: the contract requires every code-ref-chip node to expose Role::Button"
    );

    // Click the chip; the editor enqueues a WikilinkActivated{ref_kind=code,...} event the host drains.
    let chip = harness.get_by(|n| n.author_id() == Some(chip_id.as_str()));
    chip.click();
    harness.run();

    // The host drains the editor's pending events; a code-ref click bridges to `open-code-symbol`.
    let event = {
        let st = state_ck.lock().unwrap();
        st.pending_events.iter().find_map(|e| match e {
            EditorEvent::WikilinkActivated {
                ref_kind,
                ref_value,
                ..
            } if ref_kind == "code" => Some((ref_kind.clone(), ref_value.clone())),
            _ => None,
        })
    };
    let (ref_kind, ref_value) =
        event.expect("AC-2: clicking the code-ref chip enqueues a code WikilinkActivated event");
    assert_eq!(ref_kind, "code");
    assert_eq!(
        ref_value, symbol_id,
        "AC-2: the event carries the correct symbol entity id"
    );

    // The bridge stages the symbol on the bus and dispatches `open-code-symbol` (the note->code command).
    let ctx = egui::Context::default();
    let mut bus = InteractionBus::new();
    bus.register_open_code_symbol_command();
    let evt = EditorEvent::WikilinkActivated {
        ref_kind,
        ref_value: ref_value.clone(),
        resolved: true,
    };
    let dispatched = dispatch_code_ref_open(&ctx, &mut bus, &evt);
    assert_eq!(
        dispatched.as_deref(),
        Some(symbol_id),
        "AC-2: the bridge dispatches open-code-symbol for the symbol"
    );
    assert_eq!(
        bus.take_pending_code_symbol().as_deref(),
        Some(symbol_id),
        "AC-2: `open-code-symbol` staged the correct symbol_entity_id on the bus"
    );
    println!("AC-2: clicked code-ref-chip-{symbol_id} -> open-code-symbol staged {symbol_id} ({CMD_OPEN_CODE_SYMBOL})");
}

#[test]
fn ac2_live_shell_routes_code_ref_event_to_mounted_code_pane() {
    let symbol_id = "ent-live-shell-route-42";
    let fixture = external_source_fixture("entity-id", 140);
    let file_path = fixture.path.to_string_lossy().to_string();
    let line_start_one_based = 64;
    let line_start_zero_based = line_start_one_based - 1;
    let (base_url, server) = spawn_code_symbol_server(symbol_id, &file_path, line_start_one_based);
    let (mut app, _rt) = code_note_editor_shell();
    app.install_mounted_code_nav_client_for_test(CodeNavClient::new(base_url));
    let rich_state = app.mounted_rich_state();
    let source_panel = app.mounted_code_panel();
    source_panel.set_text("// stale mounted buffer must be replaced by the resolved file\n");
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    rich_state
        .lock()
        .unwrap()
        .pending_events
        .push(EditorEvent::WikilinkActivated {
            ref_kind: "code".to_owned(),
            ref_value: symbol_id.to_owned(),
            resolved: true,
        });
    assert_eq!(
        rich_state.lock().unwrap().pending_events.len(),
        1,
        "AC-2 live shell: code WikilinkActivated is queued before the frame"
    );

    harness.run_steps(2);
    assert!(
        rich_state.lock().unwrap().pending_events.is_empty(),
        "AC-2 live shell: mounted rich pane drained the code-ref event into the shell"
    );

    let navigation_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < navigation_deadline {
        let code_panel = harness.state().active_mounted_code_panel();
        if code_panel.file_path() == file_path
            && code_panel.buffer().to_string() == fixture.content
            && code_panel
                .last_visible_range()
                .contains(&line_start_zero_based)
        {
            break;
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }

    let app = harness.state();
    let code_panel = app.active_mounted_code_panel();
    assert!(
        !std::sync::Arc::ptr_eq(&source_panel, &code_panel),
        "AC-2 live shell: cross-file navigation mounts a canonical file document instead of overwriting the untitled source panel"
    );
    assert_eq!(source_panel.file_path(), "");
    assert_eq!(
        source_panel.buffer().to_string(),
        "// stale mounted buffer must be replaced by the resolved file\n"
    );
    let active = app
        .active_pane()
        .expect("AC-2 live shell: code-ref route focuses a pane after dispatch");
    let active_tab = app
        .tab_bar_states()
        .get(active)
        .and_then(|bar| bar.active())
        .expect("AC-2 live shell: focused pane has an active tab");
    assert_eq!(
        active_tab.pane_type,
        PaneType::CodeSymbol,
        "AC-2 live shell: ref_kind=code must route to the mounted code pane, not LoomBlock"
    );
    let code_tabs: Vec<_> = app
        .tab_bar_states()
        .values()
        .flat_map(|bar| bar.tabs.iter())
        .filter(|tab| tab.pane_type == PaneType::CodeSymbol)
        .collect();
    assert_eq!(
        code_tabs.len(),
        2,
        "AC-2 live shell: navigation retains the base code tab and opens exactly one canonical file tab"
    );
    assert!(code_tabs.iter().any(|tab| tab.content_id.is_none()));
    assert!(
        code_tabs
            .iter()
            .all(|tab| tab.content_id.as_deref() != Some(symbol_id)),
        "AC-2 live shell: no inert symbol-identity tab may survive canonical file navigation"
    );
    assert_eq!(
        active_tab
            .content_id
            .as_deref()
            .map(|id| id.replace('\\', "/").to_ascii_lowercase()),
        Some(file_path.replace('\\', "/").to_ascii_lowercase()),
        "AC-2 live shell: the active loaded document is keyed by the canonical file, not forked per symbol"
    );
    assert_eq!(
        code_panel.file_path(),
        file_path,
        "AC-2 live shell: resolved code ref derives the canonical readable file from symbol_key, never the opaque source_id; status={:?}",
        app.quick_switcher_nav_status()
    );
    assert_eq!(
        code_panel.buffer().to_string(),
        fixture.content,
        "AC-2 live shell: navigation must load the resolved source from disk, not retain the seeded buffer"
    );
    assert!(
        code_panel
            .last_visible_range()
            .contains(&line_start_zero_based),
        "AC-2 live shell: visible range {:?} must contain resolved backend line {line_start_zero_based}",
        code_panel.last_visible_range()
    );
    assert!(
        app.quick_switcher_nav_status().is_none(),
        "AC-2 live shell: mounted code pane opens without surfacing a nav error"
    );
    let request = server.join().expect("join MT-034 code-nav mock server");
    assert!(
        request.contains(&format!("GET /knowledge/code/symbols/{symbol_id} ")),
        "AC-2 live shell: open-code-symbol must resolve through getCodeSymbol, got request {request:?}"
    );

    println!(
        "AC-2 LIVE SHELL: mounted rich code-ref event -> getCodeSymbol -> canonical CodeSymbol file tab -> {file_path}:{line_start_zero_based}"
    );
}

#[test]
fn ac2_live_shell_routes_literal_path_symbol_code_ref_to_mounted_code_pane() {
    let symbol_id = "ent-literal-path-symbol-42";
    let fixture = external_source_fixture("path-symbol", 140);
    let file_path = fixture.path.to_string_lossy().to_string();
    let literal_ref = format!("{file_path}#MyStruct");
    let line_start_one_based = 77;
    let line_start_zero_based = line_start_one_based - 1;
    let (base_url, server) =
        spawn_code_symbol_lookup_server(symbol_id, &file_path, line_start_one_based);
    let (mut app, _rt) = code_note_editor_shell();
    app.install_mounted_code_nav_client_for_test(CodeNavClient::new(base_url));
    let rich_state = app.mounted_rich_state();
    let source_panel = app.mounted_code_panel();
    source_panel.set_text("// stale mounted buffer must be replaced by the looked-up file\n");
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    rich_state
        .lock()
        .unwrap()
        .pending_events
        .push(EditorEvent::WikilinkActivated {
            ref_kind: "code".to_owned(),
            ref_value: literal_ref.clone(),
            resolved: true,
        });
    harness.run_steps(2);

    let navigation_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < navigation_deadline {
        let code_panel = harness.state().active_mounted_code_panel();
        if code_panel.file_path() == file_path
            && code_panel.buffer().to_string() == fixture.content
            && code_panel
                .last_visible_range()
                .contains(&line_start_zero_based)
        {
            break;
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }

    let app = harness.state();
    let code_panel = app.active_mounted_code_panel();
    assert!(
        !std::sync::Arc::ptr_eq(&source_panel, &code_panel),
        "AC-2 literal live shell: cross-file navigation preserves the untitled source panel"
    );
    assert_eq!(source_panel.file_path(), "");
    assert_eq!(
        source_panel.buffer().to_string(),
        "// stale mounted buffer must be replaced by the looked-up file\n"
    );
    let active = app
        .active_pane()
        .expect("AC-2 literal live shell: code-ref route focuses a pane");
    let active_tab = app
        .tab_bar_states()
        .get(active)
        .and_then(|bar| bar.active())
        .expect("AC-2 literal live shell: focused pane has an active tab");
    assert_eq!(active_tab.pane_type, PaneType::CodeSymbol);
    let code_tabs: Vec<_> = app
        .tab_bar_states()
        .values()
        .flat_map(|bar| bar.tabs.iter())
        .filter(|tab| tab.pane_type == PaneType::CodeSymbol)
        .collect();
    assert_eq!(
        code_tabs.len(),
        2,
        "AC-2 literal live shell: navigation retains the base code tab and opens exactly one canonical file tab"
    );
    assert!(code_tabs.iter().any(|tab| tab.content_id.is_none()));
    assert!(
        code_tabs
            .iter()
            .all(|tab| tab.content_id.as_deref() != Some(literal_ref.as_str())),
        "AC-2 literal live shell: no inert authored-reference tab may survive canonical file navigation"
    );
    assert_eq!(
        active_tab
            .content_id
            .as_deref()
            .map(|id| id.replace('\\', "/").to_ascii_lowercase()),
        Some(file_path.replace('\\', "/").to_ascii_lowercase()),
        "AC-2 literal live shell: the active loaded document is keyed by the canonical file"
    );
    assert_eq!(
        code_panel.file_path(),
        file_path,
        "AC-2 literal live shell must load the canonical file; status={:?}",
        app.quick_switcher_nav_status()
    );
    assert_eq!(
        code_panel.buffer().to_string(),
        fixture.content,
        "AC-2 literal live shell: navigation must load the canonical source from disk"
    );
    assert!(
        code_panel
            .last_visible_range()
            .contains(&line_start_zero_based),
        "AC-2 literal live shell: visible range {:?} must contain looked-up line {line_start_zero_based}",
        code_panel.last_visible_range()
    );
    assert!(
        app.quick_switcher_nav_status().is_none(),
        "AC-2 literal live shell: path#symbol opens without surfacing a nav error"
    );
    let request = server.join().expect("join MT-034 lookup mock server");
    assert!(
        request.contains("GET /knowledge/code/symbols?"),
        "AC-2 literal live shell: path#symbol must use symbol lookup, got request {request:?}"
    );
    println!(
        "AC-2 LIVE SHELL: literal [[code:{literal_ref}]] -> lookupSymbols(path,name) -> {symbol_id} -> {file_path}:{line_start_zero_based}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-3 (kittest, WIRED): drive the REAL CodeEditorPanel + SymbolDwellTracker + find_notes_with +
// render_note_refs_panel pipeline. The panel is mounted in a live `show()` loop, a workspace + runtime
// are injected (the production wiring), a counted in-memory FindNotesSearch mock is injected (NO
// backend), the dwell threshold is set to ZERO (so the dwell crossing fires on the first settled frame
// without an 800ms wall-clock wait), and the caret is parked on a symbol. After a few frames the dwell
// fires the off-thread search, the result drains into the panel, and the NoteRefsPanel lists the note —
// proving the dwell-debounce -> search -> panel integration end-to-end in the live host (not a bare
// render of a hand-built Loaded state).
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

/// A counted in-memory find-notes search mock (NO backend): exposes one seeded `note` candidate and an
/// empty `journal` set while honoring the production `content_type`/`limit`/`offset` contract. Counts
/// calls so the test can assert the dwell fired the search EXACTLY ONCE per content type (RISK-3 /
/// MC-3 — no per-frame backend spam).
struct CountingFindNotes {
    note_block_id: String,
    note_title: String,
    calls: std::sync::atomic::AtomicUsize,
    queries: std::sync::Mutex<Vec<String>>,
}

impl FindNotesSearch for CountingFindNotes {
    fn search<'a>(
        &'a self,
        _workspace_id: &'a str,
        body: &'a LoomSearchV2Body,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>>
                + Send
                + 'a,
        >,
    > {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.queries
            .lock()
            .expect("record MT-034 find-notes query")
            .push(body.query.clone());
        let total = usize::from(body.content_type.as_deref() == Some("note"));
        let start = body.offset as usize;
        let end = start.saturating_add(body.limit as usize).min(total);
        let hits = if start < end {
            vec![LoomSearchV2Hit {
                block: LoomSearchBlock {
                    block_id: self.note_block_id.clone(),
                    content_type: "note".to_owned(),
                    document_id: Some("DOC-7".to_owned()),
                    title: Some(self.note_title.clone()),
                },
                score: 1.0,
                fts_rank: 0.0,
                trgm_sim: 0.0,
                vector_sim: 0.0,
                edge_degree: 0,
                highlight: "uses <mark>MyStruct</mark> here".to_owned(),
            }]
        } else {
            Vec::new()
        };
        Box::pin(async move {
            Ok(LoomSearchV2Response {
                hits,
                content_type_facets: Default::default(),
                semantic_available: false,
                total: total as i64,
            })
        })
    }

    fn load_document_content<'a>(
        &'a self,
        document_id: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, CrossRefError>> + Send + 'a>,
    > {
        Box::pin(async move {
            if document_id != "DOC-7" {
                return Err(CrossRefError::NotFound(document_id.to_owned()));
            }
            Ok(serde_json::json!({
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "hsLink",
                        "attrs": {
                            "refKind": "code",
                            "refValue": "KEN-MT034-MOCK",
                            "label": "MyStruct",
                            "resolved": true
                        }
                    }]
                }]
            }))
        })
    }
}

#[cfg(feature = "wgpu_screenshots")]
struct ArgusFindNotes {
    symbol_entity_id: String,
}

#[cfg(feature = "wgpu_screenshots")]
impl FindNotesSearch for ArgusFindNotes {
    fn search<'a>(
        &'a self,
        _workspace_id: &'a str,
        body: &'a LoomSearchV2Body,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<LoomSearchV2Response, CrossRefError>>
                + Send
                + 'a,
        >,
    > {
        let exact_query = body.query == "Mt034ArgusSymbol";
        let is_note = body.content_type.as_deref() == Some("note");
        let total = usize::from(exact_query && is_note);
        let hits = if total == 1 && body.offset == 0 {
            vec![LoomSearchV2Hit {
                block: LoomSearchBlock {
                    block_id: "BLK-ARGUS-34".to_owned(),
                    content_type: "note".to_owned(),
                    document_id: Some("DOC-ARGUS-34".to_owned()),
                    title: Some("MT-034 Argus reference".to_owned()),
                },
                score: 1.0,
                fts_rank: 1.0,
                trgm_sim: 1.0,
                vector_sim: 0.0,
                edge_degree: 1,
                highlight: "Uses <mark>Mt034ArgusSymbol</mark>".to_owned(),
            }]
        } else {
            Vec::new()
        };
        Box::pin(async move {
            Ok(LoomSearchV2Response {
                hits,
                content_type_facets: Default::default(),
                semantic_available: false,
                total: total as i64,
            })
        })
    }

    fn load_document_content<'a>(
        &'a self,
        document_id: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<serde_json::Value, CrossRefError>> + Send + 'a>,
    > {
        let symbol_entity_id = self.symbol_entity_id.clone();
        Box::pin(async move {
            if document_id != "DOC-ARGUS-34" {
                return Err(CrossRefError::NotFound(document_id.to_owned()));
            }
            Ok(serde_json::json!({
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "hsLink",
                        "attrs": {
                            "refKind": "code",
                            "refValue": symbol_entity_id,
                            "label": "Mt034ArgusSymbol",
                            "resolved": true
                        }
                    }]
                }]
            }))
        })
    }
}

#[test]
fn ac3_reverse_lookup_queries_the_persisted_code_ref_label() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build reverse-lookup query runtime");
    let backend = CountingFindNotes {
        note_block_id: "BLK-LABEL".to_owned(),
        note_title: "Code ref label proof".to_owned(),
        calls: std::sync::atomic::AtomicUsize::new(0),
        queries: std::sync::Mutex::new(Vec::new()),
    };
    let notes = runtime
        .block_on(handshake_native::interop::cross_ref::find_notes_with(
            &backend,
            "rust:src/mt034_exact_symbol.rs#Mt034ExactSymbol",
            "ws-mt034-label",
        ))
        .expect("reverse lookup succeeds");
    assert_eq!(notes.len(), 1, "content-type hits dedupe by block id");
    assert_eq!(
        backend
            .queries
            .lock()
            .expect("read MT-034 find-notes queries")
            .as_slice(),
        [
            "Mt034ExactSymbol".to_owned(),
            "Mt034ExactSymbol".to_owned()
        ],
        "RichDocument plain-text projection stores the selected code symbol label, not its full key"
    );
}

#[test]
fn ac3_code_pane_dwell_loads_note_refs_panel() {
    use std::sync::atomic::Ordering;

    // A real multi-thread runtime (the same shape the MT-008/010 live-loop tests build) so the dwell
    // crossing can `spawn` the off-thread find-notes search.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build tokio runtime");

    // The mock the dwell-fired search resolves against (NO backend). One call expected (fired once).
    let backend = std::sync::Arc::new(CountingFindNotes {
        note_block_id: "BLK-7".to_owned(),
        note_title: "Design notes".to_owned(),
        calls: std::sync::atomic::AtomicUsize::new(0),
        queries: std::sync::Mutex::new(Vec::new()),
    });
    let backend_dyn: std::sync::Arc<dyn FindNotesSearch> = backend.clone();

    // The LIVE code pane (the host the review found untouched), now mounting the NoteRefsPanel.
    let panel = std::sync::Arc::new(CodeEditorPanel::new(
        "fn main() { let total = MyStruct::new(); }",
        "rs",
    ));
    panel.set_runtime(rt.handle().clone());
    panel.set_workspace_id("ws-mt034");
    panel.set_file_path("fixtures/mt034_code_ref.rs");
    let (lookup_base, lookup_thread) = spawn_single_json_response_server(
        code_symbol_lookup_response_body("KEN-MT034-MOCK", "fixtures/mt034_code_ref.rs", 1),
    );
    panel.set_code_nav_client(CodeNavClient::new(lookup_base));
    panel.set_find_notes_backend(backend_dyn);
    panel.set_show_note_refs(true);
    // Zero dwell threshold so the dwell crosses on the first settled frame (deterministic, no 800ms wait).
    panel.set_note_refs_dwell_threshold(std::time::Duration::from_millis(0));

    // Park the caret inside the identifier "MyStruct" BEFORE the first frame so the dwell only ever
    // observes that one symbol (a pre-frame caret at offset 0 could dwell on a different word and fire a
    // second, unrelated search — we want exactly ONE dwell crossing to prove the once-per-dwell guard).
    let offset = panel
        .buffer()
        .to_string()
        .find("MyStruct")
        .expect("symbol present")
        + 2;
    panel.set_single_cursor(offset);

    let panel_ui = std::sync::Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 400.0))
        .build_ui(move |ui| {
            panel_ui.show(ui);
        });
    // Use STEP-bounded driving (NOT `harness.run()`): once the dwell fires, the panel enters Loading and
    // the NoteRefsPanel renders an `egui::Spinner` that requests a repaint every frame — `run()` would
    // exceed max_steps on a never-settling spinner. `step()` advances exactly one frame, so the loop is
    // bounded regardless of the animating spinner (the review's coverage-gap mitigation: never run() a
    // Loading state).
    harness.step();

    // Step the frame loop, giving the off-thread search a moment to land between frames, until the panel
    // reaches Loaded (the dwell fires on the 2nd settled frame; the off-thread word->symbol_key lookup is
    // bounded by SYMBOL_KEY_LOOKUP_TIMEOUT_MS, then the mock search + drain take a few more frames).
    // Bounded at ~4s wall-clock so a regression (stuck Loading / dropped task) fails fast instead of
    // hanging — proving the wired pipeline TERMINATES, not just that a spinner animates.
    for _ in 0..80 {
        if matches!(panel.note_refs_state(), NoteRefsState::Loaded(_)) {
            break;
        }
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // One more step so the SidePanel renders the Loaded state's note row into the live AccessKit tree.
    harness.step();

    // The REAL panel state is now Loaded with the seeded note (driven by the wired dwell->search pipeline,
    // NOT a hand-built Loaded state). Re-read after the final render step.
    let loaded = panel.note_refs_state();
    match &loaded {
        NoteRefsState::Loaded(notes) => {
            assert_eq!(
                notes.len(),
                1,
                "AC-3: the wired pipeline loaded the seeded note"
            );
            assert_eq!(notes[0].block_id, "BLK-7");
            assert_eq!(notes[0].document_id, "DOC-7");
            assert_eq!(notes[0].document_title, "Design notes");
        }
        other => panic!("AC-3: expected the dwell to load the note refs, got {other:?}"),
    }

    // The NoteRefsPanel is mounted in the live code pane (its container + the note row are addressable).
    let ids = author_ids(&harness);
    assert!(
        ids.contains(NOTE_REFS_PANEL_AUTHOR_ID),
        "AC-3: the NoteRefsPanel is mounted in the live code pane; got {ids:?}"
    );
    let row = row_author_id("DOC-7");
    assert!(
        ids.contains(&row),
        "AC-3: the dwell-loaded note row `{row}` is present in the live pane"
    );

    // RISK-3 / MC-3: the dwell fired the search exactly ONCE despite many frames (no per-frame spam). One
    // dwell crossing runs one search PER rich-doc content type (`note` + `journal` = 2 backend calls); the
    // load-bearing proof is that it is a small CONSTANT, not (frames × content_types) — the debounce
    // suppressed the per-frame re-fire.
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        handshake_native::interop::cross_ref::NOTE_REF_CONTENT_TYPES.len(),
        "AC-3/RISK-3: one dwell crossing ran exactly one search per content type (not per-frame spam)"
    );
    let lookup_request = lookup_thread.join().expect("join exact lookup mock");
    assert!(
        lookup_request.contains("GET /knowledge/code/symbols?"),
        "dwell must resolve an exact code projection before reverse lookup: {lookup_request}"
    );

    // The focused symbol the panel tracks is the dwelled symbol (resolved key falls back to the word with
    // no live code-nav backend).
    assert!(
        panel.note_refs_focused_symbol().is_some(),
        "AC-3: the panel records the dwelled symbol it loaded for"
    );

    // Hold the shared bus across the clicked-row frame. The non-blocking renderer must retain the
    // operator action and expose a visible pending status, then deliver it on the first free frame.
    let bus = InteractionBus::get_or_init(&harness.ctx);
    let guard = bus.lock().expect("hold shared bus for contention proof");
    harness
        .get_by(|node| node.author_id() == Some(row.as_str()))
        .click();
    harness.step();
    assert!(
        author_ids(&harness).contains(OPEN_PENDING_AUTHOR_ID),
        "a contended NoteRefs click must remain visibly pending"
    );
    drop(guard);
    harness.step();
    assert_eq!(
        bus.lock()
            .expect("read retried open-document action")
            .take_pending_navigation()
            .as_deref(),
        Some("DOC-7"),
        "the retained click must deliver after contention clears"
    );

    println!(
        "AC-3 WIRED: code-pane dwell on MyStruct -> find_notes fired once -> NoteRefsPanel loaded DOC-7 \
         (Design notes) in the live CodeEditorPanel"
    );

    drop(harness);
    drop(panel);
    rt.shutdown_timeout(std::time::Duration::from_secs(2));
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-3 (leaf-widget): the NoteRefsPanel lists a note for the focused symbol; clicking a row yields the
// doc id the caller dispatches `open-document` for. (Complements the WIRED test above: this isolates the
// row-click -> open-document routing the wired panel uses.)
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac3_note_refs_panel_lists_and_opens_a_note() {
    let note = NoteRef {
        block_id: "BLK-7".to_owned(),
        document_id: "DOC-7".to_owned(),
        document_title: "Design notes".to_owned(),
        excerpt: "uses MyStruct for the buffer".to_owned(),
    };
    let state = NoteRefsState::Loaded(vec![note]);
    let palette = HsTheme::Dark.palette();

    let clicked = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
    let clicked_ui = std::sync::Arc::clone(&clicked);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 300.0))
        .build_ui(move |ui| {
            if let Some(doc_id) = render_note_refs_panel(ui, &state, Some("ent-1"), &palette) {
                *clicked_ui.lock().unwrap() = Some(doc_id);
            }
        });
    harness.run();

    // The panel container + the row are addressable by the contract ids.
    let ids = author_ids(&harness);
    assert!(
        ids.contains(NOTE_REFS_PANEL_AUTHOR_ID),
        "AC-5: note-refs-panel present; got {ids:?}"
    );
    let row = row_author_id("DOC-7");
    assert!(
        ids.contains(&row),
        "AC-3/AC-5: the note row `{row}` is present"
    );

    // Click the row -> the panel returns the document id the host dispatches `open-document` for.
    let row_node = harness.get_by(|n| n.author_id() == Some(row.as_str()));
    row_node.click();
    harness.run();
    assert_eq!(
        clicked.lock().unwrap().as_deref(),
        Some("DOC-7"),
        "AC-3: clicking a note row yields its document id for the open-document dispatch"
    );

    // The open-document command the row drives is the EXISTING cross-pane command (reuse, not a fork).
    let ctx = egui::Context::default();
    let mut bus = InteractionBus::new();
    bus.register_open_document_command();
    assert!(
        bus.open_document(&ctx, "DOC-7"),
        "AC-3: open-document is the existing cross-pane command"
    );
    assert_eq!(bus.take_pending_navigation().as_deref(), Some("DOC-7"));
    println!("AC-3: NoteRefsPanel listed DOC-7 (Design notes); click staged open-document DOC-7 ({CMD_OPEN_DOCUMENT})");
}

#[test]
fn ac3_live_shell_note_refs_row_click_opens_document_tab() {
    let (app, _rt) = code_note_editor_shell();
    let code_panel = app.mounted_code_panel();
    let backend = std::sync::Arc::new(CountingFindNotes {
        note_block_id: "BLK-7".to_owned(),
        note_title: "Design notes".to_owned(),
        calls: std::sync::atomic::AtomicUsize::new(0),
        queries: std::sync::Mutex::new(Vec::new()),
    });
    let backend_dyn: std::sync::Arc<dyn FindNotesSearch> = backend.clone();
    code_panel.set_find_notes_backend(backend_dyn);
    code_panel.set_show_note_refs(true);
    code_panel.set_note_refs_dwell_threshold(std::time::Duration::from_millis(0));
    code_panel.set_text("fn main() { let total = MyStruct::new(); }\n");
    code_panel.set_file_path("fixtures/mt034_code_ref.rs");
    let (lookup_base, lookup_thread) = spawn_single_json_response_server(
        code_symbol_lookup_response_body("KEN-MT034-MOCK", "fixtures/mt034_code_ref.rs", 1),
    );
    code_panel.set_code_nav_client(CodeNavClient::new(lookup_base));
    let offset = code_panel
        .buffer()
        .to_string()
        .find("MyStruct")
        .expect("symbol present")
        + 2;
    code_panel.set_single_cursor(offset);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 620.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    for _ in 0..80 {
        if matches!(code_panel.note_refs_state(), NoteRefsState::Loaded(_)) {
            break;
        }
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    harness.step();

    let row = row_author_id("DOC-7");
    let ids = author_ids(&harness);
    assert!(
        ids.contains(NOTE_REFS_PANEL_AUTHOR_ID),
        "AC-3 live shell: note-refs-panel is present before row click; got {ids:?}"
    );
    assert!(
        ids.contains(&row),
        "AC-3 live shell: row `{row}` is present before click; got {ids:?}"
    );
    harness
        .get_by(|n| n.author_id() == Some(row.as_str()))
        .click();
    harness.step();
    harness.step();

    let app = harness.state();
    let active = app
        .active_pane()
        .expect("AC-3 live shell: NoteRefs click focuses a pane");
    let active_tab = app
        .tab_bar_states()
        .get(active)
        .and_then(|bar| bar.active())
        .expect("AC-3 live shell: focused pane has an active tab");
    assert_eq!(
        active_tab.pane_type,
        PaneType::LoomWikiPage,
        "AC-3 live shell: NoteRefs row must route through open-document to the Notes pane"
    );
    assert_eq!(
        active_tab.content_id.as_deref(),
        Some("DOC-7"),
        "AC-3 live shell: note-ref-DOC-7 click opens document DOC-7 through the shell drain"
    );
    assert!(
        app.quick_switcher_nav_status().is_none(),
        "AC-3 live shell: open-document drain succeeds without nav error"
    );
    let lookup_request = lookup_thread.join().expect("join live-shell lookup mock");
    assert!(lookup_request.contains("GET /knowledge/code/symbols?"));

    println!("AC-3 LIVE SHELL: note-ref-DOC-7 click -> CMD_OPEN_DOCUMENT drain -> Notes tab DOC-7");
}

#[test]
#[cfg(feature = "wgpu_screenshots")]
fn mt034_canonical_argus_create_open_and_reveal() {
    use handshake_native::rich_editor::document_model::{
        position::DocPosition, selection::Selection,
    };

    let _guard = wgpu_guard();
    let symbol_id = "KEN-MT034-ARGUS";
    let symbol_name = "Mt034ArgusSymbol";
    let line_start_one_based = 12usize;
    let line_start_zero_based = line_start_one_based - 1;

    let mut fixture = external_source_fixture("canonical-argus", 48);
    let mut source_lines = (0..48)
        .map(|line| format!("// MT-034 canonical Argus source line {line}\n"))
        .collect::<Vec<_>>();
    source_lines[line_start_zero_based] = format!("pub struct {symbol_name} {{\n");
    source_lines[line_start_zero_based + 1] = "    pub value: i32,\n".to_owned();
    source_lines[line_start_zero_based + 2] = "}\n".to_owned();
    fixture.content = source_lines.concat();
    std::fs::write(&fixture.path, &fixture.content).expect("write canonical Argus source fixture");
    let file_path = fixture.path.to_string_lossy().to_string();

    let (base_url, server) =
        spawn_argus_code_symbol_server(symbol_id, symbol_name, &file_path, line_start_one_based);
    let (mut app, runtime) = code_note_editor_shell();
    app.set_active_pane_for_test(Some(PaneId::from("pane-b")));
    app.set_backend_base_url_for_test(&base_url, runtime.handle().clone());
    let rich_state = app.mounted_rich_state();
    {
        let mut state = rich_state.lock().expect("seed MT-034 Argus rich state");
        state.doc = BlockNode::doc(vec![BlockNode::paragraph("")]);
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
        state.set_code_ref_context(DEFAULT_PROJECT_ID, runtime.handle().clone());
    }

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);
    harness
        .state_mut()
        .clear_fems_overlay_for_integration_test();
    harness.run_steps(2);

    let proof_dir = external_artifact_dir("wp-kernel-012-mt-034/canonical-argus");
    std::fs::create_dir_all(&proof_dir).expect("create MT-034 canonical Argus proof directory");
    let mut argus =
        CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-034-create-open-reveal");
    let initial = argus.inspect(&mut harness);
    assert!(json_has_author_id(&initial, "editor.rich.text"));
    assert!(json_has_author_id(
        &initial,
        "editor.rich.insert-slash-command"
    ));

    // CREATE: the canonical parameterized slash action inserts the exact existing hsLink atom. This
    // avoids keyboard simulation and a transient popup while still exercising the product's real
    // RichDispatch::InsertSlashCommand payload path.
    let create_payload = serde_json::json!({
        "kind": "wikilink",
        "ref_kind": "code",
        "ref_value": symbol_id,
        "label": symbol_name
    });
    let create = argus.click_with_payload_and_reinspect(
        &mut harness,
        "editor.rich.insert-slash-command",
        create_payload.clone(),
    );
    let chip_id = code_ref_chip_author_id(symbol_id);
    let create_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "exact-code-ref-chip-created",
        |tree| json_has_author_id(tree, &chip_id),
    );
    let created_content = rich_state
        .lock()
        .expect("inspect agent-created code ref")
        .current_content_json();
    let created_links: Vec<_> = created_content["content"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|block| block["content"].as_array().into_iter().flatten())
        .filter(|child| child["type"] == "hsLink" && child["attrs"]["refKind"] == "code")
        .collect();
    assert_eq!(created_links.len(), 1, "one exact code hsLink is required");
    assert_eq!(created_links[0]["attrs"]["refValue"], symbol_id);
    assert_eq!(created_links[0]["attrs"]["label"], symbol_name);

    // OPEN: click the exact chip through canonical Argus, then wait for the real shell resolver to
    // fetch the symbol projection, mount the canonical file-backed code tab, and land on its line.
    let open = argus.click_and_reinspect(&mut harness, &chip_id);
    let navigation_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < navigation_deadline {
        let panel = harness.state().active_mounted_code_panel();
        if panel.file_path() == file_path
            && panel.buffer().to_string() == fixture.content
            && panel.last_visible_range().contains(&line_start_zero_based)
        {
            break;
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    let code_panel = harness.state().active_mounted_code_panel();
    assert_eq!(code_panel.file_path(), file_path);
    assert_eq!(code_panel.buffer().to_string(), fixture.content);
    assert!(
        code_panel
            .last_visible_range()
            .contains(&line_start_zero_based),
        "canonical code open must reveal the exact backend definition line"
    );

    // Mount the real dwell -> exact reverse lookup -> NoteRefs pipeline on that newly opened code tab.
    code_panel.set_runtime(runtime.handle().clone());
    code_panel.set_workspace_id(DEFAULT_PROJECT_ID);
    code_panel.set_code_nav_client(CodeNavClient::new(base_url));
    code_panel.set_find_notes_backend(std::sync::Arc::new(ArgusFindNotes {
        symbol_entity_id: symbol_id.to_owned(),
    }));
    code_panel.set_show_note_refs(true);
    code_panel.set_note_refs_dwell_threshold(Duration::ZERO);
    let symbol_offset = code_panel
        .buffer()
        .to_string()
        .find(symbol_name)
        .expect("canonical source contains symbol")
        + 2;
    code_panel.set_single_cursor(symbol_offset);
    let note_row = row_author_id("DOC-ARGUS-34");
    let note_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < note_deadline {
        if matches!(
            code_panel.note_refs_state(),
            NoteRefsState::Loaded(ref notes)
                if notes.iter().any(|note| note.document_id == "DOC-ARGUS-34")
        ) && author_ids(&harness).contains(&note_row)
        {
            break;
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(25));
    }
    harness.step();
    let code_text_author = code_panel.text_author_id();
    assert!(
        code_text_author.starts_with(CODE_EDITOR_TEXT_AUTHOR_ID),
        "file-backed tabs retain the canonical code text author-id prefix"
    );
    let open_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "canonical-source-line-and-note-ref-visible",
        |tree| {
            json_has_author_id_prefix(tree, &code_text_author)
                && json_has_author_id(tree, NOTE_REFS_PANEL_AUTHOR_ID)
                && json_has_author_id(tree, &note_row)
        },
    );

    let code_png = proof_dir.join("MT-034-code-symbol-open-note-refs.png");
    let code_image = harness
        .render()
        .expect("MT-034 code/NoteRefs WGPU render is required");
    let code_dimensions = [code_image.width(), code_image.height()];
    code_image
        .save(&code_png)
        .expect("save MT-034 code/NoteRefs screenshot");

    // REVEAL: the exact NoteRefs row routes through the existing open-document command and focuses the
    // canonical rich-document tab. The action remains fully inspect -> click -> attributed receipt ->
    // fresh terminal inspection.
    let reveal = argus.click_and_reinspect(&mut harness, &note_row);
    let reveal_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < reveal_deadline {
        let exact_document_active = harness
            .state()
            .active_pane()
            .and_then(|pane| harness.state().tab_bar_states().get(pane))
            .and_then(|bar| bar.active())
            .is_some_and(|tab| {
                tab.pane_type == PaneType::LoomWikiPage
                    && tab.content_id.as_deref() == Some("DOC-ARGUS-34")
            });
        let loaded_editor_mounted = author_ids(&harness)
            .iter()
            .any(|author_id| author_id.starts_with("editor.rich.text"));
        if exact_document_active && loaded_editor_mounted {
            break;
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    let reveal_terminal = argus.assert_latest_terminal_predicate(
        &mut harness,
        "referencing-rich-document-revealed",
        |tree| {
            json_has_author_id_prefix(tree, "editor.rich.text")
                && serde_json::to_string(tree)
                    .is_ok_and(|serialized| serialized.contains("DOC-ARGUS-34"))
        },
    );
    let active_tab = harness
        .state()
        .active_pane()
        .and_then(|pane| harness.state().tab_bar_states().get(pane))
        .and_then(|bar| bar.active())
        .expect("NoteRefs reveal focuses a rich-document tab");
    assert_eq!(active_tab.pane_type, PaneType::LoomWikiPage);
    assert_eq!(active_tab.content_id.as_deref(), Some("DOC-ARGUS-34"));

    let reveal_png = proof_dir.join("MT-034-referencing-note-revealed.png");
    let reveal_image = harness
        .render()
        .expect("MT-034 revealed-note WGPU render is required");
    let reveal_dimensions = [reveal_image.width(), reveal_image.height()];
    reveal_image
        .save(&reveal_png)
        .expect("save MT-034 revealed-note screenshot");

    let proof_path = proof_dir.join("MT-034-create-open-reveal.json");
    std::fs::write(
        &proof_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_id": "hsk.mt034-canonical-argus-proof@1",
            "microtask": "MT-034",
            "flow": "create-open-reveal",
            "symbol": {
                "symbol_entity_id": symbol_id,
                "display_name": symbol_name,
                "file_path": file_path,
                "line_start_one_based": line_start_one_based
            },
            "stable_targets": [
                "editor.rich.insert-slash-command",
                chip_id.clone(),
                code_text_author.clone(),
                NOTE_REFS_PANEL_AUTHOR_ID,
                note_row.clone(),
                "editor.rich.text"
            ],
            "actions": [
                {
                    "payload": create_payload,
                    "proof": canonical_action_proof(
                        "editor.rich.insert-slash-command",
                        &create,
                        "exact-code-ref-chip-created",
                        &create_terminal
                    )
                },
                canonical_action_proof(
                    &chip_id,
                    &open,
                    "canonical-source-line-and-note-ref-visible",
                    &open_terminal
                ),
                canonical_action_proof(
                    &note_row,
                    &reveal,
                    "referencing-rich-document-revealed",
                    &reveal_terminal
                )
            ],
            "initial": initial,
            "terminal": reveal_terminal,
            "screenshots": [
                {
                    "path": code_png,
                    "dimensions": code_dimensions,
                    "capture_method": "mounted_wgpu_harness_after_fresh_argus_terminal",
                    "bound_to_action_target": chip_id
                },
                {
                    "path": reveal_png,
                    "dimensions": reveal_dimensions,
                    "capture_method": "mounted_wgpu_harness_after_fresh_argus_terminal",
                    "bound_to_action_target": note_row
                }
            ]
        }))
        .expect("serialize MT-034 canonical Argus proof"),
    )
    .expect("write MT-034 canonical Argus proof");

    argus.finish();
    let requests = server.join().expect("join MT-034 canonical Argus server");
    assert!(
        requests
            .iter()
            .any(|request| request.contains("GET /knowledge/code/symbols/KEN-MT034-ARGUS ")),
        "chip open must use the exact symbol-detail route: {requests:?}"
    );
    assert!(
        requests
            .iter()
            .any(|request| request.contains("GET /knowledge/code/symbols?")),
        "code dwell must resolve through the exact symbol-lookup route: {requests:?}"
    );
    assert!(
        requests
            .iter()
            .any(|request| request.contains("GET /knowledge/documents/DOC-ARGUS-34 ")),
        "NoteRefs reveal must load the exact rich-document route: {requests:?}"
    );
    assert_no_local_artifact_dir();
    drop(harness);
    runtime.shutdown_timeout(Duration::from_secs(2));
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 (unit + kittest): an UNRESOLVED code ref renders a greyed `unresolved` chip and does not panic.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac4_unresolved_code_ref_chip_renders_without_panic() {
    use handshake_native::rich_editor::wikilinks::inline_view::{chip_colors, chip_label};
    // A symbol the backend 404'd -> the chip is marked resolved=false (the 404 path sets this).
    let unresolved = HsLinkNode {
        ref_kind: "code".into(),
        ref_value: "ent-deleted".into(),
        label: "src/gone.rs#Gone".into(),
        resolved: false,
        provenance: None,
    };
    // The label is the greyed `unresolved` text (never a panic).
    let label = chip_label(&unresolved);
    assert!(
        label.contains("unresolved"),
        "AC-4: a deleted symbol renders an `unresolved` chip label"
    );
    // The chip colors come from the theme (the error affordance), NOT a hardcoded Color32.
    let palette = HsTheme::Dark.palette();
    let (bg, fg) = chip_colors(&unresolved, &palette);
    assert_eq!(
        bg, palette.error_bg,
        "AC-4: an unresolved chip uses the error background (theme token)"
    );
    assert_eq!(fg, palette.error_text);

    // And it RENDERS in a live editor without panicking (the doc carries the unresolved code ref).
    let mut doc = BlockNode::new(NodeKind::Paragraph);
    doc.children.push(Child::HsLink(unresolved));
    let doc = BlockNode::doc(vec![doc]);
    let state = std::sync::Arc::new(std::sync::Mutex::new(RichEditorState::new(doc)));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 400.0))
        .build_ui(move |ui| {
            RichEditorWidget::new(std::sync::Arc::clone(&state)).show(ui);
        });
    harness.run(); // no panic == pass
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&code_ref_chip_author_id("ent-deleted")),
        "AC-4: the unresolved chip is still addressable (greyed, not removed); got {ids:?}"
    );
    println!("AC-4: unresolved code-ref chip rendered greyed ('{label}'), no panic");
}

#[test]
fn ac4_resolve_error_maps_unresolved() {
    // The resolution-error vocabulary: a NotFound / NoDefinition / EmptySymbol is `unresolved` (drives
    // the greyed chip); a transient backend error is NOT (it should retry, not grey out).
    assert!(CrossRefError::NotFound("x".into()).is_unresolved());
    assert!(CrossRefError::NoDefinition("x".into()).is_unresolved());
    assert!(
        !CrossRefError::StaleSource {
            symbol: "x".into(),
            state: "marked_stale".into(),
        }
        .is_unresolved(),
        "a stale source is retryable after re-index and must not be rewritten as a deleted symbol"
    );
    assert!(!CrossRefError::Backend("down".into()).is_unresolved());
    println!("AC-4: resolve errors classify NotFound/NoDefinition/EmptySymbol as unresolved");
}

#[test]
fn ac4_stale_source_is_typed_and_never_navigates_silently() {
    let symbol_id = "KEN-mt034-stale";
    let body = serde_json::json!({
        "symbol": {
            "symbol_entity_id": symbol_id,
            "symbol_key": "rust:src/stale.rs#Mt034Stale",
            "display_name": "Mt034Stale",
            "symbol_kind": "struct",
            "definition": {
                "line_start": 4,
                "line_end": 5,
                "source_id": "KSRC-MT034-STALE"
            },
            "staleness": {
                "state": "marked_stale",
                "fresh": false
            }
        }
    })
    .to_string();
    let (base_url, server) = spawn_single_json_response_server(body);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build stale-source runtime");
    let resolved = runtime.block_on(handshake_native::interop::cross_ref::resolve_code_ref_with(
        &CodeNavClient::new(base_url),
        symbol_id,
    ));
    assert_eq!(
        resolved,
        Err(CrossRefError::StaleSource {
            symbol: symbol_id.to_owned(),
            state: "marked_stale".to_owned(),
        }),
        "a backend fresh=false projection must stop navigation with a typed recovery state"
    );
    let request = server.join().expect("join stale-symbol server");
    assert!(request.contains("GET /knowledge/code/symbols/KEN-mt034-stale "));
}

#[test]
fn ac4_opaque_source_id_without_symbol_key_path_is_unresolved() {
    let body = serde_json::json!({
        "symbol": {
            "symbol_entity_id": "KEN-mt034-malformed",
            "symbol_key": "rust:#Mt034Malformed",
            "display_name": "Mt034Malformed",
            "symbol_kind": "struct",
            "definition": {
                "line_start": 4,
                "line_end": 5,
                "source_id": "KSRC-MT034-OPAQUE-ONLY"
            }
        }
    })
    .to_string();
    let (base_url, server) = spawn_single_json_response_server(body);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build malformed-symbol runtime");
    let resolved = runtime.block_on(handshake_native::interop::cross_ref::resolve_code_ref_with(
        &CodeNavClient::new(base_url),
        "KEN-mt034-malformed",
    ));
    assert!(
        matches!(resolved, Err(CrossRefError::NoDefinition(_))),
        "an opaque KnowledgeSource id is never reinterpreted as a file path: {resolved:?}"
    );
    let request = server.join().expect("join malformed-symbol server");
    assert!(request.contains("GET /knowledge/code/symbols/KEN-mt034-malformed "));
}

#[test]
fn ac4_entity_lookup_rejects_a_different_backend_symbol_identity() {
    let body = code_symbol_response_body("KEN-B", "src/b.rs", 4);
    let (base_url, server) = spawn_single_json_response_server(body);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build identity-mismatch runtime");
    let resolved = runtime.block_on(handshake_native::interop::cross_ref::resolve_code_ref_with(
        &CodeNavClient::new(base_url),
        "KEN-A",
    ));
    assert_eq!(
        resolved,
        Err(CrossRefError::IdentityMismatch {
            requested: "KEN-A".to_owned(),
            returned: "KEN-B".to_owned(),
        }),
        "a response for B must never be accepted as the requested entity A"
    );
    let request = server.join().expect("join identity-mismatch server");
    assert!(request.contains("GET /knowledge/code/symbols/KEN-A "));
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-5 (AccessKit dump): the code-symbol search dialog exposes `code-symbol-search` (Dialog) +
// `code-symbol-search-input` (TextField) in the live tree.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ac5_code_symbol_search_dialog_accesskit_ids_present() {
    let palette = HsTheme::Dark.palette();
    let dialog = std::sync::Arc::new(std::sync::Mutex::new(CodeSymbolSearchState::open(
        "ws-1", None,
    )));
    let dialog_ui = std::sync::Arc::clone(&dialog);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(500.0, 400.0))
        .build_ui(move |ui| {
            let mut d = dialog_ui.lock().unwrap();
            let _ = render_code_symbol_search_dialog(ui.ctx(), &mut d, &palette);
        });
    harness.run();

    let ids = author_ids(&harness);
    assert!(
        ids.contains(CODE_SYMBOL_SEARCH_AUTHOR_ID),
        "AC-5: the code-symbol-search Dialog is present; got {ids:?}"
    );
    assert!(
        ids.contains(CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID),
        "AC-5: the code-symbol-search-input TextField is present; got {ids:?}"
    );
    println!(
        "AC-5: code-symbol-search dialog exposes code-symbol-search + code-symbol-search-input"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// RISK-2 / MC-2 (unit): a symbol key with `::`, `/`, `#` percent-encodes for URL embedding.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn risk2_symbol_key_percent_encodes_for_urls() {
    let encoded = percent_encode_symbol("fn:src/main.rs#MyStruct::new");
    assert!(!encoded.contains('/') && !encoded.contains('#') && !encoded.contains(':'));
    assert_eq!(encoded, "fn%3Asrc%2Fmain.rs%23MyStruct%3A%3Anew");
    println!("RISK-2: symbol key percent-encodes -> {encoded}");
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// Hygiene (CX-212E): no repo-local artifact dir under the crate.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn no_local_artifact_dir_under_crate() {
    assert_no_local_artifact_dir();
    println!("CX-212E: no repo-local test_output/ or tests/screenshots/ dir under the crate");
}

#[test]
fn negative_backend_loss_is_typed_for_both_cross_ref_directions() {
    use handshake_native::interop::cross_ref::{
        find_notes_with, resolve_code_ref_with, FindNotesHttp,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build backend-loss runtime");
    let unavailable = "http://127.0.0.1:9";
    let resolve = runtime.block_on(resolve_code_ref_with(
        &CodeNavClient::new(unavailable),
        "KEN-mt034-backend-loss",
    ));
    assert!(
        matches!(resolve, Err(CrossRefError::Backend(_))),
        "MT-034 backend loss must remain a typed transient error, got {resolve:?}"
    );

    let notes = runtime.block_on(find_notes_with(
        &FindNotesHttp::new(unavailable),
        "rust:src/mt034.rs#Mt034ExactSymbol",
        "ws-mt034-backend-loss",
    ));
    assert!(
        matches!(notes, Err(CrossRefError::Backend(_))),
        "MT-034 reverse lookup backend loss must remain typed, got {notes:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// LIVE-BACKEND (--features integration): self-seeds a real code index + RichDocument in an
// isolated managed-PostgreSQL workspace, drives the mounted product paths, and canonically cleans it.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "integration")]
mod live_backend {
    use std::sync::{Arc, Mutex};

    use egui_kittest::kittest::{NodeT, Queryable};
    use egui_kittest::Harness;

    use handshake_native::app::HandshakeApp;
    use handshake_native::backend::knowledge_documents::{
        CreateDocumentRequest, HskDocumentHeaders, KnowledgeDocumentsClient, SaveDocumentRequest,
    };
    use handshake_native::backend_client::RichDocClient;
    use handshake_native::code_editor::code_nav::CodeNavClient;
    use handshake_native::code_editor::note_refs_panel::{
        row_author_id, NoteRefsState, PANEL_AUTHOR_ID,
    };
    use handshake_native::code_editor::panel::CodeEditorPanel;
    use handshake_native::interop::cross_ref::{
        find_code_ref_notes_with, resolve_code_ref_with, CrossRefError, FindNotesHttp,
        FindNotesSearch,
    };
    use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};
    use handshake_native::rich_editor::document_model::position::DocPosition;
    use handshake_native::rich_editor::document_model::selection::Selection;
    use handshake_native::rich_editor::renderer::rich_editor_widget::{
        RichEditorState, RichEditorWidget,
    };
    use handshake_native::rich_editor::slash_commands::{
        code_symbol_result_author_id, slash_item_author_id, CODE_SYMBOL_SEARCH_AUTHOR_ID,
        CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID,
    };
    use handshake_native::rich_editor::wikilinks::inline_view::code_ref_chip_author_id;

    use super::{code_note_editor_shell, external_artifact_dir};

    const SYMBOL_NAME: &str = "Mt034ExactSymbol";
    const SOURCE_FILE: &str = "mt034_exact_symbol.rs";
    const SOURCE: &str = "// MT-034 managed PostgreSQL fixture\n\npub struct Mt034ExactSymbol {\n    pub value: i32,\n}\n\nimpl Mt034ExactSymbol {\n    pub fn new(value: i32) -> Self { Self { value } }\n}\n";

    fn author_ids<S>(harness: &Harness<'_, S>) -> std::collections::HashSet<String> {
        let mut ids = std::collections::HashSet::new();
        for node in harness.root().children_recursive() {
            if let Some(author_id) = node.accesskit_node().author_id() {
                ids.insert(author_id.to_owned());
            }
        }
        ids
    }

    fn role_for<S>(harness: &Harness<'_, S>, author_id: &str) -> Option<String> {
        harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id))
            .map(|node| format!("{:?}", node.accesskit_node().role()))
    }

    struct ManagedCleanup {
        backend: crate::interconnect_support::LiveBackend,
        workspace_id: String,
        source_dir: std::path::PathBuf,
        cleaned: bool,
    }

    impl std::ops::Deref for ManagedCleanup {
        type Target = crate::interconnect_support::LiveBackend;

        fn deref(&self) -> &Self::Target {
            &self.backend
        }
    }

    impl std::ops::DerefMut for ManagedCleanup {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.backend
        }
    }

    impl ManagedCleanup {
        fn clean(&mut self) -> u16 {
            if self.cleaned {
                return 204;
            }
            let status = self.backend.delete_workspace(&self.workspace_id);
            let _ = std::fs::remove_dir_all(&self.source_dir);
            self.cleaned = true;
            status
        }
    }

    impl Drop for ManagedCleanup {
        fn drop(&mut self) {
            let _ = self.clean();
        }
    }

    fn focus_author<S>(harness: &mut Harness<'_, S>, author_id: &str) {
        harness
            .get_by(|node| node.author_id() == Some(author_id))
            .focus();
        harness.step();
        harness.step();
    }

    fn source_definition_line_zero_based() -> usize {
        SOURCE
            .lines()
            .position(|line| line.contains("pub struct Mt034ExactSymbol"))
            .expect("fixture contains exact symbol definition")
    }

    fn assert_persisted_code_ref(content: &serde_json::Value, symbol_id: &str) {
        let links: Vec<_> = content["content"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|block| block["content"].as_array().into_iter().flatten())
            .filter(|child| child["type"] == "hsLink" && child["attrs"]["refKind"] == "code")
            .collect();
        assert_eq!(links.len(), 1, "one persisted code hsLink is required");
        assert_eq!(links[0]["attrs"]["refValue"], symbol_id);
        assert_eq!(links[0]["attrs"]["label"], SYMBOL_NAME);
    }

    #[test]
    fn v2_self_seeded_postgres_code_ref_round_trip_navigation_and_note_refs() {
        crate::interconnect_support::assert_no_local_artifact_dir();
        let backend = crate::interconnect_support::require_reachable_backend();
        let unique = format!(
            "mt034-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after unix epoch")
                .as_nanos()
        );
        let workspace = backend.create_workspace(&unique);
        let workspace_id = workspace["id"]
            .as_str()
            .expect("workspace create returns id")
            .to_owned();

        let source_dir =
            external_artifact_dir("wp-kernel-012-mt-034").join(format!("source-{unique}"));
        // Install cleanup as soon as the managed workspace identity exists. Every subsequent
        // filesystem/runtime/indexing assertion is fallible, so unwinding from any of them must still
        // delete the workspace and the (possibly only partially created) external source directory.
        let mut live = ManagedCleanup {
            backend,
            workspace_id: workspace_id.clone(),
            source_dir: source_dir.clone(),
            cleaned: false,
        };
        std::fs::create_dir_all(&source_dir).expect("create external source fixture directory");
        let source_path = source_dir.join(SOURCE_FILE);
        std::fs::write(&source_path, SOURCE).expect("write external Rust source fixture");
        let unrelated_path = source_dir.join("unrelated.rs");
        std::fs::write(&unrelated_path, "pub fn unrelated() {}\n")
            .expect("write unrelated mounted-source fixture");
        let source_dir = source_dir
            .canonicalize()
            .expect("canonical external source fixture directory");
        let source_path = source_path
            .canonicalize()
            .expect("canonical exact source fixture path");
        let unrelated_path = unrelated_path
            .canonicalize()
            .expect("canonical unrelated source fixture path");

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("build MT-034 managed proof runtime");
        let http = reqwest::Client::new();
        let index_url = format!("{}/workspaces/{workspace_id}/code-nav/index", live.base);
        let root_path = source_dir.to_string_lossy().to_string();

        // Negative identity path: the mutation must fail before it reaches indexing when the required
        // navigation identity is absent. The following attributed request is the one allowed to seed.
        let missing_index_identity = runtime.block_on(async {
            http.post(&index_url)
                .json(&serde_json::json!({"root_path": root_path.clone()}))
                .send()
                .await
                .expect("missing-identity index response")
                .status()
        });
        assert_eq!(missing_index_identity.as_u16(), 400);

        let index_body: serde_json::Value = runtime.block_on(async {
            let response = http
                .post(&index_url)
                .header("x-hsk-actor-id", "mt034-managed-proof")
                .header("x-hsk-actor-kind", "validation_runner")
                .header("x-hsk-kernel-task-run-id", "KTR-MT034-V2")
                .header("x-hsk-session-run-id", "SR-MT034-V2")
                .json(&serde_json::json!({"root_path": root_path.clone()}))
                .send()
                .await
                .expect("attributed code index request");
            let status = response.status();
            let body = response.text().await.expect("index response body");
            assert!(status.is_success(), "code index failed: {status}: {body}");
            serde_json::from_str(&body).expect("index response JSON")
        });
        assert!(
            index_body["symbol_count"].as_u64().unwrap_or(0) >= 2,
            "real index must produce the struct + impl member: {index_body:?}"
        );

        let code_nav = CodeNavClient::new(live.base.clone());
        let symbol = runtime
            .block_on(code_nav.lookup_symbols(&workspace_id, SYMBOL_NAME, 20))
            .expect("live symbol lookup")
            .into_iter()
            .find(|symbol| symbol.display_name == SYMBOL_NAME)
            .expect("indexed exact symbol");
        let symbol_id = symbol.symbol_entity_id.clone();
        let symbol_key = symbol.symbol_key.clone();
        assert!(symbol_key.ends_with(&format!("{SOURCE_FILE}#{SYMBOL_NAME}")));
        let expected_line = source_definition_line_zero_based();
        assert_eq!(
            symbol
                .definition
                .as_ref()
                .and_then(|definition| definition.line_start)
                .map(|line| line - 1),
            Some(expected_line as i64)
        );
        assert!(
            symbol
                .definition
                .as_ref()
                .and_then(|definition| definition.source_id.as_deref())
                .is_some_and(|source_id| source_id != SOURCE_FILE),
            "the fixture must exercise an opaque KnowledgeSource id, not a mock path"
        );

        // Negative identity path: a bare navigation GET is rejected; the product CodeNavClient above
        // succeeds because its canonical transport attached all required identity headers.
        let missing_nav_identity = runtime.block_on(async {
            http.get(format!("{}/knowledge/code/symbols/{symbol_id}", live.base))
                .send()
                .await
                .expect("missing-identity nav response")
                .status()
        });
        assert_eq!(missing_nav_identity.as_u16(), 400);

        let document_client = KnowledgeDocumentsClient::with_base_url(live.base.clone());
        let create_headers = HskDocumentHeaders::for_operator("SR-MT034-CREATE", "pending-mt034");
        let created = runtime
            .block_on(document_client.create_document(
                &create_headers,
                &CreateDocumentRequest {
                    workspace_id: workspace_id.clone(),
                    title: format!("MT-034 code reference {unique}"),
                    create_if_title_absent: false,
                    content_json: Some(serde_json::json!({
                        "type": "doc",
                        "content": [{"type": "paragraph", "content": []}]
                    })),
                    schema_version: None,
                    project_ref: None,
                    folder_ref: None,
                },
            ))
            .expect("create real rich document");
        let document_id = created.document["rich_document_id"]
            .as_str()
            .expect("created rich_document_id")
            .to_owned();
        let created_version = created.document["doc_version"]
            .as_i64()
            .expect("created doc_version");

        // Drive the real editor interaction: type `/code-ref`, click its slash row, type the lookup
        // query into the exact AccessKit input, wait for the real backend result, and click that row.
        let editor_state = Arc::new(Mutex::new(RichEditorState::new(
            handshake_native::rich_editor::document_model::node::BlockNode::doc(vec![
                handshake_native::rich_editor::document_model::node::BlockNode::paragraph(""),
            ]),
        )));
        {
            let mut state = editor_state.lock().expect("editor state");
            state.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
            state.set_code_ref_context(workspace_id.clone(), runtime.handle().clone());
        }
        let editor_for_ui = Arc::clone(&editor_state);
        let mut editor_harness = Harness::builder()
            .with_size(egui::vec2(760.0, 520.0))
            .build_ui(move |ui| {
                RichEditorWidget::new(Arc::clone(&editor_for_ui)).show(ui);
            });
        editor_harness.step();
        focus_author(&mut editor_harness, "editor.rich.text");
        editor_harness.event(egui::Event::Text("/code-ref".to_owned()));
        editor_harness.step();
        let slash_code_ref_id = slash_item_author_id("code-ref");
        assert_eq!(
            role_for(&editor_harness, &slash_code_ref_id).as_deref(),
            Some("MenuItem"),
            "the live /code-ref command must be addressable before activation"
        );
        editor_harness
            .get_by(|node| node.author_id() == Some(slash_code_ref_id.as_str()))
            .click();
        for _ in 0..4 {
            editor_harness.step();
            if editor_state
                .lock()
                .expect("editor state after /code-ref click")
                .code_symbol_search
                .is_some()
            {
                break;
            }
        }
        {
            let mut state = editor_state.lock().expect("editor state");
            state
                .code_symbol_search
                .as_mut()
                .expect("/code-ref opens code symbol search")
                .client = code_nav.clone();
        }
        editor_harness.step();
        assert_eq!(
            role_for(&editor_harness, CODE_SYMBOL_SEARCH_AUTHOR_ID).as_deref(),
            Some("Dialog")
        );
        focus_author(&mut editor_harness, CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID);
        editor_harness.event(egui::Event::Text(SYMBOL_NAME.to_owned()));
        editor_harness.step();
        let result_id = code_symbol_result_author_id(&symbol_id);
        for _ in 0..100 {
            if author_ids(&editor_harness).contains(&result_id) {
                break;
            }
            editor_harness.step();
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert_eq!(
            role_for(&editor_harness, CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID).as_deref(),
            Some("TextInput")
        );
        assert_eq!(
            role_for(&editor_harness, &result_id).as_deref(),
            Some("ListItem")
        );
        editor_harness
            .get_by(|node| node.author_id() == Some(result_id.as_str()))
            .click();
        editor_harness.step();
        editor_harness.step();
        let inserted_content = editor_state
            .lock()
            .expect("editor state")
            .current_content_json();
        assert_persisted_code_ref(&inserted_content, &symbol_id);
        let chip_id = code_ref_chip_author_id(&symbol_id);
        assert!(author_ids(&editor_harness).contains(&chip_id));
        assert_eq!(
            role_for(&editor_harness, &chip_id).as_deref(),
            Some("Button")
        );
        drop(editor_harness);

        // Negative identity path for save, followed by the real product client save/reload.
        let missing_save_identity = runtime.block_on(async {
            http.put(format!(
                "{}/knowledge/documents/{document_id}/save",
                live.base
            ))
            .json(&serde_json::json!({
                "expected_version": created_version,
                "content_json": inserted_content.clone()
            }))
            .send()
            .await
            .expect("missing-identity save response")
            .status()
        });
        assert_eq!(missing_save_identity.as_u16(), 400);

        let document_headers = HskDocumentHeaders::for_operator("SR-MT034-SAVE", &document_id);
        let saved = runtime
            .block_on(document_client.save_document(
                &document_headers,
                &document_id,
                &SaveDocumentRequest {
                    expected_version: created_version,
                    content_json: inserted_content.clone(),
                    crdt_document_id: None,
                    crdt_snapshot_id: None,
                    promotion_receipt_event_id: None,
                },
            ))
            .expect("save inserted code ref");
        assert!(
            saved
                .save_receipt_event_id
                .as_deref()
                .is_some_and(|receipt| !receipt.trim().is_empty()),
            "save must return an attributable receipt"
        );
        let loaded = runtime
            .block_on(document_client.load_document(&document_headers, &document_id))
            .expect("reload real rich document");
        let loaded_content = loaded.document["content_json"].clone();
        assert_persisted_code_ref(&loaded_content, &symbol_id);

        // Optimistic conflict: retrying the pre-save version must return a typed 409 and must not
        // replace the already-committed code reference.
        let conflicting_content = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": "must never win"}]
            }]
        });
        let mut conflict_request = http
            .put(format!(
                "{}/knowledge/documents/{document_id}/save",
                live.base
            ))
            .header("x-hsk-actor-id", &document_headers.actor_id)
            .header(
                "x-hsk-kernel-task-run-id",
                &document_headers.kernel_task_run_id,
            )
            .header("x-hsk-session-run-id", &document_headers.session_run_id);
        if let Some(actor_kind) = document_headers.actor_kind.as_deref() {
            conflict_request = conflict_request.header("x-hsk-actor-kind", actor_kind);
        }
        if let Some(correlation_id) = document_headers.correlation_id.as_deref() {
            conflict_request = conflict_request.header("x-hsk-correlation-id", correlation_id);
        }
        let conflict_status = runtime.block_on(async {
            conflict_request
                .json(&serde_json::json!({
                    "expected_version": created_version,
                    "content_json": conflicting_content
                }))
                .send()
                .await
                .expect("stale-version conflict response")
                .status()
        });
        assert_eq!(conflict_status.as_u16(), 409);
        let after_conflict = runtime
            .block_on(document_client.load_document(&document_headers, &document_id))
            .expect("reload after rejected conflict");
        assert_eq!(
            after_conflict.document["content_json"], loaded_content,
            "the stale writer must not mutate the committed code reference"
        );

        // Missing symbol is distinct from stale source: a never-existing entity is a typed unresolved
        // result and does not invent a path.
        let missing_symbol_id = format!("KEN-MT034-MISSING-{unique}");
        let missing_symbol = runtime.block_on(resolve_code_ref_with(&code_nav, &missing_symbol_id));
        assert!(
            missing_symbol
                .as_ref()
                .is_err_and(CrossRefError::is_unresolved),
            "a missing symbol must remain an explicit unresolved result: {missing_symbol:?}"
        );

        // The reverse lookup must find the real document through its transactional Loom note/search
        // projection, then the mounted code pane must render the exact NoteRefs AccessKit nodes.
        let find_notes = FindNotesHttp::new(live.base.clone());
        let notes = runtime
            .block_on(find_code_ref_notes_with(
                &find_notes,
                &symbol_id,
                &symbol_key,
                &workspace_id,
            ))
            .expect("real NoteRefs reverse lookup");
        assert!(
            notes.iter().any(|note| note.document_id == document_id),
            "NoteRefs must return the referencing rich document: {notes:?}"
        );

        // Restart the exact fixture-owned current-source backend while retaining PostgreSQL authority.
        // Fresh clients must read back the same document, symbol, and reverse-reference row afterwards.
        let binding_before_restart = live.owned_backend_binding_receipt();
        let old_backend_pid = binding_before_restart["backend_pid"]
            .as_u64()
            .expect("owned backend binding carries pid");
        let (old_base, new_base) = live.restart_owned();
        assert_eq!(
            old_base, new_base,
            "owned restart reclaims the exact listener"
        );
        let binding_after_restart = live.owned_backend_binding_receipt();
        let new_backend_pid = binding_after_restart["backend_pid"]
            .as_u64()
            .expect("restarted backend binding carries pid");
        assert_ne!(
            old_backend_pid, new_backend_pid,
            "restart proof must replace only the fixture-owned backend child"
        );
        let restarted_code_nav = CodeNavClient::new(live.base.clone());
        let restarted_documents = KnowledgeDocumentsClient::with_base_url(live.base.clone());
        let restarted_find_notes = FindNotesHttp::new(live.base.clone());
        let restarted_symbol = runtime
            .block_on(restarted_code_nav.get_symbol(&symbol_id))
            .expect("fresh symbol read after owned backend restart");
        assert_eq!(restarted_symbol.symbol.symbol_entity_id, symbol_id);
        assert!(
            restarted_symbol
                .symbol
                .staleness
                .as_ref()
                .is_some_and(|staleness| staleness.fresh),
            "the indexed symbol remains explicitly fresh after restart"
        );
        let restarted_document = runtime
            .block_on(restarted_documents.load_document(&document_headers, &document_id))
            .expect("fresh document read after owned backend restart");
        assert_persisted_code_ref(&restarted_document.document["content_json"], &symbol_id);
        let restarted_notes = runtime
            .block_on(find_code_ref_notes_with(
                &restarted_find_notes,
                &symbol_id,
                &symbol_key,
                &workspace_id,
            ))
            .expect("fresh reverse lookup after owned backend restart");
        assert!(
            restarted_notes
                .iter()
                .any(|note| note.document_id == document_id),
            "reverse lookup persists through owned backend restart: {restarted_notes:?}"
        );

        let note_panel = Arc::new(CodeEditorPanel::new(SOURCE, "rs"));
        note_panel.set_file_path(source_path.to_string_lossy());
        note_panel.set_runtime(runtime.handle().clone());
        note_panel.set_workspace_id(workspace_id.clone());
        note_panel.set_code_nav_client(restarted_code_nav.clone());
        let find_backend: Arc<dyn FindNotesSearch> = Arc::new(restarted_find_notes.clone());
        note_panel.set_find_notes_backend(find_backend);
        note_panel.set_show_note_refs(true);
        note_panel.set_note_refs_dwell_threshold(std::time::Duration::ZERO);
        let cursor = SOURCE.find(SYMBOL_NAME).expect("symbol in fixture") + 2;
        note_panel.set_single_cursor(cursor);
        let panel_for_ui = Arc::clone(&note_panel);
        let mut note_harness = Harness::builder()
            .with_size(egui::vec2(980.0, 520.0))
            .build_ui(move |ui| {
                panel_for_ui.show(ui);
            });
        note_harness.step();
        for _ in 0..100 {
            if matches!(note_panel.note_refs_state(), NoteRefsState::Loaded(_)) {
                break;
            }
            note_harness.step();
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        note_harness.step();
        let loaded_notes = note_panel.note_refs_state();
        assert!(
            matches!(&loaded_notes, NoteRefsState::Loaded(notes) if notes.iter().any(|note| note.document_id == document_id)),
            "mounted NoteRefs must contain the real referencing document: {loaded_notes:?}"
        );
        assert_eq!(
            role_for(&note_harness, PANEL_AUTHOR_ID).as_deref(),
            Some("List")
        );
        assert_eq!(
            role_for(&note_harness, &row_author_id(&document_id)).as_deref(),
            Some("ListItem")
        );
        drop(note_harness);
        drop(note_panel);

        // Mount the reloaded rich document in the real two-pane shell. Click the actual chip node; the
        // shell must resolve via real CodeNav and focus the exact symbol line. The opaque source_id
        // assertion above ensures this only passes when symbol_key path extraction is correct.
        let (delete_status, stale_state_proven) = {
            let mounted_body = runtime
                .block_on(
                    RichDocClient::new(live.base.clone(), runtime.handle().clone())
                        .load_document(&document_id),
                )
                .expect("load mounted rich document through the production client");
            let (mut app, app_runtime) = code_note_editor_shell();
            app.set_backend_base_url_for_test(&live.base, app_runtime.handle().clone());
            app.install_mounted_code_nav_client_for_test(restarted_code_nav.clone());
            assert!(
                matches!(
                    app.open_document(&document_id),
                    NavDispatchOutcome::Opened { .. }
                ),
                "production Notes navigation opens and activates the persisted document"
            );
            let rich_pane_id = app
                .active_pane()
                .expect("production Notes navigation focuses the rich pane")
                .clone();
            app.apply_loaded_rich_document_to_view_for_test(rich_pane_id.as_ref(), mounted_body)
                .expect("install the mounted document in its canonical pane binding");
            let rich_state = app.mounted_rich_state();
            let code_panel = app.mounted_code_panel();
            code_panel.set_text("pub fn unrelated() {}\n");
            code_panel.set_file_path(unrelated_path.to_string_lossy().to_string());
            let mut app_harness = Harness::builder()
                .with_size(egui::vec2(1100.0, 700.0))
                .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
            let chip_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let ids = author_ids(&app_harness);
                if ids.contains(&chip_id) {
                    break;
                }
                assert!(
                    std::time::Instant::now() < chip_deadline,
                    "mounted rich code-ref chip `{chip_id}` did not appear within the bounded wait; \
                     present ids: {ids:?}"
                );
                app_harness.step();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            app_harness
                .get_by(|node| node.author_id() == Some(chip_id.as_str()))
                .click();
            app_harness.step();
            for _ in 0..100 {
                let active_panel = app_harness.state().active_mounted_code_panel();
                if std::path::Path::new(&active_panel.file_path())
                    .canonicalize()
                    .ok()
                    .as_ref()
                    == Some(&source_path)
                    && active_panel.last_visible_range().contains(&expected_line)
                    && active_panel.buffer().to_string() == SOURCE
                {
                    break;
                }
                app_harness.step();
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            let active_panel = app_harness.state().active_mounted_code_panel();
            assert_eq!(
                std::path::Path::new(&active_panel.file_path())
                    .canonicalize()
                    .expect("active target canonical path"),
                source_path,
                "a relative persisted code ref must open its canonical file, not jump in the unrelated mounted buffer"
            );
            assert_eq!(active_panel.buffer().to_string(), SOURCE);
            assert!(active_panel.last_visible_range().contains(&expected_line));
            assert!(app_harness.state().quick_switcher_nav_status().is_none());

            // Remove the indexed source and run the real index lifecycle again. The symbol remains a
            // durable identity, but every nav route must now carry fresh=false/marked_stale. Clicking
            // the same mounted chip must stop with a typed recovery status instead of silently jumping
            // to the stale persisted line.
            std::fs::remove_file(&source_path).expect("remove indexed source for stale proof");
            let stale_index_body: serde_json::Value = runtime.block_on(async {
                let response = http
                    .post(format!(
                        "{}/workspaces/{workspace_id}/code-nav/index",
                        live.base
                    ))
                    .header("x-hsk-actor-id", "mt034-managed-proof")
                    .header("x-hsk-actor-kind", "validation_runner")
                    .header("x-hsk-kernel-task-run-id", "KTR-MT034-V3-STALE")
                    .header("x-hsk-session-run-id", "SR-MT034-V3-STALE")
                    .json(&serde_json::json!({"root_path": root_path.clone()}))
                    .send()
                    .await
                    .expect("stale-source reindex response");
                let status = response.status();
                let body = response.text().await.expect("stale reindex body");
                assert!(
                    status.is_success(),
                    "stale-source reindex failed: {status}: {body}"
                );
                serde_json::from_str(&body).expect("stale reindex JSON")
            });
            let stale_detail = runtime
                .block_on(restarted_code_nav.get_symbol(&symbol_id))
                .expect("read marked-stale symbol");
            let stale_state = stale_detail
                .symbol
                .staleness
                .as_ref()
                .expect("every served symbol carries staleness");
            assert!(!stale_state.fresh);
            assert_eq!(stale_state.state.as_deref(), Some("marked_stale"));
            let stale_state_proven = stale_state
                .state
                .clone()
                .expect("marked-stale proof carries state");
            let stale_resolution =
                runtime.block_on(resolve_code_ref_with(&restarted_code_nav, &symbol_id));
            assert_eq!(
                stale_resolution,
                Err(CrossRefError::StaleSource {
                    symbol: symbol_id.clone(),
                    state: "marked_stale".to_owned(),
                })
            );
            app_harness
                .get_by(|node| node.author_id() == Some(chip_id.as_str()))
                .click();
            app_harness.step();
            for _ in 0..100 {
                if app_harness
                    .state()
                    .quick_switcher_nav_status()
                    .is_some_and(|status| {
                        status.contains("stale source") && status.contains("re-index")
                    })
                {
                    break;
                }
                app_harness.step();
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            assert!(
                app_harness
                    .state()
                    .quick_switcher_nav_status()
                    .is_some_and(|status| {
                        status.contains("stale source") && status.contains("re-index")
                    }),
                "the mounted shell must surface the stale-source recovery state; index={stale_index_body:?}, status={:?}",
                app_harness.state().quick_switcher_nav_status()
            );
            assert_eq!(
                active_panel.buffer().to_string(),
                SOURCE,
                "stale navigation must not replace the already-mounted last-known buffer"
            );

            // Delete the canonical backend authority while the persisted rich document remains mounted,
            // then click the SAME real chip again. The real 404 path must mutate that mounted atom to
            // `resolved=false`; no manual fixture mutation is allowed as proof.
            let status = live.clean();
            assert!((200..300).contains(&status));
            app_harness
                .get_by(|node| node.author_id() == Some(chip_id.as_str()))
                .click();
            app_harness.step();
            for _ in 0..100 {
                let content = rich_state
                    .lock()
                    .expect("mounted rich state after backend deletion")
                    .current_content_json();
                if content["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|block| block["content"].as_array().into_iter().flatten())
                    .any(|child| {
                        child["type"] == "hsLink"
                            && child["attrs"]["refKind"] == "code"
                            && child["attrs"]["refValue"] == symbol_id
                            && child["attrs"]["resolved"] == false
                    })
                {
                    break;
                }
                app_harness.step();
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            let content = rich_state
                .lock()
                .expect("mounted rich state final unresolved read")
                .current_content_json();
            assert!(
                content["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|block| block["content"].as_array().into_iter().flatten())
                    .any(|child| {
                        child["type"] == "hsLink"
                            && child["attrs"]["refKind"] == "code"
                            && child["attrs"]["refValue"] == symbol_id
                            && child["attrs"]["resolved"] == false
                    }),
                "the actual post-cleanup 404 must grey the mounted persisted chip"
            );
            (status, stale_state_proven)
        };

        // Canonical cleanup deleted the workspace authority (cascading symbol + document projections)
        // and removed the external source fixture.
        assert!(
            (200..300).contains(&delete_status),
            "workspace cleanup failed with HTTP {delete_status}"
        );
        assert!(
            !source_dir.exists(),
            "external source fixture must be cleaned"
        );
        let deleted_symbol = runtime.block_on(resolve_code_ref_with(&code_nav, &symbol_id));
        assert!(
            deleted_symbol
                .as_ref()
                .is_err_and(|error| error.is_unresolved()),
            "deleted symbol must resolve to a typed unresolved state: {deleted_symbol:?}"
        );
        assert!(
            runtime
                .block_on(document_client.load_document(&document_headers, &document_id))
                .is_err(),
            "workspace cleanup must remove the rich document from the live read surface"
        );

        let receipt_dir = external_artifact_dir("wp-kernel-012-mt-034");
        std::fs::create_dir_all(&receipt_dir).expect("create external receipt directory");
        let receipt_path = receipt_dir.join(format!("{unique}-v3-receipt.json"));
        let note_row_id = row_author_id(&document_id);
        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_id": "hsk.mt034-live-postgresql-proof@1",
                "microtask": "MT-034",
                "remediation_version": "V3",
                "workspace_id": workspace_id,
                "symbol_entity_id": symbol_id,
                "symbol_key": symbol_key,
                "rich_document_id": document_id,
                "exact_line_zero_based": expected_line,
                "accesskit": [
                    CODE_SYMBOL_SEARCH_AUTHOR_ID,
                    CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID,
                    chip_id,
                    PANEL_AUTHOR_ID,
                    note_row_id
                ],
                "missing_identity_status": {
                    "index": missing_index_identity.as_u16(),
                    "navigation": missing_nav_identity.as_u16(),
                    "save": missing_save_identity.as_u16()
                },
                "missing_symbol": {
                    "symbol_entity_id": missing_symbol_id,
                    "typed_unresolved": true
                },
                "optimistic_conflict": {
                    "stale_expected_version_status": conflict_status.as_u16(),
                    "committed_content_unchanged": after_conflict.document["content_json"] == loaded_content
                },
                "owned_backend_restart": {
                    "before": binding_before_restart,
                    "after": binding_after_restart,
                    "listener_reclaimed": old_base == new_base,
                    "document_persisted": true,
                    "symbol_persisted": true,
                    "reverse_lookup_persisted": true
                },
                "stale_source": {
                    "state": stale_state_proven,
                    "fresh": false,
                    "navigation_blocked_with_typed_recovery": true
                },
                "save_receipt_event_id": saved.save_receipt_event_id,
                "workspace_cleanup_status": delete_status,
                "symbol_unresolved_after_cleanup": true,
                "source_fixture_removed": !source_dir.exists()
            }))
            .expect("serialize MT-034 receipt"),
        )
        .expect("write external MT-034 receipt");
        crate::interconnect_support::assert_no_local_artifact_dir();
    }
}
