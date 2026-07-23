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
    io::{ErrorKind, Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

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

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E). Used by the `wgpu_screenshots`-
/// gated screenshot test; `#[allow(dead_code)]` so the default (no-feature) build does not warn.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
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
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
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
    listener
        .set_nonblocking(true)
        .expect("nonblocking MT-034 code-nav mock server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let file_path = file_path.to_owned();
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(pair) => break pair,
                Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return "NO_REQUEST".to_owned(),
                Err(e) => return format!("ACCEPT_ERROR:{e}"),
            }
        };
        stream
            .set_nonblocking(false)
            .expect("blocking accepted code-symbol stream");
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
    listener
        .set_nonblocking(true)
        .expect("nonblocking MT-034 lookup mock server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let file_path = file_path.to_owned();
    let handle = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(pair) => break pair,
                Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => return "NO_REQUEST".to_owned(),
                Err(e) => return format!("ACCEPT_ERROR:{e}"),
            }
        };
        stream
            .set_nonblocking(false)
            .expect("blocking accepted lookup stream");
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
    assert!(!CrossRefError::Backend("down".into()).is_unresolved());
    println!("AC-4: resolve errors classify NotFound/NoDefinition/EmptySymbol as unresolved");
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

    use super::role_for;
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;

    use handshake_native::app::HandshakeApp;
    use handshake_native::backend::knowledge_documents::{
        CreateDocumentRequest, HskDocumentHeaders, KnowledgeDocumentsClient, SaveDocumentRequest,
    };
    use handshake_native::code_editor::code_nav::CodeNavClient;
    use handshake_native::code_editor::note_refs_panel::{
        row_author_id, NoteRefsState, PANEL_AUTHOR_ID,
    };
    use handshake_native::code_editor::panel::CodeEditorPanel;
    use handshake_native::interop::cross_ref::{
        find_code_ref_notes_with, resolve_code_ref_with, FindNotesHttp, FindNotesSearch,
    };
    use handshake_native::rich_editor::document_model::doc_json::from_json_string;
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

    use super::{author_ids, code_note_editor_shell, external_artifact_dir};

    const SYMBOL_NAME: &str = "Mt034ExactSymbol";
    const SOURCE_FILE: &str = "mt034_exact_symbol.rs";
    const SOURCE: &str = "// MT-034 managed PostgreSQL fixture\n\npub struct Mt034ExactSymbol {\n    pub value: i32,\n}\n\nimpl Mt034ExactSymbol {\n    pub fn new(value: i32) -> Self { Self { value } }\n}\n";

    struct ManagedCleanup<'a> {
        backend: &'a crate::interconnect_support::LiveBackend,
        workspace_id: String,
        source_dir: std::path::PathBuf,
        cleaned: bool,
    }

    impl ManagedCleanup<'_> {
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

    impl Drop for ManagedCleanup<'_> {
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
        let live = crate::interconnect_support::require_reachable_backend();
        let unique = format!(
            "mt034-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after unix epoch")
                .as_nanos()
        );
        let workspace = live.create_workspace(&unique);
        let workspace_id = workspace["id"]
            .as_str()
            .expect("workspace create returns id")
            .to_owned();

        let source_dir =
            external_artifact_dir("wp-kernel-012-mt-034").join(format!("source-{unique}"));
        // Install cleanup as soon as the managed workspace identity exists. Every subsequent
        // filesystem/runtime/indexing assertion is fallible, so unwinding from any of them must still
        // delete the workspace and the (possibly only partially created) external source directory.
        let mut cleanup = ManagedCleanup {
            backend: &live,
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
        let reloaded_doc =
            from_json_string(&loaded_content.to_string()).expect("parse reloaded DocJson");

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

        let note_panel = Arc::new(CodeEditorPanel::new(SOURCE, "rs"));
        note_panel.set_file_path(source_path.to_string_lossy());
        note_panel.set_runtime(runtime.handle().clone());
        note_panel.set_workspace_id(workspace_id.clone());
        note_panel.set_code_nav_client(code_nav.clone());
        let find_backend: Arc<dyn FindNotesSearch> = Arc::new(find_notes.clone());
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
        let delete_status = {
            let (mut app, _app_runtime) = code_note_editor_shell();
            app.install_mounted_code_nav_client_for_test(code_nav.clone());
            let rich_state = app.mounted_rich_state();
            rich_state.lock().expect("mounted rich state").doc = reloaded_doc.clone();
            let code_panel = app.mounted_code_panel();
            code_panel.set_text("pub fn unrelated() {}\n");
            code_panel.set_file_path(unrelated_path.to_string_lossy().to_string());
            let mut app_harness = Harness::builder()
                .with_size(egui::vec2(1100.0, 700.0))
                .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
            app_harness.run_steps(3);
            assert!(author_ids(&app_harness).contains(&chip_id));
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

            // Delete the canonical backend authority while the persisted rich document remains mounted,
            // then click the SAME real chip again. The real 404 path must mutate that mounted atom to
            // `resolved=false`; no manual fixture mutation is allowed as proof.
            let status = cleanup.clean();
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
            status
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
        let receipt_path = receipt_dir.join(format!("{unique}-v2-receipt.json"));
        let note_row_id = row_author_id(&document_id);
        std::fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "microtask": "MT-034",
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
