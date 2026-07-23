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

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{
    HandshakeApp, HealthDisplayState, NOTES_LOAD_ERROR_AUTHOR_ID, NOTES_LOAD_RETRY_AUTHOR_ID,
};
use handshake_native::backend_client::HealthInfo;
use handshake_native::command_registry::{
    CMD_EDITOR_FILE_NEW, CMD_EDITOR_FILE_SAVE, CMD_VIEW_OUTLINE,
};
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
        Self::spawn_scripted(HashMap::new(), HashMap::new())
    }

    fn spawn_with_first_get_delays(get_delays: HashMap<String, Duration>) -> Self {
        Self::spawn_scripted(get_delays, HashMap::new())
    }

    fn spawn_with_plain_get_failures(get_failures: HashMap<String, usize>) -> Self {
        Self::spawn_scripted(HashMap::new(), get_failures)
    }

    fn spawn_scripted(
        get_delays: HashMap<String, Duration>,
        get_failures: HashMap<String, usize>,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind notes mock server");
        listener
            .set_nonblocking(true)
            .expect("set notes mock server nonblocking");
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let started_requests = Arc::new(Mutex::new(Vec::new()));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let get_delays = Arc::new(get_delays);
        let get_failures = Arc::new(get_failures);
        let get_counts = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
        let child_handles = Arc::new(Mutex::new(Vec::new()));
        let mut docs = HashMap::new();
        docs.insert(
            DOC_ID.to_owned(),
            ServerDocState {
                content_json: paragraph_doc(INITIAL_TEXT),
                doc_version: 7,
                title: "MT-099 note".to_owned(),
                draft_text: None,
            },
        );
        docs.insert(
            SECOND_DOC_ID.to_owned(),
            ServerDocState {
                content_json: paragraph_doc(SECOND_INITIAL_TEXT),
                doc_version: 13,
                title: "MT-099 second note".to_owned(),
                draft_text: None,
            },
        );
        let state = Arc::new(Mutex::new(ServerState { docs }));
        let stop_for_thread = Arc::clone(&stop);
        let started_requests_for_thread = Arc::clone(&started_requests);
        let requests_for_thread = Arc::clone(&requests);
        let state_for_thread = Arc::clone(&state);
        let get_delays_for_thread = Arc::clone(&get_delays);
        let get_failures_for_thread = Arc::clone(&get_failures);
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
                        let get_failures = Arc::clone(&get_failures_for_thread);
                        let get_counts = Arc::clone(&get_counts_for_thread);
                        let child = std::thread::spawn(move || {
                            if let Some(request) = read_request(&mut stream) {
                                started_requests.lock().unwrap().push(request.clone());
                                let plain_get = (request.method == "GET")
                                    .then(|| document_id_from_plain_path(&request.path))
                                    .flatten()
                                    .map(|document_id| {
                                        let mut counts = get_counts.lock().unwrap();
                                        let count = counts.entry(document_id.clone()).or_insert(0);
                                        let index = *count;
                                        *count += 1;
                                        (document_id, index)
                                    });
                                if let Some((document_id, request_index)) = plain_get.as_ref() {
                                    if *request_index == 0 {
                                        if let Some(delay) = get_delays.get(document_id) {
                                            std::thread::sleep(*delay);
                                        }
                                    }
                                }
                                let response = match plain_get {
                                    Some((document_id, request_index))
                                        if request_index
                                            < get_failures
                                                .get(&document_id)
                                                .copied()
                                                .unwrap_or_default() =>
                                    {
                                        (
                                            "HTTP/1.1 503 Service Unavailable",
                                            serde_json::json!({
                                                "detail": format!(
                                                    "scripted Notes load failure for {document_id}"
                                                )
                                            }),
                                        )
                                    }
                                    _ => route_request(&request, &state),
                                };
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

    /// WP-KERNEL-012 MT-020: stage a persisted draft for `document_id` so the NEXT `GET /draft`
    /// (e.g. on a second open) serves a non-null draft based on the current server version.
    fn set_document_draft(&self, document_id: &str, text: &str) {
        let mut state = self.state.lock().unwrap();
        let doc = state
            .docs
            .get_mut(document_id)
            .expect("test document exists");
        doc.draft_text = Some(text.to_owned());
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
    /// WP-KERNEL-012 MT-020 (draft recovery): when set, `GET /draft` serves a NON-NULL persisted
    /// draft with this paragraph text, based on the CURRENT `doc_version` (the offerable shape the
    /// client re-checks defensively). `None` -> `draft: null` (the pre-existing MT-099 behavior).
    draft_text: Option<String>,
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
            // MT-020 draft recovery: serve the staged draft (offerable shape — based on the CURRENT
            // doc_version, carrying content) when one is set, else the pre-existing `draft: null`.
            let draft = match &doc.draft_text {
                Some(text) => serde_json::json!({
                    "base_doc_version": doc.doc_version,
                    "base_content_sha256": "mock-sha",
                    "draft_content_sha256": "mock-draft-sha",
                    "content_json": paragraph_doc(text)
                }),
                None => serde_json::Value::Null,
            };
            (
                "HTTP/1.1 200 OK",
                serde_json::json!({
                    "rich_document_id": document_id,
                    "current_doc_version": doc.doc_version,
                    "current_content_sha256": "mock-sha",
                    "draft": draft
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
        "workspace_id": "WS-mt099",
        "doc_version": doc.doc_version,
        "title": doc.title.clone(),
        "content_json": doc.content_json.clone(),
        "crdt_document_id": null,
        "authority_label": "draft",
        "owner_actor_kind": "operator",
        "owner_actor_id": "handshake-native-editor",
        "project_ref": "PRJ-mt099",
        "folder_ref": null,
        "created_at": "2026-06-29T09:00:00Z",
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

fn rich_doc_body(
    document_id: &str,
    text: &str,
    doc_version: u64,
) -> handshake_native::backend_client::RichDocBody {
    rich_doc_body_with_content(document_id, paragraph_doc(text), doc_version)
}

fn rich_doc_body_with_content(
    document_id: &str,
    content_json: serde_json::Value,
    doc_version: u64,
) -> handshake_native::backend_client::RichDocBody {
    handshake_native::backend_client::RichDocBody {
        document_id: document_id.to_owned(),
        workspace_id: "ws-mt099".to_owned(),
        doc_version,
        title: document_id.to_owned(),
        content_json,
        crdt_document_id: None,
        authority_label: "AUTHORITATIVE".to_owned(),
        owner_actor_kind: Some("operator".to_owned()),
        owner_actor_id: Some("operator".to_owned()),
        project_ref: None,
        folder_ref: None,
        created_at: "2026-07-17T00:00:00Z".to_owned(),
        updated_at: "2026-07-17T00:00:00Z".to_owned(),
    }
}

fn heading_doc(text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": [{
            "type": "heading",
            "attrs": { "level": 1 },
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
        .find(|n| n.accesskit_node().author_id() == Some("editor.rich.text"))
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
fn slow_note_get_shows_blank_non_editable_loading_surface_without_demo_content() {
    let server = NotesMockServer::spawn_with_first_get_delays(HashMap::from([(
        DOC_ID.to_owned(),
        Duration::from_millis(350),
    )]));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());
    assert!(matches!(
        app.open_document(DOC_ID),
        NavDispatchOutcome::Opened { .. }
    ));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();

    let state = harness
        .state()
        .mounted_rich_state_for_document_for_test(DOC_ID);
    assert_eq!(
        state.lock().unwrap().block_plain_text(0).as_deref(),
        Some("")
    );
    let loading_label = format!("Loading Notes document {DOC_ID}…");
    assert!(harness.query_all_by_label(&loading_label).count() >= 1);
    assert_eq!(
        harness.query_all_by_label("Heading One").count(),
        0,
        "the demo fixture is never rendered while the authoritative GET is pending"
    );
    let _ = server.shutdown();
}

#[test]
fn outline_retains_exact_launching_note_and_click_targets_that_notes_scroll() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test("http://127.0.0.1:9", runtime.handle().clone());
    assert!(matches!(
        app.open_document(DOC_ID),
        NavDispatchOutcome::Opened { .. }
    ));
    let pane_id = app.active_pane().expect("active Notes pane").to_string();
    app.apply_loaded_rich_document_to_view_for_test(
        &pane_id,
        rich_doc_body_with_content(DOC_ID, heading_doc("Exact Outline Heading"), 7),
    )
    .expect("install heading document");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_VIEW_OUTLINE));
    harness.step();
    let block_id = {
        let outline = harness.state().mounted_outline_panel_for_test();
        let outline = outline.lock().unwrap();
        assert_eq!(outline.roots[0].text, "Exact Outline Heading");
        outline.roots[0].block_id.clone()
    };
    let outline_author_id = format!("outline.heading.{block_id}");
    harness
        .get_by(|node| node.author_id() == Some(outline_author_id.as_str()))
        .click();
    harness.step();
    let rich = harness
        .state()
        .mounted_rich_state_for_view_for_test(&pane_id, DOC_ID);
    let rich = rich.lock().unwrap();
    assert!(
        rich.pending_scroll_block.as_deref() == Some(&[0][..])
            || rich.last_consumed_scroll_block.as_deref() == Some(&[0][..]),
        "the exact launching Notes view receives and either retains or consumes the heading scroll"
    );
    assert!(matches!(
        &rich.selection,
        handshake_native::rich_editor::document_model::selection::Selection::Text { anchor, head }
            if anchor.path.as_slice() == [0, 0]
                && anchor.char_offset == 0
                && head.path.as_slice() == [0, 0]
                && head.char_offset == "Exact Outline Heading".chars().count()
    ));
}

#[test]
fn workspace_switch_retires_rich_views_and_discards_stale_delivery() {
    let mut app = ok_app();
    app.set_active_project_id_for_test("workspace-a");
    assert!(app.open_document_in_pane_for_test("pane-b", DOC_ID));
    let old = app.mounted_rich_state_for_view_for_test("pane-b", DOC_ID);
    old.lock().unwrap().doc =
        handshake_native::rich_editor::document_model::node::BlockNode::doc(vec![
            handshake_native::rich_editor::document_model::node::BlockNode::paragraph(
                "workspace A",
            ),
        ]);
    app.queue_rich_document_load_result_for_test(
        DOC_ID,
        Err("stale workspace A delivery".to_owned()),
    );
    app.set_active_project_id_for_test("workspace-b");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    let new = harness
        .state()
        .mounted_rich_state_for_view_for_test("pane-b", DOC_ID);
    assert!(!Arc::ptr_eq(&old, &new));
    assert_ne!(
        new.lock().unwrap().block_plain_text(0).as_deref(),
        Some("workspace A")
    );
    assert!(harness
        .state()
        .rich_document_load_failure_for_test(DOC_ID)
        .is_some());
    assert!(!harness
        .state()
        .rich_document_load_failure_for_test(DOC_ID)
        .unwrap()
        .contains("stale workspace A delivery"));
}

#[test]
fn same_document_split_views_share_document_save_authority_but_keep_view_state_and_ids_independent()
{
    let server = NotesMockServer::spawn();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());
    assert!(app.open_document_in_pane_for_test("pane-a", DOC_ID));
    app.apply_loaded_rich_document_to_view_for_test("pane-a", rich_doc_body(DOC_ID, "pane A", 7))
        .expect("load pane A view");
    assert!(app.open_document_in_pane_for_test("pane-b", DOC_ID));
    app.apply_loaded_rich_document_to_view_for_test("pane-b", rich_doc_body(DOC_ID, "pane B", 7))
        .expect("load pane B view");
    let first = app.mounted_rich_state_for_view_for_test("pane-a", DOC_ID);
    let second = app.mounted_rich_state_for_view_for_test("pane-b", DOC_ID);
    assert!(!Arc::ptr_eq(&first, &second));
    assert_ne!(
        first.lock().unwrap().accessibility_namespace,
        second.lock().unwrap().accessibility_namespace
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    let first = harness
        .state()
        .mounted_rich_state_for_view_for_test("pane-a", DOC_ID);
    let second = harness
        .state()
        .mounted_rich_state_for_view_for_test("pane-b", DOC_ID);
    assert_eq!(
        first.lock().unwrap().block_plain_text(0).as_deref(),
        Some("pane A")
    );
    assert_eq!(
        second.lock().unwrap().block_plain_text(0).as_deref(),
        Some("pane A"),
        "the second stale GET body cannot fork/overwrite the shared canonical buffer"
    );
    assert!(first.lock().unwrap().save.is_some());
    assert!(second.lock().unwrap().save.is_none());
    assert!(first.lock().unwrap().draft.is_some());
    assert!(second.lock().unwrap().draft.is_none());

    let second_namespace = second
        .lock()
        .unwrap()
        .accessibility_namespace
        .clone()
        .expect("secondary view namespace");
    let second_surface = format!("editor.rich.text--view-{second_namespace}");
    harness
        .get_by(|node| node.author_id() == Some(second_surface.as_str()))
        .focus();
    harness.step();
    harness.event(egui::Event::Text("shared ".to_owned()));
    harness.step();
    assert_eq!(
        first.lock().unwrap().block_plain_text(0).as_deref(),
        Some("shared pane A"),
        "an edit in pane B is published immediately into pane A's canonical document core"
    );
    assert_eq!(
        second.lock().unwrap().block_plain_text(0).as_deref(),
        Some("shared pane A")
    );
    assert!(harness
        .state_mut()
        .open_document_in_pane_for_test("pane-b", DOC_ID));
    assert_eq!(
        first.lock().unwrap().block_plain_text(0).as_deref(),
        Some("shared pane A"),
        "reopening pane B invalidates only B and preserves pane A's unsaved shared core"
    );
    assert!(first.lock().unwrap().save.is_some());
    harness
        .state_mut()
        .apply_loaded_rich_document_to_view_for_test(
            "pane-b",
            rich_doc_body(DOC_ID, "stale reload body", 7),
        )
        .expect("deliver pane B reload");
    assert_eq!(
        second.lock().unwrap().block_plain_text(0).as_deref(),
        Some("shared pane A"),
        "pane B's reload response cannot overwrite the retained unsaved shared core"
    );

    first.lock().unwrap().selection =
        handshake_native::rich_editor::document_model::selection::Selection::caret(
            handshake_native::rich_editor::document_model::position::DocPosition::new(
                vec![0, 0],
                0,
            ),
        );
    second.lock().unwrap().selection =
        handshake_native::rich_editor::document_model::selection::Selection::caret(
            handshake_native::rich_editor::document_model::position::DocPosition::new(
                vec![0, 0],
                3,
            ),
        );
    harness.step();
    assert_ne!(
        first.lock().unwrap().selection,
        second.lock().unwrap().selection,
        "selection remains view-local while document/undo/save authority is shared"
    );

    let ids: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect();
    let mut counts = HashMap::<String, usize>::new();
    for id in &ids {
        *counts.entry(id.clone()).or_default() += 1;
    }
    let duplicates: Vec<_> = counts.iter().filter(|(_, count)| **count > 1).collect();
    assert!(
        duplicates.is_empty(),
        "every live AccessKit author_id must be unique across the whole tree: {duplicates:?}"
    );
    assert_eq!(
        ids.iter()
            .filter(|id| id.as_str() == "editor.rich.root")
            .count(),
        1
    );
    assert_eq!(
        ids.iter()
            .filter(|id| id.starts_with("editor.rich.root--view-document-"))
            .count(),
        1
    );

    for base in [
        "properties-header",
        "rich-editor-export-button",
        "rich-reading-mode-reading",
        "toolbar-btn-undo",
    ] {
        let expected = format!("{base}--view-{second_namespace}");
        assert!(
            ids.iter().any(|id| id == &expected),
            "secondary pane must publish namespaced {base}; missing {expected}"
        );
    }
    let mut action_namespace = String::from("document-");
    for byte in format!("{DOC_ID}\0pane-b").as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(action_namespace, "{byte:02x}");
    }
    let secondary_save = format!("editor.rich.save.{action_namespace}");
    assert!(ids.iter().any(|id| id == &secondary_save));
    let secondary_save_node_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(secondary_save.as_str()))
        .expect("secondary namespaced Save action node")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: secondary_save_node_id,
            data: None,
        },
    ));
    harness.step();
    harness.step();
    wait_for_requests(
        &server,
        |requests| save_request_count_for(requests, DOC_ID) >= 1,
        Duration::from_secs(2),
    );
    let save = server
        .requests()
        .into_iter()
        .find(|request| {
            request.method == "PUT" && request.path == format!("/knowledge/documents/{DOC_ID}/save")
        })
        .expect("secondary namespaced Save dispatches one real PUT");
    assert!(
        save.body.contains("shared pane A"),
        "secondary Save must publish pane content before canonical persistence: {}",
        save.body
    );
    let export_button = format!("rich-editor-export-button--view-{second_namespace}");
    let export_button_node_id = harness
        .get_by(|node| node.author_id() == Some(export_button.as_str()))
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: export_button_node_id,
            data: None,
        },
    ));
    harness.step();
    harness.step();
    assert!(!first.lock().unwrap().export_picker_open);
    assert!(second.lock().unwrap().export_picker_open);
    let picker = format!("export-format-picker--view-{second_namespace}");
    assert!(harness
        .root()
        .children_recursive()
        .any(|node| node.accesskit_node().author_id() == Some(picker.as_str())));
    let _ = server.shutdown();
}

#[test]
fn reverse_open_order_keeps_pane_a_as_deterministic_unsuffixed_rich_view() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test("http://127.0.0.1:9", runtime.handle().clone());
    assert!(app.open_document_in_pane_for_test("pane-b", DOC_ID));
    app.apply_loaded_rich_document_to_view_for_test("pane-b", rich_doc_body(DOC_ID, "shared", 7))
        .expect("load pane B first");
    assert!(app.open_document_in_pane_for_test("pane-a", DOC_ID));
    app.apply_loaded_rich_document_to_view_for_test(
        "pane-a",
        rich_doc_body(DOC_ID, "stale reverse-order body", 7),
    )
    .expect("mark pane A ready without replacing the shared body");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    let pane_a = harness
        .state()
        .mounted_rich_state_for_view_for_test("pane-a", DOC_ID);
    let pane_b = harness
        .state()
        .mounted_rich_state_for_view_for_test("pane-b", DOC_ID);
    assert_eq!(pane_a.lock().unwrap().accessibility_namespace, None);
    assert!(pane_b.lock().unwrap().accessibility_namespace.is_some());
    assert!(pane_a.lock().unwrap().save.is_some());
    assert!(pane_b.lock().unwrap().save.is_none());
    let ids: Vec<String> = harness
        .root()
        .children_recursive()
        .filter_map(|node| node.accesskit_node().author_id().map(str::to_owned))
        .collect();
    assert_eq!(ids.iter().filter(|id| *id == "editor.rich.root").count(), 1);
    assert_eq!(ids.iter().filter(|id| *id == "editor.rich.save").count(), 1);
}

#[test]
fn closing_file_backed_note_restores_unsuffixed_untitled_action_authority() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test("http://127.0.0.1:9", runtime.handle().clone());
    assert!(app.open_document_in_pane_for_test("pane-b", DOC_ID));
    app.apply_loaded_rich_document_to_view_for_test("pane-b", rich_doc_body(DOC_ID, "file", 7))
        .expect("load file-backed note");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();
    assert_eq!(
        harness
            .root()
            .children_recursive()
            .filter(|node| node.accesskit_node().author_id() == Some("editor.rich.save"))
            .count(),
        1
    );
    assert!(harness.state_mut().close_active_tab_for_test());
    assert!(harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_EDITOR_FILE_NEW));
    harness.step();
    harness.step();
    assert_eq!(
        harness
            .root()
            .children_recursive()
            .filter(|node| node.accesskit_node().author_id() == Some("editor.rich.save"))
            .count(),
        1,
        "the base/untitled editor regains canonical action registration index 0"
    );
    assert!(harness
        .root()
        .children_recursive()
        .any(|node| node.accesskit_node().author_id() == Some("editor.rich.root")));
}

#[test]
fn two_visible_unready_notes_panes_load_without_activating_each_pane() {
    let server = NotesMockServer::spawn();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());
    assert!(app.open_document_in_pane_for_test("pane-a", DOC_ID));
    assert!(app.open_document_in_pane_for_test("pane-b", SECOND_DOC_ID));
    assert_eq!(
        app.active_pane().map(ToString::to_string).as_deref(),
        Some("pane-b"),
        "pane A remains visible but is never re-activated during this proof"
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 800.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        harness.step();
        let pane_a = harness
            .state()
            .mounted_rich_state_for_view_for_test("pane-a", DOC_ID);
        let pane_b = harness
            .state()
            .mounted_rich_state_for_view_for_test("pane-b", SECOND_DOC_ID);
        let loaded_a = pane_a.lock().unwrap().block_plain_text(0).as_deref() == Some(INITIAL_TEXT);
        let loaded_b =
            pane_b.lock().unwrap().block_plain_text(0).as_deref() == Some(SECOND_INITIAL_TEXT);
        if loaded_a && loaded_b {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "both visible Notes panes must auto-load"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        harness
            .state()
            .active_pane()
            .map(ToString::to_string)
            .as_deref(),
        Some("pane-b"),
        "the scheduler loads pane A without focus/activation churn"
    );
    wait_for_requests(
        &server,
        |requests| {
            document_get_count_for(requests, DOC_ID) >= 1
                && document_get_count_for(requests, SECOND_DOC_ID) >= 1
        },
        Duration::from_secs(2),
    );
    let _ = server.shutdown();
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

    let first_state = harness
        .state()
        .mounted_rich_state_for_document_for_test(DOC_ID);
    let second_state = harness
        .state()
        .mounted_rich_state_for_document_for_test(SECOND_DOC_ID);
    assert!(
        !Arc::ptr_eq(&first_state, &second_state),
        "two Notes document ids must own distinct mounted RichEditorState instances"
    );
    assert_eq!(
        first_state.lock().unwrap().block_plain_text(0).as_deref(),
        Some(INITIAL_TEXT),
        "the delayed first GET installs only the first document's retained state"
    );
    assert_eq!(
        second_state.lock().unwrap().block_plain_text(0).as_deref(),
        Some(SECOND_INITIAL_TEXT),
        "the second GET remains installed in the second document state after the first completes"
    );

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
fn mismatched_note_get_identity_fails_closed_without_cross_document_mutation() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test("http://127.0.0.1:9", runtime.handle().clone());
    assert!(matches!(
        app.open_document(DOC_ID),
        NavDispatchOutcome::Opened { .. }
    ));
    app.apply_loaded_rich_document_for_test(rich_doc_body(DOC_ID, INITIAL_TEXT, 7))
        .expect("seed requested document through the real installer");
    app.queue_rich_document_load_result_for_test(
        DOC_ID,
        Ok(rich_doc_body(SECOND_DOC_ID, "wrong document body", 13)),
    );

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.step();

    let failure = harness
        .state()
        .rich_document_load_failure_for_test(DOC_ID)
        .expect("mismatched response is latched as a typed load failure");
    assert!(failure.contains(DOC_ID) && failure.contains(SECOND_DOC_ID));
    assert_eq!(
        harness
            .state()
            .mounted_rich_state_for_document_for_test(DOC_ID)
            .lock()
            .unwrap()
            .block_plain_text(0)
            .as_deref(),
        Some(INITIAL_TEXT),
        "the requested document keeps its prior body"
    );
    assert_ne!(
        harness
            .state()
            .mounted_rich_state_for_document_for_test(SECOND_DOC_ID)
            .lock()
            .unwrap()
            .block_plain_text(0)
            .as_deref(),
        Some("wrong document body"),
        "the response identity must not select or mutate another document store entry"
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

#[test]
fn failed_note_load_latches_until_explicit_accesskit_retry() {
    let _wgpu_guard = wgpu_guard();
    let server =
        NotesMockServer::spawn_with_plain_get_failures(HashMap::from([(DOC_ID.to_owned(), 1)]));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());
    assert!(
        matches!(app.open_document(DOC_ID), NavDispatchOutcome::Opened { .. }),
        "opening the note starts the authoritative GET"
    );

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    wait_for_author_node(
        &mut harness,
        NOTES_LOAD_ERROR_AUTHOR_ID,
        Duration::from_secs(5),
    );
    assert!(
        find_author_node_exists(&harness, NOTES_LOAD_RETRY_AUTHOR_ID),
        "the latched load failure exposes an explicit AccessKit Retry action"
    );
    let error_label = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(NOTES_LOAD_ERROR_AUTHOR_ID))
        .and_then(|node| node.accesskit_node().label())
        .unwrap_or_default();
    assert!(
        error_label.contains(DOC_ID) && error_label.contains("5xx"),
        "the stable error identifies the exact failed document and backend failure: {error_label:?}"
    );

    // Pump well beyond the response frame. A terminal failure must remain latched and must not turn
    // each repaint into another GET.
    for _ in 0..24 {
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        document_get_count_for(&server.requests(), DOC_ID),
        1,
        "failed Notes load issues exactly one GET until the operator retries"
    );

    let retry_id = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(NOTES_LOAD_RETRY_AUTHOR_ID))
        .expect("Retry AccessKit node remains present")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: retry_id,
            data: None,
        },
    ));
    harness.step();
    harness.step();
    wait_for_requests(
        &server,
        |requests| document_get_count_for(requests, DOC_ID) == 2,
        Duration::from_secs(5),
    );
    wait_for_text(&mut harness, INITIAL_TEXT, Duration::from_secs(5));
    for _ in 0..8 {
        harness.step();
    }
    assert_eq!(
        document_get_count_for(&server.requests(), DOC_ID),
        2,
        "the explicit Retry issues exactly one additional authoritative GET"
    );
    assert!(
        !find_author_node_exists(&harness, NOTES_LOAD_ERROR_AUTHOR_ID),
        "a successful retry clears the latched error"
    );

    let _ = server.shutdown();
}

fn text_from_doc(value: &serde_json::Value) -> String {
    value["content"][0]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_owned()
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-020 — APP-LEVEL draft recovery proof (the remediation's missing half): the MT-099
// mock serves a NON-NULL draft on the SECOND open, and the recovery flows through the MOUNTED pane —
// the `draft-recovery-banner` appears in the live tree and clicking `draft-restore` loads the draft
// content into the mounted editor. This is the operator-visible crash-recovery path, not a
// state-level unit assertion.
// ═══════════════════════════════════════════════════════════════════════════════════════════════════

const DRAFT_TEXT: &str = "MT-020 recovered draft note";

fn find_author_node_exists(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> bool {
    harness
        .root()
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(author_id))
}

fn wait_for_author_node(
    harness: &mut Harness<'_, HandshakeApp>,
    author_id: &str,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        harness.step();
        if find_author_node_exists(harness, author_id) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for AccessKit node author_id={author_id:?} in the mounted app tree");
}

#[test]
fn draft_recovery_banner_restores_draft_through_mounted_pane_on_second_open() {
    let _wgpu_guard = wgpu_guard();
    let server = NotesMockServer::spawn();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut app = ok_app();
    app.set_backend_base_url_for_test(&server.base_url, runtime.handle().clone());

    // FIRST open: no draft exists (`draft: null`) — no recovery banner may appear.
    assert!(
        matches!(app.open_document(DOC_ID), NavDispatchOutcome::Opened { .. }),
        "first open routes through ShellNavigator"
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1180.0, 760.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    wait_for_text(&mut harness, INITIAL_TEXT, Duration::from_secs(5));
    // Let the first draft GET land, then assert NO banner (draft was null).
    wait_for_requests(
        &server,
        |requests| {
            requests.iter().any(|r| {
                r.method == "GET" && r.path == format!("/knowledge/documents/{DOC_ID}/draft")
            })
        },
        Duration::from_secs(5),
    );
    harness.step();
    harness.step();
    assert!(
        !find_author_node_exists(&harness, "draft-recovery-banner"),
        "no draft on first open -> no recovery banner"
    );

    // Stage a persisted draft server-side (the crash-recovery scenario: unsaved edits survived),
    // then reopen the SAME document — the second open's fresh GET /draft serves it non-null.
    server.set_document_draft(DOC_ID, DRAFT_TEXT);
    assert!(
        matches!(
            harness.state_mut().open_document(DOC_ID),
            NavDispatchOutcome::Opened { .. }
        ),
        "second open routes through ShellNavigator again"
    );

    // The mounted pane shows the 'Draft recovery' banner (the operator-visible AC surface).
    wait_for_author_node(
        &mut harness,
        "draft-recovery-banner",
        Duration::from_secs(5),
    );
    assert!(
        find_author_node_exists(&harness, "draft-restore"),
        "the banner offers the Restore-draft button"
    );
    assert!(
        find_author_node_exists(&harness, "draft-discard"),
        "the banner offers the Discard button"
    );

    // Restore THROUGH the mounted pane: activate the live "Restore draft" button via an
    // AccessKit Click ACTION targeted at its node id (the deterministic route a swarm agent / AT
    // client uses; the shell repaints continuously in this harness, so `run()`-settled pointer
    // clicks are unavailable — the action request reaches `Response::clicked` directly).
    let restore_id = harness.get_by_label("Restore draft").accesskit_node().id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: restore_id,
            data: None,
        },
    ));
    harness.step();
    harness.step();

    // The mounted editor now carries the DRAFT content (recovered in-progress edits).
    wait_for_text(&mut harness, DRAFT_TEXT, Duration::from_secs(5));

    // The restored draft is UNSAVED recovered work -> the banner is gone and the doc is dirty
    // (a later Ctrl+S persists it); the restore itself must not silently canonical-save.
    harness.step();
    assert!(
        !find_author_node_exists(&harness, "draft-recovery-banner"),
        "restoring dismisses the banner"
    );

    let requests = server.shutdown();
    let draft_path = format!("/knowledge/documents/{DOC_ID}/draft");
    let draft_gets = requests
        .iter()
        .filter(|r| r.method == "GET" && r.path == draft_path)
        .count();
    assert!(
        draft_gets >= 2,
        "both opens issued a draft GET (got {draft_gets}); requests={requests:?}"
    );
    let saves = requests
        .iter()
        .filter(|r| r.method == "PUT" && r.path == format!("/knowledge/documents/{DOC_ID}/save"))
        .count();
    assert_eq!(
        saves, 0,
        "restoring a draft must NOT canonical-save by itself (the operator saves explicitly)"
    );
}
