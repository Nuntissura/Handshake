//! MT-019 daily-notes / journal panel PROOFS: kittest screenshots (panel + calendar popup), AccessKit
//! id assertions (`journal-prev-day`/`journal-next-day`/`journal-today`/`journal-date-display`/…),
//! the date display, the Loading-spinner state, the typed error chip + Retry, and the gated real-backend
//! openDailyJournal round-trip.
//!
//! ## Spinner trap avoidance (KERNEL_BUILDER gate / MT-015 lesson)
//!
//! The Loading state renders an animating `egui::Spinner`, and `harness.run()` loops forever on an
//! animating frame. EVERY test here uses `harness.step()` (single-frame) NEVER `harness.run()`, and
//! holds the process-wide [`wgpu_guard`] for any `.wgpu()` screenshot harness (MT-018 pattern: creating
//! several wgpu devices on parallel threads aborts on Windows).
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG goes ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-019/` root
//! via [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` dir exists (the MT contract literally names a repo-local
//! screenshot path under `src/`, but the CX-212E artifact rule OVERRIDES it — a tracked PNG under src/
//! is a hygiene failure the reviewer greps for with `git ls-files "src/**/*.png"`).
//!
//! ## Backend reality (Spec-Realism Gate)
//!
//! The state machine, date nav, calendar grid, word/char count, and auto-save debounce are FULLY proven
//! here + in the module unit tests with a MOCK backend + mock clock — NO live backend. The real-backend
//! `openDailyJournal` round-trip is the `#[ignore]` integration test (`test_real_open_today`), which
//! needs a live Handshake-managed backend on 127.0.0.1:37501 with a seeded workspace; absent that, it is
//! NEEDS_MANAGED_RESOURCE_PROOF (run with `--features integration -- --ignored` against a live backend).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::Queryable;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use chrono::NaiveDate;

use handshake_native::rich_editor::daily_notes::date_nav::{
    DateNav, CALENDAR_TOGGLE_ID, DATE_DISPLAY_ID, NEXT_DAY_ID, PREV_DAY_ID, TODAY_ID,
};
use handshake_native::rich_editor::daily_notes::journal_panel::{
    JournalPaneFactory, JournalPanelState, JournalPanelWidget, JOURNAL_ROOT_ID, LINK_GAP_ID,
    RETRY_ID, START_WRITING_ID,
};
use handshake_native::rich_editor::daily_notes::journal_store::{
    JournalBackend, JournalBlock, JournalDocLoad, JournalError, JournalFuture, JournalReady,
    JournalSaveSeam, JournalStore, ReqwestJournalBackend, ReqwestSaveSeam, RichDocumentBody,
};
use handshake_native::theme::HsTheme;

// ── Artifact-root helpers (CX-212E) ─────────────────────────────────────────────────────────────

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach the sibling `Handshake_Artifacts`.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path the MT contract literally names, which this rule
/// overrides).
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

/// Serialize the `.wgpu()` screenshot tests (mirrors test_properties.rs / test_rich_editor_widget.rs):
/// creating several wgpu devices concurrently on parallel test threads aborts the process on Windows.
static WGPU_SERIAL_GUARD: Mutex<()> = Mutex::new(());

fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Debug)]
struct CapturedRequest {
    request_line: String,
    headers: std::collections::HashMap<String, String>,
    body: String,
}

fn spawn_capture_server(
    status_line: &'static str,
    body: serde_json::Value,
) -> (String, std::thread::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind MT-019 capture server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept MT-019 request");
        let captured = read_one_request(&mut stream);
        let body = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        captured
    });
    (base_url, handle)
}

fn read_one_request(stream: &mut TcpStream) -> CapturedRequest {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some(hdr_end) = text.find("\r\n\r\n") {
            let header_text = &text[..hdr_end];
            let content_len = header_text
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    if key.trim().eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            if buf.len() >= hdr_end + 4 + content_len {
                break;
            }
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let hdr_end = text.find("\r\n\r\n").unwrap_or(text.len());
    let mut lines = text[..hdr_end].lines();
    let request_line = lines.next().unwrap_or("").to_owned();
    let mut headers = std::collections::HashMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let body = text[(hdr_end + 4).min(text.len())..].to_owned();
    CapturedRequest {
        request_line,
        headers,
        body,
    }
}

fn assert_hsk_document_headers(req: &CapturedRequest) {
    for header in [
        "x-hsk-actor-id",
        "x-hsk-kernel-task-run-id",
        "x-hsk-session-run-id",
    ] {
        assert!(
            req.headers.contains_key(header),
            "MT-019 document request must carry {header}; got headers {:?}",
            req.headers.keys().collect::<Vec<_>>()
        );
    }
}

fn assert_hsk_operator_document_headers(req: &CapturedRequest) {
    assert_hsk_document_headers(req);
    assert_eq!(
        req.headers.get("x-hsk-actor-kind").map(String::as_str),
        Some("operator"),
        "MT-019 write requests must carry x-hsk-actor-kind: operator; got headers {:?}",
        req.headers
    );
}

fn knowledge_document_response(id: &str, version: u64) -> serde_json::Value {
    serde_json::json!({
        "document": {
            "rich_document_id": id,
            "workspace_id": "WS-1",
            "title": "Daily Note",
            "schema_version": "rich_document_block_tree_v1",
            "content_json": {"type": "doc", "content": []},
            "content_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "doc_version": version,
            "authority_label": "draft",
            "owner_actor_kind": "operator",
            "owner_actor_id": "handshake-native-editor"
        },
        "tree": {
            "schema_version": "rich_document_block_tree_v1",
            "schema_matches": true,
            "block_ids": [],
            "blocks": []
        },
        "code_nodes": []
    })
}

// ── A fixed mock "today" so the proofs are deterministic ───────────────────────────────────────

/// The fixed mock today the ACs use (2026-06-19, a Friday).
fn mock_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 6, 19).unwrap()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// A journal block for `date` with an optional linked document id.
fn journal_block(date: &str, doc_id: Option<&str>) -> JournalBlock {
    JournalBlock {
        block_id: format!("LB-{date}"),
        workspace_id: "ws-1".into(),
        content_type: Some("journal".into()),
        document_id: doc_id.map(|s| s.to_owned()),
        title: Some(format!("Daily Note {date}")),
        journal_date: Some(date.to_owned()),
    }
}

/// A rich-document body whose content_json carries `text` (for the footer word/char-count proof).
fn doc_body_with_text(id: &str, text: &str) -> RichDocumentBody {
    RichDocumentBody {
        rich_document_id: id.into(),
        title: "Daily Note".into(),
        doc_version: 3,
        content_json: Some(serde_json::json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }]
        })),
    }
}

// ── A counted mock backend (no live backend) ────────────────────────────────────────────────────

/// A counted mock backend: open returns a journal block (with or without a linked document, and with an
/// optional forced error), load returns a fixed document, create returns a fresh one. Records the open
/// call count so the date-nav re-open proof is non-trivial.
struct MockBackend {
    link_document: bool,
    fail_open: bool,
    open_calls: Mutex<u32>,
}

impl MockBackend {
    fn new(link_document: bool, fail_open: bool) -> Self {
        Self {
            link_document,
            fail_open,
            open_calls: Mutex::new(0),
        }
    }
    fn open_count(&self) -> u32 {
        *self.open_calls.lock().unwrap()
    }
}

impl JournalBackend for MockBackend {
    fn open_daily_journal<'a>(
        &'a self,
        _ws: &'a str,
        date: &'a str,
    ) -> JournalFuture<'a, JournalBlock> {
        *self.open_calls.lock().unwrap() += 1;
        let date = date.to_owned();
        let fail = self.fail_open;
        let link = self.link_document;
        Box::pin(async move {
            if fail {
                return Err(JournalError::OpenFailed("HTTP 500".into()));
            }
            Ok(JournalBlock {
                block_id: format!("LB-{date}"),
                workspace_id: "ws-1".into(),
                content_type: Some("journal".into()),
                document_id: if link { Some("KRD-1".into()) } else { None },
                title: Some(format!("Daily Note {date}")),
                journal_date: Some(date),
            })
        })
    }
    fn load_document<'a>(&'a self, id: &'a str) -> JournalFuture<'a, JournalDocLoad> {
        let id = id.to_owned();
        Box::pin(async move {
            Ok(JournalDocLoad {
                body: RichDocumentBody {
                    rich_document_id: id,
                    title: "Daily Note".into(),
                    doc_version: 3,
                    content_json: Some(serde_json::json!({
                        "type": "doc",
                        "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "today I wrote three words" }] }]
                    })),
                },
            })
        })
    }
    fn create_document<'a>(
        &'a self,
        _ws: &'a str,
        title: &'a str,
    ) -> JournalFuture<'a, JournalDocLoad> {
        let title = title.to_owned();
        Box::pin(async move {
            Ok(JournalDocLoad {
                body: RichDocumentBody {
                    rich_document_id: "KRD-NEW".into(),
                    title,
                    doc_version: 1,
                    content_json: None,
                },
            })
        })
    }
}

struct MockSeam;
impl JournalSaveSeam for MockSeam {
    fn save<'a>(&'a self, _id: &'a str, v: u64, _c: serde_json::Value) -> JournalFuture<'a, u64> {
        Box::pin(async move { Ok(v + 1) })
    }
}

/// A save seam that COUNTS dispatches (for the auto-save-through-show-path proof). The count is bumped
/// the instant the save future runs — `dispatch_save` spawns it on the live runtime.
struct CountingSeam {
    saves: Arc<Mutex<u32>>,
}
impl JournalSaveSeam for CountingSeam {
    fn save<'a>(&'a self, _id: &'a str, v: u64, _c: serde_json::Value) -> JournalFuture<'a, u64> {
        let saves = Arc::clone(&self.saves);
        Box::pin(async move {
            *saves.lock().unwrap() += 1;
            Ok(v + 1)
        })
    }
}

/// Build a headless panel state at `current` with the given mock backend behavior, opened (so a staged
/// load can drive it to Ready).
fn headless_panel(current: NaiveDate, link_document: bool, fail_open: bool) -> JournalPanelState {
    let store = JournalStore::headless(
        Arc::new(MockBackend::new(link_document, fail_open)),
        Arc::new(MockSeam),
    );
    let nav = DateNav::new(current, mock_today());
    let mut p = JournalPanelState::new(store, nav);
    p.theme = HsTheme::Dark;
    p
}

/// Collect all AccessKit author_ids present in the rendered tree.
fn collect_author_ids<State>(harness: &Harness<'_, State>) -> std::collections::HashSet<String> {
    use egui_kittest::kittest::NodeT;
    let mut found = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            found.insert(a.to_owned());
        }
    }
    found
}

// ── AC-1 + AC-11: panel opens, shows today's date, AccessKit nav ids present ──────────────────────

#[test]
fn mt019_panel_shows_today_and_nav_accesskit_ids() {
    // Ready state for the fixed mock today (2026-06-19) with a linked document.
    let mut p = headless_panel(mock_today(), true, false);
    p.store.seed_ready(JournalReady::new(
        "2026-06-19",
        journal_block("2026-06-19", Some("KRD-1")),
        Some(doc_body_with_text("KRD-1", "today I wrote three words")),
    ));

    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 520.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    // step(), not run() — the panel may animate (no perpetual spinner, but the editor caret blinks).
    harness.step();
    harness.step();

    // AC-1: the header shows today's date in human-readable form.
    assert!(
        harness
            .query_by_label_contains("Friday, June 19, 2026")
            .is_some(),
        "AC-1: the header shows today's date (Friday, June 19, 2026)"
    );

    // AC-11: the nav AccessKit author_ids are present in the live tree.
    let found = collect_author_ids(&harness);
    for id in [
        PREV_DAY_ID,
        NEXT_DAY_ID,
        TODAY_ID,
        DATE_DISPLAY_ID,
        CALENDAR_TOGGLE_ID,
    ] {
        assert!(
            found.contains(id),
            "AC-11: the AccessKit tree must contain '{id}' (found {found:?})"
        );
    }
    assert!(
        found.contains(JOURNAL_ROOT_ID),
        "the panel root is addressable"
    );
    assert!(
        found.contains("journal-block-badge"),
        "the block-id badge is addressable"
    );
}

// ── AC-2 / AC-3 / AC-4 / AC-5: openDailyJournal on mount + date navigation re-opens ──────────────

#[test]
fn mt019_date_navigation_reopens_journal_and_calls_open_per_date() {
    // A LIVE tokio runtime so open() actually dispatches against the mock and we can count calls. This
    // proves AC-2 (open on mount), AC-3 (prev → 2026-06-18), AC-4 (next back to 2026-06-19), AC-5
    // (today). The mock backend counts open() calls.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend = Arc::new(MockBackend::new(false, false));
    let store = JournalStore::new(
        Arc::clone(&backend) as Arc<dyn JournalBackend>,
        Arc::new(MockSeam),
        Some(rt.handle().clone()),
    );
    let mut p = JournalPanelState::new(store, DateNav::new(mock_today(), mock_today()));
    p.store.workspace_id = "ws-1".into();

    // AC-2: open on mount.
    p.open_current();
    drain_until(&mut p, "2026-06-19");
    assert_eq!(
        p.store.state.ready().map(|r| r.date.clone()),
        Some("2026-06-19".into())
    );
    assert_eq!(
        backend.open_count(),
        1,
        "AC-2: openDailyJournal called once on mount"
    );

    // AC-3: prev day → 2026-06-18, a new open.
    p.nav.prev_day();
    p.store.open(p.nav.current_storage());
    drain_until(&mut p, "2026-06-18");
    assert_eq!(p.nav.current, date(2026, 6, 18));
    assert_eq!(
        p.store.state.ready().map(|r| r.date.clone()),
        Some("2026-06-18".into())
    );
    assert_eq!(
        backend.open_count(),
        2,
        "AC-3: prev triggers a new openDailyJournal"
    );

    // AC-4: next day → back to 2026-06-19.
    p.nav.next_day();
    p.store.open(p.nav.current_storage());
    drain_until(&mut p, "2026-06-19");
    assert_eq!(p.nav.current, date(2026, 6, 19));
    assert_eq!(
        backend.open_count(),
        3,
        "AC-4: next triggers a new openDailyJournal"
    );

    // AC-5: today from a far date → 2026-06-19.
    p.nav.navigate_to(date(2025, 1, 1));
    p.nav.jump_today();
    p.store.open(p.nav.current_storage());
    drain_until(&mut p, "2026-06-19");
    assert_eq!(
        p.nav.current,
        mock_today(),
        "AC-5: Today returns to the fixed mock today"
    );
}

/// Poll the store drain until the Ready date matches `expected` (bounded — the mock resolves fast).
fn drain_until(p: &mut JournalPanelState, expected: &str) {
    for _ in 0..400 {
        p.store.drain();
        if p.store.state.ready().map(|r| r.date.as_str()) == Some(expected) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("journal did not reach Ready for {expected} (state did not resolve)");
}

// ── AC-8: Loading state shows a spinner (genuine fetch only) ──────────────────────────────────────

#[test]
fn mt019_loading_state_shows_spinner() {
    let _g = wgpu_guard();
    // Force the Loading state directly (the state a live fetch produces). We use step(), never run(),
    // because the spinner animates — run() would loop forever (the MT-015 trap this MT avoids).
    let mut p = headless_panel(mock_today(), true, false);
    // Seed Loading (a live runtime would have done this via open(); here we assert the RENDER of the
    // Loading spinner via a single-frame step(), never run() — the MT-015 trap this MT avoids).
    p.store.seed_loading("2026-06-19");
    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 420.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    harness.step();

    assert!(
        harness
            .query_by_label_contains("Loading daily journal")
            .is_some(),
        "AC-8: the Loading state shows the loading text + spinner"
    );

    // The state must still be Loading after stepping (it does not self-resolve in a headless harness).
    assert!(
        state.lock().unwrap().store.state.is_loading(),
        "AC-8: the Loading state persists (the spinner is genuine fetch UX, not a stuck headless state — \
         but here we forced Loading explicitly to render the spinner)"
    );
}

// ── AC-7: error state shows a typed chip + Retry ─────────────────────────────────────────────────

#[test]
fn mt019_error_state_shows_typed_chip_and_retry() {
    let mut p = headless_panel(mock_today(), true, true);
    p.store
        .seed_error("2026-06-19", JournalError::OpenFailed("HTTP 500".into()));
    assert!(
        p.store.state.error().is_some(),
        "the store entered the Error state"
    );

    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 420.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    harness.step();

    // AC-7: a typed (kind-stamped) error chip renders, never blank.
    assert!(
        harness.query_by_label_contains("open_failed").is_some(),
        "AC-7: the error chip carries the typed kind (open_failed)"
    );
    // ... and a Retry button is addressable.
    let found = collect_author_ids(&harness);
    assert!(
        found.contains(RETRY_ID),
        "AC-7: the error state shows a Retry button (journal-retry)"
    );
}

// ── "Start writing" affordance for a blank journal block ─────────────────────────────────────────

#[test]
fn mt019_start_writing_button_for_blank_block() {
    let mut p = headless_panel(mock_today(), false, false);
    p.store.seed_ready(JournalReady::new(
        "2026-06-19",
        journal_block("2026-06-19", None),
        None,
    ));
    assert!(p.store.state.ready().unwrap().needs_document());

    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 420.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    harness.step();

    let found = collect_author_ids(&harness);
    assert!(
        found.contains(START_WRITING_ID),
        "a blank journal block shows the 'Start writing' button (journal-start-writing)"
    );
}

#[test]
fn mt019_created_blank_journal_discloses_missing_durable_link() {
    let mut p = headless_panel(mock_today(), false, false);
    p.store.seed_ready(JournalReady::new(
        "2026-06-19",
        journal_block("2026-06-19", None),
        Some(doc_body_with_text("KRD-created", "session note")),
    ));
    assert!(
        p.store.state.ready().unwrap().document_link_missing(),
        "fixture represents a created rich doc that the backend cannot attach to the journal block"
    );

    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 440.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    harness.step();

    assert!(
        harness
            .query_by_label_contains("Journal link not persisted")
            .is_some(),
        "the editor renders a visible backend-gap banner for an unlinked created journal document"
    );
    let found = collect_author_ids(&harness);
    assert!(
        found.contains(LINK_GAP_ID),
        "the backend-gap banner is model-addressable by '{LINK_GAP_ID}'"
    );
}

// ── PT-2: panel screenshot (header, date, content area, footer) ───────────────────────────────────

#[test]
fn mt019_panel_screenshot() {
    let _g = wgpu_guard();
    let mut p = headless_panel(mock_today(), true, false);
    p.store.seed_ready(JournalReady::new(
        "2026-06-19",
        journal_block("2026-06-19", Some("KRD-1")),
        Some(doc_body_with_text("KRD-1", "today I wrote three words")),
    ));
    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(680.0, 540.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    harness.step();

    // The footer word count reflects the 5-word body ("today I wrote three words").
    assert!(
        harness.query_by_label_contains("5 words").is_some(),
        "AC-9: the footer word count matches the 5-word journal body"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0);
            let ext = external_artifact_dir("wp-kernel-012-mt-019");
            let _ = std::fs::create_dir_all(&ext);
            let path = ext.join("mt019_journal_panel.png");
            let saved = image.save(&path).is_ok();
            println!("PT-2 panel screenshot: {}x{} saved={saved} ({})", image.width(), image.height(), path.display());
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt019_journal_panel screenshot unavailable (no wgpu adapter): {e}. \
             The structural + footer-count proofs passed; the PNG is a GPU-host item."
        ),
    }
    assert_no_local_artifact_dir();
}

// ── PT-3 + AC-6: calendar popup screenshot (the fixed 6-row month grid) ───────────────────────────

#[test]
fn mt019_calendar_popup_screenshot() {
    let _g = wgpu_guard();
    let mut p = headless_panel(mock_today(), true, false);
    p.store.seed_ready(JournalReady::new(
        "2026-06-19",
        journal_block("2026-06-19", Some("KRD-1")),
        Some(RichDocumentBody {
            rich_document_id: "KRD-1".into(),
            title: "Daily Note".into(),
            doc_version: 3,
            content_json: None,
        }),
    ));
    // Open the calendar popup directly (the toggle is proven addressable in the AccessKit test).
    p.nav.calendar_open = true;

    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 480.0))
        .wgpu()
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    harness.step();

    // AC-6: the calendar shows the June 2026 month grid with correct day numbers (1..30).
    assert!(
        harness.query_by_label_contains("June 2026").is_some(),
        "AC-6: the calendar popup shows the June 2026 month header"
    );
    // The day-1 and day-30 cells are addressable (proves the grid rendered the real day numbers).
    let found = collect_author_ids(&harness);
    assert!(
        found.contains("journal-calendar-day-1"),
        "AC-6: day 1 cell is present"
    );
    assert!(
        found.contains("journal-calendar-day-30"),
        "AC-6: day 30 cell is present (June has 30 days)"
    );

    match harness.render() {
        Ok(image) => {
            assert!(image.width() > 0 && image.height() > 0);
            let ext = external_artifact_dir("wp-kernel-012-mt-019");
            let _ = std::fs::create_dir_all(&ext);
            let path = ext.join("mt019_calendar_popup.png");
            let saved = image.save(&path).is_ok();
            println!("PT-3 calendar screenshot: {}x{} saved={saved} ({})", image.width(), image.height(), path.display());
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): mt019_calendar_popup screenshot unavailable (no wgpu adapter): {e}. \
             The grid-structure + day-cell proofs passed; the PNG is a GPU-host item."
        ),
    }
    assert_no_local_artifact_dir();
}

// ── AC-10 (runtime): auto-save fires from a real edit through the live show() render path ─────────

#[test]
fn mt019_auto_save_fires_from_edit_through_show_render_path() {
    use handshake_native::rich_editor::document_model::node::BlockNode;

    // A LIVE tokio runtime so the store's dispatch_save actually spawns the (counting) save seam. This
    // is the must-fix proof: auto-save fires during REAL rendering (JournalPanelWidget::show), driven by
    // an edit detected frame-to-frame — NOT by calling tick_auto_save()/on_edit() directly.
    let rt = tokio::runtime::Runtime::new().unwrap();
    let saves = Arc::new(Mutex::new(0u32));
    let seam = Arc::new(CountingSeam {
        saves: Arc::clone(&saves),
    });
    let store = JournalStore::new(
        Arc::new(MockBackend::new(true, false)) as Arc<dyn JournalBackend>,
        seam,
        Some(rt.handle().clone()),
    );
    let mut p = JournalPanelState::new(store, DateNav::new(mock_today(), mock_today()));
    p.theme = HsTheme::Dark;
    p.store.workspace_id = "ws-1".into();
    // Seed a Ready state with a loaded document so the editor mounts and a save has a target.
    p.store.seed_ready(JournalReady::new(
        "2026-06-19",
        journal_block("2026-06-19", Some("KRD-1")),
        Some(doc_body_with_text("KRD-1", "initial body")),
    ));

    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 520.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });

    // Frame 1-2: mount + sync the document into the editor, anchoring the edit-detection fingerprint.
    harness.step();
    harness.step();
    assert_eq!(*saves.lock().unwrap(), 0, "no save before any edit");

    // Simulate the user typing: mutate the embedded editor doc (what the editor's input handler does on
    // a keystroke). The panel's show() detects this on the next frame via the content fingerprint.
    {
        let st = state.lock().unwrap();
        let mut editor = st.editor.lock().unwrap();
        editor.doc = BlockNode::doc(vec![BlockNode::paragraph("initial body plus typed words")]);
    }

    // Step enough frames to (a) detect the edit, then (b) accrue >180 idle frames so the debounce fires.
    // Each show() call ticks the frame counter once via detect_edits_and_tick.
    for _ in 0..190 {
        harness.step();
    }

    // The save dispatch spawned on the runtime; poll briefly for the counting seam to record it.
    let mut fired = false;
    for _ in 0..200 {
        if *saves.lock().unwrap() >= 1 {
            fired = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        fired,
        "AC-10 (runtime): auto-save dispatched through the live show() path after an idle edit \
         (save count = {})",
        *saves.lock().unwrap()
    );
}

// ── Ctrl+S manual save through the live show() render path ────────────────────────────────────────

#[test]
fn mt019_ctrl_s_dispatches_manual_save_through_show_path() {
    use handshake_native::rich_editor::document_model::node::BlockNode;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let saves = Arc::new(Mutex::new(0u32));
    let seam = Arc::new(CountingSeam {
        saves: Arc::clone(&saves),
    });
    let store = JournalStore::new(
        Arc::new(MockBackend::new(true, false)) as Arc<dyn JournalBackend>,
        seam,
        Some(rt.handle().clone()),
    );
    let mut p = JournalPanelState::new(store, DateNav::new(mock_today(), mock_today()));
    p.theme = HsTheme::Dark;
    p.store.workspace_id = "ws-1".into();
    p.store.seed_ready(JournalReady::new(
        "2026-06-19",
        journal_block("2026-06-19", Some("KRD-1")),
        Some(doc_body_with_text("KRD-1", "body to save")),
    ));

    let state = Arc::new(Mutex::new(p));
    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 520.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            JournalPanelWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();
    harness.step();
    {
        let st = state.lock().unwrap();
        let mut editor = st.editor.lock().unwrap();
        editor.doc = BlockNode::doc(vec![BlockNode::paragraph("body to save edited")]);
    }
    // Press Ctrl+S — the panel's show() consumes the chord (Modifiers::COMMAND == ctrl here) and
    // dispatches an immediate save. Inject the key event the same way the editor keymap tests do.
    harness.event(egui::Event::Key {
        key: egui::Key::S,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    });
    for _ in 0..3 {
        harness.step();
    }

    let mut fired = false;
    for _ in 0..200 {
        if *saves.lock().unwrap() >= 1 {
            fired = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        fired,
        "Ctrl+S dispatched a manual save through show() (save count = {})",
        *saves.lock().unwrap()
    );
}

// ── factory wiring (the sibling pane mounts through the WP-011 host) ──────────────────────────────

#[test]
fn mt019_factory_is_the_journal_sibling_pane() {
    use handshake_native::pane_registry::{PaneFactory, PaneType};
    let p = headless_panel(mock_today(), true, false);
    let f = JournalPaneFactory::new(Arc::new(Mutex::new(p)));
    assert_eq!(
        f.pane_type(),
        PaneType::LoomDailyJournal,
        "the journal is the LoomDailyJournal sibling surface"
    );
}

#[test]
fn mt019_operator_route_mounts_bound_journal_editor_root_in_app() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build app runtime");
    let mut app = handshake_native::app::HandshakeApp::with_health(
        handshake_native::app::HealthDisplayState::Ok(
            handshake_native::backend_client::HealthInfo {
                status: "ok".to_owned(),
                db_status: "ok".to_owned(),
                migration_version: Some(1),
            },
        ),
    );
    app.set_runtime_handle(runtime.handle().clone());

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_state(
            |ctx, app: &mut handshake_native::app::HandshakeApp| app.ui(ctx),
            app,
        );
    harness.run_steps(2);

    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test(handshake_native::command_registry::CMD_VIEW_JOURNAL),
        "view.journal dispatches through the real app command route"
    );

    for _ in 0..20 {
        harness.run_steps(2);
        let found = collect_author_ids(&harness);
        if found.contains(JOURNAL_ROOT_ID) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    panic!(
        "view.journal did not mount the bound journal editor root; found author_ids {:?}",
        collect_author_ids(&harness)
    );
}

#[test]
fn mt019_open_daily_journal_uses_put_and_deserializes_document_id() {
    let body = serde_json::json!({
        "block_id": "LB-2026-06-19",
        "workspace_id": "WS-1",
        "content_type": "journal",
        "document_id": "KRD-linked",
        "title": "Daily Note 2026-06-19",
        "journal_date": "2026-06-19"
    });
    let (base_url, server) = spawn_capture_server("HTTP/1.1 200 OK", body);
    let backend = ReqwestJournalBackend::new(base_url);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let block = rt
        .block_on(async { backend.open_daily_journal("WS-1", "2026-06-19").await })
        .expect("mock openDailyJournal succeeds");
    let request = server.join().unwrap();

    assert!(
        request
            .request_line
            .starts_with("PUT /workspaces/WS-1/loom/journals/2026-06-19"),
        "openDailyJournal uses the Loom journal PUT route: {}",
        request.request_line
    );
    assert_eq!(block.block_id, "LB-2026-06-19");
    assert_eq!(block.workspace_id, "WS-1");
    assert_eq!(block.content_type.as_deref(), Some("journal"));
    assert_eq!(block.document_id.as_deref(), Some("KRD-linked"));
}

#[test]
fn mt019_linked_document_load_uses_required_hsk_headers() {
    let (base_url, server) = spawn_capture_server(
        "HTTP/1.1 200 OK",
        knowledge_document_response("KRD-load", 7),
    );
    let backend = ReqwestJournalBackend::new(base_url);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let loaded = rt
        .block_on(async { backend.load_document("KRD-load").await })
        .expect("mock linked document load succeeds");
    let request = server.join().unwrap();

    assert_eq!(loaded.body.rich_document_id, "KRD-load");
    assert!(
        request
            .request_line
            .starts_with("GET /knowledge/documents/KRD-load"),
        "load request hit the linked document route: {}",
        request.request_line
    );
    assert_hsk_document_headers(&request);
}

#[test]
fn mt019_start_writing_create_uses_required_hsk_headers() {
    let (base_url, server) = spawn_capture_server(
        "HTTP/1.1 200 OK",
        knowledge_document_response("KRD-created", 1),
    );
    let backend = ReqwestJournalBackend::new(base_url);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let created = rt
        .block_on(async { backend.create_document("WS-1", "Daily Note").await })
        .expect("mock create document succeeds");
    let request = server.join().unwrap();

    assert_eq!(created.body.rich_document_id, "KRD-created");
    assert!(
        request
            .request_line
            .starts_with("POST /knowledge/documents"),
        "create request hit the document create route: {}",
        request.request_line
    );
    assert_hsk_operator_document_headers(&request);
    assert!(
        request.body.contains("\"workspace_id\":\"WS-1\"")
            && request.body.contains("\"content_json\":null"),
        "create body preserves the journal workspace and empty rich-doc body: {}",
        request.body
    );
}

#[test]
fn mt019_auto_save_uses_required_hsk_headers() {
    let (base_url, server) = spawn_capture_server(
        "HTTP/1.1 200 OK",
        knowledge_document_response("KRD-save", 8),
    );
    let seam = ReqwestSaveSeam::new(base_url);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let version = rt
        .block_on(async {
            seam.save(
                "KRD-save",
                7,
                serde_json::json!({"type": "doc", "content": []}),
            )
            .await
        })
        .expect("mock auto-save succeeds");
    let request = server.join().unwrap();

    assert_eq!(version, 8);
    assert!(
        request
            .request_line
            .starts_with("PUT /knowledge/documents/KRD-save/save"),
        "save request hit the document save route: {}",
        request.request_line
    );
    assert_hsk_operator_document_headers(&request);
    assert!(
        request.body.contains("\"expected_version\":7"),
        "save body carries optimistic concurrency version: {}",
        request.body
    );
}

// ── PT-4 / integration (real backend, gated): openDailyJournal returns today's block ──────────────

/// PT-4: open today's journal against a REAL Handshake-managed backend and verify a journal LoomBlock is
/// returned. Gated behind `--features integration` + `#[ignore]` because it needs a live backend on
/// 127.0.0.1:37501 with a seeded workspace whose id is in `HANDSHAKE_TEST_WORKSPACE_ID`. Absent that,
/// this is NEEDS_MANAGED_RESOURCE_PROOF.
///
/// Run with: `cargo test -p handshake-native --features integration --test test_daily_notes -- \
///   test_real_open_today --ignored`
#[test]
#[ignore = "needs a live Handshake-managed backend + a seeded workspace (NEEDS_MANAGED_RESOURCE_PROOF)"]
#[cfg(feature = "integration")]
fn test_real_open_today() {
    use handshake_native::rich_editor::daily_notes::journal_store::ReqwestJournalBackend;
    let workspace_id = std::env::var("HANDSHAKE_TEST_WORKSPACE_ID")
        .expect("set HANDSHAKE_TEST_WORKSPACE_ID to a seeded workspace id");
    let today = mock_today().format("%Y-%m-%d").to_string();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let backend = ReqwestJournalBackend::production();
    rt.block_on(async {
        let block = backend
            .open_daily_journal(&workspace_id, &today)
            .await
            .expect("openDailyJournal returns a journal block for today");
        assert_eq!(
            block.workspace_id, workspace_id,
            "the block belongs to the requested workspace"
        );
        // The verified open_daily_journal get-or-creates a journal-content-type block for the date.
        assert_eq!(
            block.content_type.as_deref(),
            Some("journal"),
            "PT-4: a journal block is returned"
        );
        // If the block links a document, it must load.
        if let Some(doc_id) = block.document_id.as_deref() {
            let load = backend
                .load_document(doc_id)
                .await
                .expect("the linked document loads");
            assert_eq!(load.body.rich_document_id, doc_id);
        }
    });
}
