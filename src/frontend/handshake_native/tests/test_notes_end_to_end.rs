//! WP-KERNEL-012 MT-099 — Notes end-to-end usability proof.
//!
//! This test targets the operator complaint directly: open a knowledge document, see its backend content
//! in the mounted Notes editor, edit the live editor, save through `/knowledge/documents/:id/save`, then
//! reopen the same document and prove a fresh `GET /knowledge/documents/:id` returns and renders the
//! saved edit. The mock is route-shaped HTTP only; it does not call app internals or fake editor success.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::backend_client::HealthInfo;
use handshake_native::command_registry::CMD_EDITOR_FILE_SAVE;
use handshake_native::pane_registry::{PaneId, PaneType};
use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};

const DOC_ID: &str = "KRD-mt099-note";
const SECOND_DOC_ID: &str = "KRD-mt099-note-b";
const INITIAL_TEXT: &str = "server initial note";
const SECOND_INITIAL_TEXT: &str = "server second note";
const REFRESHED_TEXT: &str = "server refreshed note";
const EDIT_PREFIX: &str = "MT-099 edited ";
const SECOND_EDIT_PREFIX: &str = "MT-099 second edited ";
const SAVED_TEXT: &str = "MT-099 edited server initial note";
const SECOND_SAVED_TEXT: &str = "MT-099 second edited server second note";

static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
    headers_raw: String,
    body: String,
}

struct NotesMockServer {
    base_url: String,
    stop: Arc<AtomicBool>,
    started_requests: Arc<Mutex<Vec<RecordedRequest>>>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    state: Arc<Mutex<ServerState>>,
    handle: std::thread::JoinHandle<()>,
}

impl NotesMockServer {
    fn spawn() -> Self {
        Self::spawn_with_first_get_delays(HashMap::new())
    }

    fn spawn_with_first_get_delays(get_delays: HashMap<String, Duration>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind notes mock server");
        listener
            .set_nonblocking(true)
            .expect("set notes mock server nonblocking");
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let started_requests = Arc::new(Mutex::new(Vec::new()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let get_delays = Arc::new(get_delays);
        let get_counts = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
        let child_handles = Arc::new(Mutex::new(Vec::new()));
        let mut docs = HashMap::new();
        docs.insert(
            DOC_ID.to_owned(),
            ServerDocState {
                content_json: paragraph_doc(INITIAL_TEXT),
                doc_version: 7,
                title: "MT-099 note".to_owned(),
            },
        );
        docs.insert(
            SECOND_DOC_ID.to_owned(),
            ServerDocState {
                content_json: paragraph_doc(SECOND_INITIAL_TEXT),
                doc_version: 13,
                title: "MT-099 second note".to_owned(),
            },
        );
        let state = Arc::new(Mutex::new(ServerState { docs }));
        let stop_for_thread = Arc::clone(&stop);
        let started_requests_for_thread = Arc::clone(&started_requests);
        let requests_for_thread = Arc::clone(&requests);
        let state_for_thread = Arc::clone(&state);
        let get_delays_for_thread = Arc::clone(&get_delays);
        let get_counts_for_thread = Arc::clone(&get_counts);
        let child_handles_for_thread = Arc::clone(&child_handles);
        let handle = std::thread::spawn(move || {
            while !stop_for_thread.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let started_requests = Arc::clone(&started_requests_for_thread);
                        let requests = Arc::clone(&requests_for_thread);
                        let state = Arc::clone(&state_for_thread);
                        let get_delays = Arc::clone(&get_delays_for_thread);
                        let get_counts = Arc::clone(&get_counts_for_thread);
                        let child = std::thread::spawn(move || {
                            if let Some(request) = read_request(&mut stream) {
                                let response = route_request(&request, &state);
                                started_requests.lock().unwrap().push(request.clone());
                                if let Some(document_id) =
                                    document_id_from_plain_path(&request.path)
                                {
                                    let request_index = {
                                        let mut counts = get_counts.lock().unwrap();
                                        let count = counts.entry(document_id.clone()).or_insert(0);
                                        let index = *count;
                                        *count += 1;
                                        index
                                    };
                                    if request_index == 0 {
                                        if let Some(delay) = get_delays.get(&document_id) {
                                            std::thread::sleep(*delay);
                                        }
                                    }
                                }
                                requests.lock().unwrap().push(request);
                                write_json(&mut stream, response.0, response.1);
                            }
                        });
                        child_handles_for_thread.lock().unwrap().push(child);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
            for child in child_handles_for_thread.lock().unwrap().drain(..) {
                let _ = child.join();
            }
        });
        Self {
            base_url,
            stop,
            started_requests,
            requests,
            state,
            handle,
        }
    }

    fn started_requests(&self) -> Vec<RecordedRequest> {
        self.started_requests.lock().unwrap().clone()
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }

    fn set_document_text(&self, document_id: &str, text: &str, doc_version: u64) {
        let mut state = self.state.lock().unwrap();
        let doc = state
            .docs
            .get_mut(document_id)
            .expect("test document exists");
        doc.content_json = paragraph_doc(text);
        doc.doc_version = doc_version;
    }

    fn shutdown(self) -> Vec<RecordedRequest> {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.base_url.strip_prefix("http://").unwrap_or(""));
        let _ = self.handle.join();
        self.requests.lock().unwrap().clone()
    }
}

struct ServerDocState {
    content_json: serde_json::Value,
    doc_version: u64,
    title: String,
}

struct ServerState {
    docs: HashMap<String, ServerDocState>,
}

fn route_request(
    request: &RecordedRequest,
    state: &Arc<Mutex<ServerState>>,
) -> (&'static str, serde_json::Value) {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", path) if document_id_from_plain_path(path).is_some() => {
            let document_id = document_id_from_plain_path(path).unwrap();
            let state = state.lock().unwrap();
            let Some(doc) = state.docs.get(&document_id) else {
                return missing_document_response(&document_id);
            };
            (
                "HTTP/1.1 200 OK",
                serde_json::json!({
                    "document": document_record(&document_id, doc),
                    "tree": {
                        "schema_version": "rich_document_v1",
                        "schema_matches": true,
                        "block_ids": [],
                        "blocks": []
                    },
                    "code_nodes": []
                }),
            )
        }
        ("GET", path) if document_id_from_suffixed_path(path, "/draft").is_some() => {
            let document_id = document_id_from_suffixed_path(path, "/draft").unwrap();
            let state = state.lock().unwrap();
            let Some(doc) = state.docs.get(&document_id) else {
                return missing_document_response(&document_id);
            };
            (
                "HTTP/1.1 200 OK",
                serde_json::json!({
                    "rich_document_id": document_id,
                    "current_doc_version": doc.doc_version,
                    "current_content_sha256": "mock-sha",
                    "draft": null
                }),
            )
        }
        ("PUT", path) if document_id_from_suffixed_path(path, "/save").is_some() => {
            let document_id = document_id_from_suffixed_path(path, "/save").unwrap();
            let body: serde_json::Value =
                serde_json::from_str(&request.body).expect("save body is JSON");
            let content_json = body
                .get("content_json")
                .cloned()
                .expect("save body carries content_json");
            let mut state = state.lock().unwrap();
            let Some(doc) = state.docs.get_mut(&document_id) else {
                return missing_document_response(&document_id);
            };
            doc.content_json = content_json;
            doc.doc_version += 1;
            (
                "HTTP/1.1 200 OK",
                serde_json::json!({
                    "document": document_record(&document_id, doc),
                    "save_receipt_event_id": "EVT-mt099-save",
                    "backlinks_persisted": 0,
                    "embeds_persisted": 0,
                    "knowledge_indexed": true
                }),
            )
        }
        ("DELETE", path) if document_id_from_suffixed_path(path, "/draft").is_some() => (
            "HTTP/1.1 200 OK",
            serde_json::json!({
                "rich_document_id": document_id_from_suffixed_path(path, "/draft").unwrap(),
                "draft": null,
                "cleared": true
            }),
        ),
        _ => (
            "HTTP/1.1 404 Not Found",
            serde_json::json!({ "detail": format!("unexpected route {} {}", request.method, request.path) }),
        ),
    }
}

fn document_id_from_plain_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/knowledge/documents/")?;
    (!rest.is_empty() && !rest.contains('/')).then(|| rest.to_owned())
}

fn document_id_from_suffixed_path(path: &str, suffix: &str) -> Option<String> {
    let rest = path.strip_prefix("/knowledge/documents/")?;
    let document_id = rest.strip_suffix(suffix)?;
    (!document_id.is_empty() && !document_id.contains('/')).then(|| document_id.to_owned())
}

fn missing_document_response(document_id: &str) -> (&'static str, serde_json::Value) {
    (
        "HTTP/1.1 404 Not Found",
        serde_json::json!({ "detail": format!("unknown document {document_id}") }),
    )
}

fn document_record(document_id: &str, doc: &ServerDocState) -> serde_json::Value {
    serde_json::json!({
        "rich_document_id": document_id,
        "doc_version": doc.doc_version,
        "title": doc.title.clone(),
        "content_json": doc.content_json.clone(),
        "crdt_document_id": null,
        "updated_at": "2026-06-29T10:00:00Z"
    })
}

fn paragraph_doc(text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": text }]
        }]
    })
}

fn read_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_header_end(&buf) {
            break pos;
        }
    };
    let headers_bytes = &buf[..header_end];
    let headers_raw = String::from_utf8_lossy(headers_bytes).to_string();
    let content_length = headers_raw
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let body_start = header_end + 4;
    while buf.len().saturating_sub(body_start) < content_length {
        let n = stream.read(&mut tmp).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let body = String::from_utf8_lossy(
        &buf[body_start..body_start + content_length.min(buf.len().saturating_sub(body_start))],
    )
    .to_string();
    let request_line = headers_raw.lines().next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.to_owned();
    Some(RecordedRequest {
        method,
        path,
        headers_raw,
        body,
    })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn write_json(stream: &mut TcpStream, status_line: &str, body: serde_json::Value) {
    let body = body.to_string();
    let response = format!(
        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn ok_app() -> HandshakeApp {
    HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }))
}

fn wait_for_text(harness: &mut Harness<'_, HandshakeApp>, expected: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        harness.step();
        if harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .block_plain_text(0)
            .as_deref()
            == Some(expected)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let current = harness
        .state()
        .mounted_rich_state()
        .lock()
        .unwrap()
        .block_plain_text(0);
    panic!("timed out waiting for editor text {expected:?}; current={current:?}");
}

fn wait_for_requests<F>(server: &NotesMockServer, pred: F, timeout: Duration)
where
    F: Fn(&[RecordedRequest]) -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let requests = server.requests();
        if pred(&requests) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "timed out waiting for expected HTTP requests; got {:?}",
        server.requests()
    );
}

fn wait_for_started_requests<F>(server: &NotesMockServer, pred: F, timeout: Duration)
where
    F: Fn(&[RecordedRequest]) -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let requests = server.started_requests();
        if pred(&requests) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "timed out waiting for expected started HTTP requests; got {:?}",
        server.started_requests()
    );
}

fn focus_rich_editor_surface(harness: &mut Harness<'_, HandshakeApp>) {
    let root = harness.root();
    let surface = root
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("rich-editor-surface"))
        .expect("rich editor surface is present");
    surface.focus();
    harness.step();
    harness.step();
}

fn document_get_count(requests: &[RecordedRequest]) -> usize {
    document_get_count_for(requests, DOC_ID)
}

fn document_get_count_for(requests: &[RecordedRequest], document_id: &str) -> usize {
    let doc_path = format!("/knowledge/documents/{document_id}");
    requests
        .iter()
        .filter(|r| r.method == "GET" && r.path == doc_path)
        .count()
}

fn save_request_count_for(requests: &[RecordedRequest], document_id: &str) -> usize {
    let save_path = format!("/knowledge/documents/{document_id}/save");
    requests
        .iter()
        .filter(|r| r.method == "PUT" && r.path == save_path)
        .count()
}

fn assert_notes_opened_in_seeded_notes_pane(harness: &Harness<'_, HandshakeApp>) {
    let pane_a = PaneId::from("pane-a");
    let pane_b = PaneId::from("pane-b");
    let bars = harness.state().tab_bar_states();
    assert_eq!(
        bars.get(&pane_a)
            .and_then(|bar| bar.active())
            .map(|tab| &tab.pane_type),
        Some(&PaneType::CodeSymbol),
        "fresh document open keeps the seeded Code pane intact"
    );
    let notes_tab = bars
        .get(&pane_b)
        .and_then(|bar| bar.active())
        .expect("seeded Notes pane has an active tab");
    assert_eq!(
        notes_tab.pane_type,
        PaneType::LoomWikiPage,
        "fresh document open targets the seeded Notes pane"
    );
    assert_eq!(
        notes_tab.content_id.as_deref(),
        Some(DOC_ID),
        "seeded Notes pane carries the opened document id"
    );
}

fn assert_single_notes_pane(harness: &Harness<'_, HandshakeApp>) {
    let notes_panes: Vec<_> = harness
        .state()
        .tab_bar_states()
        .iter()
        .filter(|(_, bar)| {
            bar.tabs
                .iter()
                .any(|tab| tab.pane_type == PaneType::LoomWikiPage)
        })
        .map(|(pane_id, _)| pane_id.clone())
        .collect();
    assert_eq!(
        notes_panes,
        vec![PaneId::from("pane-b")],
        "document opens reuse the seeded singleton Notes pane instead of duplicating rich-editor panes"
    );
}

#[test]
fn open_edit_save_reopen_round_trips_through_knowledge_documents() {
    let _wgpu_guard = wgpu_guard();
    let server = NotesMockServer::spawn();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());

    let opened = app.open_document(DOC_ID);
    assert!(
        matches!(opened, NavDispatchOutcome::Opened { .. }),
        "ShellNavigator opens the mounted Notes editor; got {opened:?}"
    );

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    wait_for_text(&mut harness, INITIAL_TEXT, Duration::from_secs(5));
    assert_notes_opened_in_seeded_notes_pane(&harness);
    wait_for_requests(
        &server,
        |requests| document_get_count(requests) >= 1,
        Duration::from_secs(2),
    );

    focus_rich_editor_surface(&mut harness);
    harness.event(egui::Event::Text(EDIT_PREFIX.to_owned()));
    harness.step();
    harness.step();
    assert_eq!(
        harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .block_plain_text(0)
            .as_deref(),
        Some(SAVED_TEXT),
        "typing edits the mounted live Notes editor state"
    );

    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test(CMD_EDITOR_FILE_SAVE),
        "File > Save dispatch reaches the editor save path"
    );
    wait_for_requests(
        &server,
        |requests| {
            requests.iter().any(|r| {
                r.method == "PUT" && r.path == format!("/knowledge/documents/{DOC_ID}/save")
            })
        },
        Duration::from_secs(5),
    );
    harness.step();
    harness.step();

    assert!(
        matches!(
            harness.state_mut().open_document(DOC_ID),
            NavDispatchOutcome::Opened { .. }
        ),
        "reopening the same note routes through ShellNavigator again"
    );
    harness.step();
    harness.step();
    wait_for_requests(
        &server,
        |requests| document_get_count(requests) >= 2,
        Duration::from_secs(5),
    );
    wait_for_text(&mut harness, SAVED_TEXT, Duration::from_secs(5));

    match harness.render() {
        Ok(image) => {
            assert!(
                image.width() > 0 && image.height() > 0,
                "rendered image is non-empty"
            );
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-099");
            let _ = std::fs::create_dir_all(&ext_dir);
            let path = ext_dir.join("mt099-notes-e2e.png");
            image.save(&path).expect("save MT-099 Notes screenshot");
            println!(
                "PT-099 notes screenshot: {}x{} ({})",
                image.width(),
                image.height(),
                path.display()
            );
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): MT-099 Notes screenshot render unavailable: {e}. The HTTP + \
             AccessKit/editor-state proof passed."
        ),
    }

    let requests = server.shutdown();
    let doc_path = format!("/knowledge/documents/{DOC_ID}");
    let save_path = format!("/knowledge/documents/{DOC_ID}/save");
    let doc_gets: Vec<_> = requests
        .iter()
        .filter(|r| r.method == "GET" && r.path == doc_path)
        .collect();
    assert!(
        doc_gets.len() >= 2,
        "open and reopen both issue authoritative document GETs; requests={requests:?}"
    );
    let save = requests
        .iter()
        .find(|r| r.method == "PUT" && r.path == save_path)
        .expect("canonical save PUT was captured");
    let headers = save.headers_raw.to_lowercase();
    for required in [
        "x-hsk-actor-id:",
        "x-hsk-actor-kind:",
        "x-hsk-kernel-task-run-id:",
        "x-hsk-session-run-id:",
    ] {
        assert!(
            headers.contains(required),
            "save request carries required identity header {required}; headers={}",
            save.headers_raw
        );
    }
    let save_body: serde_json::Value = serde_json::from_str(&save.body).expect("save body JSON");
    assert_eq!(
        save_body.get("expected_version").and_then(|v| v.as_i64()),
        Some(7),
        "save uses the loaded document version, not hardcoded zero"
    );
    assert_eq!(
        text_from_doc(save_body.get("content_json").unwrap()),
        SAVED_TEXT,
        "save body carries the live edited document content"
    );
}

#[test]
fn switching_notes_does_not_save_with_stale_document_context() {
    let _wgpu_guard = wgpu_guard();
    let server = NotesMockServer::spawn();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());

    assert!(
        matches!(
            app.open_code_symbol("SYM-mt099-code"),
            NavDispatchOutcome::Opened { .. }
        ),
        "seed the active pane as Code before opening a note"
    );
    assert!(
        matches!(app.open_document(DOC_ID), NavDispatchOutcome::Opened { .. }),
        "opening a note from Code routes to the mounted Notes pane"
    );

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    wait_for_text(&mut harness, INITIAL_TEXT, Duration::from_secs(5));
    assert_single_notes_pane(&harness);

    assert!(
        matches!(
            harness.state_mut().open_document(SECOND_DOC_ID),
            NavDispatchOutcome::Opened { .. }
        ),
        "switch to a second note through the same mounted Notes pane"
    );
    let _ = harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_EDITOR_FILE_SAVE);
    std::thread::sleep(Duration::from_millis(150));
    harness.step();
    let requests_after_early_save = server.requests();
    assert_eq!(
        save_request_count_for(&requests_after_early_save, DOC_ID),
        0,
        "saving before the second note loads must not use the first note's stale SaveManager"
    );
    assert_eq!(
        save_request_count_for(&requests_after_early_save, SECOND_DOC_ID),
        0,
        "saving before the active note loads must wait for an authoritative GET/version"
    );

    wait_for_text(&mut harness, SECOND_INITIAL_TEXT, Duration::from_secs(5));
    assert_single_notes_pane(&harness);
    focus_rich_editor_surface(&mut harness);
    harness.event(egui::Event::Text(SECOND_EDIT_PREFIX.to_owned()));
    harness.step();
    assert_eq!(
        harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .block_plain_text(0)
            .as_deref(),
        Some(SECOND_SAVED_TEXT),
        "the second active note receives the live editor edit"
    );

    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test(CMD_EDITOR_FILE_SAVE),
        "File > Save dispatch reaches the active note save path"
    );
    wait_for_requests(
        &server,
        |requests| save_request_count_for(requests, SECOND_DOC_ID) == 1,
        Duration::from_secs(5),
    );
    let requests = server.shutdown();
    assert_eq!(
        save_request_count_for(&requests, DOC_ID),
        0,
        "the stale first note is never saved during the second-note flow"
    );
    let save = requests
        .iter()
        .find(|r| {
            r.method == "PUT" && r.path == format!("/knowledge/documents/{SECOND_DOC_ID}/save")
        })
        .expect("second-note canonical save PUT was captured");
    let body: serde_json::Value = serde_json::from_str(&save.body).expect("save body JSON");
    assert_eq!(
        body.get("expected_version").and_then(|v| v.as_i64()),
        Some(13),
        "second-note save uses that note's loaded version"
    );
    assert_eq!(
        text_from_doc(body.get("content_json").unwrap()),
        SECOND_SAVED_TEXT,
        "second-note save body carries the active editor content"
    );
}

#[test]
fn out_of_order_note_gets_keep_current_document_load() {
    let _wgpu_guard = wgpu_guard();
    let server = NotesMockServer::spawn_with_first_get_delays(HashMap::from([(
        DOC_ID.to_owned(),
        Duration::from_millis(180),
    )]));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());
    assert!(
        matches!(app.open_document(DOC_ID), NavDispatchOutcome::Opened { .. }),
        "opening the first note starts an authoritative GET"
    );

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    assert!(
        matches!(
            harness.state_mut().open_document(SECOND_DOC_ID),
            NavDispatchOutcome::Opened { .. }
        ),
        "switching notes before the first GET returns starts the second note load"
    );
    harness.step();

    // Let B complete first and delayed A complete second without a UI drain in between. The old
    // single-slot delivery cell lost B here; the FIFO queue must keep and apply the current B result.
    std::thread::sleep(Duration::from_millis(260));
    harness.step();
    wait_for_text(&mut harness, SECOND_INITIAL_TEXT, Duration::from_secs(5));
    assert_single_notes_pane(&harness);

    let requests = server.shutdown();
    assert!(
        document_get_count_for(&requests, DOC_ID) >= 1,
        "the stale first note GET was issued"
    );
    assert!(
        document_get_count_for(&requests, SECOND_DOC_ID) >= 1,
        "the current second note GET was issued and remained deliverable"
    );
}

#[test]
fn same_document_reopen_ignores_stale_get_generation() {
    let _wgpu_guard = wgpu_guard();
    let server = NotesMockServer::spawn_with_first_get_delays(HashMap::from([(
        DOC_ID.to_owned(),
        Duration::from_millis(220),
    )]));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());
    assert!(
        matches!(app.open_document(DOC_ID), NavDispatchOutcome::Opened { .. }),
        "opening the note starts the first authoritative GET"
    );

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    wait_for_started_requests(
        &server,
        |requests| document_get_count_for(requests, DOC_ID) >= 1,
        Duration::from_secs(2),
    );
    server.set_document_text(DOC_ID, REFRESHED_TEXT, 31);

    assert!(
        matches!(
            harness.state_mut().open_document(DOC_ID),
            NavDispatchOutcome::Opened { .. }
        ),
        "reopening the same note invalidates the old in-flight GET generation"
    );
    harness.step();
    wait_for_started_requests(
        &server,
        |requests| document_get_count_for(requests, DOC_ID) >= 2,
        Duration::from_secs(2),
    );

    // Let the fresh same-document GET complete first and the old delayed same-document GET complete after
    // it. The load generation must keep the old initial-content response from applying as current.
    std::thread::sleep(Duration::from_millis(300));
    harness.step();
    wait_for_text(&mut harness, REFRESHED_TEXT, Duration::from_secs(5));

    let requests = server.shutdown();
    assert!(
        document_get_count_for(&requests, DOC_ID) >= 2,
        "same-document reopen issues a fresh authoritative GET"
    );
}

fn text_from_doc(value: &serde_json::Value) -> String {
    value["content"][0]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_owned()
}
